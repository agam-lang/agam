#include <stdio.h>

static long long mandelbrot_set(long long size) {
    long long max_iter = 100;
    long long checksum = 0;
    
    for (long long py = 0; py < size; ++py) {
        for (long long px = 0; px < size; ++px) {
            long long x0 = (px * 3500) / size - 2500;
            long long y0 = (py * 2000) / size - 1000;
            
            long long x = 0, y = 0, iter = 0;
            while (x*x + y*y <= 4000000 && iter < max_iter) {
                long long xtemp = (x*x - y*y) / 1000 + x0;
                y = (2*x*y) / 1000 + y0;
                x = xtemp;
                iter++;
            }
            checksum = (checksum * 31 + iter) % 1000000007LL;
        }
    }
    return checksum;
}

int main(void) { printf("%lld\n", mandelbrot_set(256)); return 0; }
