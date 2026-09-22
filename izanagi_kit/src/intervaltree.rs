//! Static interval tree — stabbing and overlap queries over a fixed
//! set of half-open intervals `[lo, hi)`.
//!
//! The classical centered interval tree (Edelsbrunner 1980): pick a
//! pivot between interval endpoints, send intervals strictly left to
//! the left child, strictly right to the right child, and keep the
//! intervals straddling the pivot in two sorted arrays (ascending by
//! `lo`, descending by `hi`). Queries then descend a single search
//! path: a point left of the pivot can only overlap straddling
//! intervals whose `lo <= x`, and the sorted arrays let the scan stop
//! early. `O(n log n)` build, `O(log n + k)` query for `k` answers —
//! where the brute-force scan would be `O(n)` every time.
//!
//! Intervals carry their original indices (`u32`), so results are
//! `Vec<u32>` sorted ascending — a pure function of the interval
//! multiset, independent of input order. Degenerate intervals
//! (`lo >= hi`) are dropped at construction. All arithmetic is on
//! `i64` so negative coordinates work.
//!
//! ```
//! use izanagi_kit::intervaltree::IntervalTree;
//! let t = IntervalTree::new(&[(0, 5), (2, 8), (10, 12)]);
//! assert_eq!(t.stab(3), vec![0, 1]);
//! assert_eq!(t.stab(9), Vec::<u32>::new());
//! assert_eq!(t.overlap(4, 11), vec![0, 1, 2]); // [10,12) touches [4,11)
//! ```
//!
//! Use cases in the sim stack: spawn-zone hit tests, timed buff
//! windows (`status` timers), camera-culled trigger volumes,
//! scheduling overlap checks — anywhere "does this interval meet any
//! of these" is asked repeatedly against a static world layout.

/// One node of the centered interval tree.
#[derive(Clone, Debug)]
struct Node {
    /// Pivot coordinate — strictly inside every `center` interval.
    pivot: i64,
    /// Center intervals `(lo, hi, index)` sorted ascending by `lo`.
    by_lo: Vec<(i64, i64, u32)>,
    /// The same intervals sorted *descending* by `hi`.
    by_hi: Vec<(i64, i64, u32)>,
    /// Intervals entirely left of `pivot`.
    left: Option<Box<Node>>,
    /// Intervals entirely right of `pivot`.
    right: Option<Box<Node>>,
}

impl Node {
    fn build(mut ivs: Vec<(i64, i64, u32)>) -> Option<Box<Node>> {
        if ivs.is_empty() {
            return None;
        }
        // Pivot = midpoint of the endpoint span — strictly inside at
        // least one interval, so every child is a strict subset of
        // this node's intervals (termination). An endpoint *median*
        // can equal a leaf interval's hi and loop forever.
        let (mut lo_end, mut hi_end) = (i64::MAX, i64::MIN);
        for &(l, h, _) in &ivs {
            lo_end = lo_end.min(l);
            hi_end = hi_end.max(h);
        }
        let pivot = lo_end + (hi_end - lo_end) / 2;
        let mut left = Vec::new();
        let mut center = Vec::new();
        let mut right = Vec::new();
        for iv in ivs.drain(..) {
            if iv.1 <= pivot {
                left.push(iv);
            } else if iv.0 > pivot {
                right.push(iv);
            } else {
                center.push(iv); // lo <= pivot < hi
            }
        }
        let mut by_lo = center.clone();
        by_lo.sort_unstable_by_key(|&(l, _, _)| l);
        let mut by_hi = center;
        by_hi.sort_unstable_by_key(|&(_, h, _)| std::cmp::Reverse(h)); // hi descending
        Some(Box::new(Node {
            pivot,
            by_lo,
            by_hi,
            left: Node::build(left),
            right: Node::build(right),
        }))
    }

