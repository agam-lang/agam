use std::fs::File;
use std::io::Write;
use std::time::Instant;

const PI: f64 = std::f64::consts::PI;

struct Row {
    index: usize,
    point_type: &'static str,
    input_re: f64,
    input_im: f64,
    output_re: f64,
    output_im: f64,
    terms: usize,
}

fn run_all(num_harmonics: i32, num_samples: usize, num_points: usize) -> Vec<Row> {
    let mut rows = Vec::with_capacity(2 * num_points);

    // 1. Fourier coefficients
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
            sum_re += fx * angle.cos();
            sum_im += -fx * angle.sin();
        }
        cn_re[idx] = sum_re / (num_samples as f64);
        cn_im[idx] = sum_im / (num_samples as f64);
    }

    // 2. Evaluate 1000 Fourier points
    let d_eval = 2.0 * PI / (num_points as f64);
    for k in 0..num_points {
        let x = (k as f64) * d_eval;
        let mut re_recon = 0.0;
        let mut im_recon = 0.0;
        for n in -num_harmonics..=num_harmonics {
            let idx = (n + num_harmonics) as usize;
            let cre = cn_re[idx];
            let cim = cn_im[idx];
            let angle = (n as f64) * x;
            let e_re = angle.cos();
            let e_im = angle.sin();
            re_recon += cre * e_re - cim * e_im;
            im_recon += cre * e_im + cim * e_re;
        }
        rows.push(Row {
            index: k,
            point_type: "fourier",
            input_re: x,
            input_im: 0.0,
            output_re: re_recon,
            output_im: im_recon,
            terms: num_coeffs,
        });
    }

    // 3. Evaluate 1000 Hypergeometric 2F1 points
    let a = 0.5;
    let b = 1.0;
    let c = 2.0;
    let max_terms = 200;
    let tol2 = 1e-30;

    for k in 0..num_points {
        let r = 0.05 + 0.85 * ((k as f64) / (num_points as f64));
        let theta = (k as f64) * 2.399963229728653;
        let zr = r * theta.cos();
        let zi = r * theta.sin();

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
        rows.push(Row {
            index: 1000 + k,
            point_type: "hypergeometric",
            input_re: zr,
            input_im: zi,
            output_re: sum_re,
            output_im: sum_im,
            terms: n_used,
        });
    }

    rows
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out_path = if args.len() > 1 { &args[1] } else { "output_rust.csv" };

    let t0 = Instant::now();
    let rows = run_all(16, 1024, 1000);
    let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let mut f = File::create(out_path).expect("cannot create file");
    writeln!(f, "index,type,input_re,input_im,output_re,output_im,terms").unwrap();
    for r in &rows {
        writeln!(
            f,
            "{},{},{:.10},{:.10},{:.12},{:.12},{}",
            r.index, r.point_type, r.input_re, r.input_im, r.output_re, r.output_im, r.terms
        ).unwrap();
    }

    println!("[Rust -O] Computed 2,000 values in {:.3} ms -> written to {}", elapsed_ms, out_path);
}
