//! Analytic ray queries on `Fixed`/`Vec2` — the continuous-space
//! counterpart to `geometry`'s grid DDA: *where* does a ray first hit a
//! shape, in `t` units along `origin + t·dir`. All queries return the
//! smallest non-negative `t`, never a hit behind the origin.
//!
//! ```
//! use izanagi_kit::fixed::Fixed;
//! use izanagi_kit::ray::hit_aabb;
//! use izanagi_kit::vec::Vec2;
//!
//! let o = Vec2::new(Fixed::ZERO, Fixed::ZERO);
//! let d = Vec2::new(Fixed::ONE, Fixed::ZERO); // +x
//! let t = hit_aabb(
//!     o, d,
//!     Vec2::new(Fixed::from_int(2), Fixed::from_int(-1)),
//!     Vec2::new(Fixed::from_int(4), Fixed::from_int(1)),
//! ).unwrap();
//! assert_eq!(t, Fixed::from_int(2));
//! ```

use crate::fixed::Fixed;
use crate::vec::Vec2;

/// Point along the ray: `origin + dir·t`. No normalization — `dir`
/// carries its own scale, so `t` is in "dir lengths".
pub fn at(origin: Vec2, dir: Vec2, t: Fixed) -> Vec2 {
    origin + dir.scale(t)
}

/// Ray vs. axis-aligned box (slab method): the smallest `t ≥ 0` where
/// the ray enters `[min, max]`, or `None` on a miss. An origin inside
/// the box returns `t = 0`. A degenerate box (`min > max` on any axis)
/// misses.
pub fn hit_aabb(origin: Vec2, dir: Vec2, min: Vec2, max: Vec2) -> Option<Fixed> {
    if min.x > max.x || min.y > max.y {
        return None;
    }
    let mut t_lo = Fixed::MIN;
    let mut t_hi = Fixed::MAX;
    for (o, d, lo, hi) in [
        (origin.x, dir.x, min.x, max.x),
        (origin.y, dir.y, min.y, max.y),
    ] {
        if d.is_zero() {
            // Parallel to this slab: miss unless already inside.
            if o < lo || o > hi {
                return None;
            }
            continue;
        }
        let mut t1 = (lo - o).div(d);
        let mut t2 = (hi - o).div(d);
        if t1 > t2 {
            std::mem::swap(&mut t1, &mut t2);
        }
        if t1 > t_lo {
            t_lo = t1;
        }
        if t2 < t_hi {
            t_hi = t2;
        }
        if t_lo > t_hi {
            return None;
        }
    }
    if t_hi < Fixed::ZERO {
        return None; // box is entirely behind the ray
    }
    Some(t_lo.max(Fixed::ZERO))
}

/// Ray vs. circle: smallest `t ≥ 0` entering the disc, `None` on a miss.
/// Solving `|o + t·d − c|² = r²` gives `t = (−b ± √D)/a` with
/// `a = d·d`, `b = (o−c)·d`, `D = b² − a·(oc·oc − r²)` — the
/// half-coefficient form, so `Fixed::sqrt` sees a smaller radicand.
pub fn hit_circle(origin: Vec2, dir: Vec2, center: Vec2, r: Fixed) -> Option<Fixed> {
    if r < Fixed::ZERO || dir.is_zero() {
        return None;
    }
    let oc = origin - center;
    let a = dir.dot(dir);
    let b = oc.dot(dir);
    let d = b.mul(b) - a.mul(oc.dot(oc) - r.mul(r));
    if d < Fixed::ZERO {
        return None;
    }
    let sqrt_d = d.sqrt();
    // Nearest intersection first; if it's behind us but the far one is
    // ahead, the origin is inside the circle.
    let t_near = (Fixed::ZERO - b - sqrt_d).div(a);
    let t_far = (Fixed::ZERO - b + sqrt_d).div(a);
    if t_far < Fixed::ZERO {
        return None;
    }
    Some(t_near.max(Fixed::ZERO))
}

/// Ray vs. segment `a→b`: the `t ≥ 0` of the hit point along the ray,
/// `None` when parallel or the crossing falls outside `[a, b]`.
/// `t = (a−o)×e / d×e`, `u = (a−o)×d / d×e` with `e = b−a`.
pub fn hit_segment(origin: Vec2, dir: Vec2, a: Vec2, b: Vec2) -> Option<Fixed> {
    let e = b - a;
    let denom = dir.cross_2d(e);
    if denom.is_zero() {
        return None; // parallel — coincident or disjoint, no single t
    }
    let ao = a - origin;
    let t = ao.cross_2d(e).div(denom);
    let u = ao.cross_2d(dir).div(denom);
    if t < Fixed::ZERO || u < Fixed::ZERO || u > Fixed::ONE {
        return None;
    }
    Some(t)
}

