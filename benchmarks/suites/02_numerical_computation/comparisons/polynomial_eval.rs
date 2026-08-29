fn polynomial_cost(points: i64, degree: i64) -> i64 {
    let mut checksum: i64 = 0;
    for p in 0..points {
        let x = (p % 97) + 3;
        let mut value: i64 = 1;
        let mut c = degree;
        while c > 0 { value = ((value * x) + ((c * 11) + (p % 29))) % 1000003; c -= 1; }
        checksum += value;
    }
    checksum
}
fn main() { println!("{}", polynomial_cost(800000, 16)); }
