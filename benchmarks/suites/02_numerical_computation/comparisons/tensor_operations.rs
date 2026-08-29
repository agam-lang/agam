fn tensor_checksum(w: i64, h: i64, d: i64) -> i64 {
    let mut total: i64 = 0;
    for z in 0..d { for y in 0..h { for x in 0..w {
        total += ((x * 31) + (y * 17) + (z * 13)) % 4093;
    }}}
    total
}
fn main() { println!("{}", tensor_checksum(96, 96, 16)); }
