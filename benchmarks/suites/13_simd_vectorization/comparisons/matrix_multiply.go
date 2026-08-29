package main

import "fmt"

func matrixMultiply(size int64) int64 {
	checksum := int64(0)
	s := size
	for i := int64(0); i < s; i++ {
		for j := int64(0); j < s; j++ {
			sum := int64(0)
			for k := int64(0); k < s; k++ {
				a := ((i*s+k)*17 + 13) % 256
				b := ((k*s+j)*19 + 7) % 256
				sum += a * b
			}
			checksum = (checksum*31 + sum) % 1000000007
		}
	}
	return checksum
}

func main() { fmt.Println(matrixMultiply(64)) }
