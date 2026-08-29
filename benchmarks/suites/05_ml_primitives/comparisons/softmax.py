def softmax_like(width: int, rounds: int) -> int:
    total = 0
    for r in range(rounds):
        partial = 0
        for l in range(width):
            partial += ((l * 11) + r) % 251
        total += partial
    return total
if __name__ == "__main__":
    print(softmax_like(4096, 4000))
