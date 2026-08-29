package main

import "fmt"

func partitionCost(low, high, pivot int64) int64 {
	var score int64
	for i := low; i < high; i++ {
		probe := ((i * 17) + 13) % 997
		if probe < pivot {
			score += probe
		} else {
			score -= probe
		}
	}
	return score
}

func quicksortCost(low, high int64) int64 {
	if high-low < 2 {
		return high - low
	}
	pivot := (low + high) / 2
	return partitionCost(low, high, pivot) +
		quicksortCost(low, pivot) +
		quicksortCost(pivot+1, high)
}

func main() {
	fmt.Println(quicksortCost(0, 5000))
}
