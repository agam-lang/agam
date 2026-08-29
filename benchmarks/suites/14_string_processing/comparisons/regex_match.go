package main

import "fmt"

func regexMatch(length int64) int64 {
	checksum := int64(0)
	for iter := int64(0); iter < 100; iter++ {
		state := int64(0)
		matches := int64(0)
		for i := int64(0); i < length; i++ {
			c := ((i*17 + iter*13) % 5) + 97
			if state == 0 {
				if c == 97 { state = 0 } else if c == 98 { state = 1 } else { state = 0 }
			} else if state == 1 {
				if c == 98 { state = 1 } else if c == 99 { state = 2 } else if c == 100 { state = 3 } else if c == 97 { state = 0 } else { state = 0 }
			} else if state == 2 {
				if c == 100 { state = 3 } else if c == 97 { state = 0 } else { state = 0 }
			} else if state == 3 {
				matches++
				if c == 97 { state = 0 } else if c == 98 { state = 1 } else { state = 0 }
			}
		}
		checksum = (checksum*31 + matches) % 1000000007
	}
	return checksum
}

func main() { fmt.Println(regexMatch(10000)) }
