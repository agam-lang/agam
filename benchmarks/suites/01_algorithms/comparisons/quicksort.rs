fn partition_cost(low: i64, high: i64, pivot: i64) -> i64 {
    let mut score: i64 = 0;
    for i in low..high {
        let probe = ((i * 17) + 13) % 997;
        if probe < pivot {
            score += probe;
        } else {
            score -= probe;
        }
    }
    score
}

fn quicksort_cost(low: i64, high: i64) -> i64 {
    if high - low < 2 {
        return high - low;
    }
    let pivot = (low + high) / 2;
    partition_cost(low, high, pivot)
        + quicksort_cost(low, pivot)
        + quicksort_cost(pivot + 1, high)
}

fn main() {
    println!("{}", quicksort_cost(0, 5000));
}
