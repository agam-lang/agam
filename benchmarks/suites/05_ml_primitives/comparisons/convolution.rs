fn conv_score(w: i64, h: i64) -> i64 {
    let mut total: i64 = 0;
    for y in 1..h-1 { for x in 1..w-1 { total += ((x * 3) + (y * 5)) % 97; } }
    total
}
fn main() { println!("{}", conv_score(512, 512)); }
