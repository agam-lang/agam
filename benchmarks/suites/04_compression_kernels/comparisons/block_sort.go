package main

import "fmt"

func blockSortChecksum(size int64) int64 {
	checksum := int64(0)
	for i := int64(0); i < size; i++ {
		rotChecksum := int64(0)
		for r := int64(0); r < size; r++ {
			idx := (i + r) % size
			val := (idx*7 + 13) % 256
			rotChecksum = (rotChecksum*31 + val) % 1000000007
		}
		checksum = (checksum + rotChecksum) % 1000000007
	}
	sortedCheck := int64(0)
	for j := int64(0); j < size; j++ {
		lastCol := ((j+size-1)%size*7 + 13) % 256
		sortedCheck = (sortedCheck*37 + lastCol*(j+1)) % 1000000007
	}
	return (checksum + sortedCheck) % 1000000007
}

func main() { fmt.Println(blockSortChecksum(256)) }
