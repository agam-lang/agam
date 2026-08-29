def ring_buffer_cost(cap: int, rounds: int) -> int:
    head, tail, acc = 0, 0, 0
    for item in range(rounds):
        slot = (head + item) % cap
        acc += ((slot * 17) + item) % 257
        if item % 3 == 0:
            tail = (tail + 1) % cap
            acc += tail
        head = (head + 1) % cap
    return acc + head + tail
if __name__ == "__main__":
    print(ring_buffer_cost(4096, 12000000))
