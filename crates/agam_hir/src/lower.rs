//! AST → HIR lowering pass.
//!
//! Transforms the parsed AST into the HIR by:
//! - Desugaring for-in loops into while loops.
//! - Desugaring f-strings into string concatenation.
//! - Attaching resolved type information.
//! - Flattening nested declarations.

use std::collections::HashMap;

use agam_ast::decl::*;
use agam_ast::expr::*;
use agam_ast::stmt::*;
use agam_ast::types::TypeExpr;
use agam_ast::*;
use agam_sema::types::{builtin_type_id_for_name, TypeStore};

use crate::nodes::*;

/// The HIR lowering context.
pub struct HirLowering {
    next_id: u32,
    types: TypeStore,
    scopes: Vec<HashMap<String, agam_sema::symbol::TypeId>>,
}

impl HirLowering {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            types: TypeStore::new(),
            scopes: Vec::new(),
        }
    }

    fn fresh_id(&mut self) -> HirId {
        let id = HirId(self.next_id);
        self.next_id += 1;
        id
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn bind_local(&mut self, name: String, ty: agam_sema::symbol::TypeId) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, ty);
        }
    }

    fn lookup_local(&self, name: &str) -> Option<agam_sema::symbol::TypeId> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    /// Lower a parsed AST module into HIR.
    pub fn lower_module(&mut self, module: &Module) -> HirModule {
        let functions = module
            .declarations
            .iter()
            .filter_map(|decl| self.lower_decl(decl))
            .collect();
        HirModule { functions }
    }

    fn lower_decl(&mut self, decl: &Decl) -> Option<HirFunction> {
        match &decl.kind {
            DeclKind::Function(f) => Some(self.lower_function(f)),
            _ => None,
        }
    }

    fn lower_function(&mut self, f: &FunctionDecl) -> HirFunction {
        self.push_scope();
        let params: Vec<HirParam> = f
            .params
            .iter()
            .map(|p| {
                let name = self.pattern_name(&p.pattern).unwrap_or_else(|| "_".into());
                let ty = self.resolve_type_expr(&p.ty);
                self.bind_local(name.clone(), ty);
                HirParam {
                    name,
                    ty,
                    mutable: true,
                }
            })
            .collect();

        let body = if let Some(b) = &f.body {
            self.lower_block(b)
        } else {
            HirBlock {
                stmts: vec![],
                expr: None,
            }
        };

        let lowered = HirFunction {
            id: self.fresh_id(),
            name: f.name.name.clone(),
            params,
            return_ty: f
                .return_type
                .as_ref()
                .map(|ty| self.resolve_type_expr(ty))
                .unwrap_or_else(|| self.types.unit()),
            body,
            is_async: f.is_async,
        };
        self.pop_scope();
        lowered
    }

    fn lower_block(&mut self, block: &Block) -> HirBlock {
        self.push_scope();
        let stmts = block.stmts.iter().map(|s| self.lower_stmt(s)).collect();
        let expr = block.expr.as_ref().map(|e| Box::new(self.lower_expr(e)));
        let lowered = HirBlock { stmts, expr };
        self.pop_scope();
        lowered
    }

    fn lower_stmt(&mut self, stmt: &Stmt) -> HirStmt {
        match &stmt.kind {
            StmtKind::Let {
                pattern,
                ty,
                value,
                mutable,
            } => {
                let name = self.pattern_name(pattern).unwrap_or_else(|| "_".into());
                let lowered_value = value.as_ref().map(|v| self.lower_expr(v));
                let inferred_ty = lowered_value
                    .as_ref()
                    .map(|expr| expr.ty)
                    .unwrap_or_else(|| self.types.fresh_var());
                let lowered = HirStmt::Let {
                    name,
                    ty: ty
                        .as_ref()
                        .map(|te| self.resolve_type_expr(te))
                        .unwrap_or(inferred_ty),
                    value: lowered_value,
                    mutable: *mutable,
                };
                if let HirStmt::Let { name, ty, .. } = &lowered {
                    self.bind_local(name.clone(), *ty);
                }
                lowered
            }
            StmtKind::Expression(expr) => HirStmt::Expr(self.lower_expr(expr)),
            StmtKind::Return(val) => HirStmt::Return(val.as_ref().map(|v| self.lower_expr(v))),
            StmtKind::While { condition, body } => HirStmt::While {
                condition: self.lower_expr(condition),
                body: self.lower_block(body),
            },
            StmtKind::Loop { body } => HirStmt::Loop {
                body: self.lower_block(body),
            },
            // Desugar for-in → while + iterator pattern
            StmtKind::For {
                pattern,
                iterable,
                body,
            } => {
                let iter_name = format!("__iter_{}", self.next_id);
                let item_name = self.pattern_name(pattern).unwrap_or_else(|| "_".into());
                let iter_ty = self.types.fresh_var();
                let item_ty = self.types.fresh_var();

                // Desugar: let __iter = iterable; while __iter.has_next(): let item = __iter.next(); body
                let iter_init = HirStmt::Let {
                    name: iter_name.clone(),
                    ty: iter_ty,
                    value: Some(self.lower_expr(iterable)),
                    mutable: true,
                };

                let has_next = HirExpr {
                    id: self.fresh_id(),
                    ty: self.types.bool(),
                    kind: HirExprKind::MethodCall {
                        object: Box::new(HirExpr {
                            id: self.fresh_id(),
                            ty: iter_ty,
                            kind: HirExprKind::Var(iter_name.clone()),
                        }),
                        method: "has_next".into(),
                        args: vec![],
                    },
                };

                let next_call = HirExpr {
                    id: self.fresh_id(),
                    ty: item_ty,
                    kind: HirExprKind::MethodCall {
                        object: Box::new(HirExpr {
                            id: self.fresh_id(),
                            ty: iter_ty,
                            kind: HirExprKind::Var(iter_name.clone()),
                        }),
                        method: "next".into(),
                        args: vec![],
                    },
                };

                let mut loop_stmts = vec![HirStmt::Let {
                    name: item_name.clone(),
                    ty: item_ty,
                    value: Some(next_call),
                    mutable: true,
                }];
                self.push_scope();
                self.bind_local(item_name, item_ty);
                let inner_block = self.lower_block(body);
                self.pop_scope();
                loop_stmts.extend(inner_block.stmts);

                HirStmt::Expr(HirExpr {
                    id: self.fresh_id(),
                    ty: self.types.unit(),
                    kind: HirExprKind::Block(HirBlock {
                        stmts: vec![
                            iter_init,
                            HirStmt::While {
                                condition: has_next,
                                body: HirBlock {
                                    stmts: loop_stmts,
                                    expr: inner_block.expr,
                                },
                            },
                        ],
                        expr: None,
                    }),
                })
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let else_block = else_branch.as_ref().map(|eb| match eb {
                    ElseBranch::Else(block) => self.lower_block(block),
                    ElseBranch::ElseIf(s) => HirBlock {
                        stmts: vec![self.lower_stmt(s)],
                        expr: None,
                    },
                });
                HirStmt::If {
                    condition: self.lower_expr(condition),
                    then_branch: self.lower_block(then_branch),
                    else_branch: else_block,
                }
            }
            StmtKind::Break(v) => HirStmt::Break(v.as_ref().map(|e| self.lower_expr(e))),
            StmtKind::Continue => HirStmt::Continue,
            StmtKind::Const { name, value, .. } => {
                let value = self.lower_expr(value);
                let lowered = HirStmt::Let {
                    name: name.name.clone(),
                    ty: value.ty,
                    value: Some(value),
                    mutable: false,
                };
                if let HirStmt::Let { name, ty, .. } = &lowered {
                    self.bind_local(name.clone(), *ty);
                }
                lowered
            }
            _ => HirStmt::Expr(HirExpr {
                id: self.fresh_id(),
                ty: self.types.unit(),
                kind: HirExprKind::Tuple(vec![]),
            }),
        }
    }

    fn lower_expr(&mut self, expr: &Expr) -> HirExpr {
        let id = self.fresh_id();
        let (ty, kind) = match &expr.kind {
            ExprKind::IntLiteral(v) => (self.types.i32(), HirExprKind::IntLit(*v)),
            ExprKind::FloatLiteral(v) => (self.types.f64(), HirExprKind::FloatLit(*v)),
            ExprKind::BoolLiteral(v) => (self.types.bool(), HirExprKind::BoolLit(*v)),
            ExprKind::StringLiteral(v) => (self.types.str(), HirExprKind::StringLit(v.clone())),

            // Desugar f-string into string concat
            ExprKind::FStringLiteral { parts } => {
                let mut result_parts = Vec::new();
                for part in parts {
                    match part {
                        FStringPart::Literal(s) => {
                            result_parts.push(HirExpr {
                                id: self.fresh_id(),
                                ty: self.types.str(),
                                kind: HirExprKind::StringLit(s.clone()),
                            });
                        }
                        FStringPart::Expr(e) => {
                            result_parts.push(self.lower_expr(e));
                        }
                    }
                }
                if result_parts.len() == 1 {
                    return result_parts.pop().unwrap();
                }
                // chain binary Add: part0 + part1 + part2 ...
                let mut acc = result_parts.remove(0);
                for part in result_parts {
                    acc = HirExpr {
                        id: self.fresh_id(),
                        ty: self.types.str(),
                        kind: HirExprKind::Binary {
                            op: HirBinOp::Add,
                            left: Box::new(acc),
                            right: Box::new(part),
                        },
                    };
                }
                return acc;
            }

            ExprKind::Identifier(ident) => (
                self.lookup_local(&ident.name)
                    .unwrap_or_else(|| self.types.fresh_var()),
                HirExprKind::Var(ident.name.clone()),
            ),
            ExprKind::PathExpr(path) => {
                let full = path
                    .segments
                    .iter()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join("::");
                (
                    self.lookup_local(&full)
                        .unwrap_or_else(|| self.types.fresh_var()),
                    HirExprKind::Var(full),
                )
            }

            ExprKind::Binary { op, left, right } => {
                let left = self.lower_expr(left);
                let right = self.lower_expr(right);
                let hir_op = lower_binop(*op);
                (
                    self.resolve_binary_expr_type(hir_op, &left, &right),
                    HirExprKind::Binary {
                        op: hir_op,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                )
            }
            ExprKind::Unary { op, operand } => {
                let operand = self.lower_expr(operand);
                let hir_op = lower_unaryop(*op);
                (
                    self.resolve_unary_expr_type(hir_op, &operand),
                    HirExprKind::Unary {
                        op: hir_op,
                        operand: Box::new(operand),
                    },
                )
            }

            ExprKind::Call { callee, args } => (
                self.types.fresh_var(),
                HirExprKind::Call {
                    callee: Box::new(self.lower_expr(callee)),
                    args: args.iter().map(|a| self.lower_expr(a)).collect(),
                },
            ),
            ExprKind::MethodCall {
                object,
                method,
                args,
            } => (
                self.types.fresh_var(),
                HirExprKind::MethodCall {
                    object: Box::new(self.lower_expr(object)),
                    method: method.name.clone(),
                    args: args.iter().map(|a| self.lower_expr(a)).collect(),
                },
            ),

            ExprKind::FieldAccess { object, field } => (
                self.types.fresh_var(),
                HirExprKind::FieldAccess {
                    object: Box::new(self.lower_expr(object)),
                    field: field.name.clone(),
                },
            ),
            ExprKind::Index { object, index } => (
                self.types.fresh_var(),
                HirExprKind::Index {
                    object: Box::new(self.lower_expr(object)),
                    index: Box::new(self.lower_expr(index)),
                },
            ),

            ExprKind::Assign { target, value } => {
                let target = self.lower_expr(target);
                let value = self.lower_expr(value);
                (
                    target.ty,
                    HirExprKind::Assign {
                        target: Box::new(target),
                        value: Box::new(value),
                    },
                )
            }
            ExprKind::CompoundAssign { op, target, value } => {
                // Desugar: x += 1 → x = x + 1
                let target_hir = self.lower_expr(target);
                let val_hir = self.lower_expr(value);
                let binop = HirExpr {
                    id: self.fresh_id(),
                    ty: self.resolve_binary_expr_type(lower_binop(*op), &target_hir, &val_hir),
                    kind: HirExprKind::Binary {
                        op: lower_binop(*op),
                        left: Box::new(HirExpr {
                            id: self.fresh_id(),
                            ty: target_hir.ty,
                            kind: target_hir.kind.clone_var_name(),
                        }),
                        right: Box::new(val_hir),
                    },
                };
                (
                    target_hir.ty,
                    HirExprKind::Assign {
                        target: Box::new(target_hir),
                        value: Box::new(binop),
                    },
                )
            }

            ExprKind::ArrayLiteral(elems) => {
                (
                    self.types.fresh_var(),
                    HirExprKind::Array(elems.iter().map(|e| self.lower_expr(e)).collect()),
                )
            }
            ExprKind::TupleLiteral(elems) => {
                (
                    self.types.unit(),
                    HirExprKind::Tuple(elems.iter().map(|e| self.lower_expr(e)).collect()),
                )
            }

            ExprKind::BlockExpr(block) => {
                let block = self.lower_block(block);
                let ty = block
                    .expr
                    .as_ref()
                    .map(|expr| expr.ty)
                    .unwrap_or_else(|| self.types.unit());
                (ty, HirExprKind::Block(block))
            }

            ExprKind::Cast {
                expr: inner,
                target_type,
            } => {
                let target_ty = self.resolve_type_expr(target_type);
                (
                    target_ty,
                    HirExprKind::Cast {
                        expr: Box::new(self.lower_expr(inner)),
                        target_ty,
                    },
                )
            }

            // Fallback for unhandled expressions
            _ => (self.types.unit(), HirExprKind::Tuple(vec![])), // Unit value
        };

        HirExpr { id, ty, kind }
    }

    fn resolve_type_expr(&mut self, ty: &TypeExpr) -> agam_sema::symbol::TypeId {
        match &ty.kind {
            agam_ast::types::TypeExprKind::Named(path) => {
                if let Some(segment) = path.segments.last() {
                    builtin_type_id_for_name(&self.types, &segment.name)
                        .unwrap_or_else(|| self.types.fresh_var())
                } else {
                    self.types.error()
                }
            }
            agam_ast::types::TypeExprKind::Inferred => self.types.fresh_var(),
            agam_ast::types::TypeExprKind::Dynamic | agam_ast::types::TypeExprKind::Any => {
                self.types.any()
            }
            agam_ast::types::TypeExprKind::Never => self.types.never(),
            agam_ast::types::TypeExprKind::Refined { base, .. } => self.resolve_type_expr(base),
            _ => self.types.fresh_var(),
        }
    }

    fn resolve_binary_expr_type(
        &self,
        op: HirBinOp,
        left: &HirExpr,
        right: &HirExpr,
    ) -> agam_sema::symbol::TypeId {
        match op {
            HirBinOp::Eq
            | HirBinOp::NotEq
            | HirBinOp::Lt
            | HirBinOp::LtEq
            | HirBinOp::Gt
            | HirBinOp::GtEq
            | HirBinOp::And
            | HirBinOp::Or => self.types.bool(),
            HirBinOp::Add if left.ty == self.types.str() || right.ty == self.types.str() => {
                self.types.str()
            }
            _ if left.ty == self.types.f64() || right.ty == self.types.f64() => self.types.f64(),
            _ => left.ty,
        }
    }

    fn resolve_unary_expr_type(
        &self,
        op: HirUnaryOp,
        operand: &HirExpr,
    ) -> agam_sema::symbol::TypeId {
        match op {
            HirUnaryOp::Not => self.types.bool(),
            _ => operand.ty,
        }
    }

    fn pattern_name(&self, pattern: &agam_ast::pattern::Pattern) -> Option<String> {
        match &pattern.kind {
            agam_ast::pattern::PatternKind::Identifier { name, .. } => Some(name.name.clone()),
            _ => None,
        }
    }
}

