//! Type checker — walks the AST and generates type constraints.
//!
//! This pass runs after name resolution. It:
//! 1. Assigns type variables to all expressions and bindings.
//! 2. Generates constraints based on how values are used.
//! 3. Delegates constraint solving to the `InferenceEngine`.
//! 4. Reports type errors to the user.

use agam_ast::decl::*;
use agam_ast::expr::*;
use agam_ast::stmt::*;
use agam_ast::types::{TypeExpr, TypeExprKind};
use agam_ast::*;
use agam_errors::Span;

use crate::consteval::ConstEvaluator;
use crate::gpu::{resolve_gpu_builtin_expr, resolve_gpu_builtin_member};
use crate::infer::InferenceEngine;
use crate::resolver::Resolver;
use crate::scope::ScopeStack;
use crate::symbol::TypeId;
use crate::types::{Type, TypeStore, builtin_type_id_for_name};
use agam_smt::solver::{Constraint, SmtSolver, SolverResult, Z3Solver};
use agam_smt::verify::{VerificationCache, VerificationStatus};

/// A type error reported to the user.
#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub span: Span,
}

/// The type checker: generates constraints and solves them.
pub struct TypeChecker {
    pub types: TypeStore,
    pub scopes: ScopeStack,
    pub engine: InferenceEngine,
    pub errors: Vec<TypeError>,
    pub smt_cache: VerificationCache,
}

impl TypeChecker {
    /// Create a type checker from an already-resolved module.
    pub fn from_resolver(resolver: Resolver) -> Self {
        let capacity = 128;
        Self {
            types: resolver.types,
            scopes: resolver.scopes,
            engine: InferenceEngine::new(capacity),
            errors: Vec::new(),
            smt_cache: VerificationCache::new(),
        }
    }

    /// Run type checking on a module.
    pub fn check_module(&mut self, module: &Module) {
        for decl in &module.declarations {
            self.check_decl(decl);
        }
        // Solve all accumulated constraints.
        self.engine.solve(&self.types);
        // Convert inference errors to type errors.
        for err in &self.engine.errors {
            self.errors.push(TypeError {
                message: err.message.clone(),
                span: Span::dummy(),
            });
        }
    }

    // ── Declarations ──

    fn check_decl(&mut self, decl: &Decl) {
        match &decl.kind {
            DeclKind::Function(f) => {
                // Check if already verified
                if let Some(VerificationStatus::VerifiedSafe) = self.smt_cache.get_status(decl.id) {
                    // Skip SMT checking if unchanged
                } else {
                    // Basic SMT verification pass
                    let mut solver = Z3Solver::new();
                    // (In a full implementation, we'd walk the body and assert preconditions/path conditions)
                    // Mock verification: prove basic bounds
                    solver.declare_int("v");
                    solver.push();
                    // Assume v != 0 locally
                    solver.assert(Constraint::NotEq(
                        Box::new(Constraint::Var("v".to_string())),
                        Box::new(Constraint::Int(0)),
                    ));

                    let is_safe = match solver.check_sat() {
                        SolverResult::Sat | SolverResult::Unknown => VerificationStatus::Failed,
                        SolverResult::Unsat => VerificationStatus::VerifiedSafe,
                    };
                    self.smt_cache.set_status(decl.id, is_safe);
                }

                self.check_function(f);
            }
            DeclKind::Struct(_) => {} // Struct fields are checked when used
            DeclKind::Impl(imp) => {
                for item in &imp.items {
                    self.check_decl(item);
                }
            }
            _ => {}
        }
    }

    fn check_function(&mut self, f: &FunctionDecl) {
        if let Some(body) = &f.body {
            self.check_block(body);
        }
    }

    // ── Blocks & Statements ──

