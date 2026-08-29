package main

import "fmt"

func clamp(val int64) int64 {
	if val < 0 { return 0 }
	if val > 255 { return 255 }
	return val
}

func pixelFilterChecksum(width, height int64) int64 {
	checksum := int64(0)
	for y := int64(1); y < height-1; y++ {
		for x := int64(1); x < width-1; x++ {
			p00 := (((y-1)*width+(x-1))*17 + 13) % 256
			p01 := (((y-1)*width+x)*17 + 13) % 256
			p02 := (((y-1)*width+(x+1))*17 + 13) % 256
			p10 := ((y*width+(x-1))*17 + 13) % 256
			p11 := ((y*width+x)*17 + 13) % 256
			p12 := ((y*width+(x+1))*17 + 13) % 256
			p20 := (((y+1)*width+(x-1))*17 + 13) % 256
			p21 := (((y+1)*width+x)*17 + 13) % 256
			p22 := (((y+1)*width+(x+1))*17 + 13) % 256

			blur := (p00 + 2*p01 + p02 + 2*p10 + 4*p11 + 2*p12 + p20 + 2*p21 + p22) / 16
			sobelX := -p00 + p02 - 2*p10 + 2*p12 - p20 + p22
			sobelY := -p00 - 2*p01 - p02 + p20 + 2*p21 + p22
			sobel := sobelX*sobelX + sobelY*sobelY
			edge := sobel / 100

			out := clamp(blur + edge)
			checksum = (checksum*31 + out) % 1000000007
		}
	}
	return checksum
}

func main() { fmt.Println(pixelFilterChecksum(256, 256)) }
