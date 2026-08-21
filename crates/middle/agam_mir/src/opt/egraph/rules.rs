//! Equality Saturation Rules for E-Graph Superoptimization.
//!
//! Implements algebraic arithmetic simplifications, bitwise identities,
//! square-zero nilpotent variable cancellations, and fused tensor contractions.

use super::{EClassId, EGraph, ENode};
use crate::ir::MirBinOp;

/// Result of matching a rewrite pattern.
#[derive(Debug, Clone)]
pub enum MatchAction {
    Node(ENode),
    ShiftCoalesce { x: EClassId, total_shift: i64 },
}

#[derive(Debug, Clone)]
pub struct Match {
    pub class: EClassId,
    pub action: MatchAction,
}

/// Apply all rewrite rules to saturation or until no new equivalences are found.
pub fn apply_rules(egraph: &mut EGraph) -> usize {
    let mut matches = Vec::new();

    // 1. Collect all pattern matches across all e-classes
    for class in egraph.classes() {
        let class_id = class.id;
        for node in &class.nodes {
            match_arithmetic_identities(egraph, class_id, node, &mut matches);
            match_constant_folding(egraph, class_id, node, &mut matches);
            match_bitwise_rules(egraph, class_id, node, &mut matches);
            match_square_zero_nilpotent(egraph, class_id, node, &mut matches);
            match_tensor_fusions(egraph, class_id, node, &mut matches);
        }
    }

    let match_count = matches.len();

    // 2. Apply all matches by adding replacement nodes and unioning
    for m in matches {
        let rep_id = match m.action {
            MatchAction::Node(node) => egraph.add(node),
            MatchAction::ShiftCoalesce { x, total_shift } => {
                let shift_const = egraph.add(ENode::ConstInt(total_shift));
                egraph.add(ENode::BinOp {
                    op: MirBinOp::Shl,
                    left: x,
                    right: shift_const,
                })
            }
        };
        egraph.union(m.class, rep_id);
    }

    match_count
}

/// Match classical arithmetic algebraic identities.
fn match_arithmetic_identities(
    egraph: &EGraph,
    class_id: EClassId,
    node: &ENode,
    matches: &mut Vec<Match>,
) {
    match node {
        // x + 0 => x
        ENode::BinOp {
            op: MirBinOp::Add,
            left,
            right,
        } => {
            if is_const_zero(egraph, *right) {
                matches.push(Match {
                    class: class_id,
                    action: MatchAction::Node(egraph.get_canonical_node(*left)),
                });
            } else if is_const_zero(egraph, *left) {
                matches.push(Match {
                    class: class_id,
                    action: MatchAction::Node(egraph.get_canonical_node(*right)),
                });
            }
            // Commutativity: a + b => b + a
            matches.push(Match {
                class: class_id,
                action: MatchAction::Node(ENode::BinOp {
                    op: MirBinOp::Add,
                    left: *right,
                    right: *left,
                }),
            });
        }

        // x - 0 => x, x - x => 0
        ENode::BinOp {
            op: MirBinOp::Sub,
            left,
            right,
        } => {
            if is_const_zero(egraph, *right) {
                matches.push(Match {
                    class: class_id,
                    action: MatchAction::Node(egraph.get_canonical_node(*left)),
                });
            }
            if egraph.find(*left) == egraph.find(*right) {
                matches.push(Match {
                    class: class_id,
                    action: MatchAction::Node(ENode::ConstInt(0)),
                });
            }
        }

        // x * 1 => x, x * 0 => 0
        ENode::BinOp {
            op: MirBinOp::Mul,
            left,
            right,
        } => {
            if is_const_one(egraph, *right) {
                matches.push(Match {
                    class: class_id,
                    action: MatchAction::Node(egraph.get_canonical_node(*left)),
                });
            } else if is_const_one(egraph, *left) {
                matches.push(Match {
                    class: class_id,
                    action: MatchAction::Node(egraph.get_canonical_node(*right)),
                });
            }

            if is_const_zero(egraph, *right) || is_const_zero(egraph, *left) {
                matches.push(Match {
                    class: class_id,
                    action: MatchAction::Node(ENode::ConstInt(0)),
                });
            }

            // Commutativity: a * b => b * a
            matches.push(Match {
                class: class_id,
                action: MatchAction::Node(ENode::BinOp {
                    op: MirBinOp::Mul,
                    left: *right,
                    right: *left,
                }),
            });
        }

        // x / 1 => x, x / x => 1 (when non-zero)
        ENode::BinOp {
            op: MirBinOp::Div,
            left,
            right,
        } => {
            if is_const_one(egraph, *right) {
                matches.push(Match {
                    class: class_id,
                    action: MatchAction::Node(egraph.get_canonical_node(*left)),
                });
            }
            if egraph.find(*left) == egraph.find(*right) && !is_const_zero(egraph, *right) {
                matches.push(Match {
                    class: class_id,
                    action: MatchAction::Node(ENode::ConstInt(1)),
                });
            }
        }

        _ => {}
    }
}

