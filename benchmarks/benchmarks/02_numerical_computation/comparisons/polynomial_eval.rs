fn polynomial_cost(points: i64, degree: i64) -> i64 {
    let mut checksum = 0_i64;
    for point in 0..points {
        let x = (point % 97) + 3;
        let mut value = 1_i64;
        for coeff in (1..=degree).rev() {
            value = ((value * x) + ((coeff * 11) + (point % 29))) % 1_000_003;
        }
        checksum += value;
    }
    checksum
}

fn main() {
    println!("{}", polynomial_cost(800000, 16));
}
