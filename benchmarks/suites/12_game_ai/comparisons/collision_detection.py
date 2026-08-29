def collision_detection(entities: int) -> int:
    checksum = 0
    frames = 60
    
    for f in range(frames):
        collisions = 0
        for i in range(entities):
            x1 = ((i * 17 + f * 13) % 1000) - 500
            y1 = ((i * 19 + f * 11) % 1000) - 500
            r1 = ((i * 23) % 40) + 10
            
            for j in range(i + 1, entities):
                x2 = ((j * 17 + f * 13) % 1000) - 500
                y2 = ((j * 19 + f * 11) % 1000) - 500
                r2 = ((j * 23) % 40) + 10
                
                dx, dy = x2 - x1, y2 - y1
                dist_sq = dx * dx + dy * dy
                rad_sum = r1 + r2
                
                if dist_sq <= rad_sum * rad_sum:
                    collisions += 1
                    checksum = (checksum * 31 + i + j) % 1000000007
        checksum = (checksum * 37 + collisions) % 1000000007
    return checksum

if __name__ == "__main__":
    print(collision_detection(512))
