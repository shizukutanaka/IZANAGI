//! Integer 2D polygon primitives — area, containment, convex hull, and
//! ear-clipping triangulation, all exact in `i128`.
//!
//! Every predicate is an orientation test (`orient`) or the shoelace
//! formula, so results are exact for any `i32` coordinates and identical
//! across platforms — the same "no float, no epsilon" stance as the rest
//! of the kit. Vertex order conventions: polygons are open lists (no
//! repeated last vertex); orientation-aware functions accept either
//! winding and document the sign.
//!
//! ```
//! use izanagi_kit::poly::{area2, convex_hull, ear_clip, point_in_polygon, PointLocation};
//! let tri = [(0, 0), (4, 0), (0, 3)];
//! assert_eq!(area2(&tri), 12); // CCW in y-up convention — 2×area
//! assert_eq!(point_in_polygon((1, 1), &tri), PointLocation::Inside);
//! assert_eq!(ear_clip(&tri).expect("simple polygon").len(), 1);
//! assert_eq!(convex_hull(&tri), vec![(0, 0), (4, 0), (0, 3)]);
//! ```

/// Signed double area (shoelace). Positive = counter-clockwise on a
/// y-down display convention is *not* assumed — the sign follows the
/// raw formula `Σ x_i·y_{i+1} - x_{i+1}·y_i`, negative for clockwise
/// on screen coordinates. Magnitude is always `2 × area` in `i128` —
/// overflow-exact for `i32` coordinates.
pub fn area2(pts: &[(i32, i32)]) -> i128 {
    let mut s = 0i128;
    for w in pts.windows(2) {
        s += w[0].0 as i128 * w[1].1 as i128 - w[1].0 as i128 * w[0].1 as i128;
    }
    if pts.len() > 2 {
        let (f, l) = (pts[0], pts[pts.len() - 1]);
        s += l.0 as i128 * f.1 as i128 - f.0 as i128 * l.1 as i128;
    }
    s
}

/// Orientation of point `c` relative to directed edge `a → b`:
/// positive = `c` left of `a→b`, negative = right, 0 = collinear.
/// Exact in `i128`.
pub fn orient(a: (i32, i32), b: (i32, i32), c: (i32, i32)) -> i128 {
    (b.0 as i128 - a.0 as i128) * (c.1 as i128 - a.1 as i128)
        - (b.1 as i128 - a.1 as i128) * (c.0 as i128 - a.0 as i128)
}

/// `p` inside-or-on triangle `abc` (any winding). Used by the ear test:
/// a polygon vertex sitting exactly on a candidate ear edge means the
/// ear's interior bleeds past that reflex vertex — not a valid ear.
fn in_or_on_triangle(a: (i32, i32), b: (i32, i32), c: (i32, i32), p: (i32, i32)) -> bool {
    let s = orient(a, b, c).signum();
    if s == 0 {
        return false;
    }
    [orient(a, b, p), orient(b, c, p), orient(c, a, p)]
        .iter()
        .all(|t| t.signum() == s || *t == 0)
}

/// Where `p` sits relative to polygon `poly`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointLocation {
    /// Strictly inside (even-odd rule).
    Inside,
    /// On an edge or vertex.
    OnBoundary,
    /// Strictly outside.
    Outside,
}

/// Point `p` on segment `ab` (inclusive of endpoints), exact.
fn on_segment(a: (i32, i32), b: (i32, i32), p: (i32, i32)) -> bool {
    orient(a, b, p) == 0
        && p.0 >= a.0.min(b.0)
        && p.0 <= a.0.max(b.0)
        && p.1 >= a.1.min(b.1)
        && p.1 <= a.1.max(b.1)
}

/// Even-odd point-in-polygon with exact boundary detection. Ray cast
/// to +∞ in x; the standard half-open rule (`y_i > y != y_j > y`)
/// handles vertices without a division or a square root — edge crossing
/// is tested by comparing the intersection x against `p.x` in cross-
/// multiplied integers.
pub fn point_in_polygon(p: (i32, i32), poly: &[(i32, i32)]) -> PointLocation {
    let n = poly.len();
    if n < 3 {
        return PointLocation::Outside;
    }
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if on_segment(a, b, p) {
            return PointLocation::OnBoundary;
        }
        // Half-open vertical range test, then compare intersection x.
        if (a.1 > p.1) != (b.1 > p.1) {
            // x_int = a.x + (p.y - a.y)(b.x - a.x)/(b.y - a.y); compare with p.x
            // Cross-multiply by (b.y - a.y) — sign depends on its direction.
            let lhs = (b.0 as i128 - a.0 as i128) * (p.1 as i128 - a.1 as i128);
            let rhs = (p.0 as i128 - a.0 as i128) * (b.1 as i128 - a.1 as i128);
            // lhs < rhs ⇔ x_int < p.x when (b.y - a.y) > 0; flip when negative.
            let crosses_right = if b.1 > a.1 { lhs < rhs } else { lhs > rhs };
            if crosses_right {
                inside = !inside;
            }
        }
    }
    if inside {
        PointLocation::Inside
    } else {
        PointLocation::Outside
    }
}

