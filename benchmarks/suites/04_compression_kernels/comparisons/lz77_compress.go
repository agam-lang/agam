package main

import "fmt"

func lz77CompressChecksum(dataSize, windowSize int64) int64 {
	checksum, pos := int64(0), int64(0)
	for pos < dataSize {
		bestLen, bestDist := int64(0), int64(0)
		searchStart := pos - windowSize
		if searchStart < 0 { searchStart = 0 }
		for s := searchStart; s < pos; s++ {
			matchLen := int64(0)
			for pos+matchLen < dataSize {
				a := (s+matchLen)*7 + 13
				b := (pos+matchLen)*7 + 13
				if a%256 != b%256 { break }
				matchLen++
				if matchLen > 15 { break }
			}
			if matchLen > bestLen { bestLen = matchLen; bestDist = pos - s }
		}
		if bestLen >= 3 {
			checksum = (checksum*31 + bestDist*17 + bestLen*13) % 1000000007
			pos += bestLen
		} else {
			literal := pos*7 + 13
			checksum = (checksum*31 + literal%256) % 1000000007
			pos++
		}
	}
	return checksum
}

func main() { fmt.Println(lz77CompressChecksum(512, 32)) }
