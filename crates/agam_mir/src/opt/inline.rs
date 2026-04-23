//! Small-function MIR inlining.

use std::collections::{HashMap, HashSet};

use crate::ir::{Instruction, MirFunction, MirModule, Op, Terminator, ValueId};

pub fn run(module: &mut MirModule) -> bool {
    let candidates: HashMap<String, MirFunction> = module
        .functions
        .iter()
        .filter(|function| is_inline_candidate(function))
        .map(|function| (function.name.clone(), function.clone()))
        .collect();

    if candidates.is_empty() {
        return false;
    }

    let mut changed = false;

    for function in &mut module.functions {
        let function_name = function.name.clone();
        let mut state = InlineState::new(function);

        for block in &mut function.blocks {
            let original = std::mem::take(&mut block.instructions);
            let mut rewritten = Vec::with_capacity(original.len());

            for instr in original {
                if let Op::Call { callee, args } = &instr.op {
                    if callee != &function_name {
                        if let Some(callee_fn) = candidates.get(callee) {
                            rewritten.extend(state.inline_call(
                                instr.result,
                                instr.ty,
                                callee_fn,
                                args,
                            ));
                            changed = true;
                            continue;
                        }
                    }
                }

                rewritten.push(instr);
            }

            block.instructions = rewritten;
        }
    }

    changed
}

fn is_inline_candidate(function: &MirFunction) -> bool {
    if function.name == "main" {
        return false;
    }

    let reachable = reachable_blocks(function);
    if reachable.len() != 1 {
        return false;
    }

    let Some(block) = function
        .blocks
        .iter()
        .find(|block| block.id == function.entry)
    else {
        return false;
    };

    if !matches!(
        block.terminator,
        Terminator::Return(_) | Terminator::ReturnVoid
    ) {
        return false;
    }

    if block.instructions.len() > 8 {
        return false;
    }

    !block
        .instructions
        .iter()
        .any(|instr| matches!(instr.op, Op::Call { .. }))
}

struct InlineState {
    next_value: u32,
    next_inline: u32,
}

impl InlineState {
    fn new(function: &MirFunction) -> Self {
        let mut next_value = 0;

        for param in &function.params {
            next_value = next_value.max(param.value.0 + 1);
        }

        for block in &function.blocks {
            for instr in &block.instructions {
                next_value = next_value.max(instr.result.0 + 1);
            }
        }

        Self {
            next_value,
            next_inline: 0,
        }
    }

    fn fresh_value(&mut self) -> ValueId {
        let value = ValueId(self.next_value);
        self.next_value += 1;
        value
    }

    fn inline_call(
        &mut self,
        call_result: ValueId,
        call_ty: agam_sema::symbol::TypeId,
        callee: &MirFunction,
        args: &[ValueId],
    ) -> Vec<Instruction> {
        let inline_id = self.next_inline;
        self.next_inline += 1;

        let block = callee
            .blocks
            .iter()
            .find(|block| block.id == callee.entry)
            .expect("inline candidate must have an entry block");

        let mut result = Vec::new();
        let mut value_map: HashMap<ValueId, ValueId> = HashMap::new();
        let mut local_map: HashMap<String, String> = HashMap::new();
        let mut names_to_rename: HashSet<String> = callee
            .params
            .iter()
            .map(|param| param.name.clone())
            .collect();

        for instr in &block.instructions {
            if let Op::Alloca { name, .. } | Op::LoadLocal(name) = &instr.op {
                names_to_rename.insert(name.clone());
            }
            if let Op::StoreLocal { name, .. } = &instr.op {
                names_to_rename.insert(name.clone());
            }
        }

        for name in names_to_rename {
            local_map.insert(
                name.clone(),
                format!("__inl{}_{}_{}", inline_id, callee.name, name),
            );
        }

        for (index, param) in callee.params.iter().enumerate() {
            let local_name = local_map
                .get(&param.name)
                .cloned()
                .unwrap_or_else(|| format!("__inl{}_{}_{}", inline_id, callee.name, param.name));

            let alloca_result = self.fresh_value();
            result.push(Instruction {
                result: alloca_result,
                ty: param.ty,
                op: Op::Alloca {
                    name: local_name.clone(),
                    ty: param.ty,
                },
            });

            let store_result = self.fresh_value();
            result.push(Instruction {
                result: store_result,
                ty: param.ty,
                op: Op::StoreLocal {
                    name: local_name,
                    value: args.get(index).copied().unwrap_or(param.value),
                },
            });
        }

        for instr in &block.instructions {
            let new_result = self.fresh_value();
            value_map.insert(instr.result, new_result);

            result.push(Instruction {
                result: new_result,
                ty: instr.ty,
                op: remap_op(&instr.op, &value_map, &local_map),
            });
        }

        let return_op = match &block.terminator {
            Terminator::Return(value) => Op::Copy(remap_value(*value, &value_map)),
            Terminator::ReturnVoid => Op::Unit,
            _ => Op::Unit,
        };

        result.push(Instruction {
            result: call_result,
            ty: call_ty,
            op: return_op,
        });

        result
    }
}

