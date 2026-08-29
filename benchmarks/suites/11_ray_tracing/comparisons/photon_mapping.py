def distance_sq(x1: int, y1: int, z1: int, x2: int, y2: int, z2: int) -> int:
    dx, dy, dz = x2 - x1, y2 - y1, z2 - z1
    return dx*dx + dy*dy + dz*dz

def photon_mapping(photons: int) -> int:
    queries = 1024
    checksum = 0
    
    for q in range(queries):
        qx, qy, qz = (q * 17) % 1000, (q * 19) % 1000, (q * 23) % 1000
        gathered = 0
        radius_sq = 10000
        
        for p in range(photons):
            px, py, pz = (p * 31) % 1000, (p * 37) % 1000, (p * 41) % 1000
            d2 = distance_sq(qx, qy, qz, px, py, pz)
            if d2 < radius_sq:
                power = 1000 - (d2 * 1000) // radius_sq
                gathered += power
        checksum = (checksum * 31 + gathered) % 1000000007
    return checksum

if __name__ == "__main__":
    print(photon_mapping(4096))
