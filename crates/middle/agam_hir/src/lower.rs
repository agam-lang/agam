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
use agam_ast::types::{TypeExpr, TypeExprKind};
use agam_ast::*;
use agam_sema::consteval::ConstEvaluator;
use agam_sema::gpu::{
    GpuBuiltin, GpuKernelParamAbi, GpuKernelScalarAbi, resolve_gpu_builtin_expr,
    resolve_gpu_builtin_member,
};
use agam_sema::types::{FloatSize, IntSize, Type, TypeStore, builtin_type_id_for_name};

use agam_sema::target::TargetProfile;

use crate::nodes::*;

/// The HIR lowering context.
pub struct HirLowering {
    next_id: u32,
    types: TypeStore,
    scopes: Vec<HashMap<String, agam_sema::symbol::TypeId>>,
    /// The target profile of the function currently being lowered.
    current_target: TargetProfile,
    /// Diagnostic messages collected during lowering.
    pub diagnostics: Vec<String>,
}

impl HirLowering {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            types: TypeStore::new(),
            scopes: Vec::new(),
            current_target: TargetProfile::Default,
            diagnostics: Vec::new(),
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
        let enum_layouts = Self::collect_enum_layouts(module);
        let struct_layouts = Self::collect_struct_layouts(module);
        let functions = module
            .declarations
            .iter()
            .filter_map(|decl| self.lower_decl(decl))
            .collect();
        HirModule {
            functions,
            enum_layouts,
            struct_layouts,
        }
    }

    fn collect_enum_layouts(module: &Module) -> HashMap<String, HirEnumLayout> {
        module
            .declarations
            .iter()
            .filter_map(|decl| {
                let DeclKind::Enum(enum_decl) = &decl.kind else {
                    return None;
                };
                let variants = enum_decl
                    .variants
                    .iter()
                    .enumerate()
                    .map(|(tag, variant)| HirEnumVariantLayout {
                        name: variant.name.name.clone(),
                        tag: tag as u32,
                        has_payload: match &variant.fields {
                            VariantFields::Unit => false,
                            VariantFields::Tuple(fields) => !fields.is_empty(),
                            VariantFields::Struct(fields) => !fields.is_empty(),
                        },
                    })
                    .collect();
                let layout = HirEnumLayout {
                    name: enum_decl.name.name.clone(),
                    variants,
                };
                Some((layout.name.clone(), layout))
            })
            .collect()
    }

    fn collect_struct_layouts(module: &Module) -> HashMap<String, HirStructLayout> {
        module
            .declarations
            .iter()
            .filter_map(|decl| {
                let DeclKind::Struct(struct_decl) = &decl.kind else {
                    return None;
                };
                let layout = HirStructLayout {
                    name: struct_decl.name.name.clone(),
                    fields: struct_decl
                        .fields
                        .iter()
                        .map(|field| field.name.name.clone())
                        .collect(),
                };
                Some((layout.name.clone(), layout))
            })
            .collect()
    }

    fn lower_decl(&mut self, decl: &Decl) -> Option<HirFunction> {
        match &decl.kind {
            DeclKind::Function(f) => Some(self.lower_function(f)),
            _ => None,
        }
    }

    fn lower_function(&mut self, f: &FunctionDecl) -> HirFunction {
        self.push_scope();

        let target = agam_sema::target::resolve_target_profile(&f.annotations)
            .unwrap_or(TargetProfile::Default);
        self.current_target = target;

        // Resolve GPU kernel config
        let gpu_config = match agam_sema::gpu::resolve_gpu_config(&f.annotations) {
            Ok(config) => config,
            Err(e) => {
                self.diagnostics.push(format!("error: {e}"));
                None
            }
        };
        if gpu_config.is_some() {
            for error in agam_sema::gpu::validate_gpu_kernel_function(f) {
                self.diagnostics.push(format!("error: {error}"));
            }
        }

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
                    gpu_abi: classify_gpu_kernel_param_abi(&p.ty),
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
            generics: f
                .generics
                .iter()
                .map(|g| {
                    HirGenericParam {
                        name: g.name.name.clone(),
                        bounds: vec![], // TODO: properly stringify TypeExpr bounds
                    }
                })
                .collect(),
            params,
            return_ty: f
                .return_type
                .as_ref()
                .map(|ty| self.resolve_type_expr(ty))
                .unwrap_or_else(|| self.types.unit()),
            body,
            is_async: f.is_async,
            target,
            gpu_config,
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
                let declared_ty = ty.as_ref().map(|te| self.resolve_type_expr(te));
                let lowered_value = value
                    .as_ref()
                    .map(|v| self.lower_expr_with_hint(v, declared_ty));
                let inferred_ty = lowered_value
                    .as_ref()
                    .map(|expr| expr.ty)
                    .unwrap_or_else(|| self.types.fresh_var());
                let lowered = HirStmt::Let {
                    name,
                    ty: declared_ty.unwrap_or(inferred_ty),
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
            StmtKind::Match { scrutinee, arms } => HirStmt::Match {
                scrutinee: self.lower_expr(scrutinee),
                arms: arms
                    .iter()
                    .map(|arm| HirMatchArm {
                        pattern: self.lower_pattern(&arm.pattern),
                        guard: arm.guard.as_ref().map(|g| self.lower_expr(g)),
                        body: self.lower_expr(&arm.body),
                    })
                    .collect(),
            },
            _ => HirStmt::Expr(HirExpr {
                id: self.fresh_id(),
                ty: self.types.unit(),
                kind: HirExprKind::Tuple(vec![]),
            }),
        }
    }

    fn lower_expr(&mut self, expr: &Expr) -> HirExpr {
        self.lower_expr_with_hint(expr, None)
    }

    fn lower_expr_with_hint(
        &mut self,
        expr: &Expr,
        expected_ty: Option<agam_sema::symbol::TypeId>,
    ) -> HirExpr {
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
                let full = path_name(path);
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

            ExprKind::Call { callee, args } => match resolve_gpu_builtin_expr(callee) {
                Some(GpuBuiltin::SharedAlloc) => {
                    let count = self.lower_gpu_shared_alloc_count(args);
                    let element_abi = self.shared_alloc_element_abi(expected_ty);
                    if element_abi == GpuKernelParamAbi::OpaquePtr {
                        self.diagnostics.push(
                            "error: `agam.gpu.shared_alloc(...)` currently requires an annotated pointer, slice, reference, or fixed-size array target type".into(),
                        );
                    }
                    (
                        expected_ty.unwrap_or_else(|| self.types.fresh_var()),
                        HirExprKind::GpuSharedAlloc {
                            element_abi,
                            count: Box::new(count),
                        },
                    )
                }
                Some(builtin) => {
                    self.validate_gpu_builtin_arity(builtin, args.len());
                    (
                        builtin.return_type(&mut self.types),
                        HirExprKind::Call {
                            callee: Box::new(self.lower_expr(callee)),
                            args: args.iter().map(|a| self.lower_expr(a)).collect(),
                        },
                    )
                }
                None => (
                    self.types.fresh_var(),
                    HirExprKind::Call {
                        callee: Box::new(self.lower_expr(callee)),
                        args: args.iter().map(|a| self.lower_expr(a)).collect(),
                    },
                ),
            },
            ExprKind::MethodCall {
                object,
                method,
                args,
            } => match resolve_gpu_builtin_member(object, &method.name) {
                Some(GpuBuiltin::SharedAlloc) => {
                    let count = self.lower_gpu_shared_alloc_count(args);
                    let element_abi = self.shared_alloc_element_abi(expected_ty);
                    if element_abi == GpuKernelParamAbi::OpaquePtr {
                        self.diagnostics.push(
                                "error: `agam.gpu.shared_alloc(...)` currently requires an annotated pointer, slice, reference, or fixed-size array target type".into(),
                            );
                    }
                    (
                        expected_ty.unwrap_or_else(|| self.types.fresh_var()),
                        HirExprKind::GpuSharedAlloc {
                            element_abi,
                            count: Box::new(count),
                        },
                    )
                }
                Some(builtin) => {
                    self.validate_gpu_builtin_arity(builtin, args.len());
                    let callee_name = format!("{}::{}", expr_name(object).unwrap(), method.name);
                    (
                        builtin.return_type(&mut self.types),
                        HirExprKind::Call {
                            callee: Box::new(HirExpr {
                                id: self.fresh_id(),
                                ty: self.types.fresh_var(),
                                kind: HirExprKind::Var(callee_name),
                            }),
                            args: args.iter().map(|a| self.lower_expr(a)).collect(),
                        },
                    )
                }
                None => (
                    self.types.fresh_var(),
                    HirExprKind::MethodCall {
                        object: Box::new(self.lower_expr(object)),
                        method: method.name.clone(),
                        args: args.iter().map(|a| self.lower_expr(a)).collect(),
                    },
                ),
            },

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

            ExprKind::ArrayLiteral(elems) => (
                self.types.fresh_var(),
                HirExprKind::Array(elems.iter().map(|e| self.lower_expr(e)).collect()),
            ),
            ExprKind::TupleLiteral(elems) => (
                self.types.unit(),
                HirExprKind::Tuple(elems.iter().map(|e| self.lower_expr(e)).collect()),
            ),

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
                        expr: Box::new(self.lower_expr_with_hint(inner, Some(target_ty))),
                        target_ty,
                    },
                )
            }

            ExprKind::Perform {
                effect,
                operation,
                args,
            } => {
                // Validate: IoT target forbids effects
                if self.current_target == TargetProfile::Iot {
                    self.diagnostics.push(format!(
                        "error: `perform {}.{}` is not allowed under @target.iot: \
                         effects require runtime support not available on IoT targets",
                        effect.name, operation.name
                    ));
                }
                (
                    self.types.fresh_var(),
                    HirExprKind::Perform {
                        effect: effect.name.clone(),
                        operation: operation.name.clone(),
                        args: args.iter().map(|a| self.lower_expr(a)).collect(),
                    },
                )
            }

            ExprKind::HandleWith { body, handler } => (
                self.types.fresh_var(),
                HirExprKind::HandleWith {
                    effect: handler.name.clone(),
                    handler: handler.name.clone(),
                    body: Box::new(self.lower_expr(body)),
                },
            ),

            ExprKind::Match { scrutinee, arms } => {
                let scrutinee_hir = self.lower_expr(scrutinee);
                let result_ty = self.types.fresh_var();
                let hir_arms: Vec<HirMatchArm> = arms
                    .iter()
                    .map(|arm| HirMatchArm {
                        pattern: self.lower_pattern(&arm.pattern),
                        guard: arm.guard.as_ref().map(|g| self.lower_expr(g)),
                        body: self.lower_expr(&arm.body),
                    })
                    .collect();
                (
                    result_ty,
                    HirExprKind::Match {
                        scrutinee: Box::new(scrutinee_hir),
                        arms: hir_arms,
                    },
                )
            }

            ExprKind::StructLiteral { path, fields } => {
                let name = path_name(path);
                let hir_fields = fields
                    .iter()
                    .map(|f| (f.name.name.clone(), self.lower_expr(&f.value)))
                    .collect();
                (
                    self.types.fresh_var(),
                    HirExprKind::StructLiteral {
                        name,
                        fields: hir_fields,
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
            agam_ast::types::TypeExprKind::Reference { mutable, inner } => {
                let inner_id = self.resolve_type_expr(inner);
                self.types.insert(Type::Ref {
                    mutable: *mutable,
                    inner: inner_id,
                })
            }
            agam_ast::types::TypeExprKind::Pointer { mutable, inner } => {
                let inner_id = self.resolve_type_expr(inner);
                self.types.insert(Type::Ptr {
                    mutable: *mutable,
                    inner: inner_id,
                })
            }
            agam_ast::types::TypeExprKind::Optional(inner) => {
                let inner_id = self.resolve_type_expr(inner);
                self.types.insert(Type::Optional(inner_id))
            }
            agam_ast::types::TypeExprKind::Tuple(elems) => {
                let ids = elems
                    .iter()
                    .map(|elem| self.resolve_type_expr(elem))
                    .collect();
                self.types.insert(Type::Tuple(ids))
            }
            agam_ast::types::TypeExprKind::Array { element, size } => {
                let element_id = self.resolve_type_expr(element);
                let Some(size_expr) = size.as_deref() else {
                    self.diagnostics
                        .push("error: fixed-size array types require a compile-time size".into());
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
            agam_ast::types::TypeExprKind::Slice(inner) => {
                let inner_id = self.resolve_type_expr(inner);
                self.types.insert(Type::Slice(inner_id))
            }
            agam_ast::types::TypeExprKind::Function {
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
            agam_ast::types::TypeExprKind::SelfType => self.types.fresh_var(),
            agam_ast::types::TypeExprKind::Never => self.types.never(),
            agam_ast::types::TypeExprKind::Refined { base, .. } => self.resolve_type_expr(base),
            _ => self.types.fresh_var(),
        }
    }

    fn lower_gpu_shared_alloc_count(&mut self, args: &[Expr]) -> HirExpr {
        if args.len() != 1 {
            self.diagnostics.push(format!(
                "error: `agam.gpu.shared_alloc(...)` expects exactly one count argument, found {}",
                args.len()
            ));
        }
        args.first()
            .map(|arg| self.lower_expr(arg))
            .unwrap_or(HirExpr {
                id: self.fresh_id(),
                ty: self.types.i32(),
                kind: HirExprKind::IntLit(0),
            })
    }

    fn validate_gpu_builtin_arity(&mut self, builtin: GpuBuiltin, actual_arg_count: usize) {
        let expected_arg_count = builtin.arg_types(&self.types).len();
        if expected_arg_count != actual_arg_count {
            self.diagnostics.push(format!(
                "error: GPU builtin {:?} expects {} arguments, found {}",
                builtin, expected_arg_count, actual_arg_count
            ));
        }
    }

    fn resolve_array_type_size(&mut self, size_expr: &Expr) -> Option<usize> {
        let mut evaluator = ConstEvaluator::new();
        match evaluator.eval_expect_int(size_expr) {
            Some(size) if size >= 0 => Some(size as usize),
            Some(_) => {
                self.diagnostics
                    .push("error: array size must be non-negative".into());
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
                self.diagnostics.push(format!("error: {message}"));
                None
            }
        }
    }

    fn shared_alloc_element_abi(
        &self,
        expected_ty: Option<agam_sema::symbol::TypeId>,
    ) -> GpuKernelParamAbi {
        let Some(expected_ty) = expected_ty else {
            return GpuKernelParamAbi::OpaquePtr;
        };
        self.shared_alloc_element_abi_from_type_id(expected_ty)
    }

    fn shared_alloc_element_abi_from_type_id(
        &self,
        ty: agam_sema::symbol::TypeId,
    ) -> GpuKernelParamAbi {
        match self.types.get(ty) {
            Type::Ref { inner, .. } | Type::Ptr { inner, .. } => match self.types.get(*inner) {
                Type::Slice(element) | Type::Array { element, .. } => self
                    .type_abi_from_type_id(*element)
                    .unwrap_or(GpuKernelParamAbi::OpaquePtr),
                _ => self
                    .type_abi_from_type_id(*inner)
                    .unwrap_or(GpuKernelParamAbi::OpaquePtr),
            },
            Type::Slice(inner) | Type::Array { element: inner, .. } => self
                .type_abi_from_type_id(*inner)
                .unwrap_or(GpuKernelParamAbi::OpaquePtr),
            _ => GpuKernelParamAbi::OpaquePtr,
        }
    }

    fn type_abi_from_type_id(&self, ty: agam_sema::symbol::TypeId) -> Option<GpuKernelParamAbi> {
        match self.types.get(ty) {
            Type::Bool => Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I1)),
            Type::Char | Type::Int(IntSize::I32) | Type::UInt(IntSize::I32) => {
                Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I32))
            }
            Type::Int(IntSize::I8) | Type::UInt(IntSize::I8) => {
                Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I8))
            }
            Type::Int(IntSize::I16) | Type::UInt(IntSize::I16) => {
                Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I16))
            }
            Type::Int(IntSize::I64)
            | Type::UInt(IntSize::I64)
            | Type::Int(IntSize::ISize)
            | Type::UInt(IntSize::ISize) => {
                Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I64))
            }
            Type::Int(IntSize::I128) | Type::UInt(IntSize::I128) => {
                Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I128))
            }
            Type::Int(IntSize::I256) | Type::UInt(IntSize::I256) => {
                Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I256))
            }
            Type::Int(IntSize::I512) | Type::UInt(IntSize::I512) => {
                Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I512))
            }
            Type::Float(FloatSize::F32) => Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::F32)),
            Type::Float(FloatSize::F64) => Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::F64)),
            Type::Ref { inner, .. } | Type::Ptr { inner, .. } => self
                .type_abi_from_type_id(*inner)
                .map(GpuKernelParamAbi::pointer_to),
            Type::Slice(inner) | Type::Array { element: inner, .. } => self
                .type_abi_from_type_id(*inner)
                .map(GpuKernelParamAbi::pointer_to),
            _ => None,
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
            HirUnaryOp::Deref => match self.types.get(operand.ty) {
                Type::Ptr { inner, .. } | Type::Ref { inner, .. } => *inner,
                _ => operand.ty,
            },
            _ => operand.ty,
        }
    }

    fn pattern_name(&self, pattern: &agam_ast::pattern::Pattern) -> Option<String> {
        match &pattern.kind {
            agam_ast::pattern::PatternKind::Identifier { name, .. } => Some(name.name.clone()),
            _ => None,
        }
    }

    fn lower_pattern(&mut self, pattern: &agam_ast::pattern::Pattern) -> HirPattern {
        use agam_ast::pattern::PatternKind;
        match &pattern.kind {
            PatternKind::Wildcard => HirPattern::Wildcard,
            PatternKind::Identifier { name, .. } => HirPattern::Bind(name.name.clone()),
            PatternKind::Literal(expr) => HirPattern::Literal(self.lower_expr(expr)),
            PatternKind::Tuple(pats) => {
                HirPattern::Tuple(pats.iter().map(|p| self.lower_pattern(p)).collect())
            }
            PatternKind::Variant { path, fields } => {
                let name = path
                    .segments
                    .last()
                    .map(|s| s.name.clone())
                    .unwrap_or_default();
                HirPattern::Variant {
                    name,
                    fields: fields.iter().map(|f| self.lower_pattern(f)).collect(),
                }
            }
            PatternKind::Struct { path, fields, .. } => {
                let name = path
                    .segments
                    .last()
                    .map(|s| s.name.clone())
                    .unwrap_or_default();
                HirPattern::Struct {
                    name,
                    fields: fields
                        .iter()
                        .map(|f| {
                            let pat = f
                                .pattern
                                .as_ref()
                                .map(|p| self.lower_pattern(p))
                                .unwrap_or(HirPattern::Bind(f.name.name.clone()));
                            (f.name.name.clone(), pat)
                        })
                        .collect(),
                }
            }
            _ => HirPattern::Wildcard,
        }
    }
}

