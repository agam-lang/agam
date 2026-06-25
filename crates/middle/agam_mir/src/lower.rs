//! HIR → MIR lowering pass.
//!
//! Transforms the high-level HIR into basic-block-based MIR with SSA values.

use std::collections::HashMap;

use agam_hir::nodes::*;
use agam_sema::gpu::{GpuBuiltin, GpuKernelConfig, resolve_gpu_builtin};
use agam_sema::symbol::TypeId;
use agam_sema::types::TypeStore;

use crate::ir::*;

/// The MIR lowering context.
pub struct MirLowering {
    next_value: u32,
    next_block: u32,
    blocks: Vec<BasicBlock>,
    current_instrs: Vec<Instruction>,
    current_block: BlockId,
    gpu_kernels: HashMap<String, GpuKernelConfig>,
    enum_layouts: HashMap<String, EnumLayout>,
    struct_layouts: HashMap<String, StructLayout>,
    types: TypeStore,
}

impl MirLowering {
    pub fn new() -> Self {
        Self {
            next_value: 0,
            next_block: 0,
            blocks: Vec::new(),
            current_instrs: Vec::new(),
            current_block: BlockId(0),
            gpu_kernels: HashMap::new(),
            enum_layouts: HashMap::new(),
            struct_layouts: HashMap::new(),
            types: TypeStore::new(),
        }
    }

    fn fresh_value(&mut self) -> ValueId {
        let id = ValueId(self.next_value);
        self.next_value += 1;
        id
    }

    fn fresh_block(&mut self) -> BlockId {
        let id = BlockId(self.next_block);
        self.next_block += 1;
        id
    }

    fn emit(&mut self, ty: TypeId, op: Op) -> ValueId {
        let result = self.fresh_value();
        self.current_instrs.push(Instruction { result, ty, op });
        result
    }

    fn finish_block(&mut self, terminator: Terminator) {
        let instrs = std::mem::take(&mut self.current_instrs);
        self.blocks.push(BasicBlock {
            id: self.current_block,
            instructions: instrs,
            terminator,
        });
    }

    /// Lower an entire HIR module into MIR.
    pub fn lower_module(&mut self, hir: &HirModule) -> MirModule {
        self.gpu_kernels = hir
            .functions
            .iter()
            .filter_map(|function| {
                function
                    .gpu_config
                    .clone()
                    .map(|config| (function.name.clone(), config))
            })
            .collect();
        self.enum_layouts = hir
            .enum_layouts
            .iter()
            .map(|(name, layout)| {
                (
                    name.clone(),
                    EnumLayout {
                        name: layout.name.clone(),
                        variants: layout
                            .variants
                            .iter()
                            .map(|variant| EnumVariantLayout {
                                name: variant.name.clone(),
                                tag: variant.tag,
                                has_payload: variant.has_payload,
                            })
                            .collect(),
                    },
                )
            })
            .collect();
        self.struct_layouts = hir
            .struct_layouts
            .iter()
            .map(|(name, layout)| {
                (
                    name.clone(),
                    StructLayout {
                        name: layout.name.clone(),
                        fields: layout.fields.clone(),
                    },
                )
            })
            .collect();
        let functions = hir
            .functions
            .iter()
            .map(|f| self.lower_function(f))
            .collect();
        MirModule {
            functions,
            enum_layouts: self.enum_layouts.clone(),
            struct_layouts: self.struct_layouts.clone(),
        }
    }

    fn lower_function(&mut self, func: &HirFunction) -> MirFunction {
        self.blocks.clear();
        self.current_instrs.clear();

        let entry = self.fresh_block();
        self.current_block = entry;

        // Emit parameter allocas
        let params: Vec<MirParam> = func
            .params
            .iter()
            .map(|p| {
                let v = self.fresh_value();
                MirParam {
                    name: p.name.clone(),
                    value: v,
                    ty: p.ty,
                    gpu_abi: p.gpu_abi,
                }
            })
            .collect();

        // Lower body
        let result = self.lower_block(&func.body);

        // Finish the last block with return
        match result {
            Some(val) => self.finish_block(Terminator::Return(val)),
            None => {
                let unit = self.emit(self.types.unit(), Op::Unit);
                self.finish_block(Terminator::Return(unit));
            }
        }

        MirFunction {
            name: func.name.clone(),
            generics: func.generics.iter().map(|g| g.name.clone()).collect(),
            params,
            return_ty: func.return_ty,
            blocks: std::mem::take(&mut self.blocks),
            entry,
            target: func.target,
            gpu_config: func.gpu_config.clone(),
        }
    }

    fn lower_block(&mut self, block: &HirBlock) -> Option<ValueId> {
        for stmt in &block.stmts {
            self.lower_stmt(stmt);
        }
        block.expr.as_ref().map(|e| self.lower_expr(e))
    }

