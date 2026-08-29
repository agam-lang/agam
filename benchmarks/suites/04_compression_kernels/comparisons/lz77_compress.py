def lz77_compress_checksum(data_size: int, window_size: int) -> int:
    checksum, pos = 0, 0
    while pos < data_size:
        best_len, best_dist = 0, 0
        search_start = max(0, pos - window_size)
        for s in range(search_start, pos):
            match_len = 0
            while pos + match_len < data_size:
                a = (s + match_len) * 7 + 13
                b = (pos + match_len) * 7 + 13
                if a % 256 != b % 256:
                    break
                match_len += 1
                if match_len > 15:
                    break
            if match_len > best_len:
                best_len = match_len
                best_dist = pos - s
        if best_len >= 3:
            checksum = (checksum * 31 + best_dist * 17 + best_len * 13) % 1000000007
            pos += best_len
        else:
            literal = pos * 7 + 13
            checksum = (checksum * 31 + literal % 256) % 1000000007
            pos += 1
    return checksum


if __name__ == "__main__":
    print(lz77_compress_checksum(512, 32))
