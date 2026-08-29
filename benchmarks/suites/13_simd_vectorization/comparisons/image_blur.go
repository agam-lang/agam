package main

import "fmt"

func imageBlur(size int64) int64 {
	checksum := int64(0)
	for y := int64(1); y < size-1; y++ {
		for x := int64(1); x < size-1; x++ {
			p00 := (((y-1)*size+(x-1))*17 + 13) % 256
			p01 := (((y-1)*size+x)*17 + 13) % 256
			p02 := (((y-1)*size+(x+1))*17 + 13) % 256

			p10 := ((y*size+(x-1))*17 + 13) % 256
			p11 := ((y*size+x)*17 + 13) % 256
			p12 := ((y*size+(x+1))*17 + 13) % 256

			p20 := (((y+1)*size+(x-1))*17 + 13) % 256
			p21 := (((y+1)*size+x)*17 + 13) % 256
			p22 := (((y+1)*size+(x+1))*17 + 13) % 256

			sum := p00 + 2*p01 + p02 + 2*p10 + 4*p11 + 2*p12 + p20 + 2*p21 + p22
			blur := sum / 16
			checksum = (checksum*31 + blur) % 1000000007
		}
	}
	return checksum
}

func main() { fmt.Println(imageBlur(256)) }
