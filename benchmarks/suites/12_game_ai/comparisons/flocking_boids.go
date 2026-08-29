package main

import "fmt"

func flockingBoids(boids int64) int64 {
	checksum := int64(0)
	frames := int64(30)

	for f := int64(0); f < frames; f++ {
		centerMassSum := int64(0)
		for i := int64(0); i < boids; i++ {
			px := (i*17 + f*7) % 1000
			py := (i*19 + f*11) % 1000
			vx := ((i * 23) % 20) - 10
			vy := ((i * 29) % 20) - 10

			sepX, sepY, alignX, alignY, cohX, cohY := int64(0), int64(0), int64(0), int64(0), int64(0), int64(0)
			neighbors := int64(0)

			for j := int64(0); j < boids; j++ {
				if i != j {
					ox := (j*17 + f*7) % 1000
					oy := (j*19 + f*11) % 1000
					ovx := ((j * 23) % 20) - 10
					ovy := ((j * 29) % 20) - 10

					dx, dy := px-ox, py-oy
					distSq := dx*dx + dy*dy

					if distSq < 2500 {
						neighbors++
						if distSq < 400 { sepX += dx; sepY += dy }
						alignX += ovx; alignY += ovy
						cohX += ox; cohY += oy
					}
				}
			}

			newVx, newVy := vx, vy
			if neighbors > 0 {
				alignX = (alignX / neighbors) - vx
				alignY = (alignY / neighbors) - vy
				cohX = (cohX / neighbors) - px
				cohY = (cohY / neighbors) - py

				newVx = vx + (sepX*5+alignX*2+cohX*1)/100
				newVy = vy + (sepY*5+alignY*2+cohY*1)/100
			}
			centerMassSum = (centerMassSum + newVx + newVy) % 1000000007
		}
		checksum = (checksum*31 + centerMassSum) % 1000000007
	}
	return checksum
}

func main() { fmt.Println(flockingBoids(512)) }
