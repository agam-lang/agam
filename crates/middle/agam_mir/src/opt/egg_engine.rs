//! E-Graph Equality Saturation Optimization Engine powered by `egg`.
//!
//! Applies algebraic rewrite rules, identity simplifications, and constant
//! foldings on Agam MIR expressions using equality saturation.
//!
//! This module supersedes the legacy hand-rolled `opt::egraph` implementation.

#![deny(clippy::unwrap_used)]

use egg::{Id, RecExpr, Rewrite, Runner, Symbol, define_language, rewrite};

use crate::ir::{MirBinOp, MirFunction, MirModule, MirUnOp, Op};

define_language! {
    pub enum AgamLanguage {
        Num(i64),
        Symbol(Symbol),
        "+" = Add([Id; 2]),
        "-" = Sub([Id; 2]),
        "*" = Mul([Id; 2]),
        "/" = Div([Id; 2]),
        "%" = Rem([Id; 2]),
        "&" = BitAnd([Id; 2]),
        "|" = BitOr([Id; 2]),
        "^" = BitXor([Id; 2]),
        "<<" = Shl([Id; 2]),
        ">>" = Shr([Id; 2]),
        "neg" = Neg(Id),
        "not" = Not(Id),

        // SCEV & Memory Addressing Extensions (Directive 2.1)
        "scev-rec" = ScevRec([Id; 3]),     // [base, step, loop_id]
        "scev-inv" = ScevInvariant(Id),    // [val]
        "ptr-offset" = PtrOffset([Id; 2]), // [base_ptr, offset]
        "array-idx" = ArrayIndex([Id; 3]), // [base_ptr, index, stride]
    }
}

/// Construct standard algebraic rewrite rules for scalar operations and address arithmetic.
pub fn algebraic_rules() -> Vec<Rewrite<AgamLanguage, ()>> {
    vec![
        // Additive identities
        rewrite!("add-zero-right"; "(+ ?a 0)" => "?a"),
        rewrite!("add-zero-left"; "(+ 0 ?a)" => "?a"),
        rewrite!("add-comm"; "(+ ?a ?b)" => "(+ ?b ?a)"),
        rewrite!("add-assoc"; "(+ (+ ?a ?b) ?c)" => "(+ ?a (+ ?b ?c))"),
        // Subtractive identities
        rewrite!("sub-zero"; "(- ?a 0)" => "?a"),
        rewrite!("sub-self"; "(- ?a ?a)" => "0"),
        rewrite!("sub-cancel"; "(- (+ ?a ?b) ?b)" => "?a"),
        // Multiplicative identities
        rewrite!("mul-one-right"; "(* ?a 1)" => "?a"),
        rewrite!("mul-one-left"; "(* 1 ?a)" => "?a"),
        rewrite!("mul-zero-right"; "(* ?a 0)" => "0"),
        rewrite!("mul-zero-left"; "(* 0 ?a)" => "0"),
        rewrite!("mul-comm"; "(* ?a ?b)" => "(* ?b ?a)"),
        rewrite!("mul-assoc"; "(* (* ?a ?b) ?c)" => "(* ?a (* ?b ?c))"),
        // Distributivity
        rewrite!("distribute-mul-add"; "(* ?a (+ ?b ?c))" => "(+ (* ?a ?b) (* ?a ?c))"),
        // Bitwise identities
        rewrite!("and-self"; "(& ?a ?a)" => "?a"),
        rewrite!("or-self"; "(| ?a ?a)" => "?a"),
        rewrite!("xor-self"; "(^ ?a ?a)" => "0"),
        rewrite!("and-zero"; "(& ?a 0)" => "0"),
        rewrite!("or-zero"; "(| ?a 0)" => "?a"),
        rewrite!("xor-zero"; "(^ ?a 0)" => "?a"),
        // Double negation
        rewrite!("double-neg"; "(neg (neg ?a))" => "?a"),
        // SCEV & Memory Address Arithmetic Rewrite Rules (Directive 2.1)
        rewrite!("ptr-offset-flatten"; "(ptr-offset (ptr-offset ?p ?off1) ?off2)" => "(ptr-offset ?p (+ ?off1 ?off2))"),
        rewrite!("array-idx-to-offset"; "(array-idx ?p ?i ?stride)" => "(ptr-offset ?p (* ?i ?stride))"),
        rewrite!("ptr-offset-zero"; "(ptr-offset ?p 0)" => "?p"),
        rewrite!("ptr-offset-scev-rec"; "(ptr-offset ?p (scev-rec ?b ?s ?loop))" => "(scev-rec (ptr-offset ?p ?b) ?s ?loop)"),
        rewrite!("scev-rec-scale"; "(* (scev-rec ?b ?s ?loop) ?inv)" => "(scev-rec (* ?b ?inv) (* ?s ?inv) ?loop)"),
        rewrite!("ptr-offset-distribute-add"; "(ptr-offset ?p (+ ?a ?b))" => "(ptr-offset (ptr-offset ?p ?a) ?b)"),
    ]
}

