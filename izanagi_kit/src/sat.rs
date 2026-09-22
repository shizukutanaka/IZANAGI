//! Separating Axis Test — exact convex-polygon overlap with a
//! minimum-overlap witness, all in `i128` so every peer agrees
//! bit-for-bit (no floating-point axis normals, ever).
//!
//! Two convex polygons overlap iff *no* edge normal is a separating
//! axis — i.e. the vertex projections on every normal overlap. Only
//! the edge normals are candidates, which is why SAT exits early on
//! the first separating axis and costs O(m+n) per query on the
//! common separated pair.
//!
//! Boundary contact counts as overlap (closed polygons).
//! [`Collision::depth`] is measured in **projection units** along
//! the unnormalised integer axis — the actual penetration distance
//! is `depth / |axis|` and the exact minimum translation vector is
//! `axis · depth / |axis|²` (rational; callers needing integer MTVs
//! should scale coordinates by a fixed-point factor first).
//!
//! A point is a legal polygon (no edges): point-vs-convex overlap
//! still resolves correctly because the point is inside iff its
//! projection lies inside every normal's projection slab — the
//! slabs of a convex polygon's facet normals reconstruct it.
//!
//! ```
//! use izanagi_kit::sat::{collide, overlap};
//! let sq = [(0, 0), (4, 0), (4, 4), (0, 4)];
//! let sq2 = [(3, 0), (7, 0), (7, 4), (3, 4)];
//! let far = [(10, 0), (14, 0), (14, 4), (10, 4)];
//! assert!(overlap(&sq, &sq2));
//! let c = collide(&sq, &sq2).unwrap();
//! assert_eq!(c.depth, 4); // 1 true unit × |axis| 4
//! assert_eq!(c.axis, (4, 0)); // edge normal of the y-parallel edges
//! assert!(!overlap(&sq, &far));
//! ```

/// A collision witness: the axis of least overlap and that overlap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Collision {
    /// The edge normal achieving minimum overlap — an unnormalised
    /// integer vector perpendicular to a polygon edge, oriented
    /// from `a`'s centroid toward `b`'s (canonical, never random:
    /// centroids coinciding keeps whichever sign the edge gave).
    pub axis: (i64, i64),
    /// Projection overlap along `axis` in dot-product units — `0`
    /// for pure boundary contact. True penetration depth is
    /// `depth / |axis|`.
    pub depth: i128,
}

fn dot(axis: (i64, i64), p: (i32, i32)) -> i128 {
    axis.0 as i128 * p.0 as i128 + axis.1 as i128 * p.1 as i128
}

fn project(axis: (i64, i64), poly: &[(i32, i32)]) -> (i128, i128) {
    let mut lo = i128::MAX;
    let mut hi = i128::MIN;
    for &p in poly {
        let d = dot(axis, p);
        lo = lo.min(d);
        hi = hi.max(d);
    }
    (lo, hi)
}

fn centroid(poly: &[(i32, i32)]) -> (i128, i128) {
    let (mut sx, mut sy) = (0i128, 0i128);
    for &p in poly {
        sx += p.0 as i128;
        sy += p.1 as i128;
    }
    (sx, sy)
}

/// Run SAT over the edge normals of both convex polygons — `None`
/// when separated, `Some` with the minimum-overlap axis when they
/// touch or overlap. Empty polygons never collide; all-degenerate
/// polygons (one distinct point each) collide only when identical.
pub fn collide(a: &[(i32, i32)], b: &[(i32, i32)]) -> Option<Collision> {
    if a.is_empty() || b.is_empty() {
        return None;
    }
    let mut best: Option<Collision> = None;
    for poly in [a, b] {
        for i in 0..poly.len() {
            let p = poly[i];
            let q = poly[(i + 1) % poly.len()];
            let (ex, ey) = (q.0 as i64 - p.0 as i64, q.1 as i64 - p.1 as i64);
            if ex == 0 && ey == 0 {
                continue;
            }
            let axis = (-ey, ex);
            let (la, ha) = project(axis, a);
            let (lb, hb) = project(axis, b);
            if ha < lb || hb < la {
                return None; // separating axis
            }
            let depth = ha.min(hb) - la.max(lb);
            match best {
                None => best = Some(Collision { axis, depth }),
                Some(c) if depth < c.depth => best = Some(Collision { axis, depth }),
                _ => {}
            }
        }
    }
    let mut c = match best {
        Some(c) => c,
        None => {
            // No usable edge normals: both sides are a single point
            // (possibly duplicated). They collide iff identical.
            if a.iter().all(|&p| p == a[0]) && b.iter().all(|&p| p == b[0]) && a[0] == b[0] {
                Collision {
                    axis: (1, 0),
                    depth: 0,
                }
            } else {
                return None;
            }
        }
    };
    // Orient the axis a→b canonically.
    let (ca, cb) = (centroid(a), centroid(b));
    let diff = (
        cb.0 * a.len() as i128 - ca.0 * b.len() as i128,
        cb.1 * a.len() as i128 - ca.1 * b.len() as i128,
    );
    if c.axis.0 as i128 * diff.0 + c.axis.1 as i128 * diff.1 < 0 {
        c.axis = (-c.axis.0, -c.axis.1);
    }
    Some(c)
}

