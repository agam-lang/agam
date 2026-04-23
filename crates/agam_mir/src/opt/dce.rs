//! Dead code elimination for MIR.

use std::collections::{HashMap, HashSet};

use crate::ir::{BlockId, Instruction, MirFunction, MirModule, Op, Terminator, ValueId};

pub fn run(module: &mut MirModule) -> bool {
    let mut changed = false;

    for function in &mut module.functions {
        changed |= dce_function(function);
    }

    changed
}

fn dce_function(function: &mut MirFunction) -> bool {
    let reachable = reachable_blocks(function);
    let original_block_count = function.blocks.len();
    function
        .blocks
        .retain(|block| reachable.contains(&block.id));

    let mut changed = function.blocks.len() != original_block_count;
    let live_locals = collect_live_locals(function);

    for block in &mut function.blocks {
        let original_len = block.instructions.len();
        block.instructions =
            sweep_instructions(&block.instructions, &block.terminator, &live_locals);
        changed |= block.instructions.len() != original_len;
    }

    changed
}

fn reachable_blocks(function: &MirFunction) -> HashSet<BlockId> {
    let by_id: HashMap<BlockId, &crate::ir::BasicBlock> = function
        .blocks
        .iter()
        .map(|block| (block.id, block))
        .collect();

    let mut reachable = HashSet::new();
    let mut worklist = vec![function.entry];

    while let Some(block_id) = worklist.pop() {
        if !reachable.insert(block_id) {
            continue;
        }

        let Some(block) = by_id.get(&block_id) else {
            continue;
        };

        match &block.terminator {
            Terminator::Jump(target) => worklist.push(*target),
            Terminator::Branch {
                then_block,
                else_block,
                ..
            } => {
                worklist.push(*then_block);
                worklist.push(*else_block);
            }
            Terminator::Return(_) | Terminator::ReturnVoid | Terminator::Unreachable => {}
        }
    }

    reachable
}

fn collect_live_locals(function: &MirFunction) -> HashSet<String> {
    let mut live = HashSet::new();

    for block in &function.blocks {
        for instr in &block.instructions {
            if let Op::LoadLocal(name) = &instr.op {
                live.insert(name.clone());
            }
        }
    }

    live
}

fn sweep_instructions(
    instructions: &[Instruction],
    terminator: &Terminator,
    live_locals: &HashSet<String>,
) -> Vec<Instruction> {
    let mut used_values = seed_from_terminator(terminator);

    let mut kept = Vec::new();
    for instr in instructions.iter().rev() {
        let keep = is_side_effectful(instr, live_locals) || used_values.contains(&instr.result);

        if keep {
            mark_used_values(instr, &mut used_values);
            kept.push(instr.clone());
        }
    }

    kept.reverse();
    kept
}

fn seed_from_terminator(terminator: &Terminator) -> HashSet<ValueId> {
    let mut used = HashSet::new();

    match terminator {
        Terminator::Return(value) => {
            used.insert(*value);
        }
        Terminator::Branch { condition, .. } => {
            used.insert(*condition);
        }
        Terminator::ReturnVoid | Terminator::Jump(_) | Terminator::Unreachable => {}
    }

    used
}

fn is_side_effectful(instr: &Instruction, live_locals: &HashSet<String>) -> bool {
    match &instr.op {
        Op::Call { .. } => true,
        Op::EffectPerform { .. } => true,
        Op::HandleWith { .. } => true,
        Op::GpuKernelLaunch { .. } => true,
        Op::StoreLocal { name, .. } | Op::Alloca { name, .. } => live_locals.contains(name),
        Op::GpuIntrinsic { .. } => true, // Conservatively mark as having side effects (like barriers)
        Op::InlineAsm { .. } => true, // Inline ASM might have side effects
        _ => false,
    }
}

fn mark_used_values(instr: &Instruction, used_values: &mut HashSet<ValueId>) {
    match &instr.op {
        Op::BinOp { left, right, .. } => {
            used_values.insert(*left);
            used_values.insert(*right);
        }
        Op::UnOp { operand, .. } => {
            used_values.insert(*operand);
        }
        Op::Call { args, .. } => {
            used_values.extend(args.iter().copied());
        }
        Op::Copy(value) => {
            used_values.insert(*value);
        }
        Op::StoreLocal { value, .. } => {
            used_values.insert(*value);
        }
        Op::GetField { object, .. } => {
            used_values.insert(*object);
        }
        Op::GetIndex { object, index } => {
            used_values.insert(*object);
            used_values.insert(*index);
        }
        Op::Phi(entries) => {
            used_values.extend(entries.iter().map(|(_, value)| *value));
        }
        Op::Cast { value, .. } => {
            used_values.insert(*value);
        }
        Op::EffectPerform { args, .. } => {
            used_values.extend(args.iter().copied());
        }
        Op::HandleWith { .. } => {}
        Op::GpuKernelLaunch {
            grid, block, args, ..
        } => {
            used_values.insert(*grid);
            used_values.insert(*block);
            used_values.extend(args.iter().copied());
        }
        Op::GpuIntrinsic { args, .. } => {
            used_values.extend(args.iter().copied());
        }
        Op::InlineAsm { args, .. } => {
            used_values.extend(args.iter().copied());
        }
        Op::ConstInt(_)
        | Op::ConstFloat(_)
        | Op::ConstBool(_)
        | Op::ConstString(_)
        | Op::Unit
        | Op::LoadLocal(_)
        | Op::Alloca { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use agam_errors::span::SourceId;
    use agam_hir::lower::HirLowering;
    use agam_lexer::Lexer;

    use crate::{ir::Op, lower::MirLowering, opt::constant_fold};

    use super::run;

    fn optimize_source(source: &str) -> crate::ir::MirModule {
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
        let mut mir = mir_lower.lower_module(&hir);
        constant_fold::run(&mut mir);
        run(&mut mir);
        mir
    }

    #[test]
    fn removes_dead_locals() {
        let mir = optimize_source("fn main() { let x = 10; let y = 20; return y; }");
        let instructions = &mir.functions[0].blocks[0].instructions;

        assert!(
            !instructions
                .iter()
                .any(|instr| matches!(&instr.op, Op::Alloca { name, .. } if name == "x"))
        );
        assert!(
            !instructions
                .iter()
                .any(|instr| matches!(&instr.op, Op::StoreLocal { name, .. } if name == "x"))
        );
    }

    #[test]
    fn removes_unreachable_blocks_after_folded_branch() {
        let mir = optimize_source("fn main() { if 1 < 2 { return 7; } else { return 9; } }");

        assert!(mir.functions[0].blocks.len() < 4);
    }
}
