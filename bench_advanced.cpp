#include <iostream>
#include <chrono>
#include <cstdlib>

using namespace std;
using namespace std::chrono;

long long sum_loop(long long n) {
    long long total = 0;
    long long state = (n % 7919) + 1;
    for (long long i = 0; i < n; i++) {
        state = (state * 57 + i * 13 + 17) % 1000003;
        total += state % 1024;
    }
    return total;
}

long long fibonacci(long long n) {
    long long a = 0;
    long long b = 1;
    for (long long i = 0; i < n; i++) {
        long long temp = a + b;
        a = b;
        b = temp;
    }
    return a;
}

long long count_primes(long long n) {
    long long count = 0;
    for (long long num = 2; num < n; num++) {
        bool is_prime = true;
        for (long long d = 2; d * d <= num; d++) {
            if (num % d == 0) {
                is_prime = false;
                break;
            }
        }
        if (is_prime) count++;
    }
    return count;
}

long long matrix_multiply(long long size) {
    long long sum = 0;
    for (long long i = 0; i < size; i++) {
        for (long long j = 0; j < size; j++) {
            long long val = 0;
            for (long long k = 0; k < size; k++) {
                val += (i * size + k) * (k * size + j);
            }
            sum += val;
        }
    }
    return sum;
}

long long integrate_x2(long long steps) {
    long long sum = 0;
    long long wobble = (steps % 1237) + 3;
    for (long long i = 0; i < steps; i++) {
        wobble = (wobble * 73 + 19) % 65521;
        sum += ((i * i) + wobble) % 4096;
    }
    return sum;
}

double run_bench(const char* name, long long (*func)(long long), long long arg) {
    auto start = high_resolution_clock::now();
    long long result = func(arg);
    auto end = high_resolution_clock::now();
    duration<double> elapsed = end - start;
    cout << "  " << name << ": " << elapsed.count() << "s  (result=" << result << ")" << endl;
    return elapsed.count();
}

int main(int argc, char** argv) {
    long long sum_n = 100000000;
    long long fib_n = 40;
    long long prime_n = 100000;
    long long mat_n = 100;
    long long integrate_n = 10000000;
    if (argc > 1) sum_n = atoll(argv[1]);
    if (argc > 2) fib_n = atoll(argv[2]);
    if (argc > 3) prime_n = atoll(argv[3]);
    if (argc > 4) mat_n = atoll(argv[4]);
    if (argc > 5) integrate_n = atoll(argv[5]);

    cout << "=================================================================" << endl;
    cout << "  Agam vs Python vs Rust vs C++ — Advanced Benchmark Suite" << endl;
    cout << "  C++ Runtime (GCC -O3)" << endl;
    cout << "=================================================================" << endl;
    cout << endl;

    double total = 0.0;
    total += run_bench("Sum", sum_loop, sum_n);
    total += run_bench("Fibonacci", fibonacci, fib_n);
    total += run_bench("PrimeCount", count_primes, prime_n);
    total += run_bench("MatMul", matrix_multiply, mat_n);
    total += run_bench("Integrate", integrate_x2, integrate_n);

    cout << endl;
    cout << "  Total C++ time: " << total << "s" << endl;
    cout << "=================================================================" << endl;

    return 0;
}
