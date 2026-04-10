//! Conservative fixed-trip loop unrolling for MIR.

use std::collections::HashMap;

use crate::ir::{BlockId, Instruction, MirBinOp, MirFunction, MirModule, Op, Terminator, ValueId};

const MAX_UNROLL_ITERS: usize = 8;

pub fn run(module: &mut MirModule) -> bool {
    let mut changed = false;

    for function in &mut module.functions {
        changed |= unroll_function(function);
    }

    changed
}

fn unroll_function(function: &mut MirFunction) -> bool {
    let block_by_id: HashMap<BlockId, usize> = function
        .blocks
        .iter()
        .enumerate()
        .map(|(index, block)| (block.id, index))
        .collect();
    let predecessors = collect_predecessors(function);
    let next_value = next_value_id(function);

    for cond_index in 0..function.blocks.len() {
        let Some(plan) = analyze_loop(function, cond_index, &block_by_id, &predecessors) else {
            continue;
        };

        let mut cloner = ValueCloner::new(next_value);
        let mut unrolled = Vec::new();
        for _ in 0..plan.trip_count {
            let mut value_map = HashMap::new();
            for instr in &function.blocks[plan.body_index].instructions {
                unrolled.push(cloner.clone_instruction(instr, &mut value_map));
            }
        }

        let preheader = &mut function.blocks[plan.preheader_index];
        preheader.instructions.extend(unrolled);
        preheader.terminator = Terminator::Jump(plan.exit_block);
        return true;
    }

    false
}

struct LoopPlan {
    preheader_index: usize,
    body_index: usize,
    exit_block: BlockId,
    trip_count: usize,
}

fn analyze_loop(
    function: &MirFunction,
    cond_index: usize,
    block_by_id: &HashMap<BlockId, usize>,
    predecessors: &HashMap<BlockId, Vec<BlockId>>,
) -> Option<LoopPlan> {
    let cond_block = function.blocks.get(cond_index)?;
    let Terminator::Branch {
        condition,
        then_block,
        else_block,
    } = cond_block.terminator
    else {
        return None;
    };

    let &body_index = block_by_id.get(&then_block)?;
    let body_block = function.blocks.get(body_index)?;
    if !matches!(body_block.terminator, Terminator::Jump(target) if target == cond_block.id) {
        return None;
    }

    let preheaders = predecessors
        .get(&cond_block.id)?
        .iter()
        .copied()
        .filter(|block| *block != then_block)
        .collect::<Vec<_>>();
    if preheaders.len() != 1 {
        return None;
    }

    let preheader_id = preheaders[0];
    let &preheader_index = block_by_id.get(&preheader_id)?;
    if !matches!(
        function.blocks[preheader_index].terminator,
        Terminator::Jump(target) if target == cond_block.id
    ) {
        return None;
    }

    let (loop_var, cmp_op, bound) = analyze_condition(cond_block, condition)?;
    let start = find_initial_value(&function.blocks[preheader_index].instructions, &loop_var)?;
    let step = find_step(body_block, &loop_var)?;
    let trip_count = compute_trip_count(start, bound, step, cmp_op)?;
    if trip_count > MAX_UNROLL_ITERS {
        return None;
    }

    Some(LoopPlan {
        preheader_index,
        body_index,
        exit_block: else_block,
        trip_count,
    })
}

fn collect_predecessors(function: &MirFunction) -> HashMap<BlockId, Vec<BlockId>> {
    let mut predecessors: HashMap<BlockId, Vec<BlockId>> = HashMap::new();

    for block in &function.blocks {
        match block.terminator {
            Terminator::Jump(target) => {
                predecessors.entry(target).or_default().push(block.id);
            }
            Terminator::Branch {
                then_block,
                else_block,
                ..
            } => {
                predecessors.entry(then_block).or_default().push(block.id);
                predecessors.entry(else_block).or_default().push(block.id);
            }
            Terminator::Return(_) | Terminator::ReturnVoid | Terminator::Unreachable => {}
        }
    }

    predecessors
}

fn next_value_id(function: &MirFunction) -> u32 {
    let mut next = 0;

    for param in &function.params {
        next = next.max(param.value.0 + 1);
    }
    for block in &function.blocks {
        for instr in &block.instructions {
            next = next.max(instr.result.0 + 1);
        }
    }

    next
}

