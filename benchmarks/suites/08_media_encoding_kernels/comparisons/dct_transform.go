package main

import "fmt"

func dctChecksum(blocks int64) int64 {
	checksum := int64(0)
	for b := int64(0); b < blocks; b++ {
		inputSum := int64(0)
		outputSum := int64(0)
		for u := int64(0); u < 8; u++ {
			for v := int64(0); v < 8; v++ {
				sum := int64(0)
				for x := int64(0); x < 8; x++ {
					for y := int64(0); y < 8; y++ {
						pixel := ((b*64+x*8+y)*17 + 13) % 256
						if u == 0 && v == 0 {
							inputSum = (inputSum + pixel) % 1000000007
						}
						cosX := ((2*x + 1) * u * 314159) / 1600000
						cosY := ((2*y + 1) * v * 314159) / 1600000
						cx := (1000 * (100 - (cosX*cosX)/20000)) / 100
						cy := (1000 * (100 - (cosY*cosY)/20000)) / 100
						sum += (pixel * cx * cy) / 1000000
					}
				}
				cu := int64(1000)
				if u == 0 { cu = 707 }
				cv := int64(1000)
				if v == 0 { cv = 707 }
				dctVal := (sum * cu * cv) / 4000000
				outputSum = (outputSum*31 + (dctVal + 10000)) % 1000000007
			}
		}
		checksum = (checksum*37 + inputSum + outputSum) % 1000000007
	}
	return checksum
}

func main() { fmt.Println(dctChecksum(64)) }
