//! Poly1305 one-time authenticator (Bernstein 2005, RFC 8439 §2.5).
//! The 256-bit key splits into `r` (clamped multiplier) and `s`
//! (addend); each 16-byte message block is accumulated into a
//! 130-bit limb accumulator and multiplied by `r` mod `2^130 − 5`,
//! exactly the DJB reference arithmetic — pure integer work, no
//! platform or endian dependence (`be`/`to_le_bytes` helpers are
//! avoided in favour of explicit LE shifts).
//!
//! Paired with [`crate::chacha`] this is the MAC half of
//! ChaCha20-Poly1305: a deterministic replay can authenticate each
//! tick's wire bytes without leaving the integer domain.
//!
//! ```
//! use izanagi_kit::poly1305::poly1305;
//! // RFC 8439 §2.5.2 test key.
//! let key = [
//!     0x85, 0xd6, 0xbe, 0x78, 0x57, 0x55, 0x6d, 0x33, 0x7f, 0x44, 0x52, 0xfe, 0x42, 0xd5,
//!     0x06, 0xa8, 0x01, 0x03, 0x80, 0x8a, 0xfb, 0x0d, 0xb2, 0xfd, 0x4a, 0xbf, 0xf6, 0xaf,
//!     0x41, 0x49, 0xf5, 0x1b,
//! ];
//! let tag = poly1305(b"Cryptographic Forum Research Group", &key);
//! assert_eq!(
//!     tag,
//!     [0xa8, 0x06, 0x1d, 0xc1, 0x30, 0x51, 0x36, 0xc6, 0xc2, 0x2b, 0x8b, 0xaf, 0x0c, 0x01, 0x27, 0xa9]
//! );
//! ```

const MASK: u64 = 0x3ff_ffff; // 26 bits

fn le64(b: &[u8]) -> u64 {
    let mut v = 0u64;
    for (i, &x) in b[..8].iter().enumerate() {
        v |= (x as u64) << (i * 8);
    }
    v
}

/// Poly1305 authentication state: `write` arbitrary byte runs,
/// `finish` to tag. Any split of the message yields the same tag.
pub struct Poly1305 {
    h: [u64; 5],
    r: [u64; 5],
    s: [u64; 5],
    buf: [u8; 16],
    buf_len: usize,
}

impl Poly1305 {
    /// `key` = `r ‖ s` as 32 bytes (16 + 16).
    pub fn new(key: &[u8; 32]) -> Self {
        let t0 = le64(&key[0..8]);
        let t1 = le64(&key[8..16]);
        let r = [
            t0 & MASK,
            ((t0 >> 26) | (t1 << 38)) & 0x3ff_ff03,
            ((t0 >> 52) | (t1 << 12)) & 0x3ff_c0ff,
            (t1 >> 14) & 0x3f0_3fff,
            (t1 >> 40) & 0x0f_ffff,
        ];
        let s = [
            le64(&key[16..24]) & MASK,
            (le64(&key[16..24]) >> 26) & MASK,
            (le64(&key[16..24]) >> 52) | ((le64(&key[24..32]) & 0x3fff) << 12),
            (le64(&key[24..32]) >> 14) & MASK,
            le64(&key[24..32]) >> 40,
        ];
        Self {
            h: [0; 5],
            r,
            s,
            buf: [0; 16],
            buf_len: 0,
        }
    }

