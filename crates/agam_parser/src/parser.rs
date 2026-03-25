//! The main parser implementation.
//!
//! Uses recursive descent for statements/declarations and Pratt parsing
//! (precedence climbing) for expressions.

use agam_ast::*;
use agam_ast::expr::*;
use agam_ast::stmt::*;
use agam_ast::decl::*;
use agam_ast::types::*;
use agam_ast::pattern::*;
use agam_lexer::{Token, TokenKind};
use agam_errors::span::{SourceId, Span};
use crate::ParseError;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    next_node_id: u32,
    errors: Vec<ParseError>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0, next_node_id: 0, errors: Vec::new() }
    }

    fn node_id(&mut self) -> NodeId {
        let id = NodeId(self.next_node_id);
        self.next_node_id += 1;
        id
    }

    // ── Token navigation ──

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&self.tokens[self.tokens.len() - 1])
    }

    fn peek_kind(&self) -> TokenKind { self.peek().kind }

    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.pos.min(self.tokens.len() - 1)];
        if self.pos < self.tokens.len() { self.pos += 1; }
        tok
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, ParseError> {
        if self.peek_kind() == kind {
            Ok(self.advance().clone())
        } else {
            Err(self.error(format!("expected {:?}, found {:?}", kind, self.peek_kind())))
        }
    }

    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.peek_kind() == kind { self.advance(); true } else { false }
    }

    fn skip_newlines(&mut self) {
        while self.peek_kind() == TokenKind::Newline
            || self.peek_kind() == TokenKind::LineComment
            || self.peek_kind() == TokenKind::BlockComment
            || self.peek_kind() == TokenKind::DocComment
        { self.advance(); }
    }

    fn at_end(&self) -> bool { self.peek_kind() == TokenKind::Eof }

    fn error(&self, msg: String) -> ParseError {
        ParseError { message: msg, span: self.peek().span }
    }

    // ── Module ──

    pub fn parse_module(&mut self, source_id: SourceId) -> Result<Module, Vec<ParseError>> {
        let mut decls = Vec::new();
        self.skip_newlines();
        while !self.at_end() {
            match self.parse_declaration() {
                Ok(d) => decls.push(d),
                Err(e) => { self.errors.push(e); self.advance(); }
            }
            self.skip_newlines();
        }
        if self.errors.is_empty() {
            Ok(Module {
                id: self.node_id(),
                span: Span::new(source_id, 0, self.peek().span.end),
                declarations: decls,
            })
        } else {
            Err(self.errors.clone())
        }
    }

    // ── Declarations ──

    fn parse_declaration(&mut self) -> Result<Decl, ParseError> {
        self.skip_newlines();
        let span_start = self.peek().span.start;
        let vis = self.parse_visibility();
        let annotations = self.parse_annotations();

        let kind = match self.peek_kind() {
            TokenKind::Fn | TokenKind::Async => {
                DeclKind::Function(self.parse_function_decl(vis, annotations)?)
            }
            TokenKind::Struct | TokenKind::Class => {
                DeclKind::Struct(self.parse_struct_decl(vis, annotations)?)
            }
            TokenKind::Enum => DeclKind::Enum(self.parse_enum_decl(vis)?),
            TokenKind::Trait => DeclKind::Trait(self.parse_trait_decl(vis)?),
            TokenKind::Impl => DeclKind::Impl(self.parse_impl_decl()?),
            TokenKind::Mod => DeclKind::Module(self.parse_module_decl(vis)?),
            TokenKind::Use | TokenKind::Import => DeclKind::Use(self.parse_use_decl(vis)?),
            _ => {
                let stmt = self.parse_statement()?;
                return Ok(Decl {
                    id: self.node_id(), kind: DeclKind::Function(FunctionDecl {
                        name: Ident::new("__top_level__", stmt.span),
                        generics: vec![], params: vec![],
                        return_type: None,
                        body: Some(Block { stmts: vec![stmt], expr: None, span: Span::dummy() }),
                        visibility: Visibility::Private, is_async: false,
                        annotations: vec![], span: Span::dummy(),
                    }),
                    span: Span::new(self.peek().span.source_id, span_start, self.peek().span.end),
                    attributes: vec![],
                });
            }
        };

        Ok(Decl {
            id: self.node_id(),
            span: Span::new(self.peek().span.source_id, span_start, self.peek().span.end),
            kind,
            attributes: vec![],
        })
    }

    fn parse_visibility(&mut self) -> Visibility {
        if self.eat(TokenKind::Pub) { Visibility::Public } else { Visibility::Private }
    }

    fn parse_annotations(&mut self) -> Vec<Annotation> {
        let mut anns = Vec::new();
        while self.peek_kind() == TokenKind::At {
            let start = self.peek().span.start;
            self.advance();
            
            let mut name_parts = Vec::new();
            let mut end_span = start;
            
            while self.peek_kind() == TokenKind::Identifier {
                let t = self.advance().clone();
                name_parts.push(t.lexeme.clone());
                end_span = t.span.end;
                
                if self.peek_kind() == TokenKind::Dot {
                    self.advance();
                } else {
                    break;
                }
            }
            
            if !name_parts.is_empty() {
                let name = Ident::new(&name_parts.join("."), Span::new(self.peek().span.source_id, start, end_span));
                anns.push(Annotation { name, args: vec![], span: Span::new(self.peek().span.source_id, start, end_span) });
            }
            self.skip_newlines();
        }
        anns
    }

    fn parse_function_decl(&mut self, vis: Visibility, annotations: Vec<Annotation>) -> Result<FunctionDecl, ParseError> {
        let is_async = self.eat(TokenKind::Async);
        let fn_tok = self.expect(TokenKind::Fn)?;
        let name_tok = self.expect(TokenKind::Identifier)?;
        let name = Ident::new(&name_tok.lexeme, name_tok.span);

        self.expect(TokenKind::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(TokenKind::RParen)?;

        let return_type = if self.eat(TokenKind::Arrow) {
            Some(self.parse_type_expr()?)
        } else { None };

        let body = if self.eat(TokenKind::Colon) || self.peek_kind() == TokenKind::LBrace {
            Some(self.parse_block()?)
        } else { None };

        Ok(FunctionDecl {
            name, generics: vec![], params, return_type, body,
            visibility: vis, is_async, annotations,
            span: Span::new(fn_tok.span.source_id, fn_tok.span.start, self.peek().span.end),
        })
    }

    fn parse_param_list(&mut self) -> Result<Vec<FunctionParam>, ParseError> {
        let mut params = Vec::new();
        while self.peek_kind() != TokenKind::RParen && !self.at_end() {
            let pat = self.parse_pattern()?;
            let ty = if self.eat(TokenKind::Colon) {
                self.parse_type_expr()?
            } else {
                TypeExpr { id: self.node_id(), span: Span::dummy(), kind: TypeExprKind::Inferred, mode: TypeMode::Inferred }
            };
            let default = if self.eat(TokenKind::Eq) { Some(self.parse_expression(0)?) } else { None };
            params.push(FunctionParam { pattern: pat, ty, default, span: Span::dummy() });
            if !self.eat(TokenKind::Comma) { break; }
        }
        Ok(params)
    }

    fn parse_struct_decl(&mut self, vis: Visibility, annotations: Vec<Annotation>) -> Result<StructDecl, ParseError> {
        self.advance(); // struct/class
        let name_tok = self.expect(TokenKind::Identifier)?;
        let name = Ident::new(&name_tok.lexeme, name_tok.span);
        self.eat(TokenKind::Colon); self.skip_newlines();
        let mut fields = Vec::new();
        if self.eat(TokenKind::LBrace) || true {
            // Parse until we hit something that isn't a field
            self.skip_newlines();
            while self.peek_kind() == TokenKind::Identifier || self.peek_kind() == TokenKind::Pub {
                let fvis = self.parse_visibility();
                let fname_tok = self.expect(TokenKind::Identifier)?;
                let fname = Ident::new(&fname_tok.lexeme, fname_tok.span);
                self.expect(TokenKind::Colon)?;
                let fty = self.parse_type_expr()?;
                let default = if self.eat(TokenKind::Eq) { Some(self.parse_expression(0)?) } else { None };
                fields.push(FieldDecl { name: fname, ty: fty, default, visibility: fvis, span: fname_tok.span });
                self.eat(TokenKind::Comma); self.eat(TokenKind::Semicolon); self.skip_newlines();
                if self.peek_kind() == TokenKind::RBrace { break; }
            }
            self.eat(TokenKind::RBrace);
        }
        Ok(StructDecl { name, generics: vec![], fields, visibility: vis, annotations, span: name_tok.span })
    }

    fn parse_enum_decl(&mut self, vis: Visibility) -> Result<EnumDecl, ParseError> {
        self.advance(); // enum
        let name_tok = self.expect(TokenKind::Identifier)?;
        let name = Ident::new(&name_tok.lexeme, name_tok.span);
        self.eat(TokenKind::Colon); self.skip_newlines(); self.eat(TokenKind::LBrace); self.skip_newlines();
        let mut variants = Vec::new();
        while self.peek_kind() == TokenKind::Identifier {
            let vname_tok = self.advance().clone();
            let vname = Ident::new(&vname_tok.lexeme, vname_tok.span);
            let fields = if self.eat(TokenKind::LParen) {
                let mut types = Vec::new();
                while self.peek_kind() != TokenKind::RParen && !self.at_end() {
                    types.push(self.parse_type_expr()?);
                    if !self.eat(TokenKind::Comma) { break; }
                }
                self.expect(TokenKind::RParen)?;
                VariantFields::Tuple(types)
            } else { VariantFields::Unit };
            variants.push(EnumVariant { name: vname, fields, span: vname_tok.span });
            self.eat(TokenKind::Comma); self.skip_newlines();
        }
        self.eat(TokenKind::RBrace);
        Ok(EnumDecl { name, generics: vec![], variants, visibility: vis, span: name_tok.span })
    }

    fn parse_trait_decl(&mut self, vis: Visibility) -> Result<TraitDecl, ParseError> {
        self.advance(); // trait
        let name_tok = self.expect(TokenKind::Identifier)?;
        let name = Ident::new(&name_tok.lexeme, name_tok.span);
        self.eat(TokenKind::Colon); self.skip_newlines(); self.eat(TokenKind::LBrace); self.skip_newlines();
        let mut items = Vec::new();
        while self.peek_kind() == TokenKind::Fn || self.peek_kind() == TokenKind::Async {
            let f = self.parse_function_decl(Visibility::Public, vec![])?;
            items.push(TraitItem::Method(f));
            self.skip_newlines();
        }
        self.eat(TokenKind::RBrace);
        Ok(TraitDecl { name, generics: vec![], super_traits: vec![], items, visibility: vis, span: name_tok.span })
    }

    fn parse_impl_decl(&mut self) -> Result<ImplDecl, ParseError> {
        let start = self.advance().span; // impl
        let target = self.parse_type_expr()?;
        self.eat(TokenKind::Colon); self.skip_newlines(); self.eat(TokenKind::LBrace); self.skip_newlines();
        let mut items = Vec::new();
        while self.peek_kind() == TokenKind::Fn || self.peek_kind() == TokenKind::Pub
            || self.peek_kind() == TokenKind::Async {
            items.push(self.parse_declaration()?);
            self.skip_newlines();
        }
        self.eat(TokenKind::RBrace);
        Ok(ImplDecl { generics: vec![], trait_path: None, target_type: target, items, span: start })
    }

    fn parse_module_decl(&mut self, vis: Visibility) -> Result<ModuleDecl, ParseError> {
        self.advance(); // mod
        let name_tok = self.expect(TokenKind::Identifier)?;
        let name = Ident::new(&name_tok.lexeme, name_tok.span);
        Ok(ModuleDecl { name, body: None, visibility: vis, span: name_tok.span })
    }

    fn parse_use_decl(&mut self, vis: Visibility) -> Result<UseDecl, ParseError> {
        let start = self.advance().span; // use/import
        let path = self.parse_path()?;
        let alias = if self.eat(TokenKind::As) {
            let a = self.expect(TokenKind::Identifier)?;
            Some(Ident::new(&a.lexeme, a.span))
        } else { None };
        self.eat(TokenKind::Semicolon); self.skip_newlines();
        Ok(UseDecl { path, alias, items: None, visibility: vis, span: start })
    }

    // ── Statements ──

    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        self.skip_newlines();
        let start = self.peek().span.start;

        let kind = match self.peek_kind() {
            TokenKind::Let => self.parse_let_stmt()?,
            TokenKind::Var => self.parse_var_stmt()?,
            TokenKind::Const => self.parse_const_stmt()?,
            TokenKind::Return => { self.advance(); let e = if !self.at_stmt_end() { Some(self.parse_expression(0)?) } else { None }; StmtKind::Return(e) }
            TokenKind::Break => { self.advance(); StmtKind::Break(None) }
            TokenKind::Continue => { self.advance(); StmtKind::Continue }
            TokenKind::While => self.parse_while_stmt()?,
            TokenKind::For => self.parse_for_stmt()?,
            TokenKind::Loop => { self.advance(); StmtKind::Loop { body: self.parse_block()? } }
            TokenKind::If => self.parse_if_stmt()?,
            _ => StmtKind::Expression(self.parse_expression(0)?),
        };
        self.eat(TokenKind::Semicolon); self.skip_newlines();
        Ok(Stmt { id: self.node_id(), span: Span::new(self.peek().span.source_id, start, self.peek().span.end), kind })
    }

    fn at_stmt_end(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Newline | TokenKind::Semicolon | TokenKind::Eof | TokenKind::RBrace)
    }

    fn parse_let_stmt(&mut self) -> Result<StmtKind, ParseError> {
        self.advance(); // let
        let mutable = self.eat(TokenKind::Mut);
        let pattern = self.parse_pattern()?;
        let ty = if self.eat(TokenKind::Colon) { Some(self.parse_type_expr()?) } else { None };
        let value = if self.eat(TokenKind::Eq) { Some(self.parse_expression(0)?) } else { None };
        Ok(StmtKind::Let { pattern, ty, value, mutable })
    }

    /// Parse a `var` statement (dynamic typing mode).
    /// `var x = 42` — type resolved at runtime, Python-like.
    /// `var x: dyn = anything` — explicitly dynamic.
    fn parse_var_stmt(&mut self) -> Result<StmtKind, ParseError> {
        self.advance(); // var
        let mutable = true; // var is always mutable
        let pattern = self.parse_pattern()?;
        let ty = if self.eat(TokenKind::Colon) {
            let mut t = self.parse_type_expr()?;
            t.mode = TypeMode::Dynamic; // var always uses dynamic mode
            Some(t)
        } else {
            Some(TypeExpr {
                id: self.node_id(), span: Span::dummy(),
                kind: TypeExprKind::Dynamic, mode: TypeMode::Dynamic,
            })
        };
        let value = if self.eat(TokenKind::Eq) { Some(self.parse_expression(0)?) } else { None };
        Ok(StmtKind::Let { pattern, ty, value, mutable })
    }

    fn parse_const_stmt(&mut self) -> Result<StmtKind, ParseError> {
        self.advance(); // const
        let name_tok = self.expect(TokenKind::Identifier)?;
        let ty = if self.eat(TokenKind::Colon) { Some(self.parse_type_expr()?) } else { None };
        self.expect(TokenKind::Eq)?;
        let value = self.parse_expression(0)?;
        Ok(StmtKind::Const { name: Ident::new(&name_tok.lexeme, name_tok.span), ty, value })
    }

    fn parse_while_stmt(&mut self) -> Result<StmtKind, ParseError> {
        self.advance(); // while
        let cond = self.parse_expression(0)?;
        let body = self.parse_block()?;
        Ok(StmtKind::While { condition: cond, body })
    }

    fn parse_for_stmt(&mut self) -> Result<StmtKind, ParseError> {
        self.advance(); // for
        let pattern = self.parse_pattern()?;
        self.expect(TokenKind::In)?;
        let iterable = self.parse_expression(0)?;
        let body = self.parse_block()?;
        Ok(StmtKind::For { pattern, iterable, body })
    }

    fn parse_if_stmt(&mut self) -> Result<StmtKind, ParseError> {
        self.advance(); // if
        let cond = self.parse_expression(0)?;
        let then = self.parse_block()?;
        let else_br = if self.eat(TokenKind::Else) {
            if self.peek_kind() == TokenKind::If {
                Some(ElseBranch::ElseIf(Box::new(self.parse_statement()?)))
            } else {
                Some(ElseBranch::Else(self.parse_block()?))
            }
        } else { None };
        Ok(StmtKind::If { condition: cond, then_branch: then, else_branch: else_br })
    }

    // ── Block ──

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        self.skip_newlines();
        let start = self.peek().span.start;
        let has_brace = self.eat(TokenKind::LBrace);
        if !has_brace { self.eat(TokenKind::Colon); }
        self.skip_newlines();

        let has_indent = if !has_brace { self.eat(TokenKind::Indent) } else { false };

        let mut stmts = Vec::new();
        while !self.at_end() {
            if has_brace && self.peek_kind() == TokenKind::RBrace { break; }
            if has_indent && self.peek_kind() == TokenKind::Dedent { break; }
            // Heuristic: stop after one statement in base mode if there was no indent
            if !has_brace && !has_indent && !stmts.is_empty() {
                break;
            }
            match self.parse_statement() {
                Ok(s) => stmts.push(s),
                Err(e) => { self.errors.push(e); self.advance(); break; }
            }
            self.skip_newlines();
        }
        if has_brace { self.eat(TokenKind::RBrace); }
        if has_indent { self.eat(TokenKind::Dedent); }

        Ok(Block { stmts, expr: None, span: Span::new(self.peek().span.source_id, start, self.peek().span.end) })
    }

    // ── Expressions (Pratt parser) ──

    pub fn parse_expression(&mut self, min_prec: u8) -> Result<Expr, ParseError> {
        let mut left = self.parse_prefix()?;

        while let Some((prec, assoc)) = self.infix_binding_power() {
            if prec < min_prec { break; }
            let next_min = if assoc == Assoc::Left { prec + 1 } else { prec };
            left = self.parse_infix(left, next_min)?;
        }

        // Contextual struct literal: only for dotted paths like `module.Type { ... }`
        if self.peek_kind() == TokenKind::LBrace {
            if let ExprKind::FieldAccess { .. } = &left.kind {
                let id = self.node_id();
                let span = self.peek().span;
                left = self.parse_infix(left, 14)?;
            }
        }

        Ok(left)
    }

    fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        let tok = self.peek().clone();
        let id = self.node_id();

        match tok.kind {
            TokenKind::IntLiteral => {
                self.advance();
                let val = tok.lexeme.replace('_', "");
                let n = if val.starts_with("0x") || val.starts_with("0X") {
                    i64::from_str_radix(&val[2..], 16).unwrap_or(0)
                } else if val.starts_with("0b") || val.starts_with("0B") {
                    i64::from_str_radix(&val[2..], 2).unwrap_or(0)
                } else if val.starts_with("0o") || val.starts_with("0O") {
                    i64::from_str_radix(&val[2..], 8).unwrap_or(0)
                } else {
                    val.parse().unwrap_or(0)
                };
                Ok(Expr { id, span: tok.span, kind: ExprKind::IntLiteral(n) })
            }
            TokenKind::FloatLiteral => {
                self.advance();
                let n: f64 = tok.lexeme.replace('_', "").parse().unwrap_or(0.0);
                Ok(Expr { id, span: tok.span, kind: ExprKind::FloatLiteral(n) })
            }
            TokenKind::StringLiteral => {
                self.advance();
                let s = tok.lexeme[1..tok.lexeme.len()-1].to_string();
                Ok(Expr { id, span: tok.span, kind: ExprKind::StringLiteral(s) })
            }
            TokenKind::FStringLiteral => {
                self.advance();
                Ok(Expr { id, span: tok.span, kind: ExprKind::FStringLiteral { parts: vec![FStringPart::Literal(tok.lexeme.clone())] } })
            }
            TokenKind::True => { self.advance(); Ok(Expr { id, span: tok.span, kind: ExprKind::BoolLiteral(true) }) }
            TokenKind::False => { self.advance(); Ok(Expr { id, span: tok.span, kind: ExprKind::BoolLiteral(false) }) }
            TokenKind::Identifier => {
                self.advance();
                let ident = Ident::new(&tok.lexeme, tok.span);
                Ok(Expr { id, span: tok.span, kind: ExprKind::Identifier(ident) })
            }
            TokenKind::Pipe => {
                self.advance();
                let mut params = Vec::new();
                while self.peek_kind() != TokenKind::Pipe && !self.at_end() {
                    let name_tok = self.expect(TokenKind::Identifier)?;
                    let name = Ident::new(&name_tok.lexeme, name_tok.span);
                    let mut ty = None;
                    if self.eat(TokenKind::Colon) {
                        ty = Some(self.parse_type_expr()?);
                    }
                    let span = name.span;
                    params.push(LambdaParam { name, ty, span });
                    if !self.eat(TokenKind::Comma) { break; }
                }
                self.expect(TokenKind::Pipe)?;
                
                let mut return_type = None;
                if self.eat(TokenKind::Arrow) {
                    return_type = Some(Box::new(self.parse_type_expr()?));
                }
                
                let body = if self.peek_kind() == TokenKind::LBrace {
                    let block = self.parse_block()?;
                    Expr { id: self.node_id(), span: block.span, kind: ExprKind::BlockExpr(block) }
                } else {
                    self.parse_expression(0)?
                };
                Ok(Expr { id, span: tok.span, kind: ExprKind::Lambda { params, return_type, body: Box::new(body) } })
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expression(0)?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::LBracket => {
                self.advance();
                let mut elements = Vec::new();
                while self.peek_kind() != TokenKind::RBracket && !self.at_end() {
                    elements.push(self.parse_expression(0)?);
                    if !self.eat(TokenKind::Comma) { break; }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Expr { id, span: tok.span, kind: ExprKind::ArrayLiteral(elements) })
            }
            TokenKind::Minus => {
                self.advance();
                let operand = self.parse_expression(15)?; // High prec for unary
                Ok(Expr { id, span: tok.span, kind: ExprKind::Unary { op: UnaryOp::Neg, operand: Box::new(operand) } })
            }
            TokenKind::Bang => {
                self.advance();
                let operand = self.parse_expression(15)?;
                Ok(Expr { id, span: tok.span, kind: ExprKind::Unary { op: UnaryOp::Not, operand: Box::new(operand) } })
            }
            TokenKind::Amp => {
                self.advance();
                let operand = self.parse_expression(15)?;
                Ok(Expr { id, span: tok.span, kind: ExprKind::Unary { op: UnaryOp::Ref, operand: Box::new(operand) } })
            }
            TokenKind::Await => {
                self.advance();
                let operand = self.parse_expression(0)?;
                Ok(Expr { id, span: tok.span, kind: ExprKind::Await(Box::new(operand)) })
            }
            _ => Err(self.error(format!("expected expression, found {:?}", tok.kind))),
        }
    }

    fn infix_binding_power(&self) -> Option<(u8, Assoc)> {
        match self.peek_kind() {
            TokenKind::Eq | TokenKind::PlusEq | TokenKind::MinusEq
            | TokenKind::StarEq | TokenKind::SlashEq => Some((1, Assoc::Right)),
            TokenKind::PipePipe => Some((2, Assoc::Left)),
            TokenKind::AmpAmp => Some((3, Assoc::Left)),
            TokenKind::Pipe => Some((4, Assoc::Left)),
            TokenKind::Caret => Some((5, Assoc::Left)),
            TokenKind::Amp => Some((6, Assoc::Left)),
            TokenKind::EqEq | TokenKind::BangEq => Some((7, Assoc::Left)),
            TokenKind::Lt | TokenKind::LtEq | TokenKind::Gt | TokenKind::GtEq => Some((8, Assoc::Left)),
            TokenKind::Shl | TokenKind::Shr => Some((9, Assoc::Left)),
            TokenKind::Plus | TokenKind::Minus => Some((10, Assoc::Left)),
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Some((11, Assoc::Left)),
            TokenKind::DoubleStar => Some((12, Assoc::Right)),
            TokenKind::As => Some((13, Assoc::Left)),
            TokenKind::Dot => Some((14, Assoc::Left)),
            TokenKind::LParen | TokenKind::LBracket => Some((14, Assoc::Left)),
            TokenKind::Question => Some((15, Assoc::Left)),
            _ => None,
        }
    }

    fn parse_infix(&mut self, left: Expr, min_prec: u8) -> Result<Expr, ParseError> {
        let op_tok = self.peek().clone();
        let id = self.node_id();

        match op_tok.kind {
            // Assignment
            TokenKind::Eq => {
                self.advance();
                let right = self.parse_expression(min_prec)?;
                Ok(Expr { id, span: op_tok.span, kind: ExprKind::Assign { target: Box::new(left), value: Box::new(right) } })
            }
            TokenKind::PlusEq | TokenKind::MinusEq | TokenKind::StarEq | TokenKind::SlashEq => {
                let op = match op_tok.kind {
                    TokenKind::PlusEq => BinOp::Add, TokenKind::MinusEq => BinOp::Sub,
                    TokenKind::StarEq => BinOp::Mul, _ => BinOp::Div,
                };
                self.advance();
                let right = self.parse_expression(min_prec)?;
                Ok(Expr { id, span: op_tok.span, kind: ExprKind::CompoundAssign { op, target: Box::new(left), value: Box::new(right) } })
            }
            // Binary ops
            TokenKind::Plus | TokenKind::Minus | TokenKind::Star | TokenKind::Slash
            | TokenKind::Percent | TokenKind::DoubleStar
            | TokenKind::EqEq | TokenKind::BangEq | TokenKind::Lt | TokenKind::LtEq
            | TokenKind::Gt | TokenKind::GtEq | TokenKind::AmpAmp | TokenKind::PipePipe
            | TokenKind::Amp | TokenKind::Pipe | TokenKind::Caret
            | TokenKind::Shl | TokenKind::Shr => {
                let op = self.token_to_binop(op_tok.kind);
                self.advance();
                let right = self.parse_expression(min_prec)?;
                Ok(Expr { id, span: op_tok.span, kind: ExprKind::Binary { op, left: Box::new(left), right: Box::new(right) } })
            }
            // Field access / method call
            TokenKind::Dot => {
                self.advance();
                let field_tok = self.expect(TokenKind::Identifier)?;
                let field = Ident::new(&field_tok.lexeme, field_tok.span);
                if self.peek_kind() == TokenKind::LParen {
                    self.advance();
                    let args = self.parse_arg_list()?;
                    self.expect(TokenKind::RParen)?;
                    Ok(Expr { id, span: op_tok.span, kind: ExprKind::MethodCall { object: Box::new(left), method: field, args } })
                } else {
                    Ok(Expr { id, span: op_tok.span, kind: ExprKind::FieldAccess { object: Box::new(left), field } })
                }
            }
            // Function call
            TokenKind::LParen => {
                self.advance();
                let args = self.parse_arg_list()?;
                self.expect(TokenKind::RParen)?;
                Ok(Expr { id, span: op_tok.span, kind: ExprKind::Call { callee: Box::new(left), args } })
            }
            // Index
            TokenKind::LBracket => {
                self.advance();
                let index = self.parse_expression(0)?;
                self.expect(TokenKind::RBracket)?;
                Ok(Expr { id, span: op_tok.span, kind: ExprKind::Index { object: Box::new(left), index: Box::new(index) } })
            }
            // Struct Literal
            TokenKind::LBrace => {
                self.advance();
                
                // Convert the 'left' expression into a Path
                let path = match &left.kind {
                    ExprKind::Identifier(ident) => Path { segments: vec![ident.clone()], span: left.span },
                    ExprKind::FieldAccess { object, field } => {
                        if let ExprKind::Identifier(obj_id) = &object.kind {
                            Path { segments: vec![obj_id.clone(), field.clone()], span: left.span }
                        } else {
                            return Err(self.error("Invalid struct path".to_string()));
                        }
                    },
                    _ => return Err(self.error("Expected identifier or path before '{'".to_string())),
                };

                let mut fields = Vec::new();
                while self.peek_kind() != TokenKind::RBrace && !self.at_end() {
                    let field_name_str = if self.peek_kind() == TokenKind::StringLiteral {
                        let lit = self.peek().lexeme.clone();
                        self.advance();
                        lit[1..lit.len()-1].to_string() // stip quotes for dict-like struct init
                    } else {
                        let id_tok = self.expect(TokenKind::Identifier)?;
                        id_tok.lexeme.clone()
                    };
                    
                    let name = Ident::new(&field_name_str, self.peek().span);
                    let span = name.span;
                    self.expect(TokenKind::Colon)?;
                    let value = self.parse_expression(0)?;
                    fields.push(FieldInit { name, value, span });
                    if !self.eat(TokenKind::Comma) { break; }
                }
                self.expect(TokenKind::RBrace)?;
                Ok(Expr { id, span: op_tok.span, kind: ExprKind::StructLiteral { path, fields } })
            }
            // Try
            TokenKind::Question => {
                self.advance();
                Ok(Expr { id, span: op_tok.span, kind: ExprKind::Try(Box::new(left)) })
            }
            // Cast
            TokenKind::As => {
                self.advance();
                let ty = self.parse_type_expr()?;
                Ok(Expr { id, span: op_tok.span, kind: ExprKind::Cast { expr: Box::new(left), target_type: Box::new(ty) } })
            }
            _ => Err(self.error(format!("unexpected infix token: {:?}", op_tok.kind))),
        }
    }

    fn parse_arg_list(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = Vec::new();
        while self.peek_kind() != TokenKind::RParen && !self.at_end() {
            args.push(self.parse_expression(0)?);
            if !self.eat(TokenKind::Comma) { break; }
        }
        Ok(args)
    }

    fn token_to_binop(&self, kind: TokenKind) -> BinOp {
        match kind {
            TokenKind::Plus => BinOp::Add, TokenKind::Minus => BinOp::Sub,
            TokenKind::Star => BinOp::Mul, TokenKind::Slash => BinOp::Div,
            TokenKind::Percent => BinOp::Mod, TokenKind::DoubleStar => BinOp::Pow,
            TokenKind::EqEq => BinOp::Eq, TokenKind::BangEq => BinOp::NotEq,
            TokenKind::Lt => BinOp::Lt, TokenKind::LtEq => BinOp::LtEq,
            TokenKind::Gt => BinOp::Gt, TokenKind::GtEq => BinOp::GtEq,
            TokenKind::AmpAmp => BinOp::And, TokenKind::PipePipe => BinOp::Or,
            TokenKind::Amp => BinOp::BitAnd, TokenKind::Pipe => BinOp::BitOr,
            TokenKind::Caret => BinOp::BitXor, TokenKind::Shl => BinOp::Shl,
            TokenKind::Shr => BinOp::Shr, _ => BinOp::Add,
        }
    }

    // ── Types ──

    fn parse_type_expr(&mut self) -> Result<TypeExpr, ParseError> {
        let id = self.node_id();
        let tok = self.peek().clone();

        // Handle `dyn` keyword: `dyn` alone = Dynamic, `dyn Trait` = DynTrait
        if tok.kind == TokenKind::Dyn {
            self.advance();
            if self.peek_kind() == TokenKind::Identifier || self.peek_kind() == TokenKind::SelfType {
                let inner = self.parse_type_expr()?;
                return Ok(TypeExpr { id, span: tok.span, kind: TypeExprKind::DynTrait(Box::new(inner)), mode: TypeMode::Dynamic });
            } else {
                return Ok(TypeExpr { id, span: tok.span, kind: TypeExprKind::Dynamic, mode: TypeMode::Dynamic });
            }
        }

        match tok.kind {
            TokenKind::Identifier => {
                self.advance();
                // Handle `Any` as the universal dynamic type
                if tok.lexeme == "Any" {
                    return Ok(TypeExpr { id, span: tok.span, kind: TypeExprKind::Any, mode: TypeMode::Dynamic });
                }
                let path = Path { segments: vec![Ident::new(&tok.lexeme, tok.span)], span: tok.span };
                if self.peek_kind() == TokenKind::Lt {
                    self.advance();
                    let mut args = Vec::new();
                    while self.peek_kind() != TokenKind::Gt && !self.at_end() {
                        args.push(self.parse_type_expr()?);
                        if !self.eat(TokenKind::Comma) { break; }
                    }
                    self.expect(TokenKind::Gt)?;
                    Ok(TypeExpr { id, span: tok.span, kind: TypeExprKind::Generic { base: path, args }, mode: TypeMode::Static })
                } else {
                    Ok(TypeExpr { id, span: tok.span, kind: TypeExprKind::Named(path), mode: TypeMode::Static })
                }
            }
            TokenKind::Amp => {
                self.advance();
                let mutable = self.eat(TokenKind::Mut);
                let inner = self.parse_type_expr()?;
                Ok(TypeExpr { id, span: tok.span, kind: TypeExprKind::Reference { mutable, inner: Box::new(inner) }, mode: TypeMode::Static })
            }
            TokenKind::Star => {
                self.advance();
                let mutable = self.eat(TokenKind::Mut);
                let inner = self.parse_type_expr()?;
                Ok(TypeExpr { id, span: tok.span, kind: TypeExprKind::Pointer { mutable, inner: Box::new(inner) }, mode: TypeMode::Static })
            }
            TokenKind::LBracket => {
                self.advance();
                let inner = self.parse_type_expr()?;
                self.expect(TokenKind::RBracket)?;
                Ok(TypeExpr { id, span: tok.span, kind: TypeExprKind::Slice(Box::new(inner)), mode: TypeMode::Static })
            }
            TokenKind::LParen => {
                self.advance();
                let mut types = Vec::new();
                while self.peek_kind() != TokenKind::RParen && !self.at_end() {
                    types.push(self.parse_type_expr()?);
                    if !self.eat(TokenKind::Comma) { break; }
                }
                self.expect(TokenKind::RParen)?;
                Ok(TypeExpr { id, span: tok.span, kind: TypeExprKind::Tuple(types), mode: TypeMode::Static })
            }
            TokenKind::SelfType => {
                self.advance();
                Ok(TypeExpr { id, span: tok.span, kind: TypeExprKind::SelfType, mode: TypeMode::Static })
            }
            _ => Err(self.error(format!("expected type, found {:?}", tok.kind))),
        }
    }

    // ── Patterns ──

    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        let id = self.node_id();
        let tok = self.peek().clone();

        match tok.kind {
            TokenKind::Identifier => {
                self.advance();
                Ok(Pattern { id, span: tok.span, kind: PatternKind::Identifier { name: Ident::new(&tok.lexeme, tok.span), mutable: false } })
            }
            TokenKind::Mut => {
                self.advance();
                let name_tok = self.expect(TokenKind::Identifier)?;
                Ok(Pattern { id, span: tok.span, kind: PatternKind::Identifier { name: Ident::new(&name_tok.lexeme, name_tok.span), mutable: true } })
            }
            TokenKind::LParen => {
                self.advance();
                let mut pats = Vec::new();
                while self.peek_kind() != TokenKind::RParen && !self.at_end() {
                    pats.push(self.parse_pattern()?);
                    if !self.eat(TokenKind::Comma) { break; }
                }
                self.expect(TokenKind::RParen)?;
                Ok(Pattern { id, span: tok.span, kind: PatternKind::Tuple(pats) })
            }
            TokenKind::IntLiteral | TokenKind::StringLiteral | TokenKind::True | TokenKind::False => {
                let expr = self.parse_prefix()?;
                Ok(Pattern { id, span: tok.span, kind: PatternKind::Literal(expr) })
            }
            _ => {
                self.advance();
                Ok(Pattern { id, span: tok.span, kind: PatternKind::Wildcard })
            }
        }
    }

    // ── Paths ──

    fn parse_path(&mut self) -> Result<Path, ParseError> {
        let first = self.expect(TokenKind::Identifier)?;
        let mut segments = vec![Ident::new(&first.lexeme, first.span)];
        while self.eat(TokenKind::ColonColon) || self.eat(TokenKind::Dot) {
            let seg = self.expect(TokenKind::Identifier)?;
            segments.push(Ident::new(&seg.lexeme, seg.span));
        }
        let span = Span::new(first.span.source_id, first.span.start, segments.last().unwrap().span.end);
        Ok(Path { segments, span })
    }
}