/// Ray vs. polygon: smallest `t ≥ 0` over every edge. `None` for fewer
/// than 3 vertices or a complete miss. (A ray starting *inside* reports
/// the exit edge — the smallest positive `t`.)
pub fn hit_polygon(origin: Vec2, dir: Vec2, poly: &[Vec2]) -> Option<Fixed> {
    if poly.len() < 3 {
        return None;
    }
    let mut best: Option<Fixed> = None;
    for i in 0..poly.len() {
        let t = hit_segment(origin, dir, poly[i], poly[(i + 1) % poly.len()]);
        if let Some(tt) = t {
            if best.map_or(true, |b| tt < b) {
                best = Some(tt);
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(v: i32) -> Fixed {
        Fixed::from_int(v)
    }
    fn v(x: i32, y: i32) -> Vec2 {
        Vec2::new(f(x), f(y))
    }

    #[test]
    fn aabb_entry_exit_and_inside() {
        let o = v(0, 0);
        let right = v(1, 0);
        let min = v(2, -1);
        let max = v(4, 1);
        assert_eq!(hit_aabb(o, right, min, max), Some(f(2)));
        // Inside: t = 0.
        assert_eq!(hit_aabb(v(3, 0), right, min, max), Some(Fixed::ZERO));
        // Box behind: miss.
        assert_eq!(hit_aabb(v(5, 0), right, min, max), None);
        // Parallel outside the y-slab: miss.
        assert_eq!(hit_aabb(v(0, 3), right, min, max), None);
        // Aim +y at a wall face.
        assert_eq!(hit_aabb(v(3, -5), v(0, 1), min, max), Some(f(4)));
    }

    #[test]
    fn circle_tangent_and_inside() {
        let c = v(5, 0);
        let o = v(0, 0);
        let right = v(1, 0);
        // r=2: near hit at t=3, far at t=7.
        assert_eq!(hit_circle(o, right, c, f(2)), Some(f(3)));
        // Inside the circle: exit at t=7.
        assert_eq!(hit_circle(v(5, 0), right, c, f(2)), Some(Fixed::ZERO));
        // Tangent at (5,2): t=5 exactly (D=0).
        assert_eq!(hit_circle(v(0, 2), right, c, f(2)), Some(f(5)));
        // Miss above: r=1, ray at y=2 vs circle center y=0.
        assert_eq!(hit_circle(v(0, 2), right, c, Fixed::ONE), None);
    }

    #[test]
    fn segment_crossing_parameterizes_both() {
        // +x ray crosses vertical segment x=3, y∈[−1,4] at t=3.
        let t = hit_segment(v(0, 0), v(1, 0), v(3, -1), v(3, 4)).unwrap();
        assert_eq!(t, f(3));
        // Misses past the segment end (dir (1,2) crosses x=3 at y=6 > 4).
        assert_eq!(hit_segment(v(0, 0), v(1, 2), v(3, -1), v(3, 4)), None);
        // Parallel ray along the segment's own line: None (documented).
        assert_eq!(hit_segment(v(0, 0), v(0, 1), v(0, 2), v(0, 5)), None);
        // Segment behind the ray: crossing at t<0 → None.
        assert_eq!(hit_segment(v(0, 0), v(1, 0), v(-3, -1), v(-3, 4)), None);
    }

    #[test]
    fn polygon_reports_nearest_edge() {
        // Square (0,0)–(4,0)–(4,4)–(0,4); ray from (−2,2) +x enters at t=2.
        let sq = [v(0, 0), v(4, 0), v(4, 4), v(0, 4)];
        assert_eq!(hit_polygon(v(-2, 2), v(1, 0), &sq), Some(f(2)));
        // From inside, reports the exit edge at t=2 (+x face at x=4).
        assert_eq!(hit_polygon(v(2, 2), v(1, 0), &sq), Some(f(2)));
        // Miss: ray at y=6 never touches.
        assert_eq!(hit_polygon(v(-2, 6), v(1, 0), &sq), None);
        assert_eq!(hit_polygon(v(0, 0), v(1, 0), &sq[..2]), None);
    }

    #[test]
    fn deterministic_twice_and_at() {
        let (o, d) = (v(1, 1), v(2, 3));
        assert_eq!(
            hit_aabb(o, d, v(5, 5), v(9, 9)),
            hit_aabb(o, d, v(5, 5), v(9, 9))
        );
        let p = at(o, d, f(2));
        assert_eq!(p, v(5, 7));
    }
}
