//! Visitor trait for traversing the AST.
//!
//! Implements the Visitor pattern so that compiler passes (type checking,
//! lowering, optimization) can traverse the tree without matching every node.

use crate::expr::*;
use crate::stmt::*;
use crate::decl::*;
use crate::types::*;
use crate::pattern::*;
use crate::Module;

/// A visitor that walks the AST and can return a result for each node type.
///
/// Default implementations walk into child nodes. Override specific methods
/// to intercept particular node types.
#[allow(unused_variables)]
pub trait Visitor {
    type Result;

    fn default_result(&self) -> Self::Result;

    // ── Module ──
    fn visit_module(&mut self, module: &Module) -> Self::Result {
        for decl in &module.declarations {
            self.visit_decl(decl);
        }
        self.default_result()
    }

    // ── Declarations ──
    fn visit_decl(&mut self, decl: &Decl) -> Self::Result {
        match &decl.kind {
            DeclKind::Function(f) => self.visit_function(f),
            DeclKind::Struct(s) => self.visit_struct(s),
            DeclKind::Enum(e) => self.visit_enum(e),
            DeclKind::Trait(t) => self.visit_trait(t),
            DeclKind::Impl(i) => self.visit_impl(i),
            DeclKind::Module(m) => self.visit_module_decl(m),
            DeclKind::Use(u) => self.visit_use(u),
            DeclKind::TypeAlias { ty, .. } => self.visit_type_expr(ty),
            DeclKind::Effect(_) => self.default_result(),
            DeclKind::Handler(_) => self.default_result(),
        }
    }

    fn visit_function(&mut self, func: &FunctionDecl) -> Self::Result {
        if let Some(body) = &func.body {
            self.visit_block(body);
        }
        self.default_result()
    }

    fn visit_struct(&mut self, s: &StructDecl) -> Self::Result {
        self.default_result()
    }

    fn visit_enum(&mut self, e: &EnumDecl) -> Self::Result {
        self.default_result()
    }

    fn visit_trait(&mut self, t: &TraitDecl) -> Self::Result {
        self.default_result()
    }

    fn visit_impl(&mut self, i: &ImplDecl) -> Self::Result {
        for item in &i.items {
            self.visit_decl(item);
        }
        self.default_result()
    }

    fn visit_module_decl(&mut self, m: &ModuleDecl) -> Self::Result {
        if let Some(body) = &m.body {
            for decl in body {
                self.visit_decl(decl);
            }
        }
        self.default_result()
    }

    fn visit_use(&mut self, u: &UseDecl) -> Self::Result {
        self.default_result()
    }

    // ── Statements ──
    fn visit_stmt(&mut self, stmt: &Stmt) -> Self::Result {
        match &stmt.kind {
            StmtKind::Let { value, .. } => {
                if let Some(v) = value {
                    self.visit_expr(v);
                }
            }
            StmtKind::Const { value, .. } => {
                self.visit_expr(value);
            }
            StmtKind::Expression(expr) => {
                self.visit_expr(expr);
            }
            StmtKind::Return(expr) => {
                if let Some(e) = expr {
                    self.visit_expr(e);
                }
            }
            StmtKind::While { condition, body, .. } => {
                self.visit_expr(condition);
                self.visit_block(body);
            }
            StmtKind::For { iterable, body, .. } => {
                self.visit_expr(iterable);
                self.visit_block(body);
            }
            StmtKind::Loop { body } => {
                self.visit_block(body);
            }
            StmtKind::If { condition, then_branch, .. } => {
                self.visit_expr(condition);
                self.visit_block(then_branch);
            }
            StmtKind::Declaration(decl) => {
                self.visit_decl(decl);
            }
            _ => {}
        }
        self.default_result()
    }

    // ── Expressions ──
    fn visit_expr(&mut self, expr: &Expr) -> Self::Result {
        match &expr.kind {
            ExprKind::Binary { left, right, .. } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }
            ExprKind::Unary { operand, .. } => {
                self.visit_expr(operand);
            }
            ExprKind::Call { callee, args, .. } => {
                self.visit_expr(callee);
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            ExprKind::FieldAccess { object, .. } => {
                self.visit_expr(object);
            }
            ExprKind::Index { object, index } => {
                self.visit_expr(object);
                self.visit_expr(index);
            }
            ExprKind::MethodCall { object, args, .. } => {
                self.visit_expr(object);
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            ExprKind::Block(block) => {
                self.visit_block(block);
            }
            ExprKind::If { condition, then_branch, else_branch } => {
                self.visit_expr(condition);
                self.visit_expr(then_branch);
                if let Some(e) = else_branch {
                    self.visit_expr(e);
                }
            }
            ExprKind::Lambda { body, .. } => {
                self.visit_expr(body);
            }
            ExprKind::Await(inner) | ExprKind::Spawn(inner) | ExprKind::Try(inner) => {
                self.visit_expr(inner);
            }
            ExprKind::Assign { target, value } | ExprKind::CompoundAssign { target, value, .. } => {
                self.visit_expr(target);
                self.visit_expr(value);
            }
            ExprKind::ArrayLiteral(elems) | ExprKind::TupleLiteral(elems) => {
                for elem in elems {
                    self.visit_expr(elem);
                }
            }
            _ => {}
        }
        self.default_result()
    }

    fn visit_block(&mut self, block: &Block) -> Self::Result {
        for stmt in &block.stmts {
            self.visit_stmt(stmt);
        }
        if let Some(expr) = &block.expr {
            self.visit_expr(expr);
        }
        self.default_result()
    }

    // ── Types ──
    fn visit_type_expr(&mut self, ty: &TypeExpr) -> Self::Result {
        self.default_result()
    }

    // ── Patterns ──
    fn visit_pattern(&mut self, pattern: &Pattern) -> Self::Result {
        self.default_result()
    }
}