    fn stab(&self, x: i64, out: &mut Vec<u32>) {
        if x < self.pivot {
            // Only center intervals with lo <= x can contain x.
            for &(l, _, i) in &self.by_lo {
                if l > x {
                    break;
                }
                out.push(i);
            }
            if let Some(l) = &self.left {
                l.stab(x, out);
            }
        } else {
            // Only center intervals with hi > x can contain x
            // (half-open: x == pivot is inside; x < hi required).
            for &(_, h, i) in &self.by_hi {
                if h <= x {
                    break;
                }
                out.push(i);
            }
            if x > self.pivot {
                if let Some(r) = &self.right {
                    r.stab(x, out);
                }
            }
        }
    }

    fn overlap(&self, l: i64, r: i64, out: &mut Vec<u32>) {
        // [l,r) meets a center interval iff its lo < r and hi > l —
        // always true on one side when the query straddles the pivot.
        if r <= self.pivot {
            for &(lo, _, i) in &self.by_lo {
                if lo >= r {
                    break;
                }
                out.push(i);
            }
            if let Some(c) = &self.left {
                c.overlap(l, r, out);
            }
        } else if l > self.pivot {
            for &(_, h, i) in &self.by_hi {
                if h <= l {
                    break;
                }
                out.push(i);
            }
            if let Some(c) = &self.right {
                c.overlap(l, r, out);
            }
        } else {
            // Query straddles the pivot — every center interval
            // contains pivot, so all overlap.
            for &(_, _, i) in &self.by_lo {
                out.push(i);
            }
            if let Some(c) = &self.left {
                c.overlap(l, r, out);
            }
            if let Some(c) = &self.right {
                c.overlap(l, r, out);
            }
        }
    }
}

/// A static centered interval tree over half-open `[lo, hi)`
/// intervals; queries return sorted original indices.
#[derive(Clone, Debug)]
pub struct IntervalTree {
    root: Option<Box<Node>>,
    /// Total intervals held (post-degenerate-drop).
    len: usize,
}

impl IntervalTree {
    /// Build over `intervals` — `(lo, hi)` half-open pairs. `lo >= hi`
    /// entries are dropped; indices into `intervals` are preserved.
    /// `O(n log n)` balanced by endpoint median.
    pub fn new(intervals: &[(i64, i64)]) -> Self {
        let ivs: Vec<(i64, i64, u32)> = intervals
            .iter()
            .enumerate()
            .filter(|(_, &(l, h))| l < h)
            .map(|(i, &(l, h))| (l, h, i as u32))
            .collect();
        let len = ivs.len();
        IntervalTree {
            root: Node::build(ivs),
            len,
        }
    }

    /// Number of intervals held (degenerate ones dropped).
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the tree holds no intervals.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Stabbing query — indices of all intervals with `lo <= x < hi`,
    /// sorted ascending.
    pub fn stab(&self, x: i64) -> Vec<u32> {
        let mut out = Vec::new();
        if let Some(r) = &self.root {
            r.stab(x, &mut out);
        }
        out.sort_unstable();
        out
    }