    fn lower_stmt(&mut self, stmt: &HirStmt) {
        match stmt {
            HirStmt::Let {
                name, ty, value, ..
            } => {
                self.emit(
                    *ty,
                    Op::Alloca {
                        name: name.clone(),
                        ty: *ty,
                    },
                );
                if let Some(val_expr) = value {
                    let val = self.lower_expr(val_expr);
                    self.emit(
                        *ty,
                        Op::StoreLocal {
                            name: name.clone(),
                            value: val,
                        },
                    );
                }
            }
            HirStmt::Expr(expr) => {
                self.lower_expr(expr);
            }
            HirStmt::Return(val) => {
                let v = if let Some(v) = val {
                    self.lower_expr(v)
                } else {
                    self.emit(self.types.unit(), Op::Unit)
                };
                self.finish_block(Terminator::Return(v));
                self.current_block = self.fresh_block(); // unreachable block
            }
            HirStmt::While { condition, body } => {
                let cond_block = self.fresh_block();
                let body_block = self.fresh_block();
                let after_block = self.fresh_block();

                self.finish_block(Terminator::Jump(cond_block));
                self.current_block = cond_block;

                let cond_val = self.lower_expr(condition);
                self.finish_block(Terminator::Branch {
                    condition: cond_val,
                    then_block: body_block,
                    else_block: after_block,
                });

                self.current_block = body_block;
                self.lower_block(body);
                self.finish_block(Terminator::Jump(cond_block));

                self.current_block = after_block;
            }
            HirStmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let then_block = self.fresh_block();
                let else_block = self.fresh_block();
                let after_block = self.fresh_block();

                let cond_val = self.lower_expr(condition);

                let target_else = if else_branch.is_some() {
                    else_block
                } else {
                    after_block
                };
                self.finish_block(Terminator::Branch {
                    condition: cond_val,
                    then_block,
                    else_block: target_else,
                });

                self.current_block = then_block;
                self.lower_block(then_branch);
                self.finish_block(Terminator::Jump(after_block));

                if let Some(eb) = else_branch {
                    self.current_block = else_block;
                    self.lower_block(eb);
                    self.finish_block(Terminator::Jump(after_block));
                }

                self.current_block = after_block;
            }
            _ => {}
        }
    }

    fn lower_expr(&mut self, expr: &HirExpr) -> ValueId {
        let ty = expr.ty;
        match &expr.kind {
            HirExprKind::IntLit(v) => self.emit(ty, Op::ConstInt(*v)),
            HirExprKind::FloatLit(v) => self.emit(ty, Op::ConstFloat(*v)),
            HirExprKind::BoolLit(v) => self.emit(ty, Op::ConstBool(*v)),
            HirExprKind::StringLit(v) => self.emit(ty, Op::ConstString(v.clone())),

            HirExprKind::Var(name) => self.emit(ty, Op::LoadLocal(name.clone())),

            HirExprKind::Binary { op, left, right } => {
                let l = self.lower_expr(left);
                let r = self.lower_expr(right);
                self.emit(
                    ty,
                    Op::BinOp {
                        op: lower_binop(*op),
                        left: l,
                        right: r,
                    },
                )
            }
            HirExprKind::Unary { op, operand } => {
                let v = self.lower_expr(operand);
                if matches!(op, HirUnaryOp::Deref) {
                    let zero = self.emit(self.types.i32(), Op::ConstInt(0));
                    return self.emit(
                        ty,
                        Op::GetIndex {
                            object: v,
                            index: zero,
                        },
                    );
                }
                self.emit(
                    ty,
                    Op::UnOp {
                        op: lower_unop(*op),
                        operand: v,
                    },
                )
            }

            HirExprKind::Call { callee, args } => {
                let arg_vals: Vec<ValueId> = args.iter().map(|a| self.lower_expr(a)).collect();
                if let Some(callee_name) = gpu_callee_name(callee)
                    && let Some(builtin) = resolve_gpu_builtin(&callee_name)
                {
                    return self.emit(
                        ty,
                        Op::GpuIntrinsic {
                            kind: lower_gpu_builtin(builtin),
                            args: arg_vals,
                        },
                    );
                }
                let callee_name = match &callee.kind {
                    HirExprKind::Var(name) => name.clone(),
                    _ => "__indirect_call".into(),
                };
                if let Some(config) = self.gpu_kernel_launch_config(callee) {
                    let grid = self.emit(
                        self.types.i32(),
                        Op::ConstInt(config.grid_dim.map(|(x, _, _)| x).unwrap_or(1) as i64),
                    );
                    let block = self.emit(
                        self.types.i32(),
                        Op::ConstInt(config.threads_per_block as i64),
                    );
                    return self.emit(
                        ty,
                        Op::GpuKernelLaunch {
                            kernel_name: callee_name,
                            grid,
                            block,
                            shared_memory_bytes: config.shared_memory_bytes,
                            args: arg_vals,
                        },
                    );
                }
                self.emit(
                    ty,
                    Op::Call {
                        callee: callee_name,
                        args: arg_vals,
                    },
                )
            }

            HirExprKind::MethodCall {
                object,
                method,
                args,
            } => {
                let obj_val = self.lower_expr(object);
                let mut all_args = vec![obj_val];
                all_args.extend(args.iter().map(|a| self.lower_expr(a)));
                self.emit(
                    ty,
                    Op::Call {
                        callee: method.clone(),
                        args: all_args,
                    },
                )
            }
            HirExprKind::GpuSharedAlloc { element_abi, count } => {
                let count = self.lower_expr(count);
                self.emit(
                    ty,
                    Op::GpuSharedAlloc {
                        element_abi: *element_abi,
                        count,
                    },
                )
            }

            HirExprKind::FieldAccess { object, field } => {
                let obj = self.lower_expr(object);
                self.emit(
                    ty,
                    Op::GetField {
                        object: obj,
                        field: field.clone(),
                    },
                )
            }
            HirExprKind::Index { object, index } => {
                let obj = self.lower_expr(object);
                let idx = self.lower_expr(index);
                self.emit(
                    ty,
                    Op::GetIndex {
                        object: obj,
                        index: idx,
                    },
                )
            }

            HirExprKind::Assign { target, value } => {
                let val = self.lower_expr(value);
                match &target.kind {
                    HirExprKind::Var(name) => self.emit(
                        ty,
                        Op::StoreLocal {
                            name: name.clone(),
                            value: val,
                        },
                    ),
                    HirExprKind::Index { object, index } => {
                        let object = self.lower_expr(object);
                        let index = self.lower_expr(index);
                        self.emit(
                            ty,
                            Op::StoreIndex {
                                object,
                                index,
                                value: val,
                            },
                        )
                    }
                    _ => val,
                }
            }

            HirExprKind::Array(elems) | HirExprKind::Tuple(elems) => {
                for e in elems {
                    self.lower_expr(e);
                }
                self.emit(ty, Op::Unit)
            }

            HirExprKind::Block(block) => self
                .lower_block(block)
                .unwrap_or_else(|| self.emit(ty, Op::Unit)),

            HirExprKind::Cast {
                expr: inner,
                target_ty,
            } => {
                let v = self.lower_expr(inner);
                self.emit(
                    *target_ty,
                    Op::Cast {
                        value: v,
                        target_ty: *target_ty,
                    },
                )
            }

            HirExprKind::Perform {
                effect,
                operation,
                args,
            } => {
                let arg_vals: Vec<ValueId> = args.iter().map(|a| self.lower_expr(a)).collect();
                self.emit(
                    ty,
                    Op::EffectPerform {
                        effect: effect.clone(),
                        operation: operation.clone(),
                        args: arg_vals,
                    },
                )
            }

            HirExprKind::HandleWith {
                effect,
                handler,
                body,
            } => {
                let body_block = self.fresh_block();
                self.emit(
                    ty,
                    Op::HandleWith {
                        effect: effect.clone(),
                        handler: handler.clone(),
                        body: body_block,
                    },
                );
                self.lower_expr(body)
            }
            agam_hir::nodes::HirExprKind::StructLiteral { name, fields } => {
                let mut args = Vec::new();
                for (_, f) in fields {
                    args.push(self.lower_expr(f));
                }
                self.emit(
                    ty,
                    Op::Call {
                        callee: format!("__struct_init_{}", name),
                        args,
                    },
                )
            }
            agam_hir::nodes::HirExprKind::EnumVariant {
                enum_name,
                variant,
                fields,
            } => {
                let payload = fields.iter().map(|f| self.lower_expr(f)).collect();
                let tag = self.enum_variant_tag(enum_name, variant);
                self.emit(ty, Op::EnumConstruct { tag, payload })
            }
            agam_hir::nodes::HirExprKind::Match { scrutinee, arms } => {
                self.lower_match_expr(ty, scrutinee, arms)
            }
        }
    }

    fn lower_match_expr(
        &mut self,
        ty: TypeId,
        scrutinee: &HirExpr,
        arms: &[HirMatchArm],
    ) -> ValueId {
        if arms.is_empty() {
            return self.emit(ty, Op::Unit);
        }

        let result_local = format!("__match_result_{}", self.next_value);
        self.emit(
            ty,
            Op::Alloca {
                name: result_local.clone(),
                ty,
            },
        );

        let scrut_val = self.lower_expr(scrutinee);
        let end_block = self.fresh_block();

        for arm in arms {
            let body_block = self.fresh_block();
            let next_arm_block = self.fresh_block();
            let condition = self.lower_match_condition(scrut_val, &arm.pattern, arm.guard.as_ref());

            self.finish_block(Terminator::Branch {
                condition,
                then_block: body_block,
                else_block: next_arm_block,
            });

            self.current_block = body_block;
            self.lower_match_pattern_bindings(scrut_val, scrutinee.ty, &arm.pattern);
            let body_value = self.lower_expr(&arm.body);
            self.emit(
                ty,
                Op::StoreLocal {
                    name: result_local.clone(),
                    value: body_value,
                },
            );
            self.finish_block(Terminator::Jump(end_block));

            self.current_block = next_arm_block;
        }

        self.finish_block(Terminator::Jump(end_block));
        self.current_block = end_block;
        self.emit(ty, Op::LoadLocal(result_local))
    }

    fn lower_match_condition(
        &mut self,
        scrutinee: ValueId,
        pattern: &HirPattern,
        guard: Option<&HirExpr>,
    ) -> ValueId {
        let bool_ty = self.types.bool();
        let pattern_condition = match pattern {
            HirPattern::Literal(lit_expr) => {
                let lit_val = self.lower_expr(lit_expr);
                self.emit(
                    bool_ty,
                    Op::BinOp {
                        op: MirBinOp::Eq,
                        left: scrutinee,
                        right: lit_val,
                    },
                )
            }
            HirPattern::Variant { name, .. } => {
                let tag = self
                    .enum_variant_tag_by_name(name)
                    .map(i64::from)
                    .unwrap_or(0);
                let tag_ty = self.types.i32();
                let actual_tag = self.emit(tag_ty, Op::EnumTag(scrutinee));
                let expected_tag = self.emit(tag_ty, Op::ConstInt(tag));
                self.emit(
                    bool_ty,
                    Op::BinOp {
                        op: MirBinOp::Eq,
                        left: actual_tag,
                        right: expected_tag,
                    },
                )
            }
            HirPattern::Wildcard | HirPattern::Bind(_) => self.emit(bool_ty, Op::ConstBool(true)),
            _ => self.emit(bool_ty, Op::ConstBool(true)),
        };

        if let Some(guard) = guard {
            let guard_value = self.lower_expr(guard);
            self.emit(
                bool_ty,
                Op::BinOp {
                    op: MirBinOp::And,
                    left: pattern_condition,
                    right: guard_value,
                },
            )
        } else {
            pattern_condition
        }
    }

    fn lower_match_pattern_bindings(
        &mut self,
        scrutinee: ValueId,
        scrutinee_ty: TypeId,
        pattern: &HirPattern,
    ) {
        match pattern {
            HirPattern::Bind(name) => {
                self.emit(
                    scrutinee_ty,
                    Op::Alloca {
                        name: name.clone(),
                        ty: scrutinee_ty,
                    },
                );
                self.emit(
                    scrutinee_ty,
                    Op::StoreLocal {
                        name: name.clone(),
                        value: scrutinee,
                    },
                );
            }
            HirPattern::Variant { fields, .. } => {
                for (index, field) in fields.iter().enumerate() {
                    let payload_ty = self.types.any();
                    let payload = self.emit(
                        payload_ty,
                        Op::EnumPayload {
                            value: scrutinee,
                            field_index: index as u32,
                        },
                    );
                    self.lower_payload_pattern_bindings(payload, payload_ty, field);
                }
            }
            _ => {}
        }
    }

    fn lower_payload_pattern_bindings(
        &mut self,
        payload: ValueId,
        payload_ty: TypeId,
        pattern: &HirPattern,
    ) {
        if let HirPattern::Bind(name) = pattern {
            self.emit(
                payload_ty,
                Op::Alloca {
                    name: name.clone(),
                    ty: payload_ty,
                },
            );
            self.emit(
                payload_ty,
                Op::StoreLocal {
                    name: name.clone(),
                    value: payload,
                },
            );
        }
    }
}

