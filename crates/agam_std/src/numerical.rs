//! Numerical methods — hardware-optimized iterative solvers.
//!
//! ## ODE Solvers
//! - RK4 (4th-order Runge-Kutta) — the workhorse of scientific computing
//!
//! ## Optimization
//! - Gradient descent with configurable learning rate
//! - Adam optimizer (adaptive moment estimation)
//!
//! All solvers operate on contiguous arrays for cache locality.

/// 4th-order Runge-Kutta ODE solver.
///
/// Solves dy/dt = f(t, y) from t0 to t_end with step size dt.
/// Returns the trajectory as (time, state) pairs.
pub fn rk4<F>(f: &F, y0: f64, t0: f64, t_end: f64, dt: f64) -> Vec<(f64, f64)>
where F: Fn(f64, f64) -> f64 {
    let n = ((t_end - t0) / dt).ceil() as usize;
    let mut trajectory = Vec::with_capacity(n + 1);
    let mut t = t0;
    let mut y = y0;

    trajectory.push((t, y));

    for _ in 0..n {
        let k1 = dt * f(t, y);
        let k2 = dt * f(t + 0.5 * dt, y + 0.5 * k1);
        let k3 = dt * f(t + 0.5 * dt, y + 0.5 * k2);
        let k4 = dt * f(t + dt, y + k3);
        y += (k1 + 2.0 * k2 + 2.0 * k3 + k4) / 6.0;
        t += dt;
        trajectory.push((t, y));
    }
    trajectory
}

/// RK4 for systems of ODEs: dy/dt = f(t, y) where y is a vector.
pub fn rk4_system<F>(f: &F, y0: &[f64], t0: f64, t_end: f64, dt: f64) -> Vec<(f64, Vec<f64>)>
where F: Fn(f64, &[f64]) -> Vec<f64> {
    let dim = y0.len();
    let n = ((t_end - t0) / dt).ceil() as usize;
    let mut trajectory = Vec::with_capacity(n + 1);
    let mut t = t0;
    let mut y = y0.to_vec();

    trajectory.push((t, y.clone()));

    for _ in 0..n {
        let k1 = f(t, &y);
        let y_tmp: Vec<f64> = (0..dim).map(|i| y[i] + 0.5 * dt * k1[i]).collect();
        let k2 = f(t + 0.5 * dt, &y_tmp);
        let y_tmp: Vec<f64> = (0..dim).map(|i| y[i] + 0.5 * dt * k2[i]).collect();
        let k3 = f(t + 0.5 * dt, &y_tmp);
        let y_tmp: Vec<f64> = (0..dim).map(|i| y[i] + dt * k3[i]).collect();
        let k4 = f(t + dt, &y_tmp);

        for i in 0..dim {
            y[i] += dt * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]) / 6.0;
        }
        t += dt;
        trajectory.push((t, y.clone()));
    }
    trajectory
}

/// Gradient descent optimizer.
///
/// Minimizes f(x) given its gradient ∇f(x).
/// Returns the optimized parameter vector.
pub fn gradient_descent<F, G>(
    grad_f: &G,
    x0: &[f64],
    learning_rate: f64,
    max_iter: usize,
    tol: f64,
) -> Vec<f64>
where F: Fn(&[f64]) -> f64, G: Fn(&[f64]) -> Vec<f64> {
    let mut x = x0.to_vec();
    for _ in 0..max_iter {
        let g = grad_f(&x);
        let grad_norm: f64 = g.iter().map(|v| v * v).sum::<f64>().sqrt();
        if grad_norm < tol { break; }
        for i in 0..x.len() {
            x[i] -= learning_rate * g[i];
        }
    }
    x
}

/// Adam optimizer — adaptive learning rate with momentum.
///
/// Best for training neural networks. Maintains per-parameter first and
/// second moment estimates.
pub fn adam<G>(
    grad_f: &G,
    x0: &[f64],
    learning_rate: f64,
    max_iter: usize,
    tol: f64,
) -> Vec<f64>
where G: Fn(&[f64]) -> Vec<f64> {
    let dim = x0.len();
    let mut x = x0.to_vec();
    let mut m = vec![0.0; dim]; // first moment
    let mut v = vec![0.0; dim]; // second moment
    let beta1 = 0.9;
    let beta2 = 0.999;
    let epsilon = 1e-8;

    for t in 1..=max_iter {
        let g = grad_f(&x);
        let grad_norm: f64 = g.iter().map(|v| v * v).sum::<f64>().sqrt();
        if grad_norm < tol { break; }

        let t_f = t as f64;
        for i in 0..dim {
            m[i] = beta1 * m[i] + (1.0 - beta1) * g[i];
            v[i] = beta2 * v[i] + (1.0 - beta2) * g[i] * g[i];
            let m_hat = m[i] / (1.0 - beta1.powf(t_f));
            let v_hat = v[i] / (1.0 - beta2.powf(t_f));
            x[i] -= learning_rate * m_hat / (v_hat.sqrt() + epsilon);
        }
    }
    x
}

