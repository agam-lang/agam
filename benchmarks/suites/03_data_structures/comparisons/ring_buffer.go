package main
import "fmt"
func ringBufferCost(cap, rounds int64) int64 {
	head, tail, acc := int64(0), int64(0), int64(0)
	for item := int64(0); item < rounds; item++ {
		slot := (head + item) % cap
		acc += ((slot * 17) + item) % 257
		if item%3 == 0 { tail = (tail + 1) % cap; acc += tail }
		head = (head + 1) % cap
	}
	return acc + head + tail
}
func main() { fmt.Println(ringBufferCost(4096, 12000000)) }