    /// Overlap query — indices of all intervals with
    /// `lo < r && l < hi`, sorted ascending. An empty query
    /// (`l == r`) degenerates to [`stab`]; an inverted query
    /// (`l > r`) returns empty.
    ///
    /// [`stab`]: Self::stab
    pub fn overlap(&self, l: i64, r: i64) -> Vec<u32> {
        if l == r {
            return self.stab(l);
        }
        let mut out = Vec::new();
        if l < r {
            if let Some(rt) = &self.root {
                rt.overlap(l, r, &mut out);
            }
        }
        out.sort_unstable();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn oracle_stab(ivs: &[(i64, i64)], x: i64) -> Vec<u32> {
        let mut v: Vec<u32> = ivs
            .iter()
            .enumerate()
            .filter(|(_, &(l, h))| l <= x && x < h)
            .map(|(i, _)| i as u32)
            .collect();
        v.sort_unstable();
        v
    }

    fn oracle_overlap(ivs: &[(i64, i64)], l: i64, r: i64) -> Vec<u32> {
        if l == r {
            return oracle_stab(ivs, l); // empty query degenerates to a stab
        }
        let mut v: Vec<u32> = ivs
            .iter()
            .enumerate()
            .filter(|(_, &(lo, hi))| lo < hi && lo < r && l < hi)
            .map(|(i, _)| i as u32)
            .collect();
        v.sort_unstable();
        v
    }

    #[test]
    fn stab_and_overlap_match_brute_force() {
        let mut rng = SplitMix64::new(0x17E4);
        for _ in 0..300 {
            let n = rng.below(20) as usize;
            let ivs: Vec<(i64, i64)> = (0..n)
                .map(|_| {
                    let a = rng.below(30) as i64 - 15;
                    let b = rng.below(30) as i64 - 15;
                    (a.min(b), a.max(b) + rng.below(2) as i64)
                })
                .collect();
            let t = IntervalTree::new(&ivs);
            assert_eq!(t.len(), ivs.iter().filter(|&&(l, h)| l < h).count());
            for _ in 0..6 {
                let x = rng.below(34) as i64 - 17;
                assert_eq!(t.stab(x), oracle_stab(&ivs, x));
                let a = rng.below(30) as i64 - 15;
                let b = rng.below(30) as i64 - 15;
                let (l, r) = (a.min(b), a.max(b));
                let g = t.overlap(l, r);
                let w = oracle_overlap(&ivs, l, r);
                assert_eq!(g, w, "q=[{l},{r}) ivs={ivs:?} t={t:?}");
            }
        }
    }

    #[test]
    fn known_layouts() {
        // Nested intervals all stabbed by the inner point.
        let t = IntervalTree::new(&[(0, 10), (2, 8), (4, 6)]);
        assert_eq!(t.stab(5), vec![0, 1, 2]);
        assert_eq!(t.stab(1), vec![0]);
        // Disjoint chain: query touching a shared endpoint sees the
        // right interval only (half-open).
        let t = IntervalTree::new(&[(0, 3), (3, 6), (6, 9)]);
        assert_eq!(t.stab(3), vec![1]);
        assert_eq!(t.overlap(3, 6), vec![1]);
        // Point queries.
        let t = IntervalTree::new(&[(5, 5), (1, 2)]); // degenerate dropped
        assert_eq!(t.len(), 1);
        assert_eq!(t.stab(5), Vec::<u32>::new());
    }

    #[test]
    fn input_order_does_not_matter() {
        let ivs = [(0, 4), (2, 6), (8, 10), (1, 3)];
        let perm = [(8, 10), (1, 3), (0, 4), (2, 6)];
        let a = IntervalTree::new(&ivs);
        let b = IntervalTree::new(&perm);
        // Results are indices into each input; compare the interval
        // multisets they select.
        let pick = |list: &[(i64, i64)], ids: &[u32]| {
            let mut v: Vec<(i64, i64)> = ids.iter().map(|&i| list[i as usize]).collect();
            v.sort_unstable();
            v
        };
        for x in -1..12 {
            assert_eq!(pick(&ivs, &a.stab(x)), pick(&perm, &b.stab(x)));
        }
        assert_eq!(pick(&ivs, &a.overlap(2, 9)), pick(&perm, &b.overlap(2, 9)));
    }

    #[test]
    fn empty_and_edge() {
        let t = IntervalTree::new(&[]);
        assert!(t.is_empty());
        assert_eq!(t.stab(0), Vec::<u32>::new());
        assert_eq!(t.overlap(0, 10), Vec::<u32>::new());
        // Inverted query returns empty rather than panicking.
        let t = IntervalTree::new(&[(0, 5)]);
        assert_eq!(t.overlap(5, 0), Vec::<u32>::new());
    }
}
