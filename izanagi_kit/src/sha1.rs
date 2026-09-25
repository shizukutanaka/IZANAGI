//! SHA-1 — RFC 3174 secure hash, integer-only by construction.
//! Streaming `write`/`finish` over a 64-byte block pipeline, mirroring
//! [`crate::sha256`]. SHA-1 is collision-broken for adversarial use,
//! but remains the addressing primitive of git objects and the HMAC
//! backend of TOTP/HOTP — this module is the shared, public home so
//! new consumers stop growing private copies.
//!
//! ```
//! use izanagi_kit::sha1::Sha1;
//! let mut h = Sha1::new();
//! h.write(b"abc");
//! assert_eq!(
//!     h.finish(),
//!     [0xa9, 0x99, 0x3e, 0x36, 0x47, 0x06, 0x81, 0x6a, 0xba, 0x3e,
//!      0x25, 0x71, 0x78, 0x50, 0xc2, 0x6c, 0x9c, 0xd0, 0xd8, 0x9d]
//! );
//! ```

const H0: [u32; 5] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0];

/// Streaming SHA-1 hasher.
#[derive(Clone)]
pub struct Sha1 {
    h: [u32; 5],
    buf: [u8; 64],
    buf_len: usize,
    total: u64,
}

impl Default for Sha1 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha1 {
    /// Fresh hasher.
    pub fn new() -> Self {
        Self {
            h: H0,
            buf: [0; 64],
            buf_len: 0,
            total: 0,
        }
    }

    /// Feed bytes; any split of the stream yields the same digest.
    pub fn write(&mut self, mut data: &[u8]) {
        self.total = self.total.wrapping_add(data.len() as u64);
        if self.buf_len > 0 {
            let take = (64 - self.buf_len).min(data.len());
            self.buf[self.buf_len..self.buf_len + take].copy_from_slice(&data[..take]);
            self.buf_len += take;
            data = &data[take..];
            if self.buf_len == 64 {
                let b = self.buf;
                self.compress(&b);
                self.buf_len = 0;
            }
        }
        while data.len() >= 64 {
            let (chunk, rest) = data.split_at(64);
            let mut b = [0u8; 64];
            b.copy_from_slice(chunk);
            self.compress(&b);
            data = rest;
        }
        if !data.is_empty() {
            self.buf[..data.len()].copy_from_slice(data);
            self.buf_len = data.len();
        }
    }

    /// Emit the 20-byte digest.
    pub fn finish(mut self) -> [u8; 20] {
        let bit_len = self.total.wrapping_mul(8);
        self.write(&[0x80]);
        while self.buf_len != 56 {
            self.write(&[0]);
        }
        let mut len_bytes = [0u8; 8];
        for (i, b) in len_bytes.iter_mut().enumerate() {
            *b = (bit_len >> (56 - i * 8)) as u8;
        }
        self.write(&len_bytes);
        let mut out = [0u8; 20];
        for i in 0..5 {
            let v = self.h[i];
            out[i * 4] = (v >> 24) as u8;
            out[i * 4 + 1] = (v >> 16) as u8;
            out[i * 4 + 2] = (v >> 8) as u8;
            out[i * 4 + 3] = v as u8;
        }
        out
    }

    fn compress(&mut self, b: &[u8; 64]) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            // Big-endian word load (spec order) without be_bytes
            // helpers, which the kit's width rules ban.
            w[i] = ((b[i * 4] as u32) << 24)
                | ((b[i * 4 + 1] as u32) << 16)
                | ((b[i * 4 + 2] as u32) << 8)
                | (b[i * 4 + 3] as u32);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let [mut a, mut b_, mut c, mut d, mut e] = self.h;
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = match i / 20 {
                0 => ((b_ & c) | (!b_ & d), 0x5a827999u32),
                1 => (b_ ^ c ^ d, 0x6ed9eba1),
                2 => ((b_ & c) | (b_ & d) | (c & d), 0x8f1bbcdc),
                _ => (b_ ^ c ^ d, 0xca62c1d6),
            };
            let t = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(wi);
            e = d;
            d = c;
            c = b_.rotate_left(30);
            b_ = a;
            a = t;
        }
        for i in 0..5 {
            self.h[i] = self.h[i].wrapping_add([a, b_, c, d, e][i]);
        }
    }
}

/// One-shot digest.
pub fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h = Sha1::new();
    h.write(data);
    h.finish()
}

/// Digest as lowercase hex (git object names use this form).
pub fn sha1_hex(data: &[u8]) -> String {
    hex(&sha1(data))
}

