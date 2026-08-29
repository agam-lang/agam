import math
import time
import sys

PI = 3.14159265358979323846

def compute_fourier_coefficients(num_harmonics=16, num_samples=1024):
    dx = 2.0 * PI / num_samples
    cn_re = [0.0] * (2 * num_harmonics + 1)
    cn_im = [0.0] * (2 * num_harmonics + 1)
    for idx, n in enumerate(range(-num_harmonics, num_harmonics + 1)):
        sum_re = 0.0
        sum_im = 0.0
        for k in range(num_samples):
            x = k * dx
            fx = x * (2.0 * PI - x) * (x - PI)
            angle = n * x
            sum_re += fx * math.cos(angle)
            sum_im += -fx * math.sin(angle)
        cn_re[idx] = sum_re / num_samples
        cn_im[idx] = sum_im / num_samples
    return cn_re, cn_im

def evaluate_fourier_1000(cn_re, cn_im, num_points=1000):
    num_harmonics = (len(cn_re) - 1) // 2
    dx = 2.0 * PI / num_points
    results = []
    for k in range(num_points):
        x = k * dx
        re_recon = 0.0
        im_recon = 0.0
        for idx, n in enumerate(range(-num_harmonics, num_harmonics + 1)):
            cre = cn_re[idx]
            cim = cn_im[idx]
            angle = n * x
            e_re = math.cos(angle)
            e_im = math.sin(angle)
            re_recon += cre * e_re - cim * e_im
            im_recon += cre * e_im + cim * e_re
        results.append((k, "fourier", x, 0.0, re_recon, im_recon, 2 * num_harmonics + 1))
    return results

def evaluate_hypergeometric_1000(num_points=1000, max_terms=200, tol=1e-15):
    a = 0.5
    b = 1.0
    c = 2.0
    tol2 = tol * tol
    results = []
    
    # 1000 distinct complex points in the unit disk: z_k = r_k * exp(i * theta_k)
    for k in range(num_points):
        r = 0.05 + 0.85 * (k / num_points)
        theta = (k * 2.399963229728653) # golden angle in radians
        zr = r * math.cos(theta)
        zi = r * math.sin(theta)

        term_re = 1.0
        term_im = 0.0
        sum_re = 1.0
        sum_im = 0.0
        n_used = max_terms

        for n in range(1, max_terms):
            factor_scalar = ((a + n - 1) * (b + n - 1)) / ((c + n - 1) * n)
            fz_re = factor_scalar * zr
            fz_im = factor_scalar * zi

            new_t_re = term_re * fz_re - term_im * fz_im
            new_t_im = term_re * fz_im + term_im * fz_re
            term_re = new_t_re
            term_im = new_t_im

            sum_re += term_re
            sum_im += term_im

            mag2 = term_re * term_re + term_im * term_im
            if mag2 < tol2:
                n_used = n
                break

        results.append((1000 + k, "hypergeometric", zr, zi, sum_re, sum_im, n_used))
    return results

def main():
    t0 = time.perf_counter()
    cn_re, cn_im = compute_fourier_coefficients(16, 1024)
    fourier_rows = evaluate_fourier_1000(cn_re, cn_im, 1000)
    hyper_rows = evaluate_hypergeometric_1000(1000, 200, 1e-15)
    t1 = time.perf_counter()
    elapsed_ms = (t1 - t0) * 1000.0

    out_path = "output_python.csv"
    if len(sys.argv) > 1:
        out_path = sys.argv[1]

    with open(out_path, "w", encoding="utf-8") as f:
        f.write("index,type,input_re,input_im,output_re,output_im,terms\n")
        for row in fourier_rows:
            f.write(f"{row[0]},{row[1]},{row[2]:.10f},{row[3]:.10f},{row[4]:.12f},{row[5]:.12f},{row[6]}\n")
        for row in hyper_rows:
            f.write(f"{row[0]},{row[1]},{row[2]:.10f},{row[3]:.10f},{row[4]:.12f},{row[5]:.12f},{row[6]}\n")

    print(f"[Python] Computed 2,000 values in {elapsed_ms:.3f} ms -> written to {out_path}")

if __name__ == "__main__":
    main()
