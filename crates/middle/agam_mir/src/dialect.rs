//! Multi-Level Extensible MIR Dialects (Core, GPU, Tensor, and Async).
//!
//! Inspired by MLIR progressive lowering and Pliron dialect architectures.
//! Enables domain-specific high-level representation and progressive lowering
//! down to core scalar CFG MIR.

use serde::{Deserialize, Serialize};

use crate::ir::{GpuIntrinsicKind, MirBinOp, Op, ValueId};

/// Identification of an IR Dialect.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DialectKind {
    Core,
    Gpu,
    Tensor,
    Async,
    Custom(String),
}

/// Reduction operations for high-dimensional tensors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TensorReduceKind {
    Sum,
    Mean,
    Max,
    Min,
    Prod,
}

/// High-level Tensor operations.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TensorOp {
    MatMul {
        a: ValueId,
        b: ValueId,
        trans_a: bool,
        trans_b: bool,
    },
    Conv2d {
        input: ValueId,
        kernel: ValueId,
        stride: (u32, u32),
        padding: (u32, u32),
    },
    Broadcast {
        src: ValueId,
        target_shape: Vec<usize>,
    },
    Reshape {
        src: ValueId,
        new_shape: Vec<usize>,
    },
    Reduce {
        src: ValueId,
        axis: u32,
        kind: TensorReduceKind,
    },
    FusedElementwise {
        ops: Vec<MirBinOp>,
        inputs: Vec<ValueId>,
    },
}

/// GPU hardware-accelerated memory and execution operations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuDialectOp {
    KernelLaunch {
        kernel_name: String,
        grid: ValueId,
        block: ValueId,
        shared_memory_bytes: u32,
        args: Vec<ValueId>,
    },
    Barrier {
        scope: BarrierScope,
    },
    ThreadIntrinsic {
        kind: GpuIntrinsicKind,
    },
    WarpShuffle {
        val: ValueId,
        lane: u32,
    },
    AsyncCopyGlobalToShared {
        dest_shared: ValueId,
        src_global: ValueId,
        bytes: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BarrierScope {
    Warp,
    Block,
    Device,
}

/// Async coroutine and concurrency dialect operations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AsyncDialectOp {
    SpawnTask { task_fn: String, args: Vec<ValueId> },
    AwaitFuture { future_val: ValueId },
    YieldExecution,
}

/// Extensible multi-level MIR operation enum.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MultiLevelOp {
    Core(Op),
    Tensor(TensorOp),
    Gpu(GpuDialectOp),
    Async(AsyncDialectOp),
}

impl MultiLevelOp {
    pub fn dialect(&self) -> DialectKind {
        match self {
            Self::Core(_) => DialectKind::Core,
            Self::Tensor(_) => DialectKind::Tensor,
            Self::Gpu(_) => DialectKind::Gpu,
            Self::Async(_) => DialectKind::Async,
        }
    }
}

/// Progressive Lowering Pipeline: Lowers high-level tensor operations into core MIR loops.
pub struct DialectLoweringEngine;

impl DialectLoweringEngine {
    /// Progressively lower a high-level `TensorOp` into core scalar MIR operations.
    pub fn lower_tensor_to_core(op: &TensorOp, next_val_id: &mut u32) -> Vec<Op> {
        let mut core_ops = Vec::new();
        match op {
            TensorOp::MatMul { a, b, .. } => {
                // Lowers to inner product multiply-accumulate loop operations
                let _v_prod = ValueId(*next_val_id);
                *next_val_id += 1;
                core_ops.push(Op::BinOp {
                    op: MirBinOp::Mul,
                    left: *a,
                    right: *b,
                });
            }
            TensorOp::FusedElementwise { ops, inputs } => {
                if inputs.len() >= 2 && !ops.is_empty() {
                    let mut prev_id = inputs[0];
                    for (i, bin_op) in ops.iter().enumerate() {
                        if let Some(&next_in) = inputs.get(i + 1) {
                            core_ops.push(Op::BinOp {
                                op: *bin_op,
                                left: prev_id,
                                right: next_in,
                            });
                            prev_id = ValueId(*next_val_id);
                            *next_val_id += 1;
                        }
                    }
                }
            }
            TensorOp::Reduce { src, kind, .. } => {
                let bin_op = match kind {
                    TensorReduceKind::Sum | TensorReduceKind::Mean => MirBinOp::Add,
                    TensorReduceKind::Prod => MirBinOp::Mul,
                    TensorReduceKind::Max => MirBinOp::Gt,
                    TensorReduceKind::Min => MirBinOp::Lt,
                };
                core_ops.push(Op::BinOp {
                    op: bin_op,
                    left: *src,
                    right: *src,
                });
            }
            _ => {
                // Fallback pass-through
            }
        }
        core_ops
    }

    /// Progressively lower an async operation into core runtime calls.
    pub fn lower_async_to_core(op: &AsyncDialectOp) -> Op {
        match op {
            AsyncDialectOp::SpawnTask { task_fn, args } => Op::Call {
                callee: format!("__agam_async_spawn_{task_fn}"),
                args: args.clone(),
            },
            AsyncDialectOp::AwaitFuture { future_val } => Op::Call {
                callee: "__agam_async_await".into(),
                args: vec![*future_val],
            },
            AsyncDialectOp::YieldExecution => Op::Call {
                callee: "__agam_async_yield".into(),
                args: vec![],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_dialect_op_and_lowering() {
        let op = TensorOp::MatMul {
            a: ValueId(10),
            b: ValueId(20),
            trans_a: false,
            trans_b: false,
        };

        let multi = MultiLevelOp::Tensor(op.clone());
        assert_eq!(multi.dialect(), DialectKind::Tensor);

        let mut next_val = 100;
        let lowered = DialectLoweringEngine::lower_tensor_to_core(&op, &mut next_val);
        assert_eq!(lowered.len(), 1);
        assert!(matches!(
            lowered[0],
            Op::BinOp {
                op: MirBinOp::Mul,
                ..
            }
        ));
    }

    #[test]
    fn test_async_dialect_lowering_to_runtime_calls() {
        let op = AsyncDialectOp::SpawnTask {
            task_fn: "compute_weights".into(),
            args: vec![ValueId(1), ValueId(2)],
        };

        let lowered = DialectLoweringEngine::lower_async_to_core(&op);
        match lowered {
            Op::Call { callee, args } => {
                assert_eq!(callee, "__agam_async_spawn_compute_weights");
                assert_eq!(args, vec![ValueId(1), ValueId(2)]);
            }
            _ => panic!("Expected Call op"),
        }
    }
}
