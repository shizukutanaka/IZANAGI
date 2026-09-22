//! Interval set — a sorted, disjoint, merged set of `i64` intervals
//! with insert/remove/query, stored as a flat `Vec<(lo, hi)>`.
//!
//! Occupancy and reservation bookkeeping: blocked time ranges, reserved
//! ID windows, wall segments along a scanline, "which tiles are
//! occupied in row y". Half-open `[lo, hi)` semantics throughout, and
//! every query is a binary search — `O(log n)` lookup, `O(n)` mutating
//! insert (documented; the data structure is the sorted vec, not a
//! balanced tree, because sets here are small and read-heavy).
//!
//! Iteration order is ascending — a pure function of the set.
//!
//! ```
//! use izanagi_kit::interval::IntervalSet;
//! let mut s = IntervalSet::new();
//! s.insert(10, 20);
//! s.insert(15, 30);            // merges into [10,30)
//! assert!(s.contains(25));
//! assert!(!s.contains(30));    // half-open
//! assert_eq!(s.intervals(), &[(10, 30)]);
//! ```

/// A sorted set of disjoint half-open intervals `[lo, hi)`.
/// Invariant: strictly ordered, non-adjacent (`ivs[i].1 < ivs[i+1].0`).
#[derive(Clone, Default)]
pub struct IntervalSet {
    ivs: Vec<(i64, i64)>,
}

impl IntervalSet {
    /// Empty set.
    pub fn new() -> Self {
        Self { ivs: Vec::new() }
    }

    /// The disjoint intervals, ascending.
    pub fn intervals(&self) -> &[(i64, i64)] {
        &self.ivs
    }

    /// Total covered length (sum of widths).
    pub fn covered_len(&self) -> i64 {
        self.ivs.iter().map(|&(a, b)| b - a).sum()
    }

    /// Whether `x` is inside some interval.
    pub fn contains(&self, x: i64) -> bool {
        // First interval with lo > x is at `right`; the candidate is
        // the one before it.
        let right = self.ivs.partition_point(|&(lo, _)| lo <= x);
        right > 0 && self.ivs[right - 1].1 > x
    }

    /// Whether `[lo, hi)` overlaps any stored interval.
    pub fn overlaps(&self, lo: i64, hi: i64) -> bool {
        if lo >= hi {
            return false;
        }
        // The first interval whose end passes `lo` overlaps iff it also
        // starts before `hi` (intervals are disjoint and sorted).
        let left = self.ivs.partition_point(|&(_, b)| b <= lo);
        left < self.ivs.len() && self.ivs[left].0 < hi
    }

    /// All stored intervals overlapping `[lo, hi)`, ascending.
    pub fn overlapping(&self, lo: i64, hi: i64) -> Vec<(i64, i64)> {
        if lo >= hi {
            return Vec::new();
        }
        let left = self.ivs.partition_point(|&(_, b)| b <= lo);
        let right = self.ivs.partition_point(|&(a, _)| a < hi);
        self.ivs[left..right].to_vec()
    }

    /// Insert `[lo, hi)`, merging with whatever it touches or bridges.
    /// Empty/reversed ranges are no-ops. `O(n)`.
    pub fn insert(&mut self, lo: i64, hi: i64) {
        if lo >= hi {
            return;
        }
        // First interval whose end is ≥ lo (a candidate for merge).
        let left = self.ivs.partition_point(|&(_, b)| b < lo);
        // First interval whose start is > hi (past the merge zone).
        let right = self.ivs.partition_point(|&(a, _)| a <= hi);
        let (nlo, nhi) = if left < right {
            (self.ivs[left].0.min(lo), self.ivs[right - 1].1.max(hi))
        } else {
            (lo, hi)
        };
        self.ivs.splice(left..right, [(nlo, nhi)]);
    }

    /// Remove `[lo, hi)` — splits an interval it cuts through.
    /// `O(n)`. Empty/reversed ranges are no-ops.
    pub fn remove(&mut self, lo: i64, hi: i64) {
        if lo >= hi {
            return;
        }
        let left = self.ivs.partition_point(|&(_, b)| b <= lo);
        let right = self.ivs.partition_point(|&(a, _)| a < hi);
        let mut pieces: Vec<(i64, i64)> = Vec::with_capacity(2);
        if left < right {
            let (a0, _) = self.ivs[left];
            if a0 < lo {
                pieces.push((a0, lo));
            }
            let (_, b1) = self.ivs[right - 1];
            if b1 > hi {
                pieces.push((hi, b1));
            }
        }
        self.ivs.splice(left..right, pieces);
    }

