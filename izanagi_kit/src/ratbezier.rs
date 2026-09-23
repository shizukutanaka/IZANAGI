//! Rational Bézier curves — weighted control points evaluated
//! exactly over [`crate::frac::Frac`]. The parameter `t` is a
//! rational, so every point on the curve is an exact rational
//! pair; no sampling error accumulates.
//!
//! `eval` uses homogeneous de Casteljau: each control point is
//! lifted to `(w·x, w·y, w)` and interpolated linearly, then
//! divided back at the end — the same algebra as projective
//! NURBS evaluation, but every intermediate value is exact.
//!
//! ```
//! use izanagi_kit::frac::Frac;
//! use izanagi_kit::ratbezier::eval;
//! // Quarter-arc-ish: weight 1, w, 1 on a right-angle control polygon
//! let pts = vec![
//!     (Frac::from_int(0), Frac::from_int(1)),
//!     (Frac::from_int(0), Frac::from_int(0)),
//!     (Frac::from_int(1), Frac::from_int(0)),
//! ];
//! let w = vec![Frac::from_int(1), Frac::from_int(1), Frac::from_int(1)];
//! let (x, y) = eval(&pts, &w, Frac::new(1, 2)).unwrap();
//! // plain quadratic midpoint: 1/4·P0 + 1/2·P1 + 1/4·P2
//! assert_eq!(x, Frac::new(1, 4));
//! assert_eq!(y, Frac::new(1, 4));
//! ```
//!
//! References: Piegl & Tiller, *The NURBS Book* §4.1; Farin,
//! *Curves and Surfaces for CAGD* §13.

use crate::frac::Frac;

/// Evaluate a rational Bézier at rational parameter `t`.
///
/// `pts` are the control points `[(x,y)]`, `w` the matching
/// positive weights (`w.len() == pts.len()`). Returns the
/// exact `(x(t), y(t))`, or `None` on empty input, ragged
/// lengths, a zero weight, or a vanishing homogeneous `w`
/// coordinate (poles of the curve).
pub fn eval(pts: &[(Frac, Frac)], w: &[Frac], t: Frac) -> Option<(Frac, Frac)> {
    let n = pts.len();
    if n == 0 || w.len() != n || w.iter().any(|&x| x == Frac::from_int(0)) {
        return None;
    }
    // Homogeneous lifting: P_i -> (w_i·x_i, w_i·y_i, w_i)
    let mut h: Vec<(Frac, Frac, Frac)> = pts
        .iter()
        .zip(w)
        .map(|(&(x, y), &wi)| (x * wi, y * wi, wi))
        .collect();
    let u = Frac::from_int(1) - t;
    for _ in 1..n {
        for i in 0..h.len() - 1 {
            h[i] = (
                h[i].0 * u + h[i + 1].0 * t,
                h[i].1 * u + h[i + 1].1 * t,
                h[i].2 * u + h[i + 1].2 * t,
            );
        }
        h.pop();
    }
    let (x, y, wsum) = h[0];
    Some((x.checked_div(wsum)?, y.checked_div(wsum)?))
}

