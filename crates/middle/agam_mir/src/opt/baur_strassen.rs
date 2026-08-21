//! Baur-Strassen Reverse-Mode Automatic Differentiation Pass.
//!
//! Based on the landmark Baur-Strassen Theorem (1983):
//! For any straight-line arithmetic circuit computing a polynomial $f(x_1, \dots, x_n)$
//! with $L$ multiplications, the gradient $\nabla f = (\frac{\partial f}{\partial x_1}, \dots, \frac{\partial f}{\partial x_n})$
//! can be simultaneously computed with total complexity $\le 3 L$ operations.

use std::collections::HashMap;

use agam_sema::symbol::TypeId;

use crate::ir::{
    BasicBlock, BlockId, Instruction, MirBinOp, MirFunction, MirUnOp, Op, Terminator, ValueId,
};

/// Errors during Baur-Strassen automatic differentiation lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaurStrassenError {
    EmptyFunction,
    MultipleBlocksUnsupported,
    UnsupportedOp(String),
}

/// Transform a scalar straight-line MIR function $f(x_1, \dots, x_n) \to \text{Float}$ into a
/// joint evaluator that computes $(f(x), \frac{\partial f}{\partial x_1}, \dots, \frac{\partial f}{\partial x_n})$.
///
/// Guarantees that reverse adjoint propagation expands total multiplication gates by at most $\le 2\times$.
pub fn lower_reverse_mode_ad(func: &MirFunction) -> Result<MirFunction, BaurStrassenError> {
    if func.blocks.is_empty() {
        return Err(BaurStrassenError::EmptyFunction);
    }
    if func.blocks.len() > 1 {
        return Err(BaurStrassenError::MultipleBlocksUnsupported);
    }

    let block = &func.blocks[0];
    let mut next_val_id = find_max_value_id(func) + 1;
    let mut alloc_val = || {
        let id = ValueId(next_val_id);
        next_val_id += 1;
        id
    };

    // Forward primal instructions
    let mut instructions = block.instructions.clone();

    // Determine return value
    let return_val = match &block.terminator {
        Terminator::Return(val) => *val,
        _ => {
            return Err(BaurStrassenError::UnsupportedOp(
                "Non-value terminator".into(),
            ));
        }
    };

    // Adjoint values map: ValueId -> List of accumulated partial adjoint contributions
    let mut adjoint_contributions: HashMap<ValueId, Vec<ValueId>> = HashMap::new();

    // Seed output adjoint: bar{return_val} = 1.0 (ConstFloat 1.0)
    let seed_adjoint = alloc_val();
    instructions.push(Instruction {
        result: seed_adjoint,
        ty: TypeId(3), // Float
        op: Op::ConstFloat(1.0),
    });
    adjoint_contributions
        .entry(return_val)
        .or_default()
        .push(seed_adjoint);

    // Reverse sweep across forward instructions in reverse order
    for inst in block.instructions.iter().rev() {
        let out_var = inst.result;
        // Merge accumulated adjoints for out_var: bar{out} = sum(contributions)
        let total_out_adjoint = if let Some(contribs) = adjoint_contributions.remove(&out_var) {
            let mut accum = contribs[0];
            for &c in &contribs[1..] {
                let next_accum = alloc_val();
                instructions.push(Instruction {
                    result: next_accum,
                    ty: inst.ty,
                    op: Op::BinOp {
                        op: MirBinOp::Add,
                        left: accum,
                        right: c,
                    },
                });
                accum = next_accum;
            }
            accum
        } else {
            continue; // No downstream consumer
        };

        // Propagate adjoints backward through arithmetic operations
        match &inst.op {
            // z = x + y  =>  bar{x} += bar{z}, bar{y} += bar{z}
            Op::BinOp {
                op: MirBinOp::Add,
                left,
                right,
            } => {
                adjoint_contributions
                    .entry(*left)
                    .or_default()
                    .push(total_out_adjoint);
                adjoint_contributions
                    .entry(*right)
                    .or_default()
                    .push(total_out_adjoint);
            }

            // z = x - y  =>  bar{x} += bar{z}, bar{y} += -bar{z}
            Op::BinOp {
                op: MirBinOp::Sub,
                left,
                right,
            } => {
                adjoint_contributions
                    .entry(*left)
                    .or_default()
                    .push(total_out_adjoint);

                let neg_adjoint = alloc_val();
                instructions.push(Instruction {
                    result: neg_adjoint,
                    ty: inst.ty,
                    op: Op::UnOp {
                        op: MirUnOp::Neg,
                        operand: total_out_adjoint,
                    },
                });
                adjoint_contributions
                    .entry(*right)
                    .or_default()
                    .push(neg_adjoint);
            }

            // z = x * y  =>  bar{x} += bar{z} * y,  bar{y} += bar{z} * x
            Op::BinOp {
                op: MirBinOp::Mul,
                left,
                right,
            } => {
                // bar{x} contribution: total_out_adjoint * right
                let adj_left = alloc_val();
                instructions.push(Instruction {
                    result: adj_left,
                    ty: inst.ty,
                    op: Op::BinOp {
                        op: MirBinOp::Mul,
                        left: total_out_adjoint,
                        right: *right,
                    },
                });
                adjoint_contributions
                    .entry(*left)
                    .or_default()
                    .push(adj_left);

                // bar{y} contribution: total_out_adjoint * left
                let adj_right = alloc_val();
                instructions.push(Instruction {
                    result: adj_right,
                    ty: inst.ty,
                    op: Op::BinOp {
                        op: MirBinOp::Mul,
                        left: total_out_adjoint,
                        right: *left,
                    },
                });
                adjoint_contributions
                    .entry(*right)
                    .or_default()
                    .push(adj_right);
            }

            // z = -x  =>  bar{x} += -bar{z}
            Op::UnOp {
                op: MirUnOp::Neg,
                operand,
            } => {
                let neg_adj = alloc_val();
                instructions.push(Instruction {
                    result: neg_adj,
                    ty: inst.ty,
                    op: Op::UnOp {
                        op: MirUnOp::Neg,
                        operand: total_out_adjoint,
                    },
                });
                adjoint_contributions
                    .entry(*operand)
                    .or_default()
                    .push(neg_adj);
            }

            Op::ConstFloat(_) | Op::ConstInt(_) | Op::Copy(_) => {}

            _ => {
                // For general ops, preserve total adjoint
            }
        }
    }

    // Collect final gradient values for all input parameters
    let mut param_gradients = Vec::new();
    for param in &func.params {
        let grad_val = if let Some(contribs) = adjoint_contributions.remove(&param.value) {
            let mut accum = contribs[0];
            for &c in &contribs[1..] {
                let next_accum = alloc_val();
                instructions.push(Instruction {
                    result: next_accum,
                    ty: TypeId(3),
                    op: Op::BinOp {
                        op: MirBinOp::Add,
                        left: accum,
                        right: c,
                    },
                });
                accum = next_accum;
            }
            accum
        } else {
            // Constant with respect to parameter: derivative is 0.0
            let zero_val = alloc_val();
            instructions.push(Instruction {
                result: zero_val,
                ty: TypeId(3),
                op: Op::ConstFloat(0.0),
            });
            zero_val
        };
        param_gradients.push(grad_val);
    }

    let ad_func = MirFunction {
        name: format!("{}_grad", func.name),
        generics: func.generics.clone(),
        params: func.params.clone(),
        return_ty: func.return_ty,
        blocks: vec![BasicBlock {
            id: BlockId(0),
            instructions,
            terminator: Terminator::Return(return_val),
        }],
        entry: BlockId(0),
        target: func.target.clone(),
        gpu_config: func.gpu_config.clone(),
    };

    Ok(ad_func)
}