    fn block(&mut self, m: &[u8], hibit: u64) {
        let mut chunk = [0u8; 16];
        chunk[..m.len()].copy_from_slice(m);
        let t0 = le64(&chunk[0..8]);
        let t1 = le64(&chunk[8..16]);
        let mut h = self.h;
        h[0] = h[0].wrapping_add(t0 & MASK);
        h[1] = h[1].wrapping_add(((t0 >> 26) | (t1 << 38)) & MASK);
        h[2] = h[2].wrapping_add(((t0 >> 52) | (t1 << 12)) & MASK);
        h[3] = h[3].wrapping_add((t1 >> 14) & MASK);
        h[4] = h[4].wrapping_add((t1 >> 40) | hibit);

        let [r0, r1, r2, r3, r4] = self.r;
        let s1 = r1 * 5;
        let s2 = r2 * 5;
        let s3 = r3 * 5;
        let s4 = r4 * 5;
        let d = [
            h[0] * r0 + h[1] * s4 + h[2] * s3 + h[3] * s2 + h[4] * s1,
            h[0] * r1 + h[1] * r0 + h[2] * s4 + h[3] * s3 + h[4] * s2,
            h[0] * r2 + h[1] * r1 + h[2] * r0 + h[3] * s4 + h[4] * s3,
            h[0] * r3 + h[1] * r2 + h[2] * r1 + h[3] * r0 + h[4] * s4,
            h[0] * r4 + h[1] * r3 + h[2] * r2 + h[3] * r1 + h[4] * r0,
        ];
        let mut c = d[0] >> 26;
        h[0] = d[0] & MASK;
        let d1 = d[1] + c;
        c = d1 >> 26;
        h[1] = d1 & MASK;
        let d2 = d[2] + c;
        c = d2 >> 26;
        h[2] = d2 & MASK;
        let d3 = d[3] + c;
        c = d3 >> 26;
        h[3] = d3 & MASK;
        let d4 = d[4] + c;
        c = d4 >> 26;
        h[4] = d4 & MASK;
        h[0] += c * 5;
        let c0 = h[0] >> 26;
        h[0] &= MASK;
        h[1] += c0;
        self.h = h;
    }

    /// Feed bytes; incremental writes compose.
    pub fn write(&mut self, mut data: &[u8]) {
        if self.buf_len > 0 {
            let take = (16 - self.buf_len).min(data.len());
            self.buf[self.buf_len..self.buf_len + take].copy_from_slice(&data[..take]);
            self.buf_len += take;
            data = &data[take..];
            if self.buf_len == 16 {
                let b = self.buf;
                self.buf_len = 0;
                self.block(&b, 1 << 24);
            }
        }
        while data.len() >= 16 {
            let (chunk, rest) = data.split_at(16);
            let mut b = [0u8; 16];
            b.copy_from_slice(chunk);
            self.block(&b, 1 << 24);
            data = rest;
        }
        if !data.is_empty() {
            self.buf[..data.len()].copy_from_slice(data);
            self.buf_len = data.len();
        }
    }

    /// Emit the 16-byte tag.
    pub fn finish(mut self) -> [u8; 16] {
        if self.buf_len > 0 {
            // Partial final block: the `0x01` terminator lands at
            // position `len` inside the 128-bit limb (2^(8·len)), so
            // the extra hibit stays 0.
            let len = self.buf_len;
            self.buf[len] = 1;
            for b in self.buf.iter_mut().skip(len + 1) {
                *b = 0;
            }
            let b = self.buf;
            self.block(&b, 0);
            self.buf_len = 0;
        }
        // Final carry chain.
        let mut h = self.h;
        let mut c = h[1] >> 26;
        h[1] &= MASK;
        h[2] += c;
        c = h[2] >> 26;
        h[2] &= MASK;
        h[3] += c;
        c = h[3] >> 26;
        h[3] &= MASK;
        h[4] += c;
        c = h[4] >> 26;
        h[4] &= MASK;
        h[0] += c * 5;
        c = h[0] >> 26;
        h[0] &= MASK;
        h[1] += c;

        // h < 2^130; compute g = h + 5 - 2^130 and select.
        let mut g = [0u64; 5];
        g[0] = h[0] + 5;
        c = g[0] >> 26;
        g[0] &= MASK;
        g[1] = h[1] + c;
        c = g[1] >> 26;
        g[1] &= MASK;
        g[2] = h[2] + c;
        c = g[2] >> 26;
        g[2] &= MASK;
        g[3] = h[3] + c;
        c = g[3] >> 26;
        g[3] &= MASK;
        let g4 = h[4].wrapping_add(c).wrapping_sub(1 << 26);
        // If h < 2^130 − 5 the subtraction borrows (g4 "negative" in
        // 64-bit wrap) — pick h; else pick g.
        if (g4 >> 63) == 0 {
            h = g;
        }

        // Pack limbs into two u64s (LE base-2^26 → base-2^64).
        let pad0 = h[0] | (h[1] << 26) | ((h[2] & 0xfff) << 52);
        let pad1 = (h[2] >> 12) | (h[3] << 14) | ((h[4] & 0xff_ffff) << 40);

        // Add s as a 128-bit number.
        let s0 = self.s[0] | (self.s[1] << 26) | ((self.s[2] & 0xfff) << 52);
        let s1 = (self.s[2] >> 12) | (self.s[3] << 14) | ((self.s[4] & 0xff_ffff) << 40);
        let lo = pad0.wrapping_add(s0);
        let carry = (lo < pad0) as u64;
        let hi = pad1.wrapping_add(s1).wrapping_add(carry);

        let mut out = [0u8; 16];
        for i in 0..8 {
            out[i] = (lo >> (i * 8)) as u8;
            out[i + 8] = (hi >> (i * 8)) as u8;
        }
        out
    }
}

