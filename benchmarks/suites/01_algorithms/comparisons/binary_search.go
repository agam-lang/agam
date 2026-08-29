package main

import "fmt"

func binarySearch(limit, target int64) int64 {
	low := int64(0)
	high := limit
	for low < high {
		mid := low + ((high - low) / 2)
		value := (mid * 3) + 7
		if value == target {
			return mid
		}
		if value < target {
			low = mid + 1
		} else {
			high = mid
		}
	}
	return -1
}

func main() {
	fmt.Println(binarySearch(10000000, 2999998))
}
