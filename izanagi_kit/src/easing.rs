//! Integer easing and tweening functions over [`Fixed`](crate::Fixed).
//!
//! All functions take `t` in `[0, 1]` (Q16.16) and return a value in the
//! same range. Extrapolation beyond `[0, 1]` is defined but the caller is
//! responsible for clamping `t` if needed (see [`crate::Fixed::clamp`]).
//! No float, no OS dependency — bit-identical across targets.
//!
//! Reference: Robert Penner's easing equations (standard industry set).

use crate::Fixed;

/// Smooth Hermite interpolation: `3t² − 2t³`. Maps `[0,1] → [0,1]` with
/// zero first-derivative at both endpoints — natural ease-in-out without
/// the overshoot of `ease_in_out_back`. Returns exactly `Fixed::ZERO` at
/// `t = 0` and `Fixed::ONE` at `t = 1`.
pub fn ease_smoothstep(t: Fixed) -> Fixed {
    let three = Fixed::from_int(3);
    let two = Fixed::from_int(2);
    three.mul(t.mul(t)) - two.mul(t.mul(t).mul(t))
}

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

/// Ease-in-out cubic: blends `ease_in_cubic` and `ease_out_cubic` at the midpoint.
/// Formula: `4t³` for `t < 0.5`, `1 - 4(1-t)³` for `t ≥ 0.5`.
pub fn ease_in_out_cubic(t: Fixed) -> Fixed {
    let half = Fixed::from_ratio(1, 2);
    let four = Fixed::from_int(4);
    if t < half {
        four.mul(t.mul(t).mul(t))
    } else {
        let inv = Fixed::ONE - t;
        Fixed::ONE - four.mul(inv.mul(inv).mul(inv))
    }
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

// ── back (overshoot) ─────────────────────────────────────────────────────────
//
// c1 ≈ 1.70158 (standard Penner overshoot constant).
// ease_in_back(t)  = c3·t³ − c1·t²   where c3 = c1 + 1 ≈ 2.70158
// ease_out_back(t) = 1 + c3·(t−1)³ + c1·(t−1)²

#[inline]
fn c1() -> Fixed {
    Fixed::from_ratio(170158, 100000) // ≈ 1.70158
}
#[inline]
fn c3() -> Fixed {
    Fixed::from_ratio(270158, 100000) // ≈ 2.70158 = c1 + 1
}
#[inline]
fn c2() -> Fixed {
    Fixed::from_ratio(259870, 100000) // ≈ 2.5949 = c1 * 1.525
}
/// Ease-in back: overshoots at the start (`t` briefly goes slightly below 0).
pub fn ease_in_back(t: Fixed) -> Fixed {
    let c1 = c1();
    let c3 = c3();
    c3.mul(t.mul(t).mul(t)) - c1.mul(t.mul(t))
}

/// Ease-out back: overshoots at the end (`t` briefly exceeds 1).
pub fn ease_out_back(t: Fixed) -> Fixed {
    let c1 = c1();
    let c3 = c3();
    let u = t - Fixed::ONE;
    Fixed::ONE + c3.mul(u.mul(u).mul(u)) + c1.mul(u.mul(u))
}

/// Ease-in-out back: overshoots on both ends.
pub fn ease_in_out_back(t: Fixed) -> Fixed {
    let half = Fixed::from_ratio(1, 2);
    let two = Fixed::from_int(2);
    let c2 = c2();
    if t < half {
        let u = two.mul(t);
        u.mul(u).mul((c2 + Fixed::ONE).mul(u) - c2).mul(half)
    } else {
        let u = two.mul(t) - two;
        (u.mul(u).mul((c2 + Fixed::ONE).mul(u) + c2) + two).mul(half)
    }
}

// ── bounce ───────────────────────────────────────────────────────────────────
//
// Piecewise polynomial approximation of a bouncing ball. The `out_bounce`
// version is the base; `in_bounce` inverts it, and `in_out_bounce` halves both.

fn bounce_out(t: Fixed) -> Fixed {
    let n1 = Fixed::from_ratio(121, 16); // 7.5625 = 121/16
    if t < Fixed::from_ratio(4, 11) {
        n1.mul(t.mul(t))
    } else if t < Fixed::from_ratio(8, 11) {
        let u = t - Fixed::from_ratio(6, 11);
        n1.mul(u.mul(u)) + Fixed::from_ratio(3, 4)
    } else if t < Fixed::from_ratio(10, 11) {
        let u = t - Fixed::from_ratio(9, 11);
        n1.mul(u.mul(u)) + Fixed::from_ratio(15, 16)
    } else {
        let u = t - Fixed::from_ratio(21, 22);
        n1.mul(u.mul(u)) + Fixed::from_ratio(63, 64)
    }
}

/// Ease-out bounce: ball dropped and bouncing to rest.
#[inline]
pub fn ease_out_bounce(t: Fixed) -> Fixed {
    bounce_out(t)
}

/// Ease-in bounce: mirror of `ease_out_bounce`.
#[inline]
pub fn ease_in_bounce(t: Fixed) -> Fixed {
    Fixed::ONE - bounce_out(Fixed::ONE - t)
}

/// Ease-in-out bounce.
pub fn ease_in_out_bounce(t: Fixed) -> Fixed {
    let half = Fixed::from_ratio(1, 2);
    let two = Fixed::from_int(2);
    if t < half {
        (Fixed::ONE - bounce_out(Fixed::ONE - two.mul(t))).mul(half)
    } else {
        (Fixed::ONE + bounce_out(two.mul(t) - Fixed::ONE)).mul(half)
    }
}

// ── ease reversal ────────────────────────────────────────────────────────────

/// Convert an ease-in function to its ease-out mirror by time-reversal:
/// `ease_reversed(t, f) = 1 − f(1 − t)`.
///
/// Works with any function that maps `[0, 1] → [0, 1]` and satisfies
/// `f(0) = 0`, `f(1) = 1` (all standard ease-in curves). Result maps
/// `0 → 0` and `1 → 1`, starting fast and decelerating.
#[inline]
pub fn ease_reversed(t: Fixed, ease_in: fn(Fixed) -> Fixed) -> Fixed {
    Fixed::ONE - ease_in(Fixed::ONE - t)
}

// ── exponential ─────────────────────────────────────────────────────────────
//
// Decomposes 2^x into an integer power-of-two (bit-shift) and a fractional
// part approximated via a 3-term Taylor series for e^(f·ln2). Max error < 0.5%.

fn exp2_tween(t: Fixed) -> Fixed {
    // x = 10·(t−1) ∈ (−10, 0).
    let x = Fixed::from_int(10).mul(t - Fixed::ONE);
    let f = x.fract(); // fractional part ∈ [0, 1)
    let n = x - f; // integer part (Fixed with zero fraction)
    let n_int = n.raw() >> 16; // ∈ {−10, …, −1}
                               // 2^n via right-shift: −n_int ∈ [1, 10].
    let pow_n = Fixed::from_ratio(1, 1i32 << (-n_int) as u32);
    // 2^f ≈ 1 + f·ln2 + f²·(ln²2/2) + f³·(ln³2/6)  (Horner form).
    let c1 = Fixed::from_ratio(693147, 1_000_000); // ln 2
    let c2 = Fixed::from_ratio(240227, 1_000_000); // ln²2 / 2
    let c3 = Fixed::from_ratio(55504, 1_000_000); // ln³2 / 6
    let exp_f = Fixed::ONE + f.mul(c1 + f.mul(c2 + f.mul(c3)));
    pow_n.mul(exp_f)
}

/// Ease-in exponential: `2^(10·(t−1))`. Near-zero start, explosive finish.
/// Exact `0` at `t=0`, exact `1` at `t=1`. Max approximation error ≈ 0.4%.
pub fn ease_in_expo(t: Fixed) -> Fixed {
    if t <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    if t >= Fixed::ONE {
        return Fixed::ONE;
    }
    exp2_tween(t)
}

/// Ease-out exponential: mirror of `ease_in_expo`. Explosive start, slow finish.
pub fn ease_out_expo(t: Fixed) -> Fixed {
    if t <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    if t >= Fixed::ONE {
        return Fixed::ONE;
    }
    Fixed::ONE - exp2_tween(Fixed::ONE - t)
}

/// Ease-in-out exponential: before 0.5 uses `ease_in_expo(2t)/2`,
/// after 0.5 uses the symmetric mirror.
pub fn ease_in_out_expo(t: Fixed) -> Fixed {
    if t <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    if t >= Fixed::ONE {
        return Fixed::ONE;
    }
    let half = Fixed::from_ratio(1, 2);
    let two = Fixed::from_int(2);
    if t < half {
        exp2_tween(two.mul(t)).mul(half)
    } else {
        (two - exp2_tween(two.mul(Fixed::ONE - t))).mul(half)
    }
}

/// Ease-in elastic: `−2^(10t−10) · sin((10t − 10.75) · 2π/3)`.
/// Oscillates below zero near `t = 0`, passes through `(0, 0)` and `(1, 1)`
/// exactly, giving a "spring pull-back" entrance. Float-free; uses the same
/// CORDIC `sin` and Taylor-series `exp2` as the exponential family.
pub fn ease_in_elastic(t: Fixed) -> Fixed {
    if t <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    if t >= Fixed::ONE {
        return Fixed::ONE;
    }
    // c4 = 2π/3 ≈ 2.094395; Fixed::from_ratio(2094395, 1_000_000) ≈ Q16.16 137258
    let c4 = Fixed::from_ratio(2094395, 1_000_000);
    let x = Fixed::from_int(10).mul(t - Fixed::ONE); // 10*(t-1), in (-10, 0)
    let angle = (x - Fixed::from_ratio(3, 4)).mul(c4);
    let pow_val = exp2_tween(t); // 2^(10*(t-1))
    -(pow_val.mul(angle.sin()))
}

/// Ease-out elastic: `2^(−10t) · sin((10t − 0.75) · 2π/3) + 1`.
/// Overshoots above 1 near `t = 1`, decelerating to the endpoint with a
/// "spring overshoot" finish.
pub fn ease_out_elastic(t: Fixed) -> Fixed {
    if t <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    if t >= Fixed::ONE {
        return Fixed::ONE;
    }
    let c4 = Fixed::from_ratio(2094395, 1_000_000);
    let ten_t = Fixed::from_int(10).mul(t);
    let angle = (ten_t - Fixed::from_ratio(3, 4)).mul(c4);
    let pow_val = exp2_tween(Fixed::ONE - t); // 2^(-10t)
    pow_val.mul(angle.sin()) + Fixed::ONE
}

/// Ease-in-out elastic: elastic in for the first half, elastic out for the
/// second. Oscillates slightly below 0 near `t = 0` and above 1 near `t = 1`.
pub fn ease_in_out_elastic(t: Fixed) -> Fixed {
    if t <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    if t >= Fixed::ONE {
        return Fixed::ONE;
    }
    // c5 = 2π/4.5 = 4π/9 ≈ 1.39626; Fixed::from_ratio(1396263, 1_000_000) ≈ 91506
    let c5 = Fixed::from_ratio(1396263, 1_000_000);
    // 11.125 = 89/8
    let phase = Fixed::from_ratio(89, 8);
    let twenty = Fixed::from_int(20);
    let two = Fixed::from_int(2);
    let half = Fixed::from_ratio(1, 2);
    let angle = (twenty.mul(t) - phase).mul(c5);
    if t < half {
        let pow_val = exp2_tween(two.mul(t)); // 2^(20t-10)
        -(pow_val.mul(angle.sin()).mul(half))
    } else {
        let pow_val = exp2_tween(two.mul(Fixed::ONE - t)); // 2^(-20t+10)
        pow_val.mul(angle.sin()).mul(half) + Fixed::ONE
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
    fn test_ease_in_out_cubic_endpoints() {
        let tol = 4;
        approx(
            ease_in_out_cubic(Fixed::ZERO),
            Fixed::ZERO,
            tol,
            "in_out_cubic(0)",
        );
        approx(
            ease_in_out_cubic(Fixed::ONE),
            Fixed::ONE,
            tol,
            "in_out_cubic(1)",
        );
    }

    #[test]
    fn test_ease_in_out_cubic_midpoint_is_half() {
        let h = fr(1, 2);
        approx(ease_in_out_cubic(h), h, 4, "in_out_cubic(0.5)");
    }

    #[test]
    fn test_ease_in_out_cubic_quarter_value() {
        // ease_in_out_cubic(0.25) = 4 * (0.25)^3 = 4 * 0.015625 = 0.0625 = 1/16
        let q = fr(1, 4);
        approx(ease_in_out_cubic(q), fr(1, 16), 4, "in_out_cubic(0.25)");
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
            (ease_in_back, "in_back"),
            (ease_out_back, "out_back"),
            (ease_in_out_back, "in_out_back"),
            (ease_in_bounce, "in_bounce"),
            (ease_out_bounce, "out_bounce"),
            (ease_in_out_bounce, "in_out_bounce"),
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

    // ── back (overshoot) tests ───────────────────────────────────────────────

    #[test]
    fn test_back_endpoints() {
        let z = Fixed::ZERO;
        let one = Fixed::ONE;
        let tol = 8;
        approx(ease_in_back(z), z, tol, "in_back(0)");
        approx(ease_out_back(one), one, tol, "out_back(1)");
        approx(ease_in_out_back(z), z, tol, "in_out_back(0)");
        approx(ease_in_out_back(one), one, tol, "in_out_back(1)");
    }

    #[test]
    fn test_in_back_undershoots_at_start() {
        // ease_in_back(t) briefly goes negative for small t.
        let small = fr(1, 10);
        assert!(
            ease_in_back(small).raw() < 0,
            "ease_in_back should undershoot near 0"
        );
    }

    #[test]
    fn test_out_back_overshoots_at_end() {
        // ease_out_back(t) briefly exceeds 1 near t=1.
        let near_one = fr(9, 10);
        assert!(
            ease_out_back(near_one).raw() > Fixed::ONE.raw(),
            "ease_out_back should overshoot near 1"
        );
    }

    // ── bounce tests ─────────────────────────────────────────────────────────

    #[test]
    fn test_bounce_endpoints() {
        let z = Fixed::ZERO;
        let one = Fixed::ONE;
        let tol = 8;
        approx(ease_in_bounce(z), z, tol, "in_bounce(0)");
        approx(ease_in_bounce(one), one, tol, "in_bounce(1)");
        approx(ease_out_bounce(z), z, tol, "out_bounce(0)");
        approx(ease_out_bounce(one), one, tol, "out_bounce(1)");
        approx(ease_in_out_bounce(z), z, tol, "in_out_bounce(0)");
        approx(ease_in_out_bounce(one), one, tol, "in_out_bounce(1)");
    }

    #[test]
    fn test_bounce_stays_in_range() {
        // Bounce functions should stay within [0, 1] (no overshoot by design).
        let steps = 64u32;
        for f in [ease_out_bounce, ease_in_bounce, ease_in_out_bounce] {
            for i in 0..=steps {
                let t = Fixed::from_ratio(i as i32, steps as i32);
                let v = f(t);
                assert!(
                    v.raw() >= -4 && v.raw() <= Fixed::ONE.raw() + 4,
                    "bounce out of [0,1] at t={i}/{steps}: {v:?}"
                );
            }
        }
    }

    #[test]
    fn test_bounce_in_out_midpoint() {
        // ease_in_out_bounce(0.5) should be close to 0.5.
        approx(
            ease_in_out_bounce(fr(1, 2)),
            fr(1, 2),
            512,
            "in_out_bounce(0.5)",
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

    // ── exponential easing tests ──────────────────────────────────────────────

    #[test]
    fn test_expo_endpoints() {
        let z = Fixed::ZERO;
        let one = Fixed::ONE;
        assert_eq!(ease_in_expo(z), z);
        assert_eq!(ease_in_expo(one), one);
        assert_eq!(ease_out_expo(z), z);
        assert_eq!(ease_out_expo(one), one);
        assert_eq!(ease_in_out_expo(z), z);
        assert_eq!(ease_in_out_expo(one), one);
    }

    #[test]
    fn test_ease_in_expo_half() {
        // ease_in_expo(0.5) = 2^(-5) = 1/32 ≈ 0.03125
        let result = ease_in_expo(fr(1, 2));
        approx(result, fr(1, 32), 200, "in_expo(0.5)");
    }

    #[test]
    fn test_ease_out_expo_half() {
        // ease_out_expo(0.5) = 1 - 2^(-5) = 31/32 ≈ 0.96875
        let result = ease_out_expo(fr(1, 2));
        approx(result, fr(31, 32), 200, "out_expo(0.5)");
    }

    #[test]
    fn test_ease_in_out_expo_quarter() {
        // ease_in_out_expo(0.25) = 2^(-5)/2 = 1/64 ≈ 0.015625
        let result = ease_in_out_expo(fr(1, 4));
        approx(result, fr(1, 64), 200, "in_out_expo(0.25)");
    }

    #[test]
    fn test_ease_in_out_expo_midpoint_is_half() {
        approx(
            ease_in_out_expo(fr(1, 2)),
            fr(1, 2),
            200,
            "in_out_expo(0.5)",
        );
    }

    #[test]
    fn test_expo_monotone() {
        let steps = 32u32;
        for f in [ease_in_expo, ease_out_expo, ease_in_out_expo] {
            let mut prev = Fixed::ZERO;
            for i in 0..=steps {
                let t = Fixed::from_ratio(i as i32, steps as i32);
                let v = f(t);
                assert!(v.raw() >= prev.raw() - 200, "expo non-monotone at i={i}");
                prev = v;
            }
        }
    }

    #[test]
    fn test_expo_deterministic() {
        let t = fr(3, 7);
        assert_eq!(ease_in_expo(t), ease_in_expo(t));
        assert_eq!(ease_out_expo(t), ease_out_expo(t));
        assert_eq!(ease_in_out_expo(t), ease_in_out_expo(t));
    }

    #[test]
    fn test_ease_reversed_endpoints() {
        // reversed(0, ease_in_quad) = 1 - ease_in_quad(1) = 1 - 1 = 0
        // reversed(1, ease_in_quad) = 1 - ease_in_quad(0) = 1 - 0 = 1
        assert_eq!(ease_reversed(Fixed::ZERO, ease_in_quad), Fixed::ZERO);
        assert_eq!(ease_reversed(Fixed::ONE, ease_in_quad), Fixed::ONE);
    }

    #[test]
    fn test_ease_reversed_matches_ease_out_quad() {
        // ease_reversed(t, ease_in_quad) should equal ease_out_quad(t).
        for n in [0i32, 1, 2, 3, 4] {
            let t = fr(n, 4);
            let rev = ease_reversed(t, ease_in_quad);
            let out = ease_out_quad(t);
            let diff = (rev.raw() - out.raw()).abs();
            assert!(diff <= 2, "t={n}/4: rev={rev:?} out={out:?}");
        }
    }

    #[test]
    fn test_ease_reversed_linear_is_identity() {
        // reversed(t, linear) = 1 - (1-t) = t
        let t = fr(3, 8);
        assert_eq!(ease_reversed(t, linear), t);
    }

    #[test]
    fn test_ease_smoothstep_endpoints() {
        assert_eq!(ease_smoothstep(Fixed::ZERO), Fixed::ZERO);
        assert_eq!(ease_smoothstep(Fixed::ONE), Fixed::ONE);
    }

    #[test]
    fn test_ease_smoothstep_midpoint_is_half() {
        let half = Fixed::from_ratio(1, 2);
        assert_eq!(
            ease_smoothstep(half),
            half,
            "3*(0.5^2) - 2*(0.5^3) = 0.75 - 0.25 = 0.5"
        );
    }

    #[test]
    fn test_ease_smoothstep_symmetric_around_half() {
        // smoothstep(t) + smoothstep(1-t) == 1 (point-symmetry).
        let t = fr(1, 4);
        let inv = Fixed::ONE - t;
        let sum = ease_smoothstep(t) + ease_smoothstep(inv);
        assert_eq!(sum, Fixed::ONE, "smoothstep symmetry");
    }

    // ── elastic easing tests ─────────────────────────────────────────────────

    #[test]
    fn test_elastic_endpoints() {
        let z = Fixed::ZERO;
        let one = Fixed::ONE;
        assert_eq!(ease_in_elastic(z), z);
        assert_eq!(ease_in_elastic(one), one);
        assert_eq!(ease_out_elastic(z), z);
        assert_eq!(ease_out_elastic(one), one);
        assert_eq!(ease_in_out_elastic(z), z);
        assert_eq!(ease_in_out_elastic(one), one);
    }

    #[test]
    fn test_in_elastic_oscillates_below_zero() {
        // ease_in_elastic(0.3) is in the negative-oscillation phase.
        // At t=0.3: angle=(10*0.3-10.75)*(2π/3)=-7.75*2.094≈-16.23 rad,
        // sin≈+0.5, pow=2^(-7)≈0.0078, result≈-0.004 < 0.
        let t = fr(3, 10);
        assert!(
            ease_in_elastic(t).raw() < 0,
            "ease_in_elastic(0.3) should go below 0"
        );
    }

    #[test]
    fn test_out_elastic_oscillates_above_one() {
        // ease_out_elastic(0.7) is in the positive-oscillation phase.
        // At t=0.7: angle=(7-0.75)*(2π/3)=6.25*2.094≈13.09 rad,
        // sin≈+0.5, pow=2^(-3)≈0.125, result≈1.063 > 1.
        let t = fr(7, 10);
        assert!(
            ease_out_elastic(t).raw() > Fixed::ONE.raw(),
            "ease_out_elastic(0.7) should exceed 1"
        );
    }
}
