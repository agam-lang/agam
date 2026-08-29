package main
import "fmt"
func hashmapProbeCost(slots, rounds int64) int64 {
	var total int64
	for r := int64(0); r < rounds; r++ {
		key := (r * 2654435761) % 2147483647
		total += ((key % slots) * 37) % 4099
	}
	return total
}
func main() { fmt.Println(hashmapProbeCost(65536, 5000000)) }
