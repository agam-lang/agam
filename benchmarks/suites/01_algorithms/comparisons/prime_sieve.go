package main

import (
	"fmt"
	"math"
)

func isPrime(n int64) bool {
	if n < 2 {
		return false
	}
	limit := int64(math.Sqrt(float64(n)))
	for d := int64(2); d <= limit; d++ {
		if n%d == 0 {
			return false
		}
	}
	return true
}

func primeCount(limit int64) int64 {
	var total int64
	for n := int64(2); n <= limit; n++ {
		if isPrime(n) {
			total++
		}
	}
	return total
}

func main() {
	fmt.Println(primeCount(25000))
}
