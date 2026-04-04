package main

import "fmt"

func polynomialCost(points int64, degree int64) int64 {
	var checksum int64
	for point := int64(0); point < points; point++ {
		x := (point % 97) + 3
		value := int64(1)
		for coeff := degree; coeff > 0; coeff-- {
			value = ((value * x) + ((coeff * 11) + (point % 29))) % 1000003
		}
		checksum += value
	}
	return checksum
}

func main() {
	fmt.Println(polynomialCost(800000, 16))
}
