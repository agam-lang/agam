fn binary_search(limit: i64, target: i64) -> i64 {
    let mut low: i64 = 0;
    let mut high: i64 = limit;
    while low < high {
        let mid = low + ((high - low) / 2);
        let value = (mid * 3) + 7;
        if value == target {
            return mid;
        }
        if value < target {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    -1
}

fn main() {
    println!("{}", binary_search(10000000, 2999998));
}
