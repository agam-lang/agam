package main

import "fmt"

func absVal(v int64) int64 {
	if v < 0 { return -v }
	return v
}

func astarPathfinding(gridSize int64) int64 {
	maxNodes := int64(10000)
	checksum := int64(0)

	for pathFinds := int64(0); pathFinds < 100; pathFinds++ {
		startX := (pathFinds * 17) % gridSize
		startY := (pathFinds * 19) % gridSize
		goalX := (gridSize - 1) - (pathFinds*23)%gridSize
		goalY := (gridSize - 1) - (pathFinds*29)%gridSize

		nodesExpanded := int64(0)
		currX, currY := startX, startY
		pathLen := int64(0)

		for (currX != goalX || currY != goalY) && nodesExpanded < maxNodes {
			nodesExpanded++
			bestNextX, bestNextY, bestF := currX, currY, int64(999999999)

			for dy := int64(-1); dy <= 1; dy++ {
				for dx := int64(-1); dx <= 1; dx++ {
					if dx != 0 || dy != 0 {
						nx, ny := currX+dx, currY+dy
						if nx >= 0 && nx < gridSize && ny >= 0 && ny < gridSize {
							isObstacle := ((nx*31+ny*37)%100 < 20)
							if !isObstacle {
								g := pathLen + 10
								if dx != 0 && dy != 0 { g += 4 }
								h := (absVal(nx-goalX) + absVal(ny-goalY)) * 10
								f := g + h
								if f < bestF {
									bestF = f
									bestNextX, bestNextY = nx, ny
								}
							}
						}
					}
				}
			}

			if bestNextX == currX && bestNextY == currY {
				nodesExpanded = maxNodes // stuck
			} else {
				currX, currY = bestNextX, bestNextY
				pathLen += 10
			}
		}
		checksum = (checksum*31 + pathLen + nodesExpanded) % 1000000007
	}
	return checksum
}

func main() { fmt.Println(astarPathfinding(100)) }