impl HirExprKind {
    /// Clone a Var name for compound assignment desugaring.
    fn clone_var_name(&self) -> HirExprKind {
        match self {
            HirExprKind::Var(name) => HirExprKind::Var(name.clone()),
            _ => HirExprKind::Tuple(vec![]),
        }
    }
}

fn lower_binop(op: BinOp) -> HirBinOp {
    match op {
        BinOp::Add => HirBinOp::Add,
        BinOp::Sub => HirBinOp::Sub,
        BinOp::Mul => HirBinOp::Mul,
        BinOp::Div => HirBinOp::Div,
        BinOp::Mod => HirBinOp::Mod,
        BinOp::Pow => HirBinOp::Pow,
        BinOp::Eq => HirBinOp::Eq,
        BinOp::NotEq => HirBinOp::NotEq,
        BinOp::Lt => HirBinOp::Lt,
        BinOp::LtEq => HirBinOp::LtEq,
        BinOp::Gt => HirBinOp::Gt,
        BinOp::GtEq => HirBinOp::GtEq,
        BinOp::And => HirBinOp::And,
        BinOp::Or => HirBinOp::Or,
        BinOp::BitAnd => HirBinOp::BitAnd,
        BinOp::BitOr => HirBinOp::BitOr,
        BinOp::BitXor => HirBinOp::BitXor,
        BinOp::Shl => HirBinOp::Shl,
        BinOp::Shr => HirBinOp::Shr,
    }
}

