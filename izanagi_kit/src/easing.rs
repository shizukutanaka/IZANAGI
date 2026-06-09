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

// ── quartic (t⁴) ────────────────────────────────────────────────────────────

/// Ease-in quartic: `t⁴`. Stronger acceleration than cubic.
#[inline]
pub fn ease_in_quart(t: Fixed) -> Fixed {
    let t2 = t.mul(t);
    t2.mul(t2)
}

/// Ease-out quartic: `1 - (1-t)⁴`.
#[inline]
pub fn ease_out_quart(t: Fixed) -> Fixed {
    let inv = Fixed::ONE - t;
    let i2 = inv.mul(inv);
    Fixed::ONE - i2.mul(i2)
}

/// Ease-in-out quartic.
pub fn ease_in_out_quart(t: Fixed) -> Fixed {
    let half = Fixed::from_ratio(1, 2);
    if t < half {
        let t2 = t.mul(t);
        Fixed::from_int(8).mul(t2.mul(t2))
    } else {
        let inv = Fixed::ONE - t;
        let i2 = inv.mul(inv);
        Fixed::ONE - Fixed::from_int(8).mul(i2.mul(i2))
    }
}

// ── quintic (t⁵) ────────────────────────────────────────────────────────────

/// Ease-in quintic: `t⁵`.
#[inline]
pub fn ease_in_quint(t: Fixed) -> Fixed {
    let t2 = t.mul(t);
    t2.mul(t2).mul(t)
}

/// Ease-out quintic: `1 - (1-t)⁵`.
#[inline]
pub fn ease_out_quint(t: Fixed) -> Fixed {
    let inv = Fixed::ONE - t;
    let i2 = inv.mul(inv);
    Fixed::ONE - i2.mul(i2).mul(inv)
}

/// Ease-in-out quintic.
pub fn ease_in_out_quint(t: Fixed) -> Fixed {
    let half = Fixed::from_ratio(1, 2);
    if t < half {
        let t2 = t.mul(t);
        Fixed::from_int(16).mul(t2.mul(t2).mul(t))
    } else {
        let inv = Fixed::ONE - t;
        let i2 = inv.mul(inv);
        Fixed::ONE - Fixed::from_int(16).mul(i2.mul(i2).mul(inv))
    }
}

// ── sinusoidal ──────────────────────────────────────────────────────────────
//
// Uses the fixed-point CORDIC trig in `Fixed`; results land within a few Q16.16
// LSBs of the ideal (no float). π and π/2 use Milü-style rational approximations.

#[inline]
fn pi() -> Fixed {
    Fixed::from_ratio(355, 113) // ≈ 3.14159292, error < 1e-6
}
#[inline]
fn half_pi() -> Fixed {
    Fixed::from_ratio(355, 226) // ≈ π/2
}

/// Ease-in sine: `1 - cos(t·π/2)`. The gentlest acceleration curve.
#[inline]
pub fn ease_in_sine(t: Fixed) -> Fixed {
    Fixed::ONE - t.mul(half_pi()).cos()
}

/// Ease-out sine: `sin(t·π/2)`.
#[inline]
pub fn ease_out_sine(t: Fixed) -> Fixed {
    t.mul(half_pi()).sin()
}

/// Ease-in-out sine: `(1 - cos(π·t)) / 2`.
#[inline]
pub fn ease_in_out_sine(t: Fixed) -> Fixed {
    let half = Fixed::from_ratio(1, 2);
    (Fixed::ONE - t.mul(pi()).cos()).mul(half)
}

// ── circular ──────────────────────────────────────────────────────────────────
//
// Uses the integer `Fixed::sqrt`; defined for `t` in `[0, 1]`.

/// Ease-in circular: `1 - √(1 - t²)`.
#[inline]
pub fn ease_in_circ(t: Fixed) -> Fixed {
    Fixed::ONE - (Fixed::ONE - t.mul(t)).sqrt()
}

/// Ease-out circular: `√(1 - (1-t)²)`.
#[inline]
pub fn ease_out_circ(t: Fixed) -> Fixed {
    let inv = Fixed::ONE - t;
    (Fixed::ONE - inv.mul(inv)).sqrt()
}

