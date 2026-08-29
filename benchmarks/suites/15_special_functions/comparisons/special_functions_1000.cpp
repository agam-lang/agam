#include <iostream>
#include <fstream>
#include <vector>
#include <cmath>
#include <chrono>
#include <iomanip>
#include <string>

static const double PI = 3.14159265358979323846;

struct Row {
    int index;
    std::string type;
    double input_re;
    double input_im;
    double output_re;
    double output_im;
    int terms;
};

std::vector<Row> run_all(int num_harmonics = 16, int num_samples = 1024, int num_points = 1000) {
    std::vector<Row> rows;
    rows.reserve(2 * num_points);

    // 1. Fourier coefficients
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
            sum_re += fx * std::cos(angle);
            sum_im += -fx * std::sin(angle);
        }
        cn_re[idx] = sum_re / (double)num_samples;
        cn_im[idx] = sum_im / (double)num_samples;
    }

    // 2. Evaluate 1000 Fourier points
    double d_eval = 2.0 * PI / (double)num_points;
    for (int k = 0; k < num_points; ++k) {
        double x = (double)k * d_eval;
        double re_recon = 0.0;
        double im_recon = 0.0;
        for (int n = -num_harmonics; n <= num_harmonics; ++n) {
            int idx = n + num_harmonics;
            double cre = cn_re[idx];
            double cim = cn_im[idx];
            double angle = (double)n * x;
            double e_re = std::cos(angle);
            double e_im = std::sin(angle);
            re_recon += cre * e_re - cim * e_im;
            im_recon += cre * e_im + cim * e_re;
        }
        rows.push_back({k, "fourier", x, 0.0, re_recon, im_recon, num_coeffs});
    }

    // 3. Evaluate 1000 Hypergeometric 2F1 points
    double a = 0.5;
    double b = 1.0;
    double c = 2.0;
    int max_terms = 200;
    double tol2 = 1e-30;

    for (int k = 0; k < num_points; ++k) {
        double r = 0.05 + 0.85 * ((double)k / (double)num_points);
        double theta = (double)k * 2.399963229728653;
        double zr = r * std::cos(theta);
        double zi = r * std::sin(theta);

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
        rows.push_back({1000 + k, "hypergeometric", zr, zi, sum_re, sum_im, n_used});
    }

    return rows;
}

int main(int argc, char** argv) {
    std::string out_path = "output_cpp.csv";
    if (argc > 1) {
        out_path = argv[1];
    }

    auto t0 = std::chrono::high_resolution_clock::now();
    std::vector<Row> rows = run_all(16, 1024, 1000);
    auto t1 = std::chrono::high_resolution_clock::now();
    double elapsed_ms = std::chrono::duration<double, std::milli>(t1 - t0).count();

    std::ofstream f(out_path);
    f << "index,type,input_re,input_im,output_re,output_im,terms\n";
    f << std::fixed << std::setprecision(12);
    for (const auto& r : rows) {
        f << r.index << "," << r.type << "," 
          << std::setprecision(10) << r.input_re << "," << r.input_im << "," 
          << std::setprecision(12) << r.output_re << "," << r.output_im << "," << r.terms << "\n";
    }
    f.close();

    std::cout << "[C++ Clang -O3] Computed 2,000 values in " << elapsed_ms << " ms -> written to " << out_path << std::endl;
    return 0;
}
