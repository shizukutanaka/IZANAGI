//! Minkowski sum and difference of convex polygons — the
//! configuration-space primitive of path planning and the
//! separating-axis-free way to ask "do these convex shapes
//! overlap?".
//!
//! `sum(a, b) = {p + q : p ∈ a, q ∈ b}` on convex polygons
//! is computed by merging both polygons' edge vectors in
//! angular order — `O(n + m)` once the inputs are
//! canonicalized counter-clockwise from their lowest
//! `(y, x)` vertex:
//!
//! ```text
//! edges(a) = [ (5,0), (0,5), (-5,0), (0,-5) ]
//! edges(b) = [ (2,1), (-2,1), (-2,-1), (2,-1) ]
//! merged   → sum walks one vertex into each poly's turn
//! ```
//!
//! - `diff(a, b) = sum(a, reflect(b))` — the configuration
//!   space obstacle: `b` positioned at `p` collides with `a`
//!   iff `p ∈ diff(a, b)`.
//! - `collide(a, b)` answers the question directly by testing
//!   whether the origin lies inside `diff(a, b)`.
//!
//! Everything is i32 input / i128 cross products — exact,
//! no floating point. Non-convex inputs are hull-fuzzed via
//! `poly::convex_hull` first (documented, since a Minkowski
//! sum on a non-convex input isn't the polygon's sum at all).
//!
//! ```
//! use izanagi_kit::minkowski::{sum, collide};
//! let sq = vec![(0, 0), (4, 0), (4, 4), (0, 4)]; // CCW
//! let s = sum(&sq, &sq);
//! assert_eq!(s.len(), 4); // 8×8 square
//! assert!(collide(&sq, &sq)); // overlapping at origin
//! ```

use crate::poly::{convex_hull, point_in_polygon, PointLocation};

/// Rotate `p` so vertex 0 is the lowest `(y, x)` and force
/// CCW order. Convex input assumed; degenerate inputs
/// (<3 distinct points after hull) return the hull itself
/// which may be a point or segment.
fn canonical(p: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let hull = convex_hull(p);
    if hull.is_empty() {
        return hull;
    }
    // rotate lowest (y,x) to front
    let mut lo = 0usize;
    for i in 1..hull.len() {
        if (hull[i].1, hull[i].0) < (hull[lo].1, hull[lo].0) {
            lo = i;
        }
    }
    let mut out: Vec<(i32, i32)> = hull[lo..].to_vec();
    out.extend_from_slice(&hull[..lo]);
    out
}

/// Edge vectors of a canonical CCW polygon.
fn edge_vectors(p: &[(i32, i32)]) -> Vec<(i64, i64)> {
    let n = p.len();
    (0..n)
        .map(|i| {
            let (a, b) = (p[i], p[(i + 1) % n]);
            (
                i64::from(b.0) - i64::from(a.0),
                i64::from(b.1) - i64::from(a.1),
            )
        })
        .collect()
}

/// Minkowski sum of two convex polygons.
///
/// Inputs are canonicalized to CCW-from-lowest internally;
/// any winding works. Degenerate results (a point, a
/// segment) come back as the hull of the pair sums.
pub fn sum(a: &[(i32, i32)], b: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let a = canonical(a);
    let b = canonical(b);
    if a.is_empty() {
        return b;
    }
    if b.is_empty() {
        return a;
    }
    if a.len() < 3 || b.len() < 3 {
        // degenerate input: brute-force all pair sums → hull
        return degenerate_sum(&a, &b);
    }
    let ea = edge_vectors(&a);
    let eb = edge_vectors(&b);
    // merge by polar angle — cross product tells which
    // vector turns first; ties (parallel edges) emit both
    let mut out = Vec::with_capacity(a.len() + b.len());
    let mut cur = (
        i64::from(a[0].0) + i64::from(b[0].0),
        i64::from(a[0].1) + i64::from(b[0].1),
    );
    out.push(cur);
    let (mut i, mut j) = (0usize, 0usize);
    while i < ea.len() && j < eb.len() {
        let (va, vb) = (ea[i], eb[j]);
        let cross = va.0 * vb.1 - va.1 * vb.0;
        cur = if cross > 0 {
            // a's edge turns first
            i += 1;
            (cur.0 + va.0, cur.1 + va.1)
        } else if cross < 0 {
            j += 1;
            (cur.0 + vb.0, cur.1 + vb.1)
        } else {
            // parallel — combined step
            i += 1;
            j += 1;
            (cur.0 + va.0 + vb.0, cur.1 + va.1 + vb.1)
        };
        out.push(cur);
    }
    while i < ea.len() {
        let v = ea[i];
        i += 1;
        cur = (cur.0 + v.0, cur.1 + v.1);
        out.push(cur);
    }
    while j < eb.len() {
        let v = eb[j];
        j += 1;
        cur = (cur.0 + v.0, cur.1 + v.1);
        out.push(cur);
    }
    // last point returns to start — drop it, and hull to
    // strip collinear merged edges (parallel ties can leave
    // 180° vertices)
    out.pop();
    let pts: Vec<(i32, i32)> = out
        .iter()
        .map(|&(x, y)| (i32::try_from(x).unwrap_or(0), i32::try_from(y).unwrap_or(0)))
        .collect();
    convex_hull(&pts)
}