/// Linear regression via least squares: y = ax + b.
/// Returns (slope a, intercept b, R²).
pub fn linear_regression(x: &[f64], y: &[f64]) -> (f64, f64, f64) {
    let n = x.len() as f64;
    let sum_x: f64 = x.iter().sum();
    let sum_y: f64 = y.iter().sum();
    let sum_xy: f64 = x.iter().zip(y).map(|(a, b)| a * b).sum();
    let sum_x2: f64 = x.iter().map(|a| a * a).sum();
    let sum_y2: f64 = y.iter().map(|a| a * a).sum();

    let a = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
    let b = (sum_y - a * sum_x) / n;

    // R² coefficient of determination
    let ss_res: f64 = x.iter().zip(y).map(|(xi, yi)| (yi - (a * xi + b)).powi(2)).sum();
    let mean_y = sum_y / n;
    let ss_tot: f64 = y.iter().map(|yi| (yi - mean_y).powi(2)).sum();
    let r_sq = 1.0 - ss_res / ss_tot;

    (a, b, r_sq)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rk4_exponential() {
        // dy/dt = y, y(0) = 1 → y = e^t
        let traj = rk4(&|_t, y| y, 1.0, 0.0, 1.0, 0.001);
        let y_final = traj.last().unwrap().1;
        assert!((y_final - std::f64::consts::E).abs() < 1e-6);
    }

    #[test]
    fn test_rk4_polynomial() {
        // dy/dt = 2t, y(0) = 0 → y = t²
        let traj = rk4(&|t, _y| 2.0 * t, 0.0, 0.0, 2.0, 0.01);
        let y_final = traj.last().unwrap().1;
        assert!((y_final - 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_rk4_system_harmonic() {
        // Simple harmonic oscillator: x'' = -x
        // → dx/dt = v, dv/dt = -x
        // y = [x, v], y0 = [1, 0] → x(t) = cos(t)
        let traj = rk4_system(
            &|_t, y| vec![y[1], -y[0]],
            &[1.0, 0.0],
            0.0, std::f64::consts::PI, 0.001,
        );
        let y_final = &traj.last().unwrap().1;
        // cos(π) = -1
        assert!((y_final[0] - (-1.0)).abs() < 1e-4);
    }

    #[test]
    fn test_gradient_descent_quadratic() {
        // Minimize f(x) = (x-3)². ∇f = 2(x-3)
        let result = gradient_descent::<fn(&[f64]) -> f64, _>(
            &|x: &[f64]| vec![2.0 * (x[0] - 3.0)],
            &[0.0],
            0.1, 1000, 1e-10,
        );
        assert!((result[0] - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_adam_quadratic() {
        // Minimize f(x) = (x-5)². ∇f = 2(x-5)
        let result = adam(
            &|x: &[f64]| vec![2.0 * (x[0] - 5.0)],
            &[0.0],
            0.1, 5000, 1e-10,
        );
        assert!((result[0] - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_adam_2d() {
        // Minimize f(x,y) = (x-1)² + (y-2)²
        let result = adam(
            &|x: &[f64]| vec![2.0 * (x[0] - 1.0), 2.0 * (x[1] - 2.0)],
            &[0.0, 0.0],
            0.1, 5000, 1e-10,
        );
        assert!((result[0] - 1.0).abs() < 0.01);
        assert!((result[1] - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_linear_regression() {
        // Perfect linear data: y = 2x + 1
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![3.0, 5.0, 7.0, 9.0, 11.0];
        let (slope, intercept, r_sq) = linear_regression(&x, &y);
        assert!((slope - 2.0).abs() < 1e-10);
        assert!((intercept - 1.0).abs() < 1e-10);
        assert!((r_sq - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_linear_regression_noisy() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.8, 5.1, 7.2, 8.9, 11.1];
        let (slope, _intercept, r_sq) = linear_regression(&x, &y);
        assert!((slope - 2.0).abs() < 0.2);
        assert!(r_sq > 0.99);
    }
}
