package main
import "fmt"
func branchWalk(depth, fanout int64) int64 {
	if depth == 0 { return fanout }
	var total int64
	for c := int64(0); c < fanout; c++ { total += branchWalk(depth-1, fanout-1) + c }
	return total
}
func main() { fmt.Println(branchWalk(5, 6)) }
