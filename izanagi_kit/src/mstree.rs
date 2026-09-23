//! Merge-sort tree — a static range-counting index: the same
//! recursion a merge sort performs on the array, except every node
//! *keeps* its sorted run instead of discarding it. A query
//! `[l, r]` decomposes into `O(log n)` nodes, each answered with a
//! binary search — `O(log² n)` per query, `O(n log n)` build.
//!
//! The tree is a pure function of the input array: shape and node
//! contents are fixed before any query runs, and nothing mutates.
//!
//! ```
//! use izanagi_kit::mstree::MergeSortTree;
//! let t = MergeSortTree::new(&[5, 1, 9, 3, 7]);
//! // values in a[1..=3] inside [2, 8]: {3} → 1
//! assert_eq!(t.count_range(1, 3, 2, 8), 1);
//! // values ≤ 4 in a[0..=4]: {1, 3} → 2
//! assert_eq!(t.count_le(0, 4, 4), 2);
//! ```

/// Static merge-sort tree over `u64`.
#[derive(Clone, Debug)]
pub struct MergeSortTree {
    n: usize,
    /// `run[i]` is the sorted content of node `i` (1-indexed
    /// segment tree, leaves are singletons).
    run: Vec<Vec<u64>>,
}

impl MergeSortTree {
    /// Build over `a`; empty input yields a zero-answer index.
    /// Node layout is the recursive mid-split — the same indexing
    /// `query` walks — so arbitrary `n` is fine (not a heap layout).
    pub fn new(a: &[u64]) -> MergeSortTree {
        let n = a.len();
        let mut t = MergeSortTree {
            n,
            run: vec![Vec::new(); 4 * n.max(1) + 2],
        };
        if n > 0 {
            t.build(1, 0, n - 1, a);
        }
        t
    }

    fn build(&mut self, node: usize, nl: usize, nr: usize, a: &[u64]) {
        if nl == nr {
            self.run[node] = vec![a[nl]];
            return;
        }
        let mid = (nl + nr) / 2;
        self.build(2 * node, nl, mid, a);
        self.build(2 * node + 1, mid + 1, nr, a);
        // Sorted merge of the two children's sorted runs.
        let (l, r) = (2 * node, 2 * node + 1);
        let mut merged = Vec::with_capacity(self.run[l].len() + self.run[r].len());
        let (mut x, mut y) = (0usize, 0usize);
        while x < self.run[l].len() || y < self.run[r].len() {
            let take_left = if x == self.run[l].len() {
                false
            } else if y == self.run[r].len() {
                true
            } else {
                self.run[l][x] <= self.run[r][y]
            };
            if take_left {
                merged.push(self.run[l][x]);
                x += 1;
            } else {
                merged.push(self.run[r][y]);
                y += 1;
            }
        }
        self.run[node] = merged;
    }

    /// Number of `a[i]` for `i ∈ [l, r]` with `lo ≤ a[i] ≤ hi`.
    /// `l > r` or an out-of-range query answers 0.
    pub fn count_range(&self, l: usize, r: usize, lo: u64, hi: u64) -> u64 {
        if lo > hi || l > r || l >= self.n {
            return 0;
        }
        let r = r.min(self.n - 1);
        self.query(1, 0, self.n - 1, (l, r, lo, hi))
    }

    /// Number of `a[i] ≤ x` for `i ∈ [l, r]`.
    pub fn count_le(&self, l: usize, r: usize, x: u64) -> u64 {
        self.count_range(l, r, 0, x)
    }

    /// Number of `a[i] ≥ x` for `i ∈ [l, r]`.
    pub fn count_ge(&self, l: usize, r: usize, x: u64) -> u64 {
        self.count_range(l, r, x, u64::MAX)
    }

    /// `count_range` restricted to one node's span; `win` packs the
    /// index window plus the value bounds (l, r, lo, hi).
    fn query(&self, node: usize, nl: usize, nr: usize, win: (usize, usize, u64, u64)) -> u64 {
        let (l, r, lo, hi) = win;
        if nr < l || r < nl {
            return 0;
        }
        if l <= nl && nr <= r {
            let run = &self.run[node];
            let first_lo = run.partition_point(|&v| v < lo);
            let first_hi = run.partition_point(|&v| v <= hi);
            return (first_hi - first_lo) as u64;
        }
        let mid = (nl + nr) / 2;
        self.query(2 * node, nl, mid, win) + self.query(2 * node + 1, mid + 1, nr, win)
    }

    /// Sorted run held by the root (the fully sorted array).
    pub fn sorted(&self) -> &[u64] {
        if self.n == 0 {
            &[]
        } else {
            &self.run[1]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn brute(a: &[u64], l: usize, r: usize, lo: u64, hi: u64) -> u64 {
        a.iter()
            .enumerate()
            .filter(|&(i, v)| i >= l && i <= r && *v >= lo && *v <= hi)
            .count() as u64
    }

    #[test]
    fn basics() {
        let t = MergeSortTree::new(&[5, 1, 9, 3, 7]);
        assert_eq!(t.count_range(1, 3, 2, 8), 1);
        assert_eq!(t.count_le(0, 4, 4), 2);
        assert_eq!(t.count_ge(0, 4, 7), 2);
        assert_eq!(t.count_range(0, 4, 0, u64::MAX), 5);
        assert_eq!(t.count_range(4, 2, 0, 9), 0); // l > r
        assert_eq!(t.sorted(), &[1, 3, 5, 7, 9]);
    }

    #[test]
    fn empty_and_single() {
        let e = MergeSortTree::new(&[]);
        assert_eq!(e.count_range(0, 10, 0, u64::MAX), 0);
        assert_eq!(e.sorted(), &[] as &[u64]);
        let s = MergeSortTree::new(&[42]);
        assert_eq!(s.count_range(0, 0, 42, 42), 1);
        assert_eq!(s.count_range(0, 0, 0, 41), 0);
        assert_eq!(s.count_range(5, 9, 0, u64::MAX), 0);
    }

    #[test]
    fn oracle() {
        let mut rng = SplitMix64::new(0xdead_beef_cafe_f00d);
        for _case in 0..40 {
            let n = 1 + rng.below(64) as usize;
            let a: Vec<u64> = (0..n).map(|_| rng.next_u64() % 40).collect();
            let t = MergeSortTree::new(&a);
            for _q in 0..200 {
                let l = rng.below(n as u32 + 4) as usize;
                let r = rng.below(n as u32 + 4) as usize;
                let lo = rng.next_u64() % 45;
                let hi = rng.next_u64() % 45;
                // `count_range` clamps r to n-1 and rejects l>r / l>=n;
                // the same clamp makes the brute oracle agree exactly.
                assert_eq!(
                    t.count_range(l, r, lo, hi),
                    brute(&a, l, r.min(n - 1), lo, hi)
                );
            }
        }
    }
}