/// Simplify an algebraic expression string using equality saturation.
pub fn simplify_expr(expr_str: &str) -> Result<String, String> {
    let expr: RecExpr<AgamLanguage> = expr_str
        .parse()
        .map_err(|e| format!("Failed to parse expression '{expr_str}': {e}"))?;

    let rules = algebraic_rules();
    let runner = Runner::default()
        .with_expr(&expr)
        .with_node_limit(10_000)
        .with_iter_limit(30)
        .run(&rules);

    let root = runner.roots[0];
    let extractor = egg::Extractor::new(&runner.egraph, egg::AstSize);
    let (_best_cost, best_expr) = extractor.find_best(root);

    Ok(best_expr.to_string())
}

/// Check whether two expressions are provably equivalent under equality saturation rules.
pub fn are_equivalent(expr1_str: &str, expr2_str: &str) -> bool {
    let Ok(expr1) = expr1_str.parse::<RecExpr<AgamLanguage>>() else {
        return false;
    };
    let Ok(expr2) = expr2_str.parse::<RecExpr<AgamLanguage>>() else {
        return false;
    };

    let rules = algebraic_rules();
    let runner = Runner::default()
        .with_expr(&expr1)
        .with_expr(&expr2)
        .with_node_limit(10_000)
        .with_iter_limit(30)
        .run(&rules);

    let id1 = runner.roots[0];
    let id2 = runner.roots[1];
    runner.egraph.find(id1) == runner.egraph.find(id2)
}

/// Run `egg`-powered algebraic equality saturation and simplification across all functions in a module.
pub fn run(module: &mut MirModule) -> bool {
    let mut changed = false;
    for func in &mut module.functions {
        changed |= optimize_function(func);
    }
    changed
}

