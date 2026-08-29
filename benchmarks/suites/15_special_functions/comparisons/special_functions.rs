use std::time::Instant;

const PI: f64 = std::f64::consts::PI;

struct FourierResult {
    energy: f64,
    total_re: f64,
    total_im: f64,
}

fn fourier_series_benchmark(num_harmonics: i32, num_samples: usize, num_evals: usize) -> FourierResult {
    let dx = 2.0 * PI / (num_samples as f64);
    let num_coeffs = (2 * num_harmonics + 1) as usize;
    let mut cn_re = vec![0.0; num_coeffs];
    let mut cn_im = vec![0.0; num_coeffs];

    for n in -num_harmonics..=num_harmonics {
        let idx = (n + num_harmonics) as usize;
        let mut sum_re = 0.0;
        let mut sum_im = 0.0;
        for k in 0..num_samples {
            let x = (k as f64) * dx;
            let fx = x * (2.0 * PI - x) * (x - PI);
            let angle = (n as f64) * x;
            let cos_a = angle.cos();
            let sin_a = angle.sin();
            sum_re += fx * cos_a;
            sum_im += -fx * sin_a;
        }
        cn_re[idx] = sum_re / (num_samples as f64);
        cn_im[idx] = sum_im / (num_samples as f64);
    }

    let d_eval = 2.0 * PI / (num_evals as f64);
    let mut total_re = 0.0;
    let mut total_im = 0.0;
    for e in 0..num_evals {
        let x_ev = (e as f64) * d_eval;
        let mut re_recon = 0.0;
        let mut im_recon = 0.0;
        for n in -num_harmonics..=num_harmonics {
            let idx = (n + num_harmonics) as usize;
            let cre = cn_re[idx];
            let cim = cn_im[idx];
            let angle = (n as f64) * x_ev;
            let e_re = angle.cos();
            let e_im = angle.sin();
            re_recon += cre * e_re - cim * e_im;
            im_recon += cre * e_im + cim * e_re;
        }
        total_re += re_recon;
        total_im += im_recon;
    }

    let mut energy = 0.0;
    for i in 0..num_coeffs {
        energy += cn_re[i] * cn_re[i] + cn_im[i] * cn_im[i];
    }

    FourierResult { energy, total_re, total_im }
}

struct HyperResult {
    sum_re: f64,
    sum_im: f64,
    total_terms: i64,
}

fn hypergeometric_2f1_benchmark(grid_size: i32, max_terms: i32, tol: f64) -> HyperResult {
    let a = 0.5;
    let b = 1.0;
    let c = 2.0;
    let mut total_re = 0.0;
    let mut total_im = 0.0;
    let mut total_terms = 0;
    let tol2 = tol * tol;

    for ix in 0..grid_size {
        let zr = ((ix as f64) - 25.0) * 0.015;
        for iy in 0..grid_size {
            let zi = ((iy as f64) - 25.0) * 0.015;

            let mut term_re = 1.0;
            let mut term_im = 0.0;
            let mut sum_re = 1.0;
            let mut sum_im = 0.0;

            let mut n_used = max_terms;
            for n in 1..max_terms {
                let dn = n as f64;
                let factor_scalar = ((a + dn - 1.0) * (b + dn - 1.0)) / ((c + dn - 1.0) * dn);
                let fz_re = factor_scalar * zr;
                let fz_im = factor_scalar * zi;

                let new_t_re = term_re * fz_re - term_im * fz_im;
                let new_t_im = term_re * fz_im + term_im * fz_re;
                term_re = new_t_re;
                term_im = new_t_im;

                sum_re += term_re;
                sum_im += term_im;

                let mag2 = term_re * term_re + term_im * term_im;
                if mag2 < tol2 {
                    n_used = n;
                    break;
                }
            }
            total_terms += n_used as i64;
            total_re += sum_re;
            total_im += sum_im;
        }
    }

    HyperResult { sum_re: total_re, sum_im: total_im, total_terms }
}

fn main() {
    let t0 = Instant::now();
    let f_res = fourier_series_benchmark(16, 1024, 256);
    let fourier_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let t1 = Instant::now();
    let h_res = hypergeometric_2f1_benchmark(50, 200, 1e-15);
    let hyper_ms = t1.elapsed().as_secs_f64() * 1000.0;

    println!("Fourier Series: energy={:.6}, total_re={:.6} in {:.4} ms", f_res.energy, f_res.total_re, fourier_ms);
    println!("Hypergeometric 2F1: sum_re={:.6}, sum_im={:.6}, terms={} in {:.4} ms", h_res.sum_re, h_res.sum_im, h_res.total_terms, hyper_ms);

    let chk_fourier = (f_res.energy * 100000.0) as i64 + (f_res.total_re.abs() * 1000.0) as i64;
    let chk_hyper = (h_res.sum_re * 1000.0) as i64 + (h_res.sum_im * 1000.0) as i64 + h_res.total_terms;
    let combined_checksum = (chk_fourier * 1000003) ^ chk_hyper;
    println!("COMBINED_CHECKSUM: {}", combined_checksum);
}
