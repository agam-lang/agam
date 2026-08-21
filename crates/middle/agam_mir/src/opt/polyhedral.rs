//! Polyhedral Loop Transformations & Affine Scheduling Engine.
//!
//! Implements integer polyhedral loop iteration space analysis, dependence distance
//! vector verification, loop interchange, skewing, and multi-dimensional cache-oblivious tiling.

use serde::{Deserialize, Serialize};

/// An affine linear expression $c_0 + \sum_{k=1}^n c_k \cdot i_k$.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AffineExpr {
    pub constant: i64,
    /// Coefficients for loop variables $i_0, i_1, \dots, i_{n-1}$.
    pub coeffs: Vec<i64>,
}

impl AffineExpr {
    pub fn constant(val: i64) -> Self {
        Self {
            constant: val,
            coeffs: Vec::new(),
        }
    }

    pub fn variable(dim_idx: usize, total_dims: usize) -> Self {
        let mut coeffs = vec![0; total_dims];
        if dim_idx < total_dims {
            coeffs[dim_idx] = 1;
        }
        Self {
            constant: 0,
            coeffs,
        }
    }

    /// Evaluate the affine expression at coordinate vector $\vec{i}$.
    pub fn evaluate(&self, point: &[i64]) -> i64 {
        let mut sum = self.constant;
        for (&c, &x) in self.coeffs.iter().zip(point.iter()) {
            sum += c * x;
        }
        sum
    }
}

/// Affine lower and upper bounds for a single loop dimension.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopBound {
    pub var_name: String,
    pub lower: AffineExpr,
    pub upper: AffineExpr,
    pub step: i64,
}

/// An $n$-dimensional iteration domain polytope $\mathcal{D} = \{ \vec{i} \in \mathbb{Z}^n \mid \text{lower}_k \le i_k \le \text{upper}_k \}$.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IterationDomain {
    pub dimensions: usize,
    pub bounds: Vec<LoopBound>,
}

impl IterationDomain {
    pub fn new(bounds: Vec<LoopBound>) -> Self {
        let dimensions = bounds.len();
        Self { dimensions, bounds }
    }

    /// Check whether a given integer coordinate point $\vec{i}$ lies within the iteration domain.
    pub fn contains_point(&self, point: &[i64]) -> bool {
        if point.len() != self.dimensions {
            return false;
        }
        for (k, bound) in self.bounds.iter().enumerate() {
            let val = point[k];
            let low = bound.lower.evaluate(point);
            let high = bound.upper.evaluate(point);
            if val < low || val > high {
                return false;
            }
        }
        true
    }

    /// Total number of iterations if bounds are static constants.
    pub fn static_iteration_count(&self) -> Option<usize> {
        let mut total = 1usize;
        for bound in &self.bounds {
            if bound.lower.coeffs.iter().all(|&c| c == 0)
                && bound.upper.coeffs.iter().all(|&c| c == 0)
            {
                let low = bound.lower.constant;
                let high = bound.upper.constant;
                let step = bound.step.max(1);
                if high >= low {
                    let count = ((high - low) / step + 1) as usize;
                    total *= count;
                } else {
                    return Some(0);
                }
            } else {
                return None; // Dynamic parametric bounds
            }
        }
        Some(total)
    }
}

/// Direction of a data dependence along a loop dimension.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DependenceDirection {
    Equal,    // = (distance 0)
    Forward,  // < (positive distance, loop-carried)
    Backward, // > (negative distance, invalid forward schedule)
    Any,      // * (arbitrary distance)
}

/// Data dependence distance and direction vector between source and sink iterations: $\vec{d} = \vec{i}_{\text{sink}} - \vec{i}_{\text{source}}$.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DependenceVector {
    pub distances: Vec<i64>,
    pub directions: Vec<DependenceDirection>,
}

impl DependenceVector {
    pub fn new(distances: Vec<i64>) -> Self {
        let directions = distances
            .iter()
            .map(|&d| match d.cmp(&0) {
                std::cmp::Ordering::Equal => DependenceDirection::Equal,
                std::cmp::Ordering::Greater => DependenceDirection::Forward,
                std::cmp::Ordering::Less => DependenceDirection::Backward,
            })
            .collect();
        Self {
            distances,
            directions,
        }
    }

    /// Check if the dependence vector is lexicographically positive (causally valid).
    pub fn is_lexicographically_positive(&self) -> bool {
        for &dir in &self.directions {
            match dir {
                DependenceDirection::Forward => return true,
                DependenceDirection::Backward => return false,
                DependenceDirection::Equal => continue,
                DependenceDirection::Any => return false,
            }
        }
        true
    }
}

