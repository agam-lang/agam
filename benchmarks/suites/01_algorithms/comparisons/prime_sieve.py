import math


def is_prime(n: int) -> bool:
    if n < 2:
        return False
    for d in range(2, int(math.isqrt(n)) + 1):
        if n % d == 0:
            return False
    return True


def prime_count(limit: int) -> int:
    total = 0
    for n in range(2, limit + 1):
        if is_prime(n):
            total += 1
    return total


if __name__ == "__main__":
    print(prime_count(25000))
