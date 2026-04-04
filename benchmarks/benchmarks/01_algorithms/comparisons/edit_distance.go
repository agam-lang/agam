package main

import "fmt"

func editDistanceCost(left int64, right int64) int64 {
	var total int64
	for row := int64(0); row < left; row++ {
		for col := int64(0); col < right; col++ {
			mismatch := ((row * 17) + (col * 13)) % 11
			if mismatch < 2 {
				total += 1
			} else {
				total += (row + col) % 3
			}
		}
	}
	return total
}

func main() {
	fmt.Println(editDistanceCost(2800, 2800))
}
