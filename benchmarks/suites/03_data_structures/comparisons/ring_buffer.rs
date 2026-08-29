fn ring_buffer_cost(cap: i64, rounds: i64) -> i64 {
    let (mut head, mut tail, mut acc) = (0i64, 0i64, 0i64);
    for item in 0..rounds {
        let slot = (head + item) % cap;
        acc += ((slot * 17) + item) % 257;
        if item % 3 == 0 { tail = (tail + 1) % cap; acc += tail; }
        head = (head + 1) % cap;
    }
    acc + head + tail
}
fn main() { println!("{}", ring_buffer_cost(4096, 12000000)); }