/// Constant folding over e-nodes with concrete constant children.
fn match_constant_folding(
    egraph: &EGraph,
    class_id: EClassId,
    node: &ENode,
    matches: &mut Vec<Match>,
) {
    if let ENode::BinOp { op, left, right } = node {
        if let (Some(l), Some(r)) = (get_const_int(egraph, *left), get_const_int(egraph, *right)) {
            let res = match op {
                MirBinOp::Add => l.checked_add(r),
                MirBinOp::Sub => l.checked_sub(r),
                MirBinOp::Mul => l.checked_mul(r),
                MirBinOp::Div => {
                    if r != 0 {
                        l.checked_div(r)
                    } else {
                        None
                    }
                }
                MirBinOp::Mod => {
                    if r != 0 {
                        l.checked_rem(r)
                    } else {
                        None
                    }
                }
                MirBinOp::BitAnd => Some(l & r),
                MirBinOp::BitOr => Some(l | r),
                MirBinOp::BitXor => Some(l ^ r),
                MirBinOp::Shl => {
                    if (0..64).contains(&r) {
                        Some(l << r)
                    } else {
                        None
                    }
                }
                MirBinOp::Shr => {
                    if (0..64).contains(&r) {
                        Some(l >> r)
                    } else {
                        None
                    }
                }
                _ => None,
            };

            if let Some(val) = res {
                matches.push(Match {
                    class: class_id,
                    action: MatchAction::Node(ENode::ConstInt(val)),
                });
            }
        }
    }
}

/// Bitwise simplification and shift coalescing rules.
fn match_bitwise_rules(
    egraph: &EGraph,
    class_id: EClassId,
    node: &ENode,
    matches: &mut Vec<Match>,
) {
    match node {
        // x & x => x, x & 0 => 0
        ENode::BinOp {
            op: MirBinOp::BitAnd,
            left,
            right,
        } => {
            if egraph.find(*left) == egraph.find(*right) {
                matches.push(Match {
                    class: class_id,
                    action: MatchAction::Node(egraph.get_canonical_node(*left)),
                });
            }
            if is_const_zero(egraph, *right) || is_const_zero(egraph, *left) {
                matches.push(Match {
                    class: class_id,
                    action: MatchAction::Node(ENode::ConstInt(0)),
                });
            }
        }

        // x | x => x, x | 0 => x
        ENode::BinOp {
            op: MirBinOp::BitOr,
            left,
            right,
        } => {
            if egraph.find(*left) == egraph.find(*right) {
                matches.push(Match {
                    class: class_id,
                    action: MatchAction::Node(egraph.get_canonical_node(*left)),
                });
            }
            if is_const_zero(egraph, *right) {
                matches.push(Match {
                    class: class_id,
                    action: MatchAction::Node(egraph.get_canonical_node(*left)),
                });
            } else if is_const_zero(egraph, *left) {
                matches.push(Match {
                    class: class_id,
                    action: MatchAction::Node(egraph.get_canonical_node(*right)),
                });
            }
        }

        // x ^ x => 0, x ^ 0 => x
        ENode::BinOp {
            op: MirBinOp::BitXor,
            left,
            right,
        } => {
            if egraph.find(*left) == egraph.find(*right) {
                matches.push(Match {
                    class: class_id,
                    action: MatchAction::Node(ENode::ConstInt(0)),
                });
            }
            if is_const_zero(egraph, *right) {
                matches.push(Match {
                    class: class_id,
                    action: MatchAction::Node(egraph.get_canonical_node(*left)),
                });
            }
        }

        // Shift Coalescing: (x << c1) << c2 => x << (c1 + c2)
        ENode::BinOp {
            op: MirBinOp::Shl,
            left,
            right: c2_id,
        } => {
            if let Some(c2) = get_const_int(egraph, *c2_id) {
                for inner_node in egraph.get_class_nodes(*left) {
                    if let ENode::BinOp {
                        op: MirBinOp::Shl,
                        left: x,
                        right: c1_id,
                    } = inner_node
                    {
                        if let Some(c1) = get_const_int(egraph, c1_id) {
                            if c1 + c2 < 64 {
                                matches.push(Match {
                                    class: class_id,
                                    action: MatchAction::ShiftCoalesce {
                                        x,
                                        total_shift: c1 + c2,
                                    },
                                });
                            }
                        }
                    }
                }
            }
        }

        _ => {}
    }
}