fn lower_unaryop(op: UnaryOp) -> HirUnaryOp {
    match op {
        UnaryOp::Neg => HirUnaryOp::Neg,
        UnaryOp::Not => HirUnaryOp::Not,
        UnaryOp::BitNot => HirUnaryOp::BitNot,
        UnaryOp::Ref => HirUnaryOp::Ref,
        UnaryOp::Deref => HirUnaryOp::Deref,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agam_errors::span::SourceId;
    use agam_lexer::Lexer;
    use agam_sema::types::TypeStore;

    fn lower_source(source: &str) -> HirModule {
        let source_id = SourceId(0);
        let mut lexer = Lexer::new(source, source_id);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            let is_eof = tok.kind == agam_lexer::TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        let mut parser = agam_parser::Parser::new(tokens);
        let module = parser.parse_module(source_id).expect("parse failed");

        let mut lowering = HirLowering::new();
        lowering.lower_module(&module)
    }

    #[test]
    fn test_lower_simple_function() {
        let hir = lower_source("fn main(): return 42");
        assert_eq!(hir.functions.len(), 1);
        assert_eq!(hir.functions[0].name, "main");
    }

    #[test]
    fn test_lower_let_binding() {
        let hir = lower_source("fn main(): let x = 42");
        let f = &hir.functions[0];
        assert!(!f.body.stmts.is_empty());
        match &f.body.stmts[0] {
            HirStmt::Let { name, mutable, .. } => {
                assert_eq!(name, "x");
                assert!(*mutable, "plain `let` should lower as mutable by default");
            }
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn test_lower_binary_expr() {
        let hir = lower_source("fn main(): let x = 1 + 2");
        let f = &hir.functions[0];
        match &f.body.stmts[0] {
            HirStmt::Let {
                value: Some(expr), ..
            } => match &expr.kind {
                HirExprKind::Binary { op, .. } => assert_eq!(*op, HirBinOp::Add),
                _ => panic!("expected Binary"),
            },
            _ => panic!("expected Let with value"),
        }
    }

    #[test]
    fn test_lower_function_call() {
        let hir = lower_source("fn main(): print(42)");
        let f = &hir.functions[0];
        match &f.body.stmts[0] {
            HirStmt::Expr(expr) => match &expr.kind {
                HirExprKind::Call { args, .. } => assert_eq!(args.len(), 1),
                _ => panic!("expected Call"),
            },
            _ => panic!("expected Expr"),
        }
    }

    #[test]
    fn test_lower_while() {
        let hir = lower_source("fn main(): while true: let x = 1");
        let f = &hir.functions[0];
        match &f.body.stmts[0] {
            HirStmt::While { condition, .. } => match &condition.kind {
                HirExprKind::BoolLit(true) => {}
                _ => panic!("expected BoolLit(true)"),
            },
            _ => panic!("expected While"),
        }
    }

    #[test]
    fn test_lower_for_initializes_iterator_before_loop() {
        let hir = lower_source("fn main(): for item in values: print(item)");
        let f = &hir.functions[0];
        match &f.body.stmts[0] {
            HirStmt::Expr(expr) => match &expr.kind {
                HirExprKind::Block(block) => {
                    assert_eq!(block.stmts.len(), 2);
                    match &block.stmts[0] {
                        HirStmt::Let {
                            name,
                            value: Some(iterable),
                            ..
                        } => {
                            assert!(name.starts_with("__iter_"));
                            match &iterable.kind {
                                HirExprKind::Var(var) => assert_eq!(var, "values"),
                                _ => panic!("expected iterator init to use iterable"),
                            }
                        }
                        _ => panic!("expected iterator let binding"),
                    }
                    assert!(matches!(&block.stmts[1], HirStmt::While { .. }));
                }
                _ => panic!("expected block expression"),
            },
            _ => panic!("expected expression statement"),
        }
    }

    #[test]
    fn test_lower_preserves_explicit_scalar_types() {
        let hir = lower_source("fn add(x: i64) -> i64: let y: i64 = x; return y");
        let builtins = TypeStore::new();
        let f = &hir.functions[0];
        assert_eq!(f.params[0].ty, builtins.i64());
        assert_eq!(f.return_ty, builtins.i64());
        match &f.body.stmts[0] {
            HirStmt::Let { ty, .. } => assert_eq!(*ty, builtins.i64()),
            _ => panic!("expected let binding"),
        }
    }

    #[test]
    fn test_lower_int_literals_default_to_i32() {
        let hir = lower_source("fn main(): let x = 42");
        let builtins = TypeStore::new();
        let f = &hir.functions[0];
        match &f.body.stmts[0] {
            HirStmt::Let {
                value: Some(expr), ..
            } => assert_eq!(expr.ty, builtins.i32()),
            _ => panic!("expected let binding with initializer"),
        }
    }

    #[test]
    fn test_lower_variable_use_preserves_binding_type() {
        let hir = lower_source("fn add(x: i64) -> i64: return x + 1");
        let builtins = TypeStore::new();
        let f = &hir.functions[0];
        match &f.body.stmts[0] {
            HirStmt::Return(Some(expr)) => match &expr.kind {
                HirExprKind::Binary { left, .. } => assert_eq!(left.ty, builtins.i64()),
                _ => panic!("expected binary return expression"),
            },
            _ => panic!("expected return"),
        }
    }
}
