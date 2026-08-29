package main

import "fmt"

func zbufferRasterize(triangles int64) int64 {
	width, height := int64(256), int64(256)
	checksum := int64(0)

	for t := int64(0); t < triangles; t++ {
		x0, y0, z0 := (t*17)%width, (t*19)%height, (t*23)%1000
		x1, y1, z1 := (x0+50)%width, (y0+20)%height, (t*29)%1000
		x2, y2, z2 := (x0+20)%width, (y0+50)%height, (t*31)%1000

		minX := x0
		if x1 < minX { minX = x1 }
		if x2 < minX { minX = x2 }
		maxX := x0
		if x1 > maxX { maxX = x1 }
		if x2 > maxX { maxX = x2 }
		minY := y0
		if y1 < minY { minY = y1 }
		if y2 < minY { minY = y2 }
		maxY := y0
		if y1 > maxY { maxY = y1 }
		if y2 > maxY { maxY = y2 }

		area := (x1-x0)*(y2-y0) - (x2-x0)*(y1-y0)
		if area == 0 { area = 1 }
		if area < 0 { area = -area }

		pixelsDrawn := int64(0)
		for py := minY; py <= maxY; py++ {
			for px := minX; px <= maxX; px++ {
				w0 := (x1-px)*(y2-py) - (x2-px)*(y1-py)
				w1 := (x2-px)*(y0-py) - (x0-px)*(y2-py)
				w2 := (x0-px)*(y1-py) - (x1-px)*(y0-py)

				isInside := false
				if w0 >= 0 && w1 >= 0 && w2 >= 0 { isInside = true }
				if w0 <= 0 && w1 <= 0 && w2 <= 0 { isInside = true }

				if isInside {
					z := (w0*z0 + w1*z1 + w2*z2) / area
					pixelsDrawn++
					checksum = (checksum*31 + z + px + py) % 1000000007
				}
			}
		}
		checksum = (checksum*37 + pixelsDrawn) % 1000000007
	}
	return checksum
}

func main() { fmt.Println(zbufferRasterize(1024)) }
