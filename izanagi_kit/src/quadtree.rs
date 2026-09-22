//! Point quadtree — dynamic spatial index over a bounded `u32`
//! grid. Inserts subdivide a leaf into four children once it
//! exceeds `bucket` points; 1-wide/1-tall strips cannot split and
//! simply overflow (a leaf, never a hang). Answers are sorted so
//! results are canonical regardless of insertion order — the tree
//! shape is a pure function of the insertion sequence.
//!
//! [`crate::kdtree`] is the static bulk-built counterpart; this is
//! the *incremental* one for moving/appearing entities.
//!
//! ```
//! use izanagi_kit::quadtree::Quadtree;
//! let mut q = Quadtree::new(0, 0, 64, 64, 4).unwrap();
//! assert!(q.insert(10, 20));
//! assert!(q.insert(30, 40));
//! assert!(!q.insert(100, 40)); // outside bounds
//! assert_eq!(q.query(0, 0, 64, 64), vec![(10, 20), (30, 40)]);
//! ```

use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Half-open rectangle `[x, x+w) × [y, y+h)` in `u32` space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

impl Rect {
    fn contains(&self, p: (u32, u32)) -> bool {
        p.0 >= self.x && p.0 < self.x + self.w && p.1 >= self.y && p.1 < self.y + self.h
    }
    fn intersects(&self, o: &Rect) -> bool {
        self.x < o.x + o.w && o.x < self.x + self.w && self.y < o.y + o.h && o.y < self.y + self.h
    }
    /// Squared distance from `p` to this rect (0 when inside).
    fn dist2(&self, p: (u32, u32)) -> u64 {
        let dx = if p.0 < self.x {
            (self.x - p.0) as u64
        } else if p.0 >= self.x + self.w {
            (p.0 - (self.x + self.w - 1)) as u64
        } else {
            0
        };
        let dy = if p.1 < self.y {
            (self.y - p.1) as u64
        } else if p.1 >= self.y + self.h {
            (p.1 - (self.y + self.h - 1)) as u64
        } else {
            0
        };
        dx * dx + dy * dy
    }
    fn can_split(&self) -> bool {
        self.w >= 2 && self.h >= 2
    }
    /// NW, NE, SW, SE in that order — canonical.
    fn children(&self) -> [Rect; 4] {
        let mx = self.x + self.w / 2;
        let my = self.y + self.h / 2;
        [
            Rect {
                x: self.x,
                y: self.y,
                w: mx - self.x,
                h: my - self.y,
            },
            Rect {
                x: mx,
                y: self.y,
                w: self.x + self.w - mx,
                h: my - self.y,
            },
            Rect {
                x: self.x,
                y: my,
                w: mx - self.x,
                h: self.y + self.h - my,
            },
            Rect {
                x: mx,
                y: my,
                w: self.x + self.w - mx,
                h: self.y + self.h - my,
            },
        ]
    }
}

enum Node {
    Leaf(Vec<(u32, u32)>),
    Internal(Box<[Quadtree; 4]>),
}

/// A bucketed point quadtree over `u32` coordinates.
pub struct Quadtree {
    rect: Rect,
    bucket: usize,
    node: Node,
}

impl Quadtree {
    /// Root covering `[x,x+w) × [y,y+h)`; leaves split past `bucket`.
    /// `None` when `w == 0`, `h == 0`, or `bucket == 0`.
    pub fn new(x: u32, y: u32, w: u32, h: u32, bucket: usize) -> Option<Self> {
        if w == 0 || h == 0 || bucket == 0 {
            return None;
        }
        Some(Quadtree {
            rect: Rect { x, y, w, h },
            bucket,
            node: Node::Leaf(Vec::new()),
        })
    }

    /// Insert `p` — `false` when outside the root bounds.
    /// Duplicate points are stored (multiset semantics).
    pub fn insert(&mut self, x: u32, y: u32) -> bool {
        let p = (x, y);
        if !self.rect.contains(p) {
            return false;
        }
        self.insert_rec(p);
        true
    }

