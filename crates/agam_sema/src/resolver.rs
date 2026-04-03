//! Name resolution pass.
//!
//! Walks the AST and:
//! 1. Declares all names (functions, variables, structs, etc.) in the symbol table.
//! 2. Resolves all name references to their declarations.
//! 3. Reports errors for undeclared names and illegal redeclarations.
//!
//! This is the first semantic pass and runs before type inference.

use agam_ast::decl::*;
use agam_ast::expr::*;
use agam_ast::stmt::*;
use agam_ast::types::{TypeExpr, TypeExprKind};
use agam_ast::*;
use agam_errors::Span;

use crate::scope::ScopeStack;
use crate::symbol::{SymbolKind, TypeId};
use crate::types::TypeStore;

/// Errors produced during name resolution.
#[derive(Debug, Clone)]
pub struct ResolveError {
    pub message: String,
    pub span: Span,
}

/// The name resolver walks the AST and populates the scope stack.
pub struct Resolver {
    pub scopes: ScopeStack,
    pub types: TypeStore,
    pub errors: Vec<ResolveError>,
}

impl Resolver {
    pub fn new() -> Self {
        let mut resolver = Self {
            scopes: ScopeStack::new(),
            types: TypeStore::new(),
            errors: Vec::new(),
        };
        resolver.declare_builtin_functions();
        resolver
    }

    /// Run name resolution on a parsed module.
    pub fn resolve_module(&mut self, module: &Module) {
        // First pass: declare all top-level names (so functions can call each other).
        for decl in &module.declarations {
            self.declare_top_level(decl);
        }
        // Second pass: resolve bodies (expressions, statements).
        for decl in &module.declarations {
            self.resolve_decl(decl);
        }
    }

    // ── Top-level declaration (forward-declare names) ──

    fn declare_top_level(&mut self, decl: &Decl) {
        match &decl.kind {
            DeclKind::Function(f) => {
                let ret_ty = self.types.fresh_var();
                let param_tys: Vec<TypeId> =
                    f.params.iter().map(|_| self.types.fresh_var()).collect();
                if let Err(prev) = self.scopes.declare(
                    f.name.name.clone(),
                    SymbolKind::Function {
                        params: param_tys,
                        return_ty: ret_ty,
                        is_async: f.is_async,
                    },
                    f.span,
                ) {
                    let prev_sym = self.scopes.get(prev);
                    self.errors.push(ResolveError {
                        message: format!(
                            "'{}' is already declared (previously at {:?})",
                            f.name.name, prev_sym.span
                        ),
                        span: f.span,
                    });
                }
            }
            DeclKind::Struct(s) => {
                let fields: Vec<(String, TypeId)> = s
                    .fields
                    .iter()
                    .map(|f| (f.name.name.clone(), self.types.fresh_var()))
                    .collect();
                let _ =
                    self.scopes
                        .declare(s.name.name.clone(), SymbolKind::Struct { fields }, s.span);
            }
            DeclKind::Enum(e) => {
                let variants = e.variants.iter().map(|v| v.name.name.clone()).collect();
                let _ =
                    self.scopes
                        .declare(e.name.name.clone(), SymbolKind::Enum { variants }, e.span);
            }
            DeclKind::Trait(t) => {
                let methods = t
                    .items
                    .iter()
                    .filter_map(|item| match item {
                        TraitItem::Method(f) => Some(f.name.name.clone()),
                        _ => None,
                    })
                    .collect();
                let _ =
                    self.scopes
                        .declare(t.name.name.clone(), SymbolKind::Trait { methods }, t.span);
            }
            DeclKind::Module(m) => {
                let _ = self
                    .scopes
                    .declare(m.name.name.clone(), SymbolKind::Module, m.span);
            }
            DeclKind::TypeAlias { name, ty, .. } => {
                let target = self.types.fresh_var();
                let _ = self.scopes.declare(
                    name.name.clone(),
                    SymbolKind::TypeAlias { target },
                    ty.span,
                );
            }
            // Use/Import and Impl handled elsewhere
            _ => {}
        }
    }

    // ── Resolve declarations (walk bodies) ──

