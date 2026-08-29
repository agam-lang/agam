package main
import "fmt"
func matrixChecksum(size int64) int64 {
	var sum int64
	for row := int64(0); row < size; row++ {
		for col := int64(0); col < size; col++ {
			var cell int64
			for inner := int64(0); inner < size; inner++ {
				cell += ((row * inner) + 3) * ((inner * col) + 5)
			}
			sum += cell % 104729
		}
	}
	return sum
}
func main() { fmt.Println(matrixChecksum(64)) }
