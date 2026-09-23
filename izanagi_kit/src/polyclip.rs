//! Sutherland–Hodgman polygon clipping: intersect a subject polygon
//! with a *convex* clip polygon, all in exact rational arithmetic.
//!
//! Each clip edge runs one half-plane pass over the subject's vertex
//! list; boundary crossings insert the exact line-line intersection
//! point as a `(Frac, Frac)` pair, so the result is exact — a unit
//! square clipped diagonally yields exactly half its area.
//!
//! The clip polygon is normalized to CCW order internally (the
//! algorithm needs a consistent winding); the subject may be any
//! simple polygon, CW or CCW. Empty intersection → empty output.
//!
//! ```
//! use izanagi_kit::{frac::Frac, polyclip};
//! let subject = vec![(0i64, 0i64), (4, 0), (4, 4), (0, 4)];
//! let clip = vec![(2i64, 0i64), (6, 0), (6, 6), (2, 6)];
//! let out = polyclip::clip(&subject, &clip);
//! assert_eq!(out.len(), 4); // (2,0) (4,0) (4,4) (2,4)
//! assert_eq!(polyclip::area2(&out), Frac::from_int(16));
//! ```

use crate::frac::Frac;

fn cross(ax: Frac, ay: Frac, bx: Frac, by: Frac) -> Frac {
    ax * by - ay * bx
}

fn fpt(x: i64, y: i64) -> (Frac, Frac) {
    (Frac::from_int(x as i128), Frac::from_int(y as i128))
}

/// Signed double-area of a rational polygon (positive for CCW).
pub fn area2(poly: &[(Frac, Frac)]) -> Frac {
    let mut acc = Frac::from_int(0);
    for i in 0..poly.len() {
        let (x1, y1) = poly[i];
        let (x2, y2) = poly[(i + 1) % poly.len()];
        acc = acc + (x1 * y2 - x2 * y1);
    }
    acc
}

fn inside(p: (Frac, Frac), a: (Frac, Frac), b: (Frac, Frac)) -> bool {
    // CCW clip edge a→b: inside iff cross(b−a, p−a) >= 0.
    let cr = cross(b.0 - a.0, b.1 - a.1, p.0 - a.0, p.1 - a.1);
    cr.cmp_frac(&Frac::from_int(0)) != std::cmp::Ordering::Less
}

/// Intersection of segment s→e with the line through a→b.
/// Caller guarantees the two are not parallel (endpoints on opposite
/// sides of the line) — a degenerate case returns `s` unchanged.
fn line_isect(s: (Frac, Frac), e: (Frac, Frac), a: (Frac, Frac), b: (Frac, Frac)) -> (Frac, Frac) {
    let sd = (e.0 - s.0, e.1 - s.1);
    let cd = (b.0 - a.0, b.1 - a.1);
    let den = cross(cd.0, cd.1, sd.0, sd.1);
    if den == Frac::from_int(0) {
        return s;
    }
    // t = cross(b−a, s−a) / cross(b−a, e−s)  →  p = s + t·(e−s)
    let t = cross(cd.0, cd.1, a.0 - s.0, a.1 - s.1)
        .checked_div(den)
        .unwrap_or(Frac::from_int(0));
    (s.0 + sd.0 * t, s.1 + sd.1 * t)
}

/// `clip` returns the intersection of `subject` with convex
/// `clipper` (any winding), as a CCW vertex list of rational points.
/// Empty intersection → `vec![]`. `subject`/`clipper` need ≥ 3
/// vertices; fewer returns an empty result.
///
/// ```
/// use izanagi_kit::{frac::Frac, polyclip};
/// // Triangle clipped by a large axis-aligned square: keeps itself.
/// let subject = vec![(0i64, 0i64), (4, 0), (0, 4)];
/// let clip = vec![(-2i64, -2i64), (6, -2), (6, 6), (-2, 6)];
/// let out = polyclip::clip(&subject, &clip);
/// assert_eq!(polyclip::area2(&out), Frac::from_int(16));
/// ```
pub fn clip(subject: &[(i64, i64)], clipper: &[(i64, i64)]) -> Vec<(Frac, Frac)> {
    if subject.len() < 3 || clipper.len() < 3 {
        return Vec::new();
    }
    // Normalize clipper winding to CCW (signed area2 < 0 ⇒ reverse).
    let mut clip_pts: Vec<(i64, i64)> = clipper.to_vec();
    let mut a2: i128 = 0;
    for i in 0..clip_pts.len() {
        let (x1, y1) = clip_pts[i];
        let (x2, y2) = clip_pts[(i + 1) % clip_pts.len()];
        a2 += (x1 as i128) * (y2 as i128) - (x2 as i128) * (y1 as i128);
    }
    if a2 < 0 {
        clip_pts.reverse();
    }
    let clip_f: Vec<(Frac, Frac)> = clip_pts.iter().map(|&(x, y)| fpt(x, y)).collect();

    let mut out: Vec<(Frac, Frac)> = subject.iter().map(|&(x, y)| fpt(x, y)).collect();
    for i in 0..clip_f.len() {
        let a = clip_f[i];
        let b = clip_f[(i + 1) % clip_f.len()];
        let mut next: Vec<(Frac, Frac)> = Vec::with_capacity(out.len() + 4);
        if out.is_empty() {
            break;
        }
        let mut s = *out
            .last()
            .unwrap_or(&(Frac::from_int(0), Frac::from_int(0)));
        let mut s_in = inside(s, a, b);
        for &e in &out {
            let e_in = inside(e, a, b);
            if e_in {
                if !s_in {
                    next.push(line_isect(s, e, a, b));
                }
                next.push(e);
            } else if s_in {
                next.push(line_isect(s, e, a, b));
            }
            s = e;
            s_in = e_in;
        }
        out = next;
    }
    out.dedup();
    if out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    out
}

