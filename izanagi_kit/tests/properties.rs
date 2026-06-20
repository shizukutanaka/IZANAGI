//! Property / metamorphic test perspective.
//!
//! A verification lens that complements the kit's example-based unit tests and
//! its golden-value pins. Instead of asserting a *specific* output for a
//! *specific* input, each test here asserts an algebraic **law** that must hold
//! for *all* inputs — commutativity, identity, involution, ordering,
//! range-containment — and checks it over thousands of inputs generated
//! deterministically by the kit's own `SplitMix64`. A single violated law
//! localizes a whole class of bugs that hand-picked examples never reach, while
//! the seeded generator keeps every run reproducible in CI (no `proptest`
//! dependency, in keeping with the zero-dependency policy).

use izanagi_kit::geometry::line_len;
use izanagi_kit::{
    chebyshev_distance, cone, knockback, line, manhattan_distance, reflect_point, rotate_90_ccw,
    rotate_90_cw, splash_attack, Fixed, SplitMix64, Stats, Vec2, Vec3,
};

const ITERS: usize = 3000;

/// A diverse `Fixed` in a moderate range — fractions, integers, and the
/// identities — chosen to avoid saturation so laws sensitive to it (lerp
/// endpoints, double negation) hold exactly.
fn rand_fixed(rng: &mut SplitMix64) -> Fixed {
    match rng.below(5) {
        0 => Fixed::ZERO,
        1 => Fixed::ONE,
        2 => Fixed::from_int(rng.range(-200, 200)),
        3 => Fixed::from_ratio(rng.range(-1000, 1000), rng.range(1, 100)),
        _ => Fixed::from_int(rng.range(-3000, 3000)),
    }
}

/// Like [`rand_fixed`] but also emits the saturating extremes, for laws that
/// must hold even at `Fixed::MIN` / `Fixed::MAX`.
fn rand_fixed_ext(rng: &mut SplitMix64) -> Fixed {
    match rng.below(7) {
        0 => Fixed::MIN,
        1 => Fixed::MAX,
        _ => rand_fixed(rng),
    }
}

fn rand_coord(rng: &mut SplitMix64) -> (i32, i32) {
    (rng.range(-1000, 1001), rng.range(-1000, 1001))
}

// ── Fixed: algebraic laws ──────────────────────────────────────────────────

#[test]
fn prop_fixed_add_is_commutative() {
    // Saturating addition is commutative even at the extremes.
    let mut rng = SplitMix64::new(0xADDC0);
    for _ in 0..ITERS {
        let (a, b) = (rand_fixed_ext(&mut rng), rand_fixed_ext(&mut rng));
        assert_eq!(a + b, b + a, "add not commutative for {a:?},{b:?}");
    }
}

#[test]
fn prop_fixed_mul_is_commutative() {
    let mut rng = SplitMix64::new(0x34C);
    for _ in 0..ITERS {
        let (a, b) = (rand_fixed_ext(&mut rng), rand_fixed_ext(&mut rng));
        assert_eq!(a.mul(b), b.mul(a), "mul not commutative for {a:?},{b:?}");
    }
}

#[test]
fn prop_fixed_identities() {
    let mut rng = SplitMix64::new(0x1DE77);
    for _ in 0..ITERS {
        let a = rand_fixed_ext(&mut rng);
        assert_eq!(a + Fixed::ZERO, a, "additive identity");
        assert_eq!(a.mul(Fixed::ONE), a, "multiplicative identity");
        assert_eq!(a.mul(Fixed::ZERO), Fixed::ZERO, "annihilation by zero");
    }
}

#[test]
fn prop_fixed_abs_is_non_negative() {
    let mut rng = SplitMix64::new(0xAB5);
    for _ in 0..ITERS {
        let a = rand_fixed_ext(&mut rng);
        assert!(a.abs() >= Fixed::ZERO, "abs negative for {a:?}");
    }
}

