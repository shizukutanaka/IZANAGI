//! Robustness / totality test perspective.
//!
//! The other lenses verify *correctness relations*. This one verifies the most
//! basic contract of a deterministic engine: every public operation is **total**
//! — it returns (possibly an error, `None`, an empty result, or a saturated
//! value) rather than **panicking** — for *all* inputs, including degenerate and
//! extreme ones. A panic is the worst possible desync: one peer aborts while the
//! others march on. The kit's saturating, panic-free arithmetic policy is the
//! promise; this lens is the enforcement.
//!
//! It hammers the numeric, spatial, and combat APIs with the values most likely
//! to break them — `i32::MIN`/`i32::MAX`, `0`, empty slices, out-of-range
//! indices — and asserts each call returns. (A test that merely *returns* has,
//! by definition, not panicked; arithmetic-overflow panics in debug builds turn
//! into test failures here.) This perspective found and drove the fix for a
//! cluster of overflow panics in `combat` (`heal`, `modified`, `base_damage`,
//! `roll_damage`, `splash_attack`).
//!
//! Deterministic via `SplitMix64`.

use izanagi_kit::{
    base_damage, cone, knockback, line, ray_cast, splash_attack, Distance, Fixed, SplitMix64,
    Stats, StatsModifier,
};

const EXTREMES: [i32; 7] = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];

#[test]
fn fixed_arithmetic_is_total_at_extremes() {
    // Every pairing of extreme raw-ish values through the saturating ops must
    // return without panicking.
    for &a in &EXTREMES {
        for &b in &EXTREMES {
            let fa = Fixed::from_int(a / 65536); // keep within Q16.16 integer range-ish
            let fb = Fixed::from_int(b / 65536);
            let _ = fa + fb;
            let _ = fa - fb;
            let _ = fa.mul(fb);
            let _ = fa.div(fb);
            let _ = (-fa, fa.abs(), fa.sign(), fa.sqrt());
            let _ = fa.clamp(fb, fa.max(fb));
            let _ = Fixed::lerp(fa, fb, Fixed::from_int(2));
        }
        // Construction from raw extremes must also saturate, not panic.
        let _ = Fixed::from_int(a);
        let _ = Fixed::from_ratio(a, 0); // zero denominator must be handled
        let _ = Fixed::from_ratio(a, -1);
    }
    // The saturating bounds themselves.
    let _ = Fixed::MAX + Fixed::MAX;
    let _ = Fixed::MIN - Fixed::MAX;
    let _ = (-Fixed::MIN, Fixed::MIN.abs());
    let _ = Fixed::MIN.mul(Fixed::MIN);
}

#[test]
fn combat_is_total_at_extremes() {
    // The cluster of arithmetic that previously overflow-panicked. Every
    // combination of extreme stat values and modifiers must return.
    for &v in &EXTREMES {
        for &w in &EXTREMES {
            let mut s = Stats::new(v, w, v);
            s.set_max_hp(w);
            s.take_damage(v);
            s.heal(w); // <- was the overflow panic
            s.take_overkill_damage(v);

            let m = StatsModifier {
                attack: v,
                defense: w,
                max_hp: v,
            };
            let _ = Stats::new(w, v, w).modified(&m); // <- raw adds, now saturating

            let atk = Stats::new(1000, v, w);
            let def = Stats::new(1000, w, v);
            let _ = base_damage(&atk, &def); // <- raw subtract, now saturating

            let mut victims = [Stats::new(500, 1, w), Stats::new(500, 1, v)];
            let _ = splash_attack(&atk, &mut victims, v); // <- raw mul/sub, now saturating
        }
    }
}

#[test]
fn distance_metrics_are_total_at_extreme_coords() {
    // Distance::between uses i64 + saturation; every metric must survive the
    // full-span coordinate pair without overflow.
    let pts = [
        (i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX),
        (i32::MIN, i32::MAX),
        (0, 0),
    ];
    for &a in &pts {
        for &b in &pts {
            for m in [
                Distance::Manhattan,
                Distance::Chebyshev,
                Distance::EuclideanSquared,
                Distance::Euclidean,
            ] {
                let _ = m.between(a, b);
            }
        }
    }
}

#[test]
fn rng_is_total_on_degenerate_inputs() {
    let mut rng = SplitMix64::new(0x_4A6_05);
    // Empty / degenerate ranges and collections must not panic.
    let _ = rng.below(0);
    let _ = rng.range(5, 5);
    let _ = rng.range(10, 0);
    let _ = rng.range_closed(i32::MIN, i32::MAX);
    let _ = rng.range_u32(7, 7);
    let _ = rng.dice(0, 0);
    let _ = rng.dice(1000, 0);
    let _ = rng.weighted_index(&[]);
    let _ = rng.weighted_index(&[0, 0, 0]);
    let empty: [u32; 0] = [];
    let _ = rng.pick(&empty);
    let mut single = [42u32];
    rng.shuffle(&mut single);
    let mut none: [u32; 0] = [];
    rng.shuffle(&mut none);
}

#[test]
fn geometry_is_total_on_degenerate_inputs() {
    // Zero-length lines, zero/negative ranges, zero direction, blocked origins.
    let _ = line((5, 5), (5, 5));
    let _ = ray_cast((3, 3), (3, 3), |_, _| true);
    let _ = ray_cast((0, 0), (10, 0), |_, _| true); // immediately blocked everywhere
    let _ = cone((0, 0), (0, 0), 5); // zero facing
    let _ = cone((0, 0), (1, 0), -1); // negative range
    let _ = cone((0, 0), (1, 0), 0); // zero range
    let _ = knockback((0, 0), (0, 0), 5, |_, _| false); // zero direction
    let _ = knockback((0, 0), (1, 0), -3, |_, _| false); // negative distance
    let _ = knockback((0, 0), (1, 0), 5, |_, _| true); // wall everywhere
}
