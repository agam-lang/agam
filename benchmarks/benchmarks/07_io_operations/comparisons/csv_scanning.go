package main

import "fmt"

func csvScanCost(rows int64, cols int64) int64 {
	var checksum int64
	for row := int64(0); row < rows; row++ {
		for col := int64(0); col < cols; col++ {
			field := ((row * 37) + (col * 13)) % 1009
			if (col + 1) < cols {
				checksum += field + 44
			} else {
				checksum += field + 10
			}
		}
	}
	return checksum
}

func main() {
	fmt.Println(csvScanCost(900000, 9))
}
