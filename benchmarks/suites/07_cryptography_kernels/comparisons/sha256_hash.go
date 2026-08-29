package main

import "fmt"

func rotr32(x, n int64) int64 {
	mask := int64(4294967295)
	xm := x & mask
	return ((xm >> n) | (xm << (32 - n))) & mask
}

func sha256Checksum(blocks int64) int64 {
	h0, h1, h2, h3 := int64(1779033703), int64(3144134277), int64(1013904242), int64(2773480762)
	h4, h5, h6, h7 := int64(1359893119), int64(2600822924), int64(528734635), int64(1541459225)
	mask := int64(4294967295)
	checksum := int64(0)

	for b := int64(0); b < blocks; b++ {
		a, bv, c, d := h0, h1, h2, h3
		e, f, g, hv := h4, h5, h6, h7
		for round := int64(0); round < 64; round++ {
			w := (b*64+round)*1103515245 + 12345
			s1 := rotr32(e, 6) ^ rotr32(e, 11) ^ rotr32(e, 25)
			ch := (e & f) ^ ((^e) & g)
			temp1 := (hv + s1 + (ch & mask) + (w & mask) + round*7) & mask
			s0 := rotr32(a, 2) ^ rotr32(a, 13) ^ rotr32(a, 22)
			maj := (a & bv) ^ (a & c) ^ (bv & c)
			temp2 := (s0 + (maj & mask)) & mask

			hv = g
			g = f
			f = e
			e = (d + temp1) & mask
			d = c
			c = bv
			bv = a
			a = (temp1 + temp2) & mask
		}
		checksum = (checksum*31 + a + bv + e + hv) % 1000000007
	}
	return checksum
}

func main() { fmt.Println(sha256Checksum(256)) }
