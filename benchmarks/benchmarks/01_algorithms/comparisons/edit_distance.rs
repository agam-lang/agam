fn edit_distance_cost(left: i64, right: i64) -> i64 {
    let mut total = 0_i64;
    for row in 0..left {
        for col in 0..right {
            let mismatch = ((row * 17) + (col * 13)) % 11;
            if mismatch < 2 {
                total += 1;
            } else {
                total += (row + col) % 3;
            }
        }
    }
    total
}

fn main() {
    println!("{}", edit_distance_cost(2800, 2800));
}