impl MirLowering {
    fn gpu_kernel_launch_config(&self, callee: &HirExpr) -> Option<GpuKernelConfig> {
        let callee_name = gpu_callee_name(callee)?;
        self.gpu_kernels.get(&callee_name).cloned().or_else(|| {
            callee_name
                .rsplit("::")
                .next()
                .and_then(|name| self.gpu_kernels.get(name).cloned())
        })
    }

    fn enum_variant_tag(&self, enum_name: &str, variant_name: &str) -> u32 {
        self.enum_layouts
            .get(enum_name)
            .and_then(|layout| {
                layout
                    .variants
                    .iter()
                    .find(|variant| variant.name == variant_name)
            })
            .map(|variant| variant.tag)
            .unwrap_or(0)
    }

    fn enum_variant_tag_by_name(&self, variant_name: &str) -> Option<u32> {
        self.enum_layouts
            .values()
            .find_map(|layout| {
                layout
                    .variants
                    .iter()
                    .find(|variant| variant.name == variant_name)
            })
            .map(|variant| variant.tag)
    }
}

impl Default for MirLowering {
    fn default() -> Self {
        Self::new()
    }
}

fn lower_binop(op: HirBinOp) -> MirBinOp {
    match op {
        HirBinOp::Add => MirBinOp::Add,
        HirBinOp::Sub => MirBinOp::Sub,
        HirBinOp::Mul => MirBinOp::Mul,
        HirBinOp::Div => MirBinOp::Div,
        HirBinOp::Mod => MirBinOp::Mod,
        HirBinOp::Eq => MirBinOp::Eq,
        HirBinOp::NotEq => MirBinOp::NotEq,
        HirBinOp::Lt => MirBinOp::Lt,
        HirBinOp::LtEq => MirBinOp::LtEq,
        HirBinOp::Gt => MirBinOp::Gt,
        HirBinOp::GtEq => MirBinOp::GtEq,
        HirBinOp::And => MirBinOp::And,
        HirBinOp::Or => MirBinOp::Or,
        HirBinOp::BitAnd => MirBinOp::BitAnd,
        HirBinOp::BitOr => MirBinOp::BitOr,
        HirBinOp::BitXor => MirBinOp::BitXor,
        HirBinOp::Shl => MirBinOp::Shl,
        HirBinOp::Shr => MirBinOp::Shr,
        HirBinOp::Pow => MirBinOp::Mul, // Simplified for now
    }
}

