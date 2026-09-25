//! MT19937 — the Mersenne Twister of Matsumoto & Nishimura (1998).
//!
//! The reference PRNG of NumPy, Ruby, PHP, and `std::mt19937`: a 624-word
//! twisted GFSR with period 2¹⁹⁹³⁷−1. [`Mt19937`] reproduces the standard
//! `init_genrand` seeding and output verbatim, so sequences match the
//! canonical vectors (first outputs for seed 5489 are
//! `0xd091bb5c, 0x22ae9ef6, 0xe7e1faee, …`).
//!
//! For stream-parallel work prefer [`crate::rng::SplitMix64`] or
//! [`crate::rng_xoshiro`]; MT19937 exists for compatibility with the
//! de-facto standard sequence and for its enormous period.
//!
//! ```
//! use izanagi_kit::mt::Mt19937;
//!
//! let mut mt = Mt19937::new(5489);
//! let first = mt.next_u32();
//! assert_eq!(first, 0xd091bb5c); // canonical vector
//! let mut again = Mt19937::new(5489);
//! assert_eq!(again.next_u32(), first); // same seed → same stream
//! ```

/// State size: the recurrence walks a 624-word ring.
pub const N: usize = 624;
/// Twist tap offset.
const M: usize = 397;
/// Twist multiplier: xA_{-1} = xA XOR x·A for the lowest bit.
const MATRIX_A: u32 = 0x9908_b0df;
/// Mask of the top bit (kept from `x[i]`).
const UPPER: u32 = 0x8000_0000;
/// Mask of the low 31 bits (kept from `x[i+1]`).
const LOWER: u32 = 0x7fff_ffff;

/// MT19937 generator: 624-word state + position.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Mt19937 {
    x: [u32; N],
    pos: usize,
}

impl Mt19937 {
    /// Standard `init_genrand` seeding (`x[i+1] = 1812433253·(x[i] ⊕ x[i]>>30) + i`).
    pub fn new(seed: u32) -> Self {
        let mut x = [0u32; N];
        x[0] = seed;
        for i in 1..N {
            let p = x[i - 1];
            x[i] = 1812433253u32
                .wrapping_mul(p ^ (p >> 30))
                .wrapping_add(i as u32);
        }
        Mt19937 { x, pos: N }
    }

    /// `init_by_array` seeding for multi-word keys (RFC-reference variant).
    pub fn from_key(key: &[u32]) -> Self {
        let mut s = Mt19937::new(19650218);
        let mut i = 1;
        let n = N.max(key.len());
        for j in 0..n {
            let p = s.x[i - 1];
            s.x[i] = (s.x[i] ^ ((p ^ (p >> 30)).wrapping_mul(1664525)))
                .wrapping_add(key[j % key.len()])
                .wrapping_add(j as u32);
            i += 1;
            if i >= N {
                s.x[0] = s.x[N - 1];
                i = 1;
            }
        }
        for _ in 0..N - 1 {
            let p = s.x[i - 1];
            s.x[i] = (s.x[i] ^ ((p ^ (p >> 30)).wrapping_mul(1566083941))).wrapping_sub(i as u32);
            i += 1;
            if i >= N {
                s.x[0] = s.x[N - 1];
                i = 1;
            }
        }
        s.x[0] = 0x8000_0000;
        s
    }

    fn twist(&mut self) {
        for i in 0..N {
            let y = (self.x[i] & UPPER) | (self.x[(i + 1) % N] & LOWER);
            let mut v = self.x[(i + M) % N] ^ (y >> 1);
            if y & 1 != 0 {
                v ^= MATRIX_A;
            }
            self.x[i] = v;
        }
        self.pos = 0;
    }

    /// Next raw 32-bit word, tempered (`y ^ y>>11`, `^ (y<<7)&0x9d2c5680`,
    /// `^ (y<<15)&0xefc60000`, `^ y>>18`).
    pub fn next_u32(&mut self) -> u32 {
        if self.pos >= N {
            self.twist();
        }
        let mut y = self.x[self.pos];
        self.pos += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }

    /// `next_u64`: two words, high first.
    pub fn next_u64(&mut self) -> u64 {
        (self.next_u32() as u64) << 32 | self.next_u32() as u64
    }

    /// `genrand_res53`: 53-bit uniform in `[0, 1)` as a `u64` fraction
    /// (multiply by 2⁻⁵³). Integer form keeps the port float-free.
    pub fn next_res53(&mut self) -> u64 {
        let a = (self.next_u32() >> 5) as u64;
        let b = (self.next_u32() >> 6) as u64;
        (a << 26) | b
    }

    /// Uniform `u64` in `[0, n)` via masking rejection (n ≤ 2³², no modulo bias).
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        let zone = u32::MAX - u32::MAX % n;
        loop {
            let v = self.next_u32();
            if v < zone {
                return v % n;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_vector_seed_5489() {
        // First 10 outputs of MT19937(seed=5489), Matsumoto–Nishimura reference.
        let want = [
            0xd091bb5cu32,
            0x22ae9ef6,
            0xe7e1faee,
            0xd5c31f79,
            0x2082352c,
            0xf807b7df,
            0xe9d30005,
            0x3895afe1,
            0xa1e24bba,
            0x4ee4092b,
        ];
        let mut mt = Mt19937::new(5489);
        for (i, &w) in want.iter().enumerate() {
            assert_eq!(mt.next_u32(), w, "word {i}");
        }
    }

    #[test]
    fn seeded_determinism() {
        let mut a = Mt19937::new(42);
        let mut b = Mt19937::new(42);
        for _ in 0..5000 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
        let mut c = Mt19937::new(43);
        assert_ne!(c.next_u32(), a.next_u32());
    }

    #[test]
    fn array_seeding_matches_reference() {
        // init_by_array{0x1234, 0x5678, 0x9abc, 0xdef0} — mt19937ar.c reference.
        let mut mt = Mt19937::from_key(&[0x1234, 0x5678, 0x9abc, 0xdef0]);
        let want = [0x2dba3276u32, 0x6047c08b, 0xedf15300, 0xad9ffff9];
        for (i, &w) in want.iter().enumerate() {
            assert_eq!(mt.next_u32(), w, "word {i}");
        }
    }

    #[test]
    fn res53_and_below() {
        let mut mt = Mt19937::new(7);
        let r = mt.next_res53();
        assert!(r < (1u64 << 53));
        for _ in 0..1000 {
            assert!(mt.below(6) < 6);
        }
        assert_eq!(mt.below(0), 0);
        assert_eq!(mt.below(1), 0);
        // below is unbiased in range: bucket counts stay finite & non-degenerate.
        let mut mt = Mt19937::new(1);
        let mut cnt = [0u32; 4];
        for _ in 0..4000 {
            cnt[mt.below(4) as usize] += 1;
        }
        assert!(cnt.iter().all(|&c| c > 800));
    }

    #[test]
    fn state_round_trip_via_clone() {
        let mut a = Mt19937::new(99);
        for _ in 0..700 {
            a.next_u32();
        }
        let mut b = a.clone();
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn next_u64_uses_two_words() {
        let mut a = Mt19937::new(5);
        let mut b = Mt19937::new(5);
        let x = a.next_u64();
        let w0 = b.next_u32();
        let w1 = b.next_u32();
        assert_eq!(x, ((w0 as u64) << 32) | w1 as u64);
    }
}