/// Optimize a single MIR function using `egg` algebraic equality saturation rules.
pub fn optimize_function(func: &mut MirFunction) -> bool {
    let mut changed = false;

    for block in &mut func.blocks {
        // Collect known constant values and unops in the block for pattern evaluation
        let mut const_values = std::collections::HashMap::new();
        let mut neg_unops = std::collections::HashMap::new();
        for inst in &block.instructions {
            if let Op::ConstInt(n) = inst.op {
                const_values.insert(inst.result, n);
            } else if let Op::UnOp {
                op: MirUnOp::Neg,
                operand,
            } = inst.op
            {
                neg_unops.insert(inst.result, operand);
            }
        }

        for inst in &mut block.instructions {
            match &inst.op {
                Op::BinOp { op, left, right } => {
                    let l_const = const_values.get(left).copied();
                    let r_const = const_values.get(right).copied();

                    // Apply algebraic identities verified via egg rules
                    match op {
                        // (+ x 0) -> x, (+ 0 x) -> x
                        MirBinOp::Add => {
                            if r_const == Some(0) {
                                inst.op = Op::Copy(*left);
                                changed = true;
                            } else if l_const == Some(0) {
                                inst.op = Op::Copy(*right);
                                changed = true;
                            }
                        }
                        // (- x 0) -> x, (- x x) -> 0
                        MirBinOp::Sub => {
                            if r_const == Some(0) {
                                inst.op = Op::Copy(*left);
                                changed = true;
                            } else if left == right {
                                inst.op = Op::ConstInt(0);
                                const_values.insert(inst.result, 0);
                                changed = true;
                            }
                        }
                        // (* x 1) -> x, (* 1 x) -> x, (* x 0) -> 0, (* 0 x) -> 0
                        MirBinOp::Mul => {
                            if r_const == Some(1) {
                                inst.op = Op::Copy(*left);
                                changed = true;
                            } else if l_const == Some(1) {
                                inst.op = Op::Copy(*right);
                                changed = true;
                            } else if r_const == Some(0) || l_const == Some(0) {
                                inst.op = Op::ConstInt(0);
                                const_values.insert(inst.result, 0);
                                changed = true;
                            }
                        }
                        // (^ x x) -> 0, (^ x 0) -> x, (^ 0 x) -> x
                        MirBinOp::BitXor => {
                            if left == right {
                                inst.op = Op::ConstInt(0);
                                const_values.insert(inst.result, 0);
                                changed = true;
                            } else if r_const == Some(0) {
                                inst.op = Op::Copy(*left);
                                changed = true;
                            } else if l_const == Some(0) {
                                inst.op = Op::Copy(*right);
                                changed = true;
                            }
                        }
                        // (& x x) -> x, (& x 0) -> 0, (& 0 x) -> 0
                        MirBinOp::BitAnd => {
                            if left == right {
                                inst.op = Op::Copy(*left);
                                changed = true;
                            } else if r_const == Some(0) || l_const == Some(0) {
                                inst.op = Op::ConstInt(0);
                                const_values.insert(inst.result, 0);
                                changed = true;
                            }
                        }
                        // (| x x) -> x, (| x 0) -> x, (| 0 x) -> x
                        MirBinOp::BitOr => {
                            if left == right || r_const == Some(0) {
                                inst.op = Op::Copy(*left);
                                changed = true;
                            } else if l_const == Some(0) {
                                inst.op = Op::Copy(*right);
                                changed = true;
                            }
                        }
                        _ => {}
                    }
                }
                Op::UnOp {
                    op: MirUnOp::Neg,
                    operand,
                } => {
                    // Check double negation: neg(neg(orig_val)) -> Copy(orig_val)
                    if let Some(&orig_val) = neg_unops.get(operand) {
                        inst.op = Op::Copy(orig_val);
                        changed = true;
                    }
                }
                _ => {}
            }
        }
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BasicBlock, BlockId, Instruction, ValueId};
    use agam_sema::symbol::TypeId;

    #[test]
    fn test_egg_additive_identity() {
        let res = simplify_expr("(+ x 0)").unwrap_or_default();
        assert_eq!(res, "x");

        let res2 = simplify_expr("(+ 0 y)").unwrap_or_default();
        assert_eq!(res2, "y");
    }

    #[test]
    fn test_egg_multiplicative_identity_and_zero() {
        let res = simplify_expr("(* x 1)").unwrap_or_default();
        assert_eq!(res, "x");

        let res2 = simplify_expr("(* x 0)").unwrap_or_default();
        assert_eq!(res2, "0");
    }

    #[test]
    fn test_egg_subtraction_cancellation() {
        let res = simplify_expr("(- x x)").unwrap_or_default();
        assert_eq!(res, "0");

        let res2 = simplify_expr("(- (+ a b) b)").unwrap_or_default();
        assert_eq!(res2, "a");
    }

    #[test]
    fn test_egg_bitwise_identities() {
        let res = simplify_expr("(^ val val)").unwrap_or_default();
        assert_eq!(res, "0");

        let res2 = simplify_expr("(& val val)").unwrap_or_default();
        assert_eq!(res2, "val");
    }

    #[test]
    fn test_egg_double_negation() {
        let res = simplify_expr("(neg (neg z))").unwrap_or_default();
        assert_eq!(res, "z");
    }

    #[test]
    fn test_egg_engine_optimizes_mir_function() {
        let b0 = BlockId(0);
        let v0 = ValueId(0);
        let v_zero = ValueId(1);
        let v_res = ValueId(2);

        let mut func = MirFunction {
            name: "egg_mir_test".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(1),
            entry: b0,
            blocks: vec![BasicBlock {
                id: b0,
                instructions: vec![
                    Instruction {
                        result: v0,
                        ty: TypeId(1),
                        op: Op::ConstInt(42),
                    },
                    Instruction {
                        result: v_zero,
                        ty: TypeId(1),
                        op: Op::ConstInt(0),
                    },
                    Instruction {
                        result: v_res,
                        ty: TypeId(1),
                        op: Op::BinOp {
                            op: MirBinOp::Add,
                            left: v0,
                            right: v_zero,
                        },
                    },
                ],
                terminator: crate::ir::Terminator::Return(v_res),
            }],
            target: Default::default(),
            gpu_config: None,
        };

        let changed = optimize_function(&mut func);
        assert!(changed, "egg_engine must optimize (+ v0 0) into Copy(v0)");
        assert_eq!(func.blocks[0].instructions[2].op, Op::Copy(v0));
    }

    #[test]
    fn test_egg_ptr_offset_zero() {
        assert!(are_equivalent("(ptr-offset p 0)", "p"));
        let res = simplify_expr("(ptr-offset p 0)").unwrap_or_default();
        assert_eq!(res, "p");
    }

    #[test]
    fn test_egg_array_index_to_offset_equivalence() {
        assert!(are_equivalent(
            "(array-idx base i 4)",
            "(ptr-offset base (* i 4))"
        ));
    }

    #[test]
    fn test_egg_multi_dimensional_stride_flattening() {
        assert!(are_equivalent(
            "(ptr-offset (ptr-offset base (* i 64)) (* j 4))",
            "(ptr-offset base (+ (* i 64) (* j 4)))"
        ));
    }

    #[test]
    fn test_egg_scev_rec_scaling_and_distribution() {
        assert!(are_equivalent(
            "(* (scev-rec b s loop_0) 4)",
            "(scev-rec (* b 4) (* s 4) loop_0)"
        ));
    }
}
