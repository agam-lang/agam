package main

import "fmt"

func tokenFrequencyCost(tokens int64, buckets int64) int64 {
	var total int64
	for token := int64(0); token < tokens; token++ {
		bucket := ((token * 19) + (token / 7)) % buckets
		if bucket < 8 {
			total += (bucket * 7) + 3
		} else if bucket < 24 {
			total += (bucket * 3) + 1
		} else {
			total += bucket + 11
		}
	}
	return total
}

func main() {
	fmt.Println(tokenFrequencyCost(7000000, 64))
}
