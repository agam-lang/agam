def regex_match(length: int) -> int:
    checksum = 0
    for iter_idx in range(100):
        state = 0
        matches = 0
        for i in range(length):
            c = ((i * 17 + iter_idx * 13) % 5) + 97
            if state == 0:
                if c == 97: state = 0
                elif c == 98: state = 1
                else: state = 0
            elif state == 1:
                if c == 98: state = 1
                elif c == 99: state = 2
                elif c == 100: state = 3
                elif c == 97: state = 0
                else: state = 0
            elif state == 2:
                if c == 100: state = 3
                elif c == 97: state = 0
                else: state = 0
            elif state == 3:
                matches += 1
                if c == 97: state = 0
                elif c == 98: state = 1
                else: state = 0
        checksum = (checksum * 31 + matches) % 1000000007
    return checksum

if __name__ == "__main__":
    print(regex_match(10000))