fn analyze_condition(
    block: &crate::ir::BasicBlock,
    condition: ValueId,
) -> Option<(String, MirBinOp, i64)> {
    let instructions: HashMap<ValueId, &Instruction> = block
        .instructions
        .iter()
        .map(|instr| (instr.result, instr))
        .collect();
    let compare = instructions.get(&condition)?;
    let Op::BinOp { op, left, right } = compare.op else {
        return None;
    };

    let left_local = resolve_loaded_local(left, &instructions);
    let right_local = resolve_loaded_local(right, &instructions);
    let left_const = resolve_const_int(left, &instructions);
    let right_const = resolve_const_int(right, &instructions);

    match (left_local, right_const, left_const, right_local) {
        (Some(name), Some(bound), _, _) => Some((name, op, bound)),
        (_, _, Some(bound), Some(name)) => Some((name, flip_cmp(op), bound)),
        _ => None,
    }
}

fn flip_cmp(op: MirBinOp) -> MirBinOp {
    match op {
        MirBinOp::Lt => MirBinOp::Gt,
        MirBinOp::LtEq => MirBinOp::GtEq,
        MirBinOp::Gt => MirBinOp::Lt,
        MirBinOp::GtEq => MirBinOp::LtEq,
        other => other,
    }
}

fn resolve_loaded_local(
    value: ValueId,
    instructions: &HashMap<ValueId, &Instruction>,
) -> Option<String> {
    match &instructions.get(&value)?.op {
        Op::LoadLocal(name) => Some(name.clone()),
        _ => None,
    }
}

fn resolve_const_int(value: ValueId, instructions: &HashMap<ValueId, &Instruction>) -> Option<i64> {
    match instructions.get(&value)?.op {
        Op::ConstInt(value) => Some(value),
        _ => None,
    }
}

fn find_initial_value(instructions: &[Instruction], loop_var: &str) -> Option<i64> {
    let by_result: HashMap<ValueId, &Instruction> = instructions
        .iter()
        .map(|instr| (instr.result, instr))
        .collect();

    instructions.iter().rev().find_map(|instr| match &instr.op {
        Op::StoreLocal { name, value } if name == loop_var => resolve_const_int(*value, &by_result),
        _ => None,
    })
}

