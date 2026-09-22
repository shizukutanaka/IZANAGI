//! Integer-exact parametric curves — cubic Bézier and Catmull-Rom
//! evaluated at a *rational* `t = num/den`, so every coordinate is an
//! exact `i128` fraction rather than a float approximation. Camera
//! paths, projectile arcs, and tween-driven patrol routes stay
//! bit-identical across machines because `(1−t)³P₀ + …` is computed
//! in integers and reduced by the common gcd.
//!
//! ```
//! use izanagi_kit::bezier::cubic_pos;
//! // Horizontal cubic; at t = 1/2 the exact midpoint.
//! let p = [(0, 0), (10, 0), (20, 0), (30, 0)];
//! assert_eq!(cubic_pos(&p, 1, 2), Some((15, 0, 1)));
//! ```

/// `(x_num, y_num, den)` — a 2-D point as exact reduced rationals
/// sharing one denominator.
pub type Rat2 = (i128, i128, i128);

fn gcd3(a: i128, b: i128, c: i128) -> i128 {
    fn g(a: i128, b: i128) -> i128 {
        let (mut a, mut b) = (a.unsigned_abs(), b.unsigned_abs());
        while b > 0 {
            let t = b;
            b = a % b;
            a = t;
        }
        a as i128
    }
    g(g(a, b), c).max(1)
}

fn bez4(c0: i128, c1: i128, c2: i128, c3: i128, t: i128, d: i128) -> i128 {
    // (1-t)³c0 + 3(1-t)²t c1 + 3(1-t)t² c2 + t³ c3, numerator over d³.
    let u = d - t;
    u * u * u * c0 + 3 * u * u * t * c1 + 3 * u * t * t * c2 + t * t * t * c3
}

/// Cubic Bézier at `t = num/den ∈ [0, 1]`, exact. `None` when
/// `den == 0` or `num > den`.
pub fn cubic_pos(p: &[(i64, i64); 4], num: u64, den: u64) -> Option<Rat2> {
    if den == 0 || num > den {
        return None;
    }
    let (t, d) = (num as i128, den as i128);
    let xn = bez4(
        p[0].0 as i128,
        p[1].0 as i128,
        p[2].0 as i128,
        p[3].0 as i128,
        t,
        d,
    );
    let yn = bez4(
        p[0].1 as i128,
        p[1].1 as i128,
        p[2].1 as i128,
        p[3].1 as i128,
        t,
        d,
    );
    let den3 = d * d * d;
    let g = gcd3(xn, yn, den3);
    Some((xn / g, yn / g, den3 / g))
}

/// Catmull-Rom through `pts[1]→pts[2]` at `t = num/den` — the spline
/// interpolates both inner knots exactly.
pub fn catmull_pos(pts: &[(i64, i64); 4], num: u64, den: u64) -> Option<Rat2> {
    if den == 0 || num > den {
        return None;
    }
    let (t, d) = (num as i128, den as i128);
    // 2·CR(t) = 2P1 + (−P0+P2)t + (2P0−5P1+4P2−P3)t² + (−P0+3P1−3P2+P3)t³
    let coord = |p0: i64, p1: i64, p2: i64, p3: i64| -> i128 {
        let (p0, p1, p2, p3) = (p0 as i128, p1 as i128, p2 as i128, p3 as i128);
        let c0 = 2 * p1;
        let c1 = -p0 + p2;
        let c2 = 2 * p0 - 5 * p1 + 4 * p2 - p3;
        let c3 = -p0 + 3 * p1 - 3 * p2 + p3;
        c0 * d * d * d + c1 * t * d * d + c2 * t * t * d + c3 * t * t * t
    };
    let xn = coord(pts[0].0, pts[1].0, pts[2].0, pts[3].0);
    let yn = coord(pts[0].1, pts[1].1, pts[2].1, pts[3].1);
    let den3 = 2 * d * d * d;
    let g = gcd3(xn, yn, den3);
    Some((xn / g, yn / g, den3 / g))
}