#[test]
fn prop_fixed_double_negation_is_identity_except_min() {
    // -(-a) == a for every value except MIN (whose negation saturates to MAX).
    let mut rng = SplitMix64::new(0x4E61);
    for _ in 0..ITERS {
        let a = rand_fixed(&mut rng); // moderate range never hits MIN
        assert_eq!(-(-a), a, "double negation not identity for {a:?}");
    }
    // The documented MIN exception.
    assert_eq!(-Fixed::MIN, Fixed::MAX);
}

#[test]
fn prop_fixed_clamp_is_in_range_and_idempotent() {
    let mut rng = SplitMix64::new(0xC1A3);
    for _ in 0..ITERS {
        let x = rand_fixed_ext(&mut rng);
        let (p, q) = (rand_fixed(&mut rng), rand_fixed(&mut rng));
        let (lo, hi) = (p.min(q), p.max(q)); // ensure lo <= hi
        let c = x.clamp(lo, hi);
        assert!(c >= lo && c <= hi, "clamp out of range: {c:?} not in [{lo:?},{hi:?}]");
        assert_eq!(c.clamp(lo, hi), c, "clamp not idempotent");
    }
}

#[test]
fn prop_fixed_min_max_consistency() {
    let mut rng = SplitMix64::new(0x31AC);
    for _ in 0..ITERS {
        let (a, b) = (rand_fixed_ext(&mut rng), rand_fixed_ext(&mut rng));
        assert!(a.min(b) <= a.max(b), "min > max for {a:?},{b:?}");
        assert_eq!(a.min(b), b.min(a), "min not commutative");
        assert_eq!(a.max(b), b.max(a), "max not commutative");
        // min and max together recover the original pair.
        assert!(
            (a.min(b) == a && a.max(b) == b) || (a.min(b) == b && a.max(b) == a),
            "min/max do not partition the pair"
        );
    }
}

#[test]
fn prop_fixed_lerp_hits_endpoints() {
    // lerp(a, b, 0) == a and lerp(a, b, 1) == b when no intermediate saturates
    // (guaranteed by the moderate generator).
    let mut rng = SplitMix64::new(0x1E47);
    for _ in 0..ITERS {
        let (a, b) = (rand_fixed(&mut rng), rand_fixed(&mut rng));
        assert_eq!(Fixed::lerp(a, b, Fixed::ZERO), a, "lerp t=0 != a");
        assert_eq!(Fixed::lerp(a, b, Fixed::ONE), b, "lerp t=1 != b");
    }
}

#[test]
fn prop_fixed_sqrt_is_monotonic_and_non_negative() {
    let mut rng = SplitMix64::new(0x5417);
    for _ in 0..ITERS {
        let a = rand_fixed(&mut rng).abs();
        let b = rand_fixed(&mut rng).abs();
        assert!(a.sqrt() >= Fixed::ZERO, "sqrt negative");
        if a <= b {
            assert!(a.sqrt() <= b.sqrt(), "sqrt not monotonic: {a:?} <= {b:?}");
        }
    }
}

// ── geometry: inverse / symmetry / containment laws ────────────────────────

#[test]
fn prop_rotate_90_four_times_is_identity() {
    let mut rng = SplitMix64::new(0x407);
    for _ in 0..ITERS {
        let (x, y) = rand_coord(&mut rng);
        let (mut a, mut b) = (x, y);
        for _ in 0..4 {
            let (nx, ny) = rotate_90_cw(a, b);
            a = nx;
            b = ny;
        }
        assert_eq!((a, b), (x, y), "4x rotate_cw != identity for ({x},{y})");
    }
}

#[test]
fn prop_rotate_cw_ccw_are_inverses() {
    let mut rng = SplitMix64::new(0xCCC);
    for _ in 0..ITERS {
        let (x, y) = rand_coord(&mut rng);
        let (cx, cy) = rotate_90_cw(x, y);
        assert_eq!(rotate_90_ccw(cx, cy), (x, y), "ccw∘cw != id for ({x},{y})");
    }
}