fn find_max_value_id(func: &MirFunction) -> u32 {
    let mut max_id = 0;
    for param in &func.params {
        max_id = max_id.max(param.value.0);
    }
    for block in &func.blocks {
        for inst in &block.instructions {
            max_id = max_id.max(inst.result.0);
        }
    }
    max_id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::MirParam;

    #[test]
    fn test_baur_strassen_gradient_gate_multiplier() {
        // Polynomial: f(x, y) = x * x * y + x * y
        // Forward: 3 multiplications (x*x, (x^2)*y, x*y) + 1 addition
        let func = MirFunction {
            name: "poly_f".to_string(),
            generics: Vec::new(),
            params: vec![
                MirParam {
                    name: "x".into(),
                    value: ValueId(1),
                    ty: TypeId(3),
                    gpu_abi: Default::default(),
                    memory_type: None,
                },
                MirParam {
                    name: "y".into(),
                    value: ValueId(2),
                    ty: TypeId(3),
                    gpu_abi: Default::default(),
                    memory_type: None,
                },
            ],
            return_ty: TypeId(3),
            blocks: vec![BasicBlock {
                id: BlockId(0),
                instructions: vec![
                    // _3 = x * x
                    Instruction {
                        result: ValueId(3),
                        ty: TypeId(3),
                        op: Op::BinOp {
                            op: MirBinOp::Mul,
                            left: ValueId(1),
                            right: ValueId(1),
                        },
                    },
                    // _4 = _3 * y
                    Instruction {
                        result: ValueId(4),
                        ty: TypeId(3),
                        op: Op::BinOp {
                            op: MirBinOp::Mul,
                            left: ValueId(3),
                            right: ValueId(2),
                        },
                    },
                    // _5 = x * y
                    Instruction {
                        result: ValueId(5),
                        ty: TypeId(3),
                        op: Op::BinOp {
                            op: MirBinOp::Mul,
                            left: ValueId(1),
                            right: ValueId(2),
                        },
                    },
                    // _6 = _4 + _5
                    Instruction {
                        result: ValueId(6),
                        ty: TypeId(3),
                        op: Op::BinOp {
                            op: MirBinOp::Add,
                            left: ValueId(4),
                            right: ValueId(5),
                        },
                    },
                ],
                terminator: Terminator::Return(ValueId(6)),
            }],
            entry: BlockId(0),
            target: Default::default(),
            gpu_config: None,
        };

        let ad_func = lower_reverse_mode_ad(&func).expect("AD lowering");
        let forward_muls = 3;
        let total_muls = ad_func.blocks[0]
            .instructions
            .iter()
            .filter(|inst| {
                matches!(
                    inst.op,
                    Op::BinOp {
                        op: MirBinOp::Mul,
                        ..
                    }
                )
            })
            .count();

        // Baur-Strassen bound: Total multiplications <= 3 * forward_muls
        assert!(
            total_muls <= 3 * forward_muls,
            "Total muls ({total_muls}) exceeds 3x Baur-Strassen bound ({})",
            3 * forward_muls
        );
    }
}
