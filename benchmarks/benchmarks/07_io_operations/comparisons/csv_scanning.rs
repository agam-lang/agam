fn csv_scan_cost(rows: i64, cols: i64) -> i64 {
    let mut checksum = 0_i64;
    for row in 0..rows {
        for col in 0..cols {
            let field = ((row * 37) + (col * 13)) % 1009;
            if (col + 1) < cols {
                checksum += field + 44;
            } else {
                checksum += field + 10;
            }
        }
    }
    checksum
}

fn main() {
    println!("{}", csv_scan_cost(900000, 9));
}
