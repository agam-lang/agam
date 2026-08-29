def zbuffer_rasterize(triangles: int) -> int:
    width, height = 256, 256
    checksum = 0
    
    for t in range(triangles):
        x0, y0, z0 = (t * 17) % width, (t * 19) % height, (t * 23) % 1000
        x1, y1, z1 = (x0 + 50) % width, (y0 + 20) % height, (t * 29) % 1000
        x2, y2, z2 = (x0 + 20) % width, (y0 + 50) % height, (t * 31) % 1000
        
        min_x, max_x = min(x0, x1, x2), max(x0, x1, x2)
        min_y, max_y = min(y0, y1, y2), max(y0, y1, y2)
        
        area = (x1 - x0) * (y2 - y0) - (x2 - x0) * (y1 - y0)
        if area == 0: area = 1
        if area < 0: area = -area
        
        pixels_drawn = 0
        for py in range(min_y, max_y + 1):
            for px in range(min_x, max_x + 1):
                w0 = (x1 - px) * (y2 - py) - (x2 - px) * (y1 - py)
                w1 = (x2 - px) * (y0 - py) - (x0 - px) * (y2 - py)
                w2 = (x0 - px) * (y1 - py) - (x1 - px) * (y0 - py)
                
                is_inside = (w0 >= 0 and w1 >= 0 and w2 >= 0) or (w0 <= 0 and w1 <= 0 and w2 <= 0)
                
                if is_inside:
                    z = int((w0 * z0 + w1 * z1 + w2 * z2) / area)
                    pixels_drawn += 1
                    checksum = (checksum * 31 + z + px + py) % 1000000007
        checksum = (checksum * 37 + pixels_drawn) % 1000000007
    return checksum

if __name__ == "__main__":
    print(zbuffer_rasterize(1024))