/// One-shot Poly1305 tag over `msg` with the 32-byte key `r ‖ s`.
pub fn poly1305(msg: &[u8], key: &[u8; 32]) -> [u8; 16] {
    let mut p = Poly1305::new(key);
    p.write(msg);
    p.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn from_hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or(0))
            .collect()
    }

    #[test]
    fn rfc8439_tag_vector() {
        let key: [u8; 32] =
            from_hex("85d6be7857556d337f4452fe42d506a80103808afb0db2fd4abff6af4149f51b")
                .try_into()
                .unwrap_or([0; 32]);
        let tag = poly1305(b"Cryptographic Forum Research Group", &key);
        assert_eq!(tag, from_hex("a8061dc1305136c6c22b8baf0c0127a9").as_slice());
    }

    #[test]
    fn zero_key_yields_zero_tag() {
        assert_eq!(poly1305(b"anything", &[0; 32]), [0; 16]);
        assert_eq!(poly1305(b"", &[0; 32]), [0; 16]);
    }

    #[test]
    fn r_zero_s_set_tag_is_s() {
        // r = 0 → every block multiply resets the accumulator to 0,
        // so a message of whole 16-byte blocks leaves h = 0 and the
        // tag is exactly s = key[16..32].
        let mut key = [0u8; 32];
        for (i, k) in key.iter_mut().enumerate().skip(16) {
            *k = i as u8;
        }
        let tag = poly1305(b"0123456789abcdef", &key); // exactly 16 B
        assert_eq!(&tag, &key[16..32]);
    }

    #[test]
    fn chunked_write_is_split_invariant() {
        let mut rng = SplitMix64::new(11);
        let key: [u8; 32] = core::array::from_fn(|_| rng.next_u64() as u8);
        let data: Vec<u8> = (0..500).map(|_| rng.next_u64() as u8).collect();
        let want = poly1305(&data, &key);
        let mut p = Poly1305::new(&key);
        let mut i = 0;
        while i < data.len() {
            let n = (rng.below(40) as usize + 1).min(data.len() - i);
            p.write(&data[i..i + n]);
            i += n;
        }
        assert_eq!(p.finish(), want);
    }

    #[test]
    fn one_bit_flip_changes_tag() {
        let mut rng = SplitMix64::new(7);
        let key: [u8; 32] = core::array::from_fn(|_| rng.next_u64() as u8);
        let a = poly1305(b"seed-value", &key);
        let mut b = *b"seed-value";
        b[3] ^= 1;
        assert_ne!(a, poly1305(&b, &key));
    }
}
