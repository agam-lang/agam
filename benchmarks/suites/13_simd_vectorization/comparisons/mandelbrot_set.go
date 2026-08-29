package main

import "fmt"

func mandelbrotSet(size int64) int64 {
	maxIter := int64(100)
	checksum := int64(0)

	for py := int64(0); py < size; py++ {
		for px := int64(0); px < size; px++ {
			x0 := (px*3500)/size - 2500
			y0 := (py*2000)/size - 1000

			x, y, iter := int64(0), int64(0), int64(0)
			for x*x+y*y <= 4000000 && iter < maxIter {
				xtemp := (x*x-y*y)/1000 + x0
				y = (2*x*y)/1000 + y0
				x = xtemp
				iter++
			}
			checksum = (checksum*31 + iter) % 1000000007
		}
	}
	return checksum
}

func main() { fmt.Println(mandelbrotSet(256)) }