    fn resolve_decl(&mut self, decl: &Decl) {
        match &decl.kind {
            DeclKind::Function(f) => self.resolve_function(f),
            DeclKind::Struct(s) => self.resolve_struct(s),
            DeclKind::Impl(imp) => self.resolve_impl(imp),
            _ => {}
        }
    }

    fn resolve_function(&mut self, f: &FunctionDecl) {
        self.scopes.push_scope();

        // Declare parameters
        for param in &f.params {
            let ty = self.resolve_type_expr_to_id(&param.ty);
            if let Some(name) = self.pattern_name(&param.pattern) {
                let _ = self.scopes.declare(
                    name,
                    SymbolKind::Variable { mutable: true, ty },
                    param.span,
                );
            }
        }

        // Resolve body
        if let Some(body) = &f.body {
            self.resolve_block(body);
        }

        self.scopes.pop_scope();
    }

    fn resolve_struct(&mut self, s: &StructDecl) {
        // Resolve field default expressions
        for field in &s.fields {
            if let Some(default) = &field.default {
                self.resolve_expr(default);
            }
        }
    }

    fn resolve_impl(&mut self, imp: &ImplDecl) {
        for item in &imp.items {
            self.resolve_decl(item);
        }
    }

    // ── Statements ──

    fn resolve_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.resolve_stmt(stmt);
        }
        if let Some(expr) = &block.expr {
            self.resolve_expr(expr);
        }
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Let {
                pattern,
                ty,
                value,
                mutable,
            } => {
                // Resolve initializer first (before declaring the name).
                if let Some(val) = value {
                    self.resolve_expr(val);
                }
                let resolved_ty = if let Some(t) = ty {
                    self.resolve_type_expr_to_id(t)
                } else {
                    self.types.fresh_var()
                };
                if let Some(name) = self.pattern_name(pattern) {
                    if let Err(prev) = self.scopes.declare(
                        name.clone(),
                        SymbolKind::Variable {
                            mutable: *mutable,
                            ty: resolved_ty,
                        },
                        stmt.span,
                    ) {
                        let prev_sym = self.scopes.get(prev);
                        self.errors.push(ResolveError {
                            message: format!(
                                "'{}' is already declared in this scope (previously at {:?})",
                                name, prev_sym.span
                            ),
                            span: stmt.span,
                        });
                    }
                }
            }
            StmtKind::Const { name, ty, value } => {
                self.resolve_expr(value);
                let resolved_ty = if let Some(t) = ty {
                    self.resolve_type_expr_to_id(t)
                } else {
                    self.types.fresh_var()
                };
                let _ = self.scopes.declare(
                    name.name.clone(),
                    SymbolKind::Constant { ty: resolved_ty },
                    stmt.span,
                );
            }
            StmtKind::Expression(expr) => {
                self.resolve_expr(expr);
            }
            StmtKind::Return(val) | StmtKind::Break(val) | StmtKind::Yield(val) => {
                if let Some(e) = val {
                    self.resolve_expr(e);
                }
            }
            StmtKind::Continue => {}
            StmtKind::While { condition, body } => {
                self.resolve_expr(condition);
                self.scopes.push_scope();
                self.resolve_block(body);
                self.scopes.pop_scope();
            }
            StmtKind::Loop { body } => {
                self.scopes.push_scope();
                self.resolve_block(body);
                self.scopes.pop_scope();
            }
            StmtKind::For {
                pattern,
                iterable,
                body,
            } => {
                self.resolve_expr(iterable);
                self.scopes.push_scope();
                if let Some(name) = self.pattern_name(pattern) {
                    let _ = self.scopes.declare(
                        name,
                        SymbolKind::Variable {
                            mutable: true,
                            ty: self.types.fresh_var(),
                        },
                        stmt.span,
                    );
                }
                self.resolve_block(body);
                self.scopes.pop_scope();
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.resolve_expr(condition);
                self.scopes.push_scope();
                self.resolve_block(then_branch);
                self.scopes.pop_scope();
                if let Some(eb) = else_branch {
                    match eb {
                        ElseBranch::Else(block) => {
                            self.scopes.push_scope();
                            self.resolve_block(block);
                            self.scopes.pop_scope();
                        }
                        ElseBranch::ElseIf(stmt) => {
                            self.resolve_stmt(stmt);
                        }
                    }
                }
            }
            StmtKind::Match { scrutinee, arms } => {
                self.resolve_expr(scrutinee);
                for arm in arms {
                    self.scopes.push_scope();
                    // Declare pattern bindings
                    if let Some(name) = self.pattern_name(&arm.pattern) {
                        let _ = self.scopes.declare(
                            name,
                            SymbolKind::Variable {
                                mutable: false,
                                ty: self.types.fresh_var(),
                            },
                            arm.span,
                        );
                    }
                    if let Some(guard) = &arm.guard {
                        self.resolve_expr(guard);
                    }
                    self.resolve_expr(&arm.body);
                    self.scopes.pop_scope();
                }
            }
            StmtKind::TryCatch { body, catches } => {
                self.scopes.push_scope();
                self.resolve_block(body);
                self.scopes.pop_scope();
                for catch in catches {
                    self.scopes.push_scope();
                    if let Some(binding) = &catch.binding {
                        let _ = self.scopes.declare(
                            binding.name.clone(),
                            SymbolKind::Variable {
                                mutable: false,
                                ty: self.types.fresh_var(),
                            },
                            catch.span,
                        );
                    }
                    self.resolve_block(&catch.body);
                    self.scopes.pop_scope();
                }
            }
            StmtKind::Throw(expr) => {
                self.resolve_expr(expr);
            }
            StmtKind::Declaration(decl) => {
                self.declare_top_level(decl);
                self.resolve_decl(decl);
            }
        }
    }

    // ── Expressions ──

    fn resolve_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Identifier(ident) => {
                if let Some(id) = self.scopes.lookup(&ident.name) {
                    self.scopes.mark_used(id);
                } else {
                    self.errors.push(ResolveError {
                        message: format!("undeclared identifier '{}'", ident.name),
                        span: expr.span,
                    });
                }
            }
            ExprKind::PathExpr(path) => {
                if let Some(first) = path.segments.first() {
                    if let Some(id) = self.scopes.lookup(&first.name) {
                        self.scopes.mark_used(id);
                    }
                    // Multi-segment paths resolved in later passes (module system)
                }
            }
            ExprKind::Binary { left, right, .. } => {
                self.resolve_expr(left);
                self.resolve_expr(right);
            }
            ExprKind::Unary { operand, .. } => {
                self.resolve_expr(operand);
            }
            ExprKind::Call { callee, args } => {
                self.resolve_expr(callee);
                for arg in args {
                    self.resolve_expr(arg);
                }
            }
            ExprKind::MethodCall { object, args, .. } => {
                self.resolve_expr(object);
                for arg in args {
                    self.resolve_expr(arg);
                }
            }
            ExprKind::FieldAccess { object, .. } => {
                self.resolve_expr(object);
            }
            ExprKind::Index { object, index } => {
                self.resolve_expr(object);
                self.resolve_expr(index);
            }
            ExprKind::Assign { target, value } => {
                self.resolve_expr(target);
                self.resolve_expr(value);
            }
            ExprKind::CompoundAssign { target, value, .. } => {
                self.resolve_expr(target);
                self.resolve_expr(value);
            }
            ExprKind::ArrayLiteral(elems) | ExprKind::TupleLiteral(elems) => {
                for e in elems {
                    self.resolve_expr(e);
                }
            }
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.resolve_expr(condition);
                self.resolve_expr(then_branch);
                if let Some(eb) = else_branch {
                    self.resolve_expr(eb);
                }
            }
            ExprKind::Block(block) => {
                self.scopes.push_scope();
                self.resolve_block(block);
                self.scopes.pop_scope();
            }
            ExprKind::Lambda { params, body, .. } => {
                self.scopes.push_scope();
                for p in params {
                    let _ = self.scopes.declare(
                        p.name.name.clone(),
                        SymbolKind::Variable {
                            mutable: false,
                            ty: self.types.fresh_var(),
                        },
                        p.span,
                    );
                }
                self.resolve_expr(body);
                self.scopes.pop_scope();
            }
            ExprKind::Await(inner) | ExprKind::Spawn(inner) | ExprKind::Try(inner) => {
                self.resolve_expr(inner);
            }
            ExprKind::Cast { expr: inner, .. } => {
                self.resolve_expr(inner);
            }
            ExprKind::Range { start, end, .. } => {
                if let Some(s) = start {
                    self.resolve_expr(s);
                }
                if let Some(e) = end {
                    self.resolve_expr(e);
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                self.resolve_expr(scrutinee);
                for arm in arms {
                    self.scopes.push_scope();
                    if let Some(name) = self.pattern_name(&arm.pattern) {
                        let _ = self.scopes.declare(
                            name,
                            SymbolKind::Variable {
                                mutable: false,
                                ty: self.types.fresh_var(),
                            },
                            arm.span,
                        );
                    }
                    if let Some(guard) = &arm.guard {
                        self.resolve_expr(guard);
                    }
                    self.resolve_expr(&arm.body);
                    self.scopes.pop_scope();
                }
            }
            ExprKind::StructLiteral { fields, .. } => {
                for f in fields {
                    self.resolve_expr(&f.value);
                }
            }
            ExprKind::FStringLiteral { parts } => {
                for part in parts {
                    if let FStringPart::Expr(e) = part {
                        self.resolve_expr(e);
                    }
                }
            }
            // Differentiable programming
            ExprKind::Grad { func, .. } => {
                self.resolve_expr(func);
            }
            ExprKind::Backward(inner) => {
                self.resolve_expr(inner);
            }
            ExprKind::Resume(inner) => {
                self.resolve_expr(inner);
            }
            ExprKind::BlockExpr(block) => {
                for stmt in &block.stmts {
                    self.resolve_stmt(stmt);
                }
            }
            // Literals don't need resolution
            ExprKind::IntLiteral(_)
            | ExprKind::FloatLiteral(_)
            | ExprKind::StringLiteral(_)
            | ExprKind::BoolLiteral(_) => {}
        }
    }

    // ── Helpers ──

    /// Map an AST `TypeExpr` → an internal `TypeId`.
    fn resolve_type_expr_to_id(&mut self, te: &TypeExpr) -> TypeId {
        use crate::types::{Type, builtin_type_id_for_name};

        match &te.kind {
            TypeExprKind::Named(path) => {
                if let Some(seg) = path.segments.last() {
                    if let Some(type_id) = builtin_type_id_for_name(&self.types, &seg.name) {
                        type_id
                    } else {
                        let name = seg.name.as_str();
                        // User-defined type — look it up
                        if let Some(sym_id) = self.scopes.lookup(name) {
                            self.scopes.mark_used(sym_id);
                            self.types.insert(Type::Named(sym_id))
                        } else {
                            self.errors.push(ResolveError {
                                message: format!("unknown type '{}'", name),
                                span: te.span,
                            });
                            self.types.error()
                        }
                    }
                } else {
                    self.types.error()
                }
            }
            TypeExprKind::Inferred => self.types.fresh_var(),
            TypeExprKind::Dynamic | TypeExprKind::Any => self.types.any(),
            TypeExprKind::Never => self.types.never(),
            TypeExprKind::SelfType => self.types.fresh_var(), // resolved during trait/impl checking
            TypeExprKind::Reference { mutable, inner } => {
                let inner_id = self.resolve_type_expr_to_id(inner);
                self.types.insert(Type::Ref {
                    mutable: *mutable,
                    inner: inner_id,
                })
            }
            TypeExprKind::Pointer { mutable, inner } => {
                let inner_id = self.resolve_type_expr_to_id(inner);
                self.types.insert(Type::Ptr {
                    mutable: *mutable,
                    inner: inner_id,
                })
            }
            TypeExprKind::Optional(inner) => {
                let inner_id = self.resolve_type_expr_to_id(inner);
                self.types.insert(Type::Optional(inner_id))
            }
            TypeExprKind::Tuple(elems) => {
                let ids: Vec<TypeId> = elems
                    .iter()
                    .map(|e| self.resolve_type_expr_to_id(e))
                    .collect();
                self.types.insert(Type::Tuple(ids))
            }
            TypeExprKind::Slice(inner) => {
                let inner_id = self.resolve_type_expr_to_id(inner);
                self.types.insert(Type::Slice(inner_id))
            }
            TypeExprKind::Function {
                params,
                return_type,
            } => {
                let param_ids: Vec<TypeId> = params
                    .iter()
                    .map(|p| self.resolve_type_expr_to_id(p))
                    .collect();
                let ret_id = self.resolve_type_expr_to_id(return_type);
                self.types.insert(Type::Function {
                    params: param_ids,
                    ret: ret_id,
                })
            }
            TypeExprKind::Refined { base, .. } => {
                // Return the base type ID while type system learns refinement predicates
                self.resolve_type_expr_to_id(base)
            }
            _ => self.types.fresh_var(),
        }
    }

    /// Extract the simple name from a pattern (for binding purposes).
    fn pattern_name(&self, pattern: &agam_ast::pattern::Pattern) -> Option<String> {
        match &pattern.kind {
            agam_ast::pattern::PatternKind::Identifier { name, .. } => Some(name.name.clone()),
            _ => None, // Destructuring patterns handled in a later pass
        }
    }

    fn declare_builtin_functions(&mut self) {
        for (name, params, return_ty) in self.builtin_function_signatures() {
            let _ = self.scopes.declare(
                name.to_string(),
                SymbolKind::Function {
                    params,
                    return_ty,
                    is_async: false,
                },
                Span::dummy(),
            );
        }
    }

    fn builtin_function_signatures(&self) -> Vec<(&'static str, Vec<TypeId>, TypeId)> {
        let any = self.types.any();
        let bool_ty = self.types.bool();
        let float_ty = self.types.f64();
        let int_ty = self.types.i64();
        let str_ty = self.types.str();
        let unit_ty = self.types.unit();

        vec![
            ("print", vec![any], unit_ty),
            ("println", vec![any], unit_ty),
            ("print_int", vec![int_ty], unit_ty),
            ("print_str", vec![str_ty], unit_ty),
            ("argc", Vec::new(), int_ty),
            ("argv", vec![int_ty], str_ty),
            ("parse_int", vec![str_ty], int_ty),
            ("clock", Vec::new(), float_ty),
            ("adam", vec![any], float_ty),
            ("dataframe_mean", vec![any], float_ty),
            ("tensor_checksum", vec![any], float_ty),
            ("dataframe_build_sin", vec![any], any),
            ("dataframe_filter_gt", vec![any], any),
            ("dataframe_sort", vec![any], any),
            ("dataframe_group_by", vec![any], any),
            ("tensor_fill_rand", vec![any], any),
            ("dense_layer", vec![any], any),
            ("conv2d", vec![any], any),
            ("dataframe_free", vec![any], unit_ty),
            ("tensor_free", vec![any], unit_ty),
            ("len", vec![any], int_ty),
            ("has_next", vec![any], bool_ty),
            ("next", vec![any], any),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agam_errors::span::SourceId;
    use agam_lexer::Lexer;

    fn parse_and_resolve(source: &str) -> Resolver {
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

        let mut resolver = Resolver::new();
        resolver.resolve_module(&module);
        resolver
    }

    #[test]
    fn test_resolve_simple_function() {
        let r = parse_and_resolve("fn main(): let x = 42");
        assert!(r.errors.is_empty(), "errors: {:?}", r.errors);
        assert!(r.scopes.lookup("main").is_some());
    }

    #[test]
    fn test_undeclared_variable() {
        let r = parse_and_resolve("fn main(): y");
        assert!(!r.errors.is_empty());
        assert!(r.errors.iter().any(|e| e.message.contains("undeclared")));
    }

    #[test]
    fn test_function_params_in_scope() {
        let r = parse_and_resolve("fn add(a: i32, b: i32): return a");
        // Should resolve `a` without error
        let undeclared: Vec<_> = r
            .errors
            .iter()
            .filter(|e| e.message.contains("undeclared"))
            .collect();
        assert!(undeclared.is_empty(), "unexpected errors: {:?}", undeclared);
    }

    #[test]
    fn test_for_loop_variable() {
        let r = parse_and_resolve(
            r#"
fn main():
    let items = [1, 2, 3]
    for item in items:
        item
"#,
        );
        let undeclared: Vec<_> = r
            .errors
            .iter()
            .filter(|e| e.message.contains("undeclared"))
            .collect();
        assert!(undeclared.is_empty(), "unexpected errors: {:?}", undeclared);
    }
}
