import math
import time

def fourier_series_benchmark(num_harmonics=16, num_samples=1024, num_evals=256):
    dx = 2.0 * math.pi / num_samples
    # 1. Compute Fourier coefficients c_n
    cn_re = [0.0] * (2 * num_harmonics + 1)
    cn_im = [0.0] * (2 * num_harmonics + 1)
    
    for idx, n in enumerate(range(-num_harmonics, num_harmonics + 1)):
        sum_re = 0.0
        sum_im = 0.0
        for k in range(num_samples):
            x = k * dx
            # Signal: f(x) = x * (2*pi - x) * (x - pi)
            fx = x * (2.0 * math.pi - x) * (x - math.pi)
            angle = n * x
            cos_a = math.cos(angle)
            sin_a = math.sin(angle)
            # exp(-i * n * x) = cos(nx) - i * sin(nx)
            sum_re += fx * cos_a
            sum_im += -fx * sin_a
        cn_re[idx] = sum_re / num_samples
        cn_im[idx] = sum_im / num_samples

    # 2. Synthesize at num_evals points
    d_eval = 2.0 * math.pi / num_evals
    total_re = 0.0
    total_im = 0.0
    for e in range(num_evals):
        x_ev = e * d_eval
        re_recon = 0.0
        im_recon = 0.0
        for idx, n in enumerate(range(-num_harmonics, num_harmonics + 1)):
            cre = cn_re[idx]
            cim = cn_im[idx]
            angle = n * x_ev
            e_re = math.cos(angle)
            e_im = math.sin(angle)
            # (cre + i cim) * (e_re + i e_im)
            re_recon += cre * e_re - cim * e_im
            im_recon += cre * e_im + cim * e_re
        total_re += re_recon
        total_im += im_recon

    energy = 0.0
    for idx in range(len(cn_re)):
        energy += cn_re[idx] * cn_re[idx] + cn_im[idx] * cn_im[idx]
        
    return energy, total_re, total_im

def hypergeometric_2f1_benchmark(grid_size=50, max_terms=200, tol=1e-15):
    # Evaluate 2F1(0.5, 1.0; 2.0; z) across grid
    a = 0.5
    b = 1.0
    c = 2.0
    total_re = 0.0
    total_im = 0.0
    total_terms = 0

    for ix in range(grid_size):
        zr = (ix - 25) * 0.015 # [-0.375, 0.36]
        for iy in range(grid_size):
            zi = (iy - 25) * 0.015
            
            # Series sum
            term_re = 1.0
            term_im = 0.0
            sum_re = 1.0
            sum_im = 0.0
            
            for n in range(1, max_terms):
                factor_scalar = ((a + n - 1) * (b + n - 1)) / ((c + n - 1) * n)
                fz_re = factor_scalar * zr
                fz_im = factor_scalar * zi
                
                # term = term * fz = (term_re + i term_im) * (fz_re + i fz_im)
                new_t_re = term_re * fz_re - term_im * fz_im
                new_t_im = term_re * fz_im + term_im * fz_re
                term_re = new_t_re
                term_im = new_t_im
                
                sum_re += term_re
                sum_im += term_im
                
                mag2 = term_re * term_re + term_im * term_im
                if mag2 < tol * tol:
                    total_terms += n
                    break
            else:
                total_terms += max_terms
                
            total_re += sum_re
            total_im += sum_im

    return total_re, total_im, total_terms

def main():
    # 1. Fourier Series
    t0 = time.perf_counter()
    energy, f_re, f_im = fourier_series_benchmark()
    t1 = time.perf_counter()
    fourier_ms = (t1 - t0) * 1000.0
    
    # 2. Hypergeometric Series
    t2 = time.perf_counter()
    h_re, h_im, h_terms = hypergeometric_2f1_benchmark()
    t3 = time.perf_counter()
    hyper_ms = (t3 - t2) * 1000.0

    print(f"Fourier Series: energy={energy:.10f}, total_re={f_re:.6f}, total_im={f_im:.6f} in {fourier_ms:.3f} ms")
    print(f"Hypergeometric 2F1: sum_re={h_re:.6f}, sum_im={h_im:.6f}, terms={h_terms} in {hyper_ms:.3f} ms")

    # Combined Checksum as fixed 64-bit integer
    chk_fourier = int(energy * 100000.0) + int(abs(f_re) * 1000.0)
    chk_hyper = int(h_re * 1000.0) + int(h_im * 1000.0) + h_terms
    combined_checksum = (chk_fourier * 1000003) ^ chk_hyper
    print(f"COMBINED_CHECKSUM: {combined_checksum}")

if __name__ == "__main__":
    main()
