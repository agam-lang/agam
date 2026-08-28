//! Integration tests for E-Graph Superoptimization & Algebraic Tensor Fusion.

#![allow(deprecated)]

use std::collections::HashMap;

use agam_mir::ir::{
    BasicBlock, BlockId, Instruction, MirBinOp, MirFunction, MirModule, Op, Terminator, ValueId,
};
use agam_mir::opt::egraph::{CostModel, EGraph, ENode, Extractor};
use agam_sema::symbol::TypeId;

#[test]
fn test_arithmetic_identity_saturation() {
    let mut egraph = EGraph::new();

    // Expression: x * 1 + 0
    let var_x = egraph.add(ENode::Var(ValueId(10)));
    let const_one = egraph.add(ENode::ConstInt(1));
    let const_zero = egraph.add(ENode::ConstInt(0));

    let mul_node = egraph.add(ENode::BinOp {
        op: MirBinOp::Mul,
        left: var_x,
        right: const_one,
    });

    let add_node = egraph.add(ENode::BinOp {
        op: MirBinOp::Add,
        left: mul_node,
        right: const_zero,
    });

    // Saturate
    let matches = egraph.saturate(5);
    assert!(matches > 0, "Expected rewrite rules to fire");

    // Extract best
    let extractor = Extractor::new(&egraph, CostModel);
    let (cost, best_node) = extractor.find_best(add_node).expect("Expected best node");

    // Best node should have simplified to x directly (cost 1)
    assert_eq!(best_node, ENode::Var(ValueId(10)));
    assert_eq!(cost, 1);
}

#[test]
fn test_constant_folding_saturation() {
    let mut egraph = EGraph::new();

    // Expression: (10 + 20) * 2
    let c10 = egraph.add(ENode::ConstInt(10));
    let c20 = egraph.add(ENode::ConstInt(20));
    let c2 = egraph.add(ENode::ConstInt(2));

    let add = egraph.add(ENode::BinOp {
        op: MirBinOp::Add,
        left: c10,
        right: c20,
    });

    let mul = egraph.add(ENode::BinOp {
        op: MirBinOp::Mul,
        left: add,
        right: c2,
    });

    egraph.saturate(5);

    let extractor = Extractor::new(&egraph, CostModel);
    let (_, best_node) = extractor.find_best(mul).expect("Expected best node");

    assert_eq!(best_node, ENode::ConstInt(60));
}

#[test]
fn test_shift_coalescing() {
    let mut egraph = EGraph::new();

    // Expression: (x << 2) << 3
    let var_x = egraph.add(ENode::Var(ValueId(1)));
    let c2 = egraph.add(ENode::ConstInt(2));
    let c3 = egraph.add(ENode::ConstInt(3));

    let shift1 = egraph.add(ENode::BinOp {
        op: MirBinOp::Shl,
        left: var_x,
        right: c2,
    });

    let shift2 = egraph.add(ENode::BinOp {
        op: MirBinOp::Shl,
        left: shift1,
        right: c3,
    });

    egraph.saturate(5);

    let extractor = Extractor::new(&egraph, CostModel);
    let (_, best_node) = extractor.find_best(shift2).expect("Expected best node");

    if let ENode::BinOp { op, left, right } = best_node {
        assert_eq!(op, MirBinOp::Shl);
        assert_eq!(egraph.get_canonical_node(left), ENode::Var(ValueId(1)));
        assert_eq!(egraph.get_canonical_node(right), ENode::ConstInt(5));
    } else {
        panic!("Expected shifted node, found {:?}", best_node);
    }
}

#[test]
fn test_square_zero_nilpotent_cancellation() {
    let mut egraph = EGraph::new();

    // Nilpotent variable z_1 with degree 2 (z_1^2 = 0 in S)
    let var_z = egraph.add(ENode::Var(ValueId(42)));
    let nilpotent_term = egraph.add(ENode::NilpotentTerm {
        var: var_z,
        degree: 2,
    });

    egraph.saturate(5);

    let extractor = Extractor::new(&egraph, CostModel);
    let (_, best_node) = extractor
        .find_best(nilpotent_term)
        .expect("Expected best node");

    assert_eq!(best_node, ENode::ConstInt(0));
}

