package main
import "fmt"
func monteCarlo(samples int64) int64 {
	var inside, seed int64 = 0, 123456789
	for i := int64(0); i < samples; i++ {
		seed = ((seed * 1103515245) + 12345) % 2147483647
		x := seed % 10000
		seed = ((seed * 1103515245) + 12345) % 2147483647
		y := seed % 10000
		if (x*x)+(y*y) <= 100000000 { inside++ }
	}
	return (inside * 4000000) / samples
}
func main() { fmt.Println(monteCarlo(500000)) }