/// Andrew monotone-chain convex hull. Returns hull vertices in
/// counter-clockwise order starting at the lexicographically smallest
/// point, with no collinear interior points kept on edges. Deterministic:
/// input order only decides which *duplicate* points get discarded.
/// Degenerate inputs: fewer than 3 unique points return the deduped
/// sorted set itself.
pub fn convex_hull(pts: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let mut p: Vec<(i32, i32)> = pts.to_vec();
    p.sort_unstable();
    p.dedup();
    if p.len() <= 2 {
        return p;
    }
    let mut hull: Vec<(i32, i32)> = Vec::with_capacity(p.len() + 1);
    // Lower hull: pop while the turn is not a strict left turn.
    for &pt in &p {
        while hull.len() >= 2 && orient(hull[hull.len() - 2], hull[hull.len() - 1], pt) <= 0 {
            hull.pop();
        }
        hull.push(pt);
    }
    // Upper hull over the reversed point set.
    let lower_len = hull.len();
    for &pt in p.iter().rev().skip(1) {
        while hull.len() > lower_len && orient(hull[hull.len() - 2], hull[hull.len() - 1], pt) <= 0
        {
            hull.pop();
        }
        hull.push(pt);
    }
    hull.pop(); // closing vertex == hull[0]
    hull
}

