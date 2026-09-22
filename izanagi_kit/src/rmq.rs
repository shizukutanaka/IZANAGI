//! Sparse table — static range-minimum/maximum queries in `O(1)` after
//! `O(n log n)` build, on `i64` slices.
//!
//! The complement to [`crate::segtree::SegTree`]: segtree supports
//! *point updates*, this is strictly faster to query and immutable —
//! use it for static data queried hot (terrain columns, baked cost
//! fields, precomputed wave envelopes). For min/max the operation is
//! idempotent, so overlapping blocks compose directly — no
//! disjoint-cover bookkeeping needed.
//!
//! ```
//! use izanagi_kit::rmq::SparseTable;
//! let t = SparseTable::new(&[4, 2, 7, 1, 9, 3]);
//! assert_eq!(t.range_min(1, 5), Some(1));
//! assert_eq!(t.range_max(0, 3), Some(7));
//! assert_eq!(t.range_min(3, 3), None); // empty range
//! ```

/// Static sparse table over `i64`: `table[k][i]` aggregates the
/// `2^k`-wide block starting at `i`.
pub struct SparseTable {
    n: usize,
    /// `logs[i]` = floor(log2(i)) — used to pick the covering width.
    logs: Vec<usize>,
    mins: Vec<Vec<i64>>,
    maxs: Vec<Vec<i64>>,
}

impl SparseTable {
    /// Build over `vals` in `O(n log n)`. Empty input yields a table
    /// whose every query returns `None`.
    pub fn new(vals: &[i64]) -> Self {
        let n = vals.len();
        let mut logs = vec![0usize; n + 1];
        for i in 2..=n {
            logs[i] = logs[i / 2] + 1;
        }
        let levels = if n == 0 { 0 } else { logs[n] + 1 };
        let mut mins = Vec::with_capacity(levels);
        let mut maxs = Vec::with_capacity(levels);
        if levels > 0 {
            mins.push(vals.to_vec());
            maxs.push(vals.to_vec());
            for k in 1..levels {
                let w = 1usize << (k - 1); // half-width
                let count = n + 1 - (w * 2);
                let (mut mn, mut mx) = (Vec::with_capacity(count), Vec::with_capacity(count));
                for i in 0..count {
                    mn.push(mins[k - 1][i].min(mins[k - 1][i + w]));
                    mx.push(maxs[k - 1][i].max(maxs[k - 1][i + w]));
                }
                mins.push(mn);
                maxs.push(mx);
            }
        }
        Self {
            n,
            logs,
            mins,
            maxs,
        }
    }

    /// Number of elements.
    pub fn len(&self) -> usize {
        self.n
    }

    /// `len() == 0`.
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Min over `[lo, hi)` — `None` on empty or out-of-range spans.
    /// `O(1)`: min of two overlapping blocks whose union is the range.
    pub fn range_min(&self, lo: usize, hi: usize) -> Option<i64> {
        if lo >= hi || hi > self.n {
            return None;
        }
        let k = self.logs[hi - lo];
        let w = 1usize << k;
        Some(self.mins[k][lo].min(self.mins[k][hi - w]))
    }

    /// Max over `[lo, hi)` — `None` on empty or out-of-range spans.
    pub fn range_max(&self, lo: usize, hi: usize) -> Option<i64> {
        if lo >= hi || hi > self.n {
            return None;
        }
        let k = self.logs[hi - lo];
        let w = 1usize << k;
        Some(self.maxs[k][lo].max(self.maxs[k][hi - w]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn matches_brute_force_every_range() {
        let mut rng = SplitMix64::new(0xF11E);
        for _ in 0..200 {
            let n = 1 + rng.below(50) as usize;
            let vals: Vec<i64> = (0..n).map(|_| rng.range(-500, 501) as i64).collect();
            let t = SparseTable::new(&vals);
            // Exhaustive check on small n: all ranges vs brute force.
            for lo in 0..n {
                for hi in lo + 1..=n {
                    let w = &vals[lo..hi];
                    assert_eq!(t.range_min(lo, hi), Some(*w.iter().min().unwrap()));
                    assert_eq!(t.range_max(lo, hi), Some(*w.iter().max().unwrap()));
                }
            }
        }
    }

    #[test]
    fn edge_cases() {
        let empty = SparseTable::new(&[]);
        assert!(empty.is_empty());
        assert_eq!(empty.range_min(0, 0), None);
        let one = SparseTable::new(&[42]);
        assert_eq!(one.len(), 1);
        assert!(!one.is_empty());
        assert_eq!(one.range_min(0, 1), Some(42));
        assert_eq!(one.range_max(0, 1), Some(42));
        // Out-of-range → None, no panic.
        assert_eq!(one.range_min(0, 2), None);
        assert_eq!(one.range_min(1, 1), None);
    }

    #[test]
    fn full_range_and_single_element() {
        let t = SparseTable::new(&[3, 1, 4, 1, 5, 9, 2, 6]);
        assert_eq!(t.range_min(0, 8), Some(1));
        assert_eq!(t.range_max(0, 8), Some(9));
        assert_eq!(t.range_min(3, 4), Some(1));
    }
}