/// Polyline approximation of a cubic at `segs` equal steps —
/// `segs + 1` exact samples (both endpoints included). `segs == 0`
/// yields an empty vec.
pub fn flatten_cubic(p: &[(i64, i64); 4], segs: u32) -> Vec<Rat2> {
    if segs == 0 {
        return Vec::new();
    }
    (0..=segs as u64)
        .filter_map(|i| cubic_pos(p, i, segs as u64))
        .collect()
}

/// End-clamped Catmull-Rom through every span of `points`
/// (`points` repeated at both ends), `segs_per_span` samples each —
/// the canonical deterministic camera-path polyline.
pub fn flatten_catmull(points: &[(i64, i64)], segs_per_span: u32) -> Vec<Rat2> {
    let n = points.len();
    if n < 2 || segs_per_span == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for s in 0..n - 1 {
        let p = [
            points[s.saturating_sub(1)],
            points[s],
            points[s + 1],
            points[(s + 2).min(n - 1)],
        ];
        for i in 0..segs_per_span as u64 {
            if let Some(r) = catmull_pos(&p, i, segs_per_span as u64) {
                out.push(r);
            }
        }
    }
    out.push((points[n - 1].0 as i128, points[n - 1].1 as i128, 1));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Horner-form evaluation of the Bernstein polynomial — an
    /// independent code path from the u=1−t expansion in `bez4`.
    fn horner(p: &[(i64, i64); 4], k: usize, t: i128, d: i128) -> i128 {
        let c = |i: usize| -> i128 {
            if k == 0 {
                p[i].0 as i128
            } else {
                p[i].1 as i128
            }
        };
        let b0 = c(0);
        let b1 = 3 * (c(1) - c(0));
        let b2 = 3 * (c(0) - 2 * c(1) + c(2));
        let b3 = -c(0) + 3 * c(1) - 3 * c(2) + c(3);
        ((b3 * t + b2 * d) * t + b1 * d * d) * t + b0 * d * d * d
    }

    #[test]
    fn cubic_matches_independent_expansion() {
        let mut rng = SplitMix64::new(0xBE31);
        for _ in 0..2000 {
            let p: [(i64, i64); 4] = std::array::from_fn(|_| {
                (
                    (rng.next_u64() % 200) as i64 - 100,
                    (rng.next_u64() % 200) as i64 - 100,
                )
            });
            let den = rng.below(20) as u64 + 1;
            let num = rng.next_u64() % (den + 1);
            let (xn, yn, dd) = cubic_pos(&p, num, den).unwrap_or_default();
            let (t, d) = (num as i128, den as i128);
            // Reduced numerator equals exact value: xn/dd == horner/d³.
            assert_eq!(xn * d * d * d, horner(&p, 0, t, d) * dd);
            assert_eq!(yn * d * d * d, horner(&p, 1, t, d) * dd);
        }
    }

    #[test]
    fn endpoints_and_interpolation() {
        let p = [(3, 7), (10, 20), (40, 50), (90, 1)];
        assert_eq!(cubic_pos(&p, 0, 9), Some((3, 7, 1)));
        assert_eq!(cubic_pos(&p, 9, 9), Some((90, 1, 1)));
        // Catmull-Rom interpolates inner knots exactly.
        let c = [(0, 0), (10, 5), (30, -5), (50, 0)];
        assert_eq!(catmull_pos(&c, 0, 1), Some((10, 5, 1)));
        assert_eq!(catmull_pos(&c, 1, 1), Some((30, -5, 1)));
        assert_eq!(catmull_pos(&c, 1, 0), None);
        assert_eq!(cubic_pos(&p, 3, 2), None);
        assert!(flatten_cubic(&p, 0).is_empty());
        let flat = flatten_cubic(&p, 8);
        assert_eq!(flat.len(), 9);
        assert_eq!(flat[0], (3, 7, 1));
        assert_eq!(flat[8], (90, 1, 1));
        // End-clamped Catmull-Rom starts at points[0] and ends at
        // points[n-1] exactly.
        let path = flatten_catmull(&[(0, 0), (10, 0), (10, 10), (30, 10)], 4);
        assert_eq!(path.len(), 13);
        assert_eq!(path[0], (0, 0, 1));
        assert_eq!(path[12], (30, 10, 1));
        assert!(flatten_catmull(&[(1, 2)], 4).is_empty());
    }
}