fn lower_unop(op: HirUnaryOp) -> MirUnOp {
    match op {
        HirUnaryOp::Neg => MirUnOp::Neg,
        HirUnaryOp::Not => MirUnOp::Not,
        HirUnaryOp::BitNot => MirUnOp::BitNot,
        _ => MirUnOp::Not, // Ref/Deref handled at higher level
    }
}

fn lower_gpu_builtin(builtin: GpuBuiltin) -> GpuIntrinsicKind {
    match builtin {
        GpuBuiltin::ThreadIdX => GpuIntrinsicKind::ThreadIdX,
        GpuBuiltin::ThreadIdY => GpuIntrinsicKind::ThreadIdY,
        GpuBuiltin::ThreadIdZ => GpuIntrinsicKind::ThreadIdZ,
        GpuBuiltin::BlockIdX => GpuIntrinsicKind::BlockIdX,
        GpuBuiltin::BlockIdY => GpuIntrinsicKind::BlockIdY,
        GpuBuiltin::BlockIdZ => GpuIntrinsicKind::BlockIdZ,
        GpuBuiltin::BlockDimX => GpuIntrinsicKind::BlockDimX,
        GpuBuiltin::BlockDimY => GpuIntrinsicKind::BlockDimY,
        GpuBuiltin::BlockDimZ => GpuIntrinsicKind::BlockDimZ,
        GpuBuiltin::Barrier => GpuIntrinsicKind::Barrier,
        GpuBuiltin::SharedAlloc => {
            unreachable!("shared_alloc is lowered through HirExprKind::GpuSharedAlloc")
        }
        GpuBuiltin::WarpShuffleDown => GpuIntrinsicKind::WarpShuffleDown,
        GpuBuiltin::WarpReduceAdd => GpuIntrinsicKind::WarpReduceAdd,
        GpuBuiltin::BallotSync => GpuIntrinsicKind::BallotSync,
        GpuBuiltin::Sin => GpuIntrinsicKind::NvvmSin,
        GpuBuiltin::Cos => GpuIntrinsicKind::NvvmCos,
        GpuBuiltin::Sqrt => GpuIntrinsicKind::NvvmSqrt,
        GpuBuiltin::Exp => GpuIntrinsicKind::NvvmExp,
    }
}

