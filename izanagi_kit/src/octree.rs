//! Bucketed 3-D point octree — the volumetric counterpart to
//! [`crate::quadtree`]. Integer `i64` coordinates throughout;
//! every node covers an axis-aligned cube `[o, o+s)³` split into
//! 8 octants. Points sit at leaves up to `BUCKET` capacity; an
//! overflowing leaf subdivides until depth `MAX_DEPTH` (degenerate
//! stacks of identical or collinear points stay in the deep leaf
//! instead of hanging). Range and nearest queries return results
//! **sorted** by point — output is a pure function of the point
//! multiset, not of insertion order.
//!
//! ```
//! use izanagi_kit::octree::Octree;
//! let mut t = Octree::new(-128, 256);
//! t.insert([0, 0, 0], 5);
//! t.insert([10, 10, 10], 7);
//! assert_eq!(t.query([-1, -1, -1], [20, 20, 20]).len(), 2);
//! assert_eq!(t.nearest([9, 9, 9]), Some(([10, 10, 10], 7)));
//! ```

const BUCKET: usize = 8;
const MAX_DEPTH: usize = 32;

type Pt = [i64; 3];

fn sub_bounds(o: Pt, s: i64, oct: usize) -> (Pt, i64) {
    let h = s / 2;
    (
        [
            o[0] + h * (oct & 1) as i64,
            o[1] + h * ((oct >> 1) & 1) as i64,
            o[2] + h * ((oct >> 2) & 1) as i64,
        ],
        h,
    )
}

fn in_box(p: Pt, lo: Pt, hi: Pt) -> bool {
    (0..3).all(|i| lo[i] <= p[i] && p[i] < hi[i])
}

fn box_min_dist2(p: Pt, lo: Pt, hi: Pt) -> i128 {
    let mut d = 0i128;
    for i in 0..3 {
        let v = if p[i] < lo[i] {
            lo[i] - p[i]
        } else if p[i] >= hi[i] {
            p[i] - hi[i] + 1
        } else {
            0
        };
        d += (v as i128) * (v as i128);
    }
    d
}

fn dist2(a: Pt, b: Pt) -> i128 {
    (0..3)
        .map(|i| ((a[i] - b[i]) as i128) * ((a[i] - b[i]) as i128))
        .sum()
}

enum Node {
    /// (bounds) + up to BUCKET entries.
    Leaf(Vec<(Pt, u64)>),
    /// 8 children by octant index.
    Branch(Box<[Node; 8]>),
}

impl Node {
    fn new_leaf() -> Self {
        Node::Leaf(Vec::new())
    }
}

/// Dynamic 3-D point index keyed by `i64` coordinate triples,
/// each carrying a `u64` payload.
pub struct Octree {
    root: Node,
    origin: Pt,
    size: i64,
    len: usize,
}

impl Octree {
    /// Root cube `[origin, origin+size)³` on each axis.
    pub fn new(origin: i64, size: i64) -> Self {
        Self {
            root: Node::new_leaf(),
            origin: [origin; 3],
            size: size.max(1),
            len: 0,
        }
    }

    /// Insert `(point, value)` — duplicates accumulate (the index
    /// tracks a multiset of `(pt, value)` pairs). Returns `false`
    /// outside the root bounds.
    pub fn insert(&mut self, pt: Pt, val: u64) -> bool {
        if !in_box(
            pt,
            self.origin,
            [
                self.origin[0] + self.size,
                self.origin[1] + self.size,
                self.origin[2] + self.size,
            ],
        ) {
            return false;
        }
        Self::insert_at(&mut self.root, pt, val, self.origin, self.size, 0);
        self.len += 1;
        true
    }

    fn insert_at(n: &mut Node, pt: Pt, val: u64, o: Pt, s: i64, depth: usize) {
        match n {
            Node::Leaf(v) => {
                // A leaf only subdivides while its cube can still
                // halve (s > 1) — identical-coordinate stacks land
                // at the deepest real cell instead of hanging.
                if v.len() < BUCKET || depth >= MAX_DEPTH || s <= 1 {
                    v.push((pt, val));
                    v.sort_unstable();
                } else {
                    let mut kids: [Node; 8] = [
                        Node::new_leaf(),
                        Node::new_leaf(),
                        Node::new_leaf(),
                        Node::new_leaf(),
                        Node::new_leaf(),
                        Node::new_leaf(),
                        Node::new_leaf(),
                        Node::new_leaf(),
                    ];
                    for (q, w) in v.drain(..) {
                        let oct = Self::octant(q, o, s);
                        Self::insert_at(
                            &mut kids[oct],
                            q,
                            w,
                            sub_bounds(o, s, oct).0,
                            s / 2,
                            depth + 1,
                        );
                    }
                    *n = Node::Branch(Box::new(kids));
                    // Recurse on the now-branch node.
                    let oct = Self::octant(pt, o, s);
                    if let Node::Branch(k) = n {
                        Self::insert_at(
                            &mut k[oct],
                            pt,
                            val,
                            sub_bounds(o, s, oct).0,
                            s / 2,
                            depth + 1,
                        );
                    }
                }
            }
            Node::Branch(k) => {
                let oct = Self::octant(pt, o, s);
                Self::insert_at(
                    &mut k[oct],
                    pt,
                    val,
                    sub_bounds(o, s, oct).0,
                    s / 2,
                    depth + 1,
                );
            }
        }
    }

    fn octant(p: Pt, o: Pt, s: i64) -> usize {
        let h = s / 2;
        usize::from(p[0] >= o[0] + h)
            | (usize::from(p[1] >= o[1] + h) << 1)
            | (usize::from(p[2] >= o[2] + h) << 2)
    }

