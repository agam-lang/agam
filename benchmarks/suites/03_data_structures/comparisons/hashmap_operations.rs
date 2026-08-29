fn hashmap_probe_cost(slots: i64, rounds: i64) -> i64 {
    let mut total: i64 = 0;
    for r in 0..rounds {
        let key = (r * 1103515245) % 2147483647;
        total += ((key % slots) * 37) % 4099;
    }
    total
}

fn main() {
    println!("{}", hashmap_probe_cost(65536, 5000000));
}