/// Boolean form of [`collide`].
pub fn overlap(a: &[(i32, i32)], b: &[(i32, i32)]) -> bool {
    collide(a, b).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poly::{convex_hull, point_in_polygon, PointLocation};
    use crate::rng::SplitMix64;
    use crate::segment::{point_on_segment, segments_intersect};

    /// Independent oracle: an edge pair intersects (inclusive) or a
    /// vertex of one is inside/on the other — exact for convex sets.
    fn oracle(a: &[(i32, i32)], b: &[(i32, i32)]) -> bool {
        if a.is_empty() || b.is_empty() {
            return false;
        }
        for i in 0..a.len() {
            for j in 0..b.len() {
                if segments_intersect(a[i], a[(i + 1) % a.len()], b[j], b[(j + 1) % b.len()]) {
                    return true;
                }
            }
        }
        let on_or_in = |p: (i32, i32), poly: &[(i32, i32)]| {
            matches!(
                point_in_polygon(p, poly),
                PointLocation::Inside | PointLocation::OnBoundary
            )
        };
        if a.len() >= 3 && b.len() >= 3 {
            return a.iter().any(|&p| on_or_in(p, b)) || b.iter().any(|&p| on_or_in(p, a));
        }
        // Degenerate (point/segment) cases.
        let distinct = |p: &[(i32, i32)]| {
            let mut v = p.to_vec();
            v.sort();
            v.dedup();
            v
        };
        let (da, db) = (distinct(a), distinct(b));
        if da.len() == 1 && db.len() == 1 {
            return da[0] == db[0];
        }
        if da.len() == 1 && db.len() == 2 && point_on_segment(db[0], db[1], da[0]) {
            return true;
        }
        if db.len() == 1 && da.len() == 2 && point_on_segment(da[0], da[1], db[0]) {
            return true;
        }
        false
    }

    fn rand_convex(rng: &mut SplitMix64, n: usize, span: i32) -> Vec<(i32, i32)> {
        let pts: Vec<(i32, i32)> = (0..n)
            .map(|_| {
                (
                    rng.below(span as u32) as i32 - span / 2,
                    rng.below(span as u32) as i32 - span / 2,
                )
            })
            .collect();
        convex_hull(&pts)
    }

    #[test]
    fn matches_edge_oracle() {
        let mut rng = SplitMix64::new(0x5A7);
        let mut checked = 0;
        for _ in 0..3000 {
            let na = 3 + rng.below(6) as usize;
            let nb = 3 + rng.below(6) as usize;
            let a = rand_convex(&mut rng, na, 40);
            let b = rand_convex(&mut rng, nb, 40);
            if a.len() < 3 || b.len() < 3 {
                continue;
            }
            checked += 1;
            let expect = oracle(&a, &b);
            assert_eq!(overlap(&a, &b), expect, "a={a:?} b={b:?}");
            if expect {
                assert!(collide(&a, &b).unwrap().depth >= 0);
            }
        }
        assert!(checked > 500);
    }

    #[test]
    fn depth_and_axis_orientation() {
        // x-overlap [3,4]→1 vs y-overlap [0,4]→4: min axis is x-ish.
        let a = [(0, 0), (4, 0), (4, 4), (0, 4)];
        let b = [(3, 0), (7, 0), (7, 4), (3, 4)];
        let c = collide(&a, &b).unwrap();
        // x-normal wins: 4 projection units = 1 true unit × |axis|4.
        assert_eq!(c.depth, 4);
        assert_eq!(c.axis, (4, 0)); // oriented a→b
                                    // Reverse operand order: axis flips, depth holds.
        let c2 = collide(&b, &a).unwrap();
        assert_eq!(c2.depth, 4);
        assert_eq!(c2.axis, (-4, 0));
    }

    #[test]
    fn touching_boundaries_and_degenerates() {
        let a = [(0, 0), (4, 0), (4, 4), (0, 4)];
        let edge = [(4, 0), (8, 0), (8, 4), (4, 4)]; // shares x=4 edge
        let corner = [(4, 4), (8, 4), (8, 8), (4, 8)]; // touches at (4,4)
        assert!(overlap(&a, &edge));
        assert_eq!(collide(&a, &edge).unwrap().depth, 0);
        assert!(overlap(&a, &corner));
        // point vs polygon
        assert!(overlap(&[(2, 2)], &a));
        assert!(!overlap(&[(9, 2)], &a));
        // identical points collide, distinct don't
        assert!(overlap(&[(2, 2)], &[(2, 2)]));
        assert!(!overlap(&[(2, 2)], &[(3, 2)]));
        // empty never collides
        assert!(!overlap(&[], &a));
    }
}
