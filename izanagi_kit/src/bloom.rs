//! Seeded Bloom filter — a set-membership oracle with one-sided error:
//! `contains` may return true for absent items (false positives) but
//! never false for inserted ones (no false negatives). Uses Kirsch–
//! Mitzenmacher double hashing `h_i = h1 + i·h2` so a single hash pair
//! seeds all `k` probes, making the filter a pure function of
//! `(seed, params, inserted multiset)`.
//!
//! Intended use in the kit: fast "probably not present" pre-filters
//! ahead of expensive exact lookups (replay checkpoints, entity-id
//! dedup on merge), where a false positive merely costs one wasted
//! exact check.
//!
//! ```
//! use izanagi_kit::bloom::Bloom;
//! let mut b = Bloom::new(1024, 4);
//! b.insert(42);
//! b.insert(7);
//! assert!(b.contains(42));
//! // `contains` on a never-inserted value may be a false positive
//! // but an inserted value is always reported present.
//! ```

use crate::world_hash::Fnv1a;

const DOMAIN_A: u64 = 0x424c_4f4f_4d41_2020; // "BLOOMA  "
const DOMAIN_B: u64 = 0x424c_4f4f_4d42_2020; // "BLOOMB  "

fn h(seed: u64, domain: u64, x: u64) -> u64 {
    let mut h = Fnv1a::new();
    h.write_u64(domain);
    h.write_u64(seed);
    h.write_u64(x);
    h.finish()
}

/// A bit array of `m` bits consulted with `k` double-hash probes.
/// `seed` salts both hashes so the same logical filter can be
/// re-randomized (e.g. per replay epoch) without resizing.
#[derive(Debug, Clone)]
pub struct Bloom {
    bits: Vec<u64>,
    m: usize,
    k: u32,
    seed: u64,
}

impl Bloom {
    /// Filter with `m` bits (rounded up to a 64-word multiple) and `k`
    /// probes per item, unseeded.
    pub fn new(m: usize, k: u32) -> Self {
        Self::with_seed(m, k, 0)
    }

    /// Seeded variant.
    pub fn with_seed(m: usize, k: u32, seed: u64) -> Self {
        let words = m.div_ceil(64).max(1);
        Self {
            bits: vec![0; words],
            m: words * 64,
            k,
            seed,
        }
    }

    /// Capacity heuristic: bits-per-item ≈ 8.5 gives ~2% FPR at k=6.
    /// Returns `(m, k)` for `n` expected items.
    pub fn sizing(n: usize) -> (usize, u32) {
        ((n.max(1)) * 34 / 4, 6)
    }

    /// Bit count `m`.
    pub fn bits_len(&self) -> usize {
        self.m
    }

    /// Probe count `k`.
    pub fn probes(&self) -> u32 {
        self.k
    }

    /// Insert `x` — sets all `k` probe bits.
    pub fn insert(&mut self, x: u64) {
        let (h1, h2) = self.hashes(x);
        for i in 0..self.k as u64 {
            self.set_bit(self.probe(h1, h2, i));
        }
    }

    /// `true` iff every probe bit is set — possibly a false positive.
    pub fn contains(&self, x: u64) -> bool {
        let (h1, h2) = self.hashes(x);
        (0..self.k as u64).all(|i| self.bit(self.probe(h1, h2, i)))
    }

    /// `true` iff `x` is provably absent — `!contains` without the
    /// false-positive hedge. Equivalent but names the safe direction.
    pub fn definitely_absent(&self, x: u64) -> bool {
        !self.contains(x)
    }

    /// Fraction of bits set — a load proxy (FPR ≈ load^k).
    pub fn load_permille(&self) -> u32 {
        let set: u32 = self.bits.iter().map(|w| w.count_ones()).sum();
        (set * 1000) / self.m as u32
    }

    /// Clear all bits (seed and geometry retained).
    pub fn clear(&mut self) {
        self.bits.iter_mut().for_each(|w| *w = 0);
    }

