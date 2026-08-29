def pointer_chase(nodes: int, rounds: int) -> int:
    cursor, checksum = 1, 0
    for _ in range(rounds):
        cursor = ((cursor * 1103515245) + 12345) % nodes
        checksum += cursor
    return checksum
if __name__ == "__main__":
    print(pointer_chase(1000003, 8000000))
