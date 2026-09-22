//! Fenwick tree (binary indexed tree) over `i64` — prefix sums and
//! point updates in `O(log n)`, plus an order-statistic `lower_bound`.
//!
//! Deterministic by construction: a plain integer array with fixed
//! lowest-bit arithmetic, no hashing anywhere. Use for running
//! leaderboards, economy ledgers, damage-per-tick accumulators — any
//! `O(log n)` running-total query.
//!
//! ```
//! use izanagi_kit::fenwick::Fenwick;
//! let mut t = Fenwick::new(5);
//! t.add(0, 3);
//! t.add(3, 7);
//! t.add(4, 1);
//! assert_eq!(t.prefix_sum(4), 10); // indices 0..4 → 3+0+0+7
//! assert_eq!(t.total(), 11);
//! assert_eq!(t.lower_bound(2), 0); // prefix(1)=3 already exceeds 2
//! assert_eq!(t.lower_bound(3), 3); // prefix(3)=3 is not >3; cell 3 pushes past
//! ```

/// Fenwick tree. Index API is 0-based; internally the array is shifted
/// by one so `i & -i` gives the responsibility window.
pub struct Fenwick {
    n: usize,
    t: Vec<i64>,
}

impl Fenwick {
    /// Empty tree of `n` cells.
    pub fn new(n: usize) -> Self {
        Self {
            n,
            t: vec![0; n + 1],
        }
    }

    /// Build from a value slice in `O(n)` — each cell stores its own
    /// responsibility window directly instead of `n` sequential adds.
    pub fn from_slice(vals: &[i64]) -> Self {
        let mut t = vec![0i64; vals.len() + 1];
        for (i, &v) in vals.iter().enumerate() {
            t[i + 1] = v;
        }
        for i in 1..=vals.len() {
            let j = i + (i & (!i + 1)); // i + lowestbit(i)
            if j <= vals.len() {
                t[j] += t[i];
            }
        }
        Self { n: vals.len(), t }
    }

    /// Number of cells.
    pub fn len(&self) -> usize {
        self.n
    }

    /// `self.len() == 0`.
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// `add(i, delta)` — point update, `O(log n)`. Returns `false` and
    /// does nothing when `i >= n`.
    pub fn add(&mut self, i: usize, delta: i64) -> bool {
        if i >= self.n {
            return false;
        }
        let mut x = i + 1;
        while x <= self.n {
            self.t[x] += delta;
            x += x & (!x + 1);
        }
        true
    }

    /// Sum of cells `[0, i)` — `O(log n)`. `prefix_sum(0) == 0`,
    /// `prefix_sum(n) == total()`.
    pub fn prefix_sum(&self, i: usize) -> i64 {
        let i = i.min(self.n);
        let mut s = 0i64;
        let mut x = i;
        while x > 0 {
            s += self.t[x];
            x -= x & (!x + 1);
        }
        s
    }

    /// Sum over the inclusive range `[lo, hi]` — `None` on `lo > hi` or
    /// `hi >= n`.
    pub fn range_sum(&self, lo: usize, hi: usize) -> Option<i64> {
        if lo > hi || hi >= self.n {
            return None;
        }
        Some(self.prefix_sum(hi + 1) - self.prefix_sum(lo))
    }

    /// `prefix_sum(n)` — total of all cells.
    pub fn total(&self) -> i64 {
        self.prefix_sum(self.n)
    }

    /// Smallest index `i` such that `prefix_sum(i + 1) > target` — i.e.
    /// the cell where cumulative mass first exceeds `target`. `O(log n)`
    /// bit-descent. Correct for non-negative cell values; with negatives
    /// the answer is still deterministic but not meaningful as an order
    /// statistic. Returns `n` when `target >= total`.
    pub fn lower_bound(&self, target: i64) -> usize {
        let mut idx = 0usize;
        let mut remaining = target;
        // Highest power of two <= n.
        let mut bit = 1usize;
        while bit * 2 <= self.n {
            bit *= 2;
        }
        while bit != 0 {
            let next = idx + bit;
            if next <= self.n && self.t[next] <= remaining {
                idx = next;
                remaining -= self.t[next];
            }
            bit >>= 1;
        }
        idx // already count of cells with prefix_sum <= target
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn matches_brute_force_prefix_sums() {
        let mut rng = SplitMix64::new(0xFE44);
        for _ in 0..100 {
            let n = 1 + rng.below(40) as usize;
            let mut vals: Vec<i64> = (0..n).map(|_| rng.range(-50, 51) as i64).collect();
            let mut tree = Fenwick::from_slice(&vals);
            for op in 0..200 {
                match rng.below(3) {
                    0 => {
                        // update
                        let i = rng.below(n as u32) as usize;
                        let d = rng.range(-20, 21) as i64;
                        vals[i] += d;
                        tree.add(i, d);
                    }
                    1 => {
                        // prefix
                        let i = rng.below(n as u32 + 1) as usize;
                        let brute: i64 = vals[..i].iter().sum();
                        assert_eq!(tree.prefix_sum(i), brute);
                    }
                    _ => {
                        // range
                        let lo = rng.below(n as u32) as usize;
                        let hi = rng.below(n as u32) as usize;
                        let (lo, hi) = (lo.min(hi), lo.max(hi));
                        let brute: i64 = vals[lo..=hi].iter().sum();
                        assert_eq!(tree.range_sum(lo, hi), Some(brute));
                    }
                }
                assert_eq!(tree.total(), vals.iter().sum::<i64>(), "op {op}");
            }
        }
    }

    #[test]
    fn lower_bound_finds_order_statistic() {
        // weights [2, 0, 5, 1, 4] — prefix sums [2, 2, 7, 8, 12].
        let t = Fenwick::from_slice(&[2, 0, 5, 1, 4]);
        assert_eq!(t.lower_bound(0), 0); // first cell already exceeds 0
        assert_eq!(t.lower_bound(1), 0);
        assert_eq!(t.lower_bound(2), 2); // prefix(2)=2 not >2; cell 2 pushes past
        assert_eq!(t.lower_bound(6), 2);
        assert_eq!(t.lower_bound(7), 3);
        assert_eq!(t.lower_bound(11), 4);
        // Strict inequality: target == total means no prefix exceeds → n.
        assert_eq!(t.lower_bound(12), 5);
        assert_eq!(t.lower_bound(13), 5);
    }

    #[test]
    fn lower_bound_edge_cases() {
        let t = Fenwick::from_slice(&[1, 2, 3]);
        assert_eq!(t.lower_bound(5), 2); // prefix: 1,3,6 — first >5 is idx2
        assert_eq!(t.lower_bound(6), 3); // total=6 → past-the-end = n
        let empty = Fenwick::new(0);
        assert_eq!(empty.lower_bound(0), 0);
        assert!(empty.is_empty());
        assert_eq!(empty.total(), 0);
        assert_eq!(empty.range_sum(0, 0), None);
    }
}
