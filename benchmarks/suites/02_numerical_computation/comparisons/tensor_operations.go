package main
import "fmt"
func tensorChecksum(w, h, d int64) int64 {
	var total int64
	for z := int64(0); z < d; z++ {
		for y := int64(0); y < h; y++ {
			for x := int64(0); x < w; x++ {
				total += ((x * 31) + (y * 17) + (z * 13)) % 4093
			}
		}
	}
	return total
}
func main() { fmt.Println(tensorChecksum(96, 96, 16)) }
