//! Integer easing and tweening functions over [`Fixed`](crate::Fixed).
//!
//! All functions take `t` in `[0, 1]` (Q16.16) and return a value in the
//! same range. Extrapolation beyond `[0, 1]` is defined but the caller is
//! responsible for clamping `t` if needed (see [`crate::Fixed::clamp`]).
//! No float, no OS dependency — bit-identical across targets.
//!
//! Reference: Robert Penner's easing equations (standard industry set).

use crate::Fixed;

/// Ease-in quadratic: `t²`. Starts slow, accelerates.
#[inline]
pub fn ease_in_quad(t: Fixed) -> Fixed {
    t.mul(t)
}

/// Ease-out quadratic: `1 - (1-t)²`. Starts fast, decelerates.
#[inline]
pub fn ease_out_quad(t: Fixed) -> Fixed {
    let one = Fixed::ONE;
    let inv = one - t;
    one - inv.mul(inv)
}

/// Ease-in-out quadratic: blends `ease_in` and `ease_out` at the midpoint.
pub fn ease_in_out_quad(t: Fixed) -> Fixed {
    let two = Fixed::from_int(2);
    let half = Fixed::from_ratio(1, 2);
    if t < half {
        two.mul(t.mul(t))
    } else {
        let inv = Fixed::ONE - t;
        Fixed::ONE - two.mul(inv.mul(inv))
    }
}

/// Ease-in cubic: `t³`. Stronger acceleration than quadratic.
#[inline]
pub fn ease_in_cubic(t: Fixed) -> Fixed {
    t.mul(t).mul(t)
}

/// Ease-out cubic: `1 - (1-t)³`.
#[inline]
pub fn ease_out_cubic(t: Fixed) -> Fixed {
    let inv = Fixed::ONE - t;
    Fixed::ONE - inv.mul(inv).mul(inv)
}

/// Linear (identity): `t`. Useful as a slot-compatible no-op easer.
#[inline]
pub fn linear(t: Fixed) -> Fixed {
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Fixed;

    fn fr(n: i32, d: i32) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    // Utility: check |a.raw() - b.raw()| <= tol.
    fn approx(a: Fixed, b: Fixed, tol: i32, label: &str) {
        let diff = (a.raw() - b.raw()).abs();
        assert!(diff <= tol, "{label}: |{a:?} - {b:?}| = {diff} > {tol}");
    }

    #[test]
    fn test_all_easers_at_zero_return_zero() {
        let z = Fixed::ZERO;
        assert_eq!(ease_in_quad(z), z);
        assert_eq!(ease_out_quad(z), z);
        assert_eq!(ease_in_out_quad(z), z);
        assert_eq!(ease_in_cubic(z), z);
        assert_eq!(ease_out_cubic(z), z);
        assert_eq!(linear(z), z);
    }

    #[test]
    fn test_all_easers_at_one_return_one() {
        let one = Fixed::ONE;
        let tol = 4; // ≈0.00006, a couple of Q16.16 LSBs for rounding
        approx(ease_in_quad(one), one, tol, "in_quad(1)");
        approx(ease_out_quad(one), one, tol, "out_quad(1)");
        approx(ease_in_out_quad(one), one, tol, "in_out_quad(1)");
        approx(ease_in_cubic(one), one, tol, "in_cubic(1)");
        approx(ease_out_cubic(one), one, tol, "out_cubic(1)");
        assert_eq!(linear(one), one);
    }

    #[test]
    fn test_ease_in_quad_half() {
        // ease_in_quad(0.5) = 0.25
        let h = fr(1, 2);
        approx(ease_in_quad(h), fr(1, 4), 4, "in_quad(0.5)");
    }

    #[test]
    fn test_ease_out_quad_half() {
        // ease_out_quad(0.5) = 1 - 0.25 = 0.75
        let h = fr(1, 2);
        approx(ease_out_quad(h), fr(3, 4), 4, "out_quad(0.5)");
    }

    #[test]
    fn test_ease_in_out_quad_midpoint_is_half() {
        // ease_in_out_quad(0.5) should be exactly 0.5 (symmetry point).
        let h = fr(1, 2);
        approx(ease_in_out_quad(h), h, 4, "in_out_quad(0.5)");
    }

    #[test]
    fn test_ease_in_cubic_half() {
        // ease_in_cubic(0.5) = 0.125
        let h = fr(1, 2);
        approx(ease_in_cubic(h), fr(1, 8), 4, "in_cubic(0.5)");
    }

    #[test]
    fn test_linear_is_identity() {
        for (n, d) in [(0, 1), (1, 4), (1, 2), (3, 4), (1, 1)] {
            let t = fr(n, d);
            assert_eq!(linear(t), t);
        }
    }

    #[test]
    fn test_ease_in_out_monotone() {
        // ease_in_out_quad should be non-decreasing on [0,1].
        let steps = 20u32;
        let one = Fixed::ONE;
        let mut prev = Fixed::ZERO;
        for i in 0..=steps {
            let t = Fixed::from_ratio(i as i32, steps as i32).clamp(Fixed::ZERO, one);
            let v = ease_in_out_quad(t);
            assert!(
                v.raw() >= prev.raw() - 2,
                "non-monotone at i={i}: {prev:?} > {v:?}"
            );
            prev = v;
        }
    }

    #[test]
    fn test_easers_are_deterministic() {
        let t = Fixed::from_ratio(3, 7);
        assert_eq!(ease_in_quad(t), ease_in_quad(t));
        assert_eq!(ease_out_quad(t), ease_out_quad(t));
        assert_eq!(ease_in_out_quad(t), ease_in_out_quad(t));
    }
}
