//! Strict MIR SSA & CFG Invariant Verifier.
//!
//! Validates:
//! 1. CFG Validity: entry block exists, every block terminates with a valid terminator, all target blocks exist.
//! 2. SSA Invariant: every ValueId is defined exactly once (parameter or instruction result).
//! 3. Dominance of Uses: every operand use is dominated by its definition.
//! 4. Phi Node Validity: Phi nodes appear only at the beginning of a block and have operands corresponding to CFG predecessors.

use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::analysis::{ControlFlowGraph, DominatorTree, ReversePostOrder};
use crate::ir::{BlockId, Instruction, MirFunction, MirModule, Op, Terminator, ValueId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MirVerificationError {
    MissingEntryBlock(BlockId),
    DuplicateBlockId(BlockId),
    InvalidBranchTarget {
        from: BlockId,
        target: BlockId,
    },
    MultipleDefinitions {
        value: ValueId,
    },
    UndefinedValue {
        value: ValueId,
        in_block: BlockId,
    },
    UseNotDominatedByDef {
        value: ValueId,
        def_block: BlockId,
        use_block: BlockId,
    },
    PhiNotAtBlockStart {
        block: BlockId,
        instr_index: usize,
    },
    PhiPredecessorMismatch {
        block: BlockId,
        phi_block: BlockId,
    },
    EscapingStackAllocation {
        value: ValueId,
        in_block: BlockId,
        reason: String,
    },
}

impl fmt::Display for MirVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MirVerificationError::MissingEntryBlock(id) => {
                write!(f, "entry block B{} does not exist", id.0)
            }
            MirVerificationError::DuplicateBlockId(id) => {
                write!(f, "duplicate block ID B{}", id.0)
            }
            MirVerificationError::InvalidBranchTarget { from, target } => {
                write!(
                    f,
                    "block B{} branches to non-existent block B{}",
                    from.0, target.0
                )
            }
            MirVerificationError::MultipleDefinitions { value } => {
                write!(
                    f,
                    "SSA violation: value %{} is defined multiple times",
                    value.0
                )
            }
            MirVerificationError::UndefinedValue { value, in_block } => {
                write!(
                    f,
                    "use of undefined value %{} in block B{}",
                    value.0, in_block.0
                )
            }
            MirVerificationError::UseNotDominatedByDef {
                value,
                def_block,
                use_block,
            } => {
                write!(
                    f,
                    "dominance violation: use of value %{} in block B{} is not dominated by definition in B{}",
                    value.0, use_block.0, def_block.0
                )
            }
            MirVerificationError::PhiNotAtBlockStart { block, instr_index } => {
                write!(
                    f,
                    "Phi node in block B{} appears at instruction index {} (must be at start)",
                    block.0, instr_index
                )
            }
            MirVerificationError::PhiPredecessorMismatch { block, phi_block } => {
                write!(
                    f,
                    "Phi node in block B{} references predecessor B{} which is not in CFG",
                    block.0, phi_block.0
                )
            }
            MirVerificationError::EscapingStackAllocation {
                value,
                in_block,
                reason,
            } => {
                write!(
                    f,
                    "escape safety violation: stack allocation %{} in block B{} escapes: {}",
                    value.0, in_block.0, reason
                )
            }
        }
    }
}

pub struct MirVerifier;

