def html_escape(length: int) -> int:
    checksum = 0
    for iter_idx in range(100):
        out_len = 0
        for i in range(length):
            c = (i * 23 + iter_idx * 11) % 10
            if c == 0:
                out_len += 4
                checksum = (checksum * 31 + 60) % 1000000007
            elif c == 1:
                out_len += 4
                checksum = (checksum * 31 + 62) % 1000000007
            elif c == 2:
                out_len += 5
                checksum = (checksum * 31 + 38) % 1000000007
            elif c == 3:
                out_len += 6
                checksum = (checksum * 31 + 34) % 1000000007
            elif c == 4:
                out_len += 5
                checksum = (checksum * 31 + 39) % 1000000007
            else:
                out_len += 1
                checksum = (checksum * 31 + 97 + c) % 1000000007
        checksum = (checksum * 37 + out_len) % 1000000007
    return checksum

if __name__ == "__main__":
    print(html_escape(10000))
