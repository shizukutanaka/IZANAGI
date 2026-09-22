//! Cartesian tree — Vuillemin's O(n) stack construction. The tree
//! that is simultaneously a heap on values and a binary search
//! tree on positions: inorder traversal replays the array, and the
//! LCA of `i` and `j` is the index of the range minimum/maximum of
//! `a[i..=j]`. The canonical bridge between arrays and trees —
//! Fischer–Heun RMQ, treap shapes ([`crate::treap`] is a cartesian
//! tree keyed on `(key, priority)`), and suffix-array height work
//! all route through this construction.
//!
//! Duplicates are canonicalized by `(value, index)` ordering, so
//! the tree is unique for any input — leftmost among equals wins
//! the root.
//!
//! ```
//! use izanagi_kit::cartesian::Cartesian;
//! let t = Cartesian::build_min(&[5, 2, 7, 1, 4]).unwrap();
//! assert_eq!(t.root(), 3); // the 1
//! assert_eq!(t.rmq(0, 2), Some(1)); // min of [5,2,7] at index 1
//! ```

/// Heap direction for [`Cartesian`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Min,
    Max,
}

/// A cartesian tree over an `i64` array, stored as parent/child
/// arrays — no pointers, fully canonical.
#[derive(Clone, Debug)]
pub struct Cartesian {
    a: Vec<i64>,
    parent: Vec<Option<u32>>,
    left: Vec<Option<u32>>,
    right: Vec<Option<u32>>,
    root: u32,
}

impl Cartesian {
    /// Min-heap cartesian tree — `rmq` yields range *minima*.
    /// `None` on an empty array.
    pub fn build_min(a: &[i64]) -> Option<Self> {
        Self::build(a, Kind::Min)
    }

    /// Max-heap cartesian tree — `rmq` yields range *maxima*.
    /// `None` on an empty array.
    pub fn build_max(a: &[i64]) -> Option<Self> {
        Self::build(a, Kind::Max)
    }

    fn build(a: &[i64], kind: Kind) -> Option<Self> {
        let n = a.len();
        if n == 0 {
            return None;
        }
        // (value, index) total order — strictly, so equal values
        // resolve to the leftmost occurrence.
        let better = |x: usize, y: usize| match kind {
            Kind::Min => (a[x], x) < (a[y], y),
            Kind::Max => (a[x], x) > (a[y], y),
        };
        let mut parent = vec![None; n];
        let mut left: Vec<Option<u32>> = vec![None; n];
        let mut right: Vec<Option<u32>> = vec![None; n];
        let mut stack: Vec<usize> = Vec::with_capacity(n);
        for i in 0..n {
            // pop everything that must hang below i
            let mut last: Option<usize> = None;
            while let Some(&t) = stack.last() {
                if better(i, t) {
                    last = stack.pop();
                } else {
                    break;
                }
            }
            left[i] = last.map(|l| l as u32);
            if let Some(l) = last {
                parent[l] = Some(i as u32);
            }
            if let Some(&t) = stack.last() {
                right[t] = Some(i as u32);
                parent[i] = Some(t as u32);
            }
            stack.push(i);
        }
        Some(Cartesian {
            a: a.to_vec(),
            parent,
            left,
            right,
            root: stack[0] as u32,
        })
    }

    /// Length of the underlying array.
    pub fn len(&self) -> usize {
        self.a.len()
    }

    /// `len == 0` — always `false` (build rejects empty).
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Root index — position of the heap extremum.
    pub fn root(&self) -> usize {
        self.root as usize
    }

    /// Parent of `i` — `None` at the root.
    pub fn parent(&self, i: usize) -> Option<usize> {
        self.parent.get(i).copied().flatten().map(|p| p as usize)
    }

    /// Left child of `i`.
    pub fn left(&self, i: usize) -> Option<usize> {
        self.left.get(i).copied().flatten().map(|l| l as usize)
    }

    /// Right child of `i`.
    pub fn right(&self, i: usize) -> Option<usize> {
        self.right.get(i).copied().flatten().map(|r| r as usize)
    }

    /// Index of the extreme value in `a[i..=j]` — the LCA of `i`
    /// and `j`. `None` when `i > j` or either is out of bounds.
    /// Runs O(depth) — fine for audit queries.
    pub fn rmq(&self, i: usize, j: usize) -> Option<usize> {
        let n = self.a.len();
        if i > j || j >= n {
            return None;
        }
        // climb from the deeper node — ancestor set of i crossed
        // with ancestor chain of j
        let mut anc = vec![false; n];
        let mut cur = Some(i);
        while let Some(u) = cur {
            anc[u] = true;
            cur = self.parent[u].map(|p| p as usize);
        }
        let mut cur = Some(j);
        while let Some(u) = cur {
            if anc[u] {
                return Some(u);
            }
            cur = self.parent[u].map(|p| p as usize);
        }
        None
    }

