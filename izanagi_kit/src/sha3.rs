//! SHA-3 and SHAKE — the Keccak-f\[1600\] sponge, FIPS 202.
//!
//! The state is 25 lanes of `u64` (5×5). Each absorb step
//! XORs a rate-sized block of input into the lanes and runs
//! the 24-round permutation `keccak_f`:
//!
//! ```text
//! θ: C[x] = A[x,0]⊕A[x,1]⊕A[x,2]⊕A[x,3]⊕A[x,4]
//!    A[x,y] ^= C[x-1] ⊕ rot(C[x+1], 1)
//! ρ: A[x,y] <<= rot(x,y)   (the spec's triangle table)
//! π: B[y, 2x+3y] = A[x,y]
//! χ: A[x,y] = B[x,y] ^ (~B[x+1,y] & B[x+2,y])
//! ι: A[0,0] ^= RC[round]
//! ```
//!
//! Squeeze: read `out_len` bytes from the lane byte order
//! (little-endian per lane), squeezing permutations as
//! needed.
//!
//! Domain suffixes per FIPS 202: SHA3 = `0x06`, SHAKE =
//! `0x1F`, padded with pad10*1 (`0x01`/`0x80` terminators).
//!
//! ```
//! use izanagi_kit::sha3::{sha3_256, sha3_512, shake128};
//! assert_eq!(
//!     sha3_256(b""),
//!     [
//!         0xa7, 0xff, 0xc6, 0xf8, 0xbf, 0x1e, 0xd7, 0x66, 0x51, 0xc1, 0x47, 0x56, 0xa0, 0x61,
//!         0xd6, 0x62, 0xf5, 0x80, 0xff, 0x4d, 0xe4, 0x3b, 0x49, 0xfa, 0x82, 0xd8, 0x0a, 0x4b,
//!         0x80, 0xf8, 0x43, 0x4a,
//!     ]
//! );
//! assert_eq!(shake128(b"", 4), [0x7f, 0x9c, 0x2b, 0xa4]);
//! let d512 = sha3_512(b"");
//! assert_eq!(d512[0], 0xa6);
//! ```

/// Keccak-f\[1600\] round constants (one per round).
const RC: [u64; 24] = [
    0x0000000000000001,
    0x0000000000008082,
    0x800000000000808a,
    0x8000000080008000,
    0x000000000000808b,
    0x0000000080000001,
    0x8000000080008081,
    0x8000000000008009,
    0x000000000000008a,
    0x0000000000000088,
    0x0000000080008009,
    0x000000008000000a,
    0x000000008000808b,
    0x800000000000008b,
    0x8000000000008089,
    0x8000000000008003,
    0x8000000000008002,
    0x8000000000000080,
    0x000000000000800a,
    0x800000008000000a,
    0x8000000080008081,
    0x8000000000008080,
    0x0000000080000001,
    0x8000000080008008,
];

/// ρ offsets — `ROT[x][y]` from the spec's (x,y)-triangle.
const ROT: [[u32; 5]; 5] = [
    [0, 36, 3, 41, 18],
    [1, 44, 10, 45, 2],
    [62, 6, 43, 15, 61],
    [28, 55, 25, 21, 56],
    [27, 20, 39, 8, 14],
];

/// One Keccak-f\[1600\] permutation over the 25-lane state.
fn keccak_f(a: &mut [u64; 25]) {
    for &rc in &RC {
        // θ
        let mut c = [0u64; 5];
        for x in 0..5 {
            c[x] = a[x] ^ a[x + 5] ^ a[x + 10] ^ a[x + 15] ^ a[x + 20];
        }
        for x in 0..5 {
            let d = c[(x + 4) % 5] ^ c[(x + 1) % 5].rotate_left(1);
            for y in 0..5 {
                a[x + 5 * y] ^= d;
            }
        }
        // ρ + π fused: B[y][2x+3y] = rot(A[x][y])
        let mut b = [0u64; 25];
        for x in 0..5 {
            for y in 0..5 {
                b[y + 5 * ((2 * x + 3 * y) % 5)] = a[x + 5 * y].rotate_left(ROT[x][y]);
            }
        }
        // χ
        for y in 0..5 {
            for x in 0..5 {
                a[x + 5 * y] = b[x + 5 * y] ^ (!b[(x + 1) % 5 + 5 * y] & b[(x + 2) % 5 + 5 * y]);
            }
        }
        // ι
        a[0] ^= rc;
    }
}