    fn check_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
        if let Some(expr) = &block.expr {
            self.infer_expr(expr);
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Let { ty, value, .. } => {
                let declared_ty = if let Some(t) = ty {
                    self.resolve_type_expr(t)
                } else {
                    self.types.fresh_var()
                };
                if let Some(val) = value {
                    let val_ty = self.infer_expr(val);
                    self.engine.constrain(
                        declared_ty,
                        val_ty,
                        "let binding type must match initializer",
                    );
                }
            }
            StmtKind::Const { ty, value, .. } => {
                let declared_ty = if let Some(t) = ty {
                    self.resolve_type_expr(t)
                } else {
                    self.types.fresh_var()
                };
                let val_ty = self.infer_expr(value);
                self.engine
                    .constrain(declared_ty, val_ty, "const type must match value");
            }
            StmtKind::Expression(expr) => {
                self.infer_expr(expr);
            }
            StmtKind::Return(val) => {
                if let Some(e) = val {
                    self.infer_expr(e);
                }
            }
            StmtKind::While { condition, body } => {
                let cond_ty = self.infer_expr(condition);
                self.engine
                    .constrain(self.types.bool(), cond_ty, "while condition must be bool");
                self.check_block(body);
            }
            StmtKind::Loop { body } => {
                self.check_block(body);
            }
            StmtKind::For { iterable, body, .. } => {
                self.infer_expr(iterable);
                self.check_block(body);
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_ty = self.infer_expr(condition);
                self.engine
                    .constrain(self.types.bool(), cond_ty, "if condition must be bool");
                self.check_block(then_branch);
                if let Some(eb) = else_branch {
                    match eb {
                        ElseBranch::Else(block) => self.check_block(block),
                        ElseBranch::ElseIf(s) => self.check_stmt(s),
                    }
                }
            }
            StmtKind::Match { scrutinee, arms } => {
                self.infer_expr(scrutinee);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        let g_ty = self.infer_expr(guard);
                        self.engine
                            .constrain(self.types.bool(), g_ty, "match guard must be bool");
                    }
                    self.infer_expr(&arm.body);
                }
            }
            StmtKind::TryCatch { body, catches } => {
                self.check_block(body);
                for catch in catches {
                    self.check_block(&catch.body);
                }
            }
            StmtKind::Throw(expr) => {
                self.infer_expr(expr);
            }
            StmtKind::Break(v) | StmtKind::Yield(v) => {
                if let Some(e) = v {
                    self.infer_expr(e);
                }
            }
            StmtKind::Continue => {}
            StmtKind::Declaration(decl) => self.check_decl(decl),
        }
    }

    // ── Expressions ──

    /// Infer the type of an expression, generating constraints as needed.
    /// Returns the TypeId assigned to this expression.
    fn infer_expr(&mut self, expr: &Expr) -> TypeId {
        match &expr.kind {
            // ── Literals ──
            ExprKind::IntLiteral(_) => self.types.i32(),
            ExprKind::FloatLiteral(_) => self.types.f64(),
            ExprKind::StringLiteral(_) => self.types.str(),
            ExprKind::FStringLiteral { parts } => {
                for part in parts {
                    if let FStringPart::Expr(e) = part {
                        self.infer_expr(e);
                    }
                }
                self.types.str()
            }
            ExprKind::BoolLiteral(_) => self.types.bool(),

            ExprKind::ArrayLiteral(elems) => {
                let elem_ty = self.types.fresh_var();
                for e in elems {
                    let t = self.infer_expr(e);
                    self.engine
                        .constrain(elem_ty, t, "array elements must have same type");
                }
                self.types.fresh_var() // Array<elem_ty> — full generic support later
            }

            ExprKind::TupleLiteral(elems) => {
                let elem_tys: Vec<TypeId> = elems.iter().map(|e| self.infer_expr(e)).collect();
                self.types.insert(Type::Tuple(elem_tys))
            }

            // ── Names ──
            ExprKind::Identifier(ident) => {
                if let Some(sym_id) = self.scopes.lookup(&ident.name) {
                    let sym = self.scopes.get(sym_id);
                    match &sym.kind {
                        crate::symbol::SymbolKind::Variable { ty, .. } => *ty,
                        crate::symbol::SymbolKind::Function { return_ty, .. } => *return_ty,
                        crate::symbol::SymbolKind::Constant { ty, .. } => *ty,
                        _ => self.types.fresh_var(),
                    }
                } else {
                    self.types.error()
                }
            }
            ExprKind::PathExpr(_) => self.types.fresh_var(),

            // ── Binary operations ──
            ExprKind::Binary { op, left, right } => {
                let lt = self.infer_expr(left);
                let rt = self.infer_expr(right);

                match op {
                    // Arithmetic: both operands same type, result = same type
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod | BinOp::Pow => {
                        self.engine
                            .constrain(lt, rt, "binary operands must have same type");
                        lt
                    }
                    // Comparison: both operands same, result = bool
                    BinOp::Eq
                    | BinOp::NotEq
                    | BinOp::Lt
                    | BinOp::LtEq
                    | BinOp::Gt
                    | BinOp::GtEq => {
                        self.engine
                            .constrain(lt, rt, "comparison operands must have same type");
                        self.types.bool()
                    }
                    // Logical: both must be bool
                    BinOp::And | BinOp::Or => {
                        self.engine.constrain(
                            self.types.bool(),
                            lt,
                            "logical and/or requires bool",
                        );
                        self.engine.constrain(
                            self.types.bool(),
                            rt,
                            "logical and/or requires bool",
                        );
                        self.types.bool()
                    }
                    // Bitwise: both operands same type
                    BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr => {
                        self.engine
                            .constrain(lt, rt, "bitwise operands must have same type");
                        lt
                    }
                }
            }

            // ── Unary ──
            ExprKind::Unary { op, operand } => {
                let t = self.infer_expr(operand);
                match op {
                    UnaryOp::Neg => t,
                    UnaryOp::Not => {
                        self.engine
                            .constrain(self.types.bool(), t, "! requires bool operand");
                        self.types.bool()
                    }
                    UnaryOp::BitNot => t,
                    UnaryOp::Ref => self.types.insert(Type::Ref {
                        mutable: false,
                        inner: t,
                    }),
                    UnaryOp::Deref => self.types.fresh_var(),
                }
            }

            // ── Calls ──
            ExprKind::Call { callee, args } => {
                self.infer_expr(callee);
                let arg_tys: Vec<TypeId> = args.iter().map(|arg| self.infer_expr(arg)).collect();
                if let Some(builtin) = resolve_gpu_builtin_expr(callee) {
                    let expected_arg_tys = builtin.arg_types(&self.types);
                    for (expected, actual) in expected_arg_tys.into_iter().zip(arg_tys) {
                        self.engine.constrain(
                            expected,
                            actual,
                            "GPU builtin argument type mismatch",
                        );
                    }
                    return builtin.return_type(&mut self.types);
                }
                self.types.fresh_var() // Return type inferred later
            }
            ExprKind::MethodCall { object, args, .. } => {
                self.infer_expr(object);
                let arg_tys: Vec<TypeId> = args.iter().map(|arg| self.infer_expr(arg)).collect();
                if let ExprKind::MethodCall { object, method, .. } = &expr.kind
                    && let Some(builtin) = resolve_gpu_builtin_member(object, &method.name)
                {
                    let expected_arg_tys = builtin.arg_types(&self.types);
                    for (expected, actual) in expected_arg_tys.into_iter().zip(arg_tys) {
                        self.engine.constrain(
                            expected,
                            actual,
                            "GPU builtin argument type mismatch",
                        );
                    }
                    return builtin.return_type(&mut self.types);
                }
                self.types.fresh_var()
            }

            // ── Access ──
            ExprKind::FieldAccess { object, .. } => {
                self.infer_expr(object);
                self.types.fresh_var()
            }
            ExprKind::Index { object, index } => {
                self.infer_expr(object);
                self.infer_expr(index);
                self.types.fresh_var()
            }

            // ── Assignment ──
            ExprKind::Assign { target, value } => {
                let lt = self.infer_expr(target);
                let rt = self.infer_expr(value);
                self.engine.constrain(lt, rt, "assignment type mismatch");
                self.types.unit()
            }
            ExprKind::CompoundAssign { target, value, .. } => {
                let lt = self.infer_expr(target);
                let rt = self.infer_expr(value);
                self.engine
                    .constrain(lt, rt, "compound assignment type mismatch");
                self.types.unit()
            }

            // ── Control flow expressions ──
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let ct = self.infer_expr(condition);
                self.engine
                    .constrain(self.types.bool(), ct, "if condition must be bool");
                let tt = self.infer_expr(then_branch);
                if let Some(eb) = else_branch {
                    let et = self.infer_expr(eb);
                    self.engine
                        .constrain(tt, et, "if/else branches must have same type");
                }
                tt
            }
            ExprKind::Match { scrutinee, arms } => {
                self.infer_expr(scrutinee);
                let result_ty = self.types.fresh_var();
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        let g = self.infer_expr(guard);
                        self.engine
                            .constrain(self.types.bool(), g, "match guard must be bool");
                    }
                    let arm_ty = self.infer_expr(&arm.body);
                    self.engine
                        .constrain(result_ty, arm_ty, "match arms must have same type");
                }
                result_ty
            }

            ExprKind::Block(block) => {
                self.check_block(block);
                if let Some(expr) = &block.expr {
                    self.infer_expr(expr)
                } else {
                    self.types.unit()
                }
            }

            // ── Lambda ──
            ExprKind::Lambda { params, body, .. } => {
                let param_tys: Vec<TypeId> =
                    params.iter().map(|_| self.types.fresh_var()).collect();
                let ret_ty = self.infer_expr(body);
                self.types.insert(Type::Function {
                    params: param_tys,
                    ret: ret_ty,
                })
            }

            // ── Async ──
            ExprKind::Await(inner) | ExprKind::Spawn(inner) => {
                self.infer_expr(inner);
                self.types.fresh_var()
            }

            // ── Try ──
            ExprKind::Try(inner) => {
                let inner_ty = self.infer_expr(inner);
                match self.types.get(inner_ty) {
                    Type::Result { ok, .. } => *ok,
                    Type::Optional(val) => *val,
                    _ => self.types.fresh_var(),
                }
            }

            // ── Range ──
            ExprKind::Range { start, end, .. } => {
                if let Some(s) = start {
                    self.infer_expr(s);
                }
                if let Some(e) = end {
                    self.infer_expr(e);
                }
                self.types.fresh_var()
            }

            // ── Cast ──
            ExprKind::Cast {
                expr: inner,
                target_type,
            } => {
                self.infer_expr(inner);
                self.resolve_type_expr(target_type)
            }

            // ── Struct literal ──
            ExprKind::StructLiteral { fields, .. } => {
                for f in fields {
                    self.infer_expr(&f.value);
                }
                self.types.fresh_var()
            }

            // ── Differentiable Programming ──
            ExprKind::Grad { func, .. } => {
                self.infer_expr(func);
                // grad(f, x) returns a function: the derivative of f w.r.t. x
                self.types.fresh_var()
            }
            ExprKind::Backward(inner) => {
                self.infer_expr(inner);
                // backward produces gradient values (f64)
                self.types.f64()
            }
            ExprKind::Resume(inner) => {
                self.infer_expr(inner);
                // resume passes a value to the continuation
                self.types.fresh_var()
            }
            ExprKind::Perform { args, .. } => {
                for arg in args {
                    self.infer_expr(arg);
                }
                // Effect operations return a type determined by the handler
                self.types.fresh_var()
            }
            ExprKind::HandleWith { body, .. } => {
                self.infer_expr(body);
                // handle..with returns the result of the body under the handler
                self.types.fresh_var()
            }
            ExprKind::BlockExpr(block) => {
                for stmt in &block.stmts {
                    self.check_stmt(stmt);
                }
                self.types.fresh_var()
            }
        }
    }

    // ── Helpers ──

    /// Resolve an AST type expression to an internal TypeId.
    fn resolve_type_expr(&mut self, te: &TypeExpr) -> TypeId {
        match &te.kind {
            TypeExprKind::Named(path) => {
                if let Some(seg) = path.segments.last() {
                    builtin_type_id_for_name(&self.types, &seg.name)
                        .unwrap_or_else(|| self.types.fresh_var())
                } else {
                    self.types.error()
                }
            }
            TypeExprKind::Inferred | TypeExprKind::Dynamic | TypeExprKind::Any => self.types.any(),
            TypeExprKind::Never => self.types.never(),
            TypeExprKind::Reference { mutable, inner } => {
                let inner_id = self.resolve_type_expr(inner);
                self.types.insert(Type::Ref {
                    mutable: *mutable,
                    inner: inner_id,
                })
            }
            TypeExprKind::Pointer { mutable, inner } => {
                let inner_id = self.resolve_type_expr(inner);
                self.types.insert(Type::Ptr {
                    mutable: *mutable,
                    inner: inner_id,
                })
            }
            TypeExprKind::Optional(inner) => {
                let inner_id = self.resolve_type_expr(inner);
                self.types.insert(Type::Optional(inner_id))
            }
            TypeExprKind::Tuple(elems) => {
                let ids = elems
                    .iter()
                    .map(|elem| self.resolve_type_expr(elem))
                    .collect();
                self.types.insert(Type::Tuple(ids))
            }
            TypeExprKind::Array { element, size } => {
                let element_id = self.resolve_type_expr(element);
                let Some(size_expr) = size.as_deref() else {
                    self.errors.push(TypeError {
                        message: "fixed-size array types require a compile-time size".into(),
                        span: te.span,
                    });
                    return self.types.error();
                };
                let Some(size) = self.resolve_array_type_size(size_expr) else {
                    return self.types.error();
                };
                self.types.insert(Type::Array {
                    element: element_id,
                    size,
                })
            }
            TypeExprKind::Slice(inner) => {
                let inner_id = self.resolve_type_expr(inner);
                self.types.insert(Type::Slice(inner_id))
            }
            TypeExprKind::Function {
                params,
                return_type,
            } => {
                let param_ids = params
                    .iter()
                    .map(|param| self.resolve_type_expr(param))
                    .collect();
                let ret_id = self.resolve_type_expr(return_type);
                self.types.insert(Type::Function {
                    params: param_ids,
                    ret: ret_id,
                })
            }
            TypeExprKind::Refined { base, .. } => self.resolve_type_expr(base),
            _ => self.types.fresh_var(),
        }
    }

    fn resolve_array_type_size(&mut self, size_expr: &Expr) -> Option<usize> {
        let mut evaluator = ConstEvaluator::new();
        match evaluator.eval_expect_int(size_expr) {
            Some(size) if size >= 0 => Some(size as usize),
            Some(_) => {
                self.errors.push(TypeError {
                    message: "array size must be non-negative".into(),
                    span: size_expr.span,
                });
                None
            }
            None => {
                let message = evaluator
                    .errors
                    .into_iter()
                    .next()
                    .map(|err| err.message)
                    .unwrap_or_else(|| {
                        "array size must be a compile-time integer expression".into()
                    });
                self.errors.push(TypeError {
                    message,
                    span: size_expr.span,
                });
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolver::Resolver;
    use agam_errors::span::SourceId;
    use agam_lexer::Lexer;

    fn check_source(source: &str) -> TypeChecker {
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

        let mut checker = TypeChecker::from_resolver(resolver);
        checker.check_module(&module);
        checker
    }

    #[test]
    fn test_let_int_literal() {
        let tc = check_source("fn main(): let x = 42");
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    #[test]
    fn test_while_requires_bool() {
        // `while 42` should produce a type error (int is not bool)
        let tc = check_source("fn main(): while 42: let x = 1");
        assert!(!tc.errors.is_empty(), "expected type error for while(int)");
    }

    #[test]
    fn test_if_requires_bool() {
        let tc = check_source("fn main(): if true: let x = 1");
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    #[test]
    fn test_logical_and_requires_bool() {
        let tc = check_source("fn main(): let x = true && false");
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    #[test]
    fn test_arithmetic_same_type() {
        let tc = check_source("fn main(): let x = 1 + 2");
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    #[test]
    fn test_comparison_returns_bool() {
        let tc = check_source("fn main(): let x = 1 < 2");
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    #[test]
    fn test_gpu_builtin_call_type_is_resolved() {
        let tc = check_source("@gpu\nfn kern(): let tid: i32 = agam.gpu.thread_id_x()");
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    #[test]
    fn test_gpu_math_builtin_argument_type_is_checked() {
        let tc = check_source("@gpu\nfn kern(): let y: f32 = agam.gpu.sqrt(1)");
        assert!(
            !tc.errors.is_empty(),
            "expected type error for integer gpu sqrt argument"
        );
    }

    #[test]
    fn test_gpu_shared_alloc_accepts_annotated_pointer_type() {
        let tc =
            check_source("@gpu\nfn kern(): let scratch: *mut f32 = agam.gpu.shared_alloc(128)");
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    #[test]
    fn test_gpu_shared_alloc_accepts_fixed_array_type() {
        let tc =
            check_source("@gpu\nfn kern(): let scratch: [i32; 128] = agam.gpu.shared_alloc(128)");
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    #[test]
    fn test_gpu_wide_integer_types_typecheck() {
        let tc = check_source(
            "@gpu\nfn kern(a: i256, b: *mut u512): let scratch: [u256; 32] = agam.gpu.shared_alloc(32)",
        );
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    #[test]
    fn test_checker_try_operator_type_inference() {
        let tc = check_source("fn compute(x: i32) -> i32 { let y = x?; return y; }");
        // In the absence of error annotations, try runs and returns type variable without sema crash
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }
}