fn gpu_callee_name(expr: &HirExpr) -> Option<String> {
    match &expr.kind {
        HirExprKind::Var(name) => Some(name.clone()),
        HirExprKind::FieldAccess { object, field } => {
            let mut full = gpu_callee_name(object)?;
            full.push_str("::");
            full.push_str(field);
            Some(full)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agam_errors::span::SourceId;
    use agam_hir::lower::HirLowering;
    use agam_lexer::Lexer;

    fn lower_to_mir(source: &str) -> MirModule {
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

        let mut hir_lower = HirLowering::new();
        let hir = hir_lower.lower_module(&module);

        let mut mir_lower = MirLowering::new();
        mir_lower.lower_module(&hir)
    }

    #[test]
    fn test_mir_simple_function() {
        let mir = lower_to_mir("fn main(): return 42");
        assert_eq!(mir.functions.len(), 1);
        assert_eq!(mir.functions[0].name, "main");
        assert!(!mir.functions[0].blocks.is_empty());
    }

    #[test]
    fn test_mir_has_const_int() {
        let mir = lower_to_mir("fn main(): return 42");
        let f = &mir.functions[0];
        let has_int = f.blocks.iter().any(|b| {
            b.instructions
                .iter()
                .any(|i| matches!(&i.op, Op::ConstInt(42)))
        });
        assert!(has_int, "expected ConstInt(42) in MIR");
    }

    #[test]
    fn test_mir_binary_op() {
        let mir = lower_to_mir("fn main(): let x = 1 + 2");
        let f = &mir.functions[0];
        let has_add = f.blocks.iter().any(|b| {
            b.instructions.iter().any(|i| {
                matches!(
                    &i.op,
                    Op::BinOp {
                        op: MirBinOp::Add,
                        ..
                    }
                )
            })
        });
        assert!(has_add, "expected BinOp::Add in MIR");
    }

    #[test]
    fn test_mir_function_call() {
        let mir = lower_to_mir("fn main(): print(42)");
        let f = &mir.functions[0];
        let has_call = f.blocks.iter().any(|b| {
            b.instructions
                .iter()
                .any(|i| matches!(&i.op, Op::Call { callee, .. } if callee == "print"))
        });
        assert!(has_call, "expected Call to 'print' in MIR");
    }

    #[test]
    fn test_mir_return_terminates() {
        let mir = lower_to_mir("fn main(): return 42");
        let f = &mir.functions[0];
        let entry = &f.blocks[0];
        assert!(matches!(&entry.terminator, Terminator::Return(_)));
    }

    #[test]
    fn test_mir_effect_perform() {
        let mir = lower_to_mir("fn main(): perform FileSystem.exists(\".\")");
        let f = &mir.functions[0];
        let has_perform = f.blocks.iter().any(|b| {
            b.instructions.iter().any(|i| {
                matches!(
                    &i.op,
                    Op::EffectPerform {
                        effect,
                        operation,
                        ..
                    } if effect == "FileSystem" && operation == "exists"
                )
            })
        });
        assert!(
            has_perform,
            "expected EffectPerform for FileSystem.exists in MIR"
        );
    }

    #[test]
    fn test_mir_gpu_thread_id_call_lowers_to_intrinsic() {
        let mir = lower_to_mir("@gpu\nfn kern(): let tid = agam.gpu.thread_id_x()");
        let f = &mir.functions[0];
        let has_intrinsic = f.blocks.iter().any(|b| {
            b.instructions.iter().any(|i| {
                matches!(
                    &i.op,
                    Op::GpuIntrinsic {
                        kind: GpuIntrinsicKind::ThreadIdX,
                        ..
                    }
                )
            })
        });
        assert!(has_intrinsic, "expected GpuIntrinsic::ThreadIdX in MIR");
    }

    #[test]
    fn test_mir_gpu_barrier_call_lowers_to_intrinsic() {
        let mir = lower_to_mir("@gpu\nfn kern(): agam.gpu.barrier()");
        let f = &mir.functions[0];
        let has_intrinsic = f.blocks.iter().any(|b| {
            b.instructions.iter().any(|i| {
                matches!(
                    &i.op,
                    Op::GpuIntrinsic {
                        kind: GpuIntrinsicKind::Barrier,
                        ..
                    }
                )
            })
        });
        assert!(has_intrinsic, "expected GpuIntrinsic::Barrier in MIR");
    }

    #[test]
    fn test_mir_gpu_math_call_lowers_to_intrinsic() {
        let mir = lower_to_mir("@gpu\nfn kern(x: f32): let y = agam.gpu.sqrt(x)");
        let f = &mir.functions[0];
        let has_intrinsic = f.blocks.iter().any(|b| {
            b.instructions.iter().any(|i| {
                matches!(
                    &i.op,
                    Op::GpuIntrinsic {
                        kind: GpuIntrinsicKind::NvvmSqrt,
                        ..
                    }
                )
            })
        });
        assert!(has_intrinsic, "expected GpuIntrinsic::NvvmSqrt in MIR");
    }

    #[test]
    fn test_mir_gpu_warp_shuffle_down_call_lowers_to_intrinsic() {
        let mir = lower_to_mir(
            "@gpu\nfn kern(mask: i32, value: i32, delta: i32, clamp: i32): let next = agam.gpu.warp_shuffle_down(mask, value, delta, clamp)",
        );
        let f = &mir.functions[0];
        let has_intrinsic = f.blocks.iter().any(|b| {
            b.instructions.iter().any(|i| {
                matches!(
                    &i.op,
                    Op::GpuIntrinsic {
                        kind: GpuIntrinsicKind::WarpShuffleDown,
                        ..
                    }
                )
            })
        });
        assert!(
            has_intrinsic,
            "expected GpuIntrinsic::WarpShuffleDown in MIR"
        );
    }

    #[test]
    fn test_mir_gpu_ballot_sync_call_lowers_to_intrinsic() {
        let mir =
            lower_to_mir("@gpu\nfn kern(mask: i32): let active = agam.gpu.ballot_sync(mask, true)");
        let f = &mir.functions[0];
        let has_intrinsic = f.blocks.iter().any(|b| {
            b.instructions.iter().any(|i| {
                matches!(
                    &i.op,
                    Op::GpuIntrinsic {
                        kind: GpuIntrinsicKind::BallotSync,
                        ..
                    }
                )
            })
        });
        assert!(has_intrinsic, "expected GpuIntrinsic::BallotSync in MIR");
    }

    #[test]
    fn test_mir_gpu_warp_reduce_add_call_lowers_to_intrinsic() {
        let mir = lower_to_mir(
            "@gpu\nfn kern(value: i32): let reduced = agam.gpu.warp_reduce_add(value)",
        );
        let f = &mir.functions[0];
        let has_intrinsic = f.blocks.iter().any(|b| {
            b.instructions.iter().any(|i| {
                matches!(
                    &i.op,
                    Op::GpuIntrinsic {
                        kind: GpuIntrinsicKind::WarpReduceAdd,
                        ..
                    }
                )
            })
        });
        assert!(has_intrinsic, "expected GpuIntrinsic::WarpReduceAdd in MIR");
    }

    #[test]
    fn test_mir_gpu_indexed_assignment_lowers_to_store_index() {
        let mir = lower_to_mir(
            "@gpu\nfn kern(input: [f32], output: *mut f32) { let tid = agam.gpu.thread_id_x(); output[tid] = input[tid]; }",
        );
        let f = &mir.functions[0];
        let has_index_load = f.blocks.iter().any(|b| {
            b.instructions
                .iter()
                .any(|i| matches!(&i.op, Op::GetIndex { .. }))
        });
        let has_index_store = f.blocks.iter().any(|b| {
            b.instructions
                .iter()
                .any(|i| matches!(&i.op, Op::StoreIndex { .. }))
        });
        assert!(has_index_load, "expected indexed GPU buffer read in MIR");
        assert!(has_index_store, "expected indexed GPU buffer write in MIR");
    }

    #[test]
    fn test_mir_pointer_deref_lowers_to_zero_index_load() {
        let mir = lower_to_mir("@gpu\nfn kern(input: *mut f32): let value: f32 = *input");
        let f = &mir.functions[0];
        let has_zero_offset = f.blocks.iter().any(|b| {
            b.instructions
                .iter()
                .any(|i| matches!(&i.op, Op::ConstInt(0)))
        });
        let has_deref_load = f.blocks.iter().any(|b| {
            b.instructions
                .iter()
                .any(|i| matches!(&i.op, Op::GetIndex { .. }))
        });
        assert!(has_zero_offset, "expected implicit zero offset for deref");
        assert!(has_deref_load, "expected pointer deref to reuse GetIndex");
    }

    #[test]
    fn test_mir_gpu_shared_alloc_lowers_to_explicit_op() {
        let mir =
            lower_to_mir("@gpu\nfn kern(): let scratch: *mut f32 = agam.gpu.shared_alloc(128)");
        let f = &mir.functions[0];
        let has_shared_alloc = f.blocks.iter().any(|b| {
            b.instructions.iter().any(|i| {
                matches!(
                    &i.op,
                    Op::GpuSharedAlloc {
                        element_abi: agam_sema::gpu::GpuKernelParamAbi::Scalar(
                            agam_sema::gpu::GpuKernelScalarAbi::F32
                        ),
                        ..
                    }
                )
            })
        });
        assert!(
            has_shared_alloc,
            "expected explicit GPU shared allocation in MIR"
        );
    }

    #[test]
    fn test_mir_gpu_fixed_array_shared_alloc_lowers_to_explicit_op() {
        let mir =
            lower_to_mir("@gpu\nfn kern(): let scratch: [i32; 128] = agam.gpu.shared_alloc(128)");
        let f = &mir.functions[0];
        let has_shared_alloc = f.blocks.iter().any(|b| {
            b.instructions.iter().any(|i| {
                matches!(
                    &i.op,
                    Op::GpuSharedAlloc {
                        element_abi: agam_sema::gpu::GpuKernelParamAbi::Scalar(
                            agam_sema::gpu::GpuKernelScalarAbi::I32
                        ),
                        ..
                    }
                )
            })
        });
        assert!(
            has_shared_alloc,
            "expected fixed-array GPU shared allocation in MIR"
        );
    }

    #[test]
    fn test_mir_gpu_pointer_element_shared_alloc_lowers_to_explicit_op() {
        let mir =
            lower_to_mir("@gpu\nfn kern(): let scratch: [*mut f32] = agam.gpu.shared_alloc(128)");
        let f = &mir.functions[0];
        let has_shared_alloc = f.blocks.iter().any(|b| {
            b.instructions.iter().any(|i| {
                matches!(
                    &i.op,
                    Op::GpuSharedAlloc {
                        element_abi: agam_sema::gpu::GpuKernelParamAbi::Pointer {
                            scalar: agam_sema::gpu::GpuKernelScalarAbi::F32,
                            depth: 1,
                        },
                        ..
                    }
                )
            })
        });
        assert!(
            has_shared_alloc,
            "expected pointer-element GPU shared allocation in MIR"
        );
    }

    #[test]
    fn test_host_call_to_gpu_kernel_lowers_to_gpu_kernel_launch() {
        let mir = lower_to_mir(
            "@gpu(threads=128, shared=64, grid=(8, 1, 1))\nfn kern(input: [f32], output: *mut f32) { let tid = agam.gpu.thread_id_x(); output[tid] = input[tid]; }\nfn host(input: [f32], output: *mut f32): kern(input, output)",
        );
        let host = mir
            .functions
            .iter()
            .find(|function| function.name == "host")
            .expect("expected host function");
        let launch = host
            .blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .find_map(|instruction| match &instruction.op {
                Op::GpuKernelLaunch {
                    kernel_name,
                    shared_memory_bytes,
                    ..
                } => Some((kernel_name.as_str(), *shared_memory_bytes)),
                _ => None,
            });
        assert_eq!(launch, Some(("kern", 64)));
        let has_grid = host.blocks.iter().any(|block| {
            block
                .instructions
                .iter()
                .any(|instruction| matches!(instruction.op, Op::ConstInt(8)))
        });
        let has_block = host.blocks.iter().any(|block| {
            block
                .instructions
                .iter()
                .any(|instruction| matches!(instruction.op, Op::ConstInt(128)))
        });
        assert!(has_grid, "expected launch grid constant in MIR");
        assert!(has_block, "expected launch block constant in MIR");
    }

    #[test]
    #[ignore = "parser missing struct literal support"]
    fn test_mir_struct_literal_lowers() {
        let mir =
            lower_to_mir("struct Point { x: i32, y: i32 }\nfn main(): return Point { x: 1, y: 2 }");
        let f = &mir.functions[0];
        let mut has_struct_init = false;
        for block in &f.blocks {
            for instr in &block.instructions {
                if let crate::ir::Op::Call { callee, .. } = &instr.op {
                    if callee == "__struct_init_Point" {
                        has_struct_init = true;
                    }
                }
            }
        }
        assert!(has_struct_init, "missing struct init call");
    }

    #[test]
    fn test_mir_enum_variant_lowers_to_construct() {
        let mut enum_layouts = HashMap::new();
        enum_layouts.insert(
            "Color".to_string(),
            agam_hir::nodes::HirEnumLayout {
                name: "Color".to_string(),
                variants: vec![
                    agam_hir::nodes::HirEnumVariantLayout {
                        name: "Red".to_string(),
                        tag: 0,
                        has_payload: false,
                    },
                    agam_hir::nodes::HirEnumVariantLayout {
                        name: "Green".to_string(),
                        tag: 1,
                        has_payload: true,
                    },
                ],
            },
        );
        let hir = HirModule {
            functions: vec![HirFunction {
                id: HirId(0),
                name: "main".into(),
                generics: vec![],
                params: vec![],
                return_ty: TypeId(0),
                body: HirBlock {
                    stmts: vec![HirStmt::Return(Some(HirExpr {
                        id: HirId(1),
                        ty: TypeId(0),
                        kind: HirExprKind::EnumVariant {
                            enum_name: "Color".into(),
                            variant: "Green".into(),
                            fields: vec![HirExpr {
                                id: HirId(2),
                                ty: TypeId(0),
                                kind: HirExprKind::IntLit(42),
                            }],
                        },
                    }))],
                    expr: None,
                },
                is_async: false,
                target: Default::default(),
                gpu_config: None,
            }],
            enum_layouts,
            struct_layouts: HashMap::new(),
        };
        let mut mir_lowering = MirLowering::new();
        let mir = mir_lowering.lower_module(&hir);
        let layout = mir.enum_layouts.get("Color").expect("missing enum layout");
        assert_eq!(layout.variants[1].tag, 1);
        let f = &mir.functions[0];
        let mut has_enum_construct = false;
        for block in &f.blocks {
            for instr in &block.instructions {
                if let crate::ir::Op::EnumConstruct { tag, payload } = &instr.op {
                    assert_eq!(*tag, 1);
                    assert_eq!(payload.len(), 1);
                    has_enum_construct = true;
                }
            }
        }
        assert!(has_enum_construct, "missing enum construct op");
    }

    #[test]
    fn test_mir_match_literal_lowers_to_branches_and_result_local() {
        let mir =
            lower_to_mir("fn main():\n    let x = 1\n    match x:\n        1 => 2\n        _ => 3");
        let f = &mir.functions[0];
        let mut branches = 0;
        let mut stores = 0;
        let mut loads_result = false;
        for block in &f.blocks {
            if let crate::ir::Terminator::Branch { .. } = &block.terminator {
                branches += 1;
            }
            for instr in &block.instructions {
                match &instr.op {
                    crate::ir::Op::StoreLocal { name, .. }
                        if name.starts_with("__match_result_") =>
                    {
                        stores += 1;
                    }
                    crate::ir::Op::LoadLocal(name) if name.starts_with("__match_result_") => {
                        loads_result = true;
                    }
                    _ => {}
                }
            }
        }
        assert!(branches >= 1, "missing branches for literal match");
        assert_eq!(stores, 2, "expected both match arms to store a result");
        assert!(loads_result, "missing match result load");
    }

    #[test]
    fn test_mir_match_guard_combines_with_pattern_condition() {
        let mir = lower_to_mir(
            "fn main():\n    let x = 1\n    match x:\n        1 if true => 2\n        _ => 3",
        );
        let f = &mir.functions[0];
        let has_guard_and = f.blocks.iter().any(|block| {
            block.instructions.iter().any(|instr| {
                matches!(
                    instr.op,
                    crate::ir::Op::BinOp {
                        op: crate::ir::MirBinOp::And,
                        ..
                    }
                )
            })
        });
        assert!(has_guard_and, "missing guard/pattern conjunction");
    }

    #[test]
    fn test_mir_match_wildcard_lowers() {
        let mir = lower_to_mir("fn main():\n    let x = 1\n    match x:\n        _ => 2");
        let f = &mir.functions[0];
        let stores_result = f.blocks.iter().any(|block| {
            block.instructions.iter().any(|instr| {
                matches!(
                    &instr.op,
                    crate::ir::Op::StoreLocal { name, .. }
                        if name.starts_with("__match_result_")
                )
            })
        });
        assert!(stores_result, "wildcard match didn't store its result");
    }

    #[test]
    #[ignore = "parser missing variant pattern support"]
    fn test_mir_match_enum_variant_lowers() {
        let mir = lower_to_mir(
            "enum Optional { Some(i32), None }\nfn main(opt: Optional):\n    match opt:\n        Some(val) => val\n        None => 0",
        );
        let f = &mir.functions[0];
        let mut has_tag = false;
        let mut has_payload = false;
        for block in &f.blocks {
            for instr in &block.instructions {
                if matches!(instr.op, crate::ir::Op::EnumTag(_)) {
                    has_tag = true;
                }
                if matches!(instr.op, crate::ir::Op::EnumPayload { .. }) {
                    has_payload = true;
                }
            }
        }
        assert!(has_tag, "missing EnumTag extraction");
        assert!(has_payload, "missing EnumPayload extraction");
    }
}
