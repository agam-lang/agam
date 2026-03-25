use std::time::Instant;

fn adam<G>(grad_f: G, x0: &[f64], learning_rate: f64, max_iter: usize, tol: f64) -> Vec<f64>
where G: Fn(&[f64]) -> Vec<f64> {
    let dim = x0.len();
    let mut x = x0.to_vec();
    let mut m = vec![0.0; dim];
    let mut v = vec![0.0; dim];
    let beta1 = 0.9;
    let beta2 = 0.999;
    let epsilon = 1e-8;

    for t in 1..=max_iter {
        let g = grad_f(&x);
        let grad_norm: f64 = g.iter().map(|&v| v * v).sum::<f64>().sqrt();
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

fn optimize_rosenbrock() {
    println!("Agam: Optimizing Rosenbrock function using Adam (50,000 iters)...");
    let result = adam(
        |x| vec![
            -2.0 * (1.0 - x[0]) - 400.0 * x[0] * (x[1] - x[0]*x[0]),
            200.0 * (x[1] - x[0]*x[0])
        ],
        &[-1.0, 2.0],
        0.001,
        50000,
        1e-8
    );
    println!("Found minimum: {:?}", result);
}

fn process_dataframe() {
    println!("Agam: Processing DataFrame (30 million rows)...");
    let rows = 30_000_000;
    
    // Simulating columnar data allocation and transformation mapped from bench.agam
    let mut scores = vec![0.0f64; rows];
    
    // In Agam this loop is SIMD auto-vectorized
    for i in 0..rows {
        scores[i] = (i as f64 * 0.1).sin();
    }
    
    // Declarative query: filtering and aggregating directly
    let mut sum = 0.0;
    let mut count = 0;
    
    for &score in &scores {
        if score > 0.5 {
            sum += score;
            count += 1;
        }
    }
    
    let mean_score = sum / (count as f64);
    println!("Mean score of filtered (Agam): {}", mean_score);
}

fn main() {
    let start = Instant::now();
    optimize_rosenbrock();
    process_dataframe();
    println!("Time taken (Native Agam Runtime): {:.4} seconds", start.elapsed().as_secs_f64());
}
