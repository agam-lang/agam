def valkey_pipeline_ops(num_ops: int) -> int:
    checksum = 0
    slot0 = slot1 = slot2 = slot3 = 0

    for op in range(num_ops):
        key = int((op * 100003) // 65536) % 4

        if op % 3 == 0:
            if key == 0: slot0 = op
            if key == 1: slot1 = op
            if key == 2: slot2 = op
            if key == 3: slot3 = op
        if op % 3 == 1:
            val = 0
            if key == 0: val = slot0
            if key == 1: val = slot1
            if key == 2: val = slot2
            if key == 3: val = slot3
            checksum += val
        if op % 3 == 2:
            if key == 0: slot0 += 1; checksum += slot0
            if key == 1: slot1 += 1; checksum += slot1
            if key == 2: slot2 += 1; checksum += slot2
            if key == 3: slot3 += 1; checksum += slot3

    return checksum % 1000000007

if __name__ == "__main__":
    res = valkey_pipeline_ops(100000)
    print(res)