/// The sponge: absorb `input` at `rate` bytes/round with
/// `domain` suffix, squeeze `out_len` bytes.
fn sponge(input: &[u8], rate: usize, domain: u8, out_len: usize) -> Vec<u8> {
    let mut a = [0u64; 25];
    // absorb full rate-sized blocks
    let mut off = 0usize;
    while off + rate <= input.len() {
        absorb_block(&mut a, &input[off..off + rate]);
        keccak_f(&mut a);
        off += rate;
    }
    // pad the tail with pad10*1 around the domain byte
    let mut block = vec![0u8; rate];
    block[..input.len() - off].copy_from_slice(&input[off..]);
    block[input.len() - off] ^= domain;
    block[rate - 1] ^= 0x80;
    absorb_block(&mut a, &block);
    keccak_f(&mut a);
    // squeeze — lane order, little-endian bytes
    let mut out = Vec::with_capacity(out_len);
    while out.len() < out_len {
        let take = (out_len - out.len()).min(rate);
        for i in 0..take {
            let lane = i / 8;
            let byte = (i % 8) as u32;
            out.push((a[lane] >> (8 * byte)) as u8);
        }
        if out.len() < out_len {
            keccak_f(&mut a);
        }
    }
    out
}

fn absorb_block(a: &mut [u64; 25], block: &[u8]) {
    for (i, &b) in block.iter().enumerate() {
        a[i / 8] ^= u64::from(b) << (8 * (i % 8));
    }
}

/// SHA3-256 — 32-byte digest, rate 136 bytes.
pub fn sha3_256(input: &[u8]) -> [u8; 32] {
    let v = sponge(input, 136, 0x06, 32);
    let mut out = [0u8; 32];
    out.copy_from_slice(&v);
    out
}

/// SHA3-512 — 64-byte digest, rate 72 bytes.
pub fn sha3_512(input: &[u8]) -> [u8; 64] {
    let v = sponge(input, 72, 0x06, 64);
    let mut out = [0u8; 64];
    out.copy_from_slice(&v);
    out
}

/// SHAKE128 — XOF, `out_len` bytes, rate 168.
pub fn shake128(input: &[u8], out_len: usize) -> Vec<u8> {
    sponge(input, 168, 0x1f, out_len)
}

/// SHAKE256 — XOF, `out_len` bytes, rate 136.
pub fn shake256(input: &[u8], out_len: usize) -> Vec<u8> {
    sponge(input, 136, 0x1f, out_len)
}

/// Streaming SHA3-256 — `update` in any chunking, then
/// `finalize` once. Part of the same API surface as
/// `sha256::Digest` / `sha512::Digest` for parity.
#[derive(Clone)]
pub struct Digest256 {
    state: [u64; 25],
    rate: usize,
    domain: u8,
    buf: Vec<u8>,
    squeezed: bool,
    pos: usize, // read position inside the current rate block
}

impl Digest256 {
    /// SHA3-256 sponge.
    pub fn new() -> Self {
        Self {
            state: [0; 25],
            rate: 136,
            domain: 0x06,
            buf: Vec::new(),
            squeezed: false,
            pos: 0,
        }
    }

    /// SHAKE128 sponge — same driver, different domain/rate.
    pub fn shake128() -> Self {
        Self {
            state: [0; 25],
            rate: 168,
            domain: 0x1f,
            buf: Vec::new(),
            squeezed: false,
            pos: 0,
        }
    }

    /// Buffer then absorb all complete rate blocks.
    pub fn update(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
        while self.buf.len() >= self.rate {
            let block: Vec<u8> = self.buf.drain(..self.rate).collect();
            absorb_block(&mut self.state, &block);
            keccak_f(&mut self.state);
        }
    }

    /// Pad, permute once more, and squeeze `out_len` bytes.
    pub fn finalize(&mut self, out_len: usize) -> Vec<u8> {
        if self.squeezed {
            // second call = keep squeezing
            let mut out = Vec::new();
            self.squeeze(&mut out, out_len);
            return out;
        }
        let mut block = vec![0u8; self.rate];
        block[..self.buf.len()].copy_from_slice(&self.buf);
        block[self.buf.len()] ^= self.domain;
        block[self.rate - 1] ^= 0x80;
        absorb_block(&mut self.state, &block);
        keccak_f(&mut self.state);
        self.buf.clear();
        self.squeezed = true;
        let mut out = Vec::new();
        self.squeeze(&mut out, out_len);
        out
    }

    fn squeeze(&mut self, out: &mut Vec<u8>, n: usize) {
        for _ in 0..n {
            if self.pos == self.rate {
                keccak_f(&mut self.state);
                self.pos = 0;
            }
            out.push((self.state[self.pos / 8] >> (8 * (self.pos % 8))) as u8);
            self.pos += 1;
        }
    }
}

