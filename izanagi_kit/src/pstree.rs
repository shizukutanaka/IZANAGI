//! Persistent segment tree — the "chairman tree" over a
//! coordinate-compressed `i64` array. Every prefix gets its own
//! root via path-copying insertion (O(log n) new nodes per append,
//! zero mutation of earlier versions), so a range query on
//! `a[l..=r]` is just the difference of two version roots.
//!
//! Queries: `kth` (k-th smallest in a slice), `freq` (exact value
//! count), `range_count` (value-range count). The persistent
//! structure is what [`crate::wavelet`] achieves over bit-planes —
//! this is the pointer-free, arena-allocated equivalent.
//!
//! ```
//! use izanagi_kit::pstree::PersistentTree;
//! let t = PersistentTree::build(&[4, 1, 7, 1, 3]).unwrap();
//! assert_eq!(t.kth(0, 4, 2), Some(3)); // sorted: 1,1,3,4,7
//! assert_eq!(t.freq(0, 4, 1), 2);
//! assert_eq!(t.range_count(0, 4, 2, 7), 3);
//! ```

#[derive(Clone, Copy, Debug, Default)]
struct Node {
    left: u32,
    right: u32,
    cnt: u32,
}

/// A persistent order-statistic tree over an `i64` array.
#[derive(Clone, Debug)]
pub struct PersistentTree {
    xs: Vec<i64>,     // coordinate compression table
    roots: Vec<u32>,  // roots[i] = version for prefix length i
    nodes: Vec<Node>, // arena; index 0 = null node
}

impl PersistentTree {
    /// Build a version per prefix — `None` on an empty array.
    pub fn build(a: &[i64]) -> Option<Self> {
        if a.is_empty() {
            return None;
        }
        let mut xs = a.to_vec();
        xs.sort_unstable();
        xs.dedup();
        let mut nodes = vec![Node::default()]; // null node at 0
        let mut roots = Vec::with_capacity(a.len() + 1);
        roots.push(0u32);
        for &v in a {
            let pos = xs.partition_point(|&x| x < v);
            let root = insert(
                &mut nodes,
                roots[roots.len() - 1] as usize,
                pos,
                0,
                xs.len(),
            );
            roots.push(root);
        }
        Some(PersistentTree { xs, roots, nodes })
    }

    /// Length of the underlying array.
    pub fn len(&self) -> usize {
        self.roots.len() - 1
    }

    /// `len == 0` — always `false` (build rejects empty).
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Distinct values in the array.
    pub fn distinct(&self) -> usize {
        self.xs.len()
    }

    /// `k`-th smallest value (0-indexed) of `a[l..=r]` — `None`
    /// when the range is invalid or `k` is out of bounds.
    pub fn kth(&self, l: usize, r: usize, k: usize) -> Option<i64> {
        if l > r || r >= self.len() || k > r - l {
            return None;
        }
        let mut lo = 0usize;
        let mut hi = self.xs.len();
        let mut rl = self.roots[l] as usize;
        let mut rr = self.roots[r + 1] as usize;
        let mut k = k as u32;
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            let cnt_l = self.nodes[self.nodes[rl].left as usize].cnt;
            let cnt_r = self.nodes[self.nodes[rr].left as usize].cnt;
            let have = cnt_r - cnt_l;
            if k < have {
                rl = self.nodes[rl].left as usize;
                rr = self.nodes[rr].left as usize;
                hi = mid;
            } else {
                k -= have;
                rl = self.nodes[rl].right as usize;
                rr = self.nodes[rr].right as usize;
                lo = mid;
            }
        }
        Some(self.xs[lo])
    }

    /// Count of `a[l..=r]` equal to `v` — `None` on bad range.
    pub fn freq(&self, l: usize, r: usize, v: i64) -> usize {
        if l > r || r >= self.len() {
            return 0;
        }
        let pos = self.xs.partition_point(|&x| x < v);
        if pos >= self.xs.len() || self.xs[pos] != v {
            return 0;
        }
        self.count_at(pos, l, r)
    }

    /// Count of `a[l..=r]` inside `[lo_v, hi_v]` — `None` on bad
    /// range; clamps the value bounds to the compression table.
    pub fn range_count(&self, l: usize, r: usize, lo_v: i64, hi_v: i64) -> usize {
        if l > r || r >= self.len() || lo_v > hi_v {
            return 0;
        }
        let lo = self.xs.partition_point(|&x| x < lo_v);
        let hi = self.xs.partition_point(|&x| x <= hi_v);
        if lo >= hi {
            return 0;
        }
        self.cover_count(l, r, lo, hi)
    }

    /// Total count at compressed index `pos` within a[l..=r].
    fn count_at(&self, pos: usize, l: usize, r: usize) -> usize {
        let mut lo = 0usize;
        let mut hi = self.xs.len();
        let mut rl = self.roots[l] as usize;
        let mut rr = self.roots[r + 1] as usize;
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            if pos < mid {
                rl = self.nodes[rl].left as usize;
                rr = self.nodes[rr].left as usize;
                hi = mid;
            } else {
                rl = self.nodes[rl].right as usize;
                rr = self.nodes[rr].right as usize;
                lo = mid;
            }
        }
        (self.nodes[rr].cnt - self.nodes[rl].cnt) as usize
    }

    /// Count over compressed range `[lo, hi)` within a[l..=r] —
    /// two-sided descent accumulating left covers.
    fn cover_count(&self, l: usize, r: usize, lo: usize, hi: usize) -> usize {
        fn rec(
            nodes: &[Node],
            rl: usize,
            rr: usize,
            nl: usize,
            nh: usize,
            ql: usize,
            qh: usize,
        ) -> usize {
            if qh <= nl || nh <= ql {
                return 0;
            }
            if ql <= nl && nh <= qh {
                return (nodes[rr].cnt - nodes[rl].cnt) as usize;
            }
            let mid = (nl + nh) / 2;
            rec(
                nodes,
                nodes[rl].left as usize,
                nodes[rr].left as usize,
                nl,
                mid,
                ql,
                qh,
            ) + rec(
                nodes,
                nodes[rl].right as usize,
                nodes[rr].right as usize,
                mid,
                nh,
                ql,
                qh,
            )
        }
        rec(
            &self.nodes,
            self.roots[l] as usize,
            self.roots[r + 1] as usize,
            0,
            self.xs.len(),
            lo,
            hi,
        )
    }
}

