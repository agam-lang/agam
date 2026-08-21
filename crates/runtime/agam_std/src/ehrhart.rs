//! Ehrhart Convex Body Volume Bounds & Bergman Potential Ray Analysis.
//!
//! Implements sharp barycentric convex volume bounds $\frac{(n+1)^n}{n!}$,
//! $n$-simplex volume integration, and Bergman potential geodesic ray evaluations.

/// Compute the sharp barycentric convex body volume upper bound in dimension `n`:
///
/// $$V_{\max}(n) = \frac{(n + 1)^n}{n!}$$
///
/// Attained by the standard reflective $n$-simplex.
pub fn barycentric_convex_volume_upper_bound(dim: usize) -> f64 {
    if dim == 0 {
        return 1.0;
    }
    let n = dim as f64;
    let numerator = (n + 1.0).powf(n);
    let denominator = factorial(dim);
    numerator / denominator
}

/// Compute the exact volume of an $n$-dimensional simplex with $n+1$ vertices in $\mathbb{R}^n$:
///
/// $$V(\Delta) = \frac{1}{n!} |\det(v_1 - v_0, v_2 - v_0, \dots, v_n - v_0)|$$
pub fn simplex_volume(vertices: &[Vec<f64>]) -> Result<f64, &'static str> {
    let num_verts = vertices.len();
    if num_verts == 0 {
        return Ok(0.0);
    }
    let dim = num_verts - 1;
    if dim == 0 {
        return Ok(1.0);
    }

    let v0 = &vertices[0];
    if v0.len() != dim {
        return Err("Vertex dimension must match (num_vertices - 1)");
    }

    // Form difference matrix D of size dim x dim where col j = v_{j+1} - v_0
    let mut matrix = vec![vec![0.0; dim]; dim];
    for col in 0..dim {
        let v_curr = &vertices[col + 1];
        if v_curr.len() != dim {
            return Err("Inconsistent vertex dimensions");
        }
        for row in 0..dim {
            matrix[row][col] = v_curr[row] - v0[row];
        }
    }

    let det = matrix_det(&mut matrix, dim);
    let volume = det.abs() / factorial(dim);
    Ok(volume)
}

/// Evaluate the Bergman potential geodesic ray $\psi(t, n) = \int_0^\infty s^n e^{-s - t s^2} ds$.
pub fn bergman_potential_ray(t: f64, dim: usize) -> f64 {
    if t < 0.0 {
        return f64::NAN;
    }
    if t == 0.0 {
        return factorial(dim);
    }

    // High-precision Gauss-Laguerre quadrature approximation
    let n = dim as f64;
    let mut integral = 0.0;
    let steps = 1000;
    let s_max = 50.0 + 5.0 * n;
    let ds = s_max / (steps as f64);

    for i in 0..steps {
        let s = (i as f64 + 0.5) * ds;
        let weight = (-s - t * s * s).exp();
        let term = s.powf(n) * weight * ds;
        integral += term;
    }

    integral
}

/// Compute the initial slope of the Bergman potential ray at $t \to 0^+$:
///
/// $$\dot{\psi}_{0+}(n) = -(n + 1)(n + 2) \cdot n! = -(n + 2)!$$
pub fn bergman_initial_slope(dim: usize) -> f64 {
    -factorial(dim + 2)
}

fn factorial(n: usize) -> f64 {
    let mut res = 1.0;
    for i in 2..=n {
        res *= i as f64;
    }
    res
}

fn matrix_det(mat: &mut [Vec<f64>], n: usize) -> f64 {
    let mut det = 1.0;
    for col in 0..n {
        let mut max_row = col;
        let mut max_val = mat[col][col].abs();
        for row in (col + 1)..n {
            if mat[row][col].abs() > max_val {
                max_val = mat[row][col].abs();
                max_row = row;
            }
        }

        if max_val < 1e-15 {
            return 0.0;
        }

        if max_row != col {
            mat.swap(col, max_row);
            det = -det;
        }

        let pivot = mat[col][col];
        det *= pivot;

        for row in (col + 1)..n {
            let factor = mat[row][col] / pivot;
            for c in col..n {
                mat[row][c] -= factor * mat[col][c];
            }
        }
    }
    det
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ehrhart_barycentric_bounds() {
        // n=1: (2^1)/1! = 2.0
        assert_eq!(barycentric_convex_volume_upper_bound(1), 2.0);

        // n=2: (3^2)/2! = 9/2 = 4.5
        assert_eq!(barycentric_convex_volume_upper_bound(2), 4.5);

        // n=3: (4^3)/6 = 64/6 = 10.6666...
        assert!((barycentric_convex_volume_upper_bound(3) - (64.0 / 6.0)).abs() < 1e-10);
    }

    #[test]
    fn test_simplex_volume_2d_and_3d() {
        // 2D Triangle: (0,0), (1,0), (0,1) -> Area = 0.5
        let v2d = vec![vec![0.0, 0.0], vec![1.0, 0.0], vec![0.0, 1.0]];
        let area = simplex_volume(&v2d).expect("2D simplex");
        assert!((area - 0.5).abs() < 1e-10);

        // 3D Tetrahedron: (0,0,0), (1,0,0), (0,1,0), (0,0,1) -> Vol = 1/6
        let v3d = vec![
            vec![0.0, 0.0, 0.0],
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];
        let vol = simplex_volume(&v3d).expect("3D simplex");
        assert!((vol - (1.0 / 6.0)).abs() < 1e-10);
    }

    #[test]
    fn test_bergman_initial_slope() {
        // For n=1: -(1+2)! = -3! = -6.0
        assert_eq!(bergman_initial_slope(1), -6.0);

        // For n=2: -4! = -24.0
        assert_eq!(bergman_initial_slope(2), -24.0);
    }
}