impl Default for Digest256 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// FIPS-202 vectors — the empty string is the canonical
    /// public constant; longer ones are from the Keccak
    /// team's published list.
    #[test]
    fn nist_vectors() {
        assert_eq!(
            sha3_256(b""),
            [
                0xa7, 0xff, 0xc6, 0xf8, 0xbf, 0x1e, 0xd7, 0x66, 0x51, 0xc1, 0x47, 0x56, 0xa0, 0x61,
                0xd6, 0x62, 0xf5, 0x80, 0xff, 0x4d, 0xe4, 0x3b, 0x49, 0xfa, 0x82, 0xd8, 0x0a, 0x4b,
                0x80, 0xf8, 0x43, 0x4a,
            ]
        );
        assert_eq!(
            sha3_512(b""),
            [
                0xa6, 0x9f, 0x73, 0xcc, 0xa2, 0x3a, 0x9a, 0xc5, 0xc8, 0xb5, 0x67, 0xdc, 0x18, 0x5a,
                0x75, 0x6e, 0x97, 0xc9, 0x82, 0x16, 0x4f, 0xe2, 0x58, 0x59, 0xe0, 0xd1, 0xdc, 0xc1,
                0x47, 0x5c, 0x80, 0xa6, 0x15, 0xb2, 0x12, 0x3a, 0xf1, 0xf5, 0xf9, 0x4c, 0x11, 0xe3,
                0xe9, 0x40, 0x2c, 0x3a, 0xc5, 0x58, 0xf5, 0x00, 0x19, 0x9d, 0x95, 0xb6, 0xd3, 0xe3,
                0x01, 0x75, 0x85, 0x86, 0x28, 0x1d, 0xcd, 0x26,
            ]
        );
        assert_eq!(
            shake128(b"", 32),
            vec![
                0x7f, 0x9c, 0x2b, 0xa4, 0xe8, 0x8f, 0x82, 0x7d, 0x61, 0x60, 0x45, 0x50, 0x76, 0x05,
                0x85, 0x3e, 0xd7, 0x3b, 0x80, 0x93, 0xf6, 0xef, 0xbc, 0x88, 0xeb, 0x1a, 0x6e, 0xac,
                0xfa, 0x66, 0xef, 0x26,
            ]
        );
        assert_eq!(
            shake256(b"", 32),
            vec![
                0x46, 0xb9, 0xdd, 0x2b, 0x0b, 0xa8, 0x8d, 0x13, 0x23, 0x3b, 0x3f, 0xeb, 0x74, 0x3e,
                0xeb, 0x24, 0x3f, 0xcd, 0x52, 0xea, 0x62, 0xb8, 0x1b, 0x82, 0xb5, 0x0c, 0x27, 0x64,
                0x6e, 0xd5, 0x76, 0x2f,
            ]
        );
    }

    /// A non-empty vector: "abc" is the second canonical
    /// FIPS vector.
    #[test]
    fn nist_abc() {
        assert_eq!(
            sha3_256(b"abc"),
            [
                0x3a, 0x98, 0x5d, 0xa7, 0x4f, 0xe2, 0x25, 0xb2, 0x04, 0x5c, 0x17, 0x2d, 0x6b, 0xd3,
                0x90, 0xbd, 0x85, 0x5f, 0x08, 0x6e, 0x3e, 0x9d, 0x52, 0x5b, 0x46, 0xbf, 0xe2, 0x45,
                0x11, 0x43, 0x15, 0x32,
            ]
        );
    }

    /// Streaming = one-shot on every split point.
    #[test]
    fn chunking_independent() {
        let mut rng = SplitMix64::new(0x5EED);
        let data: Vec<u8> = (0..1000).map(|_| rng.below(256) as u8).collect();
        let want = sha3_256(&data);
        for split in [0usize, 1, 7, 136, 137, 200, 999] {
            let mut d = Digest256::new();
            d.update(&data[..split]);
            d.update(&data[split..]);
            assert_eq!(&d.finalize(32)[..], &want[..], "split {split}");
        }
        // shake128 digest follows the same path
        let mut s = Digest256::shake128();
        s.update(&data);
        assert_eq!(s.finalize(32), shake128(&data, 32));
    }

    /// Avalanche: one bit flip → ≈50% of digest bits flip.
    #[test]
    fn avalanche() {
        let a = sha3_256(b"the quick brown fox");
        let b = sha3_256(b"the quick brown fox!"); // wait, need single bit
        let c = sha3_256(b"the quick brown foX");
        let diff1 = a
            .iter()
            .zip(b.iter())
            .map(|(x, y)| (x ^ y).count_ones())
            .sum::<u32>();
        let diff2 = a
            .iter()
            .zip(c.iter())
            .map(|(x, y)| (x ^ y).count_ones())
            .sum::<u32>();
        assert!(diff1 > 60 && diff2 > 60, "{diff1} {diff2}");
    }

    /// Arbitrary output length — squeeze extends correctly
    /// and prefix-stability holds.
    #[test]
    fn xof_prefix_stable() {
        let short = shake128(b"data", 16);
        let long = shake128(b"data", 64);
        assert_eq!(&long[..16], &short[..]);
        // and continuing squeeze gives the tail
        let mut d = Digest256::shake128();
        d.update(b"data");
        let first = d.finalize(16);
        let second = d.finalize(16);
        assert_eq!(&long[..16], &first[..]);
        assert_eq!(&long[16..32], &second[..]);
    }
}