/// Ease-in-out circular.
pub fn ease_in_out_circ(t: Fixed) -> Fixed {
    let half = Fixed::from_ratio(1, 2);
    let two = Fixed::from_int(2);
    if t < half {
        let u = two.mul(t); // 2t
        (Fixed::ONE - (Fixed::ONE - u.mul(u)).sqrt()).mul(half)
    } else {
        let u = two.mul(t) - two; // 2t - 2
        ((Fixed::ONE - u.mul(u)).sqrt() + Fixed::ONE).mul(half)
    }
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

    // ── extended families (quart / quint / sine / circ) ──────────────────────

    /// Every easer must map 0→0 and 1→1 (the tween boundary contract).
    #[test]
    fn test_extended_easers_hit_endpoints() {
        type Easer = fn(Fixed) -> Fixed;
        let z = Fixed::ZERO;
        let one = Fixed::ONE;
        let tol = 64; // sine/circ go through CORDIC/sqrt; allow a few LSBs
        let easers: &[(Easer, &str)] = &[
            (ease_in_quart, "in_quart"),
            (ease_out_quart, "out_quart"),
            (ease_in_out_quart, "in_out_quart"),
            (ease_in_quint, "in_quint"),
            (ease_out_quint, "out_quint"),
            (ease_in_out_quint, "in_out_quint"),
            (ease_in_sine, "in_sine"),
            (ease_out_sine, "out_sine"),
            (ease_in_out_sine, "in_out_sine"),
            (ease_in_circ, "in_circ"),
            (ease_out_circ, "out_circ"),
            (ease_in_out_circ, "in_out_circ"),
        ];
        for (f, name) in easers {
            approx(f(z), z, tol, &format!("{name}(0)"));
            approx(f(one), one, tol, &format!("{name}(1)"));
        }
    }

    #[test]
    fn test_quart_quint_known_values() {
        let h = fr(1, 2);
        approx(ease_in_quart(h), fr(1, 16), 4, "in_quart(0.5)=1/16");
        approx(ease_in_quint(h), fr(1, 32), 4, "in_quint(0.5)=1/32");
    }

    #[test]
    fn test_in_out_families_symmetric_midpoint() {
        // All in-out easers pass through 0.5 at t=0.5.
        let h = fr(1, 2);
        approx(ease_in_out_quart(h), h, 64, "in_out_quart(0.5)");
        approx(ease_in_out_quint(h), h, 64, "in_out_quint(0.5)");
        approx(ease_in_out_sine(h), h, 64, "in_out_sine(0.5)");
        approx(ease_in_out_circ(h), h, 64, "in_out_circ(0.5)");
    }

    #[test]
    fn test_sine_matches_quarter_turn() {
        // ease_out_sine(0.5) = sin(π/4) ≈ 0.70711.
        approx(
            ease_out_sine(fr(1, 2)),
            fr(70711, 100000),
            64,
            "out_sine(0.5)",
        );
        // ease_in_sine(0.5) = 1 - cos(π/4) ≈ 0.29289.
        approx(
            ease_in_sine(fr(1, 2)),
            fr(29289, 100000),
            64,
            "in_sine(0.5)",
        );
    }

    #[test]
    fn test_circ_known_value() {
        // ease_out_circ(0.5) = sqrt(1 - 0.25) = sqrt(0.75) ≈ 0.86603.
        approx(
            ease_out_circ(fr(1, 2)),
            fr(86603, 100000),
            64,
            "out_circ(0.5)",
        );
    }

    #[test]
    fn test_extended_families_monotone() {
        // Each easer should be non-decreasing across [0,1].
        let easers: &[fn(Fixed) -> Fixed] = &[
            ease_in_quart,
            ease_out_quart,
            ease_in_out_quart,
            ease_in_quint,
            ease_out_quint,
            ease_in_out_quint,
            ease_in_sine,
            ease_out_sine,
            ease_in_out_sine,
            ease_in_circ,
            ease_out_circ,
            ease_in_out_circ,
        ];
        let steps = 32u32;
        for f in easers {
            let mut prev = Fixed::ZERO;
            for i in 0..=steps {
                let t = Fixed::from_ratio(i as i32, steps as i32);
                let v = f(t);
                assert!(v.raw() >= prev.raw() - 96, "non-monotone at i={i}");
                prev = v;
            }
        }
    }
}
