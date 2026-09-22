//! ChaCha20 — RFC 8439 stream cipher over a single `u32` state block.
//! Pure integer rotations and additions make it the kit's deterministic
//! cipher: no S-box tables to mistype, no float, no platform word-size
//! dependence — `apply` XORs a `u8` keystream that is a pure function of
//! `(key, nonce, counter, position)`.
//!
//! Intended use in the kit: tamper-evident replay streams and
//! lockstep-safe seeded content generation where `SplitMix64` is too
//! easy to invert (a `ChaCha20` keystream is a CSPRNG — seed leakage
//! cannot be recovered from observed output).
//!
//! ```
//! use izanagi_kit::chacha::ChaCha20;
//! let mut c = ChaCha20::new(&[0u32; 8], &[0u32; 3]);
//! let mut data = *b"hello deterministic world..........";
//! c.apply(&mut data);
//! let mut c2 = ChaCha20::new(&[0u32; 8], &[0u32; 3]);
//! c2.apply(&mut data);
//! assert_eq!(&data, b"hello deterministic world..........");
//! ```

const SIGMA: [u32; 4] = [0x6170_7865, 0x3320_646e, 0x7962_2d32, 0x6b20_6574];

#[inline]
fn quarter(x: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    x[a] = x[a].wrapping_add(x[b]);
    x[d] ^= x[a];
    x[d] = x[d].rotate_left(16);
    x[c] = x[c].wrapping_add(x[d]);
    x[b] ^= x[c];
    x[b] = x[b].rotate_left(12);
    x[a] = x[a].wrapping_add(x[b]);
    x[d] ^= x[a];
    x[d] = x[d].rotate_left(8);
    x[c] = x[c].wrapping_add(x[d]);
    x[b] ^= x[c];
    x[b] = x[b].rotate_left(7);
}

/// A ChaCha20 stream state. `new` fixes key+nonce; keystream position
/// advances as `apply` consumes bytes (the leftover block is kept).
#[derive(Clone)]
pub struct ChaCha20 {
    key: [u32; 8],
    nonce: [u32; 3],
    counter: u32,
    /// Keystream bytes remaining in `block[..used]`.
    block: [u8; 64],
    used: usize,
}

impl ChaCha20 {
    /// Cipher state for a 256-bit `key` (8 LE words) and 96-bit
    /// `nonce` (3 LE words).
    pub fn new(key: &[u32; 8], nonce: &[u32; 3]) -> Self {
        Self {
            key: *key,
            nonce: *nonce,
            counter: 0,
            block: [0; 64],
            used: 64, // forces first block generation
        }
    }

    /// One 64-byte keystream block for `counter`; does not advance
    /// the streaming state.
    pub fn block(&self, counter: u32) -> [u8; 64] {
        let mut st = [0u32; 16];
        st[..4].copy_from_slice(&SIGMA);
        st[4..12].copy_from_slice(&self.key);
        st[12] = counter;
        st[13..16].copy_from_slice(&self.nonce);
        let mut w = st;
        for _ in 0..10 {
            for (a, b, c, d) in [
                (0, 4, 8, 12),
                (1, 5, 9, 13),
                (2, 6, 10, 14),
                (3, 7, 11, 15),
                (0, 5, 10, 15),
                (1, 6, 11, 12),
                (2, 7, 8, 13),
                (3, 4, 9, 14),
            ] {
                quarter(&mut w, a, b, c, d);
            }
        }
        let mut out = [0u8; 64];
        for i in 0..16 {
            let v = w[i].wrapping_add(st[i]);
            out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        out
    }

    /// XOR `data` with the keystream at the current position.
    /// `counter` wraps at `u32::MAX` — a stream is at most 2^32
    /// blocks (256 GiB) by spec.
    pub fn apply(&mut self, data: &mut [u8]) {
        for b in data.iter_mut() {
            if self.used == 64 {
                self.block = self.block(self.counter);
                self.counter = self.counter.wrapping_add(1);
                self.used = 0;
            }
            *b ^= self.block[self.used];
            self.used += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc8439_keystream_block() {
        // RFC 8439 §2.3.2 — key 00..1f, nonce 00..0b, counter 1.
        let key: [u32; 8] = [
            0x03020100, 0x07060504, 0x0b0a0908, 0x0f0e0d0c, 0x13121110, 0x17161514, 0x1b1a1918,
            0x1f1e1d1c,
        ];
        let nonce = [0x00000000, 0x4a00_0000, 0x00000000];
        let c = ChaCha20::new(&key, &nonce);
        let ks = c.block(1);
        const EXPECTED: [u8; 64] = [
            0x22, 0x4f, 0x51, 0xf3, 0x40, 0x1b, 0xd9, 0xe1, 0x2f, 0xde, 0x27, 0x6f, 0xb8, 0x63,
            0x1d, 0xed, 0x8c, 0x13, 0x1f, 0x82, 0x3d, 0x2c, 0x06, 0xe2, 0x7e, 0x4f, 0xca, 0xec,
            0x9e, 0xf3, 0xcf, 0x78, 0x8a, 0x3b, 0x0a, 0xa3, 0x72, 0x60, 0x0a, 0x92, 0xb5, 0x79,
            0x74, 0xcd, 0xed, 0x2b, 0x93, 0x34, 0x79, 0x4c, 0xba, 0x40, 0xc6, 0x3e, 0x34, 0xcd,
            0xea, 0x21, 0x2c, 0x4c, 0xf0, 0x7d, 0x41, 0xb7,
        ];
        assert_eq!(ks, EXPECTED);
    }

    #[test]
    fn rfc8439_encryption_vector() {
        // RFC 8439 §2.4.2 — "Ladies and Gentlemen of the class of '99".
        let key: [u32; 8] = [
            0x03020100, 0x07060504, 0x0b0a0908, 0x0f0e0d0c, 0x13121110, 0x17161514, 0x1b1a1918,
            0x1f1e1d1c,
        ];
        let nonce = [0x00000000, 0x4a00_0000, 0x00000000];
        let plain = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";
        let mut buf = *plain;
        let mut c = ChaCha20::new(&key, &nonce);
        c.counter = 1;
        c.apply(&mut buf);
        const EXPECTED: [u8; 6] = [0x6e, 0x2e, 0x35, 0x9a, 0x25, 0x68];
        assert_eq!(&buf[..6], &EXPECTED);
        // Round-trip.
        let mut d = ChaCha20::new(&key, &nonce);
        d.counter = 1;
        d.apply(&mut buf);
        assert_eq!(&buf, plain);
    }

    #[test]
    fn chunked_apply_equals_one_shot() {
        let mut rng = crate::rng::SplitMix64::new(3);
        let key: [u32; 8] = core::array::from_fn(|_| rng.next_u64() as u32);
        let nonce: [u32; 3] = core::array::from_fn(|_| rng.next_u64() as u32);
        let data: Vec<u8> = (0..500).map(|_| rng.next_u64() as u8).collect();
        let mut a = data.clone();
        ChaCha20::new(&key, &nonce).apply(&mut a);
        let mut b = data.clone();
        let mut c = ChaCha20::new(&key, &nonce);
        let mut i = 0;
        while i < b.len() {
            let n = (rng.below(70) as usize + 1).min(b.len() - i);
            c.apply(&mut b[i..i + n]);
            i += n;
        }
        assert_eq!(a, b);
    }
}
