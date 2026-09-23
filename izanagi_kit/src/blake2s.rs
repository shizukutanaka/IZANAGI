//! BLAKE2s-256 — the 32-bit-word sibling of BLAKE2 (RFC 7693).
//! 10 rounds over the ChaCha-shaped `G` mixer with a fixed
//! message permutation `σ`; all arithmetic is `u32` add/xor/rot.
//!
//! Where [`crate::siphash`] gives a keyed 64-bit PRF and
//! [`crate::sha256`] an unkeyed 256-bit digest, BLAKE2s covers
//! both modes in one construction: `Blake2s::new()` for plain
//! hashing, `Blake2s::keyed()` for a MAC without the HMAC
//! construction. Split-independence is guaranteed by the same
//! 64-byte staging buffer as `sha256`.
//!
//! ```
//! use izanagi_kit::blake2s::blake2s;
//!
//! // RFC 7693 Appendix A: BLAKE2s-256 of the empty string.
//! let d = blake2s(b"");
//! assert_eq!(d[0], 0x69);
//! assert_eq!(d[1], 0x21);
//! assert_eq!(d[31], 0xf9);
//! ```

const IV: [u32; 8] = [
    0x6A09_E667,
    0xBB67_AE85,
    0x3C6E_F372,
    0xA54F_F53A,
    0x510E_527F,
    0x9B05_688C,
    0x1F83_D9AB,
    0x5BE0_CD19,
];

const SIGMA: [[usize; 16]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
];

fn g(v: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, x: u32, y: u32) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(x);
    v[d] = (v[d] ^ v[a]).rotate_right(16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(12);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(y);
    v[d] = (v[d] ^ v[a]).rotate_right(8);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(7);
}

/// Streaming BLAKE2s-256 state.
pub struct Blake2s {
    h: [u32; 8],
    buf: [u8; 64],
    buflen: usize,
    /// Total bytes compressed so far (not counting `buf`).
    t: u64,
}

impl Blake2s {
    /// Unkeyed digest state.
    pub fn new() -> Blake2s {
        let mut h = IV;
        // Parameter block for sequential BLAKE2s-256:
        // digest=32, key=0, fanout=1, depth=1 → 0x01010020.
        h[0] ^= 0x0101_0020;
        Blake2s {
            h,
            buf: [0; 64],
            buflen: 0,
            t: 0,
        }
    }

    /// Keyed-MAC state. `key` is at most 32 bytes (RFC 7693 §3.3);
    /// returns `None` for longer keys.
    pub fn keyed(key: &[u8]) -> Option<Blake2s> {
        if key.len() > 32 {
            return None;
        }
        let mut h = IV;
        h[0] ^= 0x0101_0020 ^ ((key.len() as u32) << 8);
        let mut s = Blake2s {
            h,
            buf: [0; 64],
            buflen: 0,
            t: 0,
        };
        // The key is prepended as a full padded block. It must
        // sit in the buffer, not be compressed now — for an
        // empty message it is itself the last block.
        let mut kb = [0u8; 64];
        kb[..key.len()].copy_from_slice(key);
        s.buf = kb;
        s.buflen = 64;
        Some(s)
    }

    fn compress(&mut self, block: &[u8; 64], add: u64, last: bool) {
        self.t = self.t.wrapping_add(add);
        let mut m = [0u32; 16];
        for (i, w) in m.iter_mut().enumerate() {
            *w = u32::from_le_bytes([
                block[4 * i],
                block[4 * i + 1],
                block[4 * i + 2],
                block[4 * i + 3],
            ]);
        }
        let mut v = [0u32; 16];
        v[..8].copy_from_slice(&self.h);
        v[8..].copy_from_slice(&IV);
        v[12] ^= self.t as u32;
        v[13] ^= (self.t >> 32) as u32;
        if last {
            v[14] ^= 0xFFFF_FFFF;
        }
        for s in &SIGMA {
            g(&mut v, 0, 4, 8, 12, m[s[0]], m[s[1]]);
            g(&mut v, 1, 5, 9, 13, m[s[2]], m[s[3]]);
            g(&mut v, 2, 6, 10, 14, m[s[4]], m[s[5]]);
            g(&mut v, 3, 7, 11, 15, m[s[6]], m[s[7]]);
            g(&mut v, 0, 5, 10, 15, m[s[8]], m[s[9]]);
            g(&mut v, 1, 6, 11, 12, m[s[10]], m[s[11]]);
            g(&mut v, 2, 7, 8, 13, m[s[12]], m[s[13]]);
            g(&mut v, 3, 4, 9, 14, m[s[14]], m[s[15]]);
        }
        for i in 0..8 {
            self.h[i] ^= v[i] ^ v[i + 8];
        }
    }