    /// Intersect in place with `[lo, hi)` — keeps only the covered
    /// portion of each interval.
    pub fn clip(&mut self, lo: i64, hi: i64) {
        if lo >= hi {
            self.ivs.clear();
            return;
        }
        let kept: Vec<(i64, i64)> = self
            .overlapping(lo, hi)
            .iter()
            .map(|&(a, b)| (a.max(lo), b.min(hi)))
            .collect();
        self.ivs = kept;
    }

    /// Number of disjoint intervals.
    pub fn len(&self) -> usize {
        self.ivs.len()
    }

    /// `len() == 0`.
    pub fn is_empty(&self) -> bool {
        self.ivs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    /// Oracle: the point set covered by the interval set.
    fn points(s: &IntervalSet) -> BTreeSet<i64> {
        let mut out = BTreeSet::new();
        for &(a, b) in s.intervals() {
            for x in a..b {
                out.insert(x);
            }
        }
        out
    }

    #[test]
    fn matches_point_set_oracle() {
        let mut rng = SplitMix64::new(0x1A7E);
        for _ in 0..300 {
            let mut s = IntervalSet::new();
            let mut oracle = BTreeSet::new();
            for _ in 0..30 {
                let a = rng.range(-20, 21) as i64;
                let b = rng.range(-20, 21) as i64;
                let (lo, hi) = (a.min(b), a.max(b));
                match rng.below(3) {
                    0 => {
                        s.insert(lo, hi);
                        for x in lo..hi {
                            oracle.insert(x);
                        }
                    }
                    1 => {
                        s.remove(lo, hi);
                        for x in lo..hi {
                            oracle.remove(&x);
                        }
                    }
                    _ => {
                        s.clip(lo, hi);
                        oracle.retain(|&x| x >= lo && x < hi);
                    }
                }
                // Structural invariants each step.
                for w in s.intervals().windows(2) {
                    assert!(w[0].1 < w[1].0, "adjacent/overlapping: {w:?}");
                    assert!(w[0].0 < w[0].1);
                }
            }
            assert_eq!(points(&s), oracle);
            // Queries agree on the dense probe range.
            for x in -22..22i64 {
                assert_eq!(s.contains(x), oracle.contains(&x));
            }
            for lo in -20..20i64 {
                for hi in lo + 1..20 {
                    let probe: Vec<i64> = (lo..hi).collect();
                    let want = probe.iter().any(|x| oracle.contains(x));
                    assert_eq!(s.overlaps(lo, hi), want, "[{lo},{hi})");
                }
            }
        }
    }

    #[test]
    fn merge_and_split_edges() {
        let mut s = IntervalSet::new();
        s.insert(0, 10);
        s.insert(20, 30);
        s.insert(8, 22); // bridges both
        assert_eq!(s.intervals(), &[(0, 30)]);
        s.remove(5, 25); // cuts the middle out
        assert_eq!(s.intervals(), &[(0, 5), (25, 30)]);
        s.remove(0, 30);
        assert!(s.is_empty());
        // No-op degenerates.
        s.insert(5, 5);
        s.insert(9, 2);
        assert!(s.is_empty());
        s.remove(4, 4);
        // Adjacent intervals merge (half-open: [0,5)+[5,9) = [0,9)).
        s.insert(0, 5);
        s.insert(5, 9);
        assert_eq!(s.intervals(), &[(0, 9)]);
    }

    #[test]
    fn covered_len_and_overlapping() {
        let mut s = IntervalSet::new();
        s.insert(0, 4);
        s.insert(10, 20);
        assert_eq!(s.len(), 2);
        assert_eq!(s.covered_len(), 14);
        assert_eq!(s.overlapping(2, 12), vec![(0, 4), (10, 20)]);
        assert_eq!(s.overlapping(5, 9), Vec::<(i64, i64)>::new());
        assert_eq!(s.overlapping(20, 30), Vec::<(i64, i64)>::new()); // half-open
    }
}