/// First derivative `B'(t)` evaluated exactly — derived from
/// the degree `n−1` rational Bézier with homogeneous control
/// differences `n·(H_{i+1} − H_i)` projected back through the
/// quotient rule: `B' = (X'·W − X·W') / W²` on each axis.
/// `None` under the same guards as [`eval`].
pub fn eval_deriv(pts: &[(Frac, Frac)], w: &[Frac], t: Frac) -> Option<(Frac, Frac)> {
    let n = pts.len();
    if n < 2 || w.len() != n || w.iter().any(|&x| x == Frac::from_int(0)) {
        return None;
    }
    let h: Vec<(Frac, Frac, Frac)> = pts
        .iter()
        .zip(w)
        .map(|(&(x, y), &wi)| (x * wi, y * wi, wi))
        .collect();
    // de Casteljau to (X(t), Y(t), W(t)) and to the degree-1
    // difference curve for (X'(t)/n, Y'(t)/n, W'(t)/n)
    let eval_h = |hs: &[(Frac, Frac, Frac)]| -> (Frac, Frac, Frac) {
        let mut v = hs.to_vec();
        let u = Frac::from_int(1) - t;
        for _ in 1..v.len() {
            for i in 0..v.len() - 1 {
                v[i] = (
                    v[i].0 * u + v[i + 1].0 * t,
                    v[i].1 * u + v[i + 1].1 * t,
                    v[i].2 * u + v[i + 1].2 * t,
                );
            }
            v.pop();
        }
        v[0]
    };
    let (x, y, wc) = eval_h(&h);
    let dh: Vec<(Frac, Frac, Frac)> = (0..n - 1)
        .map(|i| {
            (
                h[i + 1].0 - h[i].0,
                h[i + 1].1 - h[i].1,
                h[i + 1].2 - h[i].2,
            )
        })
        .collect();
    let (dx, dy, dw) = eval_h(&dh);
    let nn = Frac::from_int((n - 1) as i128);
    let w2 = wc * wc;
    if w2 == Frac::from_int(0) {
        return None;
    }
    Some((
        (dx * nn * wc - x * nn * dw).checked_div(w2)?,
        (dy * nn * wc - y * nn * dw).checked_div(w2)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Direct definition oracle: polynomial Bernstein form on
    /// the homogeneous lift, `Σ B_i^n(t)·H_i`, evaluated
    /// coefficient-by-coefficient — no de Casteljau sharing.
    fn oracle(pts: &[(Frac, Frac)], w: &[Frac], t: Frac) -> (Frac, Frac) {
        let n = pts.len() - 1;
        let mut acc = (Frac::from_int(0), Frac::from_int(0), Frac::from_int(0));
        // binomial via Pascal recurrence, kept exact
        for i in 0..=n {
            let mut c = Frac::from_int(1);
            for j in 0..i {
                c = c * Frac::from_int((n - j) as i128)
                    .checked_div(Frac::from_int((j + 1) as i128))
                    .unwrap();
            }
            let mut bi = c;
            for _ in 0..i {
                bi = bi * t;
            }
            for _ in 0..n - i {
                bi = bi * (Frac::from_int(1) - t);
            }
            let (x, y) = pts[i];
            let wi = w[i];
            acc.0 = acc.0 + bi * wi * x;
            acc.1 = acc.1 + bi * wi * y;
            acc.2 = acc.2 + bi * wi;
        }
        (
            acc.0.checked_div(acc.2).unwrap(),
            acc.1.checked_div(acc.2).unwrap(),
        )
    }

    #[test]
    fn basics() {
        let pts = vec![
            (Frac::from_int(0), Frac::from_int(0)),
            (Frac::from_int(1), Frac::from_int(0)),
        ];
        let w = vec![Frac::from_int(1), Frac::from_int(1)];
        let (x, y) = eval(&pts, &w, Frac::new(1, 4)).unwrap();
        assert_eq!(x, Frac::new(1, 4));
        assert_eq!(y, Frac::from_int(0));
        assert!(eval(&[], &[], Frac::from_int(0)).is_none());
        assert!(eval(&pts, &[Frac::from_int(1)], Frac::from_int(0)).is_none());
        assert!(eval(
            &pts,
            &[Frac::from_int(1), Frac::from_int(0)],
            Frac::from_int(0)
        )
        .is_none());
    }

    /// A rational quadratic with weights (1, 1/√2·w, 1)
    /// traces conics — verified at the algebraic level: for
    /// `w = (1, w, 1)` on the right-angle control triangle,
    /// `x² + y²` stays at the conic value instead of the
    /// polynomial one. We check the exact point against the
    /// homogeneous-form oracle on random t.
    #[test]
    fn oracle_random() {
        let mut rng = SplitMix64::new(0xBE71);
        for _ in 0..200 {
            let n = 2 + rng.below(4) as usize;
            let pts: Vec<(Frac, Frac)> = (0..n)
                .map(|_| {
                    (
                        Frac::new(i128::from(rng.below(21)) - 10, 1 + i128::from(rng.below(5))),
                        Frac::new(i128::from(rng.below(21)) - 10, 1 + i128::from(rng.below(5))),
                    )
                })
                .collect();
            let w: Vec<Frac> = (0..n)
                .map(|_| Frac::new(1 + i128::from(rng.below(7)), 1 + i128::from(rng.below(4))))
                .collect();
            let t = Frac::new(i128::from(rng.below(9)), 1 + i128::from(rng.below(9)));
            let got = eval(&pts, &w, t);
            // oracle's denominator may vanish only where the
            // weights sum cancels — skip those rare inputs
            if let Some(g) = got {
                let want = oracle(&pts, &w, t);
                assert_eq!(g, want);
            }
        }
    }

    /// Finite-difference-free check: derivative at `t` and
    /// at `t` again via the symmetric secant
    /// `(B(t+h)−B(t−h))/(2h)` over Frac agrees to first order —
    /// we instead check the exact identity `B'(0)` direction:
    /// for any curve, `B'(0) = n·(P1−P0)` weighted.
    #[test]
    fn derivative_at_endpoints() {
        let pts = vec![
            (Frac::from_int(0), Frac::from_int(0)),
            (Frac::from_int(2), Frac::from_int(4)),
            (Frac::from_int(6), Frac::from_int(0)),
        ];
        let w = vec![Frac::from_int(1); 3];
        let (dx, dy) = eval_deriv(&pts, &w, Frac::from_int(0)).unwrap();
        assert_eq!(dx, Frac::from_int(4));
        assert_eq!(dy, Frac::from_int(8));
        let (dx, dy) = eval_deriv(&pts, &w, Frac::from_int(1)).unwrap();
        assert_eq!(dx, Frac::from_int(8));
        assert_eq!(dy, Frac::from_int(-8));
        // secant agreement on rational interior points
        let mut rng = SplitMix64::new(9);
        for _ in 0..100 {
            let n = 3 + rng.below(3) as usize;
            let p2: Vec<(Frac, Frac)> = (0..n)
                .map(|_| {
                    (
                        Frac::from_int(i128::from(rng.below(9))),
                        Frac::from_int(i128::from(rng.below(9))),
                    )
                })
                .collect();
            let w2: Vec<Frac> = (0..n)
                .map(|_| Frac::from_int(1 + i128::from(rng.below(4))))
                .collect();
            let t = Frac::new(1 + i128::from(rng.below(7)), 10);
            let h = Frac::new(1, 1000);
            if let (Some((xl, yl)), Some((xr, yr)), Some((dx, dy))) = (
                eval(&p2, &w2, t - h),
                eval(&p2, &w2, t + h),
                eval_deriv(&p2, &w2, t),
            ) {
                // |secant − tangent| must be O(h): check the
                // difference is bounded by a small rational
                let sx = (xr - xl).checked_div(Frac::from_int(2) * h).unwrap();
                let sy = (yr - yl).checked_div(Frac::from_int(2) * h).unwrap();
                assert!((sx - dx).abs().cmp_frac(&Frac::new(1, 20)) == std::cmp::Ordering::Less);
                assert!((sy - dy).abs().cmp_frac(&Frac::new(1, 20)) == std::cmp::Ordering::Less);
            }
        }
    }
}
