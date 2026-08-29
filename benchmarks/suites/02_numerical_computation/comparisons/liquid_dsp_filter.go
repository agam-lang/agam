package main

import "fmt"

func liquidDspFir(numSamples int64, taps int64) int64 {
	var accumulator int64 = 0
	for i := taps; i < numSamples; i++ {
		var sampleAcc int64 = 0
		for k := int64(0); k < taps; k++ {
			inputVal := ((i - k) * 37) % 1000
			coeff := (k * 13) % 256
			sampleAcc += inputVal * coeff
		}
		accumulator = (accumulator + sampleAcc) % 1000000007
	}
	return accumulator
}

func main() {
	res := liquidDspFir(50000, 32)
	fmt.Println(res)
}