#[derive(PartialEq)]
enum Assoc { Left, Right }

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use agam_lexer::tokenize;

    fn parse_expr(src: &str) -> Expr {
        let tokens = tokenize(src, SourceId(0));
        let mut parser = Parser::new(tokens);
        parser.parse_expression(0).unwrap()
    }

    fn parse_src(src: &str) -> Module {
        let tokens = tokenize(src, SourceId(0));
        let mut parser = Parser::new(tokens);
        parser.parse_module(SourceId(0)).unwrap()
    }

    #[test]
    fn test_integer_literal() {
        let expr = parse_expr("42");
        assert!(matches!(expr.kind, ExprKind::IntLiteral(42)));
    }

    #[test]
    fn test_hex_literal() {
        let expr = parse_expr("0xFF");
        assert!(matches!(expr.kind, ExprKind::IntLiteral(255)));
    }

    #[test]
    fn test_float_literal() {
        let expr = parse_expr("3.14");
        if let ExprKind::FloatLiteral(n) = expr.kind { assert!((n - 3.14).abs() < 0.001); }
    }

    #[test]
    fn test_string_literal() {
        let expr = parse_expr(r#""hello""#);
        assert!(matches!(expr.kind, ExprKind::StringLiteral(ref s) if s == "hello"));
    }

    #[test]
    fn test_binary_add() {
        let expr = parse_expr("1 + 2");
        assert!(matches!(expr.kind, ExprKind::Binary { op: BinOp::Add, .. }));
    }

    #[test]
    fn test_precedence() {
        let expr = parse_expr("1 + 2 * 3");
        // Should parse as 1 + (2 * 3)
        if let ExprKind::Binary { op, right, .. } = &expr.kind {
            assert_eq!(*op, BinOp::Add);
            assert!(matches!(right.kind, ExprKind::Binary { op: BinOp::Mul, .. }));
        } else { panic!("not binary"); }
    }

    #[test]
    fn test_function_call() {
        let expr = parse_expr("foo(1, 2)");
        assert!(matches!(expr.kind, ExprKind::Call { .. }));
    }

    #[test]
    fn test_method_call() {
        let expr = parse_expr("x.len()");
        assert!(matches!(expr.kind, ExprKind::MethodCall { .. }));
    }

    #[test]
    fn test_field_access() {
        let expr = parse_expr("point.x");
        assert!(matches!(expr.kind, ExprKind::FieldAccess { .. }));
    }

    #[test]
    fn test_index() {
        let expr = parse_expr("arr[0]");
        assert!(matches!(expr.kind, ExprKind::Index { .. }));
    }

    #[test]
    fn test_array_literal() {
        let expr = parse_expr("[1, 2, 3]");
        if let ExprKind::ArrayLiteral(elems) = &expr.kind {
            assert_eq!(elems.len(), 3);
        } else { panic!("not array"); }
    }

    #[test]
    fn test_unary_neg() {
        let expr = parse_expr("-42");
        assert!(matches!(expr.kind, ExprKind::Unary { op: UnaryOp::Neg, .. }));
    }

    #[test]
    fn test_boolean() {
        assert!(matches!(parse_expr("true").kind, ExprKind::BoolLiteral(true)));
        assert!(matches!(parse_expr("false").kind, ExprKind::BoolLiteral(false)));
    }

    #[test]
    fn test_chained_method() {
        let expr = parse_expr("a.b().c()");
        assert!(matches!(expr.kind, ExprKind::MethodCall { .. }));
    }

    #[test]
    fn test_parse_let() {
        let module = parse_src("let x = 42");
        assert!(!module.declarations.is_empty());
    }

    #[test]
    fn test_parse_function() {
        let module = parse_src("fn add(a: i32, b: i32) -> i32 { return a + b }");
        assert!(!module.declarations.is_empty());
        if let DeclKind::Function(f) = &module.declarations[0].kind {
            assert_eq!(f.name.name, "add");
            assert_eq!(f.params.len(), 2);
        } else { panic!("not function"); }
    }

    #[test]
    fn test_parse_struct() {
        let module = parse_src("struct Point { x: f64, y: f64 }");
        if let DeclKind::Struct(s) = &module.declarations[0].kind {
            assert_eq!(s.name.name, "Point");
            assert_eq!(s.fields.len(), 2);
        } else { panic!("not struct"); }
    }

    #[test]
    fn test_parse_enum() {
        let module = parse_src("enum Color { Red, Green, Blue }");
        if let DeclKind::Enum(e) = &module.declarations[0].kind {
            assert_eq!(e.name.name, "Color");
            assert_eq!(e.variants.len(), 3);
        } else { panic!("not enum"); }
    }

    #[test]
    fn test_comparison() {
        let expr = parse_expr("a == b");
        assert!(matches!(expr.kind, ExprKind::Binary { op: BinOp::Eq, .. }));
    }

    #[test]
    fn test_try_operator() {
        let expr = parse_expr("result?");
        assert!(matches!(expr.kind, ExprKind::Try(_)));
    }
}
