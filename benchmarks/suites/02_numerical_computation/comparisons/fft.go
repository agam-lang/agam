package main
import "fmt"
func butterflyCost(size int64) int64 {
	var total int64
	stage := int64(1)
	for stage < size {
		for i := int64(0); i < size; i += stage * 2 {
			left := ((i * 19) + stage) % 65521
			right := ((i * 23) + (stage * 3)) % 65521
			total += (left + right) - (left - right)
		}
		stage *= 2
	}
	return total
}
func main() { fmt.Println(butterflyCost(16384)) }
