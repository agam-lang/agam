//! Audited, language-native cryptographic primitives (SHA-256, HMAC-SHA256, ChaCha20).

// ── SHA-256 Implementation (FIPS 180-4) ──

const K256: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

#[derive(Clone, Debug)]
pub struct Sha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    buffer_len: usize,
    total_len: u64,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    pub fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buffer: [0u8; 64],
            buffer_len: 0,
            total_len: 0,
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.total_len += data.len() as u64;
        let mut input = data;

        if self.buffer_len > 0 {
            let needed = 64 - self.buffer_len;
            if input.len() >= needed {
                self.buffer[self.buffer_len..64].copy_from_slice(&input[..needed]);
                let block = self.buffer;
                self.process_block(&block);
                self.buffer_len = 0;
                input = &input[needed..];
            } else {
                self.buffer[self.buffer_len..self.buffer_len + input.len()].copy_from_slice(input);
                self.buffer_len += input.len();
                return;
            }
        }

        while input.len() >= 64 {
            let mut block = [0u8; 64];
            block.copy_from_slice(&input[..64]);
            self.process_block(&block);
            input = &input[64..];
        }

        if !input.is_empty() {
            self.buffer[..input.len()].copy_from_slice(input);
            self.buffer_len = input.len();
        }
    }

    pub fn finalize(mut self) -> [u8; 32] {
        let total_bits = self.total_len * 8;
        // 1. Append bit 1 (0x80)
        self.buffer[self.buffer_len] = 0x80;
        self.buffer_len += 1;

        // 2. Pad with zeros
        if self.buffer_len > 56 {
            self.buffer[self.buffer_len..64].fill(0);
            let block = self.buffer;
            self.process_block(&block);
            self.buffer.fill(0);
        } else {
            self.buffer[self.buffer_len..56].fill(0);
        }

        // 3. Append 64-bit length in big-endian
        self.buffer[56..64].copy_from_slice(&total_bits.to_be_bytes());
        let block = self.buffer;
        self.process_block(&block);

        // 4. Output digest
        let mut out = [0u8; 32];
        for (i, word) in self.state.iter().enumerate() {
            out[i * 4..(i + 1) * 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    fn process_block(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];
        let mut e = self.state[4];
        let mut f = self.state[5];
        let mut g = self.state[6];
        let mut h = self.state[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K256[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }
}

/// Compute SHA-256 hash of a byte slice.
pub fn sha256_digest(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize()
}

/// Compute HMAC-SHA256 of data using secret key (RFC 2104).
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut key_pad = [0u8; 64];
    if key.len() > 64 {
        let digest = sha256_digest(key);
        key_pad[..32].copy_from_slice(&digest);
    } else {
        key_pad[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for i in 0..64 {
        ipad[i] ^= key_pad[i];
        opad[i] ^= key_pad[i];
    }

    let mut inner = Sha256::new();
    inner.update(&ipad);
    inner.update(data);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(&opad);
    outer.update(&inner_hash);
    outer.finalize()
}

// ── ChaCha20 Stream Cipher (RFC 8439) ──

fn chacha20_quarter_round(a: &mut u32, b: &mut u32, c: &mut u32, d: &mut u32) {
    *a = a.wrapping_add(*b);
    *d = (*d ^ *a).rotate_left(16);
    *c = c.wrapping_add(*d);
    *b = (*b ^ *c).rotate_left(12);
    *a = a.wrapping_add(*b);
    *d = (*d ^ *a).rotate_left(8);
    *c = c.wrapping_add(*d);
    *b = (*b ^ *c).rotate_left(7);
}

/// Encrypt or decrypt data in-place using ChaCha20 (RFC 8439).
pub fn chacha20_xor(key: &[u8; 32], nonce: &[u8; 12], counter: u32, data: &mut [u8]) {
    let constants = [0x61707865, 0x3320646e, 0x79622d32, 0x6b206574]; // "expand 32-byte k"

    let key_words = [
        u32::from_le_bytes([key[0], key[1], key[2], key[3]]),
        u32::from_le_bytes([key[4], key[5], key[6], key[7]]),
        u32::from_le_bytes([key[8], key[9], key[10], key[11]]),
        u32::from_le_bytes([key[12], key[13], key[14], key[15]]),
        u32::from_le_bytes([key[16], key[17], key[18], key[19]]),
        u32::from_le_bytes([key[20], key[21], key[22], key[23]]),
        u32::from_le_bytes([key[24], key[25], key[26], key[27]]),
        u32::from_le_bytes([key[28], key[29], key[30], key[31]]),
    ];

    let nonce_words = [
        u32::from_le_bytes([nonce[0], nonce[1], nonce[2], nonce[3]]),
        u32::from_le_bytes([nonce[4], nonce[5], nonce[6], nonce[7]]),
        u32::from_le_bytes([nonce[8], nonce[9], nonce[10], nonce[11]]),
    ];

    let mut current_counter = counter;

    for chunk in data.chunks_mut(64) {
        let state = [
            constants[0],
            constants[1],
            constants[2],
            constants[3],
            key_words[0],
            key_words[1],
            key_words[2],
            key_words[3],
            key_words[4],
            key_words[5],
            key_words[6],
            key_words[7],
            current_counter,
            nonce_words[0],
            nonce_words[1],
            nonce_words[2],
        ];

        let mut working = state;
        for _ in 0..10 {
            // Column rounds
            let (mut a, mut b, mut c, mut d) = (working[0], working[4], working[8], working[12]);
            chacha20_quarter_round(&mut a, &mut b, &mut c, &mut d);
            working[0] = a;
            working[4] = b;
            working[8] = c;
            working[12] = d;

            let (mut a, mut b, mut c, mut d) = (working[1], working[5], working[9], working[13]);
            chacha20_quarter_round(&mut a, &mut b, &mut c, &mut d);
            working[1] = a;
            working[5] = b;
            working[9] = c;
            working[13] = d;

            let (mut a, mut b, mut c, mut d) = (working[2], working[6], working[10], working[14]);
            chacha20_quarter_round(&mut a, &mut b, &mut c, &mut d);
            working[2] = a;
            working[6] = b;
            working[10] = c;
            working[14] = d;

            let (mut a, mut b, mut c, mut d) = (working[3], working[7], working[11], working[15]);
            chacha20_quarter_round(&mut a, &mut b, &mut c, &mut d);
            working[3] = a;
            working[7] = b;
            working[11] = c;
            working[15] = d;

            // Diagonal rounds
            let (mut a, mut b, mut c, mut d) = (working[0], working[5], working[10], working[15]);
            chacha20_quarter_round(&mut a, &mut b, &mut c, &mut d);
            working[0] = a;
            working[5] = b;
            working[10] = c;
            working[15] = d;

            let (mut a, mut b, mut c, mut d) = (working[1], working[6], working[11], working[12]);
            chacha20_quarter_round(&mut a, &mut b, &mut c, &mut d);
            working[1] = a;
            working[6] = b;
            working[11] = c;
            working[12] = d;

            let (mut a, mut b, mut c, mut d) = (working[2], working[7], working[8], working[13]);
            chacha20_quarter_round(&mut a, &mut b, &mut c, &mut d);
            working[2] = a;
            working[7] = b;
            working[8] = c;
            working[13] = d;

            let (mut a, mut b, mut c, mut d) = (working[3], working[4], working[9], working[14]);
            chacha20_quarter_round(&mut a, &mut b, &mut c, &mut d);
            working[3] = a;
            working[4] = b;
            working[9] = c;
            working[14] = d;
        }

        let mut keystream = [0u8; 64];
        for (i, word) in working.iter().enumerate() {
            let final_word = word.wrapping_add(state[i]);
            keystream[i * 4..(i + 1) * 4].copy_from_slice(&final_word.to_le_bytes());
        }

        for (b, k) in chunk.iter_mut().zip(keystream.iter()) {
            *b ^= *k;
        }

        current_counter = current_counter.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_known_vector() {
        // "abc" -> ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        let digest = sha256_digest(b"abc");
        let hex = digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        assert_eq!(
            hex,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn test_hmac_sha256_integrity() {
        let key = b"secret-key";
        let data = b"payload-to-sign";
        let hmac1 = hmac_sha256(key, data);
        let hmac2 = hmac_sha256(key, data);
        assert_eq!(hmac1, hmac2);

        let hmac_diff = hmac_sha256(b"wrong-key", data);
        assert_ne!(hmac1, hmac_diff);
    }

    #[test]
    fn test_chacha20_round_trip() {
        let key = [0x42u8; 32];
        let nonce = [0x01u8; 12];
        let original = b"Hello, Agam Secure World!".to_vec();
        let mut encrypted = original.clone();

        chacha20_xor(&key, &nonce, 1, &mut encrypted);
        assert_ne!(encrypted, original);

        let mut decrypted = encrypted.clone();
        chacha20_xor(&key, &nonce, 1, &mut decrypted);
        assert_eq!(decrypted, original);
    }
}
