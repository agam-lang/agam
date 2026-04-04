fn ring_buffer_cost(capacity: i64, rounds: i64) -> i64 {
    let mut head = 0_i64;
    let mut tail = 0_i64;
    let mut acc = 0_i64;
    for item in 0..rounds {
        let slot = (head + item) % capacity;
        acc += ((slot * 17) + item) % 257;
        if (item % 3) == 0 {
            tail = (tail + 1) % capacity;
            acc += tail;
        }
        head = (head + 1) % capacity;
    }
    acc + head + tail
}

fn main() {
    println!("{}", ring_buffer_cost(4096, 12000000));
}
