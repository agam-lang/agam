package main

import "fmt"

func raySphereIntersect(rays int64) int64 {
	checksum := int64(0)
	sx, sy, sz, radius := int64(500), int64(500), int64(1000), int64(300)
	r2 := radius * radius

	for r := int64(0); r < rays; r++ {
		ox := ((r * 17) % 2000) - 1000
		oy := ((r * 31) % 2000) - 1000
		oz := int64(-1000)

		dx := ((r * 13) % 100) - 50
		dy := ((r * 19) % 100) - 50
		dz := int64(100)

		invLen := 10000 / (dx*dx + dy*dy + dz*dz + 1)
		dx = (dx * invLen) / 100
		dy = (dy * invLen) / 100
		dz = (dz * invLen) / 100

		ocx := ox - sx
		ocy := oy - sy
		ocz := oz - sz

		b := 2 * (ocx*dx + ocy*dy + ocz*dz)
		c := (ocx*ocx + ocy*ocy + ocz*ocz) - r2
		discriminant := (b * b) - (4 * c)

		hit := false
		if discriminant >= 0 {
			t1 := (-b - discriminant/100) / 2
			t2 := (-b + discriminant/100) / 2
			if t1 > 0 || t2 > 0 {
				hit = true
				checksum = (checksum*31 + t1 + t2) % 1000000007
			}
		}
		if !hit {
			checksum = (checksum*37 + 1) % 1000000007
		}
	}
	return checksum
}

func main() { fmt.Println(raySphereIntersect(8192)) }
