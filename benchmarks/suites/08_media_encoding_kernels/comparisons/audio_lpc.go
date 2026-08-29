package main

import "fmt"

func audioLpcChecksum(frames int64) int64 {
	order, frameSize := int64(10), int64(256)
	checksum := int64(0)
	for f := int64(0); f < frames; f++ {
		maxR := int64(0)
		for lag := int64(0); lag <= order; lag++ {
			r := int64(0)
			for i := int64(0); i < frameSize-lag; i++ {
				s1 := ((f*frameSize+i)*17+13)%65536 - 32768
				s2 := ((f*frameSize+i+lag)*17+13)%65536 - 32768
				r += (s1 * s2) / 32768
			}
			if r < 0 { r = -r }
			if r > maxR { maxR = r }
			checksum = (checksum*31 + r) % 1000000007
		}
		residual := int64(0)
		for i_r := order; i_r < frameSize; i_r++ {
			pred := int64(0)
			for j := int64(1); j <= order; j++ {
				s := ((f*frameSize+i_r-j)*17+13)%65536 - 32768
				coef := (j * 17) % 100
				pred += (s * coef) / 100
			}
			actual := ((f*frameSize+i_r)*17+13)%65536 - 32768
			diff := actual - pred
			if diff < 0 { diff = -diff }
			residual += diff
		}
		checksum = (checksum*37 + residual + maxR) % 1000000007
	}
	return checksum
}

func main() { fmt.Println(audioLpcChecksum(32)) }
