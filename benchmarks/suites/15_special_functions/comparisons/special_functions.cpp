#include <iostream>
#include <vector>
#include <cmath>
#include <chrono>

static const double PI = 3.14159265358979323846;

struct FourierResult {
    double energy;
    double total_re;
    double total_im;
};

FourierResult fourier_series_benchmark(int num_harmonics = 16, int num_samples = 1024, int num_evals = 256) {
    double dx = 2.0 * PI / (double)num_samples;
    int num_coeffs = 2 * num_harmonics + 1;
    std::vector<double> cn_re(num_coeffs, 0.0);
    std::vector<double> cn_im(num_coeffs, 0.0);

    for (int n = -num_harmonics; n <= num_harmonics; ++n) {
        int idx = n + num_harmonics;
        double sum_re = 0.0;
        double sum_im = 0.0;
        for (int k = 0; k < num_samples; ++k) {
            double x = (double)k * dx;
            double fx = x * (2.0 * PI - x) * (x - PI);
            double angle = (double)n * x;
            double cos_a = std::cos(angle);
            double sin_a = std::sin(angle);
            sum_re += fx * cos_a;
            sum_im += -fx * sin_a;
        }
        cn_re[idx] = sum_re / (double)num_samples;
        cn_im[idx] = sum_im / (double)num_samples;
    }

    double d_eval = 2.0 * PI / (double)num_evals;
    double total_re = 0.0;
    double total_im = 0.0;
    for (int e = 0; e < num_evals; ++e) {
        double x_ev = (double)e * d_eval;
        double re_recon = 0.0;
        double im_recon = 0.0;
        for (int n = -num_harmonics; n <= num_harmonics; ++n) {
            int idx = n + num_harmonics;
            double cre = cn_re[idx];
            double cim = cn_im[idx];
            double angle = (double)n * x_ev;
            double e_re = std::cos(angle);
            double e_im = std::sin(angle);
            re_recon += cre * e_re - cim * e_im;
            im_recon += cre * e_im + cim * e_re;
        }
        total_re += re_recon;
        total_im += im_recon;
    }

    double energy = 0.0;
    for (int i = 0; i < num_coeffs; ++i) {
        energy += cn_re[i] * cn_re[i] + cn_im[i] * cn_im[i];
    }

    return {energy, total_re, total_im};
}

struct HyperResult {
    double sum_re;
    double sum_im;
    int total_terms;
};

HyperResult hypergeometric_2f1_benchmark(int grid_size = 50, int max_terms = 200, double tol = 1e-15) {
    double a = 0.5;
    double b = 1.0;
    double c = 2.0;
    double total_re = 0.0;
    double total_im = 0.0;
    int total_terms = 0;
    double tol2 = tol * tol;

    for (int ix = 0; ix < grid_size; ++ix) {
        double zr = ((double)ix - 25.0) * 0.015;
        for (int iy = 0; iy < grid_size; ++iy) {
            double zi = ((double)iy - 25.0) * 0.015;

            double term_re = 1.0;
            double term_im = 0.0;
            double sum_re = 1.0;
            double sum_im = 0.0;

            int n_used = max_terms;
            for (int n = 1; n < max_terms; ++n) {
                double dn = (double)n;
                double factor_scalar = ((a + dn - 1.0) * (b + dn - 1.0)) / ((c + dn - 1.0) * dn);
                double fz_re = factor_scalar * zr;
                double fz_im = factor_scalar * zi;

                double new_t_re = term_re * fz_re - term_im * fz_im;
                double new_t_im = term_re * fz_im + term_im * fz_re;
                term_re = new_t_re;
                term_im = new_t_im;

                sum_re += term_re;
                sum_im += term_im;

                double mag2 = term_re * term_re + term_im * term_im;
                if (mag2 < tol2) {
                    n_used = n;
                    break;
                }
            }
            total_terms += n_used;
            total_re += sum_re;
            total_im += sum_im;
        }
    }

    return {total_re, total_im, total_terms};
}

int main() {
    // 1. Fourier Series
    auto t0 = std::chrono::high_resolution_clock::now();
    FourierResult f_res = fourier_series_benchmark();
    auto t1 = std::chrono::high_resolution_clock::now();
    double fourier_ms = std::chrono::duration<double, std::milli>(t1 - t0).count();

    // 2. Hypergeometric Series
    auto t2 = std::chrono::high_resolution_clock::now();
    HyperResult h_res = hypergeometric_2f1_benchmark();
    auto t3 = std::chrono::high_resolution_clock::now();
    double hyper_ms = std::chrono::duration<double, std::milli>(t3 - t2).count();

    std::cout << "Fourier Series: energy=" << f_res.energy << ", total_re=" << f_res.total_re << " in " << fourier_ms << " ms" << std::endl;
    std::cout << "Hypergeometric 2F1: sum_re=" << h_res.sum_re << ", sum_im=" << h_res.sum_im << ", terms=" << h_res.total_terms << " in " << hyper_ms << " ms" << std::endl;

    long long chk_fourier = (long long)(f_res.energy * 100000.0) + (long long)(std::abs(f_res.total_re) * 1000.0);
    long long chk_hyper = (long long)(h_res.sum_re * 1000.0) + (long long)(h_res.sum_im * 1000.0) + (long long)h_res.total_terms;
    long long combined_checksum = (chk_fourier * 1000003LL) ^ chk_hyper;
    std::cout << "COMBINED_CHECKSUM: " << combined_checksum << std::endl;

    return 0;
}
