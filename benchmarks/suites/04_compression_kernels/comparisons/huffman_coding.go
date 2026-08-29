package main

import "fmt"

func huffmanChecksum(dataSize int64) int64 {
	checksum, totalBits := int64(0), int64(0)
	for j := int64(0); j < dataSize; j++ {
		byteVal := (j*7 + 13) % 256
		depth, val := int64(1), byteVal
		for val > 1 { val /= 2; depth++ }
		codeLen := depth
		if codeLen < 2 { codeLen = 2 }
		if codeLen > 12 { codeLen = 12 }
		totalBits += codeLen
		code := byteVal % (int64(1) << codeLen)
		checksum = (checksum*37 + code*19 + codeLen*7) % 1000000007
	}
	return (checksum*53 + totalBits) % 1000000007
}

func main() { fmt.Println(huffmanChecksum(4096)) }