#[test]
fn test_fused_matmul_add_tensor_contraction() {
    let mut egraph = EGraph::new();

    // Expression: MatMul(A, B) + C
    let mat_a = egraph.add(ENode::Var(ValueId(100)));
    let mat_b = egraph.add(ENode::Var(ValueId(101)));
    let bias_c = egraph.add(ENode::Var(ValueId(102)));

    let matmul = egraph.add(ENode::TensorMatMul {
        a: mat_a,
        b: mat_b,
        trans_a: false,
        trans_b: false,
    });

    let add_bias = egraph.add(ENode::BinOp {
        op: MirBinOp::Add,
        left: matmul,
        right: bias_c,
    });

    egraph.saturate(5);

    let extractor = Extractor::new(&egraph, CostModel);
    let (cost, best_node) = extractor.find_best(add_bias).expect("Expected best node");

    // Fused node should be chosen due to lower cost (4 vs 13)
    match best_node {
        ENode::FusedMatmulAdd { a, b, bias, .. } => {
            assert_eq!(a, mat_a);
            assert_eq!(b, mat_b);
            assert_eq!(bias, bias_c);
            assert!(cost < 10, "Fused cost must be lower than unfused matmul");
        }
        _ => panic!("Expected FusedMatmulAdd, found {:?}", best_node),
    }
}

#[test]
fn test_fused_conv2d_relu_contraction() {
    let mut egraph = EGraph::new();

    // Expression: Relu(Conv2d(Input, Kernel) + Bias)
    let input = egraph.add(ENode::Var(ValueId(200)));
    let kernel = egraph.add(ENode::Var(ValueId(201)));
    let bias = egraph.add(ENode::Var(ValueId(202)));

    let conv = egraph.add(ENode::TensorConv2d {
        input,
        kernel,
        stride: (1, 1),
        padding: (0, 0),
    });

    let add_bias = egraph.add(ENode::BinOp {
        op: MirBinOp::Add,
        left: conv,
        right: bias,
    });

    let relu_call = egraph.add(ENode::Call {
        callee: "relu".to_string(),
        args: vec![add_bias],
    });

    egraph.saturate(5);

    let extractor = Extractor::new(&egraph, CostModel);
    let (cost, best_node) = extractor.find_best(relu_call).expect("Expected best node");

    match best_node {
        ENode::FusedConv2dRelu {
            input: in_id,
            kernel: k_id,
            bias: b_id,
            ..
        } => {
            assert_eq!(in_id, input);
            assert_eq!(k_id, kernel);
            assert_eq!(b_id, Some(bias));
            assert!(cost < 15, "Fused conv2d relu cost must be minimal");
        }
        _ => panic!("Expected FusedConv2dRelu, found {:?}", best_node),
    }
}

#[test]
fn test_mir_module_egraph_optimization_pass() {
    let mut module = MirModule {
        functions: vec![MirFunction {
            name: "compute".to_string(),
            generics: Vec::new(),
            params: Vec::new(),
            return_ty: TypeId(1),
            blocks: vec![BasicBlock {
                id: BlockId(0),
                instructions: vec![
                    // _1 = 10
                    Instruction {
                        result: ValueId(1),
                        ty: TypeId(1),
                        op: Op::ConstInt(10),
                    },
                    // _2 = 20
                    Instruction {
                        result: ValueId(2),
                        ty: TypeId(1),
                        op: Op::ConstInt(20),
                    },
                    // _3 = _1 + _2  (folded to 30)
                    Instruction {
                        result: ValueId(3),
                        ty: TypeId(1),
                        op: Op::BinOp {
                            op: MirBinOp::Add,
                            left: ValueId(1),
                            right: ValueId(2),
                        },
                    },
                ],
                terminator: Terminator::Return(ValueId(3)),
            }],
            entry: BlockId(0),
            target: Default::default(),
            gpu_config: None,
        }],
        enum_layouts: HashMap::new(),
        struct_layouts: HashMap::new(),
    };

    let changed = agam_mir::opt::optimize_module(&mut module);
    assert!(changed, "Expected optimizer to modify module");

    // DCE eliminates dead constants _1 and _2, leaving only _3 = ConstInt(30)
    assert_eq!(module.functions[0].blocks[0].instructions.len(), 1);
    let final_op = &module.functions[0].blocks[0].instructions[0].op;
    assert_eq!(*final_op, Op::ConstInt(30));
}
