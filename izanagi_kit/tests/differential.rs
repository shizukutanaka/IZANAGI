//! Differential-oracle test perspective.
//!
//! The golden, property, and model-based lenses all check the kit against
//! *itself* — past bytes, self-relations, or a reference model written to mirror
//! the same semantics. This lens checks the kit's deterministic, integer-only
//! math against an **independent ground truth**: `f64`.
//!
//! The kit forbids floating point in the *simulation layer* (enforced by
//! `tests/no_float_in_sim.rs`, which scans only production `src/` code), but a
//! test may freely use `f64` as an oracle. So `Fixed`'s CORDIC `sin`/`cos`, its
//! integer `sqrt`, and its Q16.16 `mul`/`div` are validated here against the
//! true real-valued functions within bounded error.
//!
//! Why this catches what property laws cannot: a quadrant/sign error in CORDIC
//! is internally consistent (`sin² + cos² ≈ 1` still holds) yet diverges from
//! real `sin`/`cos`; a wrong shift in `mul` keeps commutativity true yet changes
//! the value. Only an independent oracle exposes those. Deterministic inputs via
//! `SplitMix64`, so every run is reproducible without a float in the engine.

use izanagi_kit::{Fixed, SplitMix64};

const ITERS: usize = 50_000;

/// Q16.16 → real. The fractional resolution is `1/65536 ≈ 1.526e-5`.
fn to_f(x: Fixed) -> f64 {
    x.raw() as f64 / 65536.0
}

// Observed worst-case errors (measured): sin 1.67e-4, cos 1.25e-4, sqrt 1.5e-5.
// Tolerances sit a few × above so the tests are robust, yet far below the O(0.1)
// error a real sign/quadrant/scaling bug would produce.
const TRIG_TOL: f64 = 1.0e-3;
const SQRT_TOL: f64 = 1.0e-4;
const ARITH_TOL: f64 = 1.0e-4;

#[test]
fn fixed_sin_cos_match_f64_over_range() {
    let mut rng = SplitMix64::new(0x_5_1_4C_05);
    for _ in 0..ITERS {
        // Angle in roughly [-2π, 2π] radians, fractional.
        let ang = Fixed::from_ratio(rng.range(-650, 651), 100);
        let theta = to_f(ang);
        let (s, c) = ang.sin_cos();
        assert!(
            (to_f(s) - theta.sin()).abs() < TRIG_TOL,
            "sin({theta}) = {} vs {} (diff {})",
            to_f(s),
            theta.sin(),
            (to_f(s) - theta.sin()).abs()
        );
        assert!(
            (to_f(c) - theta.cos()).abs() < TRIG_TOL,
            "cos({theta}) = {} vs {} (diff {})",
            to_f(c),
            theta.cos(),
            (to_f(c) - theta.cos()).abs()
        );
        // `sin`/`cos` convenience methods must agree with `sin_cos`.
        assert_eq!(ang.sin(), s, "sin() disagrees with sin_cos().0");
        assert_eq!(ang.cos(), c, "cos() disagrees with sin_cos().1");
    }
}

#[test]
fn fixed_sin_cos_anchor_at_zero() {
    // The integer CORDIC is an approximation — even at 0 it is not bit-exact —
    // so anchor within tolerance rather than asserting (0, 1) exactly.
    let (s, c) = Fixed::ZERO.sin_cos();
    assert!(to_f(s).abs() < TRIG_TOL, "sin(0) = {} not ≈ 0", to_f(s));
    assert!((to_f(c) - 1.0).abs() < TRIG_TOL, "cos(0) = {} not ≈ 1", to_f(c));
}

#[test]
fn fixed_sqrt_matches_f64_and_never_overestimates() {
    let mut rng = SplitMix64::new(0x_50_47_05);
    for _ in 0..ITERS {
        // x in [0, ~30000], fractional.
        let x = Fixed::from_ratio(rng.range(0, 30_000), 7);
        let truth = to_f(x).sqrt();
        let got = to_f(x.sqrt());
        assert!((got - truth).abs() < SQRT_TOL, "sqrt({}) = {got} vs {truth}", to_f(x));
        // Integer sqrt floors, so it is never an over-estimate.
        assert!(got <= truth + 1e-9, "sqrt overestimated: {got} > {truth}");
    }
}

#[test]
fn fixed_sqrt_of_perfect_squares_is_exact() {
    for n in 0..=180i32 {
        let sq = Fixed::from_int(n * n);
        assert_eq!(sq.sqrt(), Fixed::from_int(n), "sqrt({}) != {n}", n * n);
    }
}

#[test]
fn fixed_mul_matches_f64() {
    let mut rng = SplitMix64::new(0x_3_4C_05);
    for _ in 0..ITERS {
        // Moderate operands so the product never saturates (|a*b| < 32767).
        let a = Fixed::from_ratio(rng.range(-150, 151), 10);
        let b = Fixed::from_ratio(rng.range(-150, 151), 10);
        let got = to_f(a.mul(b));
        let truth = to_f(a) * to_f(b);
        assert!(
            (got - truth).abs() < ARITH_TOL,
            "{} * {} = {got} vs {truth}",
            to_f(a),
            to_f(b)
        );
    }
}

#[test]
fn fixed_div_matches_f64() {
    let mut rng = SplitMix64::new(0x_D1_05);
    for _ in 0..ITERS {
        let a = Fixed::from_ratio(rng.range(-1000, 1001), 10);
        // Divisor bounded away from zero so the quotient stays in range.
        let mut den = rng.range(-100, 101);
        if den == 0 {
            den = 1;
        }
        let b = Fixed::from_ratio(den, 10);
        let got = to_f(a.div(b));
        let truth = to_f(a) / to_f(b);
        // Skip cases where the true quotient is near the saturating bound.
        if truth.abs() > 30_000.0 {
            continue;
        }
        assert!(
            (got - truth).abs() < ARITH_TOL,
            "{} / {} = {got} vs {truth}",
            to_f(a),
            to_f(b)
        );
    }
}