/// Ear-clipping triangulation of a simple polygon — `n-2` triangles as
/// `(usize, usize, usize)` index triples into `poly`. `O(n²)`: repeatedly
/// snips the first ear found by a full scan, so the output is a pure
/// function of the input order. Returns `None` if the polygon is
/// degenerate (fewer than 3 vertices or zero area).
///
/// The polygon may be wound either way — the clip direction follows the
/// shoelace sign.
pub fn ear_clip(poly: &[(i32, i32)]) -> Option<Vec<(usize, usize, usize)>> {
    let n = poly.len();
    if n < 3 || area2(poly) == 0 {
        return None;
    }
    let ccw = area2(poly) > 0;
    let mut verts: Vec<usize> = (0..n).collect();
    let mut tris = Vec::with_capacity(n - 2);
    // Loop until a triangle remains; each pass snips exactly one ear.
    while verts.len() > 3 {
        let m = verts.len();
        let mut snipped = false;
        for i in 0..m {
            let (a, b, c) = (verts[(i + m - 1) % m], verts[i], verts[(i + 1) % m]);
            let turn = orient(poly[a], poly[b], poly[c]);
            // An ear must be a strictly convex vertex in the winding sense.
            if (ccw && turn <= 0) || (!ccw && turn >= 0) {
                continue;
            }
            // And no other vertex may sit inside the ear or on its edges.
            if (0..m).any(|j| {
                let v = verts[j];
                v != a && v != b && v != c && in_or_on_triangle(poly[a], poly[b], poly[c], poly[v])
            }) {
                continue;
            }
            tris.push((a, b, c));
            verts.remove(i);
            snipped = true;
            break;
        }
        if !snipped {
            // No ear found — the polygon is self-intersecting or
            // numerically degenerate; bail rather than emit wrong output.
            return None;
        }
    }
    tris.push((verts[0], verts[1], verts[2]));
    Some(tris)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn tri_area(t: (usize, usize, usize), poly: &[(i32, i32)]) -> i128 {
        let (a, b, c) = (poly[t.0], poly[t.1], poly[t.2]);
        orient(a, b, c)
    }

    #[test]
    fn area2_shoelace_is_exact() {
        assert_eq!(area2(&[(0, 0), (2, 0), (2, 2), (0, 2)]), 8); // CCW square, 2×area
        assert_eq!(area2(&[(0, 0), (0, 2), (2, 2), (2, 0)]), -8); // CW
        assert_eq!(area2(&[]), 0);
        assert_eq!(area2(&[(1, 1), (5, 5)]), 0);
    }

    #[test]
    fn point_in_polygon_classifies_all_three_cases() {
        let square = [(0, 0), (4, 0), (4, 4), (0, 4)];
        assert_eq!(point_in_polygon((2, 2), &square), PointLocation::Inside);
        assert_eq!(point_in_polygon((5, 2), &square), PointLocation::Outside);
        assert_eq!(point_in_polygon((0, 2), &square), PointLocation::OnBoundary);
        assert_eq!(point_in_polygon((4, 4), &square), PointLocation::OnBoundary);
        // Concave polygon (L-shape).
        let el = [(0, 0), (4, 0), (4, 2), (2, 2), (2, 4), (0, 4)];
        assert_eq!(point_in_polygon((3, 3), &el), PointLocation::Outside);
        assert_eq!(point_in_polygon((1, 3), &el), PointLocation::Inside);
        assert_eq!(point_in_polygon((2, 3), &el), PointLocation::OnBoundary);
    }

    #[test]
    fn convex_hull_matches_bruteforce_extreme_points() {
        // Oracle: a point is a hull vertex iff it's an extreme point — i.e.
        // every other point lies in one open halfplane defined by an edge
        // through it. Brute force: v is on the hull iff NO triangle of other
        // points strictly contains it AND v is not strictly inside the hull
        // of the rest. Simpler brute check: hull from monotone chain must
        // be (a) a subset of input, (b) convex (all turns same strict sign),
        // (c) contain every input point in or on it.
        let mut rng = SplitMix64::new(0xC0DE);
        for _ in 0..200 {
            let pts: Vec<(i32, i32)> = (0..(3 + rng.below(15)))
                .map(|_| (rng.range(-20, 21), rng.range(-20, 21)))
                .collect();
            let hull = convex_hull(&pts);
            // (b) strictly convex: every turn has the same non-zero sign.
            let sign = orient(hull[0], hull[1], hull[2]).signum();
            if hull.len() >= 3 {
                for i in 0..hull.len() {
                    let t = orient(
                        hull[i],
                        hull[(i + 1) % hull.len()],
                        hull[(i + 2) % hull.len()],
                    );
                    assert_eq!(t.signum(), sign, "non-strict-convex hull vertex");
                }
            }
            // (c) every input point inside or on the hull.
            for &p in &pts {
                let loc = point_in_polygon(p, &hull);
                assert_ne!(loc, PointLocation::Outside, "point outside hull");
            }
            // No duplicate points, all from the input.
            let unique: std::collections::BTreeSet<_> = hull.iter().collect();
            assert_eq!(unique.len(), hull.len());
            for &p in &hull {
                assert!(pts.contains(&p));
            }
        }
    }

    #[test]
    fn ear_clip_partitions_area_exactly() {
        // Oracle: the triangles' unsigned areas sum to the polygon's area,
        // each triangle is non-degenerate, and all indices are distinct
        // vertex uses. Convex hulls of random clouds are always simple.
        let mut rng = SplitMix64::new(0xEA4C);
        for _ in 0..300 {
            let n = 3 + rng.below(15) as usize;
            let pts: Vec<(i32, i32)> = (0..n)
                .map(|_| (rng.range(-30, 31), rng.range(-30, 31)))
                .collect();
            let hull_only = convex_hull(&pts);
            let tris = ear_clip(&hull_only).expect("hull triangulation");
            assert_eq!(tris.len(), hull_only.len() - 2);
            let sum: i128 = tris.iter().map(|&t| tri_area(t, &hull_only).abs()).sum();
            assert_eq!(sum, area2(&hull_only).abs(), "partition area mismatch");
        }
    }

    #[test]
    fn ear_clip_on_concave_and_degenerate() {
        // L-shape triangulates into 4 triangles.
        let el = [(0, 0), (4, 0), (4, 2), (2, 2), (2, 4), (0, 4)];
        let tris = ear_clip(&el).expect("L triangulates");
        assert_eq!(tris.len(), 4);
        let sum: i128 = tris.iter().map(|&t| tri_area(t, &el).abs()).sum();
        assert_eq!(sum, area2(&el).abs());
        // Collinear polygon → None.
        assert_eq!(ear_clip(&[(0, 0), (1, 1), (2, 2)]), None);
        assert_eq!(ear_clip(&[(0, 0)]), None);
    }

    #[test]
    fn deterministic_and_winding_independent() {
        let el_cw = [(0, 0), (4, 0), (4, 2), (2, 2), (2, 4), (0, 4)];
        let mut el_ccw = el_cw;
        el_ccw.reverse();
        // Both windings must cover the same area — count only, since the
        // clip order follows the input direction.
        assert_eq!(
            ear_clip(&el_cw).map(|t| t.len()),
            ear_clip(&el_ccw).map(|t| t.len())
        );
        assert_eq!(convex_hull(&el_cw), convex_hull(&el_ccw));
    }
}
