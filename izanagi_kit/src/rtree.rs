//! STR-packed static R-tree — a point index built bottom-up by
//! sort-tile-recursive packing (Leutenegger, Edgington & Lopez 1997):
//! sort points by x, tile into `ceil(n/m)` groups, sort each tile by
//! y, pack leaves of `m` points; repeat over parent rects until one
//! node remains. Because every order is a total sort, the tree shape
//! is a pure function of the point set — insertion order can never
//! leak in.
//!
//! Queries return points in ascending `(x, y)` order — canonical,
//! and equal to a brute-force scan by construction. A packed R-tree
//! is read-only, which is exactly what a lockstep world needs: build
//! at level load, query during play.
//!
//! ```
//! use izanagi_kit::rtree::Rtree;
//! let t = Rtree::build(&[(0, 0), (5, 5), (50, 50), (51, 51)], 2);
//! assert_eq!(t.query((-10, -10, 10, 10)), vec![(0, 0), (5, 5)]);
//! assert_eq!(t.count(), 4);
//! ```

/// One node: a bounding rect plus children (internal) or leaf points.
#[derive(Clone, Debug)]
struct Node {
    /// Bounding box `(min_x, min_y, max_x, max_y)`.
    rect: (i32, i32, i32, i32),
    /// Internal children (empty at leaves).
    children: Vec<Node>,
    /// Leaf points (empty at internal nodes).
    points: Vec<(i32, i32)>,
}

fn contains_pt(r: (i32, i32, i32, i32), p: (i32, i32)) -> bool {
    p.0 >= r.0 && p.0 <= r.2 && p.1 >= r.1 && p.1 <= r.3
}

fn intersects(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
    a.0 <= b.2 && a.2 >= b.0 && a.1 <= b.3 && a.3 >= b.1
}

fn bbox_of(items: &[(i32, i32, i32, i32)]) -> (i32, i32, i32, i32) {
    let mut r = items[0];
    for &i in &items[1..] {
        r.0 = r.0.min(i.0);
        r.1 = r.1.min(i.1);
        r.2 = r.2.max(i.2);
        r.3 = r.3.max(i.3);
    }
    r
}

/// A packed R-tree over `(i32, i32)` points.
#[derive(Clone, Debug)]
pub struct Rtree {
    root: Option<Box<Node>>,
    n: usize,
}

impl Rtree {
    /// Build a packed tree over `points` with node capacity `m`
    /// (`m < 2` clamps to 2). Duplicate points are kept — the index
    /// is a multiset, matching [`crate::quadtree`].
    pub fn build(points: &[(i32, i32)], m: usize) -> Self {
        let m = m.max(2);
        let n = points.len();
        if n == 0 {
            return Self { root: None, n };
        }
        // Leaves: STR tiling — sort by x, tile, sort each tile by y,
        // pack into nodes of m.
        let mut pts = points.to_vec();
        pts.sort_unstable();
        let leaf_count = n.div_ceil(m).max(1);
        let tile = n.div_ceil(leaf_count).max(1);
        let mut leaves: Vec<Node> = Vec::new();
        for chunk in pts.chunks_mut(tile) {
            chunk.sort_by_key(|p| p.1);
            for leaf in chunk.chunks(m) {
                leaves.push(Node {
                    rect: {
                        let xs: Vec<i32> = leaf.iter().map(|p| p.0).collect();
                        let ys: Vec<i32> = leaf.iter().map(|p| p.1).collect();
                        (
                            *xs.iter().min().unwrap_or(&0),
                            *ys.iter().min().unwrap_or(&0),
                            *xs.iter().max().unwrap_or(&0),
                            *ys.iter().max().unwrap_or(&0),
                        )
                    },
                    children: Vec::new(),
                    points: leaf.to_vec(),
                });
            }
        }
        // Parents: same STR packing over child rects until one root.
        let mut level = leaves;
        while level.len() > 1 {
            level.sort_by_key(|n| (n.rect.0 + n.rect.2, n.rect.0, n.rect.2));
            let groups = level.len().div_ceil(m);
            let group_size = level.len().div_ceil(groups);
            let mut next: Vec<Node> = Vec::new();
            let mut i = 0;
            while i < level.len() {
                let end = (i + group_size).min(level.len());
                let mut group: Vec<Node> = level[i..end].to_vec();
                group.sort_by_key(|n| (n.rect.1 + n.rect.3, n.rect.1, n.rect.3));
                for sub in group.chunks(m) {
                    let rects: Vec<(i32, i32, i32, i32)> = sub.iter().map(|c| c.rect).collect();
                    next.push(Node {
                        rect: bbox_of(&rects),
                        children: sub.to_vec(),
                        points: Vec::new(),
                    });
                }
                i = end;
            }
            level = next;
        }
        Self {
            root: Some(Box::new(level.into_iter().next().unwrap_or(Node {
                rect: (0, 0, 0, 0),
                children: Vec::new(),
                points: Vec::new(),
            }))),
            n,
        }
    }

