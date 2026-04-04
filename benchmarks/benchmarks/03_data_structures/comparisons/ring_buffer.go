package main

import "fmt"

func ringBufferCost(capacity int64, rounds int64) int64 {
	var head int64
	var tail int64
	var acc int64
	for item := int64(0); item < rounds; item++ {
		slot := (head + item) % capacity
		acc += ((slot * 17) + item) % 257
		if (item % 3) == 0 {
			tail = (tail + 1) % capacity
			acc += tail
		}
		head = (head + 1) % capacity
	}
	return acc + head + tail
}

func main() {
	fmt.Println(ringBufferCost(4096, 12000000))
}