    /// Every `(pt, value)` inside `[lo, hi)` per axis, sorted.
    pub fn query(&self, lo: Pt, hi: Pt) -> Vec<(Pt, u64)> {
        let mut out = Vec::new();
        Self::query_at(&self.root, lo, hi, self.origin, self.size, &mut out);
        out.sort_unstable();
        out
    }

    fn query_at(n: &Node, lo: Pt, hi: Pt, o: Pt, s: i64, out: &mut Vec<(Pt, u64)>) {
        match n {
            Node::Leaf(v) => {
                for &(p, w) in v {
                    if in_box(p, lo, hi) {
                        out.push((p, w));
                    }
                }
            }
            Node::Branch(k) => {
                for oct in 0..8 {
                    let (co, cs) = sub_bounds(o, s, oct);
                    // Skip octants disjoint from the query box.
                    if (0..3).all(|i| co[i] < hi[i] && lo[i] < co[i] + cs) {
                        Self::query_at(&k[oct], lo, hi, co, cs, out);
                    }
                }
            }
        }
    }

    /// Nearest `(pt, value)` to `q` by squared distance, best-first
    /// search; ties break by point ordering so the answer is a
    /// pure function of the stored set.
    pub fn nearest(&self, q: Pt) -> Option<(Pt, u64)> {
        let mut best: Option<((i128, Pt), (Pt, u64))> = None;
        // Stack entries carry each node's own origin — pruning bounds
        // are per-subtree, not the root's.
        let mut stack: Vec<(&Node, Pt, i64)> = vec![(&self.root, self.origin, self.size)];
        while let Some((n, o, s)) = stack.pop() {
            match n {
                Node::Leaf(v) => {
                    for &(p, w) in v {
                        let key = (dist2(q, p), p);
                        if best.map_or(true, |(bk, _)| key < bk) {
                            best = Some((key, (p, w)));
                        }
                    }
                }
                Node::Branch(k) => {
                    for oct in 0..8 {
                        let (co, cs) = sub_bounds(o, s, oct);
                        let d = box_min_dist2(q, co, [co[0] + cs, co[1] + cs, co[2] + cs]);
                        if best.map_or(true, |(bk, _)| d < bk.0) {
                            stack.push((&k[oct], co, cs));
                        }
                    }
                }
            }
        }
        best.map(|(_, b)| b)
    }

    /// Number of stored points (multiplicity counts).
    pub fn len(&self) -> usize {
        self.len
    }

    /// True when empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeMap;

    #[test]
    fn basic() {
        let mut t = Octree::new(0, 64);
        assert!(t.is_empty());
        t.insert([1, 2, 3], 10);
        t.insert([5, 5, 5], 20);
        assert_eq!(t.len(), 2);
        assert_eq!(t.query([0, 0, 0], [4, 4, 4]), vec![([1, 2, 3], 10)]);
        assert!(!t.insert([64, 64, 64], 1));
    }

    #[test]
    fn oracle_query() {
        let mut rng = SplitMix64::new(53);
        let mut t = Octree::new(-512, 1024);
        let mut pts: BTreeMap<Pt, Vec<u64>> = BTreeMap::new();
        for _ in 0..400 {
            let p: Pt = [
                rng.below(900) as i64 - 450,
                rng.below(900) as i64 - 450,
                rng.below(900) as i64 - 450,
            ];
            let v = rng.next_u64() % 1000;
            t.insert(p, v);
            pts.entry(p).or_default().push(v);
        }
        // BBox query vs linear scan.
        for _ in 0..100 {
            let lo: Pt = [
                rng.below(800) as i64 - 500,
                rng.below(800) as i64 - 500,
                rng.below(800) as i64 - 500,
            ];
            let hi: Pt = [
                lo[0] + 1 + rng.below(400) as i64,
                lo[1] + 1 + rng.below(400) as i64,
                lo[2] + 1 + rng.below(400) as i64,
            ];
            let got = t.query(lo, hi);
            let mut want: Vec<(Pt, u64)> = pts
                .iter()
                .filter(|(p, _)| in_box(**p, lo, hi))
                .flat_map(|(p, vs)| vs.iter().map(move |&v| (*p, v)))
                .collect();
            want.sort_unstable();
            assert_eq!(got, want);
        }
    }

    #[test]
    fn oracle_nearest() {
        let mut rng = SplitMix64::new(59);
        let mut t = Octree::new(-256, 512);
        let mut pts = Vec::new();
        for _ in 0..300 {
            let p: Pt = [
                rng.below(500) as i64 - 250,
                rng.below(500) as i64 - 250,
                rng.below(500) as i64 - 250,
            ];
            let v = rng.next_u64() % 500;
            t.insert(p, v);
            pts.push((p, v));
        }
        for _ in 0..100 {
            let q: Pt = [
                rng.below(600) as i64 - 300,
                rng.below(600) as i64 - 300,
                rng.below(600) as i64 - 300,
            ];
            let got = t.nearest(q);
            let want = pts
                .iter()
                .map(|&(p, v)| ((dist2(q, p), p), (p, v)))
                .min()
                .map(|(_, b)| b);
            assert_eq!(got, want);
        }
    }

    #[test]
    fn degenerate_stack() {
        // 2000 identical points must not hang — depth cap holds.
        let mut t = Octree::new(0, 8);
        for i in 0..2000u64 {
            t.insert([0, 0, 0], i);
        }
        assert_eq!(t.len(), 2000);
        assert_eq!(t.query([0, 0, 0], [1, 1, 1]).len(), 2000);
    }
}
