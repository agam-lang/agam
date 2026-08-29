package main
import "fmt"
func softmaxLike(width, rounds int64) int64 {
	var total int64
	for r := int64(0); r < rounds; r++ {
		var partial int64
		for l := int64(0); l < width; l++ { partial += ((l * 11) + r) % 251 }
		total += partial
	}
	return total
}
func main() { fmt.Println(softmaxLike(4096, 4000)) }