/// HMAC-SHA-1 (RFC 2104) — the backend of HOTP/TOTP.
pub fn hmac_sha1(key: &[u8], data: &[u8]) -> [u8; 20] {
    let mut k = [0u8; 64];
    if key.len() > 64 {
        k[..20].copy_from_slice(&sha1(key));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let mut inner = Sha1::new();
    let mut pad = [0u8; 64];
    for i in 0..64 {
        pad[i] = k[i] ^ 0x36;
    }
    inner.write(&pad);
    inner.write(data);
    let tag = inner.finish();
    let mut outer = Sha1::new();
    for i in 0..64 {
        pad[i] = k[i] ^ 0x5c;
    }
    outer.write(&pad);
    outer.write(&tag);
    outer.finish()
}

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for &v in b {
        s.push(char::from_digit((v >> 4) as u32, 16).unwrap_or('0'));
        s.push(char::from_digit((v & 15) as u32, 16).unwrap_or('0'));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc3174_vectors() {
        assert_eq!(
            sha1(b""),
            [
                0xda, 0x39, 0xa3, 0xee, 0x5e, 0x6b, 0x4b, 0x0d, 0x32, 0x55, 0xbf, 0xef, 0x95, 0x60,
                0x18, 0x90, 0xaf, 0xd8, 0x07, 0x09
            ]
        );
        assert_eq!(sha1_hex(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(
            sha1_hex(b"The quick brown fox jumps over the lazy dog"),
            "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12"
        );
        assert_eq!(
            sha1_hex(b"The quick brown fox jumps over the lazy cog"),
            "de9f2c7fd25e1b3afad3e85a0bd17d9b100db4b3"
        );
    }

    #[test]
    fn million_a_vector() {
        let mut h = Sha1::new();
        for _ in 0..10000 {
            h.write(&[b'a'; 100]);
        }
        let d = h.finish();
        let mut s = String::new();
        for &v in &d {
            s.push(char::from_digit((v >> 4) as u32, 16).unwrap_or('0'));
            s.push(char::from_digit((v & 15) as u32, 16).unwrap_or('0'));
        }
        assert_eq!(s, "34aa973cd4c4daa4f61eeb2bdbad27316534016f");
    }

    #[test]
    fn split_points_do_not_matter() {
        let data: Vec<u8> = (0..1000u32).map(|i| (i * 7) as u8).collect();
        let whole = sha1(&data);
        for cut in [1usize, 55, 56, 63, 64, 65, 119, 128, 500] {
            let mut h = Sha1::new();
            h.write(&data[..cut]);
            h.write(&data[cut..]);
            assert_eq!(h.finish(), whole);
        }
    }

    #[test]
    fn hmac_rfc2202_vectors() {
        // case 1: key = 0x0b x 20, data = "Hi There"
        assert_eq!(
            hmac_sha1(&[0x0b; 20], b"Hi There"),
            [
                0xb6, 0x17, 0x31, 0x86, 0x55, 0x05, 0x72, 0x64, 0xe2, 0x8b, 0xc0, 0xb6, 0xfb, 0x37,
                0x8c, 0x8e, 0xf1, 0x46, 0xbe, 0x00
            ]
        );
        // case 3: key = 0xaa x 20, data = 0xdd x 50
        assert_eq!(
            hmac_sha1(&[0xaa; 20], &[0xdd; 50]),
            [
                0x12, 0x5d, 0x73, 0x42, 0xb9, 0xac, 0x11, 0xcd, 0x91, 0xa3, 0x9a, 0xf4, 0x8a, 0xa1,
                0x7b, 0x4f, 0x63, 0xf1, 0x75, 0xd3
            ]
        );
        // long-key path (>64 bytes): key = 0xaa x 80, "Test Using Larger Than Block-Size Key - Hash Key First"
        assert_eq!(
            hmac_sha1(
                &[0xaa; 80],
                b"Test Using Larger Than Block-Size Key - Hash Key First"
            ),
            [
                0xaa, 0x4a, 0xe5, 0xe1, 0x52, 0x72, 0xd0, 0x0e, 0x95, 0x70, 0x56, 0x37, 0xce, 0x8a,
                0x3b, 0x55, 0xed, 0x40, 0x21, 0x12
            ]
        );
        // "Jefe"
        assert_eq!(
            hmac_sha1(b"Jefe", b"what do ya want for nothing?"),
            [
                0xef, 0xfc, 0xdf, 0x6a, 0xe5, 0xeb, 0x2f, 0xa2, 0xd2, 0x74, 0x16, 0xd5, 0xf1, 0x84,
                0xdf, 0x9c, 0x25, 0x9a, 0x7c, 0x79
            ]
        );
    }

    #[test]
    fn determinism() {
        assert_eq!(sha1(b"replay"), sha1(b"replay"));
        assert_eq!(sha1_hex(b"x").len(), 40);
    }
}