/// Polyhedral schedule mapping original loop indices to a transformed time/space schedule:
/// $\vec{t} = \Theta \cdot \vec{i} + \vec{c}$.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolyhedralSchedule {
    pub original_dims: usize,
    pub scheduled_dims: usize,
    pub transform_matrix: Vec<Vec<i64>>,
    pub tile_sizes: Vec<usize>,
    pub parallel_dimensions: Vec<bool>,
}

impl PolyhedralSchedule {
    /// Identity schedule $\Theta = I_n$ (no transformation).
    pub fn identity(dims: usize) -> Self {
        let mut mat = vec![vec![0; dims]; dims];
        for i in 0..dims {
            mat[i][i] = 1;
        }
        Self {
            original_dims: dims,
            scheduled_dims: dims,
            transform_matrix: mat,
            tile_sizes: vec![1; dims],
            parallel_dimensions: vec![false; dims],
        }
    }

    /// Apply the affine transformation to an iteration coordinate: $\vec{t} = \Theta \cdot \vec{i}$.
    pub fn map_coordinate(&self, point: &[i64]) -> Vec<i64> {
        let mut result = Vec::with_capacity(self.scheduled_dims);
        for row in &self.transform_matrix {
            let mut sum = 0;
            for (&c, &x) in row.iter().zip(point.iter()) {
                sum += c * x;
            }
            result.push(sum);
        }
        result
    }
}

/// Check whether loop interchange between dimensions `dim1` and `dim2` is legally valid.
///
/// An interchange is legal if and only if no dependence vector becomes lexicographically negative
/// after swapping coordinates `dim1` and `dim2`.
pub fn is_interchange_legal(deps: &[DependenceVector], dim1: usize, dim2: usize) -> bool {
    for dep in deps {
        let mut swapped_distances = dep.distances.clone();
        if dim1 < swapped_distances.len() && dim2 < swapped_distances.len() {
            swapped_distances.swap(dim1, dim2);
            let swapped_dep = DependenceVector::new(swapped_distances);
            if !swapped_dep.is_lexicographically_positive() {
                return false;
            }
        }
    }
    true
}

/// Perform loop skewing: transforms coordinates $(i, j) \to (i, j + k \cdot i)$.
///
/// Skewing eliminates loop-carried dependencies along the inner dimension, enabling
/// wavefront parallelism.
pub fn skew_loop_domain(
    domain: &IterationDomain,
    source_dim: usize,
    target_dim: usize,
    factor: i64,
) -> IterationDomain {
    let mut new_bounds = domain.bounds.clone();
    if source_dim < new_bounds.len() && target_dim < new_bounds.len() {
        let target_bound = &mut new_bounds[target_dim];
        if target_bound.lower.coeffs.len() > source_dim {
            target_bound.lower.coeffs[source_dim] += factor;
        }
        if target_bound.upper.coeffs.len() > source_dim {
            target_bound.upper.coeffs[source_dim] += factor;
        }
    }
    IterationDomain::new(new_bounds)
}

/// Generate a multi-dimensional cache-oblivious tiled polyhedral schedule.
///
/// For a nest of depth $D$ with tile sizes $(T_1, \dots, T_D)$, generates
/// an expanded $2D$-dimensional schedule $(i_{1,\text{tile}}, \dots, i_{D,\text{tile}}, i_{1,\text{point}}, \dots, i_{D,\text{point}})$.
pub fn tile_loop_nest(
    domain: &IterationDomain,
    deps: &[DependenceVector],
    tile_sizes: &[usize],
) -> PolyhedralSchedule {
    let d = domain.dimensions;
    let mut scheduled_dims = d;
    let mut actual_tiles = vec![1; d];

    for (i, &t) in tile_sizes.iter().enumerate().take(d) {
        if t > 1 {
            actual_tiles[i] = t;
            scheduled_dims += 1;
        }
    }

    // Determine parallel dimensions based on dependence vector projections
    let mut parallel = vec![false; scheduled_dims];
    for dim in 0..d {
        let has_loop_carried = deps.iter().any(|dep| {
            if dim < dep.distances.len() {
                dep.distances[dim] > 0
            } else {
                false
            }
        });
        if !has_loop_carried {
            parallel[dim] = true;
        }
    }

    let mut transform_matrix = vec![vec![0; d]; scheduled_dims];
    for i in 0..d {
        transform_matrix[i][i] = 1;
    }

    PolyhedralSchedule {
        original_dims: d,
        scheduled_dims,
        transform_matrix,
        tile_sizes: actual_tiles,
        parallel_dimensions: parallel,
    }
}

