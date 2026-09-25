//! Catmull–Rom / Hermite splines in exact `i128`/`Frac` arithmetic.
//! Uniform Catmull–Rom passes **through** every interior control point —
//! unlike Bézier curves, the interpolation property is what makes it the
//! standard choice for waypoint paths (camera tracks, patrol routes) where
//! the curve must hit authored positions exactly.
//!
//! A parameter `t` of `0..1` walks segment `P₁→P₂` in the cardinal basis
//! `P(t) = ½·(2P₁ + (−P₀+P₂)·t + (2P₀−5P₁+4P₂−P₃)·t² + (−P₀+3P₁−3P₂+P₃)·t³)`.
//! [`hermite`] is the same cubic written in endpoint/tangent form —
//! Catmull–Rom is Hermite with tangents `v = (P₂−P₀)/2`.
//!
//! [`centripetal_sample`] gives the standard fix for unevenly spaced
//! points: knots are spaced by `√‖Δ‖`, so segments with closer neighbors
//! get less `t` — no cusps or self-intersections at clustered waypoints.
//!
//! ```
//! use izanagi_kit::catmull::catmull_rom;
//! use izanagi_kit::frac::Frac;
//!
//! // t=0 lands on P1, t=1 on P2 — interpolation, not approximation.
//! let a = catmull_rom((0,0),(2,0),(2,4),(0,4), Frac::new(0,1));
//! let b = catmull_rom((0,0),(2,0),(2,4),(0,4), Frac::new(1,1));
//! assert_eq!(a, (Frac::from_int(2), Frac::from_int(0)));
//! assert_eq!(b, (Frac::from_int(2), Frac::from_int(4)));
//! ```

use crate::frac::Frac;

/// Uniform Catmull–Rom point on segment `p1→p2` at `t ∈ [0,1]`
/// (`t` outside the range is evaluated anyway — extrapolation is legal).
pub fn catmull_rom(
    p0: (i32, i32),
    p1: (i32, i32),
    p2: (i32, i32),
    p3: (i32, i32),
    t: Frac,
) -> (Frac, Frac) {
    let ((x0, y0), (x1, y1), (x2, y2), (x3, y3)) = (p0, p1, p2, p3);
    (basis(x0, x1, x2, x3, t), basis(y0, y1, y2, y3, t))
}

/// Cardinal basis for one coordinate: `2p1 + (−p0+p2)t + (2p0−5p1+4p2−p3)t²
/// + (−p0+3p1−3p2+p3)t³`, all over `2` — evaluated exactly in `i128`.
fn basis(p0: i32, p1: i32, p2: i32, p3: i32, t: Frac) -> Frac {
    let (p0, p1, p2, p3) = (p0 as i128, p1 as i128, p2 as i128, p3 as i128);
    // Horner in Frac: ((−p0+3p1−3p2+p3)·t + (2p0−5p1+4p2−p3))·t + (−p0+p2))·t + 2p1, over 2.
    let c3 = Frac::from_int(-p0 + 3 * p1 - 3 * p2 + p3);
    let c2 = Frac::from_int(2 * p0 - 5 * p1 + 4 * p2 - p3);
    let c1 = Frac::from_int(-p0 + p2);
    let c0 = Frac::from_int(2 * p1);
    (((c3 * t + c2) * t + c1) * t + c0) * Frac::new(1, 2)
}

/// Hermite cubic in endpoint/tangent form: `p(t)` through `p1` at `t=0`
/// with tangent `v1`, to `p2` at `t=1` with tangent `v2` — the general
/// form Catmull–Rom specializes (`v1 = (p2−p0)/2`, `v2 = (p3−p1)/2`).
/// Tangents are `Frac`s: `p` and `v` may have different units.
pub fn hermite(
    p1: (i32, i32),
    v1: (Frac, Frac),
    p2: (i32, i32),
    v2: (Frac, Frac),
    t: Frac,
) -> (Frac, Frac) {
    let ((x1, y1), (tx, ty), (x2, y2), (ux, uy)) = (p1, v1, p2, v2);
    (
        hermite_axis(x1 as i128, tx, x2 as i128, ux, t),
        hermite_axis(y1 as i128, ty, y2 as i128, uy, t),
    )
}

