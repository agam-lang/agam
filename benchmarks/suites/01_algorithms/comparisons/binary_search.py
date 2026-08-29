def binary_search(limit: int, target: int) -> int:
    low = 0
    high = limit
    while low < high:
        mid = low + ((high - low) // 2)
        value = (mid * 3) + 7
        if value == target:
            return mid
        if value < target:
            low = mid + 1
        else:
            high = mid
    return -1


if __name__ == "__main__":
    print(binary_search(10000000, 2999998))
