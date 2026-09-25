//! Signed distance fields in 2D over [`Fixed`] — primitives (circle, box,
//! segment) and composition ops (union, intersection, subtraction,
//! polynomial smooth-min) plus a sphere-tracing raymarcher, following Íñigo
//! Quilez's reference formulas. The continuous-shape side of the `geometry`
//! family's discrete grids.
//!
//! ```
//! use izanagi_kit::{sdf, fixed::Fixed, vec::Vec2};
//! let c = sdf::circle(Fixed::from_int(2));
//! assert_eq!(c(Vec2::new(Fixed::from_int(5), Fixed::ZERO)), Fixed::from_int(3));
//! ```

use crate::fixed::Fixed;
use crate::vec::Vec2;

/// A 2D signed distance function: negative inside, zero on the boundary.
pub type Sdf = fn(Vec2) -> Fixed;

/// `|p| - r` — signed distance to a circle of radius `r` centered at origin.
pub fn circle(r: Fixed) -> impl Fn(Vec2) -> Fixed {
    move |p| p.len() - r
}

/// Signed distance to an axis-aligned box of half-extent `b` centered at
/// origin: `|max(|p| − b, 0)| + min(max(qx, qy), 0)` (IQ's standard form).
pub fn rect(bx: Fixed, by: Fixed) -> impl Fn(Vec2) -> Fixed {
    move |p| {
        let qx = p.x.abs() - bx;
        let qy = p.y.abs() - by;
        let ox = qx.max(Fixed::ZERO);
        let oy = qy.max(Fixed::ZERO);
        Vec2::new(ox, oy).len() + qx.max(qy).min(Fixed::ZERO)
    }
}

/// Signed distance to segment `a..b` (IQ's projection-clamped form).
pub fn segment(a: Vec2, b: Vec2) -> impl Fn(Vec2) -> Fixed {
    move |p| {
        let pa = p - a;
        let ba = b - a;
        let denom = ba.dot(ba);
        let h = if denom.raw() <= 0 {
            Fixed::ZERO
        } else {
            (pa.dot(ba).div(denom)).clamp(Fixed::ZERO, Fixed::ONE)
        };
        (pa - ba.scale(h)).len()
    }
}

/// `min(da, db)` — hard union of two fields.
pub fn union<A: Fn(Vec2) -> Fixed, B: Fn(Vec2) -> Fixed>(a: A, b: B) -> impl Fn(Vec2) -> Fixed {
    move |p| a(p).min(b(p))
}

/// `max(da, db)` — intersection.
pub fn intersect<A: Fn(Vec2) -> Fixed, B: Fn(Vec2) -> Fixed>(a: A, b: B) -> impl Fn(Vec2) -> Fixed {
    move |p| a(p).max(b(p))
}

/// `max(da, −db)` — subtract `b` from `a`.
pub fn subtract<A: Fn(Vec2) -> Fixed, B: Fn(Vec2) -> Fixed>(a: A, b: B) -> impl Fn(Vec2) -> Fixed {
    move |p| a(p).max(Fixed::ZERO - b(p))
}

/// Polynomial smooth-min (IQ's `smin`): blends `a` and `b` within band `k`.
pub fn smin(a: Fixed, b: Fixed, k: Fixed) -> Fixed {
    if k.raw() <= 0 {
        return a.min(b);
    }
    let h = (k - (a - b).abs()).max(Fixed::ZERO).div(k);
    a.min(b) - h.mul(h).mul(k).div(Fixed::from_int(4))
}

/// `smin(da, db, k)` — smooth union of two fields.
pub fn smooth_union<A: Fn(Vec2) -> Fixed, B: Fn(Vec2) -> Fixed>(
    a: A,
    b: B,
    k: Fixed,
) -> impl Fn(Vec2) -> Fixed {
    move |p| smin(a(p), b(p), k)
}

/// One sphere-tracing step returned by [`march`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum March {
    /// Ray hit the surface: the clamped travel distance `t`.
    Hit(Fixed),
    /// Ray left without hitting: traveled `max_dist` or escaped `eps` checks
    /// after `steps` iterations.
    Miss,
}

