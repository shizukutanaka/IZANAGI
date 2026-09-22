//! Segment tree over `i64` — range min, max, and sum in `O(log n)` with
//! point updates. Complements [`crate::fenwick::Fenwick`] (prefix sums
//! only): use this when the query is over an arbitrary window — sliding
//! view ranges, terrain bands, "highest threat in radius over time".
//!
//! Layout is the classic iterative bottom-up tree (`size` = next power
//! of two), so node indices and every walk are content-defined.
//!
//! ```
//! use izanagi_kit::segtree::SegTree;
//! let mut t = SegTree::from_slice(&[5, -2, 9, 1, 7]);
//! assert_eq!(t.range_min(1, 4), Some(-2));
//! assert_eq!(t.range_max(0, 3), Some(9));
//! assert_eq!(t.range_sum(0, 5), Some(20));
//! t.set(1, 4);
//! assert_eq!(t.range_min(1, 4), Some(1));
//! ```

/// Range-query segment tree storing (min, max, sum) at every node —
/// one build serves all three query flavours.
pub struct SegTree {
    n: usize,
    /// Power-of-two leaf base.
    size: usize,
    /// `[i;2*size]` — leaves at `size..size+n` (rest padded with
    /// neutral: `i64::MAX` for min, `i64::MIN` for max, 0 for sum).
    mins: Vec<i64>,
    maxs: Vec<i64>,
    sums: Vec<i64>,
}

impl SegTree {
    /// Empty tree of `n` cells (all neutral).
    pub fn new(n: usize) -> Self {
        let size = n.max(1).next_power_of_two();
        Self {
            n,
            size,
            mins: vec![i64::MAX; 2 * size],
            maxs: vec![i64::MIN; 2 * size],
            sums: vec![0; 2 * size],
        }
    }

    /// Build from a value slice in `O(n)`.
    pub fn from_slice(vals: &[i64]) -> Self {
        let mut t = Self::new(vals.len());
        for (i, &v) in vals.iter().enumerate() {
            let j = t.size + i;
            t.mins[j] = v;
            t.maxs[j] = v;
            t.sums[j] = v;
        }
        for i in (1..t.size).rev() {
            t.pull(i);
        }
        t
    }

    /// Number of cells.
    pub fn len(&self) -> usize {
        self.n
    }

    /// `len() == 0`.
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// Recompute node `i` from its children.
    fn pull(&mut self, i: usize) {
        self.mins[i] = self.mins[2 * i].min(self.mins[2 * i + 1]);
        self.maxs[i] = self.maxs[2 * i].max(self.maxs[2 * i + 1]);
        self.sums[i] = self.sums[2 * i] + self.sums[2 * i + 1];
    }

    /// `set(i, v)` — point assign, `O(log n)`. Returns `false` and does
    /// nothing when `i >= n`.
    pub fn set(&mut self, i: usize, v: i64) -> bool {
        if i >= self.n {
            return false;
        }
        let mut j = self.size + i;
        self.mins[j] = v;
        self.maxs[j] = v;
        self.sums[j] = v;
        j /= 2;
        while j >= 1 {
            self.pull(j);
            j /= 2;
        }
        true
    }

    /// Read cell `i` (`None` when `i >= n`).
    pub fn get(&self, i: usize) -> Option<i64> {
        (i < self.n).then(|| self.sums[self.size + i])
    }

    /// Min over `[lo, hi)` — `None` on an empty/out-of-range span.
    pub fn range_min(&self, lo: usize, hi: usize) -> Option<i64> {
        self.query(lo, hi).map(|r| r.0)
    }

    /// Max over `[lo, hi)` — `None` on an empty/out-of-range span.
    pub fn range_max(&self, lo: usize, hi: usize) -> Option<i64> {
        self.query(lo, hi).map(|r| r.1)
    }

    /// Sum over `[lo, hi)` — `None` on an empty/out-of-range span.
    pub fn range_sum(&self, lo: usize, hi: usize) -> Option<i64> {
        self.query(lo, hi).map(|r| r.2)
    }