impl MirVerifier {
    pub fn verify_function(func: &MirFunction) -> Result<(), Vec<MirVerificationError>> {
        let mut errors = Vec::new();

        // 1. Check blocks and entry
        let mut block_ids = HashSet::new();
        let mut has_entry = false;
        for block in &func.blocks {
            if !block_ids.insert(block.id) {
                errors.push(MirVerificationError::DuplicateBlockId(block.id));
            }
            if block.id == func.entry {
                has_entry = true;
            }
        }
        if !has_entry && !func.blocks.is_empty() {
            errors.push(MirVerificationError::MissingEntryBlock(func.entry));
        }

        // 2. Check branch targets
        for block in &func.blocks {
            let succs = match &block.terminator {
                Terminator::Jump(target) => vec![*target],
                Terminator::Branch {
                    then_block,
                    else_block,
                    ..
                } => vec![*then_block, *else_block],
                Terminator::Switch { cases, default, .. } => {
                    let mut targets: Vec<BlockId> =
                        cases.iter().map(|(_, target)| *target).collect();
                    targets.push(*default);
                    targets
                }
                Terminator::Return(_) | Terminator::ReturnVoid | Terminator::Unreachable => {
                    Vec::new()
                }
            };
            for target in succs {
                if !block_ids.contains(&target) {
                    errors.push(MirVerificationError::InvalidBranchTarget {
                        from: block.id,
                        target,
                    });
                }
            }
        }

        // 3. Track definitions (parameters + instruction results)
        let mut def_locations: HashMap<ValueId, BlockId> = HashMap::new();
        for param in &func.params {
            if def_locations.insert(param.value, func.entry).is_some() {
                errors.push(MirVerificationError::MultipleDefinitions { value: param.value });
            }
        }

        for block in &func.blocks {
            for instr in &block.instructions {
                if def_locations.insert(instr.result, block.id).is_some() {
                    errors.push(MirVerificationError::MultipleDefinitions {
                        value: instr.result,
                    });
                }
            }
        }

        if errors
            .iter()
            .any(|e| matches!(e, MirVerificationError::MissingEntryBlock(_)))
        {
            return Err(errors);
        }

        let cfg = ControlFlowGraph::build(func);
        let rpo = ReversePostOrder::build(func, &cfg);
        let dom_tree = DominatorTree::build(func, &cfg, &rpo);

        // 4. Check Phi placement & operands
        for block in &func.blocks {
            let mut phi_section = true;
            for (idx, instr) in block.instructions.iter().enumerate() {
                if let Op::Phi(entries) = &instr.op {
                    if !phi_section {
                        errors.push(MirVerificationError::PhiNotAtBlockStart {
                            block: block.id,
                            instr_index: idx,
                        });
                    }
                    let cfg_preds: HashSet<BlockId> =
                        cfg.predecessors(block.id).iter().copied().collect();
                    for (pred_block, val) in entries {
                        if !cfg_preds.contains(pred_block) {
                            errors.push(MirVerificationError::PhiPredecessorMismatch {
                                block: block.id,
                                phi_block: *pred_block,
                            });
                        }
                        if !def_locations.contains_key(val) {
                            errors.push(MirVerificationError::UndefinedValue {
                                value: *val,
                                in_block: block.id,
                            });
                        }
                    }
                } else {
                    phi_section = false;
                }
            }
        }

        // 5. Check Dominance of uses
        for block in &func.blocks {
            if !rpo.is_reachable(block.id) {
                continue;
            }

            for instr in &block.instructions {
                if matches!(instr.op, Op::Phi(_)) {
                    continue; // Phis verified separately
                }
                let uses = instruction_uses(instr);
                for val in uses {
                    match def_locations.get(&val) {
                        Some(&def_block) => {
                            if !dom_tree.dominates(def_block, block.id) {
                                errors.push(MirVerificationError::UseNotDominatedByDef {
                                    value: val,
                                    def_block,
                                    use_block: block.id,
                                });
                            }
                        }
                        None => {
                            errors.push(MirVerificationError::UndefinedValue {
                                value: val,
                                in_block: block.id,
                            });
                        }
                    }
                }
            }

            // Terminator uses
            let term_uses = terminator_uses(&block.terminator);
            for val in term_uses {
                match def_locations.get(&val) {
                    Some(&def_block) => {
                        if !dom_tree.dominates(def_block, block.id) {
                            errors.push(MirVerificationError::UseNotDominatedByDef {
                                value: val,
                                def_block,
                                use_block: block.id,
                            });
                        }
                    }
                    None => {
                        errors.push(MirVerificationError::UndefinedValue {
                            value: val,
                            in_block: block.id,
                        });
                    }
                }
            }
        }

        // 6. Check Stack Frame Escape Safety Invariant
        // Value defined by Op::Alloca must NEVER escape through return terminator
        let mut stack_allocations: HashSet<ValueId> = HashSet::new();
        for block in &func.blocks {
            for instr in &block.instructions {
                if matches!(instr.op, Op::Alloca { .. }) {
                    stack_allocations.insert(instr.result);
                }
            }
        }

        for block in &func.blocks {
            if let Terminator::Return(ret_val) = &block.terminator {
                if stack_allocations.contains(ret_val) {
                    errors.push(MirVerificationError::EscapingStackAllocation {
                        value: *ret_val,
                        in_block: block.id,
                        reason: "returned directly from function".into(),
                    });
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn verify_module(module: &MirModule) -> Result<(), Vec<MirVerificationError>> {
        let mut all_errors = Vec::new();
        for func in &module.functions {
            if let Err(errors) = Self::verify_function(func) {
                all_errors.extend(errors);
            }
        }
        if all_errors.is_empty() {
            Ok(())
        } else {
            Err(all_errors)
        }
    }
}

fn instruction_uses(instr: &Instruction) -> Vec<ValueId> {
    match &instr.op {
        Op::ConstInt(_) | Op::ConstFloat(_) | Op::ConstBool(_) | Op::ConstString(_) | Op::Unit => {
            Vec::new()
        }
        Op::BinOp { left, right, .. } => vec![*left, *right],
        Op::UnOp { operand, .. } => vec![*operand],
        Op::Call { args, .. } => args.clone(),
        Op::Copy(v) => vec![*v],
        Op::LoadLocal(_) => Vec::new(),
        Op::StoreLocal { value, .. } => vec![*value],
        Op::StoreIndex {
            object,
            index,
            value,
        } => vec![*object, *index, *value],
        Op::Alloca { .. } => Vec::new(),
        Op::GetField { object, .. } => vec![*object],
        Op::GetIndex { object, index } => vec![*object, *index],
        Op::Phi(entries) => entries.iter().map(|(_, v)| *v).collect(),
        Op::Cast { value, .. } => vec![*value],
        Op::EffectPerform { args, .. } => args.clone(),
        Op::HandleWith { .. } => Vec::new(),
        Op::GpuKernelLaunch {
            grid, block, args, ..
        } => {
            let mut u = vec![*grid, *block];
            u.extend(args);
            u
        }
        Op::GpuIntrinsic { args, .. } => args.clone(),
        Op::GpuSharedAlloc { count, .. } => vec![*count],
        Op::InlineAsm { args, .. } => args.clone(),
        Op::EnumConstruct { payload, .. } => payload.clone(),
        Op::EnumTag(v) => vec![*v],
        Op::EnumPayload { value, .. } => vec![*value],
        Op::StructConstruct { fields, .. } => fields.iter().map(|(_, v)| *v).collect(),
    }
}

fn terminator_uses(term: &Terminator) -> Vec<ValueId> {
    match term {
        Terminator::Return(v) => vec![*v],
        Terminator::ReturnVoid | Terminator::Unreachable | Terminator::Jump(_) => Vec::new(),
        Terminator::Branch { condition, .. } => vec![*condition],
        Terminator::Switch { discriminant, .. } => vec![*discriminant],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BasicBlock, Instruction, MirFunction, MirParam, Op, Terminator, ValueId};
    use agam_sema::symbol::TypeId;

    #[test]
    fn test_verifier_valid_function() {
        let b0 = BlockId(0);
        let v0 = ValueId(0);
        let v1 = ValueId(1);
        let v2 = ValueId(2);

        let func = MirFunction {
            name: "valid_add".into(),
            generics: vec![],
            params: vec![
                MirParam {
                    name: "a".into(),
                    value: v0,
                    ty: TypeId(1),
                    gpu_abi: Default::default(),
                    memory_type: None,
                },
                MirParam {
                    name: "b".into(),
                    value: v1,
                    ty: TypeId(1),
                    gpu_abi: Default::default(),
                    memory_type: None,
                },
            ],
            return_ty: TypeId(1),
            entry: b0,
            blocks: vec![BasicBlock {
                id: b0,
                instructions: vec![Instruction {
                    result: v2,
                    ty: TypeId(1),
                    op: Op::BinOp {
                        op: crate::ir::MirBinOp::Add,
                        left: v0,
                        right: v1,
                    },
                }],
                terminator: Terminator::Return(v2),
            }],
            target: Default::default(),
            gpu_config: None,
        };

        assert!(MirVerifier::verify_function(&func).is_ok());
    }

    #[test]
    fn test_verifier_detects_dominance_violation() {
        // B0 branches to B1 and B2; B2 tries to use a value defined in B1 without dominance
        let b0 = BlockId(0);
        let b1 = BlockId(1);
        let b2 = BlockId(2);
        let v_cond = ValueId(0);
        let v_def_in_b1 = ValueId(1);

        let func = MirFunction {
            name: "broken_dom".into(),
            generics: vec![],
            params: vec![MirParam {
                name: "c".into(),
                value: v_cond,
                ty: TypeId(0),
                gpu_abi: Default::default(),
                memory_type: None,
            }],
            return_ty: TypeId(1),
            entry: b0,
            blocks: vec![
                BasicBlock {
                    id: b0,
                    instructions: vec![],
                    terminator: Terminator::Branch {
                        condition: v_cond,
                        then_block: b1,
                        else_block: b2,
                    },
                },
                BasicBlock {
                    id: b1,
                    instructions: vec![Instruction {
                        result: v_def_in_b1,
                        ty: TypeId(1),
                        op: Op::ConstInt(42),
                    }],
                    terminator: Terminator::ReturnVoid,
                },
                BasicBlock {
                    id: b2,
                    instructions: vec![],
                    terminator: Terminator::Return(v_def_in_b1), // VIOLATION: B1 does not dominate B2
                },
            ],
            target: Default::default(),
            gpu_config: None,
        };

        let res = MirVerifier::verify_function(&func);
        assert!(res.is_err());
        let errs = res.unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, MirVerificationError::UseNotDominatedByDef { .. }))
        );
    }

    #[test]
    fn test_verifier_detects_escaping_stack_allocation() {
        let b0 = BlockId(0);
        let v_stack = ValueId(0);

        let func = MirFunction {
            name: "escaping_stack".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(1),
            entry: b0,
            blocks: vec![BasicBlock {
                id: b0,
                instructions: vec![Instruction {
                    result: v_stack,
                    ty: TypeId(1),
                    op: Op::Alloca {
                        name: "x".into(),
                        ty: TypeId(1),
                    },
                }],
                terminator: Terminator::Return(v_stack), // VIOLATION: Stack allocation returned
            }],
            target: Default::default(),
            gpu_config: None,
        };

        let res = MirVerifier::verify_function(&func);
        assert!(res.is_err());
        let errs = res.unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, MirVerificationError::EscapingStackAllocation { .. }))
        );
    }
}
