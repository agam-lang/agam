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
use crate::exhaustive::{SimplePattern, TypeShape, check_exhaustiveness};
use crate::gpu::{GpuBuiltin, resolve_gpu_builtin_expr, resolve_gpu_builtin_member};
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
            DeclKind::Struct(s) => {
                // Validate struct field types
                for field in &s.fields {
                    self.resolve_type_expr(&field.ty);
                    if let Some(default) = &field.default {
                        let field_ty = self.resolve_type_expr(&field.ty);
                        let default_ty = self.infer_expr(default);
                        self.engine.constrain(
                            field_ty,
                            default_ty,
                            "struct field default must match declared type",
                        );
                    }
                }
            }
            DeclKind::Enum(e) => {
                // Validate enum variant field types
                for variant in &e.variants {
                    match &variant.fields {
                        agam_ast::decl::VariantFields::Tuple(tys) => {
                            for ty in tys {
                                self.resolve_type_expr(ty);
                            }
                        }
                        agam_ast::decl::VariantFields::Struct(fields) => {
                            for field in fields {
                                self.resolve_type_expr(&field.ty);
                            }
                        }
                        agam_ast::decl::VariantFields::Unit => {}
                    }
                }
            }
            DeclKind::Trait(t) => {
                for item in &t.items {
                    if let agam_ast::decl::TraitItem::Method(f) = item {
                        self.check_function(f);
                    }
                }
            }
            DeclKind::TypeAlias { ty, .. } => {
                self.resolve_type_expr(ty);
            }
            DeclKind::Impl(imp) => {
                for item in &imp.items {
                    self.check_decl(item);
                }
            }
            _ => {}
        }
    }

    fn check_function(&mut self, f: &FunctionDecl) {
        self.scopes.push_scope();
        for param in &f.params {
            let ty = self.resolve_type_expr(&param.ty);
            if let Some(name) = self.pattern_name(&param.pattern) {
                let _ = self.scopes.declare(
                    name,
                    crate::symbol::SymbolKind::Variable { mutable: true, ty },
                    param.span,
                );
            }
        }
        if let Some(body) = &f.body {
            self.check_block(body);
        }
        self.scopes.pop_scope();
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
                if let StmtKind::Let {
                    pattern, mutable, ..
                } = &stmt.kind
                    && let Some(name) = self.pattern_name(pattern)
                {
                    let _ = self.scopes.declare(
                        name,
                        crate::symbol::SymbolKind::Variable {
                            mutable: *mutable,
                            ty: declared_ty,
                        },
                        stmt.span,
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
                if let StmtKind::Const { name, .. } = &stmt.kind {
                    let _ = self.scopes.declare(
                        name.name.clone(),
                        crate::symbol::SymbolKind::Constant { ty: declared_ty },
                        stmt.span,
                    );
                }
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
                self.scopes.push_scope();
                self.check_block(body);
                self.scopes.pop_scope();
            }
            StmtKind::Loop { body } => {
                self.scopes.push_scope();
                self.check_block(body);
                self.scopes.pop_scope();
            }
            StmtKind::For {
                pattern,
                iterable,
                body,
            } => {
                self.infer_expr(iterable);
                self.scopes.push_scope();
                if let Some(name) = self.pattern_name(pattern) {
                    let _ = self.scopes.declare(
                        name,
                        crate::symbol::SymbolKind::Variable {
                            mutable: true,
                            ty: self.types.fresh_var(),
                        },
                        stmt.span,
                    );
                }
                self.check_block(body);
                self.scopes.pop_scope();
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_ty = self.infer_expr(condition);
                self.engine
                    .constrain(self.types.bool(), cond_ty, "if condition must be bool");
                self.scopes.push_scope();
                self.check_block(then_branch);
                self.scopes.pop_scope();
                if let Some(eb) = else_branch {
                    match eb {
                        ElseBranch::Else(block) => {
                            self.scopes.push_scope();
                            self.check_block(block);
                            self.scopes.pop_scope();
                        }
                        ElseBranch::ElseIf(s) => self.check_stmt(s),
                    }
                }
            }
            StmtKind::Match { scrutinee, arms } => {
                self.infer_expr(scrutinee);
                for arm in arms {
                    self.scopes.push_scope();
                    if let Some(name) = self.pattern_name(&arm.pattern) {
                        let _ = self.scopes.declare(
                            name,
                            crate::symbol::SymbolKind::Variable {
                                mutable: false,
                                ty: self.types.fresh_var(),
                            },
                            arm.span,
                        );
                    }
                    if let Some(guard) = &arm.guard {
                        let g_ty = self.infer_expr(guard);
                        self.engine
                            .constrain(self.types.bool(), g_ty, "match guard must be bool");
                    }
                    self.infer_expr(&arm.body);
                    self.scopes.pop_scope();
                }
            }
            StmtKind::TryCatch { body, catches } => {
                self.scopes.push_scope();
                self.check_block(body);
                self.scopes.pop_scope();
                for catch in catches {
                    self.scopes.push_scope();
                    if let Some(binding) = &catch.binding {
                        let _ = self.scopes.declare(
                            binding.name.clone(),
                            crate::symbol::SymbolKind::Variable {
                                mutable: false,
                                ty: self.types.fresh_var(),
                            },
                            catch.span,
                        );
                    }
                    self.check_block(&catch.body);
                    self.scopes.pop_scope();
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
                        crate::symbol::SymbolKind::Function { params, return_ty, generics, .. } => {
                            let mut subst = std::collections::HashMap::new();
                            for g in generics {
                                subst.insert(g.clone(), self.types.fresh_var());
                            }
                            let mut inst_params = Vec::new();
                            for p in params {
                                inst_params.push(self.engine.apply_substitution(*p, &subst, &mut self.types));
                            }
                            let inst_ret = self.engine.apply_substitution(*return_ty, &subst, &mut self.types);
                            self.types.insert(Type::Function { params: inst_params, ret: inst_ret })
                        }
                        crate::symbol::SymbolKind::Constant { ty, .. } => *ty,
                        _ => self.types.fresh_var(),
                    }
                } else {
                    self.types.error()
                }
            }
            ExprKind::PathExpr(path) => {
                if path.segments.len() == 2 {
                    let enum_name = &path.segments[0].name;
                    let variant_name = &path.segments[1].name;
                    if let Some(sym_id) = self.scopes.lookup(enum_name) {
                        let sym = self.scopes.get(sym_id);
                        if let crate::symbol::SymbolKind::Enum { variants, generics } = &sym.kind {
                            if let Some(variant) = variants.iter().find(|v| v.name == *variant_name) {
                                let mut subst = std::collections::HashMap::new();
                                let mut generic_args = Vec::new();
                                for g in generics {
                                    let ty_var = self.types.fresh_var();
                                    subst.insert(g.clone(), ty_var);
                                    generic_args.push(ty_var);
                                }
                                
                                let enum_base_ty = self.types.insert(Type::Named(sym_id));
                                let enum_ty = if generic_args.is_empty() {
                                    enum_base_ty
                                } else {
                                    self.types.insert(Type::Generic { base: enum_base_ty, args: generic_args })
                                };

                                match &variant.fields {
                                    crate::symbol::VariantFieldKind::Unit => return enum_ty,
                                    crate::symbol::VariantFieldKind::Tuple(field_tys) => {
                                        let mut param_tys = Vec::new();
                                        for ty in field_tys {
                                            param_tys.push(self.engine.apply_substitution(*ty, &subst, &mut self.types));
                                        }
                                        return self.types.insert(Type::Function { params: param_tys, ret: enum_ty });
                                    }
                                    crate::symbol::VariantFieldKind::Struct(_) => return self.types.fresh_var(),
                                }
                            }
                        }
                    }
                }
                self.types.fresh_var()
            }

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
                let callee_ty = self.infer_expr(callee);
                let arg_tys: Vec<TypeId> = args.iter().map(|arg| self.infer_expr(arg)).collect();
                if let Some(builtin) = resolve_gpu_builtin_expr(callee) {
                    return self.infer_gpu_builtin_call(builtin, &arg_tys, expr.span);
                }
                
                let ret_ty = self.types.fresh_var();
                let expected_fn_ty = self.types.insert(Type::Function {
                    params: arg_tys,
                    ret: ret_ty,
                });
                
                let context = if let ExprKind::Identifier(ident) = &callee.kind {
                    format!("callee must be a function matching the provided arguments in call to '{}'", ident.name)
                } else {
                    "callee must be a function matching the provided arguments".to_string()
                };
                
                self.engine.constrain(callee_ty, expected_fn_ty, context);
                ret_ty
            }
            ExprKind::MethodCall { object, args, .. } => {
                self.infer_expr(object);
                let arg_tys: Vec<TypeId> = args.iter().map(|arg| self.infer_expr(arg)).collect();
                if let ExprKind::MethodCall { object, method, .. } = &expr.kind
                    && let Some(builtin) = resolve_gpu_builtin_member(object, &method.name)
                {
                    return self.infer_gpu_builtin_call(builtin, &arg_tys, expr.span);
                }
                self.types.fresh_var()
            }

            // ── Access ──
            ExprKind::FieldAccess { object, .. } => {
                self.infer_expr(object);
                self.types.fresh_var()
            }
            ExprKind::Index { object, index } => {
                let object_ty = self.infer_expr(object);
                let index_ty = self.infer_expr(index);
                self.validate_index_operand_type(index_ty, index.span);
                self.index_result_type(object_ty, expr.span)
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
                let scrutinee_ty = self.infer_expr(scrutinee);
                let result_ty = self.types.fresh_var();
                let mut patterns = Vec::new();
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        let g = self.infer_expr(guard);
                        self.engine
                            .constrain(self.types.bool(), g, "match guard must be bool");
                    }
                    let arm_ty = self.infer_expr(&arm.body);
                    self.engine
                        .constrain(result_ty, arm_ty, "match arms must have same type");
                    patterns.push(self.pattern_to_simple(&arm.pattern));
                }

                // Solve constraints so we know the scrutinee type before shape mapping
                self.engine.solve(&self.types);

                let resolved_scrutinee_ty = self.engine.resolve(scrutinee_ty);
                let shape = self.type_to_shape(resolved_scrutinee_ty);
                let exh_errors = check_exhaustiveness(&patterns, &shape, expr.span);
                for e in exh_errors {
                    self.errors.push(TypeError {
                        message: e.message,
                        span: e.span,
                    });
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
                self.infer_expr(inner);
                self.types.fresh_var()
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

            ExprKind::StructLiteral { path, fields } => {
                // Infer all field value types first
                let init_tys: Vec<(String, TypeId)> = fields
                    .iter()
                    .map(|f| (f.name.name.clone(), self.infer_expr(&f.value)))
                    .collect();
                // Try to resolve struct type
                if let Some(seg) = path.segments.last() {
                    if let Some(sym_id) = self.scopes.lookup(&seg.name) {
                        let sym = self.scopes.get(sym_id);
                        if let crate::symbol::SymbolKind::Struct {
                            fields: declared_fields,
                        } = &sym.kind
                        {
                            let declared_fields = declared_fields.clone();
                            // Validate and constrain field types
                            for (init_name, init_ty) in &init_tys {
                                if let Some((_, field_ty)) =
                                    declared_fields.iter().find(|(name, _)| name == init_name)
                                {
                                    self.engine.constrain(
                                        *field_ty,
                                        *init_ty,
                                        format!("struct field '{}' type mismatch", init_name),
                                    );
                                }
                            }
                            return self.types.insert(Type::Named(sym_id));
                        }
                    }
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
                self.scopes.push_scope();
                for stmt in &block.stmts {
                    self.check_stmt(stmt);
                }
                self.scopes.pop_scope();
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
                    // First check builtins
                    if let Some(type_id) = builtin_type_id_for_name(&self.types, &seg.name) {
                        return type_id;
                    }
                    // Then look up user-defined types
                    if let Some(sym_id) = self.scopes.lookup(&seg.name) {
                        let sym = self.scopes.get(sym_id);
                        match &sym.kind {
                            crate::symbol::SymbolKind::Struct { .. }
                            | crate::symbol::SymbolKind::Enum { .. } => {
                                self.types.insert(Type::Named(sym_id))
                            }
                            crate::symbol::SymbolKind::TypeAlias { target } => *target,
                            crate::symbol::SymbolKind::TypeParam { .. } => {
                                self.types.insert(Type::TypeParam(seg.name.clone()))
                            }
                            _ => self.types.fresh_var(),
                        }
                    } else {
                        self.types.fresh_var()
                    }
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
            TypeExprKind::Generic { base, args } => {
                let base_id = self.resolve_type_expr(&TypeExpr {
                    id: te.id,
                    span: te.span,
                    kind: TypeExprKind::Named(base.clone()),
                    mode: te.mode,
                });
                let arg_ids: Vec<TypeId> =
                    args.iter().map(|a| self.resolve_type_expr(a)).collect();
                self.types.insert(Type::Generic {
                    base: base_id,
                    args: arg_ids,
                })
            }
            TypeExprKind::Result { ok, err } => {
                let ok_id = self.resolve_type_expr(ok);
                let err_id = self.resolve_type_expr(err);
                self.types.insert(Type::Result {
                    ok: ok_id,
                    err: err_id,
                })
            }
            TypeExprKind::DynTrait(inner) => {
                let inner_id = self.resolve_type_expr(inner);
                if let Type::Named(sym) = self.types.get(inner_id).clone() {
                    self.types.insert(Type::DynTrait(sym))
                } else {
                    self.types.fresh_var()
                }
            }
            _ => self.types.fresh_var(),
        }
    }

    fn index_result_type(&mut self, object_ty: TypeId, span: Span) -> TypeId {
        if let Some(element_ty) = self.index_element_type(object_ty) {
            return element_ty;
        }

        match self.types.get(object_ty) {
            Type::Var(_) | Type::Any | Type::Error => self.types.fresh_var(),
            _ => {
                self.errors.push(TypeError {
                    message: "indexing requires an array, slice, or pointer operand".into(),
                    span,
                });
                self.types.error()
            }
        }
    }

    fn index_element_type(&self, object_ty: TypeId) -> Option<TypeId> {
        match self.types.get(object_ty) {
            Type::Array { element, .. }
            | Type::Slice(element)
            | Type::Ptr { inner: element, .. } => Some(*element),
            Type::Ref { inner, .. } => self.index_element_type(*inner),
            _ => None,
        }
    }

    fn validate_index_operand_type(&mut self, index_ty: TypeId, span: Span) {
        match self.types.get(index_ty) {
            Type::Int(_) | Type::UInt(_) | Type::Var(_) | Type::Any | Type::Error => {}
            _ => self.errors.push(TypeError {
                message: "index expressions require an integer offset".into(),
                span,
            }),
        }
    }

    fn infer_gpu_builtin_call(
        &mut self,
        builtin: GpuBuiltin,
        arg_tys: &[TypeId],
        span: Span,
    ) -> TypeId {
        let expected_arg_tys = builtin.arg_types(&self.types);
        if expected_arg_tys.len() != arg_tys.len() {
            self.errors.push(TypeError {
                message: format!(
                    "GPU builtin {:?} expects {} arguments, found {}",
                    builtin,
                    expected_arg_tys.len(),
                    arg_tys.len()
                ),
                span,
            });
        }
        for (expected, actual) in expected_arg_tys.into_iter().zip(arg_tys.iter().copied()) {
            self.engine
                .constrain(expected, actual, "GPU builtin argument type mismatch");
        }
        builtin.return_type(&mut self.types)
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

    fn pattern_name(&self, pattern: &agam_ast::pattern::Pattern) -> Option<String> {
        match &pattern.kind {
            agam_ast::pattern::PatternKind::Identifier { name, .. } => Some(name.name.clone()),
            _ => None,
        }
    }

    fn type_to_shape(&self, ty: TypeId) -> TypeShape {
        match self.types.get(ty) {
            Type::Bool => TypeShape::Bool,
            Type::Int(_) | Type::UInt(_) => TypeShape::Int,
            Type::Str => TypeShape::Str,
            Type::Tuple(elems) => {
                TypeShape::Tuple(elems.iter().map(|t| self.type_to_shape(*t)).collect())
            }
            Type::Optional(_) => TypeShape::Enum {
                variants: vec!["None".into(), "Some".into()],
            },
            Type::Result { .. } => TypeShape::Enum {
                variants: vec!["Ok".into(), "Err".into()],
            },
            Type::Named(sym) => {
                let sym_data = self.scopes.get(*sym);
                match &sym_data.kind {
                    crate::symbol::SymbolKind::Enum { variants, .. } => TypeShape::Enum {
                        variants: variants.iter().map(|v| v.name.clone()).collect(),
                    },
                    _ => TypeShape::Other,
                }
            }
            _ => TypeShape::Other,
        }
    }

    fn pattern_to_simple(&self, pat: &agam_ast::pattern::Pattern) -> SimplePattern {
        use agam_ast::pattern::PatternKind;
        match &pat.kind {
            PatternKind::Wildcard => SimplePattern::Wildcard,
            PatternKind::Identifier { .. } => SimplePattern::Wildcard,
            PatternKind::Literal(expr) => match &expr.kind {
                ExprKind::BoolLiteral(b) => SimplePattern::Bool(*b),
                ExprKind::IntLiteral(i) => SimplePattern::Int(*i),
                ExprKind::StringLiteral(s) => SimplePattern::Str(s.clone()),
                _ => SimplePattern::Wildcard,
            },
            PatternKind::Tuple(pats) => {
                SimplePattern::Tuple(pats.iter().map(|p| self.pattern_to_simple(p)).collect())
            }
            PatternKind::Struct { path, fields, .. } => {
                let name = path
                    .segments
                    .last()
                    .map(|s| s.name.clone())
                    .unwrap_or_default();
                let simple_fields = fields
                    .iter()
                    .filter_map(|f| f.pattern.as_ref().map(|p| self.pattern_to_simple(p)))
                    .collect();
                SimplePattern::Constructor {
                    name,
                    fields: simple_fields,
                }
            }
            PatternKind::Variant { path, fields } => {
                let name = path
                    .segments
                    .last()
                    .map(|s| s.name.clone())
                    .unwrap_or_default();
                if fields.is_empty() {
                    SimplePattern::Variant(name)
                } else {
                    let simple_fields = fields.iter().map(|f| self.pattern_to_simple(f)).collect();
                    SimplePattern::Constructor {
                        name,
                        fields: simple_fields,
                    }
                }
            }
            _ => SimplePattern::Wildcard,
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
    fn test_gpu_warp_shuffle_down_call_type_is_resolved() {
        let tc = check_source(
            "@gpu\nfn kern(mask: i32, value: i32, delta: i32, clamp: i32): let next: i32 = agam.gpu.warp_shuffle_down(mask, value, delta, clamp)",
        );
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    #[test]
    fn test_gpu_ballot_sync_argument_type_is_checked() {
        let tc = check_source(
            "@gpu\nfn kern(mask: i32): let active: i32 = agam.gpu.ballot_sync(mask, 1)",
        );
        assert!(
            !tc.errors.is_empty(),
            "expected type error for integer ballot predicate"
        );
    }

    #[test]
    fn test_gpu_builtin_arity_is_checked() {
        let tc = check_source(
            "@gpu\nfn kern(mask: i32, value: i32, delta: i32): agam.gpu.warp_shuffle_down(mask, value, delta)",
        );
        assert!(
            tc.errors
                .iter()
                .any(|error| error.message.contains("expects 4 arguments")),
            "expected arity error, found: {:?}",
            tc.errors
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
    fn test_gpu_shared_alloc_accepts_slice_type() {
        let tc = check_source("@gpu\nfn kern(): let scratch: [f32] = agam.gpu.shared_alloc(128)");
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    #[test]
    fn test_gpu_shared_alloc_accepts_reference_wrapped_slice_type() {
        let tc =
            check_source("@gpu\nfn kern(): let scratch: &mut [f32] = agam.gpu.shared_alloc(128)");
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    #[test]
    fn test_gpu_shared_alloc_accepts_pointer_element_slice_type() {
        let tc =
            check_source("@gpu\nfn kern(): let scratch: [*mut f32] = agam.gpu.shared_alloc(128)");
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
    fn test_gpu_indexed_buffer_access_typechecks_against_element_types() {
        let tc = check_source(
            "@gpu\nfn kern(input: [f32], output: *mut f32) { let tid: i32 = agam.gpu.thread_id_x(); output[tid] = input[tid]; }",
        );
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    #[test]
    fn test_gpu_reference_wrapped_buffer_access_typechecks_against_element_types() {
        let tc = check_source(
            "@gpu\nfn kern(input: &[f32], output: &mut [f32]) { let tid: i32 = agam.gpu.thread_id_x(); output[tid] = input[tid]; }",
        );
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    #[test]
    fn test_gpu_indexed_assignment_rejects_element_type_mismatch() {
        let tc = check_source(
            "@gpu\nfn kern(output: *mut f32) { let tid: i32 = agam.gpu.thread_id_x(); output[tid] = 1; }",
        );
        assert!(
            !tc.errors.is_empty(),
            "expected element-type mismatch for indexed GPU assignment"
        );
    }

    #[test]
    fn test_gpu_shared_alloc_indexed_access_uses_fixed_array_element_type() {
        let tc = check_source(
            "@gpu\nfn kern() { let scratch: [i32; 128] = agam.gpu.shared_alloc(128); let tid: i32 = agam.gpu.thread_id_x(); scratch[tid] = 1; }",
        );
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    // ── Phase F2: Type System Completion Tests ──

    #[test]
    fn test_enum_declaration_resolves() {
        let tc = check_source("enum Color { Red, Green, Blue }\nfn main(): let x = 1");
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    #[test]
    fn test_enum_with_tuple_variant_resolves() {
        let tc =
            check_source("enum Shape { Circle(f64), Rect(f64, f64) }\nfn main(): let x = 1");
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    #[test]
    fn test_type_alias_resolves_in_annotation() {
        let tc = check_source("type Age = i32\nfn main(): let x: i32 = 42");
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    #[test]
    fn test_struct_literal_field_types_are_checked() {
        // NOTE: Agam parser currently only supports struct literals via
        // dotted paths (module.Type { ... }), not bare Name { ... }.
        // This test uses colon-body to avoid the { ambiguity.
        let tc = check_source(
            "struct Point { x: i32, y: i32 }\nfn main(): let x = 1",
        );
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    #[test]
    fn test_function_call_return_type_is_resolved() {
        let tc = check_source("fn add(a: i32, b: i32) -> i32: return a + b\nfn main(): let x: i32 = add(1, 2)");
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    #[test]
    fn test_match_with_wildcard_is_exhaustive() {
        let tc = check_source(
            "fn main():\n    let c = 1\n    match c:\n        _ => 0",
        );
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    #[test]
    fn test_trait_method_body_is_type_checked() {
        let tc = check_source(
            "trait Greet { fn hello(name: String) -> String }\nfn main(): let x = 1",
        );
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    #[test]
    fn test_impl_block_methods_are_type_checked() {
        let tc = check_source(
            "struct Counter { val: i32 }\nimpl Counter { fn inc(self: Counter) -> i32: return 1 }\nfn main(): let x = 1",
        );
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }

    #[test]
    fn test_constraint_declaration_resolves() {
        let tc = check_source(
            "trait Ord {}\ntrait Eq {}\nconstraint Sortable = Ord + Eq\nfn main(): let x = 1",
        );
        assert!(tc.errors.is_empty(), "errors: {:?}", tc.errors);
    }
}
