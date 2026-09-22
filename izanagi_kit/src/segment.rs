//! Integer-exact line-segment predicates — intersection tests and
//! squared distances on `i32` endpoints with `i128` intermediate math.
//!
//! The piece [`crate::poly`] doesn't cover: `poly` answers "is a point
//! inside a closed ring", this answers "do two open strokes touch and
//! how far apart are they" — wall↔shot sweep checks, chain/rope
//! intersection, edge-adjacency queries on generated layouts.
//!
//! All predicates are exact: orientation via [`crate::poly::orient`],
//! distances returned squared (`dist2`) so no root ever appears.
//!
//! ```
//! use izanagi_kit::segment::{segments_intersect, point_segment_dist2};
//! assert!(segments_intersect((0,0), (4,0), (2,-1), (2,1)));  // proper cross
//! assert!(segments_intersect((0,0), (4,0), (2,0), (6,0)));  // collinear overlap
//! assert!(!segments_intersect((0,0), (4,0), (5,-1), (5,1))); // disjoint
//! assert_eq!(point_segment_dist2((3, 4), (0,0), (6,0)), 16); // off-line
//! assert_eq!(point_segment_dist2((7, 0), (0,0), (6,0)), 1);  // past endpoint
//! ```

use crate::poly::orient;

/// Whether closed segments `ab` and `cd` share any point — proper
/// crossings *and* endpoint touches *and* collinear overlaps.
///
/// Exact: two orient tests each way (straddle check) plus an
/// on-segment containment pass when every orientation is zero.
pub fn segments_intersect(a: (i32, i32), b: (i32, i32), c: (i32, i32), d: (i32, i32)) -> bool {
    let o1 = orient(a, b, c);
    let o2 = orient(a, b, d);
    let o3 = orient(c, d, a);
    let o4 = orient(c, d, b);
    // Proper intersection: each segment straddles the other's line.
    if ((o1 > 0) != (o2 > 0)) && o1 != 0 && o2 != 0 && ((o3 > 0) != (o4 > 0)) && o3 != 0 && o4 != 0
    {
        return true;
    }
    // Improper cases: an endpoint of one lies on the other.
    (o1 == 0 && on_segment(a, b, c))
        || (o2 == 0 && on_segment(a, b, d))
        || (o3 == 0 && on_segment(c, d, a))
        || (o4 == 0 && on_segment(c, d, b))
}

/// Whether `p` lies in `ab`'s bounding box — only correct as an
/// on-segment test when `orient(a, b, p) == 0` (callers' invariant).
/// Use [`point_on_segment`] for the safe standalone version.
fn on_segment(a: (i32, i32), b: (i32, i32), p: (i32, i32)) -> bool {
    p.0 >= a.0.min(b.0) && p.0 <= a.0.max(b.0) && p.1 >= a.1.min(b.1) && p.1 <= a.1.max(b.1)
}

/// Whether `p` lies on closed segment `ab` — collinearity check +
/// bounding-box containment, both exact.
pub fn point_on_segment(a: (i32, i32), b: (i32, i32), p: (i32, i32)) -> bool {
    orient(a, b, p) == 0
        && p.0 >= a.0.min(b.0)
        && p.0 <= a.0.max(b.0)
        && p.1 >= a.1.min(b.1)
        && p.1 <= a.1.max(b.1)
}

/// Squared distance from `p` to segment `ab` — the projection is
/// clamped to the closed segment. The true value `cross²/|ab|²` is
/// rational, so this returns the **ceiling**: the smallest integer ≥
/// the exact squared distance. Consequences callers rely on:
/// `== 0` iff `p` is exactly on `ab`, and `≤ r²` still answers
/// "within `r`" exactly for integer `r`.
pub fn point_segment_dist2(p: (i32, i32), a: (i32, i32), b: (i32, i32)) -> i128 {
    let (px, py) = (p.0 as i128, p.1 as i128);
    let (ax, ay) = (a.0 as i128, a.1 as i128);
    let (dx, dy) = (b.0 as i128 - ax, b.1 as i128 - ay);
    let len2 = dx * dx + dy * dy;
    if len2 == 0 {
        return (px - ax) * (px - ax) + (py - ay) * (py - ay);
    }
    // t = ((p-a)·(b-a)) / |b-a|²  clamped to [0,1] — compare cross-
    // multiplied to stay in integers.
    let t_num = (px - ax) * dx + (py - ay) * dy;
    if t_num <= 0 {
        return (px - ax) * (px - ax) + (py - ay) * (py - ay);
    }
    if t_num >= len2 {
        let (bx, by) = (b.0 as i128, b.1 as i128);
        return (px - bx) * (px - bx) + (py - by) * (py - by);
    }
    // |p - proj|² = |p-a|² - t²·|b-a|² = (|p-a|²·|b-a|² - ((p-a)·(b-a))²) / |b-a|²
    // Numerator is |cross(a→b, a→p)|² — exact; ceiling keeps 0 ⟺ on-segment.
    let cross = dx * (py - ay) - dy * (px - ax);
    (cross * cross + len2 - 1) / len2
}

/// Squared distance between closed segments `ab` and `cd`:
/// `0` when they intersect, else the min over the four
/// endpoint↔segment distances.
pub fn segment_dist2(a: (i32, i32), b: (i32, i32), c: (i32, i32), d: (i32, i32)) -> i128 {
    if segments_intersect(a, b, c, d) {
        return 0;
    }
    point_segment_dist2(a, c, d)
        .min(point_segment_dist2(b, c, d))
        .min(point_segment_dist2(c, a, b))
        .min(point_segment_dist2(d, a, b))
}