fn insert(nodes: &mut Vec<Node>, prev: usize, pos: usize, lo: usize, hi: usize) -> u32 {
    let mut n = nodes[prev];
    n.cnt += 1;
    let idx = nodes.len() as u32;
    nodes.push(n);
    if hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if pos < mid {
            let c = insert(nodes, nodes[idx as usize].left as usize, pos, lo, mid);
            nodes[idx as usize].left = c;
        } else {
            let c = insert(nodes, nodes[idx as usize].right as usize, pos, mid, hi);
            nodes[idx as usize].right = c;
        }
    }
    idx
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn kth_freq_and_range_count_match_slice_oracles() {
        let mut rng = SplitMix64::new(0xC4A1);
        for _ in 0..200 {
            let n = 1 + rng.below(40) as usize;
            let a: Vec<i64> = (0..n).map(|_| rng.below(20) as i64 - 10).collect();
            let t = PersistentTree::build(&a).unwrap();
            assert_eq!(t.len(), n);
            for _ in 0..50 {
                let l = rng.below(n as u32) as usize;
                let r = l + rng.below(n as u32 - l as u32) as usize;
                let mut sorted: Vec<i64> = a[l..=r].to_vec();
                sorted.sort_unstable();
                // kth
                let k = rng.below(sorted.len() as u32) as usize;
                assert_eq!(t.kth(l, r, k), Some(sorted[k]));
                // freq
                let v = rng.below(24) as i64 - 12;
                assert_eq!(t.freq(l, r, v), sorted.iter().filter(|&&x| x == v).count());
                // range_count
                let lo = rng.below(24) as i64 - 12;
                let hi = lo + rng.below(12) as i64;
                assert_eq!(
                    t.range_count(l, r, lo, hi),
                    sorted.iter().filter(|&&x| x >= lo && x <= hi).count()
                );
            }
            // version purity: earlier roots answer prefix queries
            let mid = n / 2;
            let mut pref: Vec<i64> = a[..=mid].to_vec();
            pref.sort_unstable();
            assert_eq!(t.kth(0, mid, 0), pref.first().copied());
            // invalid inputs
            assert!(t.kth(0, n, 0).is_none());
            assert!(t.kth(1, 0, 0).is_none());
            assert_eq!(t.freq(0, n - 1, i64::MAX), 0);
            assert_eq!(t.range_count(0, n - 1, 10, 5), 0);
        }
    }

    #[test]
    fn versions_do_not_alias() {
        let t = PersistentTree::build(&[9, 3, 9, 1]).unwrap();
        // prefix 0..=1 sorted [3,9]; prefix all sorted [1,3,9,9]
        assert_eq!(t.kth(0, 1, 0), Some(3));
        assert_eq!(t.kth(0, 3, 0), Some(1));
        assert_eq!(t.kth(0, 3, 3), Some(9));
        assert_eq!(t.freq(0, 3, 9), 2);
        assert_eq!(t.distinct(), 3);
        assert!(!t.is_empty());
        assert!(PersistentTree::build(&[]).is_none());
    }
}