    /// All three aggregates over `[lo, hi)` in one walk.
    pub fn range_stats(&self, lo: usize, hi: usize) -> Option<(i64, i64, i64)> {
        self.query(lo, hi)
    }

    /// Iterative half-interval walk collecting (min, max, sum).
    fn query(&self, lo: usize, hi: usize) -> Option<(i64, i64, i64)> {
        if lo >= hi || hi > self.n {
            return None;
        }
        let (mut l, mut r) = (lo + self.size, hi + self.size);
        let (mut mn_l, mut mn_r) = (i64::MAX, i64::MAX);
        let (mut mx_l, mut mx_r) = (i64::MIN, i64::MIN);
        let (mut sm_l, mut sm_r) = (0i64, 0i64);
        while l < r {
            if l & 1 == 1 {
                mn_l = mn_l.min(self.mins[l]);
                mx_l = mx_l.max(self.maxs[l]);
                sm_l += self.sums[l];
                l += 1;
            }
            if r & 1 == 1 {
                r -= 1;
                mn_r = self.mins[r].min(mn_r);
                mx_r = self.maxs[r].max(mx_r);
                sm_r += self.sums[r];
            }
            l /= 2;
            r /= 2;
        }
        Some((mn_l.min(mn_r), mx_l.max(mx_r), sm_l + sm_r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn matches_brute_force_on_random_ops() {
        let mut rng = SplitMix64::new(0x5E67);
        for _ in 0..100 {
            let n = 1 + rng.below(40) as usize;
            let mut vals: Vec<i64> = (0..n).map(|_| rng.range(-100, 101) as i64).collect();
            let mut t = SegTree::from_slice(&vals);
            for _ in 0..200 {
                match rng.below(4) {
                    0 => {
                        let i = rng.below(n as u32) as usize;
                        let v = rng.range(-100, 101) as i64;
                        vals[i] = v;
                        assert!(t.set(i, v));
                    }
                    1 => {
                        let lo = rng.below(n as u32 + 1) as usize;
                        let hi = rng.below(n as u32 + 1) as usize;
                        let (lo, hi) = (lo.min(hi), lo.max(hi));
                        if lo == hi {
                            continue;
                        }
                        let w = &vals[lo..hi];
                        assert_eq!(t.range_min(lo, hi), Some(*w.iter().min().unwrap()));
                        assert_eq!(t.range_max(lo, hi), Some(*w.iter().max().unwrap()));
                        assert_eq!(t.range_sum(lo, hi), Some(w.iter().sum::<i64>()));
                        assert_eq!(
                            t.range_stats(lo, hi),
                            Some((
                                *w.iter().min().unwrap(),
                                *w.iter().max().unwrap(),
                                w.iter().sum::<i64>()
                            ))
                        );
                    }
                    2 => {
                        let i = rng.below(n as u32) as usize;
                        assert_eq!(t.get(i), Some(vals[i]));
                    }
                    _ => {
                        // Out-of-range returns None, not a panic.
                        assert_eq!(t.range_min(n, n + 1), None);
                        assert_eq!(t.get(n), None);
                        assert!(!t.set(n, 0));
                    }
                }
            }
        }
    }

    #[test]
    fn edge_cases() {
        let empty = SegTree::new(0);
        assert!(empty.is_empty());
        assert_eq!(empty.range_min(0, 0), None);
        let single = SegTree::from_slice(&[42]);
        assert_eq!(single.range_sum(0, 1), Some(42));
        assert_eq!(single.len(), 1);
    }

    #[test]
    fn deterministic_rebuild() {
        let vals = [3, 1, 4, 1, 5, 9, 2, 6];
        let a = SegTree::from_slice(&vals);
        let b = SegTree::from_slice(&vals);
        assert_eq!(a.range_min(0, 8), b.range_min(0, 8));
        assert_eq!(a.range_sum(2, 6), b.range_sum(2, 6));
    }
}
