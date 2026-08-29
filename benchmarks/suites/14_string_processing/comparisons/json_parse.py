def json_parse(length: int) -> int:
    checksum = 0
    for iter_idx in range(100):
        state = 0
        elements = 0
        depth = 0
        for i in range(length):
            c = (i * 19 + iter_idx * 7) % 6
            if c == 0: c = 123
            elif c == 1: c = 125
            elif c == 2: c = 34
            elif c == 3: c = 58
            elif c == 4: c = 44
            else: c = 97
            
            if state == 0:
                if c == 123: state = 1; depth += 1
            elif state == 1:
                if c == 34: state = 2
                elif c == 125:
                    depth -= 1
                    if depth == 0: state = 0
                    else: state = 6
            elif state == 2:
                if c == 34: state = 3
            elif state == 3:
                if c == 58: state = 4
            elif state == 4:
                if c == 34: state = 5
                elif c == 123: state = 1; depth += 1
            elif state == 5:
                if c == 34: state = 6; elements += 1
            elif state == 6:
                if c == 44: state = 1
                elif c == 125:
                    depth -= 1
                    if depth == 0: state = 0
        checksum = (checksum * 31 + elements + depth) % 1000000007
    return checksum

if __name__ == "__main__":
    print(json_parse(10000))