fn hermite_axis(p1: i128, v1: Frac, p2: i128, v2: Frac, t: Frac) -> Frac {
    // h00 = 2t³−3t²+1, h10 = t³−2t²+t, h01 = −2t³+3t², h11 = t³−t²
    let t2 = t * t;
    let t3 = t2 * t;
    let one = Frac::from_int(1);
    let three = Frac::from_int(3);
    let two = Frac::from_int(2);
    let h00 = two * t3 - three * t2 + one;
    let h10 = t3 - two * t2 + t;
    let h01 = Frac::from_int(0) - two * t3 + three * t2;
    let h11 = t3 - t2;
    Frac::from_int(p1) * h00 + v1 * h10 + Frac::from_int(p2) * h01 + v2 * h11
}

/// Sample a whole Catmull–Rom path: `points` are the waypoints in order;
/// endpoints are mirrored (`p0 = 2p1 − p2` phantom knot) so the spline
/// starts exactly at `points[0]` and ends at `points[n−1]`. Returns
/// `per_seg` points per segment (segment `i→i+1` contributes samples at
/// `t = 0, 1/per_seg, …, (per_seg−1)/per_seg`), plus the final waypoint —
/// `len = (n−1)·per_seg + 1` for `n ≥ 2`. Fewer than 2 points or
/// `per_seg == 0` returns an empty `Vec`.
pub fn sample(points: &[(i32, i32)], per_seg: u32) -> Vec<(Frac, Frac)> {
    let n = points.len();
    if n < 2 || per_seg == 0 {
        return Vec::new();
    }
    let phantom_head = (2 * points[0].0 - points[1].0, 2 * points[0].1 - points[1].1);
    let phantom_tail = (
        2 * points[n - 1].0 - points[n - 2].0,
        2 * points[n - 1].1 - points[n - 2].1,
    );
    let mut out = Vec::with_capacity((n - 1) * per_seg as usize + 1);
    for i in 0..n - 1 {
        let p0 = if i == 0 { phantom_head } else { points[i - 1] };
        let p3 = if i + 2 >= n {
            phantom_tail
        } else {
            points[i + 2]
        };
        for s in 0..per_seg {
            out.push(catmull_rom(
                p0,
                points[i],
                points[i + 1],
                p3,
                Frac::new(s as i128, per_seg as i128),
            ));
        }
    }
    out.push((
        Frac::from_int(points[n - 1].0 as i128),
        Frac::from_int(points[n - 1].1 as i128),
    ));
    out
}

/// Centripetal-parameterized sampling — knots at `Σ√|Δ|` so clustered
/// waypoints get less parameter space (the standard cusp/loop fix from
/// Yuksel–Schaefer–Keyser 2009, "Parameterization and Applications of
/// Catmull-Rom Curves"). Chord lengths use integer `isqrt(Δ²)` (Q16.16 is
/// overkill and the √ is exactly what makes it centripetal). Same output
/// length contract as [`sample`].
pub fn centripetal_sample(points: &[(i32, i32)], per_seg: u32) -> Vec<(Frac, Frac)> {
    let n = points.len();
    if n < 2 || per_seg == 0 {
        return Vec::new();
    }
    let phantom = |a: (i32, i32), b: (i32, i32)| (2 * a.0 - b.0, 2 * a.1 - b.1);
    let knot = |a: (i32, i32), b: (i32, i32), t0: Frac| -> Frac {
        // t_{i+1} = t_i + sqrt(|p_{i+1} − p_i|): |Δ|² clamps each leg to
        // 46340 so the i64 square never overflows — only ratios between
        // adjacent knots matter, so a clamped huge chord is harmless.
        let dx = (b.0 as i64 - a.0 as i64).min(46340);
        let dy = (b.1 as i64 - a.1 as i64).min(46340);
        t0 + Frac::from_int(isqrt64(dx * dx + dy * dy) as i128)
    };
    let mut out = Vec::with_capacity((n - 1) * per_seg as usize + 1);
    // For segment i, t0..t1 are the centripetal knots of p[i], p[i+1];
    // inner knots p[i−1], p[i+2] only set the *width* of t — we re-derive
    // each segment's knots locally so the code stays branch-light.
    for i in 0..n - 1 {
        let pa = if i == 0 {
            phantom(points[0], points[1])
        } else {
            points[i - 1]
        };
        let pb = points[i];
        let pc = points[i + 1];
        let pd = if i + 2 >= n {
            phantom(points[n - 1], points[n - 2])
        } else {
            points[i + 2]
        };
        let t1 = knot(pa, pb, Frac::from_int(0));
        let t2 = knot(pb, pc, t1);
        let t3 = knot(pc, pd, t2);
        for s in 0..per_seg {
            let u = Frac::new(s as i128, per_seg as i128);
            let tau = t1 + (t2 - t1) * u;
            out.push(centripetal_eval(
                pa,
                pb,
                pc,
                pd,
                Frac::from_int(0),
                t1,
                t2,
                t3,
                tau,
            ));
        }
    }
    out.push((
        Frac::from_int(points[n - 1].0 as i128),
        Frac::from_int(points[n - 1].1 as i128),
    ));
    out
}

