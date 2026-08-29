def autodiff_trace(steps: int) -> int:
    value, grad = 7, 1
    for _ in range(steps):
        grad += (value * 3) % 17
        value += grad
    return value + grad
if __name__ == "__main__":
    print(autodiff_trace(5000000))
