def audio_lpc_checksum(frames: int) -> int:
    order, frame_size = 10, 256
    checksum = 0
    for f in range(frames):
        max_r = 0
        for lag in range(order + 1):
            r = 0
            for i in range(frame_size - lag):
                s1 = ((f * frame_size + i) * 17 + 13) % 65536 - 32768
                s2 = ((f * frame_size + i + lag) * 17 + 13) % 65536 - 32768
                r += int((s1 * s2) / 32768)
            if r < 0: r = -r
            if r > max_r: max_r = r
            checksum = (checksum * 31 + r) % 1000000007
        
        residual = 0
        for i_r in range(order, frame_size):
            pred = 0
            for j in range(1, order + 1):
                s = ((f * frame_size + i_r - j) * 17 + 13) % 65536 - 32768
                coef = (j * 17) % 100
                pred += int((s * coef) / 100)
            actual = ((f * frame_size + i_r) * 17 + 13) % 65536 - 32768
            diff = actual - pred
            if diff < 0: diff = -diff
            residual += diff
        checksum = (checksum * 37 + residual + max_r) % 1000000007
    return checksum

if __name__ == "__main__":
    print(audio_lpc_checksum(32))
