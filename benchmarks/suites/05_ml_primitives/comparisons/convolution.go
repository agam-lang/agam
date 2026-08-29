package main
import "fmt"
func convScore(w, h int64) int64 {
	var total int64
	for y := int64(1); y < h-1; y++ {
		for x := int64(1); x < w-1; x++ { total += ((x * 3) + (y * 5)) % 97 }
	}
	return total
}
func main() { fmt.Println(convScore(512, 512)) }
