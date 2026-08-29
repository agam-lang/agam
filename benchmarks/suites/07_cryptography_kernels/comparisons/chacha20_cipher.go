package main

import "fmt"

func chachaQuarter(a, b, c, d int64) int64 {
	mask := int64(4294967295)
	av := (a + b) & mask
	dv := ((d ^ av) << 16 | ((d ^ av) & mask) >> 16) & mask
	cv := (c + dv) & mask
	bv := ((b ^ cv) << 12 | ((b ^ cv) & mask) >> 20) & mask
	av2 := (av + bv) & mask
	dv2 := ((dv ^ av2) << 8 | ((dv ^ av2) & mask) >> 24) & mask
	cv2 := (cv + dv2) & mask
	bv2 := ((bv ^ cv2) << 7 | ((bv ^ cv2) & mask) >> 25) & mask
	return (av2 + bv2 + cv2 + dv2) & mask
}

func chacha20Checksum(rounds int64) int64 {
	checksum := int64(0)
	for i := int64(0); i < rounds; i++ {
		s0, s1, s2, s3 := int64(1634760805), int64(857760878), int64(2036477234), int64(1797285236)
		for r := int64(0); r < 10; r++ {
			q1 := chachaQuarter(s0, s1, s2, s3)
			q2 := chachaQuarter(s1+i, s2+r, s3, s0)
			q3 := chachaQuarter(s2, s3+i, s0+r, s1)
			q4 := chachaQuarter(s3, s0, s1+i, s2+r)
			s0, s1, s2, s3 = q1, q2, q3, q4
		}
		checksum = (checksum*37 + s0 + s1 + s2 + s3) % 1000000007
	}
	return checksum
}

func main() { fmt.Println(chacha20Checksum(1024)) }
