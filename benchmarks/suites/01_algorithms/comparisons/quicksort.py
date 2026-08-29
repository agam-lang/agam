def partition_cost(low: int, high: int, pivot: int) -> int:
    score = 0
    for i in range(low, high):
        probe = ((i * 17) + 13) % 997
        if probe < pivot:
            score += probe
        else:
            score -= probe
    return score


def quicksort_cost(low: int, high: int) -> int:
    if high - low < 2:
        return high - low
    pivot = (low + high) // 2
    return (
        partition_cost(low, high, pivot)
        + quicksort_cost(low, pivot)
        + quicksort_cost(pivot + 1, high)
    )


if __name__ == "__main__":
    print(quicksort_cost(0, 5000))
