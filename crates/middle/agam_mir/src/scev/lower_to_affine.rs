//! Translation from SCEV Expressions to Polyhedral Affine Expressions.

use crate::ir::BlockId;
use crate::opt::polyhedral::AffineExpr;
use crate::scev::expr::ScevExpr;

/// Lower a `ScevExpr` into a polyhedral `AffineExpr` given an ordered enclosing loop nest.
///
/// `nest_chain` defines the canonical loop dimension indices:
/// - `nest_chain[0]` maps to `dim_idx = 0` (outermost loop dimension).
/// - `nest_chain[k]` maps to `dim_idx = k`.
/// - `nest_chain[N-1]` maps to `dim_idx = N - 1` (innermost loop dimension).
///
/// Returns `None` (fails closed) if the SCEV expression contains `Unknown` terms, non-constant
/// steps, non-affine products, or references a loop outside `nest_chain`.
pub fn lower_to_affine(expr: &ScevExpr, nest_chain: &[BlockId]) -> Option<AffineExpr> {
    let num_dims = nest_chain.len();
    lower_internal(expr, nest_chain, num_dims)
}

fn lower_internal(
    expr: &ScevExpr,
    nest_chain: &[BlockId],
    total_dims: usize,
) -> Option<AffineExpr> {
    match expr {
        ScevExpr::Constant(val) => {
            let coeffs = vec![0; total_dims];
            Some(AffineExpr {
                constant: *val,
                coeffs,
            })
        }
        ScevExpr::Invariant(_) => {
            // Parametric or invariant values outside the affine loop indices
            None
        }
        ScevExpr::AddRec {
            base,
            step,
            loop_id,
        } => {
            // Find positional dimension index in the enclosing loop nest
            let dim_idx = nest_chain.iter().position(|id| id == loop_id)?;

            // Invariant: Step must be an affine constant for linear induction
            let step_const = match &**step {
                ScevExpr::Constant(c) => *c,
                _ => return None,
            };

            let base_affine = lower_internal(base, nest_chain, total_dims)?;

            let mut new_coeffs = base_affine.coeffs;
            if dim_idx < new_coeffs.len() {
                new_coeffs[dim_idx] += step_const;
            } else {
                return None;
            }

            Some(AffineExpr {
                constant: base_affine.constant,
                coeffs: new_coeffs,
            })
        }
        ScevExpr::Add(terms) => {
            let mut sum_const = 0;
            let mut sum_coeffs = vec![0; total_dims];

            for term in terms {
                let term_affine = lower_internal(term, nest_chain, total_dims)?;
                sum_const += term_affine.constant;
                for (dst, &src) in sum_coeffs.iter_mut().zip(term_affine.coeffs.iter()) {
                    *dst += src;
                }
            }

            Some(AffineExpr {
                constant: sum_const,
                coeffs: sum_coeffs,
            })
        }
        ScevExpr::Mul(terms) => {
            // Only scalar-constant multiplication by an affine expression is linear
            let mut constant_factor = 1;
            let mut non_const_affine = None;

            for term in terms {
                match term {
                    ScevExpr::Constant(c) => {
                        constant_factor *= *c;
                    }
                    _ => {
                        if non_const_affine.is_some() {
                            return None; // Quadratic or non-linear term (fails closed)
                        }
                        non_const_affine = Some(lower_internal(term, nest_chain, total_dims)?);
                    }
                }
            }

            match non_const_affine {
                Some(mut affine) => {
                    affine.constant *= constant_factor;
                    for c in &mut affine.coeffs {
                        *c *= constant_factor;
                    }
                    Some(affine)
                }
                None => {
                    let coeffs = vec![0; total_dims];
                    Some(AffineExpr {
                        constant: constant_factor,
                        coeffs,
                    })
                }
            }
        }
        ScevExpr::Unknown(_) => None,
    }
}