/// Square-Zero Algebraic Nilpotent Variable Cancellation (S = C[z1, ..., zr]/(zi^2)).
fn match_square_zero_nilpotent(
    _egraph: &EGraph,
    class_id: EClassId,
    node: &ENode,
    matches: &mut Vec<Match>,
) {
    if let ENode::NilpotentTerm { var: _, degree } = node {
        if *degree >= 2 {
            // zi^2 = 0 in S
            matches.push(Match {
                class: class_id,
                action: MatchAction::Node(ENode::ConstInt(0)),
            });
        }
    }
}

/// Tensor Contraction and Kernel Fusion Rules.
fn match_tensor_fusions(
    egraph: &EGraph,
    class_id: EClassId,
    node: &ENode,
    matches: &mut Vec<Match>,
) {
    match node {
        // MatMul(A, B) + C => FusedMatmulAdd(A, B, C)
        ENode::BinOp {
            op: MirBinOp::Add,
            left,
            right,
        } => {
            // Check if left is TensorMatMul
            for left_node in egraph.get_class_nodes(*left) {
                if let ENode::TensorMatMul {
                    a,
                    b,
                    trans_a,
                    trans_b,
                } = left_node
                {
                    matches.push(Match {
                        class: class_id,
                        action: MatchAction::Node(ENode::FusedMatmulAdd {
                            a,
                            b,
                            bias: *right,
                            trans_a,
                            trans_b,
                        }),
                    });
                }
            }

            // Check if right is TensorMatMul (by commutativity)
            for right_node in egraph.get_class_nodes(*right) {
                if let ENode::TensorMatMul {
                    a,
                    b,
                    trans_a,
                    trans_b,
                } = right_node
                {
                    matches.push(Match {
                        class: class_id,
                        action: MatchAction::Node(ENode::FusedMatmulAdd {
                            a,
                            b,
                            bias: *left,
                            trans_a,
                            trans_b,
                        }),
                    });
                }
            }
        }

        // Relu(Conv2D(X, W) + B) => FusedConv2dRelu(X, W, Some(B))
        ENode::Call { callee, args } if callee == "relu" || callee == "Tensor.relu" => {
            if let Some(arg_id) = args.first() {
                for inner in egraph.get_class_nodes(*arg_id) {
                    if let ENode::BinOp {
                        op: MirBinOp::Add,
                        left,
                        right,
                    } = inner
                    {
                        for conv_node in egraph.get_class_nodes(left) {
                            if let ENode::TensorConv2d {
                                input,
                                kernel,
                                stride,
                                padding,
                            } = conv_node
                            {
                                matches.push(Match {
                                    class: class_id,
                                    action: MatchAction::Node(ENode::FusedConv2dRelu {
                                        input,
                                        kernel,
                                        bias: Some(right),
                                        stride,
                                        padding,
                                    }),
                                });
                            }
                        }
                    } else if let ENode::TensorConv2d {
                        input,
                        kernel,
                        stride,
                        padding,
                    } = inner
                    {
                        matches.push(Match {
                            class: class_id,
                            action: MatchAction::Node(ENode::FusedConv2dRelu {
                                input,
                                kernel,
                                bias: None,
                                stride,
                                padding,
                            }),
                        });
                    }
                }
            }
        }

        _ => {}
    }
}

// ── Helper Utilities ──

fn is_const_zero(egraph: &EGraph, id: EClassId) -> bool {
    get_const_int(egraph, id) == Some(0) || get_const_float(egraph, id) == Some(0.0)
}

fn is_const_one(egraph: &EGraph, id: EClassId) -> bool {
    get_const_int(egraph, id) == Some(1) || get_const_float(egraph, id) == Some(1.0)
}

fn get_const_int(egraph: &EGraph, id: EClassId) -> Option<i64> {
    for node in egraph.get_class_nodes(id) {
        if let ENode::ConstInt(val) = node {
            return Some(val);
        }
    }
    None
}

fn get_const_float(egraph: &EGraph, id: EClassId) -> Option<f64> {
    for node in egraph.get_class_nodes(id) {
        if let ENode::ConstFloat(bits) = node {
            return Some(f64::from_bits(bits));
        }
    }
    None
}