/// Pair-sums → hull for degenerate inputs (segments,
/// points). `O(n·m)`.
fn degenerate_sum(a: &[(i32, i32)], b: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let mut pts = Vec::with_capacity(a.len() * b.len());
    for &pa in a {
        for &pb in b {
            pts.push((pa.0 + pb.0, pa.1 + pb.1));
        }
    }
    convex_hull(&pts)
}

/// Configuration-space difference `a ⊖ b = a ⊕ (−b)`.
///
/// If `b` is a convex shape anchored at its own origin,
/// `diff(a, b)` is the set of positions where `b` touches
/// `a` — the C-obstacle of motion planning.
pub fn diff(a: &[(i32, i32)], b: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let neg: Vec<(i32, i32)> = b.iter().map(|&(x, y)| (-x, -y)).collect();
    sum(a, &neg)
}

/// Do convex polygons `a` and `b` intersect?
///
/// `b` is treated as positioned at the origin; translate
/// `b` before calling if needed. `a ∩ b ≠ ∅ ⟺ 0 ∈ diff(a, b)`.
pub fn collide(a: &[(i32, i32)], b: &[(i32, i32)]) -> bool {
    let d = diff(a, b);
    matches!(
        point_in_polygon((0, 0), &d),
        PointLocation::Inside | PointLocation::OnBoundary
    )
}

