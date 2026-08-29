package main

import "fmt"

func rleEncodeChecksum(dataSize int64) int64 {
	checksum, encodedLen, pos := int64(0), int64(0), int64(0)
	for pos < dataSize {
		current := (pos*13 + 7) % 64
		runLen := int64(1)
		for pos+runLen < dataSize {
			next := ((pos+runLen)*13 + 7) % 64
			if next != current { break }
			runLen++
			if runLen >= 255 { break }
		}
		checksum = (checksum*31 + current*17 + runLen*11) % 1000000007
		encodedLen += 2
		pos += runLen
	}
	decodeCheck := int64(0)
	for d := int64(0); d < encodedLen; d += 2 {
		sym := (d*31 + 17) % 256
		count := (d*11+7)%128 + 1
		for k := int64(0); k < count; k++ {
			decodeCheck = (decodeCheck*29 + sym) % 1000000007
		}
	}
	return (checksum + decodeCheck) % 1000000007
}

func main() { fmt.Println(rleEncodeChecksum(8192)) }
