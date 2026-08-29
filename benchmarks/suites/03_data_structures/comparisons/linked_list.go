package main
import "fmt"
func pointerChase(nodes, rounds int64) int64 {
	cursor, checksum := int64(1), int64(0)
	for s := int64(0); s < rounds; s++ {
		cursor = ((cursor * 1103515245) + 12345) % nodes
		checksum += cursor
	}
	return checksum
}
func main() { fmt.Println(pointerChase(1000003, 8000000)) }