    fn insert_rec(&mut self, p: (u32, u32)) {
        match &mut self.node {
            Node::Leaf(pts) => {
                pts.push(p);
                if pts.len() > self.bucket && self.rect.can_split() {
                    let mut kids = self.rect.children().map(|r| Quadtree {
                        rect: r,
                        bucket: self.bucket,
                        node: Node::Leaf(Vec::new()),
                    });
                    for &q in pts.iter() {
                        for kid in kids.iter_mut() {
                            if kid.rect.contains(q) {
                                kid.insert_rec(q);
                                break;
                            }
                        }
                    }
                    self.node = Node::Internal(Box::new(kids));
                }
            }
            Node::Internal(kids) => {
                for kid in kids.iter_mut() {
                    if kid.rect.contains(p) {
                        kid.insert_rec(p);
                        return;
                    }
                }
            }
        }
    }

    /// Total stored points (duplicates counted).
    pub fn len(&self) -> usize {
        match &self.node {
            Node::Leaf(pts) => pts.len(),
            Node::Internal(kids) => kids.iter().map(Quadtree::len).sum(),
        }
    }

    /// `len == 0`.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// All points inside `[x,x+w) × [y,y+h)`, sorted — canonical.
    /// `w == 0`/`h == 0` yields an empty answer.
    pub fn query(&self, x: u32, y: u32, w: u32, h: u32) -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        if w == 0 || h == 0 {
            return out;
        }
        let r = Rect { x, y, w, h };
        self.query_rec(&r, &mut out);
        out.sort_unstable();
        out
    }

    fn query_rec(&self, r: &Rect, out: &mut Vec<(u32, u32)>) {
        match &self.node {
            Node::Leaf(pts) => {
                for &p in pts {
                    if r.contains(p) {
                        out.push(p);
                    }
                }
            }
            Node::Internal(kids) => {
                for kid in kids.iter() {
                    if kid.rect.intersects(r) {
                        kid.query_rec(r, out);
                    }
                }
            }
        }
    }

    /// Count points inside the query rect — `query(...).len()`
    /// without materializing the list.
    pub fn count(&self, x: u32, y: u32, w: u32, h: u32) -> usize {
        if w == 0 || h == 0 {
            return 0;
        }
        let r = Rect { x, y, w, h };
        self.count_rec(&r)
    }

    fn count_rec(&self, r: &Rect) -> usize {
        match &self.node {
            Node::Leaf(pts) => pts.iter().filter(|&&p| r.contains(p)).count(),
            Node::Internal(kids) => kids
                .iter()
                .filter(|k| k.rect.intersects(r))
                .map(|k| k.count_rec(r))
                .sum(),
        }
    }

    /// Nearest stored point to `p` (Euclidean²) — `None` when empty.
    /// Best-first descent through child rects by minimum possible
    /// distance; ties break on the lexicographically smallest point,
    /// making the answer canonical.
    pub fn nearest(&self, p: (u32, u32)) -> Option<(u32, u32)> {
        let mut best: Option<(u32, u32)> = None;
        let mut best_d2 = u64::MAX;
        // (min-dist², registry index) — heap ordered by distance only
        let mut nodes: Vec<&Quadtree> = vec![self];
        let mut heap: BinaryHeap<Reverse<(u64, usize)>> = BinaryHeap::new();
        heap.push(Reverse((self.rect.dist2(p), 0)));
        while let Some(Reverse((d2, ni))) = heap.pop() {
            if d2 > best_d2 {
                break;
            }
            let node = nodes[ni];
            match &node.node {
                Node::Leaf(pts) => {
                    for &q in pts {
                        let dq = dist2(p, q);
                        if dq < best_d2 || (dq == best_d2 && best.is_some_and(|b| q < b)) {
                            best_d2 = dq;
                            best = Some(q);
                        }
                    }
                }
                Node::Internal(kids) => {
                    for kid in kids.iter() {
                        let kd = kid.rect.dist2(p);
                        if kd <= best_d2 {
                            nodes.push(kid);
                            heap.push(Reverse((kd, nodes.len() - 1)));
                        }
                    }
                }
            }
        }
        best
    }
}