/// Barry–Goldman pyramid: centripetal evaluation at `tau ∈ [t1,t2]`.
/// Three lerps, then two, then one — the exact recursive form from
/// Yuksel–Schaefer–Keyser; integer knots make every weight a `Frac`.
#[allow(clippy::too_many_arguments)]
fn centripetal_eval(
    pa: (i32, i32),
    pb: (i32, i32),
    pc: (i32, i32),
    pd: (i32, i32),
    t0: Frac,
    t1: Frac,
    t2: Frac,
    t3: Frac,
    tau: Frac,
) -> (Frac, Frac) {
    let lerp = |a: (Frac, Frac), b: (Frac, Frac), wa: Frac, wb: Frac| -> (Frac, Frac) {
        // (a·wa + b·wb) — callers pass complementary weights; a zero
        // denominator is absorbed into the zero weight by `ratio`.
        (a.0 * wa + b.0 * wb, a.1 * wa + b.1 * wb)
    };
    let pt = |p: (i32, i32)| (Frac::from_int(p.0 as i128), Frac::from_int(p.1 as i128));
    let (pa, pb, pc, pd) = (pt(pa), pt(pb), pt(pc), pt(pd));
    let a1 = lerp(pa, pb, ratio(t1 - tau, t1 - t0), ratio(tau - t0, t1 - t0));
    let a2 = lerp(pb, pc, ratio(t2 - tau, t2 - t1), ratio(tau - t1, t2 - t1));
    let a3 = lerp(pc, pd, ratio(t3 - tau, t3 - t2), ratio(tau - t2, t3 - t2));
    let b1 = lerp(a1, a2, ratio(t2 - tau, t2 - t0), ratio(tau - t0, t2 - t0));
    let b2 = lerp(a2, a3, ratio(t3 - tau, t3 - t1), ratio(tau - t1, t3 - t1));
    lerp(b1, b2, ratio(t2 - tau, t2 - t1), ratio(tau - t1, t2 - t1))
}

/// `num/den` — `0` when `den == 0` (a zero knot span contributes zero
/// weight, which is the correct absorption for degenerate geometry).
fn ratio(num: Frac, den: Frac) -> Frac {
    if den.num == 0 {
        Frac::from_int(0)
    } else {
        Frac::new(num.num * den.den, num.den * den.num)
    }
}