/// "b can be placed at `p` without touching `a`" is
/// `!point_in_polygon(p, diff(a,b))`. Exposed directly so
/// callers need not negate anything.
pub fn free_at(a: &[(i32, i32)], b: &[(i32, i32)], p: (i32, i32)) -> bool {
    let d = diff(a, b);
    point_in_polygon(p, &d) == PointLocation::Outside
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poly::orient;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    fn square(x: i32, y: i32, r: i32) -> Vec<(i32, i32)> {
        vec![
            (x - r, y - r),
            (x + r, y - r),
            (x + r, y + r),
            (x - r, y + r),
        ]
    }

    /// Brute oracle: all pairwise sums → convex hull. The
    /// merged-edge walk must produce the same polygon.
    /// Any edge of `a` crosses any edge of `b` (convex
    /// polys — boundary-touch counts as intersection).
    fn edges_cross(a: &[(i32, i32)], b: &[(i32, i32)]) -> bool {
        let na = a.len();
        let nb = b.len();
        for i in 0..na {
            for j in 0..nb {
                if crate::segment::segments_intersect(a[i], a[(i + 1) % na], b[j], b[(j + 1) % nb])
                {
                    return true;
                }
            }
        }
        false
    }

    fn brute_sum(a: &[(i32, i32)], b: &[(i32, i32)]) -> Vec<(i32, i32)> {
        let mut pts = Vec::with_capacity(a.len() * b.len());
        for &pa in a {
            for &pb in b {
                pts.push((pa.0 + pb.0, pa.1 + pb.1));
            }
        }
        convex_hull(&pts)
    }

    /// The square+square example from the docs — and a
    /// triangle to check non-parallel merging.
    #[test]
    fn basics() {
        let sq = square(0, 0, 2);
        let s = sum(&sq, &sq);
        assert_eq!(s.len(), 4);
        assert_eq!(crate::poly::area2(&s), 128); // area2 = 2×(8×8)
                                                 // triangle + square
        let tri = vec![(0, 0), (4, 0), (0, 4)];
        let s2 = sum(&sq, &tri);
        assert_eq!(s2, brute_sum(&sq, &tri));
    }

    /// Shadow oracle on random convex polygons — the merge
    /// equals pairwise-sum hull every time.
    #[test]
    fn oracle_brute_pair_sums() {
        let mut rng = SplitMix64::new(0xBEEF);
        for _ in 0..60 {
            let na = 3 + rng.below(6) as usize;
            let nb = 3 + rng.below(6) as usize;
            let mut pa: Vec<(i32, i32)> = (0..na)
                .map(|_| (rng.below(50) as i32, rng.below(50) as i32))
                .collect();
            let mut pb: Vec<(i32, i32)> = (0..nb)
                .map(|_| (rng.below(50) as i32, rng.below(50) as i32))
                .collect();
            pa = convex_hull(&pa);
            pb = convex_hull(&pb);
            if pa.len() < 3 || pb.len() < 3 {
                continue;
            }
            assert_eq!(sum(&pa, &pb), brute_sum(&pa, &pb), "{pa:?} + {pb:?}");
            // winding freedom — CW input must give the same
            let mut rev = pa.clone();
            rev.reverse();
            let mut got = sum(&rev, &pb);
            let want = sum(&pa, &pb);
            assert_eq!(
                BTreeSet::from_iter(got.drain(..)),
                BTreeSet::from_iter(want.into_iter())
            );
        }
    }

    /// C-obstacle semantics: b at p overlaps a ⟺ p ∈ diff.
    #[test]
    fn oracle_cobstacle() {
        let mut rng = SplitMix64::new(0xFACE);
        for _ in 0..40 {
            let a = {
                let mut p: Vec<(i32, i32)> = (0..4)
                    .map(|_| (rng.below(30) as i32, rng.below(30) as i32))
                    .collect();
                p = convex_hull(&p);
                if p.len() < 3 {
                    continue;
                }
                p
            };
            let b = square(0, 0, 3);
            let d = diff(&a, &b);
            // sample positions: collision ⟺ translate-b touches a
            for _ in 0..30 {
                let p = (rng.below(60) as i32 - 10, rng.below(60) as i32 - 10);
                let b_at_p: Vec<(i32, i32)> = b.iter().map(|&(x, y)| (x + p.0, y + p.1)).collect();
                // exact intersection test: any vertex in
                // other poly, or edges cross
                let brute = {
                    // point-in-polygon either way
                    let hit = |v: (i32, i32), poly: &[(i32, i32)]| {
                        matches!(
                            point_in_polygon(v, poly),
                            PointLocation::Inside | PointLocation::OnBoundary
                        )
                    };
                    b_at_p.iter().any(|&v| hit(v, &a))
                        || a.iter().any(|&v| hit(v, &b_at_p))
                        || edges_cross(&a, &b_at_p)
                };
                let loc = point_in_polygon(p, &d);
                let via_c = matches!(loc, PointLocation::Inside | PointLocation::OnBoundary);
                assert_eq!(via_c, brute, "p {p:?} d {d:?}");
            }
        }
    }

    /// `collide`/`free_at` convenience wrappers.
    #[test]
    fn collide_and_free() {
        let a = square(0, 0, 4);
        let b = square(0, 0, 2);
        assert!(collide(&a, &b)); // b at origin overlaps a
        assert!(!free_at(&a, &b, (0, 0)));
        assert!(free_at(&a, &b, (20, 20)));
        // a small b placed just outside a
        assert!(!collide(&square(0, 0, 4), &square(100, 100, 2)));
    }

    /// Degenerate inputs: point and segment.
    #[test]
    fn degenerates() {
        assert_eq!(sum(&[(3, 4)], &square(0, 0, 1)).len(), 4); // translated square
                                                               // segment + segment (parallel) → segment
        let seg = vec![(0, 0), (4, 0)];
        let s = sum(&seg, &seg);
        assert_eq!(s.len(), 2); // (0,0)-(8,0)
                                // empty input passes through the other side
        assert_eq!(sum(&[], &square(0, 0, 1)).len(), 4);
    }

    /// Determinism: same inputs → byte-identical output.
    #[test]
    fn deterministic() {
        let a = vec![(0, 0), (5, 0), (7, 3), (2, 5)];
        let b = vec![(1, 1), (4, 0), (3, 4), (0, 3)];
        assert_eq!(sum(&a, &b), sum(&a, &b));
        assert_eq!(diff(&a, &b), diff(&a, &b));
        // orient must agree on a generic output
        let s = sum(&a, &b);
        for i in 0..s.len() {
            assert!(
                orient(s[i], s[(i + 1) % s.len()], s[(i + 2) % s.len()]) >= 0,
                "hull not CCW at {i}"
            );
        }
    }
}
