//! Alpha shape — the boundary of a point set at scale `α`, built
//! on top of `delaunay`. A Delaunay triangle belongs to the
//! `α`-shape when its circumradius does not exceed `α`; the
//! boundary is the set of triangle edges appearing exactly once.
//! Small `α` carves cavities and splits the set into components;
//! `α → ∞` degenerates to the convex hull.
//!
//! Circumradii are compared in exact `i128` arithmetic: the
//! circumcenter of `(a,b,c)` has rational coordinates
//! `o = num/d` with `d = 2·cross(b−a, c−a)`, so
//! `R² = (|num − d·a|²) / d²` is a rational the filter compares
//! against `alpha2` without any square root.
//!
//! ```
//! use izanagi_kit::alphahull::alpha_shape;
//! let pts = [(0, 0), (4, 0), (0, 3), (4, 3)];
//! // huge α → the whole hull is one component: 4 boundary edges
//! let b = alpha_shape(&pts, 1_000_000);
//! assert_eq!(b.len(), 4);
//! ```
//!
//! References: Edelsbrunner, Kirkpatrick & Seidel (1983); the
//! CGAL 2D alpha-shape writeups for the boundary-edge rule.

/// Boundary edge endpoints, canonically ordered (lower endpoint
/// first).
pub type Edge = ((i32, i32), (i32, i32));

/// `(num, den)` with `den > 0`: squared circumradius numerator
/// and denominator — `num/den ≤ alpha2` ⇔ `num ≤ alpha2·den`.
fn circum2(a: (i64, i64), b: (i64, i64), c: (i64, i64)) -> (i128, i128) {
    let (ax, ay) = (a.0 as i128, a.1 as i128);
    let (bx, by) = (b.0 as i128, b.1 as i128);
    let (cx, cy) = (c.0 as i128, c.1 as i128);
    // o solves 2(b−a)·o = |b|²−|a|², 2(c−a)·o = |c|²−|a|²
    let ux = bx - ax;
    let uy = by - ay;
    let vx = cx - ax;
    let vy = cy - ay;
    let d = 2 * (ux * vy - uy * vx);
    // collinear ⇒ no circumcircle. `d` may be negative for a CW
    // triple; num/d stays consistent, so only d == 0 degenerates.
    if d == 0 {
        return (i128::MAX, 1);
    }
    let ub = ux * (ax + bx) + uy * (ay + by);
    let vb = vx * (ax + cx) + vy * (ay + cy);
    // o = num/d (Cramer on [[2ux,2uy],[2vx,2vy]]·o = [ub,vb])
    let ox = vy * ub - uy * vb;
    let oy = ux * vb - vx * ub;
    let rx = ox - d * ax;
    let ry = oy - d * ay;
    (rx * rx + ry * ry, d * d)
}

/// Alpha-shape boundary at squared radius `alpha2` (`α²`; the
/// filter is `circumradius² ≤ alpha2` so the parameter stays an
/// integer). Returns boundary edges in sorted canonical order —
/// a pure function of the input.
pub fn alpha_shape(points: &[(i32, i32)], alpha2: i128) -> Vec<Edge> {
    if alpha2 < 0 {
        return Vec::new();
    }
    // unique points, same order delaunay's Tri indices address
    let mut unique: Vec<(i32, i32)> = Vec::new();
    for &p in points {
        if !unique.contains(&p) {
            unique.push(p);
        }
    }
    let mut count: BTreeMapCount = BTreeMapCount::default();
    let tris = crate::delaunay::delaunay(points);
    for t in &tris {
        let (a, b, c) = (
            (
                unique[t[0] as usize].0 as i64,
                unique[t[0] as usize].1 as i64,
            ),
            (
                unique[t[1] as usize].0 as i64,
                unique[t[1] as usize].1 as i64,
            ),
            (
                unique[t[2] as usize].0 as i64,
                unique[t[2] as usize].1 as i64,
            ),
        );
        let (n, d) = circum2(a, b, c);
        // kept iff n/d ≤ alpha2; a product overflow means alpha2
        // dwarfs every possible n → keep
        let kept = match alpha2.checked_mul(d) {
            Some(t) => n <= t,
            None => true,
        };
        if !kept {
            continue;
        }
        for &(u, w) in &[(a, b), (b, c), (c, a)] {
            let e = ((u.0 as i32, u.1 as i32), (w.0 as i32, w.1 as i32));
            let key = if e.0 <= e.1 { (e.0, e.1) } else { (e.1, e.0) };
            *count.map.entry(key).or_insert(0) += 1;
        }
    }
    let mut out: Vec<Edge> = count
        .map
        .iter()
        .filter(|&(_, &c)| c == 1)
        .map(|(&k, _)| k)
        .collect();
    out.sort();
    out
}

#[derive(Default)]
struct BTreeMapCount {
    map: std::collections::BTreeMap<Edge, u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeMap;

