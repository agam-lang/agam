#include <cstdio>

static long long audio_lpc_checksum(long long frames) {
    long long order = 10;
    long long frame_size = 256;
    long long checksum = 0;
    
    for (long long f = 0; f < frames; ++f) {
        long long max_r = 0;
        for (long long lag = 0; lag <= order; ++lag) {
            long long r = 0;
            for (long long i = 0; i < frame_size - lag; ++i) {
                long long s1 = ((f * frame_size + i) * 17 + 13) % 65536 - 32768;
                long long s2 = ((f * frame_size + i + lag) * 17 + 13) % 65536 - 32768;
                r += (s1 * s2) / 32768;
            }
            if (r < 0) r = -r;
            if (r > max_r) max_r = r;
            checksum = (checksum * 31 + r) % 1000000007;
        }
        
        long long residual = 0;
        for (long long i_r = order; i_r < frame_size; ++i_r) {
            long long pred = 0;
            for (long long j = 1; j <= order; ++j) {
                long long s = ((f * frame_size + i_r - j) * 17 + 13) % 65536 - 32768;
                long long coef = (j * 17) % 100;
                pred += (s * coef) / 100;
            }
            long long actual = ((f * frame_size + i_r) * 17 + 13) % 65536 - 32768;
            long long diff = actual - pred;
            if (diff < 0) diff = -diff;
            residual += diff;
        }
        checksum = (checksum * 37 + residual + max_r) % 1000000007;
    }
    return checksum;
}

int main() { printf("%lld\n", audio_lpc_checksum(32)); return 0; }