fn reachable_blocks(function: &MirFunction) -> HashSet<crate::ir::BlockId> {
    let by_id: HashMap<crate::ir::BlockId, _> = function
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

fn remap_value(value: ValueId, value_map: &HashMap<ValueId, ValueId>) -> ValueId {
    value_map.get(&value).copied().unwrap_or(value)
}

fn remap_local(name: &str, local_map: &HashMap<String, String>) -> String {
    local_map
        .get(name)
        .cloned()
        .unwrap_or_else(|| name.to_string())
}

fn remap_op(
    op: &Op,
    value_map: &HashMap<ValueId, ValueId>,
    local_map: &HashMap<String, String>,
) -> Op {
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
        Op::LoadLocal(name) => Op::LoadLocal(remap_local(name, local_map)),
        Op::StoreLocal { name, value } => Op::StoreLocal {
            name: remap_local(name, local_map),
            value: remap_value(*value, value_map),
        },
        Op::Alloca { name, ty } => Op::Alloca {
            name: remap_local(name, local_map),
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
        Op::EffectPerform {
            effect,
            operation,
            args,
        } => Op::EffectPerform {
            effect: effect.clone(),
            operation: operation.clone(),
            args: args
                .iter()
                .map(|arg| remap_value(*arg, value_map))
                .collect(),
        },
        Op::HandleWith {
            effect,
            handler,
            body,
        } => Op::HandleWith {
            effect: effect.clone(),
            handler: handler.clone(),
            body: *body,
        },
        Op::GpuKernelLaunch {
            kernel_name,
            grid,
            block,
            args,
        } => Op::GpuKernelLaunch {
            kernel_name: kernel_name.clone(),
            grid: remap_value(*grid, value_map),
            block: remap_value(*block, value_map),
            args: args
                .iter()
                .map(|arg| remap_value(*arg, value_map))
                .collect(),
        },
        Op::GpuIntrinsic { kind, args } => Op::GpuIntrinsic {
            kind: *kind,
            args: args
                .iter()
                .map(|arg| remap_value(*arg, value_map))
                .collect(),
        },
        Op::InlineAsm {
            asm_string,
            constraints,
            args,
        } => Op::InlineAsm {
            asm_string: asm_string.clone(),
            constraints: constraints.clone(),
            args: args
                .iter()
                .map(|arg| remap_value(*arg, value_map))
                .collect(),
        },
    }
}

#[cfg(test)]
mod tests {
    use agam_errors::span::SourceId;
    use agam_hir::lower::HirLowering;
    use agam_lexer::Lexer;

    use crate::{ir::Op, lower::MirLowering};

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
        run(&mut mir);
        mir
    }

    #[test]
    fn inlines_small_leaf_calls() {
        let mir = optimize_source(
            "fn add1(a: i32) -> i32 { return a + 1; } fn main() { let x = add1(41); return x; }",
        );
        let main_fn = mir
            .functions
            .iter()
            .find(|function| function.name == "main")
            .unwrap();

        assert!(
            !main_fn
                .blocks
                .iter()
                .flat_map(|block| &block.instructions)
                .any(|instr| { matches!(&instr.op, Op::Call { callee, .. } if callee == "add1") })
        );
    }
}
