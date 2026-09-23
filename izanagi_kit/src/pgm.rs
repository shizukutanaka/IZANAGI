//! PGM-style learned index over a sorted `u64` array (Ferragina &
//! Vinciguerra 2020, integer-only form). The key space is split into
//! fixed-size segments; each stores a rational slope between its
//! first and last point plus an exact `ε` — the maximum deviation of
//! that line from the true index. A lookup predicts the position and
//! then binary-searches only `[pred−ε, pred+ε]`. It compresses the
//! routing step of a binary search to ~`O(log ε)` while the whole
//! structure stays a pure function of the key array — the sort order
//! and segment boundaries are fully determined by the input.
//!
//! ```
//! use izanagi_kit::pgm::PgmIndex;
//! let keys: Vec<u64> = (0..1000).map(|i| i * 10).collect();
//! let idx = PgmIndex::build(&keys, 64);
//! assert_eq!(idx.get(&250), Some(25));
//! assert_eq!(idx.get(&255), None);
//! assert_eq!(idx.rank(&255), 26); // 26 keys are < 255
//! ```

use std::cmp::Ordering;

struct Seg {
    /// First key covered by this segment (its left boundary).
    first_key: u64,
    /// Slope numerator/denominator for `pos ≈ base + (x−first)·n/d`.
    num: i128,
    den: i128,
    /// First array index of the segment.
    base: usize,
    /// Inclusive end index (exclusive of next segment's base).
    end: usize,
    /// Max |predicted − actual| over the segment.
    eps: usize,
}

impl Seg {
    /// Predicted index for `x`, clamped into `[base, end]`.
    fn predict(&self, x: u64) -> usize {
        let dx = (x.saturating_sub(self.first_key)) as i128;
        let mut p = self.base as i128 + (dx * self.num) / self.den;
        if p < self.base as i128 {
            p = self.base as i128;
        }
        if p > self.end as i128 {
            p = self.end as i128;
        }
        p as usize
    }
}

/// Static learned index over sorted `u64` keys.
pub struct PgmIndex {
    keys: Vec<u64>,
    segs: Vec<Seg>,
}

impl PgmIndex {
    /// Build over `keys` — they are sorted and deduplicated internally
    /// so the index is a pure function of the key multiset.
    /// `seg_size` is the number of keys per segment (≥2).
    pub fn build(keys: &[u64], seg_size: usize) -> Self {
        let mut ks = keys.to_vec();
        ks.sort_unstable();
        ks.dedup();
        let step = seg_size.max(2);
        let mut segs = Vec::new();
        let mut i = 0;
        while i < ks.len() {
            let end = (i + step - 1).min(ks.len() - 1);
            let x0 = ks[i];
            let x1 = ks[end];
            // slope = (end−base)/(x1−x0); for a flat (all-equal span
            // impossible after dedup unless len 1) use slope 0.
            let (num, den) = if x1 == x0 {
                (0i128, 1i128)
            } else {
                ((end - i) as i128, (x1 - x0) as i128)
            };
            let mut seg = Seg {
                first_key: x0,
                num,
                den,
                base: i,
                end,
                eps: 0,
            };
            // Exact worst deviation of the rational line.
            let mut eps = 0usize;
            for (j, &k) in ks.iter().enumerate().take(end + 1).skip(i) {
                let dev = seg.predict(k).abs_diff(j);
                if dev > eps {
                    eps = dev;
                }
            }
            seg.eps = eps;
            segs.push(seg);
            i = end + 1;
        }
        Self { keys: ks, segs }
    }

    fn seg_of(&self, x: u64) -> &Seg {
        // Segments are ordered by first_key — rightmost seg with
        // first_key <= x.
        let i = self
            .segs
            .binary_search_by(|s| {
                if s.first_key <= x {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            })
            .unwrap_or_else(|i| i.saturating_sub(1))
            .min(self.segs.len().saturating_sub(1));
        &self.segs[i]
    }

    /// Index of `x` in the sorted key array, if present.
    pub fn get(&self, x: &u64) -> Option<usize> {
        if self.keys.is_empty() {
            return None;
        }
        let s = self.seg_of(*x);
        let p = s.predict(*x);
        let lo = p.saturating_sub(s.eps).max(s.base);
        let hi = (p + s.eps).min(s.end);
        match self.keys[lo..=hi].binary_search(x) {
            Ok(off) => Some(lo + off),
            Err(_) => None,
        }
    }

    /// `rank(x)` = number of keys strictly less than `x`.
    pub fn rank(&self, x: &u64) -> usize {
        if self.keys.is_empty() {
            return 0;
        }
        let s = self.seg_of(*x);
        let p = s.predict(*x);
        let lo = p.saturating_sub(s.eps).max(s.base);
        let hi = (p + s.eps).min(s.end);
        match self.keys[lo..=hi].binary_search(x) {
            Ok(off) => lo + off,
            Err(ins) => lo + ins,
        }
    }

    /// Number of indexed keys.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// True when empty.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn uniform_data() {
        let keys: Vec<u64> = (0..500).map(|i| i * 3).collect();
        let idx = PgmIndex::build(&keys, 32);
        assert_eq!(idx.get(&0), Some(0));
        assert_eq!(idx.get(&(499 * 3)), Some(499));
        assert_eq!(idx.get(&(250 * 3)), Some(250));
        assert_eq!(idx.get(&7), None);
    }

    #[test]
    fn oracle_brute_rank_and_get() {
        let mut rng = SplitMix64::new(31);
        for trial in 0..30 {
            let n = 1 + rng.below(400) as usize;
            let mut keys: Vec<u64> = (0..n).map(|_| rng.next_u64() % (100 * n as u64)).collect();
            keys.sort_unstable();
            keys.dedup();
            let idx = PgmIndex::build(&keys, 1 + rng.below(64) as usize);
            for _ in 0..300 {
                let x = rng.next_u64() % (100 * n as u64);
                assert_eq!(idx.get(&x), keys.binary_search(&x).ok(), "trial {trial}");
                assert_eq!(
                    idx.rank(&x),
                    keys.partition_point(|&k| k < x),
                    "rank trial {trial}"
                );
            }
        }
    }

    #[test]
    fn skewed_and_duplicates() {
        // Heavily skewed + duplicate inputs still index correctly.
        let mut keys: Vec<u64> = vec![0; 10];
        keys.extend((0..50).map(|i| 1000 + i * i));
        let idx = PgmIndex::build(&keys, 8);
        assert_eq!(idx.len(), 51);
        assert_eq!(idx.get(&0), Some(0));
        // 0 at index 0; 1000 + i² at index i+1.
        assert_eq!(idx.get(&1484), Some(23));
        assert_eq!(idx.get(&1499), None);
    }

    #[test]
    fn edge_empty_single() {
        assert!(PgmIndex::build(&[], 8).is_empty());
        let one = PgmIndex::build(&[9], 8);
        assert_eq!(one.get(&9), Some(0));
        assert_eq!(one.get(&8), None);
        assert_eq!(one.rank(&10), 1);
    }
}
