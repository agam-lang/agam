def render_scene_hits(rays_per_pixel: int, width: int, height: int) -> int:
    hit_count = 0
    for y in range(height):
        for x in range(width):
            for _r in range(rays_per_pixel):
                dx = x - int(width / 2)
                dy = y - int(height / 2)
                dz = 100

                b = -5 * dz
                c = 25 - 4
                disc = (b * b) - ((dx * dx + dy * dy + dz * dz) * c)

                if disc >= 0:
                    hit_count += 1

    return hit_count

if __name__ == "__main__":
    hits = render_scene_hits(16, 128, 128)
    print(hits)