    /// Number of indexed points (multiset count).
    pub fn count(&self) -> usize {
        self.n
    }

    /// True when built over zero points.
    pub fn is_empty(&self) -> bool {
        self.n == 0
    }

    /// All points inside the inclusive rect `(x0, y0, x1, y1)`,
    /// in ascending `(x, y)` order.
    pub fn query(&self, rect: (i32, i32, i32, i32)) -> Vec<(i32, i32)> {
        let mut out = Vec::new();
        if let Some(r) = &self.root {
            Self::walk(r, rect, &mut out);
        }
        out.sort_unstable();
        out
    }

    fn walk(node: &Node, rect: (i32, i32, i32, i32), out: &mut Vec<(i32, i32)>) {
        if !intersects(node.rect, rect) {
            return;
        }
        if node.children.is_empty() {
            for &p in &node.points {
                if contains_pt(rect, p) {
                    out.push(p);
                }
            }
            return;
        }
        for c in &node.children {
            Self::walk(c, rect, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn query_matches_brute_force() {
        let mut rng = SplitMix64::new(53);
        for _ in 0..30 {
            let n = rng.below(120) as usize + 1;
            let pts: Vec<(i32, i32)> = (0..n)
                .map(|_| (rng.below(60) as i32, rng.below(60) as i32))
                .collect();
            let t = Rtree::build(&pts, 4);
            for _ in 0..15 {
                let x0 = rng.below(80) as i32 - 10;
                let y0 = rng.below(80) as i32 - 10;
                let x1 = x0 + rng.below(40) as i32;
                let y1 = y0 + rng.below(40) as i32;
                let rect = (x0, y0, x1, y1);
                let mut want: Vec<(i32, i32)> = pts
                    .iter()
                    .copied()
                    .filter(|&p| contains_pt(rect, p))
                    .collect();
                want.sort_unstable();
                assert_eq!(t.query(rect), want);
            }
        }
    }

    #[test]
    fn shape_is_pointset_only() {
        let mut a = vec![(0, 0), (5, 5), (10, 2), (7, 7), (3, 9)];
        let t1 = Rtree::build(&a, 2);
        a.reverse();
        let t2 = Rtree::build(&a, 2);
        // Same point set → identical query answers everywhere.
        for x in -5..15 {
            for y in -5..15 {
                assert_eq!(
                    t1.query((x, y, x + 3, y + 3)),
                    t2.query((x, y, x + 3, y + 3))
                );
            }
        }
    }

    #[test]
    fn empty_and_degenerate() {
        let t = Rtree::build(&[], 4);
        assert!(t.is_empty());
        assert!(t.query((0, 0, 10, 10)).is_empty());
        let t = Rtree::build(&[(3, 3)], 4);
        assert_eq!(t.query((0, 0, 5, 5)), vec![(3, 3)]);
        assert_eq!(t.query((0, 0, 2, 2)), vec![]);
    }
}
