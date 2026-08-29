package main

import "fmt"

func nonZeroDir(value int64) int64 {
	if value >= 0 {
		return value + 1
	}
	return value - 1
}

func bvhTraversal(rays int64) int64 {
	nodes := int64(256)
	checksum := int64(0)

	for r := int64(0); r < rays; r++ {
		ox := (r * 17) % 1000
		oy := (r * 31) % 1000
		dx := ((r * 13) % 20) - 10
		dy := ((r * 19) % 20) - 10
		dxDiv := nonZeroDir(dx)
		dyDiv := nonZeroDir(dy)

		stackSize := int64(0)
		currentNode := int64(0)
		hits := int64(0)

		for steps := 0; steps < 100; steps++ {
			minX := (currentNode * 7) % 1000
			maxX := minX + 100
			minY := (currentNode * 11) % 1000
			maxY := minY + 100

			tminX := (minX - ox) * 100 / dxDiv
			tmaxX := (maxX - ox) * 100 / dxDiv
			if tminX > tmaxX { tminX, tmaxX = tmaxX, tminX }

			tminY := (minY - oy) * 100 / dyDiv
			tmaxY := (maxY - oy) * 100 / dyDiv
			if tminY > tmaxY { tminY, tmaxY = tmaxY, tminY }

			tmin := tminX
			if tminY > tmin { tmin = tminY }
			tmax := tmaxX
			if tmaxY < tmax { tmax = tmaxY }

			if tmax >= tmin && tmax > 0 {
				isLeaf := (currentNode%3 == 0)
				if isLeaf {
					hits++
					stackSize--
					if stackSize < 0 { steps = 999 }
					currentNode = (currentNode * 2) % nodes
				} else {
					stackSize++
					currentNode = (currentNode*2 + 1) % nodes
				}
			} else {
				stackSize--
				if stackSize < 0 { steps = 999 }
				currentNode = (currentNode + 1) % nodes
			}
		}
		checksum = (checksum*31 + hits) % 1000000007
	}
	return checksum
}

func main() { fmt.Println(bvhTraversal(4096)) }
