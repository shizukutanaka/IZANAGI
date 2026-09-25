//! Scalar root finders on `Fixed` — bisection (interval-halving,
//! bracket guaranteed to shrink), secant (two-point interpolation, no
//! derivative needed), and Newton–Raphson (quadratic when smooth, needs
//! a derivative closure). Every method runs a *fixed* iteration budget
//! and returns the best bracket/iterate — no tolerance-adaptive loops,
//! so replays are bit-exact.
//!
//! ```
//! use izanagi_kit::fixed::Fixed;
//! use izanagi_kit::roots::bisect;
//!
//! // x² − 2 = 0 on [1, 2]: converges to √2 ≈ 1.41421.
//! let f = |x: Fixed| x.mul(x) - Fixed::from_int(2);
//! let r = bisect(f, Fixed::ONE, Fixed::from_int(2), 40).unwrap();
//! assert!((r.mul(r) - Fixed::from_int(2)).abs() < Fixed::from_ratio(1, 1000));
//! ```

use crate::fixed::Fixed;

/// Interval-halving on `[lo, hi]`: requires `f(lo)` and `f(hi)` to
/// straddle zero (or sit on it); `None` otherwise. `iters` halvings,
/// then the surviving midpoint — guaranteed within `2^−iters` of the
/// bracket. Sign ties go left, a fixed convention.
pub fn bisect<F>(mut f: F, mut lo: Fixed, mut hi: Fixed, iters: u32) -> Option<Fixed>
where
    F: FnMut(Fixed) -> Fixed,
{
    let flo = f(lo);
    let fhi = f(hi);
    if flo.is_zero() {
        return Some(lo);
    }
    if fhi.is_zero() {
        return Some(hi);
    }
    // Sign change required: product of raws ≤ 0 via i64 to avoid overflow.
    if (flo.raw() > 0) == (fhi.raw() > 0) {
        return None;
    }
    let neg_lo = flo.raw() < 0;
    for _ in 0..iters {
        let mid = lo + (hi - lo).div(Fixed::from_int(2));
        if mid == lo || mid == hi {
            break; // ulp-level bracket, cannot shrink further
        }
        let fm = f(mid);
        if fm.is_zero() {
            return Some(mid);
        }
        if (fm.raw() < 0) == neg_lo {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(lo + (hi - lo).div(Fixed::from_int(2)))
}

/// Secant method from seeds `a`, `b`: `x ← x − f·(x−x_prev)/(f−f_prev)`.
/// `None` on a zero denominator or `iters == 0`; otherwise the last
/// iterate after `iters` steps (best effort — secant can diverge on
/// pathological `f`, which the caller's budget bounds).
pub fn secant<F>(mut f: F, mut a: Fixed, mut b: Fixed, iters: u32) -> Option<Fixed>
where
    F: FnMut(Fixed) -> Fixed,
{
    if iters == 0 || a == b {
        return None;
    }
    let mut fa = f(a);
    let mut fb = f(b);
    let mut x = b;
    for _ in 0..iters {
        let d = fb - fa;
        if d.is_zero() {
            break;
        }
        let next = b - fb.mul(b - a).div(d);
        if next == b {
            break; // converged to ulp precision
        }
        a = b;
        fa = fb;
        b = next;
        fb = f(b);
        x = b;
    }
    Some(x)
}

/// Newton–Raphson from `x0` with derivative `df`:
/// `x ← x − f(x)/f'(x)`. `None` on a vanishing derivative or zero
/// budget. Like [`secant`], may diverge off a bad seed — the fixed
/// budget keeps that deterministic too.
pub fn newton<F, D>(mut f: F, mut df: D, mut x: Fixed, iters: u32) -> Option<Fixed>
where
    F: FnMut(Fixed) -> Fixed,
    D: FnMut(Fixed) -> Fixed,
{
    if iters == 0 {
        return None;
    }
    for _ in 0..iters {
        let d = df(x);
        if d.is_zero() {
            break;
        }
        let next = x - f(x).div(d);
        if next == x {
            break;
        }
        x = next;
    }
    Some(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(v: i32) -> Fixed {
        Fixed::from_int(v)
    }
    fn to_f64(x: Fixed) -> f64 {
        x.raw() as f64 / 65536.0
    }

    #[test]
    fn bisect_finds_sqrt2_and_rejects_bad_bracket() {
        let g = |x: Fixed| x.mul(x) - f(2);
        let r = bisect(g, Fixed::ONE, f(2), 40).unwrap();
        assert!((to_f64(r) - std::f64::consts::SQRT_2).abs() < 1e-4, "{r:?}");
        // Root exactly at an endpoint returns immediately.
        assert_eq!(bisect(g, r, f(2), 1), Some(r));
        // Same-sign endpoints → None.
        assert_eq!(bisect(|x: Fixed| x + f(1), f(0), f(3), 10), None);
    }

    #[test]
    fn bisect_respects_budget_and_shrinks() {
        // A wide root region narrows by half each iteration.
        let g = |x: Fixed| x - f(7);
        let lo = f(0);
        let hi = f(14);
        let r5 = bisect(g, lo, hi, 5).unwrap();
        let r30 = bisect(g, lo, hi, 30).unwrap();
        // After 30 halvings the bracket is essentially a point at 7.
        assert!((r30 - f(7)).abs() < Fixed::from_ratio(1, 100));
        // The 5-iteration answer is within 14/2⁵ of 7.
        assert!((r5 - f(7)).abs() < Fixed::from_ratio(7, 8));
    }

    #[test]
    fn secant_converges_to_root() {
        let g = |x: Fixed| x.mul(x) - f(2);
        let r = secant(g, Fixed::ONE, f(2), 20).unwrap();
        assert!((to_f64(r) - std::f64::consts::SQRT_2).abs() < 1e-3, "{r:?}");
        // Linear f: exact in one step.
        let r1 = secant(|x: Fixed| x - f(9), f(0), f(12), 2).unwrap();
        assert!((r1 - f(9)).abs() < Fixed::from_ratio(1, 100));
        assert_eq!(secant(|x: Fixed| x, f(1), f(1), 5), None);
        assert_eq!(secant(|x: Fixed| x, f(1), f(2), 0), None);
    }

    #[test]
    fn newton_quadratic_and_cubic() {
        // x²−2 via Newton with exact derivative 2x: quadratic convergent.
        let g = |x: Fixed| x.mul(x) - f(2);
        let dg = |x: Fixed| x.mul(f(2));
        let r = newton(g, dg, Fixed::ONE, 10).unwrap();
        assert!((to_f64(r) - std::f64::consts::SQRT_2).abs() < 1e-4, "{r:?}");
        // x³−8: root at 2.
        let r2 = newton(
            |x: Fixed| x.mul(x).mul(x) - f(8),
            |x: Fixed| f(3).mul(x.mul(x)),
            f(3),
            30,
        )
        .unwrap();
        assert!((to_f64(r2) - 2.0).abs() < 1e-2, "{r2:?}");
        // Derivative hits zero (x=0 for 2x) → breaks → last iterate.
        assert!(newton(g, dg, Fixed::ZERO, 5).is_some());
        assert_eq!(newton(g, dg, Fixed::ONE, 0), None);
    }

    #[test]
    fn deterministic_twice() {
        let g = |x: Fixed| x.mul(x) - f(3);
        assert_eq!(
            bisect(g, Fixed::ONE, f(2), 25),
            bisect(g, Fixed::ONE, f(2), 25)
        );
        assert_eq!(
            secant(g, Fixed::ONE, f(2), 12),
            secant(g, Fixed::ONE, f(2), 12)
        );
    }
}
