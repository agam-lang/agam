def hashmap_probe_cost(slots: int, rounds: int) -> int:
    total = 0
    for r in range(rounds):
        key = (r * 1103515245) % 2147483647
        total += ((key % slots) * 37) % 4099
    return total

if __name__ == "__main__":
    print(hashmap_probe_cost(65536, 5000000))
