def ray_sphere_intersect(rays: int) -> int:
    checksum = 0
    sx, sy, sz, radius = 500, 500, 1000, 300
    r2 = radius * radius
    
    for r in range(rays):
        ox = ((r * 17) % 2000) - 1000
        oy = ((r * 31) % 2000) - 1000
        oz = -1000
        
        dx = ((r * 13) % 100) - 50
        dy = ((r * 19) % 100) - 50
        dz = 100
        
        inv_len = 10000 // (dx*dx + dy*dy + dz*dz + 1)
        dx = (dx * inv_len) // 100
        dy = (dy * inv_len) // 100
        dz = (dz * inv_len) // 100
        
        ocx = ox - sx
        ocy = oy - sy
        ocz = oz - sz
        
        b = 2 * (ocx * dx + ocy * dy + ocz * dz)
        c = (ocx * ocx + ocy * ocy + ocz * ocz) - r2
        discriminant = (b * b) - (4 * c)
        
        hit = False
        if discriminant >= 0:
            t1 = (-b - discriminant // 100) // 2
            t2 = (-b + discriminant // 100) // 2
            if t1 > 0 or t2 > 0:
                hit = True
                checksum = (checksum * 31 + t1 + t2) % 1000000007
        
        if not hit:
            checksum = (checksum * 37 + 1) % 1000000007
            
    return checksum

if __name__ == "__main__":
    print(ray_sphere_intersect(8192))