#[test]
fn prop_reflect_point_is_an_involution() {
    let mut rng = SplitMix64::new(0x4EF1);
    for _ in 0..ITERS {
        let p = rand_coord(&mut rng);
        let c = rand_coord(&mut rng);
        assert_eq!(reflect_point(reflect_point(p, c), c), p, "reflect not involutive");
    }
}

#[test]
fn prop_distances_are_symmetric_and_ordered() {
    let mut rng = SplitMix64::new(0xD157);
    for _ in 0..ITERS {
        let a = rand_coord(&mut rng);
        let b = rand_coord(&mut rng);
        assert_eq!(manhattan_distance(a, b), manhattan_distance(b, a), "manhattan asym");
        assert_eq!(chebyshev_distance(a, b), chebyshev_distance(b, a), "chebyshev asym");
        // Chebyshev (king) distance never exceeds Manhattan (taxicab).
        assert!(
            chebyshev_distance(a, b) <= manhattan_distance(a, b),
            "chebyshev > manhattan for {a:?},{b:?}"
        );
    }
}

#[test]
fn prop_line_endpoints_length_and_adjacency() {
    let mut rng = SplitMix64::new(0x11E5);
    for _ in 0..ITERS {
        let a = rand_coord(&mut rng);
        let b = rand_coord(&mut rng);
        let cells = line(a, b);
        assert_eq!(cells.first(), Some(&a), "line must start at a");
        assert_eq!(cells.last(), Some(&b), "line must end at b");
        assert_eq!(cells.len(), line_len(a, b), "line_len disagrees with line");
        // Each Bresenham step is a single king move.
        for w in cells.windows(2) {
            let (dx, dy) = ((w[1].0 - w[0].0).abs(), (w[1].1 - w[0].1).abs());
            assert!(dx <= 1 && dy <= 1 && (dx + dy) > 0, "non-king step {:?}->{:?}", w[0], w[1]);
        }
    }
}

#[test]
fn prop_knockback_never_exceeds_distance_and_lands_open() {
    // A wall on a deterministic sparse lattice; knockback must never overshoot
    // its distance budget and must never come to rest on a blocked cell.
    let is_wall = |x: i32, y: i32| (x.rem_euclid(7) == 0) && (y.rem_euclid(5) == 0);
    let mut rng = SplitMix64::new(0xC8AC);
    let dirs = [
        (1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1),
    ];
    for _ in 0..ITERS {
        let from = rand_coord(&mut rng);
        // Only meaningful when the entity does not start inside a wall.
        if is_wall(from.0, from.1) {
            continue;
        }
        let dir = dirs[rng.below(dirs.len() as u32) as usize];
        let dist = rng.range(0, 12);
        let landing = knockback(from, dir, dist, is_wall);
        assert!(
            chebyshev_distance(from, landing) <= dist,
            "knockback overshot: {from:?} -> {landing:?} budget {dist}"
        );
        assert!(!is_wall(landing.0, landing.1), "knockback rested in a wall {landing:?}");
    }
}

#[test]
fn prop_cone_cells_are_in_front_and_in_range() {
    let mut rng = SplitMix64::new(0xC04E);
    let facings = [
        (1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1),
    ];
    for _ in 0..ITERS {
        let origin = rand_coord(&mut rng);
        let facing = facings[rng.below(facings.len() as u32) as usize];
        let range = rng.range(0, 9);
        let r2 = (range as i64) * (range as i64);
        for (cx, cy) in cone(origin, facing, range) {
            let (dx, dy) = ((cx - origin.0) as i64, (cy - origin.1) as i64);
            // In Euclidean range.
            assert!(dx * dx + dy * dy <= r2, "cone cell out of range");
            // Strictly in front of the facing (positive dot product).
            assert!(dx * facing.0 as i64 + dy * facing.1 as i64 > 0, "cone cell behind facing");
        }
    }
}

