#include <iostream>
#include <chrono>

using namespace std;
using namespace std::chrono;

long long sum_loop(long long n) {
    long long total = 0;
    for (long long i = 0; i < n; i++) {
        total += i;
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
    for (long long i = 0; i < steps; i++) {
        sum += i * i;
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

int main() {
    cout << "=================================================================" << endl;
    cout << "  Agam vs Python vs Rust vs C++ — Advanced Benchmark Suite" << endl;
    cout << "  C++ Runtime (GCC -O3)" << endl;
    cout << "=================================================================" << endl;
    cout << endl;

    double total = 0.0;
    total += run_bench("Sum(100M)", sum_loop, 100000000);
    total += run_bench("Fibonacci(40)", fibonacci, 40);
    total += run_bench("PrimeCount(100K)", count_primes, 100000);
    total += run_bench("MatMul(100x100)", matrix_multiply, 100);
    total += run_bench("Integrate(10M)", integrate_x2, 10000000);

    cout << endl;
    cout << "  Total C++ time: " << total << "s" << endl;
    cout << "=================================================================" << endl;

    return 0;
}
