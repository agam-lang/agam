//! Scalar Evolution (SCEV) Chain of Recurrences (CR) Expression Representation.

use crate::ir::{BlockId, ValueId};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// A Scalar Evolution Expression in Chain of Recurrences (CR) form.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScevExpr {
    /// A 64-bit integer constant literal.
    Constant(i64),
    /// A loop-invariant SSA value defined outside the target loop scope.
    Invariant(ValueId),
    /// An affine Chain of Recurrences recurrence: `{ base, +, step }_L`.
    AddRec {
        base: Box<ScevExpr>,
        step: Box<ScevExpr>,
        loop_id: BlockId,
    },
    /// A commutative sum of multiple SCEV expressions: `a + b + c ...`.
    Add(Vec<ScevExpr>),
    /// A commutative product of multiple SCEV expressions: `a * b * c ...`.
    Mul(Vec<ScevExpr>),
    /// An expression whose evolution could not be proven affine (fails closed).
    Unknown(ValueId),
}

impl ScevExpr {
    /// Create a constant SCEV expression.
    pub fn constant(val: i64) -> Self {
        ScevExpr::Constant(val)
    }

    /// Create an invariant SSA value SCEV expression.
    pub fn invariant(val: ValueId) -> Self {
        ScevExpr::Invariant(val)
    }

    /// Create an unknown SSA value SCEV expression.
    pub fn unknown(val: ValueId) -> Self {
        ScevExpr::Unknown(val)
    }

    /// Construct an affine recurrence `{ base, +, step }_L` with simplification.
    pub fn add_rec(base: ScevExpr, step: ScevExpr, loop_id: BlockId) -> Self {
        if let ScevExpr::Constant(0) = step {
            return base;
        }
        ScevExpr::AddRec {
            base: Box::new(base),
            step: Box::new(step),
            loop_id,
        }
    }

    /// Construct an addition of two or more SCEV expressions with canonicalization.
    pub fn add(mut exprs: Vec<ScevExpr>) -> Self {
        let mut flattened = Vec::with_capacity(exprs.len());
        let mut const_sum: i64 = 0;
        let mut has_const = false;

        for e in exprs.drain(..) {
            match e {
                ScevExpr::Constant(c) => {
                    const_sum = const_sum.wrapping_add(c);
                    has_const = true;
                }
                ScevExpr::Add(inner) => {
                    for in_e in inner {
                        if let ScevExpr::Constant(c) = in_e {
                            const_sum = const_sum.wrapping_add(c);
                            has_const = true;
                        } else {
                            flattened.push(in_e);
                        }
                    }
                }
                other => flattened.push(other),
            }
        }

        if has_const && const_sum != 0 {
            flattened.push(ScevExpr::Constant(const_sum));
        } else if flattened.is_empty() {
            return ScevExpr::Constant(0);
        }

        if flattened.len() == 1 {
            return flattened.pop().unwrap();
        }

        // Canonical stable sorting
        flattened.sort_by(cmp_scev_expr);
        ScevExpr::Add(flattened)
    }

    /// Construct a multiplication of two SCEV expressions with canonicalization.
    pub fn mul(mut exprs: Vec<ScevExpr>) -> Self {
        let mut flattened = Vec::with_capacity(exprs.len());
        let mut const_prod: i64 = 1;
        let mut has_const = false;

        for e in exprs.drain(..) {
            match e {
                ScevExpr::Constant(0) => return ScevExpr::Constant(0),
                ScevExpr::Constant(c) => {
                    const_prod = const_prod.wrapping_mul(c);
                    has_const = true;
                }
                ScevExpr::Mul(inner) => {
                    for in_e in inner {
                        if let ScevExpr::Constant(c) = in_e {
                            const_prod = const_prod.wrapping_mul(c);
                            has_const = true;
                        } else {
                            flattened.push(in_e);
                        }
                    }
                }
                other => flattened.push(other),
            }
        }

        if has_const && const_prod != 1 {
            flattened.push(ScevExpr::Constant(const_prod));
        } else if flattened.is_empty() {
            return ScevExpr::Constant(const_prod);
        }

        if flattened.len() == 1 {
            return flattened.pop().unwrap();
        }

        flattened.sort_by(cmp_scev_expr);
        ScevExpr::Mul(flattened)
    }

    /// Check if this expression contains any `Unknown` terms.
    pub fn is_affine(&self) -> bool {
        match self {
            ScevExpr::Constant(_) | ScevExpr::Invariant(_) => true,
            ScevExpr::AddRec { base, step, .. } => base.is_affine() && step.is_affine(),
            ScevExpr::Add(terms) | ScevExpr::Mul(terms) => terms.iter().all(|t| t.is_affine()),
            ScevExpr::Unknown(_) => false,
        }
    }

    /// Check if the expression is invariant with respect to a loop.
    pub fn is_loop_invariant(&self, target_loop: BlockId) -> bool {
        match self {
            ScevExpr::Constant(_) | ScevExpr::Invariant(_) => true,
            ScevExpr::AddRec { loop_id, .. } if *loop_id == target_loop => false,
            ScevExpr::AddRec { base, step, .. } => {
                base.is_loop_invariant(target_loop) && step.is_loop_invariant(target_loop)
            }
            ScevExpr::Add(terms) | ScevExpr::Mul(terms) => {
                terms.iter().all(|t| t.is_loop_invariant(target_loop))
            }
            ScevExpr::Unknown(_) => false,
        }
    }
}

fn scev_discriminant_rank(expr: &ScevExpr) -> u32 {
    match expr {
        ScevExpr::Constant(_) => 0,
        ScevExpr::Invariant(_) => 1,
        ScevExpr::AddRec { .. } => 2,
        ScevExpr::Add(_) => 3,
        ScevExpr::Mul(_) => 4,
        ScevExpr::Unknown(_) => 5,
    }
}

fn cmp_scev_expr(a: &ScevExpr, b: &ScevExpr) -> Ordering {
    let rank_a = scev_discriminant_rank(a);
    let rank_b = scev_discriminant_rank(b);
    if rank_a != rank_b {
        return rank_a.cmp(&rank_b);
    }

    match (a, b) {
        (ScevExpr::Constant(x), ScevExpr::Constant(y)) => x.cmp(y),
        (ScevExpr::Invariant(x), ScevExpr::Invariant(y)) => x.0.cmp(&y.0),
        (
            ScevExpr::AddRec {
                base: b1,
                step: s1,
                loop_id: l1,
            },
            ScevExpr::AddRec {
                base: b2,
                step: s2,
                loop_id: l2,
            },
        ) => {
            if l1.0 != l2.0 {
                l1.0.cmp(&l2.0)
            } else {
                let bc = cmp_scev_expr(b1, b2);
                if bc != Ordering::Equal {
                    bc
                } else {
                    cmp_scev_expr(s1, s2)
                }
            }
        }
        (ScevExpr::Add(v1), ScevExpr::Add(v2)) | (ScevExpr::Mul(v1), ScevExpr::Mul(v2)) => {
            if v1.len() != v2.len() {
                v1.len().cmp(&v2.len())
            } else {
                for (x, y) in v1.iter().zip(v2.iter()) {
                    let ord = cmp_scev_expr(x, y);
                    if ord != Ordering::Equal {
                        return ord;
                    }
                }
                Ordering::Equal
            }
        }
        (ScevExpr::Unknown(x), ScevExpr::Unknown(y)) => x.0.cmp(&y.0),
        _ => Ordering::Equal,
    }
}
