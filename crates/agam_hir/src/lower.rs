//! AST → HIR lowering pass.
//!
//! Transforms the parsed AST into the HIR by:
//! - Desugaring for-in loops into while loops.
//! - Desugaring f-strings into string concatenation.
//! - Attaching resolved type information.
//! - Flattening nested declarations.

use agam_ast::*;
use agam_ast::decl::*;
use agam_ast::stmt::*;
use agam_ast::expr::*;
use agam_sema::types::TypeStore;

use crate::nodes::*;

/// The HIR lowering context.
pub struct HirLowering {
    next_id: u32,
    types: TypeStore,
}

impl HirLowering {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            types: TypeStore::new(),
        }
    }

    fn fresh_id(&mut self) -> HirId {
        let id = HirId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Lower a parsed AST module into HIR.
    pub fn lower_module(&mut self, module: &Module) -> HirModule {
        let functions = module.declarations.iter()
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
        let params: Vec<HirParam> = f.params.iter().map(|p| {
            let name = self.pattern_name(&p.pattern).unwrap_or_else(|| "_".into());
            HirParam {
                name,
                ty: self.types.fresh_var(),
                mutable: false,
            }
        }).collect();

        let body = if let Some(b) = &f.body {
            self.lower_block(b)
        } else {
            HirBlock { stmts: vec![], expr: None }
        };

        HirFunction {
            id: self.fresh_id(),
            name: f.name.name.clone(),
            params,
            return_ty: self.types.fresh_var(),
            body,
            is_async: f.is_async,
        }
    }

    fn lower_block(&mut self, block: &Block) -> HirBlock {
        let stmts = block.stmts.iter().map(|s| self.lower_stmt(s)).collect();
        let expr = block.expr.as_ref().map(|e| Box::new(self.lower_expr(e)));
        HirBlock { stmts, expr }
    }

    fn lower_stmt(&mut self, stmt: &Stmt) -> HirStmt {
        match &stmt.kind {
            StmtKind::Let { pattern, ty: _, value, mutable } => {
                let name = self.pattern_name(pattern).unwrap_or_else(|| "_".into());
                HirStmt::Let {
                    name,
                    ty: self.types.fresh_var(),
                    value: value.as_ref().map(|v| self.lower_expr(v)),
                    mutable: *mutable,
                }
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
            StmtKind::For { pattern, iterable: _, body } => {
                let iter_name = format!("__iter_{}", self.next_id);
                let item_name = self.pattern_name(pattern).unwrap_or_else(|| "_".into());

                // Desugar: let __iter = iterable; while __iter.has_next(): let item = __iter.next(); body
                let _iter_var = HirExpr {
                    id: self.fresh_id(),
                    ty: self.types.fresh_var(),
                    kind: HirExprKind::Var(iter_name.clone()),
                };

                let has_next = HirExpr {
                    id: self.fresh_id(),
                    ty: self.types.bool(),
                    kind: HirExprKind::MethodCall {
                        object: Box::new(HirExpr {
                            id: self.fresh_id(),
                            ty: self.types.fresh_var(),
                            kind: HirExprKind::Var(iter_name.clone()),
                        }),
                        method: "has_next".into(),
                        args: vec![],
                    },
                };

                let next_call = HirExpr {
                    id: self.fresh_id(),
                    ty: self.types.fresh_var(),
                    kind: HirExprKind::MethodCall {
                        object: Box::new(HirExpr {
                            id: self.fresh_id(),
                            ty: self.types.fresh_var(),
                            kind: HirExprKind::Var(iter_name.clone()),
                        }),
                        method: "next".into(),
                        args: vec![],
                    },
                };

                let mut loop_stmts = vec![
                    HirStmt::Let {
                        name: item_name,
                        ty: self.types.fresh_var(),
                        value: Some(next_call),
                        mutable: false,
                    },
                ];
                let inner_block = self.lower_block(body);
                loop_stmts.extend(inner_block.stmts);

                HirStmt::While {
                    condition: has_next,
                    body: HirBlock { stmts: loop_stmts, expr: inner_block.expr },
                }
            }
            StmtKind::If { condition, then_branch, else_branch } => {
                let else_block = else_branch.as_ref().map(|eb| match eb {
                    ElseBranch::Else(block) => self.lower_block(block),
                    ElseBranch::ElseIf(s) => {
                        HirBlock {
                            stmts: vec![self.lower_stmt(s)],
                            expr: None,
                        }
                    }
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
                HirStmt::Let {
                    name: name.name.clone(),
                    ty: self.types.fresh_var(),
                    value: Some(self.lower_expr(value)),
                    mutable: false,
                }
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
        let ty = self.types.fresh_var();

        let kind = match &expr.kind {
            ExprKind::IntLiteral(v) => HirExprKind::IntLit(*v),
            ExprKind::FloatLiteral(v) => HirExprKind::FloatLit(*v),
            ExprKind::BoolLiteral(v) => HirExprKind::BoolLit(*v),
            ExprKind::StringLiteral(v) => HirExprKind::StringLit(v.clone()),

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

            ExprKind::Identifier(ident) => HirExprKind::Var(ident.name.clone()),
            ExprKind::PathExpr(path) => {
                let full = path.segments.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join("::");
                HirExprKind::Var(full)
            }

            ExprKind::Binary { op, left, right } => HirExprKind::Binary {
                op: lower_binop(*op),
                left: Box::new(self.lower_expr(left)),
                right: Box::new(self.lower_expr(right)),
            },
            ExprKind::Unary { op, operand } => HirExprKind::Unary {
                op: lower_unaryop(*op),
                operand: Box::new(self.lower_expr(operand)),
            },

            ExprKind::Call { callee, args } => HirExprKind::Call {
                callee: Box::new(self.lower_expr(callee)),
                args: args.iter().map(|a| self.lower_expr(a)).collect(),
            },
            ExprKind::MethodCall { object, method, args } => HirExprKind::MethodCall {
                object: Box::new(self.lower_expr(object)),
                method: method.name.clone(),
                args: args.iter().map(|a| self.lower_expr(a)).collect(),
            },

            ExprKind::FieldAccess { object, field } => HirExprKind::FieldAccess {
                object: Box::new(self.lower_expr(object)),
                field: field.name.clone(),
            },
            ExprKind::Index { object, index } => HirExprKind::Index {
                object: Box::new(self.lower_expr(object)),
                index: Box::new(self.lower_expr(index)),
            },

            ExprKind::Assign { target, value } => HirExprKind::Assign {
                target: Box::new(self.lower_expr(target)),
                value: Box::new(self.lower_expr(value)),
            },
            ExprKind::CompoundAssign { op, target, value } => {
                // Desugar: x += 1 → x = x + 1
                let target_hir = self.lower_expr(target);
                let val_hir = self.lower_expr(value);
                let binop = HirExpr {
                    id: self.fresh_id(),
                    ty: self.types.fresh_var(),
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
                HirExprKind::Assign {
                    target: Box::new(target_hir),
                    value: Box::new(binop),
                }
            }

            ExprKind::ArrayLiteral(elems) => {
                HirExprKind::Array(elems.iter().map(|e| self.lower_expr(e)).collect())
            }
            ExprKind::TupleLiteral(elems) => {
                HirExprKind::Tuple(elems.iter().map(|e| self.lower_expr(e)).collect())
            }

            ExprKind::BlockExpr(block) => HirExprKind::Block(self.lower_block(block)),

            ExprKind::Cast { expr: inner, .. } => HirExprKind::Cast {
                expr: Box::new(self.lower_expr(inner)),
                target_ty: self.types.fresh_var(),
            },

            // Fallback for unhandled expressions
            _ => HirExprKind::Tuple(vec![]), // Unit value
        };

        HirExpr { id, ty, kind }
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
    use agam_lexer::Lexer;
    use agam_errors::span::SourceId;

    fn lower_source(source: &str) -> HirModule {
        let source_id = SourceId(0);
        let mut lexer = Lexer::new(source, source_id);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            let is_eof = tok.kind == agam_lexer::TokenKind::Eof;
            tokens.push(tok);
            if is_eof { break; }
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
            HirStmt::Let { name, .. } => assert_eq!(name, "x"),
            _ => panic!("expected Let"),
        }
    }

    #[test]
    fn test_lower_binary_expr() {
        let hir = lower_source("fn main(): let x = 1 + 2");
        let f = &hir.functions[0];
        match &f.body.stmts[0] {
            HirStmt::Let { value: Some(expr), .. } => {
                match &expr.kind {
                    HirExprKind::Binary { op, .. } => assert_eq!(*op, HirBinOp::Add),
                    _ => panic!("expected Binary"),
                }
            }
            _ => panic!("expected Let with value"),
        }
    }

    #[test]
    fn test_lower_function_call() {
        let hir = lower_source("fn main(): print(42)");
        let f = &hir.functions[0];
        match &f.body.stmts[0] {
            HirStmt::Expr(expr) => {
                match &expr.kind {
                    HirExprKind::Call { args, .. } => assert_eq!(args.len(), 1),
                    _ => panic!("expected Call"),
                }
            }
            _ => panic!("expected Expr"),
        }
    }

    #[test]
    fn test_lower_while() {
        let hir = lower_source("fn main(): while true: let x = 1");
        let f = &hir.functions[0];
        match &f.body.stmts[0] {
            HirStmt::While { condition, .. } => {
                match &condition.kind {
                    HirExprKind::BoolLit(true) => {}
                    _ => panic!("expected BoolLit(true)"),
                }
            }
            _ => panic!("expected While"),
        }
    }
}
