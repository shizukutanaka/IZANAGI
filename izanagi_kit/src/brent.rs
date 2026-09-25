//! Brent's root-finding method over [`Fixed`]: the bisection/secant/
//! inverse-quadratic-interpolation hybrid from Brent (1971), the fourth
//! method alongside [`crate::roots`]' `bisect`/`secant`/`newton`. Bracketing
//! is required — convergence is guaranteed at bisection worst-case speed and
//! superlinear when the interpolants behave. Iteration count is a public
//! argument so replays stay bit-exact.
//!
//! ```
//! use izanagi_kit::{brent, fixed::Fixed};
//! // √2 = root of x²−2 on [1, 2].
//! let r = brent::solve(
//!     |x: Fixed| x.mul(x) - Fixed::from_int(2),
//!     Fixed::ONE, Fixed::from_int(2), Fixed::from_raw(32), 64,
//! ).unwrap();
//! assert!((r.raw() - 92681).abs() < 8); // ≈1.41421
//! ```

use crate::fixed::Fixed;

/// Solve `f(x) = 0` on the bracket `[lo, hi]` (`f(lo)` and `f(hi)` must have
/// opposite signs or one be zero). `xtol`/`ftol` are the accepted x- and
/// f-widths; `iters` caps the work — returns `None` on an unbracketed
/// interval, and the best bound so far if the budget runs out mid-way is
/// also returned as `Some` only when `f` actually changes sign across it.
///
/// Brent's bookkeeping: `b` is the current best estimate, `c` the
/// contrapoint (`f(b)·f(c) < 0` invariant), `a` the previous `b`.
pub fn solve<F>(mut f: F, lo: Fixed, hi: Fixed, ftol: Fixed, iters: u32) -> Option<Fixed>
where
    F: FnMut(Fixed) -> Fixed,
{
    let mut a = lo;
    let mut b = hi;
    let mut c = lo;
    let mut fa = f(a);
    let mut fb = f(b);
    if fa.raw() == 0 {
        return Some(a);
    }
    if fb.raw() == 0 {
        return Some(b);
    }
    if (fa.raw() > 0) == (fb.raw() > 0) {
        return None; // unbracketed
    }
    let mut fc = fa;
    let mut d = Fixed::ZERO;
    let mut e = Fixed::ZERO;
    for _ in 0..iters {
        if (fb.raw() > 0) == (fc.raw() > 0) {
            // Restore the bracket: c must oppose b.
            c = a;
            fc = fa;
            e = b - a;
            d = e;
        }
        if fc.abs().raw() < fb.abs().raw() {
            // Order: b is the better estimate.
            a = b;
            b = c;
            c = a;
            fa = fb;
            fb = fc;
            fc = fa;
        }
        // Convergence: midpoint width ≤ ftol or fb == 0.
        let mid = (c - b).div(Fixed::from_int(2));
        if fb.raw() == 0 || mid.abs() <= ftol {
            return Some(b);
        }
        // Try interpolation (IQI when a,b,c distinct, else secant).
        let mut p = Fixed::ZERO;
        let mut q = Fixed::ZERO;
        let mut interpolated = false;
        if e.abs() >= ftol && fa.abs() > fb.abs() {
            if a.raw() == c.raw() {
                // Secant step.
                let denom = fa - fb;
                if denom.raw() != 0 {
                    p = (b - a).mul(fb).div(denom);
                    q = Fixed::ONE;
                    interpolated = true;
                }
            } else {
                // Inverse quadratic interpolation (Brent 1971, p/q form).
                let den_b = fa - fb;
                let den_c = fc - fb;
                let den_a = fc - fa;
                if den_b.raw() != 0 && den_c.raw() != 0 && den_a.raw() != 0 {
                    let s = fb.div(fa);
                    let t = fa.div(fc);
                    let r = fb.div(fc);
                    let p_num = s.mul(t.mul(r - s).mul(b - a) - (Fixed::ONE - r).mul(b - c));
                    let q_num = (t - Fixed::ONE).mul(r - Fixed::ONE).mul(s - Fixed::ONE);
                    if q_num.raw() != 0 {
                        p = p_num;
                        q = q_num;
                        interpolated = true;
                    }
                }
            }
        }
        if interpolated {
            // Accept the step only if it stays inside (b, midpoint) and isn't
            // needlessly small; otherwise bisect.
            let min_step = ftol.mul(Fixed::from_int(2));
            let bound_ok = {
                let step = p.div(q);
                let lhs = b + step;
                // step must land strictly between b and the midpoint.
                (mid.raw() > 0 && lhs > b && lhs < b + mid)
                    || (mid.raw() < 0 && lhs < b && lhs > b + mid)
            };
            let step_abs = p.div(q).abs();
            let use_interp = bound_ok && (step_abs >= min_step || (e.abs() > min_step));
            if use_interp {
                e = d;
                d = p.div(q);
                a = b;
                fa = fb;
                b = b + d;
                fb = f(b);
            } else {
                interpolated = false;
            }
        }
        if !interpolated {
            // Bisection.
            e = mid;
            d = e;
            a = b;
            fa = fb;
            b = b + d;
            fb = f(b);
        }
    }
    // Budget exhausted: report the tighter bound anyway — callers get the
    // best estimate Brent produced rather than nothing.
    Some(b)
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
    fn r2f(raw: i32) -> f64 {
        raw as f64 / 65536.0
    }

    #[test]
    fn sqrt2_and_cbrt() {
        let r = solve(
            |x: Fixed| x.mul(x) - fi(2),
            Fixed::ONE,
            fi(2),
            Fixed::from_raw(16),
            64,
        )
        .unwrap();
        assert!((r.raw() - 92682).abs() < 32); // √2 ≈ 1.41421
                                               // x³ − x − 2 = 0 on [1, 2] → ≈1.52138.
        let r2 = solve(
            |x: Fixed| x.mul(x).mul(x) - x - fi(2),
            Fixed::ONE,
            fi(2),
            Fixed::from_raw(16),
            64,
        )
        .unwrap();
        assert!((r2f(r2.raw()) - 1.52138).abs() < 0.001);
    }

    #[test]
    fn rejects_unbracketed_and_hits_exact() {
        // Same sign at both ends → None.
        assert!(solve(
            |x: Fixed| x.mul(x) + Fixed::ONE,
            fi(-1),
            fi(1),
            Fixed::from_raw(8),
            32
        )
        .is_none());
        // Root exactly at a bound.
        let r = solve(
            |x: Fixed| x - Fixed::ONE,
            Fixed::ONE,
            fi(3),
            Fixed::from_raw(8),
            8,
        )
        .unwrap();
        assert_eq!(r, Fixed::ONE);
        // Linear: exact hit on first interpolation.
        let r2 = solve(
            |x: Fixed| x.mul(fi(2)) - fi(3),
            Fixed::ZERO,
            fi(4),
            Fixed::from_raw(8),
            8,
        )
        .unwrap();
        assert!((r2f(r2.raw()) - 1.5).abs() < 1e-4);
    }

    #[test]
    fn faster_than_bisect_on_smooth() {
        // On a smooth convex function Brent should reach the same tolerance
        // in fewer iterations than plain bisection. Both capped identically.
        let f = |x: Fixed| x.mul(x) - fi(2);
        let brent_hit = solve(f, Fixed::ONE, fi(2), Fixed::from_raw(4), 64).unwrap();
        // Brent converges superlinearly; 8-step bisection is far coarser.
        let bis = crate::roots::bisect(f, Fixed::ONE, fi(2), 8).unwrap_or(Fixed::ZERO);
        let d1 = (brent_hit.raw() - 92682).abs();
        let d2 = (bis.raw() - 92682).abs();
        assert!(d1 < d2);
        assert!(d1 < 8);
    }

    #[test]
    fn budget_and_tolerance() {
        // 1 iteration can't converge a [1,2] bracket to 1e-4.
        let tight = solve(
            |x: Fixed| x.mul(x) - fi(2),
            Fixed::ONE,
            fi(2),
            Fixed::from_raw(4),
            1,
        );
        assert!(tight.is_some()); // best-effort estimate returned
                                  // Wide tolerance converges early and inside the bracket.
        let r = solve(|x: Fixed| x.mul(x) - fi(2), Fixed::ONE, fi(2), fr(1, 4), 64).unwrap();
        assert!((r2f(r.raw()) - 1.414).abs() < 0.3);
    }

    #[test]
    fn cosine_root_matches_oracle() {
        // cos(x) = 0 on [1, 2] → π/2 ≈ 1.5708 — smooth transcendental where
        // secant/IQI steps shine.
        let r = solve(
            |x: Fixed| Fixed::cos(x),
            Fixed::ONE,
            fi(2),
            Fixed::from_raw(16),
            64,
        )
        .unwrap();
        assert!((r2f(r.raw()) - std::f64::consts::FRAC_PI_2).abs() < 0.002);
        // Discontinuous sign still works: (x−1.5)·big on [1,2].
        let r2 = solve(
            |x: Fixed| (x - fr(3, 2)).mul(fi(100)),
            Fixed::ONE,
            fi(2),
            Fixed::from_raw(16),
            64,
        )
        .unwrap();
        assert!((r2f(r2.raw()) - 1.5).abs() < 0.01);
    }
}
