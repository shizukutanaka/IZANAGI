//! SHA-512 — FIPS 180-4 secure hash over 64-bit words, the digest
//! `ed25519` (RFC 8032) and other wide-hash constructions need.
//! Same streaming contract as `sha256`: 128-byte block pipeline,
//! split-invariant `write`/`finish`, and manual big-endian word
//! assembly (the kit's width rules ban `be_bytes` helpers).
//!
//! ```
//! use izanagi_kit::sha512::Sha512;
//! let mut h = Sha512::new();
//! h.write(b"abc");
//! assert_eq!(
//!     &h.finish()[..4],
//!     &[0xdd, 0xaf, 0x35, 0xa1]
//! );
//! ```

const K: [u64; 80] = [
    0x428a2f98d728ae22,
    0x7137449123ef65cd,
    0xb5c0fbcfec4d3b2f,
    0xe9b5dba58189dbbc,
    0x3956c25bf348b538,
    0x59f111f1b605d019,
    0x923f82a4af194f9b,
    0xab1c5ed5da6d8118,
    0xd807aa98a3030242,
    0x12835b0145706fbe,
    0x243185be4ee4b28c,
    0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f,
    0x80deb1fe3b1696b1,
    0x9bdc06a725c71235,
    0xc19bf174cf692694,
    0xe49b69c19ef14ad2,
    0xefbe4786384f25e3,
    0x0fc19dc68b8cd5b5,
    0x240ca1cc77ac9c65,
    0x2de92c6f592b0275,
    0x4a7484aa6ea6e483,
    0x5cb0a9dcbd41fbd4,
    0x76f988da831153b5,
    0x983e5152ee66dfab,
    0xa831c66d2db43210,
    0xb00327c898fb213f,
    0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2,
    0xd5a79147930aa725,
    0x06ca6351e003826f,
    0x142929670a0e6e70,
    0x27b70a8546d22ffc,
    0x2e1b21385c26c926,
    0x4d2c6dfc5ac42aed,
    0x53380d139d95b3df,
    0x650a73548baf63de,
    0x766a0abb3c77b2a8,
    0x81c2c92e47edaee6,
    0x92722c851482353b,
    0xa2bfe8a14cf10364,
    0xa81a664bbc423001,
    0xc24b8b70d0f89791,
    0xc76c51a30654be30,
    0xd192e819d6ef5218,
    0xd69906245565a910,
    0xf40e35855771202a,
    0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8,
    0x1e376c085141ab53,
    0x2748774cdf8eeb99,
    0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63,
    0x4ed8aa4ae3418acb,
    0x5b9cca4f7763e373,
    0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc,
    0x78a5636f43172f60,
    0x84c87814a1f0ab72,
    0x8cc702081a6439ec,
    0x90befffa23631e28,
    0xa4506cebde82bde9,
    0xbef9a3f7b2c67915,
    0xc67178f2e372532b,
    0xca273eceea26619c,
    0xd186b8c721c0c207,
    0xeada7dd6cde0eb1e,
    0xf57d4f7fee6ed178,
    0x06f067aa72176fba,
    0x0a637dc5a2c898a6,
    0x113f9804bef90dae,
    0x1b710b35131c471b,
    0x28db77f523047d84,
    0x32caab7b40c72493,
    0x3c9ebe0a15c9bebc,
    0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6,
    0x597f299cfc657e2a,
    0x5fcb6fab3ad6faec,
    0x6c44198c4a475817,
];

const H0: [u64; 8] = [
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
];

/// Streaming SHA-512 hasher.
#[derive(Clone)]
pub struct Sha512 {
    h: [u64; 8],
    buf: [u8; 128],
    buf_len: usize,
    total: u128,
}

impl Default for Sha512 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha512 {
    /// Fresh hasher.
    pub fn new() -> Self {
        Self {
            h: H0,
            buf: [0; 128],
            buf_len: 0,
            total: 0,
        }
    }

    /// Feed bytes; any split of the stream yields the same digest.
    pub fn write(&mut self, mut data: &[u8]) {
        self.total = self.total.wrapping_add(data.len() as u128);
        if self.buf_len > 0 {
            let take = (128 - self.buf_len).min(data.len());
            self.buf[self.buf_len..self.buf_len + take].copy_from_slice(&data[..take]);
            self.buf_len += take;
            data = &data[take..];
            if self.buf_len == 128 {
                let b = self.buf;
                self.compress(&b);
                self.buf_len = 0;
            }
        }
        while data.len() >= 128 {
            let (chunk, rest) = data.split_at(128);
            let mut b = [0u8; 128];
            b.copy_from_slice(chunk);
            self.compress(&b);
            data = rest;
        }
        if !data.is_empty() {
            self.buf[..data.len()].copy_from_slice(data);
            self.buf_len = data.len();
        }
    }