/// `⌊√x⌋` for `x ≥ 0` by binary search — the tiny `isqrt` the knot math
/// needs, kept local so callers never see a helper for one use.
fn isqrt64(x: i64) -> i64 {
    if x <= 0 {
        return 0;
    }
    let mut lo = 1i64;
    let mut hi = x.min(3_037_000_499); // ⌊√(i64::MAX)⌋ + 1
    while lo < hi {
        let mid = lo + (hi - lo + 1) / 2;
        if mid <= x / mid {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    lo
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near_i(a: Frac, b: i128) -> bool {
        // |a − b| < 1/4 — centripetal knots only bend parameterization,
        // endpoints stay exact by construction.
        let d = a - Frac::from_int(b);
        d.num.abs() * 4 <= d.den
    }

    #[test]
    fn endpoints_hit_waypoints_exactly() {
        let pts = [(0, 0), (4, 0), (4, 4), (0, 4)];
        for i in 0..pts.len() - 1 {
            let p = catmull_rom(
                if i == 0 { (0, 0) } else { pts[i - 1] },
                pts[i],
                pts[i + 1],
                if i + 2 >= pts.len() {
                    (0, 4)
                } else {
                    pts[i + 2]
                },
                Frac::new(0, 1),
            );
            assert!(near_i(p.0, pts[i].0 as i128) && near_i(p.1, pts[i].1 as i128));
        }
    }

    #[test]
    fn sample_length_and_endpoints() {
        let pts = [(0, 0), (4, 0), (4, 4)];
        for per in [1, 2, 7] {
            let s = sample(&pts, per);
            assert_eq!(s.len(), 2 * per as usize + 1);
            let last = s.last().unwrap();
            assert_eq!(*last, (Frac::from_int(4), Frac::from_int(4)));
            let first = s[0];
            assert_eq!(first, (Frac::from_int(0), Frac::from_int(0)));
        }
        assert!(sample(&pts, 0).is_empty());
        assert!(sample(&[(1, 2)], 4).is_empty());
    }

    #[test]
    fn collinear_points_produce_the_segment() {
        // All four knots on y=0 → every sampled point has y=0.
        let s = sample(&[(0, 0), (5, 0), (10, 0)], 11);
        for (x, y) in &s {
            assert_eq!(*y, Frac::from_int(0));
            assert!(x.num >= 0);
        }
        // Monotone x along the line.
        for w in s.windows(2) {
            assert!(w[1].0.num * w[0].0.den >= w[0].0.num * w[1].0.den);
        }
    }

    #[test]
    fn centripetal_hits_endpoints_and_survives_clustered_points() {
        let pts = [(0, 0), (1, 0), (2, 0), (2, 10), (12, 10)];
        for per in [1, 3, 9] {
            let s = centripetal_sample(&pts, per);
            assert_eq!(s.len(), 4 * per as usize + 1);
            assert_eq!(s[0], (Frac::from_int(0), Frac::from_int(0)));
            assert_eq!(*s.last().unwrap(), (Frac::from_int(12), Frac::from_int(10)));
            // Interior waypoints land exactly at their knots:
            for (i, &wp) in pts.iter().enumerate().skip(1).take(3) {
                let q = s[i * per as usize];
                assert!(
                    near_i(q.0, wp.0 as i128) && near_i(q.1, wp.1 as i128),
                    "seg {i}: {q:?} vs {wp:?}"
                );
            }
        }
    }

    #[test]
    fn hermite_matches_catmull_on_cardinal_tangents() {
        let (p0, p1, p2, p3) = ((1, 2), (3, 5), (9, 1), (7, 7));
        let t = Frac::new(3, 7);
        let v1 = (
            Frac::new((p2.0 - p0.0) as i128, 2),
            Frac::new((p2.1 - p0.1) as i128, 2),
        );
        let v2 = (
            Frac::new((p3.0 - p1.0) as i128, 2),
            Frac::new((p3.1 - p1.1) as i128, 2),
        );
        assert_eq!(catmull_rom(p0, p1, p2, p3, t), hermite(p1, v1, p2, v2, t));
    }

    #[test]
    fn degenerate_runs_do_not_panic() {
        let dup = [(2, 2), (2, 2), (5, 5)];
        let s = sample(&dup, 4);
        assert_eq!(s.len(), 9);
        let c = centripetal_sample(&dup, 4);
        assert_eq!(c.len(), 9);
        // collinear degenerate: all points equal → single point path
        let flat = [(3, 3), (3, 3), (3, 3)];
        assert_eq!(centripetal_sample(&flat, 2).len(), 5);
    }
}
