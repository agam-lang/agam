def mandelbrot_set(size: int) -> int:
    max_iter = 100
    checksum = 0
    
    for py in range(size):
        for px in range(size):
            x0 = (px * 3500) // size - 2500
            y0 = (py * 2000) // size - 1000
            
            x, y, iter_count = 0, 0, 0
            while x*x + y*y <= 4000000 and iter_count < max_iter:
                xtemp = int((x*x - y*y) / 1000) + x0
                y = int((2*x*y) / 1000) + y0
                x = xtemp
                iter_count += 1
                
            checksum = (checksum * 31 + iter_count) % 1000000007
    return checksum

if __name__ == "__main__":
    print(mandelbrot_set(256))
