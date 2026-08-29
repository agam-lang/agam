package main

import "fmt"

func htmlEscape(length int64) int64 {
	checksum := int64(0)
	for iter := int64(0); iter < 100; iter++ {
		outLen := int64(0)
		for i := int64(0); i < length; i++ {
			c := (i*23 + iter*11) % 10
			if c == 0 { outLen += 4; checksum = (checksum*31 + 60) % 1000000007 } else if c == 1 { outLen += 4; checksum = (checksum*31 + 62) % 1000000007 } else if c == 2 { outLen += 5; checksum = (checksum*31 + 38) % 1000000007 } else if c == 3 { outLen += 6; checksum = (checksum*31 + 34) % 1000000007 } else if c == 4 { outLen += 5; checksum = (checksum*31 + 39) % 1000000007 } else { outLen += 1; checksum = (checksum*31 + 97 + c) % 1000000007 }
		}
		checksum = (checksum*37 + outLen) % 1000000007
	}
	return checksum
}

func main() { fmt.Println(htmlEscape(10000)) }