/// Sphere-trace `sdf` from `origin` along unit-ish `dir` for up to `steps`
/// iterations or `max_dist` travel; `eps` is the hit threshold. With `dir`
/// unnormalized the march still terminates but `t` is in `dir` units.
pub fn march<S: Fn(Vec2) -> Fixed>(
    sdf: S,
    origin: Vec2,
    dir: Vec2,
    max_dist: Fixed,
    eps: Fixed,
    steps: u32,
) -> March {
    let mut t = Fixed::ZERO;
    for _ in 0..steps {
        let d = sdf(origin + dir.scale(t));
        if d <= eps {
            return March::Hit(t);
        }
        t = t + d;
        if t >= max_dist {
            return March::Miss;
        }
    }
    March::Miss
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fi(n: i32) -> Fixed {
        Fixed::from_int(n)
    }
    fn fr(n: i32, d: i32) -> Fixed {
        Fixed::from_ratio(n, d)
    }
    fn v(x: Fixed, y: Fixed) -> Vec2 {
        Vec2::new(x, y)
    }
    fn close(a: Fixed, want: f64, eps: f64) {
        let got = a.raw() as f64 / 65536.0;
        assert!((got - want).abs() <= eps, "got {got} want {want}");
    }

    #[test]
    fn circle_inside_outside_boundary() {
        let c = circle(fi(2));
        assert!(c(v(Fixed::ZERO, Fixed::ZERO)) < Fixed::ZERO);
        assert_eq!(c(v(fi(2), Fixed::ZERO)), Fixed::ZERO);
        assert_eq!(c(v(fi(5), Fixed::ZERO)), fi(3));
        close(c(v(fi(3), fi(4))), 3.0, 0.01); // |(3,4)|=5 → 5-2
    }

    #[test]
    fn rect_corners_edges_inside() {
        let r = rect(fi(2), fi(1));
        assert!(r(v(Fixed::ZERO, Fixed::ZERO)) < Fixed::ZERO);
        assert_eq!(r(v(fi(2), Fixed::ZERO)), Fixed::ZERO);
        assert_eq!(r(v(fi(3), Fixed::ZERO)), fi(1));
        // Corner distance to (3,2): q=(1,1) → √2.
        close(r(v(fi(3), fi(2))), std::f64::consts::SQRT_2, 0.01);
    }

    #[test]
    fn segment_endpoints_and_projection() {
        let s = segment(v(Fixed::ZERO, Fixed::ZERO), v(fi(4), Fixed::ZERO));
        // Above midpoint → distance 1.
        assert_eq!(s(v(fi(2), fi(1))), fi(1));
        // Past the end → distance to endpoint (4,0).
        assert_eq!(s(v(fi(6), Fixed::ZERO)), fi(2));
        // Degenerate segment is a point.
        let p = segment(v(fi(1), fi(1)), v(fi(1), fi(1)));
        assert_eq!(p(v(fi(1), fi(2))), fi(1));
    }

    #[test]
    fn boolean_ops() {
        let u = union(circle(fi(1)), rect(fi(1), fi(1)));
        assert!(u(v(fi(1), Fixed::ZERO)) <= Fixed::ZERO);
        let i = intersect(circle(fi(2)), rect(fi(1), fi(1)));
        // (1.5,0): inside circle(2) (−0.5) but outside rect(1) (0.5) → 0.5.
        assert!(i(v(fr(3, 2), Fixed::ZERO)) > Fixed::ZERO);
        let s = subtract(circle(fi(2)), circle(fi(1)));
        // (0.5,0): inside big (−1.5), inside small (−0.5) → max(−1.5, 0.5) = 0.5.
        assert!(s(v(fr(1, 2), Fixed::ZERO)) > Fixed::ZERO);
    }

    #[test]
    fn smin_blends() {
        // smin(1,1,k) = 1 - k/4 (both distances equal → dip by k/4).
        assert_eq!(smin(fi(1), fi(1), fi(1)), fi(1) - fr(1, 4));
        // Beyond the band it's plain min.
        assert_eq!(smin(fi(1), fi(10), fi(1)), fi(1));
        // k=0 → exact min.
        assert_eq!(smin(fi(1), fi(2), Fixed::ZERO), fi(1));
        // A disc and a half-plane 0.75 apart: inside the blend band the
        // smooth union dips below the hard min.
        let su = smooth_union(circle(fr(1, 2)), |p: Vec2| p.x - fr(3, 4), fi(1));
        let hard = union(circle(fr(1, 2)), |p: Vec2| p.x - fr(3, 4));
        let probe = v(fr(3, 5), Fixed::ZERO);
        assert!(su(probe) < hard(probe));
    }

    #[test]
    fn march_hits_and_misses() {
        // Ray from (-5,0) toward +x hits circle boundary at t≈4.
        let hit = march(
            circle(fi(1)),
            v(fi(-5), Fixed::ZERO),
            v(fi(1), Fixed::ZERO),
            fi(20),
            fr(1, 100),
            100,
        );
        match hit {
            March::Hit(t) => close(t, 4.0, 0.02),
            March::Miss => panic!("expected hit"),
        }
        // Ray pointing away misses.
        assert_eq!(
            march(
                circle(fi(1)),
                v(fi(-5), Fixed::ZERO),
                v(fi(-1), Fixed::ZERO),
                fi(20),
                fr(1, 100),
                100
            ),
            March::Miss
        );
        // A 1-iteration cap cannot converge → Miss.
        assert_eq!(
            march(
                circle(fi(1)),
                v(fi(-5), Fixed::ZERO),
                v(fi(1), Fixed::ZERO),
                fi(20),
                fr(1, 100),
                1
            ),
            March::Miss
        );
    }
}
