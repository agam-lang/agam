fn render_scene_hits(rays_per_pixel: i64, width: i64, height: i64) -> i64 {
    let mut hit_count: i64 = 0;
    for y in 0..height {
        for x in 0..width {
            for _r in 0..rays_per_pixel {
                let dx = x - (width / 2);
                let dy = y - (height / 2);
                let dz = 100i64;

                let b = -5i64 * dz;
                let c = 25i64 - 4i64;
                let disc = (b * b) - ((dx * dx + dy * dy + dz * dz) * c);

                if disc >= 0 {
                    hit_count += 1;
                }
            }
        }
    }
    hit_count
}

fn main() {
    let hits = render_scene_hits(16, 128, 128);
    println!("{}", hits);
}
