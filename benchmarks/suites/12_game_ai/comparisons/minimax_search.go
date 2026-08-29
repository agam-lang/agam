package main

import "fmt"

func evaluateBoard(state int64) int64 {
	score := int64(0)
	for i := int64(0); i < 9; i++ {
		piece := (state >> (i * 2)) & 3
		if piece == 1 { score += 10 }
		if piece == 2 { score -= 10 }
	}
	return score
}

func minimax(state, depth int64, isMax bool) int64 {
	if depth == 0 { return evaluateBoard(state) }

	if isMax {
		bestVal := int64(-999999)
		for i := int64(0); i < 9; i++ {
			if ((state >> (i * 2)) & 3) == 0 {
				nextState := state | (1 << (i * 2))
				val := minimax(nextState, depth-1, false)
				if val > bestVal { bestVal = val }
			}
		}
		if bestVal == -999999 { return evaluateBoard(state) }
		return bestVal
	} else {
		bestVal := int64(999999)
		for i := int64(0); i < 9; i++ {
			if ((state >> (i * 2)) & 3) == 0 {
				nextState := state | (2 << (i * 2))
				val := minimax(nextState, depth-1, true)
				if val < bestVal { bestVal = val }
			}
		}
		if bestVal == 999999 { return evaluateBoard(state) }
		return bestVal
	}
}

func minimaxSearch(games int64) int64 {
	checksum := int64(0)
	for g := int64(0); g < games; g++ {
		initialState := (g * 12345) % 262144
		result := minimax(initialState, 6, true)
		checksum = (checksum*31 + result) % 1000000007
	}
	return checksum
}

func main() { fmt.Println(minimaxSearch(64)) }
