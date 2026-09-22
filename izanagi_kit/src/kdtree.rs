//! Static 2-D kd-tree — the spatial index complement to
//! [`spatial_hash`](crate::spatial_hash): uniform-density cell hashing
//! is the right answer for hot, mutating simulations, but a median-split
//! kd-tree answers `nearest`/`within`/`in_rect` in `O(log n)`
//! expected time on *arbitrary* point sets with zero per-query
//! allocation churn, and it is a pure function of the input list —
//! same points, same tree, same query answers.
//!
//! Distances are squared `i128` (matching [`closestpair`](crate::closestpair));
//! `nearest` breaks equal-distance ties by ascending point, so the
//! answer never depends on input order.
//!
//! ```
//! use izanagi_kit::kdtree::KdTree;
//! let t = KdTree::new(&[(2, 3), (5, 4), (9, 6), (4, 7), (8, 1)]);
//! assert_eq!(t.nearest((9, 2)), Some(((8, 1), 2)));
//! assert_eq!(t.within((5, 4), 15).len(), 3);
//! ```

/// One arena node: a point plus child subtrees.
#[derive(Debug)]
struct Node {
    pt: (i32, i32),
    axis: usize, // 0 = x, 1 = y
    left: Option<usize>,
    right: Option<usize>,
}

/// Static median-split kd-tree over `(i32, i32)` points.
pub struct KdTree {
    nodes: Vec<Node>,
    root: Option<usize>,
}

fn dist2(a: (i32, i32), b: (i32, i32)) -> i128 {
    let dx = a.0 as i128 - b.0 as i128;
    let dy = a.1 as i128 - b.1 as i128;
    dx * dx + dy * dy
}

impl KdTree {
    /// Build the tree — `O(n log n)`, balanced by construction.
    /// Duplicate points are kept (each is its own leaf-side entry).
    pub fn new(pts: &[(i32, i32)]) -> Self {
        let mut t = Self {
            nodes: Vec::new(),
            root: None,
        };
        let mut order: Vec<(i32, i32)> = pts.to_vec();
        t.root = t.build(&mut order, 0);
        t
    }

    /// Median-split `pts` into a node + subtrees; returns arena index.
    fn build(&mut self, pts: &mut [(i32, i32)], depth: usize) -> Option<usize> {
        if pts.is_empty() {
            return None;
        }
        let axis = depth & 1;
        // Full tuple sort — deterministic median even under ties.
        pts.sort_by_key(|&(x, y)| if axis == 0 { (x, y) } else { (y, x) });
        let mid = pts.len() / 2;
        let (left, rest) = pts.split_at_mut(mid);
        let (pmedian, right) = rest.split_at_mut(1);
        let idx = self.nodes.len();
        self.nodes.push(Node {
            pt: pmedian[0],
            axis,
            left: None,
            right: None,
        });
        let l = self.build(left, depth + 1);
        let r = self.build(right, depth + 1);
        self.nodes[idx].left = l;
        self.nodes[idx].right = r;
        Some(idx)
    }

    /// Number of indexed points.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Points indexed, in arena (median-split) order.
    pub fn points(&self) -> Vec<(i32, i32)> {
        self.nodes.iter().map(|n| n.pt).collect()
    }

