fn valkey_pipeline_ops(num_ops: i64) -> i64 {
    let mut checksum: i64 = 0;
    let mut slot0: i64 = 0;
    let mut slot1: i64 = 0;
    let mut slot2: i64 = 0;
    let mut slot3: i64 = 0;

    for op in 0..num_ops {
        let key = ((op * 100003) / 65536) % 4;
        let op_type = op % 3;

        if op_type == 0 {
            if key == 0 { slot0 = op; }
            if key == 1 { slot1 = op; }
            if key == 2 { slot2 = op; }
            if key == 3 { slot3 = op; }
        }
        if op_type == 1 {
            let mut val = 0;
            if key == 0 { val = slot0; }
            if key == 1 { val = slot1; }
            if key == 2 { val = slot2; }
            if key == 3 { val = slot3; }
            checksum += val;
        }
        if op_type == 2 {
            if key == 0 { slot0 += 1; checksum += slot0; }
            if key == 1 { slot1 += 1; checksum += slot1; }
            if key == 2 { slot2 += 1; checksum += slot2; }
            if key == 3 { slot3 += 1; checksum += slot3; }
        }
    }
    checksum % 1000000007
}

fn main() {
    let res = valkey_pipeline_ops(100000);
    println!("{}", res);
}
