//! Halton low-discrepancy sequence — deterministic quasi-random
//! points via radical inversion in a set of bases, returned as exact
//! rationals. No floating point: `radical_inverse(b, i)` is the
//! digit-reversal `i / b^k`, a reduced [`Frac`], so the entire
//! sequence is a pure function of `(bases, index)` and replays
//! bit-identically anywhere.
//!
//! Stratification is the honest claim: the first `b^k` terms of a
//! base-`b` Halton sequence hit every `j / b^k` cell exactly once —
//! that is a property test, not a promise about floats.
//!
//! ```
//! use izanagi_kit::frac::Frac;
//! use izanagi_kit::halton::{radical_inverse, Halton};
//! assert_eq!(radical_inverse(2, 5), Frac::new(5, 8)); // 101b -> .101
//! let mut h = Halton::new(&[2, 3]);
//! let p = h.next().unwrap();
//! assert_eq!(p[0], Frac::new(1, 2));
//! assert_eq!(p[1], Frac::new(1, 3));
//! ```
//!
//! Reference: Halton (1964), "Algorithm 247: Radical-inverse
//! quasi-random point sequence" (CACM).

use crate::frac::Frac;

/// `i` written base-`b`, digits reversed after the radix point —
/// returned as the reduced rational `n / b^k`. `i == 0` maps to `0`.
pub fn radical_inverse(base: u32, mut index: u64) -> Frac {
    if index == 0 || base == 0 {
        return Frac::from_int(0);
    }
    if base == 1 {
        return Frac::from_int(index as i128);
    }
    let (mut num, mut den, b) = (0i128, 1i128, base as i128);
    while index > 0 {
        num = num * b + (index % base as u64) as i128;
        den *= b;
        index /= base as u64;
    }
    Frac::new(num, den)
}

/// Multi-dimensional Halton generator: one [`radical_inverse`] per
/// base. Bases should be pairwise coprime (primes are the classic
/// choice) — correlated bases degrade the point set's uniformity.
pub struct Halton {
    bases: Vec<u32>,
    index: u64,
}

impl Halton {
    /// Fresh sequence at index 1 — the classic convention, so the
    /// degenerate all-zero point at index 0 is skipped.
    pub fn new(bases: &[u32]) -> Halton {
        Halton {
            bases: bases.to_vec(),
            index: 1,
        }
    }

    /// Point at absolute `index` — pure function, no cursor move.
    pub fn point(&self, index: u64) -> Vec<Frac> {
        self.bases
            .iter()
            .map(|&b| radical_inverse(b, index))
            .collect()
    }

    /// Restart the sequence at index 1.
    pub fn reset(&mut self) {
        self.index = 1;
    }
}

impl Iterator for Halton {
    type Item = Vec<Frac>;
    /// Next point — `bases.len()` exact rationals in `(0, 1)`.
    /// Never `None`; the cursor saturates at `u64::MAX`.
    fn next(&mut self) -> Option<Vec<Frac>> {
        let out = self.point(self.index);
        self.index = self.index.saturating_add(1);
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Slow oracle: direct `Σ dᵢ·b^{-(i+1)}` accumulation in `Frac`.
    fn oracle_inverse(base: u32, index: u64) -> Frac {
        if index == 0 {
            return Frac::from_int(0);
        }
        let mut digits = Vec::new();
        let mut i = index;
        while i > 0 {
            digits.push(i % base as u64);
            i /= base as u64;
        }
        // value = Σ_{j} digits[j] / base^{j+1}
        let mut v = Frac::from_int(0);
        for &d in digits.iter().rev() {
            v = v + Frac::new(d as i128, 1);
            v = match v.checked_div(Frac::from_int(base as i128)) {
                Some(q) => q,
                None => Frac::from_int(0),
            };
        }
        // fold: v = (((d_{k-1}/b + d_{k-2})/b + ...) + d_0)/b
        v
    }

    #[test]
    fn known_vectors() {
        assert_eq!(radical_inverse(2, 1), Frac::new(1, 2));
        assert_eq!(radical_inverse(2, 2), Frac::new(1, 4));
        assert_eq!(radical_inverse(2, 3), Frac::new(3, 4));
        assert_eq!(radical_inverse(2, 6), Frac::new(3, 8));
        assert_eq!(radical_inverse(3, 4), Frac::new(4, 9)); // 11_3 -> .11
        assert_eq!(radical_inverse(5, 7), Frac::new(11, 25)); // 12_5 -> .21 = 2/5+1/25
        assert_eq!(radical_inverse(2, 0), Frac::from_int(0));
        assert_eq!(radical_inverse(1, 4), Frac::from_int(4));
        assert_eq!(radical_inverse(0, 9), Frac::from_int(0));
    }

    #[test]
    fn digit_reversal_oracle() {
        let mut rng = SplitMix64::new(0x8a17_ba5e_5eed_0001);
        for _ in 0..2000 {
            let base = 2 + rng.below(12);
            let index = rng.next_u64() & 0xffff_ffff;
            assert_eq!(
                radical_inverse(base, index),
                oracle_inverse(base, index),
                "base {base} index {index}"
            );
        }
    }

    #[test]
    fn stratification_oracle() {
        // First b^k terms of base-b hit every j/b^k cell exactly once.
        for (base, k) in [(2u32, 6u32), (3, 4), (5, 3)] {
            let n = (base as u64).pow(k);
            let mut cells: Vec<i64> = (0..n)
                .map(|i| {
                    (radical_inverse(base, i).num * n as i128 / radical_inverse(base, i).den) as i64
                })
                .collect();
            cells.sort_unstable();
            for (j, &c) in cells.iter().enumerate() {
                assert_eq!(c, j as i64);
            }
        }
    }

    #[test]
    fn sequence_and_reset() {
        let mut h = Halton::new(&[2, 3]);
        let first: Vec<Frac> = h.next().unwrap();
        assert_eq!(first[0], Frac::new(1, 2));
        assert_eq!(first[1], Frac::new(1, 3));
        let second = h.next().unwrap();
        assert_eq!(second[0], Frac::new(1, 4));
        assert_eq!(second[1], Frac::new(2, 3));
        // point() does not move the cursor.
        assert_eq!(h.point(9), Halton::new(&[2, 3]).point(9));
        h.reset();
        assert_eq!(h.next().unwrap(), first);
    }

    #[test]
    fn range_in_unit_interval() {
        let mut rng = SplitMix64::new(77);
        for _ in 0..3000 {
            let base = 2 + rng.below(7);
            let index = rng.next_u64() & 0x7fff_ffff;
            let f = radical_inverse(base, index);
            assert!(f.num >= 0 && f.num < f.den);
        }
    }
}
