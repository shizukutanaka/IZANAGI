//! Binary-reflected Gray code — the `2ⁿ`-sequence where
//! consecutive codes differ in exactly one bit. Conversions
//! are the classic `g = i ^ (i >> 1)` and its prefix-xor
//! inverse; [`sequence`] materializes the full order and
//! [`SubsetWalk`] walks subsets of `{0..n}` by Gray order,
//! reporting the flipped bit each step.
//!
//! The Gray order is what "changes one element at a time"
//! means for subset enumeration — useful for incremental
//! simulation state where a full rebuild per step would be
//! wasteful.
//!
//! ```
//! use izanagi_kit::gray::{to_gray, from_gray, sequence, SubsetWalk};
//! assert_eq!(to_gray(5), 0b111);
//! assert_eq!(from_gray(0b111), 5);
//! let s = sequence(3).unwrap();
//! assert_eq!(s, vec![0, 1, 3, 2, 6, 7, 5, 4]);
//! for w in s.windows(2) {
//!     assert_eq!((w[0] ^ w[1]).count_ones(), 1); // one-bit steps
//! }
//! let mut w = SubsetWalk::new(3).unwrap();
//! assert_eq!(w.next(), Some(0b000));
//! assert_eq!(w.next(), Some(0b001));
//! ```
//!
//! References: standard BRGC construction (cf. TAOCP
//! 7.2.1.1); the `i ^ (i>>1)` map is its own witness that
//! exactly one bit changes per step.

/// Integer `i` → its Gray code `i ^ (i >> 1)`.
pub fn to_gray(i: u64) -> u64 {
    i ^ (i >> 1)
}

/// Gray code `g` → the integer it encodes (prefix-xor
/// inversion).
pub fn from_gray(mut g: u64) -> u64 {
    let mut i = g;
    while g > 0 {
        g >>= 1;
        i ^= g;
    }
    i
}

/// The full `n`-bit Gray sequence `[to_gray(0), …]` —
/// `2ⁿ` entries, `n ≤ 63` (`None` beyond that or when
/// `2ⁿ` overflows `usize`; the allocation is honest about
/// its size).
pub fn sequence(n: u32) -> Option<Vec<u64>> {
    if n >= 63 {
        return None;
    }
    Some((0..(1u64 << n)).map(to_gray).collect())
}

/// A walk through the `2ⁿ` subsets of `{0..n}` in Gray
/// order, returning each subset as a bitmask and (on all but
/// the first step) which bit flipped.
#[derive(Clone, Debug)]
pub struct SubsetWalk {
    i: u64,
    n: u32,
    prev: u64,
    exhausted: bool,
}

impl SubsetWalk {
    /// Walk subsets of `{0..n}` — `None` for `n ≥ 63`.
    pub fn new(n: u32) -> Option<SubsetWalk> {
        if n >= 63 {
            return None;
        }
        Some(SubsetWalk {
            i: 0,
            n,
            prev: 0,
            exhausted: false,
        })
    }
}

impl Iterator for SubsetWalk {
    type Item = u64;

    /// Next subset mask, or `None` after `2ⁿ` steps.
    fn next(&mut self) -> Option<u64> {
        if self.i >= (1u64 << self.n) {
            self.exhausted = true;
            return None;
        }
        let g = to_gray(self.i);
        self.prev = g;
        self.i += 1;
        Some(g)
    }
}

impl SubsetWalk {
    /// The bit that flipped on the last `next` (`None` on
    /// the first call, or after exhaustion).
    pub fn last_flip(&self) -> Option<u32> {
        if self.i == 0 || self.i == 1 || self.exhausted {
            return None;
        }
        let diff = self.prev ^ to_gray(self.i - 2);
        Some(diff.trailing_zeros())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basics() {
        assert_eq!(to_gray(0), 0);
        assert_eq!(to_gray(5), 0b111);
        assert_eq!(from_gray(0b111), 5);
        assert_eq!(sequence(0), Some(vec![0]));
        assert_eq!(sequence(3), Some(vec![0, 1, 3, 2, 6, 7, 5, 4]));
        assert!(sequence(63).is_none());
    }

    /// Roundtrip is a bijection and every consecutive pair
    /// differs in exactly one bit — the defining property.
    #[test]
    fn roundtrip_and_unit_steps() {
        let mut rng = SplitMix64::new(0x62A1);
        for _ in 0..5000 {
            let i = u64::from(rng.next_u64() as u32);
            assert_eq!(from_gray(to_gray(i)), i);
        }
        for n in [2u32, 5, 8, 12] {
            let s = sequence(n).unwrap();
            // permutation of 0..2^n
            let mut sorted = s.clone();
            sorted.sort_unstable();
            assert_eq!(sorted, (0..(1u64 << n)).collect::<Vec<_>>());
            for w in s.windows(2) {
                assert_eq!((w[0] ^ w[1]).count_ones(), 1);
            }
        }
    }

    /// The walk's flip-report matches the actual mask diff.
    #[test]
    fn walk_reports_flips() {
        let mut w = SubsetWalk::new(6).unwrap();
        let mut prev = w.next().unwrap();
        assert!(w.last_flip().is_none());
        let mut seen = std::collections::BTreeSet::from([prev]);
        while let Some(g) = w.next() {
            let f = w.last_flip().unwrap();
            assert_eq!(prev ^ g, 1u64 << f);
            seen.insert(g);
            prev = g;
        }
        assert_eq!(seen.len(), 64);
        assert_eq!(w.next(), None);
        assert!(SubsetWalk::new(64).is_none());
    }
}
