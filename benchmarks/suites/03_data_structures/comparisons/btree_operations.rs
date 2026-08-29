fn branch_walk(depth: i64, fanout: i64) -> i64 {
    if depth == 0 { return fanout; }
    let mut total: i64 = 0;
    for c in 0..fanout { total += branch_walk(depth - 1, fanout - 1) + c; }
    total
}
fn main() { println!("{}", branch_walk(5, 6)); }
