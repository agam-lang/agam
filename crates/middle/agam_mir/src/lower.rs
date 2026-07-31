//! HIR → MIR lowering pass.
//!
//! Transforms the high-level HIR into basic-block-based MIR with SSA values.

use agam_hir::nodes::*;
use agam_sema::gpu::{GpuBuiltin, resolve_gpu_builtin};
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
    types: TypeStore,
    /// Variant names are globally resolved by the current front end.  Keep the
    /// corresponding discriminants available while lowering function bodies.
    variant_tags: std::collections::HashMap<String, u32>,
}

impl MirLowering {
    pub fn new() -> Self {
        Self {
            next_value: 0,
            next_block: 0,
            blocks: Vec::new(),
            current_instrs: Vec::new(),
            current_block: BlockId(0),
            types: TypeStore::new(),
            variant_tags: std::collections::HashMap::new(),
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
        self.variant_tags = hir
            .enum_layouts
            .values()
            .flat_map(|layout| layout.variants.iter())
            .map(|variant| (variant.name.clone(), variant.tag))
            .collect();

        let functions = hir
            .functions
            .iter()
            .map(|f| self.lower_function(f))
            .collect();

        // Propagate enum and struct layouts from HIR to MIR
        let enum_layouts = hir
            .enum_layouts
            .iter()
            .map(|(name, layout)| {
                (
                    name.clone(),
                    crate::ir::EnumLayout {
                        name: layout.name.clone(),
                        variants: layout
                            .variants
                            .iter()
                            .map(|v| crate::ir::EnumVariantLayout {
                                name: v.name.clone(),
                                tag: v.tag,
                                has_payload: v.has_payload,
                            })
                            .collect(),
                    },
                )
            })
            .collect();

        MirModule {
            functions,
            enum_layouts,
            struct_layouts: hir
                .struct_layouts
                .iter()
                .map(|(name, layout)| {
                    (
                        name.clone(),
                        crate::ir::StructLayout {
                            name: layout.name.clone(),
                            fields: layout.fields.clone(),
                        },
                    )
                })
                .collect(),
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
                    memory_type: p.memory_type,
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
            generics: vec![],
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
            HirStmt::Match { scrutinee, arms } => {
                let scrutinee_val = self.lower_expr(scrutinee);
                self.lower_match(self.types.unit(), scrutinee_val, arms);
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

            HirExprKind::StructLiteral { name, fields } => {
                let fields = fields
                    .iter()
                    .map(|(field_name, value)| (field_name.clone(), self.lower_expr(value)))
                    .collect();
                self.emit(
                    ty,
                    Op::StructConstruct {
                        name: name.clone(),
                        fields,
                    },
                )
            }

            HirExprKind::EnumVariant {
                variant, fields, ..
            } => {
                let tag = self.variant_tags.get(variant).copied().unwrap_or(0);
                let payload: Vec<ValueId> = fields.iter().map(|f| self.lower_expr(f)).collect();
                self.emit(ty, Op::EnumConstruct { tag, payload })
            }

            HirExprKind::Match { scrutinee, arms } => {
                let scrutinee_val = self.lower_expr(scrutinee);
                self.lower_match(ty, scrutinee_val, arms)
            }
        }
    }

    /// Lower a match expression into MIR control flow.
    ///
    /// For variant patterns: emit Switch on EnumTag with per-arm blocks.
    /// For literal patterns: emit chained equality comparisons.
    /// For wildcard/bind patterns: direct jump to arm body.
    fn lower_match(
        &mut self,
        result_ty: TypeId,
        scrutinee: ValueId,
        arms: &[HirMatchArm],
    ) -> ValueId {
        if arms.is_empty() {
            return self.emit(result_ty, Op::Unit);
        }

        let dispatch_block = self.current_block;
        let merge_block = self.fresh_block();

        // Allocate a local to hold the match result (phi-like)
        let result_name = format!("__match_result_{}", self.next_value);
        self.emit(
            result_ty,
            Op::Alloca {
                name: result_name.clone(),
                ty: result_ty,
            },
        );

        // Check if this is an enum-tag-based match (any arm has a Variant pattern)
        let has_variant_pattern = arms
            .iter()
            .any(|arm| matches!(arm.pattern, HirPattern::Variant { .. }));

        if has_variant_pattern {
            // Enum match: extract the discriminant once, then dispatch directly
            // to fully formed arm blocks.  Do not patch an already-finished
            // block: that loses instructions accumulated for the first arm.
            let tag_val = self.emit(self.types.i32(), Op::EnumTag(scrutinee));

            let mut cases: Vec<(i64, BlockId)> = Vec::new();
            let mut default_block = None;
            let mut arm_blocks = Vec::with_capacity(arms.len());
            for _ in arms {
                arm_blocks.push(self.fresh_block());
            }

            for (arm, arm_block) in arms.iter().zip(arm_blocks) {
                self.current_block = arm_block;

                match &arm.pattern {
                    HirPattern::Variant { name, fields } => {
                        if let Some(tag) = self.variant_tags.get(name) {
                            cases.push((*tag as i64, arm_block));
                        }

                        // Extract payload fields as local variables
                        for (field_idx, field_pat) in fields.iter().enumerate() {
                            if let HirPattern::Bind(name) = field_pat {
                                let payload_ty = self.types.fresh_var();
                                let payload = self.emit(
                                    payload_ty,
                                    Op::EnumPayload {
                                        value: scrutinee,
                                        field_index: field_idx as u32,
                                    },
                                );
                                self.emit(
                                    payload_ty,
                                    Op::Alloca {
                                        name: name.clone(),
                                        ty: payload_ty,
                                    },
                                );
                                self.emit(
                                    result_ty,
                                    Op::StoreLocal {
                                        name: name.clone(),
                                        value: payload,
                                    },
                                );
                            }
                        }

                        let arm_result = self.lower_expr(&arm.body);
                        self.emit(
                            result_ty,
                            Op::StoreLocal {
                                name: result_name.clone(),
                                value: arm_result,
                            },
                        );
                        self.finish_block(Terminator::Jump(merge_block));
                    }
                    HirPattern::Wildcard | HirPattern::Bind(_) => {
                        default_block = Some(arm_block);
                        let arm_result = self.lower_expr(&arm.body);
                        self.emit(
                            result_ty,
                            Op::StoreLocal {
                                name: result_name.clone(),
                                value: arm_result,
                            },
                        );
                        self.finish_block(Terminator::Jump(merge_block));
                    }
                    _ => {
                        // A mixed enum match is validated earlier.  Preserve a
                        // deterministic fallback here for recovery lowering.
                        default_block = Some(arm_block);
                        let arm_result = self.lower_expr(&arm.body);
                        self.emit(
                            result_ty,
                            Op::StoreLocal {
                                name: result_name.clone(),
                                value: arm_result,
                            },
                        );
                        self.finish_block(Terminator::Jump(merge_block));
                    }
                }
            }
            self.current_block = dispatch_block;
            self.finish_block(Terminator::Switch {
                discriminant: tag_val,
                cases,
                default: default_block.unwrap_or(merge_block),
            });
        } else {
            // Non-enum match: chained if-else on literal equality or wildcard
            for arm in arms {
                match &arm.pattern {
                    HirPattern::Wildcard | HirPattern::Bind(_) => {
                        // Default arm: just lower body
                        let arm_result = self.lower_expr(&arm.body);
                        self.emit(
                            result_ty,
                            Op::StoreLocal {
                                name: result_name.clone(),
                                value: arm_result,
                            },
                        );
                        self.finish_block(Terminator::Jump(merge_block));
                        self.current_block = merge_block;
                        break;
                    }
                    HirPattern::Literal(lit_expr) => {
                        let lit_val = self.lower_expr(lit_expr);
                        let cmp = self.emit(
                            self.types.bool(),
                            Op::BinOp {
                                op: MirBinOp::Eq,
                                left: scrutinee,
                                right: lit_val,
                            },
                        );

                        let then_block = self.fresh_block();
                        let else_block = self.fresh_block();
                        self.finish_block(Terminator::Branch {
                            condition: cmp,
                            then_block,
                            else_block,
                        });

                        self.current_block = then_block;
                        let arm_result = self.lower_expr(&arm.body);
                        self.emit(
                            result_ty,
                            Op::StoreLocal {
                                name: result_name.clone(),
                                value: arm_result,
                            },
                        );
                        self.finish_block(Terminator::Jump(merge_block));

                        self.current_block = else_block;
                    }
                    _ => {
                        // Unsupported pattern type — skip
                        let arm_result = self.lower_expr(&arm.body);
                        self.emit(
                            result_ty,
                            Op::StoreLocal {
                                name: result_name.clone(),
                                value: arm_result,
                            },
                        );
                        self.finish_block(Terminator::Jump(merge_block));
                        self.current_block = merge_block;
                        break;
                    }
                }
            }

            // If we didn't hit a default arm, jump to merge anyway
            if self.current_block != merge_block {
                self.finish_block(Terminator::Jump(merge_block));
            }
        }

        self.current_block = merge_block;
        self.emit(result_ty, Op::LoadLocal(result_name))
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
    fn test_mir_gpu_shared_alloc_lowers_to_explicit_op() {
        let mir =
            lower_to_mir("@gpu\nfn kern(): let scratch: *mut f32 = agam.gpu.shared_alloc(128)");
        let f = &mir.functions[0];
        let has_shared_alloc = f.blocks.iter().any(|b| {
            b.instructions.iter().any(|i| {
                matches!(
                    &i.op,
                    Op::GpuSharedAlloc {
                        element_abi: agam_sema::gpu::GpuKernelParamAbi::F32,
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
                        element_abi: agam_sema::gpu::GpuKernelParamAbi::I32,
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
    fn test_mir_enum_layout_propagated_from_hir() {
        let mir = lower_to_mir("enum Color { Red, Green, Blue }\nfn main(): return 0");
        assert!(
            mir.enum_layouts.contains_key("Color"),
            "expected enum layout for Color in MIR module"
        );
        let layout = &mir.enum_layouts["Color"];
        assert_eq!(layout.variants.len(), 3);
        assert_eq!(layout.variants[0].name, "Red");
        assert_eq!(layout.variants[0].tag, 0);
        assert!(!layout.variants[0].has_payload);
        assert_eq!(layout.variants[1].name, "Green");
        assert_eq!(layout.variants[2].name, "Blue");
    }

    #[test]
    fn test_mir_enum_variant_with_payload_layout() {
        let mir = lower_to_mir("enum Option { Some(i32), None }\nfn main(): return 0");
        assert!(mir.enum_layouts.contains_key("Option"));
        let layout = &mir.enum_layouts["Option"];
        assert_eq!(layout.variants.len(), 2);
        assert!(layout.variants[0].has_payload, "Some should have payload");
        assert!(
            !layout.variants[1].has_payload,
            "None should not have payload"
        );
    }

    #[test]
    fn test_mir_enum_constructor_uses_declared_tag() {
        let mir = lower_to_mir("enum Option { Some(i32), None }\nfn main(): let value = Some(42)");
        let has_some = mir.functions[0].blocks.iter().any(|block| {
            block.instructions.iter().any(|instruction| {
                matches!(
                    &instruction.op,
                    Op::EnumConstruct {
                        tag: 0,
                        payload
                    } if payload.len() == 1
                )
            })
        });
        assert!(has_some, "expected Some constructor with declared tag 0");
    }

    #[test]
    fn test_mir_struct_literal_preserves_named_field_values() {
        let mir = lower_to_mir(
            "struct Point { x: i32, y: i32 }\nfn main() { let point = Point { x: 3, y: 4 }; }",
        );
        let construct = mir.functions[0]
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .find_map(|instruction| match &instruction.op {
                Op::StructConstruct { name, fields } => Some((name, fields)),
                _ => None,
            })
            .expect("expected a struct construction operation");

        assert_eq!(construct.0, "Point");
        assert_eq!(
            construct
                .1
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>(),
            ["x", "y"]
        );
    }

    #[test]
    fn test_mir_match_on_integer_lowers_to_branch() {
        let mir = lower_to_mir("fn main(): let x = 42; match x { 1 => return 10, _ => return 20 }");
        let f = &mir.functions[0];
        let has_eq = f.blocks.iter().any(|b| {
            b.instructions.iter().any(|i| {
                matches!(
                    &i.op,
                    Op::BinOp {
                        op: MirBinOp::Eq,
                        ..
                    }
                )
            })
        });
        assert!(
            has_eq,
            "expected equality comparison for literal pattern match in MIR"
        );
    }
}