fn dist2(a: (u32, u32), b: (u32, u32)) -> u64 {
    let dx = (a.0 as i64 - b.0 as i64).unsigned_abs();
    let dy = (a.1 as i64 - b.1 as i64).unsigned_abs();
    dx * dx + dy * dy
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    #[test]
    fn query_and_count_match_brute_force() {
        let mut rng = SplitMix64::new(0x9AD7);
        for _ in 0..60 {
            let w = 1 + rng.below(40);
            let h = 1 + rng.below(40);
            let bucket = 1 + rng.below(6) as usize;
            let mut q = Quadtree::new(0, 0, w, h, bucket).unwrap();
            let n = rng.below(120) as usize;
            let mut pts = Vec::new();
            for _ in 0..n {
                let p = (rng.below(w), rng.below(h));
                assert!(q.insert(p.0, p.1));
                pts.push(p);
            }
            // out-of-bounds rejected
            assert!(!q.insert(w, h));
            assert_eq!(q.len(), n);
            assert_eq!(q.is_empty(), n == 0);
            // random rect queries vs oracle
            for _ in 0..30 {
                let rx = rng.below(w + 4);
                let ry = rng.below(h + 4);
                let rw = rng.below(w + 4);
                let rh = rng.below(h + 4);
                let mut expect: Vec<(u32, u32)> = pts
                    .iter()
                    .copied()
                    .filter(|p| p.0 >= rx && p.0 < rx + rw && p.1 >= ry && p.1 < ry + rh)
                    .collect();
                expect.sort_unstable();
                assert_eq!(q.query(rx, ry, rw, rh), expect);
                // count is multiset — duplicates included
                let expect_n = pts
                    .iter()
                    .filter(|p| p.0 >= rx && p.0 < rx + rw && p.1 >= ry && p.1 < ry + rh)
                    .count();
                assert_eq!(q.count(rx, ry, rw, rh), expect_n);
            }
        }
    }

    #[test]
    fn nearest_matches_brute_force_and_canonical_tie() {
        let mut rng = SplitMix64::new(0x1EE5);
        for _ in 0..50 {
            let mut q = Quadtree::new(0, 0, 24, 24, 3).unwrap();
            let n = 1 + rng.below(60) as usize;
            let mut pts = BTreeSet::new();
            for _ in 0..n {
                let p = (rng.below(24), rng.below(24));
                if q.insert(p.0, p.1) {
                    pts.insert(p);
                }
            }
            for _ in 0..20 {
                let p = (rng.below(30), rng.below(30));
                let oracle = pts.iter().min_by_key(|&&q2| (dist2(p, q2), q2)).copied();
                assert_eq!(q.nearest(p), oracle);
            }
        }
    }

    #[test]
    fn strips_do_not_split_forever_and_validation() {
        // 1-wide rect can't split — leaf overflows, stays correct.
        let mut q = Quadtree::new(5, 0, 1, 10, 2).unwrap();
        for y in 0..10 {
            assert!(q.insert(5, y));
        }
        assert_eq!(q.len(), 10);
        assert_eq!(q.query(0, 0, 100, 100).len(), 10);
        assert!(Quadtree::new(0, 0, 0, 5, 1).is_none());
        assert!(Quadtree::new(0, 0, 5, 5, 0).is_none());
        let mut e = Quadtree::new(0, 0, 8, 8, 2).unwrap();
        assert!(e.nearest((0, 0)).is_none());
        assert!(!e.insert(9, 0));
        assert!(e.is_empty());
        e.insert(1, 1);
        assert_eq!(e.nearest((7, 7)), Some((1, 1)));
        // duplicate points kept
        e.insert(1, 1);
        assert_eq!(e.len(), 2);
    }
}
