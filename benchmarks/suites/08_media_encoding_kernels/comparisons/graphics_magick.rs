fn min3(a: i64, b: i64, c: i64) -> i64 {
    a.min(b).min(c)
}

fn max3(a: i64, b: i64, c: i64) -> i64 {
    a.max(b).max(c)
}

fn graphics_magick_pipeline(dim: i64) -> i64 {
    let mod_byte: i64 = 256;
    let mod_prime: i64 = 1000000007;

    let mut checksum: i64 = 0;

    for y in 1..dim - 1 {
        for x in 1..dim - 1 {
            let r_c = (x * y * 17) % mod_byte;
            let g_c = (x * y * 31) % mod_byte;
            let b_c = (x * y * 47) % mod_byte;

            let r_top = (x * (y - 1) * 17) % mod_byte;
            let r_bot = (x * (y + 1) * 17) % mod_byte;
            let r_left = ((x - 1) * y * 17) % mod_byte;
            let r_right = ((x + 1) * y * 17) % mod_byte;

            let r_tl = ((x - 1) * (y - 1) * 17) % mod_byte;
            let r_tr = ((x + 1) * (y - 1) * 17) % mod_byte;
            let r_bl = ((x - 1) * (y + 1) * 17) % mod_byte;
            let r_br = ((x + 1) * (y + 1) * 17) % mod_byte;

            // Gaussian Sharpen Kernel: [-1 -2 -1; -2 13 -2; -1 -2 -1]
            let conv_r = (r_c * 13)
                       - (r_top * 2) - (r_bot * 2) - (r_left * 2) - (r_right * 2)
                       - r_tl - r_tr - r_bl - r_br;

            let sharp_r = conv_r.clamp(0, 255);

            // RGB-to-HWB Color Space Transformation (magick/color.c)
            let w_val = min3(sharp_r, g_c, b_c);
            let v_val = max3(sharp_r, g_c, b_c);
            let b_val = 255 - v_val;

            let delta = v_val - w_val;
            let mut hue = 0;
            if delta > 0 {
                if v_val == sharp_r {
                    hue = ((g_c - b_c) * 60) / delta;
                } else if v_val == g_c {
                    hue = 120 + (((b_c - sharp_r) * 60) / delta);
                } else {
                    hue = 240 + (((sharp_r - g_c) * 60) / delta);
                }
                if hue < 0 { hue += 360; }
            }

            // Swirl Coordinate Distortion Weighting
            let dx = x - (dim / 2);
            let dy = y - (dim / 2);
            let r_sq = (dx * dx) + (dy * dy);
            let swirl_factor = r_sq % 360;

            let pixel_metric = sharp_r + (w_val * 2) + (b_val * 3) + hue + swirl_factor;
            checksum = (checksum + pixel_metric) % mod_prime;
        }
    }

    checksum
}

fn main() {
    let res = graphics_magick_pipeline(256);
    println!("{}", res);
}
