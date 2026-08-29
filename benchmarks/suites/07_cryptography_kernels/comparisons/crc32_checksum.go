package main

import "fmt"

func crc32Checksum(dataSize int64) int64 {
	crc := int64(4294967295)
	mask := int64(4294967295)
	poly := int64(3988292384)
	for i := int64(0); i < dataSize; i++ {
		byteVal := (i*1103515245 + 12345) & 255
		crc ^= byteVal
		for bit := 0; bit < 8; bit++ {
			if (crc & 1) != 0 {
				crc = ((crc >> 1) & (mask >> 1)) ^ poly
			} else {
				crc = (crc >> 1) & (mask >> 1)
			}
		}
	}
	return (crc ^ mask) % 1000000007
}

func main() { fmt.Println(crc32Checksum(16384)) }
