package main

import "fmt"

func nbodySimulation(bodies int64) int64 {
	steps := int64(10)
	checksum := int64(0)

	for step := int64(0); step < steps; step++ {
		for i := int64(0); i < bodies; i++ {
			px := ((i*17 + step) % 2000) - 1000
			py := ((i*19 + step) % 2000) - 1000
			pz := ((i*23 + step) % 2000) - 1000

			fx, fy, fz := int64(0), int64(0), int64(0)
			for j := int64(0); j < bodies; j++ {
				if i != j {
					ox := ((j*17 + step) % 2000) - 1000
					oy := ((j*19 + step) % 2000) - 1000
					oz := ((j*23 + step) % 2000) - 1000

					dx, dy, dz := ox-px, oy-py, oz-pz
					dSq := dx*dx + dy*dy + dz*dz
					if dSq == 0 { dSq = 1 }

					invDistCubed := 1000000 / (dSq * dSq)
					fx += dx * invDistCubed
					fy += dy * invDistCubed
					fz += dz * invDistCubed
				}
			}
			checksum = (checksum*31 + fx + fy + fz) % 1000000007
		}
	}
	return checksum
}

func main() { fmt.Println(nbodySimulation(256)) }
