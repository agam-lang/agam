package main

import "fmt"

func absDiff(a, b int64) int64 {
	if a > b { return a - b }
	return b - a
}

func motionEstChecksum(frames int64) int64 {
	width, height, blockSize, searchRange := int64(64), int64(64), int64(8), int64(4)
	checksum := int64(0)
	for f := int64(0); f < frames; f++ {
		for by := int64(0); by < height/blockSize; by++ {
			for bx := int64(0); bx < width/blockSize; bx++ {
				bestSad := int64(999999999)
				bestDy, bestDx := int64(0), int64(0)
				for dy := -searchRange; dy <= searchRange; dy++ {
					for dx := -searchRange; dx <= searchRange; dx++ {
						sad := int64(0)
						for r := int64(0); r < blockSize; r++ {
							for c := int64(0); c < blockSize; c++ {
								cy := by*blockSize + r
								cx := bx*blockSize + c
								ry, rx := cy+dy, cx+dx
								curPixel := ((f*width*height+cy*width+cx)*17 + 13) % 256
								refPixel := int64(0)
								if ry >= 0 && ry < height && rx >= 0 && rx < width {
									refPixel = (((f-1)*width*height+ry*width+rx)*17 + 13) % 256
								}
								sad += absDiff(curPixel, refPixel)
							}
						}
						if sad < bestSad {
							bestSad = sad
							bestDy = dy
							bestDx = dx
						}
					}
				}
				checksum = (checksum*31 + bestSad + bestDy*17 + bestDx*7) % 1000000007
			}
		}
	}
	return checksum
}

func main() { fmt.Println(motionEstChecksum(8)) }
