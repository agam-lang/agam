def graphics_magick_pipeline(dim: int) -> int:
    mod_byte = 256
    mod_prime = 1000000007

    checksum = 0

    for y in range(1, dim - 1):
        for x in range(1, dim - 1):
            r_c = (x * y * 17) % mod_byte
            g_c = (x * y * 31) % mod_byte
            b_c = (x * y * 47) % mod_byte

            r_top = (x * (y - 1) * 17) % mod_byte
            r_bot = (x * (y + 1) * 17) % mod_byte
            r_left = ((x - 1) * y * 17) % mod_byte
            r_right = ((x + 1) * y * 17) % mod_byte

            r_tl = ((x - 1) * (y - 1) * 17) % mod_byte
            r_tr = ((x + 1) * (y - 1) * 17) % mod_byte
            r_bl = ((x - 1) * (y + 1) * 17) % mod_byte
            r_br = ((x + 1) * (y + 1) * 17) % mod_byte

            cross_term = (r_top * 2) + (r_bot * 2) + (r_left * 2) + (r_right * 2)
            diag_term = r_tl + r_tr + r_bl + r_br
            conv_r = (r_c * 13) - cross_term - diag_term
            sharp_r = max(0, min(255, conv_r))

            w_val = min(sharp_r, g_c, b_c)
            v_val = max(sharp_r, g_c, b_c)
            b_val = 255 - v_val

            delta = v_val - w_val
            hue = 0
            if delta > 0:
                if v_val == sharp_r:
                    hue = int(((g_c - b_c) * 60) / delta)
                elif v_val == g_c:
                    hue = 120 + int(((b_c - sharp_r) * 60) / delta)
                else:
                    hue = 240 + int(((sharp_r - g_c) * 60) / delta)
                if hue < 0:
                    hue += 360

            dx = x - int(dim / 2)
            dy = y - int(dim / 2)
            r_sq = (dx * dx) + (dy * dy)
            swirl_factor = r_sq % 360

            pixel_metric = sharp_r + (w_val * 2) + (b_val * 3) + hue + swirl_factor
            checksum = (checksum + pixel_metric) % mod_prime

    return checksum

if __name__ == "__main__":
    res = graphics_magick_pipeline(256)
    print(res)