    fn hashes(&self, x: u64) -> (u64, u64) {
        (h(self.seed, DOMAIN_A, x), h(self.seed, DOMAIN_B, x))
    }

    fn probe(&self, h1: u64, h2: u64, i: u64) -> usize {
        h1.wrapping_add(i.wrapping_mul(h2)) as usize % self.m
    }

    fn set_bit(&mut self, i: usize) {
        self.bits[i / 64] |= 1 << (i % 64);
    }

    fn bit(&self, i: usize) -> bool {
        self.bits[i / 64] >> (i % 64) & 1 == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    #[test]
    fn no_false_negatives_ever() {
        let mut rng = SplitMix64::new(0xF173);
        for &m in &[64usize, 200, 1024, 5000] {
            for &k in &[1u32, 3, 6, 10] {
                let mut b = Bloom::with_seed(m, k, rng.next_u64());
                let inserted: BTreeSet<u64> = (0..500).map(|_| rng.next_u64()).collect();
                for &x in &inserted {
                    b.insert(x);
                }
                for &x in &inserted {
                    assert!(b.contains(x), "m={m} k={k}");
                    assert!(!b.definitely_absent(x));
                }
            }
        }
    }

    #[test]
    fn measured_fpr_within_information_bound() {
        // For m = 8.5·n bits, k = 6, the classic bound
        // (1 - e^{-kn/m})^k ≈ 0.021. Generous tolerance: < 8%.
        let n = 400usize;
        let (m, k) = Bloom::sizing(n);
        let mut rng = SplitMix64::new(0xDEAD);
        let mut b = Bloom::with_seed(m, k, 0);
        let inserted: Vec<u64> = (0..n).map(|_| rng.next_u64()).collect();
        for &x in &inserted {
            b.insert(x);
        }
        let probe: BTreeSet<u64> = inserted.iter().copied().collect();
        let mut fp = 0u32;
        let trials = 20_000u32;
        for _ in 0..trials {
            let x = rng.next_u64();
            if !probe.contains(&x) && b.contains(x) {
                fp += 1;
            }
        }
        assert!(
            fp * 1000 / trials < 80,
            "fpr {fp}/{trials} exceeded 8% bound"
        );
    }

    #[test]
    fn pure_function_of_inputs() {
        // Same (m, k, seed, inserts) → identical bit pattern.
        let mut a = Bloom::with_seed(512, 4, 7);
        let mut b = Bloom::with_seed(512, 4, 7);
        let mut c = Bloom::with_seed(512, 4, 9); // different seed
        let mut rng = SplitMix64::new(0x9);
        for _ in 0..100 {
            let x = rng.next_u64();
            a.insert(x);
            b.insert(x);
            c.insert(x);
        }
        assert_eq!(a.bits, b.bits);
        assert_ne!(a.bits, c.bits);
        // Insert order is irrelevant (bitwise OR is commutative).
        let mut d = Bloom::with_seed(512, 4, 7);
        let mut rng2 = SplitMix64::new(0x9);
        let mut xs: Vec<u64> = (0..100).map(|_| rng2.next_u64()).collect();
        xs.reverse();
        for x in xs {
            d.insert(x);
        }
        assert_eq!(a.bits, d.bits);
    }

    #[test]
    fn degenerate_and_load() {
        let mut b = Bloom::new(1, 1); // rounds up to 64 bits
        assert_eq!(b.bits_len(), 64);
        assert_eq!(b.load_permille(), 0);
        b.insert(0);
        b.insert(0); // idempotent
        assert!(b.contains(0));
        assert!(b.load_permille() > 0);
        b.clear();
        assert_eq!(b.load_permille(), 0);
        assert!(!b.contains(0));
        assert_eq!(Bloom::sizing(0).1, 6);
        assert_eq!(b.probes(), 1);
    }
}
