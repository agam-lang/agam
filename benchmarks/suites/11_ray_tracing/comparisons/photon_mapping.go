package main

import "fmt"

func distanceSq(x1, y1, z1, x2, y2, z2 int64) int64 {
	dx := x2 - x1
	dy := y2 - y1
	dz := z2 - z1
	return dx*dx + dy*dy + dz*dz
}

func photonMapping(photons int64) int64 {
	queries := int64(1024)
	checksum := int64(0)

	for q := int64(0); q < queries; q++ {
		qx := (q * 17) % 1000
		qy := (q * 19) % 1000
		qz := (q * 23) % 1000

		gathered := int64(0)
		radiusSq := int64(10000)

		for p := int64(0); p < photons; p++ {
			px := (p * 31) % 1000
			py := (p * 37) % 1000
			pz := (p * 41) % 1000

			d2 := distanceSq(qx, qy, qz, px, py, pz)
			if d2 < radiusSq {
				power := 1000 - (d2*1000)/radiusSq
				gathered += power
			}
		}
		checksum = (checksum*31 + gathered) % 1000000007
	}
	return checksum
}

func main() { fmt.Println(photonMapping(4096)) }
