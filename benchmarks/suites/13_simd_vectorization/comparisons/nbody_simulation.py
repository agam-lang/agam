def nbody_simulation(bodies: int) -> int:
    steps = 10
    checksum = 0
    
    for step in range(steps):
        for i in range(bodies):
            px = ((i * 17 + step) % 2000) - 1000
            py = ((i * 19 + step) % 2000) - 1000
            pz = ((i * 23 + step) % 2000) - 1000
            
            fx, fy, fz = 0, 0, 0
            for j in range(bodies):
                if i != j:
                    ox = ((j * 17 + step) % 2000) - 1000
                    oy = ((j * 19 + step) % 2000) - 1000
                    oz = ((j * 23 + step) % 2000) - 1000
                    
                    dx, dy, dz = ox - px, oy - py, oz - pz
                    d_sq = dx*dx + dy*dy + dz*dz
                    if d_sq == 0: d_sq = 1
                    
                    inv_dist_cubed = 1000000 // (d_sq * d_sq)
                    fx += dx * inv_dist_cubed
                    fy += dy * inv_dist_cubed
                    fz += dz * inv_dist_cubed
                    
            checksum = (checksum * 31 + fx + fy + fz) % 1000000007
    return checksum

if __name__ == "__main__":
    print(nbody_simulation(256))