impl Default for HirLowering {
    fn default() -> Self {
        Self::new()
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

fn classify_gpu_kernel_param_abi(ty: &TypeExpr) -> GpuKernelParamAbi {
    match &ty.kind {
        TypeExprKind::Named(path) => path
            .segments
            .last()
            .and_then(|segment| GpuKernelParamAbi::scalar_from_name(&segment.name))
            .unwrap_or(GpuKernelParamAbi::OpaquePtr),
        TypeExprKind::Reference { inner, .. } | TypeExprKind::Pointer { inner, .. } => {
            classify_gpu_memory_wrapper_abi(inner)
        }
        TypeExprKind::Slice(inner) => classify_gpu_kernel_param_abi(inner).pointer_to(),
        TypeExprKind::Array { element, .. } => classify_gpu_kernel_param_abi(element).pointer_to(),
        _ => GpuKernelParamAbi::OpaquePtr,
    }
}

fn classify_gpu_memory_wrapper_abi(inner: &TypeExpr) -> GpuKernelParamAbi {
    match &inner.kind {
        TypeExprKind::Slice(element) => classify_gpu_kernel_param_abi(element).pointer_to(),
        TypeExprKind::Array { element, .. } => classify_gpu_kernel_param_abi(element).pointer_to(),
        _ => classify_gpu_kernel_param_abi(inner).pointer_to(),
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

fn path_name(path: &Path) -> String {
    path.segments
        .iter()
        .map(|segment| segment.name.as_str())
        .collect::<Vec<_>>()
        .join("::")
}

fn expr_name(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Identifier(ident) => Some(ident.name.clone()),
        ExprKind::PathExpr(path) => Some(path_name(path)),
        ExprKind::FieldAccess { object, field } => {
            let mut full = expr_name(object)?;
            full.push_str("::");
            full.push_str(&field.name);
            Some(full)
        }
        _ => None,
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

    #[test]
    fn test_lower_module_collects_nominal_layouts() {
        let hir = lower_source(
            "struct Point { x: i32, y: i32 }\nenum Color { Red, Green(i32), Rgb { r: i32, g: i32, b: i32 } }\nfn main(): return 0",
        );

        let point = hir
            .struct_layouts
            .get("Point")
            .expect("expected Point layout");
        assert_eq!(point.fields, vec!["x", "y"]);

        let color = hir
            .enum_layouts
            .get("Color")
            .expect("expected Color layout");
        assert_eq!(color.variants.len(), 3);
        assert_eq!(color.variants[0].name, "Red");
        assert_eq!(color.variants[0].tag, 0);
        assert!(!color.variants[0].has_payload);
        assert_eq!(color.variants[1].name, "Green");
        assert_eq!(color.variants[1].tag, 1);
        assert!(color.variants[1].has_payload);
        assert_eq!(color.variants[2].name, "Rgb");
        assert_eq!(color.variants[2].tag, 2);
        assert!(color.variants[2].has_payload);
    }

    fn lower_source_with_diagnostics(source: &str) -> (HirModule, Vec<String>) {
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
        let hir = lowering.lower_module(&module);
        (hir, lowering.diagnostics)
    }

    #[test]
    fn test_target_iot_propagates_to_hir_function() {
        let hir = lower_source("@target.iot\nfn main(): return 0");
        let f = &hir.functions[0];
        assert_eq!(f.target, TargetProfile::Iot);
    }

    #[test]
    fn test_target_hpc_propagates_to_hir_function() {
        let hir = lower_source("@target.hpc\nfn main(): return 0");
        let f = &hir.functions[0];
        assert_eq!(f.target, TargetProfile::Hpc);
    }

    #[test]
    fn test_default_target_when_no_annotation() {
        let hir = lower_source("fn main(): return 0");
        let f = &hir.functions[0];
        assert_eq!(f.target, TargetProfile::Default);
    }

    #[test]
    fn test_iot_rejects_perform_at_compile_time() {
        let (_, diagnostics) = lower_source_with_diagnostics(
            "@target.iot\nfn main(): perform Console.println(\"hello\")",
        );
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].contains("not allowed under @target.iot"));
        assert!(diagnostics[0].contains("Console.println"));
    }

    #[test]
    fn test_default_target_allows_perform() {
        let (_, diagnostics) =
            lower_source_with_diagnostics("fn main(): perform Console.println(\"hello\")");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_gpu_validation_rejects_effect_operations() {
        let (_, diagnostics) =
            lower_source_with_diagnostics("@gpu\nfn kern(): perform Console.println(1)");
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("effect perform/handle is not allowed")),
            "diagnostics: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_gpu_validation_rejects_string_operations() {
        let (_, diagnostics) = lower_source_with_diagnostics("@gpu\nfn kern(): let s = \"hello\"");
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("string operations are not allowed")),
            "diagnostics: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_gpu_validation_rejects_gpu_malloc_inside_kernel() {
        let (_, diagnostics) = lower_source_with_diagnostics("@gpu\nfn kern(): gpu_malloc(16)");
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("heap allocation is not allowed")),
            "diagnostics: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_gpu_validation_rejects_direct_recursion() {
        let (_, diagnostics) = lower_source_with_diagnostics("@gpu\nfn kern(): kern()");
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("recursion (`kern`) is not allowed")),
            "diagnostics: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_gpu_validation_rejects_non_scalar_return_types() {
        let (_, diagnostics) =
            lower_source_with_diagnostics("@gpu\nfn kern() -> *mut i32: return 0");
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("return type must be void or a scalar")),
            "diagnostics: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_gpu_validation_allows_shared_alloc_and_indexed_access() {
        let (_, diagnostics) = lower_source_with_diagnostics(
            "@gpu\nfn kern(input: [f32], output: *mut f32) { let scratch: [f32; 64] = agam.gpu.shared_alloc(64); let tid: i32 = agam.gpu.thread_id_x(); scratch[tid] = input[tid]; output[tid] = scratch[tid]; }",
        );
        assert!(diagnostics.is_empty(), "diagnostics: {:?}", diagnostics);
    }

    #[test]
    fn test_gpu_thread_id_call_uses_i32_type() {
        let hir = lower_source("@gpu\nfn kern(): let tid = agam.gpu.thread_id_x()");
        let builtins = TypeStore::new();
        let f = &hir.functions[0];
        match &f.body.stmts[0] {
            HirStmt::Let {
                value: Some(expr), ..
            } => assert_eq!(expr.ty, builtins.i32()),
            _ => panic!("expected let binding"),
        }
    }

    #[test]
    fn test_gpu_barrier_call_uses_unit_type() {
        let hir = lower_source("@gpu\nfn kern(): agam.gpu.barrier()");
        let builtins = TypeStore::new();
        let f = &hir.functions[0];
        match &f.body.stmts[0] {
            HirStmt::Expr(expr) => assert_eq!(expr.ty, builtins.unit()),
            _ => panic!("expected barrier expression statement"),
        }
    }

    #[test]
    fn test_gpu_math_call_uses_f32_type() {
        let hir = lower_source("@gpu\nfn kern(x: f32): let y = agam.gpu.sqrt(x)");
        let builtins = TypeStore::new();
        let f = &hir.functions[0];
        match &f.body.stmts[0] {
            HirStmt::Let {
                value: Some(expr), ..
            } => assert_eq!(expr.ty, builtins.f32()),
            _ => panic!("expected let binding"),
        }
    }

    #[test]
    fn test_gpu_warp_shuffle_down_call_uses_i32_type() {
        let hir = lower_source(
            "@gpu\nfn kern(mask: i32, value: i32, delta: i32, clamp: i32): let next = agam.gpu.warp_shuffle_down(mask, value, delta, clamp)",
        );
        let builtins = TypeStore::new();
        let f = &hir.functions[0];
        match &f.body.stmts[0] {
            HirStmt::Let {
                value: Some(expr), ..
            } => assert_eq!(expr.ty, builtins.i32()),
            _ => panic!("expected let binding"),
        }
    }

    #[test]
    fn test_gpu_ballot_sync_call_uses_i32_type() {
        let hir =
            lower_source("@gpu\nfn kern(mask: i32): let active = agam.gpu.ballot_sync(mask, true)");
        let builtins = TypeStore::new();
        let f = &hir.functions[0];
        match &f.body.stmts[0] {
            HirStmt::Let {
                value: Some(expr), ..
            } => assert_eq!(expr.ty, builtins.i32()),
            _ => panic!("expected let binding"),
        }
    }

    #[test]
    fn test_gpu_warp_reduce_add_call_uses_i32_type() {
        let hir = lower_source(
            "@gpu\nfn kern(value: i32): let reduced = agam.gpu.warp_reduce_add(value)",
        );
        let builtins = TypeStore::new();
        let f = &hir.functions[0];
        match &f.body.stmts[0] {
            HirStmt::Let {
                value: Some(expr), ..
            } => assert_eq!(expr.ty, builtins.i32()),
            _ => panic!("expected let binding"),
        }
    }

    #[test]
    fn test_gpu_builtin_wrong_arity_reports_diagnostic() {
        let (_, diagnostics) = lower_source_with_diagnostics(
            "@gpu\nfn kern(mask: i32, value: i32, delta: i32): agam.gpu.warp_shuffle_down(mask, value, delta)",
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("expects 4 arguments")),
            "diagnostics: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_gpu_param_abi_tracks_scalar_and_buffer_syntax() {
        let hir =
            lower_source("@gpu\nfn kern(input: [f32], output: *mut i32, scale: f32): return 0");
        let f = &hir.functions[0];
        assert_eq!(
            f.params[0].gpu_abi,
            GpuKernelParamAbi::Pointer {
                scalar: GpuKernelScalarAbi::F32,
                depth: 1,
            }
        );
        assert_eq!(
            f.params[1].gpu_abi,
            GpuKernelParamAbi::Pointer {
                scalar: GpuKernelScalarAbi::I32,
                depth: 1,
            }
        );
        assert_eq!(
            f.params[2].gpu_abi,
            GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::F32)
        );
    }

    #[test]
    fn test_gpu_pointer_deref_uses_pointee_type() {
        let hir = lower_source("@gpu\nfn kern(input: *mut f32): let value: f32 = *input");
        let builtins = TypeStore::new();
        let f = &hir.functions[0];
        match &f.body.stmts[0] {
            HirStmt::Let {
                value: Some(expr), ..
            } => assert_eq!(expr.ty, builtins.f32()),
            _ => panic!("expected let binding"),
        }
    }

    #[test]
    fn test_gpu_fixed_array_param_abi_tracks_buffer_syntax() {
        let hir = lower_source("@gpu\nfn kern(input: [f32; 64], output: *mut i32): return 0");
        let f = &hir.functions[0];
        assert_eq!(
            f.params[0].gpu_abi,
            GpuKernelParamAbi::Pointer {
                scalar: GpuKernelScalarAbi::F32,
                depth: 1,
            }
        );
        assert_eq!(
            f.params[1].gpu_abi,
            GpuKernelParamAbi::Pointer {
                scalar: GpuKernelScalarAbi::I32,
                depth: 1,
            }
        );
    }

    #[test]
    fn test_gpu_reference_wrapped_buffer_params_preserve_typed_pointer_abi() {
        let hir = lower_source("@gpu\nfn kern(input: &[f32], output: &mut [i32; 64]): return 0");
        let f = &hir.functions[0];
        assert_eq!(
            f.params[0].gpu_abi,
            GpuKernelParamAbi::Pointer {
                scalar: GpuKernelScalarAbi::F32,
                depth: 1,
            }
        );
        assert_eq!(
            f.params[1].gpu_abi,
            GpuKernelParamAbi::Pointer {
                scalar: GpuKernelScalarAbi::I32,
                depth: 1,
            }
        );
    }

    #[test]
    fn test_gpu_wide_integer_param_abi_tracks_256_and_512_bit_syntax() {
        let hir = lower_source("@gpu\nfn kern(a: i256, b: *mut u512): return 0");
        let f = &hir.functions[0];
        assert_eq!(
            f.params[0].gpu_abi,
            GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I256)
        );
        assert_eq!(
            f.params[1].gpu_abi,
            GpuKernelParamAbi::Pointer {
                scalar: GpuKernelScalarAbi::I512,
                depth: 1,
            }
        );
    }

    #[test]
    fn test_gpu_nested_pointer_param_abi_tracks_two_pointer_layers() {
        let hir = lower_source("@gpu\nfn kern(input: *mut *mut f32, output: &*mut i32): return 0");
        let f = &hir.functions[0];
        assert_eq!(
            f.params[0].gpu_abi,
            GpuKernelParamAbi::Pointer {
                scalar: GpuKernelScalarAbi::F32,
                depth: 2,
            }
        );
        assert_eq!(
            f.params[1].gpu_abi,
            GpuKernelParamAbi::Pointer {
                scalar: GpuKernelScalarAbi::I32,
                depth: 2,
            }
        );
    }

    #[test]
    fn test_gpu_shared_alloc_uses_annotated_pointer_element_abi() {
        let (hir, diagnostics) = lower_source_with_diagnostics(
            "@gpu\nfn kern(): let scratch: *mut f32 = agam.gpu.shared_alloc(128)",
        );
        assert!(diagnostics.is_empty(), "diagnostics: {:?}", diagnostics);
        let f = &hir.functions[0];
        match &f.body.stmts[0] {
            HirStmt::Let {
                value: Some(expr), ..
            } => match &expr.kind {
                HirExprKind::GpuSharedAlloc { element_abi, .. } => {
                    assert_eq!(
                        *element_abi,
                        GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::F32)
                    )
                }
                _ => panic!("expected gpu shared allocation expression"),
            },
            _ => panic!("expected let binding"),
        }
    }

    #[test]
    fn test_gpu_shared_alloc_uses_slice_element_abi() {
        let (hir, diagnostics) = lower_source_with_diagnostics(
            "@gpu\nfn kern(): let scratch: [f32] = agam.gpu.shared_alloc(128)",
        );
        assert!(diagnostics.is_empty(), "diagnostics: {:?}", diagnostics);
        let f = &hir.functions[0];
        match &f.body.stmts[0] {
            HirStmt::Let {
                value: Some(expr), ..
            } => match &expr.kind {
                HirExprKind::GpuSharedAlloc { element_abi, .. } => {
                    assert_eq!(
                        *element_abi,
                        GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::F32)
                    )
                }
                _ => panic!("expected gpu shared allocation expression"),
            },
            _ => panic!("expected let binding"),
        }
    }

    #[test]
    fn test_gpu_shared_alloc_uses_reference_wrapped_slice_element_abi() {
        let (hir, diagnostics) = lower_source_with_diagnostics(
            "@gpu\nfn kern(): let scratch: &mut [f32] = agam.gpu.shared_alloc(128)",
        );
        assert!(diagnostics.is_empty(), "diagnostics: {:?}", diagnostics);
        let f = &hir.functions[0];
        match &f.body.stmts[0] {
            HirStmt::Let {
                value: Some(expr), ..
            } => match &expr.kind {
                HirExprKind::GpuSharedAlloc { element_abi, .. } => {
                    assert_eq!(
                        *element_abi,
                        GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::F32)
                    )
                }
                _ => panic!("expected gpu shared allocation expression"),
            },
            _ => panic!("expected let binding"),
        }
    }

    #[test]
    fn test_gpu_shared_alloc_uses_pointer_element_slice_abi() {
        let (hir, diagnostics) = lower_source_with_diagnostics(
            "@gpu\nfn kern(): let scratch: [*mut f32] = agam.gpu.shared_alloc(128)",
        );
        assert!(diagnostics.is_empty(), "diagnostics: {:?}", diagnostics);
        let f = &hir.functions[0];
        match &f.body.stmts[0] {
            HirStmt::Let {
                value: Some(expr), ..
            } => match &expr.kind {
                HirExprKind::GpuSharedAlloc { element_abi, .. } => {
                    assert_eq!(
                        *element_abi,
                        GpuKernelParamAbi::Pointer {
                            scalar: GpuKernelScalarAbi::F32,
                            depth: 1,
                        }
                    )
                }
                _ => panic!("expected gpu shared allocation expression"),
            },
            _ => panic!("expected let binding"),
        }
    }

    #[test]
    fn test_gpu_shared_alloc_uses_fixed_array_element_abi() {
        let (hir, diagnostics) = lower_source_with_diagnostics(
            "@gpu\nfn kern(): let scratch: [i32; 128] = agam.gpu.shared_alloc(128)",
        );
        assert!(diagnostics.is_empty(), "diagnostics: {:?}", diagnostics);
        let f = &hir.functions[0];
        match &f.body.stmts[0] {
            HirStmt::Let {
                value: Some(expr), ..
            } => match &expr.kind {
                HirExprKind::GpuSharedAlloc { element_abi, .. } => {
                    assert_eq!(
                        *element_abi,
                        GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I32)
                    )
                }
                _ => panic!("expected gpu shared allocation expression"),
            },
            _ => panic!("expected let binding"),
        }
    }

    #[test]
    fn test_gpu_shared_alloc_uses_wide_fixed_array_element_abi() {
        let (hir, diagnostics) = lower_source_with_diagnostics(
            "@gpu\nfn kern(): let scratch: [u256; 32] = agam.gpu.shared_alloc(32)",
        );
        assert!(diagnostics.is_empty(), "diagnostics: {:?}", diagnostics);
        let f = &hir.functions[0];
        match &f.body.stmts[0] {
            HirStmt::Let {
                value: Some(expr), ..
            } => match &expr.kind {
                HirExprKind::GpuSharedAlloc { element_abi, .. } => {
                    assert_eq!(
                        *element_abi,
                        GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I256)
                    )
                }
                _ => panic!("expected gpu shared allocation expression"),
            },
            _ => panic!("expected let binding"),
        }
    }
}
