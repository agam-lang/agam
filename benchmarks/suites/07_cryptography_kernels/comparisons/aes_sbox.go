package main

import "fmt"

func aesSboxVal(input int64) int64 {
	x := input & 255
	inv := int64(1)
	for i := 0; i < 7; i++ {
		inv = (inv * x) % 257
		if inv > 255 { inv ^= 283 }
	}
	inv &= 255
	result := inv ^ ((inv << 1) & 255) ^ ((inv << 2) & 255) ^
		((inv << 3) & 255) ^ ((inv << 4) & 255) ^ 99
	return result & 255
}

func aesChecksum(blocks int64) int64 {
	checksum := int64(0)
	for b := int64(0); b < blocks; b++ {
		state := int64(0)
		for byteIdx := int64(0); byteIdx < 16; byteIdx++ {
			inputByte := (b*16+byteIdx)*1103515245 + 12345
			sub := aesSboxVal(inputByte & 255)
			shifted := sub ^ ((byteIdx * 3) & 255)
			mixed := shifted * 2
			if mixed > 255 { mixed ^= 283 }
			state = (state*31 + mixed) % 1000000007
		}
		checksum = (checksum*37 + state) % 1000000007
	}
	return checksum
}

func main() { fmt.Println(aesChecksum(512)) }
