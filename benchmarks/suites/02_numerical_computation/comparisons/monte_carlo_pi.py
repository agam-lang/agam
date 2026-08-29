def monte_carlo(samples: int) -> int:
    inside, seed = 0, 123456789
    for _ in range(samples):
        seed = ((seed * 1103515245) + 12345) % 2147483647
        x = seed % 10000
        seed = ((seed * 1103515245) + 12345) % 2147483647
        y = seed % 10000
        if (x * x) + (y * y) <= 100000000:
            inside += 1
    return (inside * 4000000) // samples

if __name__ == "__main__":
    print(monte_carlo(500000))
