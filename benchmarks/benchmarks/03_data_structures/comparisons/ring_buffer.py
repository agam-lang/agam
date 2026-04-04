def ring_buffer_cost(capacity: int, rounds: int) -> int:
    head = 0
    tail = 0
    acc = 0
    for item in range(rounds):
        slot = (head + item) % capacity
        acc += ((slot * 17) + item) % 257
        if (item % 3) == 0:
            tail = (tail + 1) % capacity
            acc += tail
        head = (head + 1) % capacity
    return acc + head + tail


if __name__ == "__main__":
    print(ring_buffer_cost(4096, 12000000))