#[test]
fn prop_fixed_floor_ceil_round_fract_laws() {
    // The defining relationships of the rounding family. Moderate inputs only
    // (rand_fixed) so the laws hold exactly without hitting the saturating edge
    // (ceil at Fixed::MAX saturates, breaking ceil-floor == ONE).
    let mut rng = SplitMix64::new(0x_F100_4_05);
    for _ in 0..ITERS {
        let x = rand_fixed(&mut rng);
        let fl = x.floor();
        let ce = x.ceil();
        let fr = x.fract();
        let rd = x.round();

        // Reconstruction: floor(x) + fract(x) == x, exactly.
        assert_eq!(fl + fr, x, "floor + fract != x for {x:?}");
        // fract is in [0, 1).
        assert!(fr >= Fixed::ZERO && fr < Fixed::ONE, "fract {fr:?} out of [0,1) for {x:?}");
        // floor(x) <= x <= ceil(x).
        assert!(fl <= x && x <= ce, "floor <= x <= ceil violated for {x:?}");
        // ceil - floor is 0 (x integer) or 1 (otherwise).
        let span = ce - fl;
        assert!(
            span == Fixed::ZERO || span == Fixed::ONE,
            "ceil - floor = {span:?} not in {{0,1}} for {x:?}"
        );
        assert_eq!(span == Fixed::ZERO, x.is_integer(), "ceil==floor iff integer, for {x:?}");
        // round(x) is one of the two bracketing integers.
        assert!(rd == fl || rd == ce, "round {rd:?} not in {{floor,ceil}} for {x:?}");
        // floor/ceil/round are idempotent and fixed on integers.
        assert_eq!(fl.floor(), fl, "floor not idempotent");
        assert_eq!(ce.ceil(), ce, "ceil not idempotent");
        assert!(fl.is_integer() && ce.is_integer() && rd.is_integer(), "results must be integers");
    }
}

#[test]
fn prop_fixed_step_toward_approaches_without_overshoot() {
    // step_toward clamps (no interpolation rounding), so its bounds are exact.
    // Laws: the result never leaves [x, target], never moves away from target,
    // a zero step is a no-op, an already-at-target value stays put, and a step
    // at least as large as the gap reaches the target exactly.
    let mut rng = SplitMix64::new(0x57E_4_05);
    for _ in 0..ITERS {
        let x = rand_fixed(&mut rng);
        let target = rand_fixed(&mut rng);
        let step = rand_fixed(&mut rng).abs(); // step_toward takes |step| anyway
        let r = x.step_toward(target, step);

        let lo = x.min(target);
        let hi = x.max(target);
        // Never overshoots: the result stays between the start and the target.
        assert!(r >= lo && r <= hi, "step_toward overshot: {r:?} not in [{lo:?},{hi:?}]");
        // Always approaches (or reaches) the target — never moves away.
        assert!(
            r.abs_diff(target) <= x.abs_diff(target),
            "step_toward moved away from target: {x:?} -> {r:?}, target {target:?}"
        );
        // A zero step does not move; being at the target stays at the target.
        assert_eq!(x.step_toward(target, Fixed::ZERO), x, "zero step moved");
        assert_eq!(target.step_toward(target, step), target, "moved off target");
        // A step covering the whole gap reaches the target exactly.
        let gap = x.abs_diff(target);
        assert_eq!(x.step_toward(target, gap), target, "full-gap step did not reach target");
    }
}

// ── vec: Vec2/Vec3 composite-operation laws ────────────────────────────────
// Small components (±10) so products/sums never saturate — the laws below then
// hold exactly. (Saturation would break antisymmetry: -clamp(v) != clamp(-v).)

/// A `Fixed` in roughly [-10, 10].
fn rand_small(rng: &mut SplitMix64) -> Fixed {
    Fixed::from_ratio(rng.range(-1000, 1001), 100)
}

