package main
import "fmt"
func polynomialCost(points, degree int64) int64 {
	var checksum int64
	for p := int64(0); p < points; p++ {
		x := (p % 97) + 3
		value := int64(1)
		for c := degree; c > 0; c-- {
			value = ((value * x) + ((c * 11) + (p % 29))) % 1000003
		}
		checksum += value
	}
	return checksum
}
func main() { fmt.Println(polynomialCost(800000, 16)) }
