package main
import ("fmt"; "os"; "strconv")
func matmulScore(size int64) int64 {
	var total int64
	for row := int64(0); row < size; row++ {
		for col := int64(0); col < size; col++ {
			var cell int64
			for inner := int64(0); inner < size; inner++ {
				cell += ((row + inner) % 31) * ((inner + col) % 29)
			}
			total += cell
		}
	}
	return total
}
func main() {
	size := int64(96)
	if len(os.Args) > 1 { if v, err := strconv.ParseInt(os.Args[1], 10, 64); err == nil { size = v } }
	fmt.Println(matmulScore(size))
}
