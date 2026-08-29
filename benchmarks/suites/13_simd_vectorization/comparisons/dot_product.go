package main

import "fmt"

func dotProduct(size int64) int64 {
	sum := int64(0)
	for i := int64(0); i < size; i++ {
		a := ((i*17 + 13) % 256) - 128
		b := ((i*19 + 7) % 256) - 128
		sum += a * b
	}
	return sum
}

func main() { fmt.Println(dotProduct(65536)) }
