package main

import "fmt"

func base64Encode(length int64) int64 {
	checksum := int64(0)
	for iter := int64(0); iter < 100; iter++ {
		for i := int64(0); i < length; i += 3 {
			b1 := (i*17 + iter*13) % 256
			b2 := ((i+1)*19 + iter*7) % 256
			b3 := ((i+2)*23 + iter*11) % 256

			enc1 := (b1 >> 2) & 63
			enc2 := ((b1 & 3) << 4) | ((b2 >> 4) & 15)
			enc3 := ((b2 & 15) << 2) | ((b3 >> 6) & 3)
			enc4 := b3 & 63

			checksum = (checksum*31 + enc1) % 1000000007
			checksum = (checksum*31 + enc2) % 1000000007
			checksum = (checksum*31 + enc3) % 1000000007
			checksum = (checksum*31 + enc4) % 1000000007
		}
	}
	return checksum
}

func main() { fmt.Println(base64Encode(10000)) }
