package main

import "fmt"

func collisionDetection(entities int64) int64 {
	checksum := int64(0)
	frames := int64(60)

	for f := int64(0); f < frames; f++ {
		collisions := int64(0)
		for i := int64(0); i < entities; i++ {
			x1 := ((i*17 + f*13) % 1000) - 500
			y1 := ((i*19 + f*11) % 1000) - 500
			r1 := ((i * 23) % 40) + 10

			for j := i + 1; j < entities; j++ {
				x2 := ((j*17 + f*13) % 1000) - 500
				y2 := ((j*19 + f*11) % 1000) - 500
				r2 := ((j * 23) % 40) + 10

				dx, dy := x2-x1, y2-y1
				distSq := dx*dx + dy*dy
				radSum := r1 + r2
				if distSq <= radSum*radSum {
					collisions++
					checksum = (checksum*31 + i + j) % 1000000007
				}
			}
		}
		checksum = (checksum*37 + collisions) % 1000000007
	}
	return checksum
}

func main() { fmt.Println(collisionDetection(512)) }
