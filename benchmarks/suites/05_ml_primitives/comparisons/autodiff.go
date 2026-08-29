package main
import "fmt"
func autodiffTrace(steps int64) int64 {
	value, grad := int64(7), int64(1)
	for s := int64(0); s < steps; s++ { grad += (value * 3) % 17; value += grad }
	return value + grad
}
func main() { fmt.Println(autodiffTrace(5000000)) }
