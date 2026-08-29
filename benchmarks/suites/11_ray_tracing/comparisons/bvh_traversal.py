def non_zero_dir(value: int) -> int:
    return value + 1 if value >= 0 else value - 1


def bvh_traversal(rays: int) -> int:
    nodes = 256
    checksum = 0
    
    for r in range(rays):
        ox = (r * 17) % 1000
        oy = (r * 31) % 1000
        dx = ((r * 13) % 20) - 10
        dy = ((r * 19) % 20) - 10
        dx_div = non_zero_dir(dx)
        dy_div = non_zero_dir(dy)
        
        stack_size = 0
        current_node = 0
        hits = 0
        
        steps = 0
        while steps < 100:
            min_x = (current_node * 7) % 1000
            max_x = min_x + 100
            min_y = (current_node * 11) % 1000
            max_y = min_y + 100
            
            tmin_x = (min_x - ox) * 100 // dx_div
            tmax_x = (max_x - ox) * 100 // dx_div
            if tmin_x > tmax_x: tmin_x, tmax_x = tmax_x, tmin_x
            
            tmin_y = (min_y - oy) * 100 // dy_div
            tmax_y = (max_y - oy) * 100 // dy_div
            if tmin_y > tmax_y: tmin_y, tmax_y = tmax_y, tmin_y
            
            tmin = max(tmin_x, tmin_y)
            tmax = min(tmax_x, tmax_y)
            
            if tmax >= tmin and tmax > 0:
                is_leaf = (current_node % 3 == 0)
                if is_leaf:
                    hits += 1
                    stack_size -= 1
                    if stack_size < 0: steps = 999
                    current_node = (current_node * 2) % nodes
                else:
                    stack_size += 1
                    current_node = (current_node * 2 + 1) % nodes
            else:
                stack_size -= 1
                if stack_size < 0: steps = 999
                current_node = (current_node + 1) % nodes
            steps += 1
        checksum = (checksum * 31 + hits) % 1000000007
    return checksum

if __name__ == "__main__":
    print(bvh_traversal(4096))