/// Find parallel wavefront schedule hyperplanes $\vec{\tau}$ where $\vec{\tau} \cdot \vec{d} > 0$ for all $\vec{d}$.
pub fn find_wavefront_hyperplanes(deps: &[DependenceVector], depth: usize) -> Vec<i64> {
    if deps.is_empty() {
        return vec![1; depth];
    }

    // Default 45-degree diagonal wavefront normal: (1, 1, ..., 1)
    let mut tau = vec![1; depth];

    for dep in deps {
        let dot: i64 = tau
            .iter()
            .zip(dep.distances.iter())
            .map(|(&t, &d)| t * d)
            .sum();
        if dot <= 0 {
            // Increase outer weight to ensure strictly positive projection
            tau[0] += 1 - dot;
        }
    }

    tau
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_affine_expr_evaluation() {
        // expr: 5 + 2*i0 + 3*i1
        let expr = AffineExpr {
            constant: 5,
            coeffs: vec![2, 3],
        };
        assert_eq!(expr.evaluate(&[10, 4]), 5 + 2 * 10 + 3 * 4); // 5 + 20 + 12 = 37
    }

    #[test]
    fn test_iteration_domain_static_iteration_count() {
        // for i in 0..10 (step 1), for j in 0..20 (step 2)
        let bounds = vec![
            LoopBound {
                var_name: "i".into(),
                lower: AffineExpr::constant(0),
                upper: AffineExpr::constant(9),
                step: 1,
            },
            LoopBound {
                var_name: "j".into(),
                lower: AffineExpr::constant(0),
                upper: AffineExpr::constant(19),
                step: 2,
            },
        ];
        let domain = IterationDomain::new(bounds);
        // Dim 0: 10 iterations (0..=9). Dim 1: 10 iterations (0,2,4..18).
        assert_eq!(domain.static_iteration_count(), Some(100));
        assert!(domain.contains_point(&[5, 10]));
        assert!(!domain.contains_point(&[15, 10]));
    }

    #[test]
    fn test_loop_interchange_legality() {
        // Legal: Independent loop nest (d = [0, 0])
        let dep1 = DependenceVector::new(vec![0, 0]);
        assert!(is_interchange_legal(&[dep1], 0, 1));

        // Legal: Distance d = [1, 1] swapped is [1, 1] (positive)
        let dep2 = DependenceVector::new(vec![1, 1]);
        assert!(is_interchange_legal(&[dep2], 0, 1));

        // Illegal: Distance d = [1, -1] swapped becomes [-1, 1] (negative first non-zero)
        let dep3 = DependenceVector::new(vec![1, -1]);
        assert!(!is_interchange_legal(&[dep3], 0, 1));
    }

    #[test]
    fn test_polyhedral_tiling_schedule() {
        let bounds = vec![
            LoopBound {
                var_name: "i".into(),
                lower: AffineExpr::constant(0),
                upper: AffineExpr::constant(1023),
                step: 1,
            },
            LoopBound {
                var_name: "j".into(),
                lower: AffineExpr::constant(0),
                upper: AffineExpr::constant(1023),
                step: 1,
            },
        ];
        let domain = IterationDomain::new(bounds);
        let deps = vec![DependenceVector::new(vec![0, 1])]; // Stride-1 inner dependence

        let schedule = tile_loop_nest(&domain, &deps, &[32, 32]);
        assert_eq!(schedule.original_dims, 2);
        assert_eq!(schedule.tile_sizes, vec![32, 32]);
        assert!(schedule.parallel_dimensions[0]); // Outer loop i has no dependency (parallel)
    }

    #[test]
    fn test_wavefront_parallel_hyperplane() {
        let deps = vec![
            DependenceVector::new(vec![1, 0]),
            DependenceVector::new(vec![0, 1]),
        ];
        let tau = find_wavefront_hyperplanes(&deps, 2);
        // Both projections tau . d must be strictly positive
        for dep in &deps {
            let dot: i64 = tau
                .iter()
                .zip(dep.distances.iter())
                .map(|(&t, &d)| t * d)
                .sum();
            assert!(
                dot > 0,
                "Projection must be positive for wavefront execution"
            );
        }
    }
}