#[test]
fn prop_vec2_composite_laws() {
    let mut rng = SplitMix64::new(0x_2EC_2_05);
    for _ in 0..ITERS {
        let a = Vec2::new(rand_small(&mut rng), rand_small(&mut rng));
        let b = Vec2::new(rand_small(&mut rng), rand_small(&mut rng));

        // dot is commutative; len_sq is dot(self,self) and never negative.
        assert_eq!(a.dot(b), b.dot(a), "Vec2 dot not commutative");
        assert_eq!(a.len_sq(), a.dot(a), "len_sq != dot(self,self)");
        assert!(a.len_sq() >= Fixed::ZERO, "len_sq negative");
        // scale by 1 is identity; scale by 0 is the zero vector.
        assert_eq!(a.scale(Fixed::ONE), a, "scale by ONE not identity");
        assert_eq!(a.scale(Fixed::ZERO), Vec2::ZERO, "scale by ZERO not zero vector");
        // 2-D cross is antisymmetric and zero with itself.
        assert_eq!(a.cross_2d(a), Fixed::ZERO, "cross_2d(a,a) != 0");
        assert_eq!(a.cross_2d(b), -(b.cross_2d(a)), "cross_2d not antisymmetric");
    }
    // normalize: zero -> None, non-zero -> Some.
    assert!(Vec2::ZERO.normalize().is_none(), "normalize(0) must be None");
    assert!(Vec2::new(Fixed::from_int(3), Fixed::from_int(4)).normalize().is_some());
}

#[test]
fn prop_vec3_composite_laws() {
    let mut rng = SplitMix64::new(0x_3EC_3_05);
    for _ in 0..ITERS {
        let a = Vec3::new(rand_small(&mut rng), rand_small(&mut rng), rand_small(&mut rng));
        let b = Vec3::new(rand_small(&mut rng), rand_small(&mut rng), rand_small(&mut rng));

        assert_eq!(a.dot(b), b.dot(a), "Vec3 dot not commutative");
        assert_eq!(a.len_sq(), a.dot(a), "len_sq != dot(self,self)");
        assert!(a.len_sq() >= Fixed::ZERO, "len_sq negative");
        assert_eq!(a.scale(Fixed::ONE), a, "scale by ONE not identity");
        assert_eq!(a.scale(Fixed::ZERO), Vec3::ZERO, "scale by ZERO not zero vector");
        // cross of a vector with itself is the zero vector.
        assert_eq!(a.cross(a), Vec3::ZERO, "cross(a,a) != 0");
        // cross is antisymmetric: cross(a,b) == -cross(b,a), componentwise.
        let ab = a.cross(b);
        let ba = b.cross(a);
        assert_eq!(ab.x, -ba.x, "cross.x not antisymmetric");
        assert_eq!(ab.y, -ba.y, "cross.y not antisymmetric");
        assert_eq!(ab.z, -ba.z, "cross.z not antisymmetric");
    }
    assert!(Vec3::ZERO.normalize().is_none(), "normalize(0) must be None");
    assert!(Vec3::new(Fixed::from_int(1), Fixed::from_int(2), Fixed::from_int(2)).normalize().is_some());
}

