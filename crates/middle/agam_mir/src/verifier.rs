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
        Self::verify_stack_provenance(func, &mut errors);

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

    /// Check Stack Frame Escape Safety Invariant
    /// A stack allocation (Op::Alloca) must NEVER escape the stack frame:
    /// - Not through return terminator directly or via alias/copy/phi/projection
    /// - Not stored into an escaping aggregate or external object
    /// - Not passed to an external/unknown call or effect handler
    /// - Not merged in a phi node with externally provided / parameter values
    pub fn verify_stack_provenance(func: &MirFunction, errors: &mut Vec<MirVerificationError>) {
        let mut provenance: HashMap<ValueId, HashSet<ValueId>> = HashMap::new();
        let mut local_provenance: HashMap<String, HashSet<ValueId>> = HashMap::new();

        for block in &func.blocks {
            for instr in &block.instructions {
                if matches!(instr.op, Op::Alloca { .. }) {
                    let mut set = HashSet::new();
                    set.insert(instr.result);
                    provenance.insert(instr.result, set);
                }
            }
        }

        if provenance.is_empty() {
            return;
        }

        // Iterative fixed-point provenance propagation
        let mut changed = true;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 32;

        while changed && iterations < MAX_ITERATIONS {
            changed = false;
            iterations += 1;

            for block in &func.blocks {
                for instr in &block.instructions {
                    match &instr.op {
                        Op::Copy(src) | Op::ArcRetain { value: src } | Op::Cast { value: src, .. } => {
                            if let Some(src_roots) = provenance.get(src).cloned() {
                                let entry = provenance.entry(instr.result).or_default();
                                for root in src_roots {
                                    if entry.insert(root) {
                                        changed = true;
                                    }
                                }
                            }
                        }
                        Op::Phi(entries) => {
                            let mut incoming = HashSet::new();
                            for (_, val) in entries {
                                if let Some(r) = provenance.get(val) {
                                    incoming.extend(r.iter().copied());
                                }
                            }
                            if !incoming.is_empty() {
                                let entry = provenance.entry(instr.result).or_default();
                                for root in incoming {
                                    if entry.insert(root) {
                                        changed = true;
                                    }
                                }
                            }
                        }
                        Op::StoreLocal { name, value } => {
                            if let Some(val_roots) = provenance.get(value).cloned() {
                                let entry = local_provenance.entry(name.clone()).or_default();
                                for root in val_roots {
                                    if entry.insert(root) {
                                        changed = true;
                                    }
                                }
                            }
                        }
                        Op::LoadLocal(name) => {
                            if let Some(loc_roots) = local_provenance.get(name).cloned() {
                                let entry = provenance.entry(instr.result).or_default();
                                for root in loc_roots {
                                    if entry.insert(root) {
                                        changed = true;
                                    }
                                }
                            }
                        }
                        Op::GetField { object, .. } | Op::GetIndex { object, .. } => {
                            if let Some(obj_roots) = provenance.get(object).cloned() {
                                let entry = provenance.entry(instr.result).or_default();
                                for root in obj_roots {
                                    if entry.insert(root) {
                                        changed = true;
                                    }
                                }
                            }
                        }
                        Op::StructConstruct { fields, .. } => {
                            let mut incoming = HashSet::new();
                            for (_, field_val) in fields {
                                if let Some(r) = provenance.get(field_val) {
                                    incoming.extend(r.iter().copied());
                                }
                            }
                            if !incoming.is_empty() {
                                let entry = provenance.entry(instr.result).or_default();
                                for root in incoming {
                                    if entry.insert(root) {
                                        changed = true;
                                    }
                                }
                            }
                        }
                        Op::EnumConstruct { payload, .. } => {
                            let mut incoming = HashSet::new();
                            for p_val in payload {
                                if let Some(r) = provenance.get(p_val) {
                                    incoming.extend(r.iter().copied());
                                }
                            }
                            if !incoming.is_empty() {
                                let entry = provenance.entry(instr.result).or_default();
                                for root in incoming {
                                    if entry.insert(root) {
                                        changed = true;
                                    }
                                }
                            }
                        }
                        Op::EnumPayload { value, .. } => {
                            if let Some(val_roots) = provenance.get(value).cloned() {
                                let entry = provenance.entry(instr.result).or_default();
                                for root in val_roots {
                                    if entry.insert(root) {
                                        changed = true;
                                    }
                                }
                            }
                        }
                        Op::StoreIndex { object, value, .. } => {
                            if let Some(val_roots) = provenance.get(value).cloned() {
                                let entry = provenance.entry(*object).or_default();
                                for root in val_roots {
                                    if entry.insert(root) {
                                        changed = true;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Now verify escape invariants across all blocks and terminators
        for block in &func.blocks {
            if let Terminator::Return(ret_val) = &block.terminator {
                if let Some(roots) = provenance.get(ret_val) {
                    if !roots.is_empty() {
                        errors.push(MirVerificationError::EscapingStackAllocation {
                            value: *ret_val,
                            in_block: block.id,
                            reason: "escapes through function return".into(),
                        });
                    }
                }
            }

            for instr in &block.instructions {
                match &instr.op {
                    Op::Call { callee, args } => {
                        for arg in args {
                            if let Some(roots) = provenance.get(arg) {
                                if !roots.is_empty() {
                                    errors.push(MirVerificationError::EscapingStackAllocation {
                                        value: *arg,
                                        in_block: block.id,
                                        reason: format!("passed to external/unknown call `{}`", callee),
                                    });
                                }
                            }
                        }
                    }
                    Op::EffectPerform { effect, operation, args } => {
                        for arg in args {
                            if let Some(roots) = provenance.get(arg) {
                                if !roots.is_empty() {
                                    errors.push(MirVerificationError::EscapingStackAllocation {
                                        value: *arg,
                                        in_block: block.id,
                                        reason: format!("passed across effect boundary {}.{}", effect, operation),
                                    });
                                }
                            }
                        }
                    }
                    Op::Phi(entries) => {
                        let has_stack_input = entries
                            .iter()
                            .any(|(_, v)| provenance.get(v).is_some_and(|s| !s.is_empty()));
                        let has_param_input = entries
                            .iter()
                            .any(|(_, v)| func.params.iter().any(|p| p.value == *v));
                        if has_stack_input && has_param_input {
                            errors.push(MirVerificationError::EscapingStackAllocation {
                                value: instr.result,
                                in_block: block.id,
                                reason: "phi node merges stack allocation with external function parameter".into(),
                            });
                        }
                    }
                    Op::StoreIndex { object, value, .. } => {
                        let is_param = func.params.iter().any(|p| p.value == *object);
                        if is_param && provenance.get(value).is_some_and(|s| !s.is_empty()) {
                            errors.push(MirVerificationError::EscapingStackAllocation {
                                value: *value,
                                in_block: block.id,
                                reason: "stack allocation stored into external function parameter".into(),
                            });
                        }
                    }
                    _ => {}
                }
            }
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
        Op::Alloca { .. } | Op::ArcAlloc { .. } => Vec::new(),
        Op::ArcRetain { value } | Op::ArcRelease { value } | Op::StackDrop { value } => {
            vec![*value]
        }
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
        Op::Syscall { number, args, .. } => {
            let mut u = vec![*number];
            u.extend(args);
            u
        }
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
        let b0 = BlockId(0);
        let b1 = BlockId(1);
        let v_def_in_b1 = ValueId(0);
        let v_use_in_b0 = ValueId(1);

        let func = MirFunction {
            name: "dominance_bug".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(1),
            entry: b0,
            blocks: vec![
                BasicBlock {
                    id: b0,
                    instructions: vec![Instruction {
                        result: v_use_in_b0,
                        ty: TypeId(1),
                        op: Op::BinOp {
                            op: crate::ir::MirBinOp::Add,
                            left: v_def_in_b1, // VIOLATION: Used before defined
                            right: v_def_in_b1,
                        },
                    }],
                    terminator: Terminator::Jump(b1),
                },
                BasicBlock {
                    id: b1,
                    instructions: vec![Instruction {
                        result: v_def_in_b1,
                        ty: TypeId(1),
                        op: Op::ConstInt(42),
                    }],
                    terminator: Terminator::Return(v_use_in_b0),
                },
            ],
            target: Default::default(),
            gpu_config: None,
        };

        let res = MirVerifier::verify_function(&func);
        assert!(res.is_err());
        if let Err(errs) = res {
            assert!(
                errs.iter()
                    .any(|e| matches!(e, MirVerificationError::UseNotDominatedByDef { .. }))
            );
        }
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
        if let Err(errs) = res {
            assert!(
                errs.iter()
                    .any(|e| matches!(e, MirVerificationError::EscapingStackAllocation { .. }))
            );
        }
    }

    #[test]
    fn test_mir_syscall_construction_and_verification() {
        let b0 = BlockId(0);
        let v_num = ValueId(0);
        let v_arg0 = ValueId(1);
        let v_dst = ValueId(2);

        let func = MirFunction {
            name: "test_getpid".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(1),
            entry: b0,
            blocks: vec![BasicBlock {
                id: b0,
                instructions: vec![
                    Instruction {
                        result: v_num,
                        ty: TypeId(1),
                        op: Op::ConstInt(39), // SYS_getpid
                    },
                    Instruction {
                        result: v_arg0,
                        ty: TypeId(1),
                        op: Op::ConstInt(0),
                    },
                    Instruction {
                        result: v_dst,
                        ty: TypeId(1),
                        op: Op::Syscall {
                            number: v_num,
                            args: vec![v_arg0],
                            dst: v_dst,
                        },
                    },
                ],
                terminator: Terminator::Return(v_dst),
            }],
            target: Default::default(),
            gpu_config: None,
        };

        assert!(MirVerifier::verify_function(&func).is_ok());
    }

    #[test]
    fn test_mir_syscall_dominance_failure() {
        let b0 = BlockId(0);
        let b1 = BlockId(1);
        let v_num_in_b1 = ValueId(0);
        let v_dst_in_b0 = ValueId(1);

        let func = MirFunction {
            name: "test_syscall_dominance".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(1),
            entry: b0,
            blocks: vec![
                BasicBlock {
                    id: b0,
                    instructions: vec![Instruction {
                        result: v_dst_in_b0,
                        ty: TypeId(1),
                        op: Op::Syscall {
                            number: v_num_in_b1, // VIOLATION: Defined in b1 but used in b0
                            args: vec![],
                            dst: v_dst_in_b0,
                        },
                    }],
                    terminator: Terminator::Jump(b1),
                },
                BasicBlock {
                    id: b1,
                    instructions: vec![Instruction {
                        result: v_num_in_b1,
                        ty: TypeId(1),
                        op: Op::ConstInt(39),
                    }],
                    terminator: Terminator::Return(v_dst_in_b0),
                },
            ],
            target: Default::default(),
            gpu_config: None,
        };

        let res = MirVerifier::verify_function(&func);
        assert!(res.is_err());
        if let Err(errs) = res {
            assert!(
                errs.iter()
                    .any(|e| matches!(e, MirVerificationError::UseNotDominatedByDef { .. }))
            );
        }
    }

    #[test]
    fn test_verifier_arc_opcodes_validity() {
        let b0 = BlockId(0);
        let v_alloc = ValueId(0);
        let v_retained = ValueId(1);
        let v_release = ValueId(2);
        let v_drop = ValueId(3);

        let func = MirFunction {
            name: "test_arc_flow".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(1),
            entry: b0,
            blocks: vec![BasicBlock {
                id: b0,
                instructions: vec![
                    Instruction {
                        result: v_alloc,
                        ty: TypeId(1),
                        op: Op::ArcAlloc {
                            name: "buf".into(),
                            ty: TypeId(1),
                        },
                    },
                    Instruction {
                        result: v_retained,
                        ty: TypeId(1),
                        op: Op::ArcRetain { value: v_alloc },
                    },
                    Instruction {
                        result: v_release,
                        ty: TypeId(0),
                        op: Op::ArcRelease { value: v_retained },
                    },
                    Instruction {
                        result: v_drop,
                        ty: TypeId(0),
                        op: Op::StackDrop { value: v_alloc },
                    },
                ],
                terminator: Terminator::Return(v_alloc), // Valid: ArcAlloc is heap-managed and permitted to escape
            }],
            target: Default::default(),
            gpu_config: None,
        };

        assert!(MirVerifier::verify_function(&func).is_ok());
    }

    #[test]
    fn test_verifier_arc_opcodes_dominance_failure() {
        let b0 = BlockId(0);
        let b1 = BlockId(1);
        let v_alloc_in_b1 = ValueId(0);
        let v_retain_in_b0 = ValueId(1);

        let func = MirFunction {
            name: "test_arc_dominance".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(1),
            entry: b0,
            blocks: vec![
                BasicBlock {
                    id: b0,
                    instructions: vec![Instruction {
                        result: v_retain_in_b0,
                        ty: TypeId(1),
                        op: Op::ArcRetain {
                            value: v_alloc_in_b1, // VIOLATION: Used before defined in b1
                        },
                    }],
                    terminator: Terminator::Jump(b1),
                },
                BasicBlock {
                    id: b1,
                    instructions: vec![Instruction {
                        result: v_alloc_in_b1,
                        ty: TypeId(1),
                        op: Op::ArcAlloc {
                            name: "buf".into(),
                            ty: TypeId(1),
                        },
                    }],
                    terminator: Terminator::Return(v_alloc_in_b1),
                },
            ],
            target: Default::default(),
            gpu_config: None,
        };

        let res = MirVerifier::verify_function(&func);
        assert!(res.is_err());
        if let Err(errs) = res {
            assert!(
                errs.iter()
                    .any(|e| matches!(e, MirVerificationError::UseNotDominatedByDef { .. }))
            );
        }
    }

    #[test]
    fn test_verifier_detects_aliased_return_escape() {
        let b0 = BlockId(0);
        let v_alloc = ValueId(0);
        let v_copy = ValueId(1);

        let func = MirFunction {
            name: "test_aliased_return".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(1),
            entry: b0,
            blocks: vec![BasicBlock {
                id: b0,
                instructions: vec![
                    Instruction {
                        result: v_alloc,
                        ty: TypeId(1),
                        op: Op::Alloca {
                            name: "x".into(),
                            ty: TypeId(1),
                        },
                    },
                    Instruction {
                        result: v_copy,
                        ty: TypeId(1),
                        op: Op::Copy(v_alloc),
                    },
                ],
                terminator: Terminator::Return(v_copy),
            }],
            target: Default::default(),
            gpu_config: None,
        };

        let res = MirVerifier::verify_function(&func);
        assert!(res.is_err());
        if let Err(errs) = res {
            assert!(
                errs.iter()
                    .any(|e| matches!(e, MirVerificationError::EscapingStackAllocation { .. }))
            );
        }
    }

    #[test]
    fn test_verifier_detects_struct_field_escape() {
        let b0 = BlockId(0);
        let v_alloc = ValueId(0);
        let v_struct = ValueId(1);

        let func = MirFunction {
            name: "test_struct_field_escape".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(2),
            entry: b0,
            blocks: vec![BasicBlock {
                id: b0,
                instructions: vec![
                    Instruction {
                        result: v_alloc,
                        ty: TypeId(1),
                        op: Op::Alloca {
                            name: "inner".into(),
                            ty: TypeId(1),
                        },
                    },
                    Instruction {
                        result: v_struct,
                        ty: TypeId(2),
                        op: Op::StructConstruct {
                            name: "Container".into(),
                            fields: vec![("inner".into(), v_alloc)],
                        },
                    },
                ],
                terminator: Terminator::Return(v_struct),
            }],
            target: Default::default(),
            gpu_config: None,
        };

        let res = MirVerifier::verify_function(&func);
        assert!(res.is_err());
        if let Err(errs) = res {
            assert!(
                errs.iter()
                    .any(|e| matches!(e, MirVerificationError::EscapingStackAllocation { .. }))
            );
        }
    }

    #[test]
    fn test_verifier_detects_call_escape() {
        let b0 = BlockId(0);
        let v_alloc = ValueId(0);
        let v_call = ValueId(1);

        let func = MirFunction {
            name: "test_call_escape".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(0),
            entry: b0,
            blocks: vec![BasicBlock {
                id: b0,
                instructions: vec![
                    Instruction {
                        result: v_alloc,
                        ty: TypeId(1),
                        op: Op::Alloca {
                            name: "buf".into(),
                            ty: TypeId(1),
                        },
                    },
                    Instruction {
                        result: v_call,
                        ty: TypeId(0),
                        op: Op::Call {
                            callee: "external_sink".into(),
                            args: vec![v_alloc],
                        },
                    },
                ],
                terminator: Terminator::ReturnVoid,
            }],
            target: Default::default(),
            gpu_config: None,
        };

        let res = MirVerifier::verify_function(&func);
        assert!(res.is_err());
        if let Err(errs) = res {
            assert!(
                errs.iter()
                    .any(|e| matches!(e, MirVerificationError::EscapingStackAllocation { .. }))
            );
        }
    }

    #[test]
    fn test_verifier_detects_effect_escape() {
        let b0 = BlockId(0);
        let v_alloc = ValueId(0);
        let v_eff = ValueId(1);

        let func = MirFunction {
            name: "test_effect_escape".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(0),
            entry: b0,
            blocks: vec![BasicBlock {
                id: b0,
                instructions: vec![
                    Instruction {
                        result: v_alloc,
                        ty: TypeId(1),
                        op: Op::Alloca {
                            name: "buf".into(),
                            ty: TypeId(1),
                        },
                    },
                    Instruction {
                        result: v_eff,
                        ty: TypeId(0),
                        op: Op::EffectPerform {
                            effect: "IO".into(),
                            operation: "write".into(),
                            args: vec![v_alloc],
                        },
                    },
                ],
                terminator: Terminator::ReturnVoid,
            }],
            target: Default::default(),
            gpu_config: None,
        };

        let res = MirVerifier::verify_function(&func);
        assert!(res.is_err());
        if let Err(errs) = res {
            assert!(
                errs.iter()
                    .any(|e| matches!(e, MirVerificationError::EscapingStackAllocation { .. }))
            );
        }
    }

    #[test]
    fn test_verifier_detects_phi_param_merge_escape() {
        let b0 = BlockId(0);
        let b1 = BlockId(1);
        let b2 = BlockId(2);
        let v_param = ValueId(0);
        let v_alloc = ValueId(1);
        let v_phi = ValueId(2);

        let func = MirFunction {
            name: "test_phi_merge".into(),
            generics: vec![],
            params: vec![crate::ir::MirParam {
                name: "p".into(),
                value: v_param,
                ty: TypeId(1),
                gpu_abi: Default::default(),
                memory_type: None,
            }],
            return_ty: TypeId(1),
            entry: b0,
            blocks: vec![
                BasicBlock {
                    id: b0,
                    instructions: vec![Instruction {
                        result: v_alloc,
                        ty: TypeId(1),
                        op: Op::Alloca {
                            name: "local_x".into(),
                            ty: TypeId(1),
                        },
                    }],
                    terminator: Terminator::Jump(b1),
                },
                BasicBlock {
                    id: b1,
                    instructions: vec![Instruction {
                        result: v_phi,
                        ty: TypeId(1),
                        op: Op::Phi(vec![(b0, v_alloc), (b2, v_param)]),
                    }],
                    terminator: Terminator::Return(v_phi),
                },
                BasicBlock {
                    id: b2,
                    instructions: vec![],
                    terminator: Terminator::Jump(b1),
                },
            ],
            target: Default::default(),
            gpu_config: None,
        };

        let res = MirVerifier::verify_function(&func);
        assert!(res.is_err());
        if let Err(errs) = res {
            assert!(
                errs.iter()
                    .any(|e| matches!(e, MirVerificationError::EscapingStackAllocation { .. }))
            );
        }
    }
}