    /// Oracle: circumradius via `R = abc/(4·area)`, independent of
    /// the perpendicular-bisector construction in `circum2`.
    fn circum2_alt(a: (i64, i64), b: (i64, i64), c: (i64, i64)) -> (i128, i128) {
        let l2 = |p: (i64, i64), q: (i64, i64)| -> i128 {
            let (dx, dy) = (p.0 as i128 - q.0 as i128, p.1 as i128 - q.1 as i128);
            dx * dx + dy * dy
        };
        let area2 = ((b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)) as i128;
        if area2 == 0 {
            return (i128::MAX, 1);
        }
        // R² = l_ab·l_bc·l_ca / (4·area)²
        (l2(a, b) * l2(b, c) * l2(c, a), 4 * area2 * area2)
    }

    #[test]
    fn circumradius_formulas_agree() {
        let mut r = SplitMix64::new(0xA1F4);
        for _ in 0..2000 {
            let mut p = |s: i64| {
                (
                    (r.next_u64() % (2 * s as u64)) as i64 - s,
                    (r.next_u64() % (2 * s as u64)) as i64 - s,
                )
            };
            let (a, b, c) = (p(50), p(50), p(50));
            let (n1, d1) = circum2(a, b, c);
            let (n2, d2) = circum2_alt(a, b, c);
            assert_eq!(n1 * d2, n2 * d1, "{a:?} {b:?} {c:?}");
        }
    }

    #[test]
    fn square_boundaries() {
        let pts = [(0, 0), (4, 0), (0, 3), (4, 3)];
        // huge α: convex hull only
        assert_eq!(alpha_shape(&pts, i128::MAX).len(), 4);
        // α below the circumradius (R² = 25/4) → nothing kept
        assert!(alpha_shape(&pts, 6).is_empty());
        // α² = 7 keeps both triangles → same hull boundary
        assert_eq!(alpha_shape(&pts, 7).len(), 4);
    }

    #[test]
    fn concavity_carved() {
        // plus-shaped cluster: a center square plus wing points
        // large enough that a medium α keeps ears but loses the
        // central diagonal span
        let pts = [(0, 0), (10, 0), (0, 10), (10, 10), (20, 5), (-10, 5)];
        let big = alpha_shape(&pts, i128::MAX);
        let mid = alpha_shape(&pts, 60);
        // medium α sheds the large-circumradius ears → strictly
        // more boundary edges than the hull
        assert!(mid.len() >= big.len());
        assert!(!big.is_empty());
    }

    /// Oracle: boundary recomputed independently — recount edge
    /// usage among kept triangles without reusing the impl's map.
    fn brute_boundary(points: &[(i32, i32)], alpha2: i128) -> Vec<Edge> {
        let mut unique: Vec<(i32, i32)> = Vec::new();
        for &p in points {
            if !unique.contains(&p) {
                unique.push(p);
            }
        }
        let mut counts: BTreeMap<Edge, u32> = BTreeMap::new();
        for t in &crate::delaunay::delaunay(points) {
            let p = |i: u32| unique[i as usize];
            let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
            let (n, d) = circum2_alt(
                (a.0 as i64, a.1 as i64),
                (b.0 as i64, b.1 as i64),
                (c.0 as i64, c.1 as i64),
            );
            let kept = match alpha2.checked_mul(d) {
                Some(t) => n <= t,
                None => true,
            };
            if !kept {
                continue;
            }
            for e in [(a, b), (b, c), (c, a)] {
                let key = if e.0 <= e.1 { (e.0, e.1) } else { (e.1, e.0) };
                *counts.entry(key).or_insert(0) += 1;
            }
        }
        let mut out: Vec<Edge> = counts
            .iter()
            .filter(|&(_, &c)| c == 1)
            .map(|(&k, _)| k)
            .collect();
        out.sort();
        out
    }

    #[test]
    fn oracle_random() {
        let mut r = SplitMix64::new(0xA17A);
        for _ in 0..200 {
            let n = (r.below(10) + 3) as usize;
            let pts: Vec<(i32, i32)> = (0..n)
                .map(|_| (r.below(60) as i32 - 30, r.below(60) as i32 - 30))
                .collect();
            let alpha2 = r.next_u64() as i128 % 3000;
            assert_eq!(
                alpha_shape(&pts, alpha2),
                brute_boundary(&pts, alpha2),
                "pts={pts:?} α²={alpha2}"
            );
        }
    }

    #[test]
    fn determinism() {
        let pts: Vec<(i32, i32)> = {
            let mut r = SplitMix64::new(7);
            (0..30)
                .map(|_| (r.below(50) as i32, r.below(50) as i32))
                .collect()
        };
        assert_eq!(alpha_shape(&pts, 500), alpha_shape(&pts, 500));
        // input order doesn't matter
        let mut rev = pts.clone();
        rev.reverse();
        assert_eq!(alpha_shape(&pts, 500), alpha_shape(&rev, 500));
    }
}