#[test]
fn prop_easing_functions_hit_their_endpoints() {
    // Every easing curve maps the unit interval's endpoints to themselves:
    // ease(0) ≈ 0 and ease(1) ≈ 1. (back/elastic overshoot in the interior but
    // still pin the endpoints.) A scaling or offset bug breaks this universally.
    use izanagi_kit::easing::*;
    type E = fn(Fixed) -> Fixed;
    let fns: &[(&str, E)] = &[
        ("smoothstep", ease_smoothstep),
        ("smootherstep", ease_smootherstep),
        ("linear", linear),
        ("in_quad", ease_in_quad),
        ("out_quad", ease_out_quad),
        ("in_out_quad", ease_in_out_quad),
        ("in_cubic", ease_in_cubic),
        ("out_cubic", ease_out_cubic),
        ("in_out_cubic", ease_in_out_cubic),
        ("in_quart", ease_in_quart),
        ("out_quart", ease_out_quart),
        ("in_out_quart", ease_in_out_quart),
        ("in_quint", ease_in_quint),
        ("out_quint", ease_out_quint),
        ("in_out_quint", ease_in_out_quint),
        ("in_sine", ease_in_sine),
        ("out_sine", ease_out_sine),
        ("in_out_sine", ease_in_out_sine),
        ("in_circ", ease_in_circ),
        ("out_circ", ease_out_circ),
        ("in_out_circ", ease_in_out_circ),
        ("in_back", ease_in_back),
        ("out_back", ease_out_back),
        ("in_out_back", ease_in_out_back),
        ("in_bounce", ease_in_bounce),
        ("out_bounce", ease_out_bounce),
        ("in_out_bounce", ease_in_out_bounce),
        ("in_expo", ease_in_expo),
        ("out_expo", ease_out_expo),
        ("in_out_expo", ease_in_out_expo),
        ("in_elastic", ease_in_elastic),
        ("out_elastic", ease_out_elastic),
        ("in_out_elastic", ease_in_out_elastic),
    ];
    let to_f = |x: Fixed| x.raw() as f64 / 65536.0;
    for (name, f) in fns {
        let at0 = to_f(f(Fixed::ZERO));
        let at1 = to_f(f(Fixed::ONE));
        assert!(at0.abs() < 5.0e-3, "{name}(0) = {at0}, expected ≈ 0");
        assert!((at1 - 1.0).abs() < 5.0e-3, "{name}(1) = {at1}, expected ≈ 1");
    }
}

/// `splash_attack` contract, generalized beyond the two example-based unit
/// tests. Target `i` receives `max(1, attack − falloff·i)` raw damage, then
/// `max(1, raw − defense)`, so two documented properties must hold for every
/// input:
///
/// 1. **Floor of 1** — every target takes at least 1 damage, regardless of how
///    high its defense is or how far the falloff has decayed the raw amount.
/// 2. **Monotone non-increasing** — with `falloff ≥ 0` and equal-defense
///    targets, "each outer ring takes less" means the returned sequence never
///    increases. (Unequal defenses can legitimately break monotonicity, so that
///    half uses a shared defense.)
///
/// Also checks HP-removal accounting: each target's HP drops by exactly
/// `min(dmg, hp_before)`.
#[test]
fn prop_splash_attack_floor_and_monotone_falloff() {
    let mut rng = SplitMix64::new(0x5_71A5_4001);
    for _ in 0..ITERS {
        let attack = rng.range(1, 500);
        let falloff = rng.range(0, 60); // non-negative
        let n = rng.range(1, 8) as usize;
        let attacker = Stats::new(100, attack, 0);

        // (1) Floor holds for arbitrary (mixed) defenses.
        let mut mixed: Vec<Stats> = (0..n)
            .map(|_| Stats::new(rng.range(1, 1000), attack, rng.range(0, 600)))
            .collect();
        let mixed_dmg = splash_attack(&attacker, &mut mixed, falloff);
        assert!(mixed_dmg.iter().all(|&d| d >= 1), "splash floor of 1 violated: {mixed_dmg:?}");

        // (2) Monotone non-increasing + HP accounting with equal defenses.
        let def = rng.range(0, 600);
        let hp_before: Vec<i32> = (0..n).map(|_| rng.range(1, 1000)).collect();
        let mut eq: Vec<Stats> = hp_before.iter().map(|&hp| Stats::new(hp, attack, def)).collect();
        let dmg = splash_attack(&attacker, &mut eq, falloff);
        for w in dmg.windows(2) {
            assert!(w[0] >= w[1], "splash damage increased across rings: {dmg:?}");
        }
        for (k, t) in eq.iter().enumerate() {
            assert_eq!(
                t.hp,
                hp_before[k] - dmg[k].min(hp_before[k]),
                "splash HP-removal accounting wrong at target {k}"
            );
        }
    }
}