    /// Every point with `dist2(p, ·) ≤ r2`, sorted ascending.
    pub fn within(&self, p: (i32, i32), r2: i128) -> Vec<(i32, i32)> {
        let mut out = Vec::new();
        let mut stack: Vec<usize> = self.root.into_iter().collect();
        while let Some(i) = stack.pop() {
            let n = &self.nodes[i];
            if dist2(n.pt, p) <= r2 {
                out.push(n.pt);
            }
            let c = if n.axis == 0 { n.pt.0 } else { n.pt.1 };
            let q = if n.axis == 0 { p.0 } else { p.1 };
            // The query ball crosses the split plane iff |q−c|² ≤ r2.
            let d = q as i128 - c as i128;
            let crosses = d * d <= r2;
            if q <= c || crosses {
                if let Some(l) = n.left {
                    stack.push(l);
                }
            }
            if q > c || crosses {
                if let Some(r) = n.right {
                    stack.push(r);
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Every point in the half-open rect `[lo.0, hi.0) × [lo.1, hi.1)`,
    /// sorted ascending — consistent with `interval`/`grid` bounds
    /// conventions.
    pub fn in_rect(&self, lo: (i32, i32), hi: (i32, i32)) -> Vec<(i32, i32)> {
        let mut out = Vec::new();
        let mut stack: Vec<usize> = self.root.into_iter().collect();
        while let Some(i) = stack.pop() {
            let n = &self.nodes[i];
            let (x, y) = n.pt;
            if x >= lo.0 && x < hi.0 && y >= lo.1 && y < hi.1 {
                out.push(n.pt);
            }
            let (c, ql, qh) = if n.axis == 0 {
                (x, lo.0, hi.0)
            } else {
                (y, lo.1, hi.1)
            };
            if ql <= c {
                if let Some(l) = n.left {
                    stack.push(l);
                }
            }
            if qh > c {
                if let Some(r) = n.right {
                    stack.push(r);
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Nearest point to `p` — `Some((point, dist²))`, ties resolved to
    /// the lexicographically smallest point.
    pub fn nearest(&self, p: (i32, i32)) -> Option<((i32, i32), i128)> {
        let mut best: Option<((i32, i32), i128)> = None;
        let mut stack: Vec<usize> = self.root.into_iter().collect();
        while let Some(i) = stack.pop() {
            let n = &self.nodes[i];
            let d = dist2(n.pt, p);
            let better = match best {
                None => true,
                Some((bp, bd)) => d < bd || (d == bd && n.pt < bp),
            };
            if better {
                best = Some((n.pt, d));
            }
            let c = if n.axis == 0 { n.pt.0 } else { n.pt.1 };
            let q = if n.axis == 0 { p.0 } else { p.1 };
            let diff = q as i128 - c as i128;
            let crosses = match best {
                Some((_, bd)) => diff * diff <= bd,
                None => true,
            };
            if q <= c || crosses {
                if let Some(l) = n.left {
                    stack.push(l);
                }
            }
            if q > c || crosses {
                if let Some(r) = n.right {
                    stack.push(r);
                }
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    fn brute_within(pts: &[(i32, i32)], p: (i32, i32), r2: i128) -> Vec<(i32, i32)> {
        let mut v: Vec<_> = pts.iter().copied().filter(|&q| dist2(p, q) <= r2).collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    fn brute_rect(pts: &[(i32, i32)], lo: (i32, i32), hi: (i32, i32)) -> Vec<(i32, i32)> {
        let mut v: Vec<_> = pts
            .iter()
            .copied()
            .filter(|&(x, y)| x >= lo.0 && x < hi.0 && y >= lo.1 && y < hi.1)
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    fn brute_nearest(pts: &[(i32, i32)], p: (i32, i32)) -> Option<((i32, i32), i128)> {
        pts.iter()
            .copied()
            .map(|q| (q, dist2(p, q)))
            .min_by_key(|&(q, d)| (d, q))
    }

    #[test]
    fn matches_brute_force() {
        let mut rng = SplitMix64::new(0xAD7E);
        for _ in 0..300 {
            let n = rng.below(60) as usize;
            let pts: Vec<(i32, i32)> = (0..n)
                .map(|_| (rng.next_u64() as i32 % 500, rng.next_u64() as i32 % 500))
                .collect();
            let t = KdTree::new(&pts);
            assert_eq!(t.len(), n);
            let p = (rng.next_u64() as i32 % 500, rng.next_u64() as i32 % 500);
            let r2 = (rng.next_u64() % 40_000) as i128;
            assert_eq!(t.within(p, r2), brute_within(&pts, p, r2));
            let lo = (rng.next_u64() as i32 % 400, rng.next_u64() as i32 % 400);
            let hi = (
                lo.0 + rng.next_u64() as i32 % 200,
                lo.1 + rng.next_u64() as i32 % 200,
            );
            assert_eq!(t.in_rect(lo, hi), brute_rect(&pts, lo, hi));
            assert_eq!(t.nearest(p), brute_nearest(&pts, p));
            // Input permutation must not change any answer.
            let mut shuffled = pts.clone();
            shuffled.reverse();
            let t2 = KdTree::new(&shuffled);
            assert_eq!(t.nearest(p), t2.nearest(p));
            assert_eq!(t.within(p, r2), t2.within(p, r2));
        }
    }

    #[test]
    fn edges() {
        let t = KdTree::new(&[]);
        assert!(t.is_empty());
        assert_eq!(t.nearest((0, 0)), None);
        assert_eq!(t.within((0, 0), 100), vec![]);
        // Duplicates index once per copy but dedup on output.
        let t = KdTree::new(&[(1, 1), (1, 1), (1, 1)]);
        assert_eq!(t.len(), 3);
        assert_eq!(t.within((0, 0), 10), vec![(1, 1)]);
        // Tie: two points equidistant — lexicographically smallest wins.
        let t = KdTree::new(&[(1, 0), (0, 1)]);
        assert_eq!(t.nearest((0, 0)), Some(((0, 1), 1)));
        let t = KdTree::new(&[(5, 5)]);
        assert_eq!(t.nearest((0, 0)), Some(((5, 5), 50)));
    }

    #[test]
    fn deep_input_does_not_stack_overflow() {
        // 100k sorted points — the pathological median case.
        let pts: Vec<(i32, i32)> = (0..100_000).map(|i| (i, i * 7 % 997)).collect();
        let t = KdTree::new(&pts);
        assert_eq!(t.nearest((50_000, 0)), brute_nearest(&pts, (50_000, 0)));
        let set: BTreeSet<_> = t.points().into_iter().collect();
        assert_eq!(
            set.len(),
            pts.iter().copied().collect::<BTreeSet<_>>().len()
        );
    }
}