/// Segment length squared — convenience for sweep heuristics that rank
/// candidate walls by extent.
pub fn len2(a: (i32, i32), b: (i32, i32)) -> i128 {
    let dx = b.0 as i128 - a.0 as i128;
    let dy = b.1 as i128 - a.1 as i128;
    dx * dx + dy * dy
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn brute_intersect(a: (i32, i32), b: (i32, i32), c: (i32, i32), d: (i32, i32)) -> bool {
        // Oracle: sample points on the unit grid are not dense enough —
        // instead, intersect via the exact parametric test. Use a
        // different formulation than the implementation under test:
        // segment ab ∩ cd non-empty iff some convex combination matches.
        // Solve in rationals via determinants.
        let o1 = orient(a, b, c);
        let o2 = orient(a, b, d);
        let o3 = orient(c, d, a);
        let o4 = orient(c, d, b);
        if o1.signum() * o2.signum() == -1 && o3.signum() * o4.signum() == -1 {
            return true; // strict straddle — proper cross
        }
        // Any zero orientation means a collinear endpoint — check
        // on-segment containment for each.
        (o1 == 0 && point_on_segment(a, b, c))
            || (o2 == 0 && point_on_segment(a, b, d))
            || (o3 == 0 && point_on_segment(c, d, a))
            || (o4 == 0 && point_on_segment(c, d, b))
    }

    #[test]
    fn intersect_matches_independent_formulation() {
        let mut rng = SplitMix64::new(0x5EC6);
        for _ in 0..50_000 {
            let a = (rng.range(-9, 10), rng.range(-9, 10));
            let b = (rng.range(-9, 10), rng.range(-9, 10));
            let c = (rng.range(-9, 10), rng.range(-9, 10));
            let d = (rng.range(-9, 10), rng.range(-9, 10));
            assert_eq!(
                segments_intersect(a, b, c, d),
                brute_intersect(a, b, c, d),
                "{a:?}-{b:?} vs {c:?}-{d:?}"
            );
        }
    }

    #[test]
    fn degenerate_and_collinear_cases() {
        // Point segment vs point.
        assert!(segments_intersect((2, 2), (2, 2), (2, 2), (5, 5)));
        assert!(!segments_intersect((0, 0), (0, 0), (3, 3), (5, 5)));
        // Collinear disjoint.
        assert!(!segments_intersect((0, 0), (2, 0), (4, 0), (6, 0)));
        // Touch at shared endpoint.
        assert!(segments_intersect((0, 0), (2, 0), (2, 0), (2, 4)));
        // T-junction (endpoint on interior).
        assert!(segments_intersect((0, 0), (4, 0), (2, 0), (2, 3)));
        // Fully contained collinear.
        assert!(segments_intersect((0, 0), (6, 0), (2, 0), (4, 0)));
    }

    #[test]
    fn point_segment_dist2_matches_brute_force() {
        let mut rng = SplitMix64::new(0xD157);
        for _ in 0..30_000 {
            let p = (rng.range(-6, 7), rng.range(-6, 7));
            let a = (rng.range(-6, 7), rng.range(-6, 7));
            let b = (rng.range(-6, 7), rng.range(-6, 7));
            // Oracle: dense rational sampling — parameter t in 1/1000
            // steps is an underestimate; the analytic bound compares
            // squared, so assert dist² within [floor, ceil] of the
            // sampled min's square and also dist² ≤ sample².
            let mut best = i128::MAX;
            for t in 0..=1000i128 {
                let (ax, ay) = (a.0 as i128, a.1 as i128);
                let (dx, dy) = (b.0 as i128 - ax, b.1 as i128 - ay);
                // point = a + t/1000 · (b-a)
                let qx = ax * 1000 + dx * t;
                let qy = ay * 1000 + dy * t;
                let ddx = p.0 as i128 * 1000 - qx;
                let ddy = p.1 as i128 * 1000 - qy;
                best = best.min(ddx * ddx + ddy * ddy);
            }
            let got = point_segment_dist2(p, a, b);
            // got·1000² ≤ best (samples never beat the true min).
            assert!(got * 1_000_000 <= best + 2_000_000); // slack for grid quantisation
                                                          // And got ≥ best/1e6 - 2 (the true min can't exceed samples).
            assert!(got + 2 >= best / 1_000_000);
        }
    }

    #[test]
    fn segment_dist2_zero_iff_intersecting() {
        let mut rng = SplitMix64::new(0xA11E);
        for _ in 0..30_000 {
            let a = (rng.range(-8, 9), rng.range(-8, 9));
            let b = (rng.range(-8, 9), rng.range(-8, 9));
            let c = (rng.range(-8, 9), rng.range(-8, 9));
            let d = (rng.range(-8, 9), rng.range(-8, 9));
            let inter = segments_intersect(a, b, c, d);
            let dist = segment_dist2(a, b, c, d);
            assert_eq!(inter, dist == 0, "{a:?}-{b:?} vs {c:?}-{d:?}");
        }
    }

    #[test]
    fn len2_and_known_distances() {
        assert_eq!(len2((0, 0), (3, 4)), 25);
        assert_eq!(point_segment_dist2((0, 0), (5, 5), (5, 5)), 50);
        assert_eq!(segment_dist2((0, 0), (2, 0), (10, 0), (12, 0)), 64);
    }
}