fn find_step(block: &crate::ir::BasicBlock, loop_var: &str) -> Option<i64> {
    let by_result: HashMap<ValueId, &Instruction> = block
        .instructions
        .iter()
        .map(|instr| (instr.result, instr))
        .collect();
    let stores = block
        .instructions
        .iter()
        .filter_map(|instr| match &instr.op {
            Op::StoreLocal { name, value } if name == loop_var => Some(*value),
            _ => None,
        })
        .collect::<Vec<_>>();

    if stores.len() != 1 {
        return None;
    }

    let update = by_result.get(&stores[0])?;
    let Op::BinOp { op, left, right } = update.op else {
        return None;
    };

    match op {
        MirBinOp::Add => {
            if matches!(&by_result.get(&left)?.op, Op::LoadLocal(name) if name == loop_var) {
                resolve_const_int(right, &by_result)
            } else if matches!(&by_result.get(&right)?.op, Op::LoadLocal(name) if name == loop_var)
            {
                resolve_const_int(left, &by_result)
            } else {
                None
            }
        }
        MirBinOp::Sub => {
            if matches!(&by_result.get(&left)?.op, Op::LoadLocal(name) if name == loop_var) {
                resolve_const_int(right, &by_result).map(|value| -value)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn compute_trip_count(start: i64, bound: i64, step: i64, cmp_op: MirBinOp) -> Option<usize> {
    if step == 0 {
        return None;
    }

    let start = i128::from(start);
    let bound = i128::from(bound);
    let step = i128::from(step);

    let count = match cmp_op {
        MirBinOp::Lt if step > 0 => {
            if start >= bound {
                0
            } else {
                ((bound - start - 1) / step) + 1
            }
        }
        MirBinOp::LtEq if step > 0 => {
            if start > bound {
                0
            } else {
                ((bound - start) / step) + 1
            }
        }
        MirBinOp::Gt if step < 0 => {
            let pos_step = -step;
            if start <= bound {
                0
            } else {
                ((start - bound - 1) / pos_step) + 1
            }
        }
        MirBinOp::GtEq if step < 0 => {
            let pos_step = -step;
            if start < bound {
                0
            } else {
                ((start - bound) / pos_step) + 1
            }
        }
        _ => return None,
    };

    usize::try_from(count).ok()
}

struct ValueCloner {
    next_value: u32,
}

impl ValueCloner {
    fn new(next_value: u32) -> Self {
        Self { next_value }
    }

    fn fresh_value(&mut self) -> ValueId {
        let value = ValueId(self.next_value);
        self.next_value += 1;
        value
    }

    fn clone_instruction(
        &mut self,
        instr: &Instruction,
        value_map: &mut HashMap<ValueId, ValueId>,
    ) -> Instruction {
        let result = self.fresh_value();
        let cloned = Instruction {
            result,
            ty: instr.ty,
            op: clone_op(&instr.op, value_map),
        };
        value_map.insert(instr.result, result);
        cloned
    }
}

fn clone_op(op: &Op, value_map: &HashMap<ValueId, ValueId>) -> Op {
    match op {
        Op::ConstInt(value) => Op::ConstInt(*value),
        Op::ConstFloat(value) => Op::ConstFloat(*value),
        Op::ConstBool(value) => Op::ConstBool(*value),
        Op::ConstString(value) => Op::ConstString(value.clone()),
        Op::Unit => Op::Unit,
        Op::BinOp { op, left, right } => Op::BinOp {
            op: *op,
            left: remap_value(*left, value_map),
            right: remap_value(*right, value_map),
        },
        Op::UnOp { op, operand } => Op::UnOp {
            op: *op,
            operand: remap_value(*operand, value_map),
        },
        Op::Call { callee, args } => Op::Call {
            callee: callee.clone(),
            args: args
                .iter()
                .map(|arg| remap_value(*arg, value_map))
                .collect(),
        },
        Op::Copy(value) => Op::Copy(remap_value(*value, value_map)),
        Op::LoadLocal(name) => Op::LoadLocal(name.clone()),
        Op::StoreLocal { name, value } => Op::StoreLocal {
            name: name.clone(),
            value: remap_value(*value, value_map),
        },
        Op::Alloca { name, ty } => Op::Alloca {
            name: name.clone(),
            ty: *ty,
        },
        Op::GetField { object, field } => Op::GetField {
            object: remap_value(*object, value_map),
            field: field.clone(),
        },
        Op::GetIndex { object, index } => Op::GetIndex {
            object: remap_value(*object, value_map),
            index: remap_value(*index, value_map),
        },
        Op::Phi(entries) => Op::Phi(
            entries
                .iter()
                .map(|(block, value)| (*block, remap_value(*value, value_map)))
                .collect(),
        ),
        Op::Cast { value, target_ty } => Op::Cast {
            value: remap_value(*value, value_map),
            target_ty: *target_ty,
        },
    }
}

fn remap_value(value: ValueId, value_map: &HashMap<ValueId, ValueId>) -> ValueId {
    value_map.get(&value).copied().unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use agam_errors::span::SourceId;
    use agam_hir::lower::HirLowering;
    use agam_lexer::Lexer;

    use crate::{
        ir::{MirBinOp, Op, Terminator},
        lower::MirLowering,
        opt,
    };

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
        opt::optimize_module(&mut mir);
        mir
    }

    #[test]
    fn unrolls_small_constant_counted_loops() {
        let mir = optimize_source(
            "fn main() -> i32 { \
                let mut i: i32 = 0; \
                let mut total: i32 = 0; \
                while i < 4 { \
                    total = total + i; \
                    i = i + 1; \
                } \
                return total; \
            }",
        );

        assert!(
            mir.functions[0]
                .blocks
                .iter()
                .all(|block| { !matches!(block.terminator, Terminator::Branch { .. }) })
        );
        let has_loop_compare = mir.functions[0].blocks.iter().any(|block| {
            block.instructions.iter().any(|instr| {
                matches!(
                    instr.op,
                    Op::BinOp {
                        op: MirBinOp::Lt | MirBinOp::LtEq | MirBinOp::Gt | MirBinOp::GtEq,
                        ..
                    }
                )
            })
        });
        assert!(!has_loop_compare);
    }

    #[test]
    fn keeps_dynamic_bound_loops_intact() {
        let source_id = SourceId(0);
        let mut lexer = Lexer::new(
            "fn main(n: i32) -> i32 { \
                let mut i: i32 = 0; \
                while i < n { i = i + 1; } \
                return i; \
            }",
            source_id,
        );
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

        assert!(!run(&mut mir));
    }
}
