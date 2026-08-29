#include <iostream>

int64_t valkey_pipeline_ops(int64_t num_ops) {
    int64_t checksum = 0;
    int64_t slot0 = 0;
    int64_t slot1 = 0;
    int64_t slot2 = 0;
    int64_t slot3 = 0;

    for (int64_t op = 0; op < num_ops; ++op) {
        int64_t key = ((op * 100003LL) / 65536) % 4;
        int64_t op_type = op % 3;

        if (op_type == 0) {
            if (key == 0) slot0 = op;
            if (key == 1) slot1 = op;
            if (key == 2) slot2 = op;
            if (key == 3) slot3 = op;
        }
        if (op_type == 1) {
            int64_t val = 0;
            if (key == 0) val = slot0;
            if (key == 1) val = slot1;
            if (key == 2) val = slot2;
            if (key == 3) val = slot3;
            checksum += val;
        }
        if (op_type == 2) {
            if (key == 0) { slot0 += 1; checksum += slot0; }
            if (key == 1) { slot1 += 1; checksum += slot1; }
            if (key == 2) { slot2 += 1; checksum += slot2; }
            if (key == 3) { slot3 += 1; checksum += slot3; }
        }
    }
    return checksum % 1000000007;
}

int main() {
    int64_t res = valkey_pipeline_ops(100000);
    std::cout << res << std::endl;
    return 0;
}