    /// Pad, compress the tail, return the 64-byte digest.
    pub fn finish(mut self) -> [u8; 64] {
        let bit_len = self.total.wrapping_mul(8);
        self.write(&[0x80]);
        while self.buf_len != 112 {
            self.write(&[0]);
        }
        // 128-bit big-endian length field, manual shifts
        let mut len_bytes = [0u8; 16];
        for (i, b) in len_bytes.iter_mut().enumerate() {
            *b = (bit_len >> (120 - i * 8)) as u8;
        }
        self.write(&len_bytes);
        let mut out = [0u8; 64];
        for i in 0..8 {
            let v = self.h[i];
            for j in 0..8 {
                out[i * 8 + j] = (v >> (56 - j * 8)) as u8;
            }
        }
        out
    }

    fn compress(&mut self, b: &[u8; 128]) {
        let mut w = [0u64; 80];
        for i in 0..16 {
            // Big-endian word load without be_bytes helpers.
            let mut v = 0u64;
            for j in 0..8 {
                v |= (b[i * 8 + j] as u64) << (56 - j * 8);
            }
            w[i] = v;
        }
        for i in 16..80 {
            let s0 = w[i - 15].rotate_right(1) ^ w[i - 15].rotate_right(8) ^ (w[i - 15] >> 7);
            let s1 = w[i - 2].rotate_right(19) ^ w[i - 2].rotate_right(61) ^ (w[i - 2] >> 6);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b_, mut c, mut d, mut e, mut f, mut g, mut h] = self.h;
        for i in 0..80 {
            let s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
            let ch = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
            let maj = (a & b_) ^ (a & c) ^ (b_ & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b_;
            b_ = a;
            a = t1.wrapping_add(t2);
        }
        self.h[0] = self.h[0].wrapping_add(a);
        self.h[1] = self.h[1].wrapping_add(b_);
        self.h[2] = self.h[2].wrapping_add(c);
        self.h[3] = self.h[3].wrapping_add(d);
        self.h[4] = self.h[4].wrapping_add(e);
        self.h[5] = self.h[5].wrapping_add(f);
        self.h[6] = self.h[6].wrapping_add(g);
        self.h[7] = self.h[7].wrapping_add(h);
    }
}

/// One-shot `sha512(data)`.
pub fn sha512(data: &[u8]) -> [u8; 64] {
    let mut h = Sha512::new();
    h.write(data);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(d: &[u8]) -> String {
        d.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn known_answer_vectors() {
        // FIPS 180-4 / NIST CAVP vectors
        assert_eq!(
            hex(&sha512(b"")),
            "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
        );
        assert_eq!(
            hex(&sha512(b"abc")),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
        assert_eq!(
            hex(&sha512(
                b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"
            )),
            "8e959b75dae313da8cf4f72814fc143f8f7779c6eb9f7fa17299aeadb6889018501d289e4900f7e4331b99dec4b5433ac7d329eeb6dd26545e96e55b874be909"
        );
        // 1_000_000 × 'a'
        let long = vec![b'a'; 1_000_000];
        assert_eq!(
            hex(&sha512(&long)),
            "e718483d0ce769644e2e42c7bc15b4638e1f98b13b2044285632a803afa973ebde0ff244877ea60a4cb0432ce577c31beb009c5c2c49aa2e4eadb217ad8cc09b"
        );
    }

    #[test]
    fn chunked_write_is_split_invariant() {
        let mut rng = crate::rng::SplitMix64::new(11);
        let data: Vec<u8> = (0..3000).map(|_| rng.next_u64() as u8).collect();
        let want = sha512(&data);
        let mut h = Sha512::new();
        let mut i = 0;
        while i < data.len() {
            let n = (rng.below(140) as usize + 1).min(data.len() - i);
            h.write(&data[i..i + n]);
            i += n;
        }
        assert_eq!(h.finish(), want);
    }

    #[test]
    fn block_boundary_lengths() {
        // 111/112/113 bytes straddle the length-field boundary
        for n in [0usize, 1, 111, 112, 113, 127, 128, 129, 255, 256] {
            let a = vec![b'x'; n];
            let d1 = sha512(&a);
            let mut h = Sha512::new();
            for b in &a {
                h.write(&[*b]);
            }
            assert_eq!(h.finish(), d1, "split-invariance failed at {n}");
        }
    }

    #[test]
    fn one_bit_flip_changes_the_digest() {
        let a = sha512(b"seed-value");
        let mut b = *b"seed-value";
        b[3] ^= 1;
        assert_ne!(a, sha512(&b));
    }
}