    /// Half-open index span `[l, r)` covered by `i`'s subtree —
    /// the subtree is always a contiguous interval by construction.
    pub fn subtree_range(&self, i: usize) -> Option<(usize, usize)> {
        if i >= self.a.len() {
            return None;
        }
        let (mut l, mut r) = (i, i + 1);
        let mut stack = vec![self.left[i], self.right[i]];
        while let Some(c) = stack.pop() {
            if let Some(u) = c {
                let u = u as usize;
                l = l.min(u);
                r = r.max(u + 1);
                stack.push(self.left[u]);
                stack.push(self.right[u]);
            }
        }
        Some((l, r))
    }

    /// Inorder traversal — always `0..n`; kept as the canonical
    /// serialization proving the BST-on-positions half.
    pub fn inorder(&self) -> Vec<usize> {
        let mut out = Vec::with_capacity(self.a.len());
        let mut stack = Vec::new();
        let mut cur = Some(self.root as usize);
        while cur.is_some() || !stack.is_empty() {
            while let Some(u) = cur {
                stack.push(u);
                cur = self.left[u].map(|l| l as usize);
            }
            if let Some(u) = stack.pop() {
                out.push(u);
                cur = self.right[u].map(|r| r as usize);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn oracle_rmq_min(a: &[i64], i: usize, j: usize) -> usize {
        // leftmost argmin in [i, j]
        (i..=j).min_by_key(|&k| (a[k], k)).unwrap()
    }

    #[test]
    fn heap_order_inorder_and_rmq_match_oracles() {
        let mut rng = SplitMix64::new(0xCA7E);
        for _ in 0..300 {
            let n = 1 + rng.below(40) as usize;
            let a: Vec<i64> = (0..n).map(|_| rng.below(25) as i64 - 12).collect();
            let t = Cartesian::build_min(&a).unwrap();
            // heap: every child ≥ parent by (val,idx)
            for i in 0..n {
                if let Some(p) = t.parent(i) {
                    assert!((a[p], p) <= (a[i], i));
                }
            }
            // inorder = 0..n
            assert_eq!(t.inorder(), (0..n).collect::<Vec<_>>());
            // root = leftmost global min
            assert_eq!(t.root(), oracle_rmq_min(&a, 0, n - 1));
            // rmq over random ranges + subtree contiguity
            for _ in 0..40 {
                let i = rng.below(n as u32) as usize;
                let j = i + rng.below(n as u32 - i as u32) as usize;
                assert_eq!(t.rmq(i, j), Some(oracle_rmq_min(&a, i, j)));
                // LCA's subtree covers the whole query span iff it is
                // the range answer
                let (l, r) = t.subtree_range(oracle_rmq_min(&a, i, j)).unwrap();
                assert!(l <= i && r > j);
            }
            assert!(t.rmq(0, n).is_none());
            assert!(t.rmq(1, 0).is_none());
            // subtree = contiguous interval; every index inside a
            // subtree is a descendant, so member ranges nest inside
            for i in 0..n {
                let (l, r) = t.subtree_range(i).unwrap();
                assert!(l <= i && i < r);
                for k in l..r {
                    let (l2, r2) = t.subtree_range(k).unwrap();
                    assert!(l2 >= l && r2 <= r);
                }
            }
        }
    }

    #[test]
    fn max_variant_and_canonical_shape() {
        let a = vec![3i64, 1, 4, 1, 5, 9, 2, 6];
        let t = Cartesian::build_max(&a).unwrap();
        assert_eq!(t.root(), 5); // the 9
        assert_eq!(t.rmq(0, 3), Some(2)); // 4 is max of [3,1,4,1]
                                          // deterministic: rebuild → identical shape
        let t2 = Cartesian::build_max(&a).unwrap();
        assert_eq!(t.inorder(), t2.inorder());
        assert_eq!(t.root(), t2.root());
        // duplicates → leftmost wins
        let d = Cartesian::build_min(&[7, 7, 7]).unwrap();
        assert_eq!(d.root(), 0);
        assert_eq!(d.inorder(), vec![0, 1, 2]);
        assert!(Cartesian::build_min(&[]).is_none());
        assert!(!d.is_empty());
        assert_eq!(d.len(), 3);
        let p = d.parent(0);
        assert!(p.is_none());
        assert_eq!(d.left(0), None);
        assert_eq!(d.right(0), Some(1));
    }
}