    /// Feeds `data`; the result does not depend on how the input
    /// was split across calls.
    pub fn write(&mut self, mut data: &[u8]) {
        while !data.is_empty() {
            if self.buflen == 64 {
                // A full buffer left by the previous call can be
                // compressed now — more data is arriving, so this
                // block is not the last.
                let blk = self.buf;
                self.compress(&blk, 64, false);
                self.buflen = 0;
            }
            let take = (64 - self.buflen).min(data.len());
            self.buf[self.buflen..self.buflen + take].copy_from_slice(&data[..take]);
            self.buflen += take;
            data = &data[take..];
            if self.buflen == 64 && !data.is_empty() {
                let blk = self.buf;
                self.compress(&blk, 64, false);
                self.buflen = 0;
            }
        }
    }

    /// Finishes and returns the 32-byte digest.
    pub fn finish(mut self) -> [u8; 32] {
        let mut tail = [0u8; 64];
        tail[..self.buflen].copy_from_slice(&self.buf[..self.buflen]);
        self.compress(&tail, self.buflen as u64, true);
        let mut out = [0u8; 32];
        for (i, w) in self.h.iter().enumerate() {
            out[4 * i..4 * i + 4].copy_from_slice(&w.to_le_bytes());
        }
        out
    }
}

impl Default for Blake2s {
    fn default() -> Blake2s {
        Blake2s::new()
    }
}

/// One-shot unkeyed BLAKE2s-256.
pub fn blake2s(data: &[u8]) -> [u8; 32] {
    let mut s = Blake2s::new();
    s.write(data);
    s.finish()
}

/// One-shot keyed BLAKE2s-256 (`None` for keys over 32 bytes).
pub fn blake2s_keyed(key: &[u8], data: &[u8]) -> Option<[u8; 32]> {
    let mut s = Blake2s::keyed(key)?;
    s.write(data);
    Some(s.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn hexdigest(h: &[u8; 32]) -> String {
        h.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn rfc7693_unkeyed_vectors() {
        // Python hashlib.blake2s(bytes(range(n))).hexdigest().
        let cases: [(usize, &str); 5] = [
            (
                0,
                "69217a3079908094e11121d042354a7c1f55b6482ca1a51e1b250dfd1ed0eef9",
            ),
            (
                1,
                "e34d74dbaf4ff4c6abd871cc220451d2ea2648846c7757fbaac82fe51ad64bea",
            ),
            (
                63,
                "e57cb79487dd57902432b250733813bd96a84efce59f650fac26e6696aefafc3",
            ),
            (
                64,
                "56f34e8b96557e90c1f24b52d0c89d51086acf1b00f634cf1dde9233b8eaaa3e",
            ),
            (
                255,
                "f03f5789d3336b80d002d59fdf918bdb775b00956ed5528e86aa994acb38fe2d",
            ),
        ];
        for (n, want) in cases {
            let msg: Vec<u8> = (0..n).map(|i| i as u8).collect();
            assert_eq!(hexdigest(&blake2s(&msg)), want, "len {n}");
        }
    }

    #[test]
    fn rfc7693_keyed_vectors() {
        let key: Vec<u8> = (0..32u8).collect();
        let d = blake2s_keyed(&key, b"").unwrap_or_else(|| panic!("key ok"));
        assert_eq!(
            hexdigest(&d),
            "48a8997da407876b3d79c0d92325ad3b89cbb754d86ab71aee047ad345fd2c49"
        );
        let msg: Vec<u8> = (0..32u8).collect();
        let d = blake2s_keyed(&key, &msg).unwrap_or_else(|| panic!("key ok"));
        assert_eq!(
            hexdigest(&d),
            "c03bc642b20959cbe133a0303e0c1abff3e31ec8e1a328ec8565c36decff5265"
        );
        assert!(blake2s_keyed(&[0u8; 33], b"x").is_none());
    }

    #[test]
    fn split_independence() {
        let mut rng = SplitMix64::new(0xB2A4);
        for _ in 0..100 {
            let n = (rng.next_u64() % 200) as usize;
            let msg: Vec<u8> = (0..n).map(|_| rng.next_u64() as u8).collect();
            let want = blake2s(&msg);
            let cut = (rng.next_u64() % (n as u64 + 1)) as usize;
            let mut s = Blake2s::new();
            s.write(&msg[..cut]);
            s.write(&msg[cut..]);
            assert_eq!(s.finish(), want, "cut {cut} of {n}");
        }
    }

    #[test]
    fn full_block_boundary() {
        // Exactly 64 bytes — the trailing full block must be
        // emitted as the last block, not a zero-length tail.
        let msg: Vec<u8> = (0..64u8).collect();
        let mut s = Blake2s::new();
        s.write(&msg);
        assert_eq!(s.finish(), blake2s(&msg));
        // 64 + 1 exercises buffer-carryover across the boundary.
        let msg: Vec<u8> = (0..65u8).collect();
        let mut s = Blake2s::new();
        s.write(&msg[..64]);
        s.write(&msg[64..]);
        assert_eq!(s.finish(), blake2s(&msg));
    }
}