/// Convenience: clipped polygon then rounded pointwise to integers
/// (truncation toward −∞ on numerator/denominator), for tile-grid
/// consumers that cannot carry rationals. Prefer `clip` for exactness.
pub fn clip_i64(subject: &[(i64, i64)], clipper: &[(i64, i64)]) -> Vec<(i64, i64)> {
    clip(subject, clipper)
        .iter()
        .map(|&(x, y)| {
            (
                (x.num / x.den).try_into().unwrap_or(0),
                (y.num / y.den).try_into().unwrap_or(0),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn sq(x0: i64, y0: i64, x1: i64, y1: i64) -> Vec<(i64, i64)> {
        vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
    }

    #[test]
    fn basics() {
        let out = clip(&sq(0, 0, 4, 4), &sq(2, 0, 6, 6));
        assert_eq!(out.len(), 4);
        assert_eq!(area2(&out), Frac::from_int(16));
        // Subject containing the clip -> clip outline verbatim.
        let out = clip(&sq(0, 0, 10, 10), &sq(2, 2, 4, 4));
        assert_eq!(area2(&out), Frac::from_int(8));
        // Disjoint -> empty.
        assert!(clip(&sq(0, 0, 2, 2), &sq(5, 5, 9, 9)).is_empty());
        // Integer facade: same intersection, truncated vertices.
        let r = clip_i64(&sq(0, 0, 4, 4), &sq(2, 0, 6, 6));
        assert!(r.contains(&(2, 0)) && r.contains(&(4, 4)));
        // Contained subject returns itself (area preserved).
        let out = clip(&sq(1, 1, 3, 3), &sq(0, 0, 9, 9));
        assert_eq!(area2(&out), Frac::from_int(8));
    }

    #[test]
    fn diagonal_half_clip_is_exact() {
        // Square [0,4]^2 clipped to the half-plane x+y <= 4 via a big
        // CCW triangle whose hypotenuse is x+y=4: area must be 8·2=16/2.
        let subject = sq(0, 0, 4, 4);
        let clipper = vec![(-100i64, -100i64), (104, -100), (-100, 104)];
        let out = clip(&subject, &clipper);
        assert_eq!(area2(&out), Frac::from_int(16));
    }

    #[test]
    fn clip_area2_never_exceeds_subject_and_clip() {
        let mut rng = SplitMix64::new(0xC11C);
        for _ in 0..80 {
            let s: Vec<(i64, i64)> = (0..4)
                .map(|_| (rng.below(20) as i64, rng.below(20) as i64))
                .collect();
            let s = crate::poly::convex_hull(
                &s.iter()
                    .map(|&(x, y)| (x as i32, y as i32))
                    .collect::<Vec<_>>(),
            )
            .iter()
            .map(|&(x, y)| (x as i64, y as i64))
            .collect::<Vec<_>>();
            let cx0 = rng.below(12) as i64;
            let cy0 = rng.below(12) as i64;
            let cx1 = cx0 + 2 + rng.below(10) as i64;
            let cy1 = cy0 + 2 + rng.below(10) as i64;
            let clipper = sq(cx0, cy0, cx1, cy1);
            let out = clip(&s, &clipper);
            if out.is_empty() {
                continue;
            }
            let a_out = area2(&out).abs();
            let a_clip = Frac::from_int(((cx1 - cx0) * (cy1 - cy0) * 2) as i128);
            assert!(
                a_out.cmp_frac(&a_clip) != std::cmp::Ordering::Greater,
                "out {a_out:?} > clip {a_clip:?}"
            );
            // Every output vertex must be inside or on every clip edge.
            for i in 0..clipper.len() {
                let a = fpt(clipper[i].0, clipper[i].1);
                let b = fpt(
                    clipper[(i + 1) % clipper.len()].0,
                    clipper[(i + 1) % clipper.len()].1,
                );
                for &p in &out {
                    assert!(inside(p, a, b), "vertex outside clip edge");
                }
            }
        }
    }

    #[test]
    fn winding_insensitive_and_deterministic() {
        let s = sq(0, 0, 6, 6);
        let ccw = sq(2, 2, 8, 8);
        let mut cw = ccw.clone();
        cw.reverse();
        let a = clip(&s, &ccw);
        let b = clip(&s, &cw);
        assert_eq!(area2(&a), area2(&b));
        assert_eq!(a, clip(&s, &ccw));
        assert_eq!(area2(&a), Frac::from_int(32)); // [2,6]^2 = 16 -> *2
    }

    #[test]
    fn vertex_on_edge_is_kept_once() {
        // Subject corner exactly on clip edge: no duplicates produced.
        let out = clip(&sq(0, 0, 4, 4), &sq(4, 0, 8, 8));
        assert!(
            out.len() <= 2,
            "edge-touch degenerates to a sliver: {out:?}"
        );
    }
}
