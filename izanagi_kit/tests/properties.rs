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

use izanagi_kit::dice::Dice;
use izanagi_kit::fov::compute_fov;
use izanagi_kit::geometry::line_len;
use izanagi_kit::pathfinding::{
    astar, descend, dijkstra_map, jps, octile_distance, smooth_path, weighted_astar,
};
use izanagi_kit::turn::{Scheduler, ACTION_COST};
use izanagi_kit::wfc::wfc_solve_backtrack;
use izanagi_kit::camera::Camera;
use izanagi_kit::combat::{
    apply_resistance, base_damage, critical_strike, melee_attack, roll_damage, roll_to_hit,
    StatsModifier,
};
use izanagi_kit::damage::{DamageType, ResistanceProfile};
use izanagi_kit::inventory::Inventory;
use izanagi_kit::status::{StatTarget, StatusSet};
use izanagi_kit::tilemap::{LayeredMap, TileMap};
use izanagi_kit::visibility::{Visibility, VisibilityMap};
use izanagi_kit::equipment::{EquipSlot, Equipment};
use izanagi_kit::progression::{LevelCurve, Progression};
use izanagi_kit::shufflebag::ShuffleBag;
use izanagi_kit::{
    chebyshev_distance, cone, fbm_1d, fbm_1d_wrap, fbm_2d, fbm_2d_wrap, fbm_3d, generate_bsp,
    generate_cave, generate_dungeon, hash_1d, hash_2d, hash_3d, knockback, line, manhattan_distance,
    normalize_noise, reflect_point, ridge_noise_2d, rotate_90_ccw, rotate_90_cw, splash_attack,
    value_noise_1d, value_noise_1d_wrap, value_noise_2d, value_noise_2d_wrap, value_noise_3d, Aabb,
    BspParams, CaveParams, Cooldown, Dungeon, Fixed, GenParams, InfluenceMap, MultiMap,
    PassabilityGrid, RandomTable, SpatialHash, SplitMix64, Stats, TimerQueue, Vec2, Vec3,
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
    let mut rng = SplitMix64::new(0x0F10_0405);
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
    let mut rng = SplitMix64::new(0x0057_E405);
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
    let mut rng = SplitMix64::new(0x002E_C205);
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
    let mut rng = SplitMix64::new(0x003E_C305);
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

/// **Monotone easings are bounded and non-decreasing** — the standard
/// (non-overshooting) easing families map `[0,1] → [0,1]` and never decrease:
/// for `t1 ≤ t2`, `f(t1) ≤ f(t2)`. (back/elastic overshoot and bounce oscillates,
/// so they are excluded here — their endpoints are covered by the test above.)
/// A scaling/clamp regression that pushed a curve out of range or made it dip
/// would corrupt animation playback. Tolerance absorbs fixed-point rounding.
#[test]
fn prop_easing_monotone_families_bounded_and_increasing() {
    use izanagi_kit::easing::*;
    type E = fn(Fixed) -> Fixed;
    let fns: &[(&str, E)] = &[
        ("linear", linear),
        ("smoothstep", ease_smoothstep),
        ("smootherstep", ease_smootherstep),
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
        ("in_expo", ease_in_expo),
        ("out_expo", ease_out_expo),
        ("in_out_expo", ease_in_out_expo),
    ];
    let to_f = |x: Fixed| x.raw() as f64 / 65536.0;
    let mut rng = SplitMix64::new(0x0EA5_1A60);
    const EPS: f64 = 1.0e-2;
    for _ in 0..ITERS {
        // Two ordered samples in [0, 1] via Q16.16 ratios.
        let p = Fixed::from_ratio(rng.below(0x1_0001) as i32, 0x1_0000);
        let q = Fixed::from_ratio(rng.below(0x1_0001) as i32, 0x1_0000);
        let (t1, t2) = if p <= q { (p, q) } else { (q, p) };
        for (name, f) in fns {
            let (v1, v2) = (to_f(f(t1)), to_f(f(t2)));
            assert!(
                v1 >= -EPS && v1 <= 1.0 + EPS,
                "{name}({}) = {v1} out of [0,1]",
                to_f(t1)
            );
            assert!(
                v2 >= v1 - EPS,
                "{name} not non-decreasing: f({})={v1} > f({})={v2}",
                to_f(t1),
                to_f(t2)
            );
        }
    }
}

/// **out-ease is the reflection of in-ease** — by definition every ease-out
/// curve is `ease_out_X(t) = 1 − ease_in_X(1 − t)`, which is exactly what
/// [`ease_reversed`] computes. This metamorphic identity ties each in/out pair
/// together; a transcription error in either half breaks it. Verified for the
/// polynomial, sine, circ and expo families (within their approximation error).
#[test]
fn prop_easing_out_equals_reversed_in() {
    use izanagi_kit::easing::*;
    type E = fn(Fixed) -> Fixed;
    let pairs: &[(&str, E, E)] = &[
        ("quad", ease_in_quad, ease_out_quad),
        ("cubic", ease_in_cubic, ease_out_cubic),
        ("quart", ease_in_quart, ease_out_quart),
        ("quint", ease_in_quint, ease_out_quint),
        ("sine", ease_in_sine, ease_out_sine),
        ("circ", ease_in_circ, ease_out_circ),
        ("expo", ease_in_expo, ease_out_expo),
    ];
    let to_f = |x: Fixed| x.raw() as f64 / 65536.0;
    let mut rng = SplitMix64::new(0x0EA5_BAC6);
    const EPS: f64 = 1.5e-2;
    for _ in 0..ITERS {
        let t = Fixed::from_ratio(rng.below(0x1_0001) as i32, 0x1_0000);
        for (name, ein, eout) in pairs {
            let direct = to_f(eout(t));
            let reflected = to_f(ease_reversed(t, *ein));
            assert!(
                (direct - reflected).abs() < EPS,
                "{name}: out({})={direct} != 1-in(1-t)={reflected}",
                to_f(t)
            );
        }
    }
}

/// **lerp endpoints, bounds and reflection** — `lerp(a, b, t) = a + (b−a)·t`
/// must pin `t=0→a` and `t=1→b` exactly, stay within `[min(a,b), max(a,b)]` for
/// `t ∈ [0,1]`, and satisfy the reflection identity `lerp(a,b,t) = lerp(b,a,1−t)`.
/// These are the contracts every tween/animation built on `lerp` depends on.
#[test]
fn prop_easing_lerp_endpoints_bounds_and_reflection() {
    use izanagi_kit::easing::lerp;
    let mut rng = SplitMix64::new(0x0EA5_1E66);
    for _ in 0..ITERS {
        let a = rand_fixed(&mut rng);
        let b = rand_fixed(&mut rng);
        assert_eq!(lerp(a, b, Fixed::ZERO), a, "lerp(_,_,0) must equal a");
        assert_eq!(lerp(a, b, Fixed::ONE), b, "lerp(_,_,1) must equal b");

        let t = Fixed::from_ratio(rng.below(0x1_0001) as i32, 0x1_0000); // [0,1]
        let v = lerp(a, b, t);
        let (lo, hi) = (a.min(b), a.max(b));
        assert!(v >= lo && v <= hi, "lerp out of [{lo:?},{hi:?}]: {v:?}");

        // Reflection: lerp(a,b,t) == lerp(b,a,1-t), exact under Q16.16 arithmetic.
        assert_eq!(
            lerp(a, b, t),
            lerp(b, a, Fixed::ONE - t),
            "lerp reflection identity failed for a={a:?} b={b:?} t={t:?}"
        );
    }
}

// ── WFC properties ────────────────────────────────────────────────────────────

/// Helper: check that every adjacent collapsed-cell pair in `grid` is permitted
/// by `rules`. Returns `false` at the first violation found. Used by both the
/// adjacency-invariant property test and the fault-injection test below.
fn adjacency_invariant_holds(
    grid: &izanagi_kit::wfc::WfcGrid,
    rules: &izanagi_kit::wfc::WfcRules,
) -> bool {
    // (dx, dy, dir_index) — dir 0=N, 1=E, 2=S, 3=W
    const DIRS: [(i32, i32, usize); 4] = [(0, -1, 0), (1, 0, 1), (0, 1, 2), (-1, 0, 3)];
    for y in 0..grid.height {
        for x in 0..grid.width {
            let Some(tile) = grid.tile_at(x, y) else {
                continue;
            };
            for (dx, dy, dir) in DIRS {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= grid.width || ny >= grid.height {
                    continue;
                }
                let Some(nb) = grid.tile_at(nx, ny) else {
                    continue;
                };
                if rules.get_allowed(tile, dir) & (1u64 << nb) == 0 {
                    return false;
                }
            }
        }
    }
    true
}

/// WFC **adjacency invariant** — the core correctness claim of constraint
/// propagation: whenever `wfc_solve` returns `Ok(grid)`, every pair of adjacent
/// fully-collapsed cells has tiles permitted by the rules. A bug in the
/// propagation step (e.g. a wrong bitmask operation, an off-by-one in the
/// direction indexing) would produce a grid that violates this law.
///
/// The test also verifies it is non-vacuous: at least 40 successful solves must
/// occur in 500 trials, so the invariant is actually exercised, not just skipped
/// due to systematic contradiction.
#[test]
fn prop_wfc_solved_grid_respects_adjacency_rules() {
    use izanagi_kit::wfc::{WfcResult, WfcRules};
    let mut rng = SplitMix64::new(0x0CF1_4D05);
    let mut solved = 0usize;

    for _ in 0..500 {
        // Random tile count [2..=5] — enough variety to exercise constraint
        // propagation paths without making contradictions too common.
        let tc = 2 + rng.below(4) as u8;
        let mut rules = WfcRules::new(tc);

        // Start fully-permissive (all tiles allowed in every direction), then
        // randomly remove ~33% of adjacencies.  Starting from fully-permissive
        // guarantees at least one valid grid exists before removals; removing
        // a fraction adds real constraint-propagation work while keeping
        // the solve-success rate high enough for a non-vacuous test.
        for tile in 0..tc {
            for dir in 0..4 {
                for nb in 0..tc {
                    rules.allow(tile, dir, nb);
                }
            }
        }
        for tile in 0..tc {
            for dir in 0..4 {
                for nb in 0..tc {
                    if rng.below(3) == 0 {
                        rules.disallow(tile, dir, nb);
                    }
                }
                // Restore at least one allowed neighbor per (tile, dir) so the
                // rules are never self-contradictory going into the solve.
                if rules.allowed_count(tile, dir) == 0 {
                    rules.allow(tile, dir, rng.below(tc as u32) as u8);
                }
            }
        }

        // Small grids. Use the backtrack solver so the invariant is exercised
        // even with tightly constrained rules — the invariant must hold
        // regardless of whether the solution was found on the first collapse
        // sequence or required backtracking.
        let w = 3 + rng.below(5) as i32;
        let h = 3 + rng.below(5) as i32;
        let seed = (rng.below(0x7FFF_FFFF) as u64) << 32 | rng.below(0x7FFF_FFFF) as u64 | 1;
        if let WfcResult::Ok(grid) =
            wfc_solve_backtrack(w, h, &rules, &mut SplitMix64::new(seed), 200)
        {
            solved += 1;
            assert!(
                grid.is_fully_collapsed(),
                "WfcResult::Ok must be fully collapsed"
            );
            assert!(
                adjacency_invariant_holds(&grid, &rules),
                "solved grid violates adjacency rules (w={w}, h={h}, tc={tc})"
            );
        }
    }

    assert!(
        solved >= 50,
        "expected ≥50 successful solves for non-vacuous coverage, got {solved}"
    );
}

/// WFC **determinism** — the module doc's headline guarantee: given identical
/// rules, dimensions, and RNG seed, `wfc_solve` always produces the same
/// `WfcResult`. Any source of non-determinism (HashMap iteration, OS entropy,
/// float rounding) would violate this across runs.
///
/// Fault-injection proof: running two identical solves from fresh `SplitMix64`
/// instances seeded identically and asserting hash equality would detect any
/// single-bit divergence in the output.
#[test]
fn prop_wfc_deterministic_same_seed_same_result() {
    use izanagi_kit::wfc::{wfc_solve, WfcResult, WfcRules};
    use izanagi_kit::hash_state;
    let mut rng = SplitMix64::new(0x07B4_C105);

    for _ in 0..500 {
        let tc = 2 + rng.below(5) as u8;
        let mut rules = WfcRules::new(tc);
        // Same permissive-then-remove approach as the adjacency test.
        for tile in 0..tc {
            for dir in 0..4 {
                for nb in 0..tc {
                    rules.allow(tile, dir, nb);
                }
            }
        }
        for tile in 0..tc {
            for dir in 0..4 {
                for nb in 0..tc {
                    if rng.below(3) == 0 {
                        rules.disallow(tile, dir, nb);
                    }
                }
                if rules.allowed_count(tile, dir) == 0 {
                    rules.allow(tile, dir, rng.below(tc as u32) as u8);
                }
            }
        }
        let w = 3 + rng.below(5) as i32;
        let h = 3 + rng.below(5) as i32;
        let seed = (rng.below(0x7FFF_FFFF) as u64) << 32 | rng.below(0x7FFF_FFFF) as u64 | 1;

        // Two fresh RNG instances from the same seed — result must be identical.
        let result_a = wfc_solve(w, h, &rules, &mut SplitMix64::new(seed));
        let result_b = wfc_solve(w, h, &rules, &mut SplitMix64::new(seed));

        match (result_a, result_b) {
            (WfcResult::Contradiction, WfcResult::Contradiction) => {}
            (WfcResult::Ok(a), WfcResult::Ok(b)) => {
                assert_eq!(
                    hash_state(&a),
                    hash_state(&b),
                    "WFC not deterministic (w={w}, h={h}, seed={seed:#x})"
                );
            }
            _ => panic!(
                "WFC gave different result types for identical inputs (w={w}, h={h}, seed={seed:#x})"
            ),
        }
    }
}

// ── savefile round-trip / fault-injection properties ─────────────────────────

/// **Round-trip identity** — for any payload bytes and version number,
/// `load_bytes(save_bytes(header, payload))` always returns the original
/// payload verbatim and the original version. Verifies 3000 (version, payload)
/// pairs generated by SplitMix64.
#[test]
fn prop_savefile_roundtrip_is_identity() {
    use izanagi_kit::savefile::{load_bytes, save_bytes, SaveHeader};
    let mut rng = SplitMix64::new(0x0F1A_2B3C);
    for _ in 0..ITERS {
        let version = rng.below(u32::MAX);
        let len = rng.below(256) as usize;
        let payload: Vec<u8> = (0..len).map(|_| rng.below(256) as u8).collect();
        let header = SaveHeader::new(version);
        let saved = save_bytes(&header, &payload);
        match load_bytes(&saved) {
            Ok((h, p)) => {
                assert_eq!(h.version, version, "version not preserved");
                assert_eq!(p, payload.as_slice(), "payload not preserved");
            }
            Err(e) => panic!("round-trip failed for version={version}, len={len}: {e:?}"),
        }
    }
}

/// **Size law** — `save_bytes` always produces exactly `20 + payload.len()`
/// bytes. Any deviation means the header or payload layout changed.
#[test]
fn prop_savefile_size_is_always_20_plus_payload() {
    use izanagi_kit::savefile::{estimate_save_size, save_bytes, SaveHeader};
    let mut rng = SplitMix64::new(0x1122_3344);
    for _ in 0..ITERS {
        let len = rng.below(256) as usize;
        let payload: Vec<u8> = (0..len).map(|_| rng.below(256) as u8).collect();
        let saved = save_bytes(&SaveHeader::new(0), &payload);
        assert_eq!(saved.len(), 20 + len, "size law violated for len={len}");
        assert_eq!(
            saved.len(),
            estimate_save_size(len),
            "estimate_save_size disagrees with actual size for len={len}"
        );
    }
}

/// **Truncation rejection** — truncating any valid save file to fewer than its
/// declared extent must always return `TooShort`, never `Ok`. Exercises the
/// boundary guard that prevents both short-header and short-payload reads.
#[test]
fn prop_savefile_truncation_always_rejected() {
    use izanagi_kit::savefile::{load_bytes, save_bytes, LoadError, SaveHeader};
    let mut rng = SplitMix64::new(0xDEAD_C0DE);
    for _ in 0..ITERS {
        let len = 1 + rng.below(127) as usize; // always at least 1-byte payload
        let payload: Vec<u8> = (0..len).map(|_| rng.below(256) as u8).collect();
        let saved = save_bytes(&SaveHeader::new(1), &payload);

        // Truncate to any length shorter than the full buffer.
        let cut = rng.below(saved.len() as u32) as usize;
        match load_bytes(&saved[..cut]) {
            Err(LoadError::TooShort) => {} // expected
            Err(LoadError::ChecksumMismatch) => {} // also acceptable (partial payload)
            Err(other) => panic!("unexpected error for cut={cut}: {other:?}"),
            Ok(_) => panic!("truncated save accepted at cut={cut}"),
        }
    }
}

/// **Payload fault injection** — flipping any single byte in the payload
/// section of a valid save buffer must always produce `ChecksumMismatch`.
/// FNV-1a has 2^-64 collision probability per flip; over 3000 × payload
/// trials no collision is expected.
#[test]
fn prop_savefile_payload_corruption_always_detected() {
    use izanagi_kit::savefile::{load_bytes, save_bytes, LoadError, SaveHeader};
    let mut rng = SplitMix64::new(0xCAFE_BABE);
    let mut checked = 0usize;
    for _ in 0..ITERS {
        let len = 1 + rng.below(127) as usize;
        let payload: Vec<u8> = (0..len).map(|_| rng.below(256) as u8).collect();
        let mut saved = save_bytes(&SaveHeader::new(7), &payload);

        // Flip one byte in the payload region (bytes 20..).
        let pos = 20 + rng.below(len as u32) as usize;
        saved[pos] ^= 0xFF;

        assert_eq!(
            load_bytes(&saved),
            Err(LoadError::ChecksumMismatch),
            "payload corruption not detected at byte {pos}"
        );
        checked += 1;
    }
    assert_eq!(checked, ITERS, "all payload fault trials must complete");
}

/// **Checksum-field fault injection** — flipping any byte in the 8-byte
/// stored-checksum field (bytes 8-15) must always produce `ChecksumMismatch`,
/// since the stored hash will no longer match the hash of the unmodified payload.
#[test]
fn prop_savefile_checksum_field_corruption_always_detected() {
    use izanagi_kit::savefile::{load_bytes, save_bytes, LoadError, SaveHeader};
    let mut rng = SplitMix64::new(0xFACE_FEED);
    for _ in 0..ITERS {
        let len = rng.below(128) as usize;
        let payload: Vec<u8> = (0..len).map(|_| rng.below(256) as u8).collect();
        let mut saved = save_bytes(&SaveHeader::new(3), &payload);

        // Flip one byte in the stored checksum (bytes 8-15).
        let pos = 8 + rng.below(8) as usize;
        saved[pos] ^= 0xFF;

        assert_eq!(
            load_bytes(&saved),
            Err(LoadError::ChecksumMismatch),
            "checksum-field corruption not detected at byte {pos}"
        );
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

// ── Scheduler (turn order) properties ────────────────────────────────────────

/// **Insertion-order independence** — two `Scheduler` instances with the same
/// actors at the same speeds but added in a different (random) order must
/// produce the same turn sequence. The `det_hash` sorts by id to make the hash
/// insertion-independent; this test proves the turn ORDER itself is also
/// insertion-order-independent (only energy and id tie-break matter, not the
/// slot position in the internal vec). A failure here would silently desync
/// replays whenever actors are registered in a different order.
#[test]
fn prop_scheduler_turn_order_is_insertion_order_independent() {
    let mut rng = SplitMix64::new(0x0D3B_4C05);
    for _ in 0..ITERS {
        let n = 2 + rng.below(5) as usize; // 2..=6 actors
        let ids: Vec<u32> = (0..n as u32).collect();
        let speeds: Vec<i32> = (0..n)
            .map(|_| ACTION_COST + rng.below(ACTION_COST as u32) as i32)
            .collect();

        // Build a permuted insertion order for scheduler B.
        let mut order: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            let j = rng.below(i as u32 + 1) as usize;
            order.swap(i, j);
        }

        let mut sched_a: Scheduler<u32> = Scheduler::new();
        let mut sched_b: Scheduler<u32> = Scheduler::new();
        for k in 0..n {
            sched_a.add(ids[k], speeds[k]);
        }
        for &k in &order {
            sched_b.add(ids[k], speeds[k]);
        }

        // Both must produce identical turn sequences for the next 50 turns.
        for step in 0..50usize {
            let ta = sched_a.next_turn();
            let tb = sched_b.next_turn();
            assert_eq!(
                ta, tb,
                "turn order diverged at step {step}: {ta:?} vs {tb:?} (n={n})"
            );
        }
    }
}

/// **Proportional fairness** — over a long run, an actor with speed `2 ×
/// ACTION_COST` acts at least 1.8× as often as an actor with speed
/// `ACTION_COST`. The exact ratio is 2:1 for whole-multiple speeds; the
/// tolerance ±10 % accommodates finite-run effects without hiding real bugs.
#[test]
fn prop_scheduler_faster_actor_acts_proportionally_more() {
    let mut rng = SplitMix64::new(0x04F8_C305);
    for _ in 0..200 {
        // Use random integer multiples [2..=4] of ACTION_COST for the fast
        // actor so the expected ratio is exact.
        let mult = 2 + rng.below(3); // 2, 3, or 4
        let mut sched: Scheduler<u32> = Scheduler::new();
        sched.add(0, ACTION_COST);
        sched.add(1, ACTION_COST * mult as i32);

        let n_turns = 500usize;
        let (mut slow_count, mut fast_count) = (0u32, 0u32);
        for _ in 0..n_turns {
            match sched.next_turn().unwrap() {
                0 => slow_count += 1,
                1 => fast_count += 1,
                _ => unreachable!(),
            }
        }

        // Fast : slow must be at least (mult - 0.2) : 1.
        let ratio_lo = (mult as f64 - 0.2) * slow_count as f64;
        assert!(
            fast_count as f64 >= ratio_lo,
            "fast actor underperformed: {fast_count} turns vs {slow_count} slow (mult={mult})"
        );
    }
}

/// **`peek_next_turn` preview fidelity** — when `peek_next_turn` returns
/// `Some(a)` (an actor is already ready with energy ≥ ACTION_COST), the
/// immediately following `next_turn` must return the same `Some(a)`. If peek
/// returns `None`, no actor is currently ready and time must be advanced by
/// `next_turn` — so a `None` peek preceding a `Some` advance is correct and
/// not an invariant violation. Any divergence in the `Some`/`Some` case means
/// the tie-breaking or energy-read logic disagrees between the two code paths.
#[test]
fn prop_scheduler_peek_when_some_matches_next_turn() {
    let mut rng = SplitMix64::new(0x07E4_B005);
    for _ in 0..ITERS {
        let n = 1 + rng.below(6) as usize;
        let mut sched: Scheduler<u32> = Scheduler::new();
        for k in 0..n {
            sched.add(k as u32, ACTION_COST + rng.below(ACTION_COST as u32) as i32);
        }
        for _ in 0..20 {
            let peeked = sched.peek_next_turn();
            let advanced = sched.next_turn();
            // The invariant: if someone is already ready, peek and advance agree.
            // When peek is None (nobody ready yet), next_turn advances time —
            // returning Some after time-skip is correct, not a bug.
            if let Some(p) = peeked {
                assert_eq!(
                    Some(p),
                    advanced,
                    "peek returned Some({p}) but next_turn returned {advanced:?}"
                );
            }
        }
    }
}

// ── FOV (field of view) properties ───────────────────────────────────────────

/// **FOV symmetry** — the module's headline guarantee: "A sees B iff B sees A."
/// For every visible non-wall cell B in A's FOV, compute FOV from B and verify
/// that A appears in it. Tests 400 random (wall map, origin, radius) triples on
/// 20×20 grids. A failure here means the symmetric-shadowcasting invariant is
/// broken — the classic source of "the goblin can see you but you can't see it"
/// bugs.
#[test]
fn prop_fov_is_symmetric() {
    const W: i32 = 20;
    const H: i32 = 20;
    let mut rng = SplitMix64::new(0x0F04_C005);

    let visible_from = |walls: &[bool], ox: i32, oy: i32, radius: i32| -> Vec<bool> {
        let mut vis = vec![false; (W * H) as usize];
        compute_fov(
            (ox, oy),
            radius,
            |x, y| {
                if x < 0 || y < 0 || x >= W || y >= H {
                    true // off-map is opaque
                } else {
                    walls[(y * W + x) as usize]
                }
            },
            |x, y| {
                if x >= 0 && y >= 0 && x < W && y < H {
                    vis[(y * W + x) as usize] = true;
                }
            },
        );
        vis
    };

    for _ in 0..400 {
        // Random wall map (~25 % wall density).
        let walls: Vec<bool> = (0..W * H).map(|_| rng.below(4) == 0).collect();

        // Random origin that is not a wall.
        let ox = rng.below(W as u32) as i32;
        let oy = rng.below(H as u32) as i32;
        if walls[(oy * W + ox) as usize] {
            continue; // skip — origin is a wall (uncommon but legal to skip)
        }

        let radius = 3 + rng.below(10) as i32;
        let vis_a = visible_from(&walls, ox, oy, radius);

        // For every floor cell visible from A, verify A is visible from it.
        for y in 0..H {
            for x in 0..W {
                let idx = (y * W + x) as usize;
                if !vis_a[idx] || walls[idx] {
                    continue;
                }
                if x == ox && y == oy {
                    continue; // origin sees itself trivially
                }
                let vis_b = visible_from(&walls, x, y, radius);
                assert!(
                    vis_b[(oy * W + ox) as usize],
                    "FOV asymmetry: A=({ox},{oy}) can see B=({x},{y}) but not vice versa \
                     (radius={radius})"
                );
            }
        }
    }
}

/// **FOV origin always visible** — `compute_fov` always invokes `mark_visible`
/// for the origin cell regardless of radius, wall configuration, or coordinate.
/// A radius-0 call must also mark the origin (and nothing else in a walled grid).
#[test]
fn prop_fov_origin_is_always_visible() {
    const W: i32 = 15;
    const H: i32 = 15;
    let mut rng = SplitMix64::new(0x04E3_A005);
    for _ in 0..ITERS {
        let walls: Vec<bool> = (0..W * H).map(|_| rng.below(4) == 0).collect();
        let ox = rng.below(W as u32) as i32;
        let oy = rng.below(H as u32) as i32;
        let radius = rng.below(12) as i32;

        let mut origin_marked = false;
        compute_fov(
            (ox, oy),
            radius,
            |x, y| {
                if x < 0 || y < 0 || x >= W || y >= H {
                    true
                } else {
                    walls[(y * W + x) as usize]
                }
            },
            |x, y| {
                if x == ox && y == oy {
                    origin_marked = true;
                }
            },
        );
        assert!(
            origin_marked,
            "origin ({ox},{oy}) not marked visible (radius={radius})"
        );
    }
}

/// **FOV radius constraint** — every cell reported by `compute_fov` must lie
/// within Euclidean distance `radius` from the origin (`dx²+dy² ≤ radius²`).
/// The symmetry and radius guard together ensure the revealed area is always
/// a bounded, consistent disc around the observer.
#[test]
fn prop_fov_all_visible_cells_are_within_radius() {
    const W: i32 = 20;
    const H: i32 = 20;
    let mut rng = SplitMix64::new(0x0C45_B005);
    for _ in 0..ITERS {
        let walls: Vec<bool> = (0..W * H).map(|_| rng.below(5) == 0).collect();
        let ox = rng.below(W as u32) as i32;
        let oy = rng.below(H as u32) as i32;
        let radius = rng.below(10) as i32;
        let r2 = (radius as i64) * (radius as i64);

        compute_fov(
            (ox, oy),
            radius,
            |x, y| {
                if x < 0 || y < 0 || x >= W || y >= H {
                    true
                } else {
                    walls[(y * W + x) as usize]
                }
            },
            |x, y| {
                let dx = (x - ox) as i64;
                let dy = (y - oy) as i64;
                let dsq = dx * dx + dy * dy;
                assert!(
                    dsq <= r2,
                    "cell ({x},{y}) is outside radius {radius} of origin ({ox},{oy}): dsq={dsq}"
                );
            },
        );
    }
}

// ── Pathfinding properties ────────────────────────────────────────────────────

const PATH_W: i32 = 16;
const PATH_H: i32 = 16;

/// Grid wall-query closure for property tests: off-map cells are opaque (bounds
/// the search), on-map cells use the provided `walls` slice.
fn path_is_blocked<'a>(walls: &'a [bool]) -> impl FnMut(i32, i32) -> bool + 'a {
    move |x: i32, y: i32| {
        if x < 0 || y < 0 || x >= PATH_W || y >= PATH_H {
            true
        } else {
            walls[(y * PATH_W + x) as usize]
        }
    }
}

/// Cost of an 8-directional path: 10 per orthogonal step, 14 per diagonal.
fn path_cost_manual(path: &[(i32, i32)]) -> i32 {
    path.windows(2)
        .map(|w| {
            let (dx, dy) = ((w[1].0 - w[0].0).abs(), (w[1].1 - w[0].1).abs());
            if dx == 1 && dy == 1 {
                14
            } else {
                10
            }
        })
        .sum()
}

/// **A* path validity** — for every path returned by `astar`:
/// 1. `path[0] == start` and `path[last] == goal`
/// 2. Each consecutive step is a single king move (no teleportation)
/// 3. No step lands on a wall cell
/// 4. No diagonal step cuts a wall corner
///
/// Tested over 500 random (wall map, start, goal) triples on 16×16 grids.
/// A single failure means the reconstructed path is geometrically impossible.
#[test]
fn prop_astar_path_is_valid() {
    let mut rng = SplitMix64::new(0x0A57_C005);
    let mut found = 0usize;

    for _ in 0..500 {
        let walls: Vec<bool> = (0..PATH_W * PATH_H)
            .map(|_| rng.below(5) == 0)
            .collect();
        let sx = rng.below(PATH_W as u32) as i32;
        let sy = rng.below(PATH_H as u32) as i32;
        let gx = rng.below(PATH_W as u32) as i32;
        let gy = rng.below(PATH_H as u32) as i32;
        if walls[(sy * PATH_W + sx) as usize] || walls[(gy * PATH_W + gx) as usize] {
            continue;
        }
        let Some(path) = astar((sx, sy), (gx, gy), path_is_blocked(&walls)) else {
            continue;
        };
        found += 1;

        assert_eq!(path[0], (sx, sy), "path must start at start");
        assert_eq!(*path.last().unwrap(), (gx, gy), "path must end at goal");

        for w in path.windows(2) {
            let (dx, dy) = ((w[1].0 - w[0].0).abs(), (w[1].1 - w[0].1).abs());
            // Each step is a king move: |dx|,|dy| ∈ {0,1}, not both 0.
            assert!(
                dx <= 1 && dy <= 1 && (dx + dy) > 0,
                "non-king step {:?} → {:?}",
                w[0],
                w[1]
            );
            // No step lands in a wall.
            assert!(
                !walls[(w[1].1 * PATH_W + w[1].0) as usize],
                "path steps into wall at {:?}",
                w[1]
            );
            // Diagonal steps must not cut a corner.
            if dx == 1 && dy == 1 {
                let h_blocked = walls[(w[0].1 * PATH_W + w[1].0) as usize];
                let v_blocked = walls[(w[1].1 * PATH_W + w[0].0) as usize];
                assert!(
                    !h_blocked && !v_blocked,
                    "diagonal step {:?}→{:?} cuts a wall corner",
                    w[0],
                    w[1]
                );
            }
        }
    }

    // Non-vacuous: at least 100 paths must have been found and validated.
    assert!(found >= 100, "expected ≥100 paths found, got {found}");
}

/// **Determinism** — same start, goal, and wall map always produces identical
/// paths. Any HashMap-iteration non-determinism would make paths differ between
/// calls and would desync AI paths in replays.
#[test]
fn prop_astar_is_deterministic() {
    let mut rng = SplitMix64::new(0x0B68_D005);
    for _ in 0..500 {
        let walls: Vec<bool> = (0..PATH_W * PATH_H)
            .map(|_| rng.below(5) == 0)
            .collect();
        let sx = rng.below(PATH_W as u32) as i32;
        let sy = rng.below(PATH_H as u32) as i32;
        let gx = rng.below(PATH_W as u32) as i32;
        let gy = rng.below(PATH_H as u32) as i32;

        let path_a = astar((sx, sy), (gx, gy), path_is_blocked(&walls));
        let path_b = astar((sx, sy), (gx, gy), path_is_blocked(&walls));
        assert_eq!(path_a, path_b, "astar not deterministic for ({sx},{sy})→({gx},{gy})");
    }
}

/// **Weighted A* bound** — with integer weight `w ≥ 1`, the path returned by
/// `weighted_astar` has cost ≤ `w × astar_cost`. Also verifies that at
/// `weight == 1`, `weighted_astar` agrees exactly with `astar`.
#[test]
fn prop_weighted_astar_cost_bound_and_unity() {
    let mut rng = SplitMix64::new(0x0C79_E005);
    let mut checked = 0usize;

    for _ in 0..500 {
        let walls: Vec<bool> = (0..PATH_W * PATH_H)
            .map(|_| rng.below(5) == 0)
            .collect();
        let sx = rng.below(PATH_W as u32) as i32;
        let sy = rng.below(PATH_H as u32) as i32;
        let gx = rng.below(PATH_W as u32) as i32;
        let gy = rng.below(PATH_H as u32) as i32;
        if walls[(sy * PATH_W + sx) as usize] || walls[(gy * PATH_W + gx) as usize] {
            continue;
        }

        let opt = astar((sx, sy), (gx, gy), path_is_blocked(&walls));
        let w1 = weighted_astar((sx, sy), (gx, gy), path_is_blocked(&walls), 1);

        // weight=1 must produce the same path and cost as plain astar.
        assert_eq!(opt, w1, "weighted_astar(1) != astar for ({sx},{sy})→({gx},{gy})");

        let Some(opt_path) = opt else {
            continue;
        };
        let opt_cost = path_cost_manual(&opt_path);
        let w = 1 + rng.below(3); // 1..=3

        if let Some(wp) = weighted_astar((sx, sy), (gx, gy), path_is_blocked(&walls), w) {
            let wp_cost = path_cost_manual(&wp);
            assert!(
                wp_cost <= w as i32 * opt_cost,
                "weighted_astar(w={w}) cost {wp_cost} > {w}×{opt_cost} for ({sx},{sy})→({gx},{gy})"
            );
            checked += 1;
        }
    }

    assert!(checked >= 50, "expected ≥50 weighted paths checked, got {checked}");
}

/// `Fn` (not `FnMut`) wall-query closure for `jps`, whose jump recursion queries
/// cells reentrantly. Same semantics as [`path_is_blocked`].
fn path_is_blocked_fn(walls: &[bool]) -> impl Fn(i32, i32) -> bool + '_ {
    move |x: i32, y: i32| {
        if x < 0 || y < 0 || x >= PATH_W || y >= PATH_H {
            true
        } else {
            walls[(y * PATH_W + x) as usize]
        }
    }
}

/// **JPS ≡ A\*** — Jump Point Search is an *exact* optimisation, so over random
/// grids it must agree with `astar` on reachability and, when a path exists,
/// return one of identical cost. Every JPS path must also be a legal king-move
/// route with no wall steps and no diagonal corner-cuts. `astar` is the trusted
/// oracle; any forced-neighbour or pruning bug shows up as a cost/None mismatch.
#[test]
fn prop_jps_matches_astar_cost_and_is_valid() {
    let mut rng = SplitMix64::new(0x0DABC_005);
    let mut reachable = 0usize;
    for _ in 0..500 {
        let walls: Vec<bool> = (0..PATH_W * PATH_H).map(|_| rng.below(5) == 0).collect();
        let sx = rng.below(PATH_W as u32) as i32;
        let sy = rng.below(PATH_H as u32) as i32;
        let gx = rng.below(PATH_W as u32) as i32;
        let gy = rng.below(PATH_H as u32) as i32;
        if walls[(sy * PATH_W + sx) as usize] || walls[(gy * PATH_W + gx) as usize] {
            continue;
        }

        let a = astar((sx, sy), (gx, gy), path_is_blocked(&walls));
        let j = jps((sx, sy), (gx, gy), path_is_blocked_fn(&walls));

        assert_eq!(
            a.is_some(),
            j.is_some(),
            "JPS/A* reachability mismatch ({sx},{sy})→({gx},{gy})"
        );
        let (Some(ap), Some(jp)) = (a, j) else {
            continue;
        };
        reachable += 1;
        assert_eq!(
            path_cost_manual(&ap),
            path_cost_manual(&jp),
            "JPS cost {} != A* cost {} for ({sx},{sy})→({gx},{gy})",
            path_cost_manual(&jp),
            path_cost_manual(&ap)
        );
        // JPS path must be a legal, corner-safe king-move route.
        assert_eq!(jp[0], (sx, sy), "JPS path must start at start");
        assert_eq!(*jp.last().unwrap(), (gx, gy), "JPS path must end at goal");
        for w in jp.windows(2) {
            let (dx, dy) = ((w[1].0 - w[0].0).abs(), (w[1].1 - w[0].1).abs());
            assert!(dx <= 1 && dy <= 1 && (dx + dy) > 0, "non-king JPS step");
            assert!(
                !walls[(w[1].1 * PATH_W + w[1].0) as usize],
                "JPS steps into wall at {:?}",
                w[1]
            );
            if dx == 1 && dy == 1 {
                let h_blocked = walls[(w[0].1 * PATH_W + w[1].0) as usize];
                let v_blocked = walls[(w[1].1 * PATH_W + w[0].0) as usize];
                assert!(!h_blocked && !v_blocked, "JPS cut a wall corner");
            }
        }
    }
    assert!(reachable >= 100, "expected ≥100 reachable pairs, got {reachable}");
}

/// **JPS determinism** — identical start, goal, and wall map always yield the
/// identical path. Any HashMap-iteration leak would desync AI routes in replays.
#[test]
fn prop_jps_is_deterministic() {
    let mut rng = SplitMix64::new(0x0DDE7_005);
    for _ in 0..500 {
        let walls: Vec<bool> = (0..PATH_W * PATH_H).map(|_| rng.below(5) == 0).collect();
        let sx = rng.below(PATH_W as u32) as i32;
        let sy = rng.below(PATH_H as u32) as i32;
        let gx = rng.below(PATH_W as u32) as i32;
        let gy = rng.below(PATH_H as u32) as i32;
        let a = jps((sx, sy), (gx, gy), path_is_blocked_fn(&walls));
        let b = jps((sx, sy), (gx, gy), path_is_blocked_fn(&walls));
        assert_eq!(a, b, "jps not deterministic for ({sx},{sy})→({gx},{gy})");
    }
}

/// **Dijkstra-map / descend monotonicity** — every cell in a Dijkstra map has a
/// cost in `[0, max_cost]`, the source is `0`, and steepest `descend` from any
/// mapped cell strictly decreases cost on each step and terminates at a source
/// (cost `0`). This is the contract chase/flee AI relies on (no cycles, always
/// reaches the goal).
#[test]
fn prop_dijkstra_descend_is_monotone_to_source() {
    let mut rng = SplitMix64::new(0x0D1357_05);
    let mut descents = 0usize;
    for _ in 0..400 {
        let walls: Vec<bool> = (0..PATH_W * PATH_H).map(|_| rng.below(6) == 0).collect();
        let sx = rng.below(PATH_W as u32) as i32;
        let sy = rng.below(PATH_H as u32) as i32;
        if walls[(sy * PATH_W + sx) as usize] {
            continue;
        }
        let max_cost = 10_000;
        let map = dijkstra_map(&[(sx, sy)], max_cost, path_is_blocked(&walls));
        assert_eq!(map.get(&(sx, sy)), Some(&0), "source must have cost 0");
        for (&(_, _), &c) in map.iter() {
            assert!(c >= 0 && c <= max_cost, "cost {c} out of [0,{max_cost}]");
        }
        // Descend from a few random mapped cells.
        let cells: Vec<(i32, i32)> = map.keys().copied().collect();
        if cells.is_empty() {
            continue;
        }
        for _ in 0..3 {
            let mut cur = cells[rng.below(cells.len() as u32) as usize];
            let mut last = map[&cur];
            let mut steps = 0;
            while let Some(next) = descend(&map, cur, path_is_blocked(&walls)) {
                assert!(map[&next] < last, "descend did not strictly decrease cost");
                last = map[&next];
                cur = next;
                steps += 1;
                assert!(steps < PATH_W * PATH_H, "descend must terminate");
            }
            assert_eq!(map[&cur], 0, "descend must end at a source");
            descents += 1;
        }
    }
    assert!(descents >= 100, "expected ≥100 descents, got {descents}");
}

/// **Octile distance is a metric** — `octile_distance` satisfies identity
/// (`d(a,a)=0`), symmetry (`d(a,b)=d(b,a)`), non-negativity, and the triangle
/// inequality `d(a,c) ≤ d(a,b)+d(b,c)`. These are what make it an admissible,
/// consistent A* heuristic; a violation would break optimality guarantees.
#[test]
fn prop_octile_distance_is_a_metric() {
    let mut rng = SplitMix64::new(0x0C71_1E05);
    for _ in 0..ITERS {
        let a = (rng.range(-200, 201), rng.range(-200, 201));
        let b = (rng.range(-200, 201), rng.range(-200, 201));
        let c = (rng.range(-200, 201), rng.range(-200, 201));
        assert_eq!(octile_distance(a, a), 0, "identity d(a,a)=0");
        assert!(octile_distance(a, b) >= 0, "non-negativity");
        assert_eq!(
            octile_distance(a, b),
            octile_distance(b, a),
            "symmetry d(a,b)=d(b,a)"
        );
        assert!(
            octile_distance(a, c) <= octile_distance(a, b) + octile_distance(b, c),
            "triangle inequality violated for {a:?},{b:?},{c:?}"
        );
    }
}

/// **smooth_path invariants** — string-pulling a valid A* path must (1) keep the
/// original start and goal and (2) leave every consecutive waypoint pair joined
/// by a straight Bresenham line with no blocked interior cell, and (3) never add
/// waypoints. So the smoothed route is still walkable and no longer than before.
#[test]
fn prop_smooth_path_preserves_endpoints_and_los() {
    let mut rng = SplitMix64::new(0x0577007_5);
    let mut smoothed = 0usize;
    for _ in 0..500 {
        let walls: Vec<bool> = (0..PATH_W * PATH_H).map(|_| rng.below(6) == 0).collect();
        let sx = rng.below(PATH_W as u32) as i32;
        let sy = rng.below(PATH_H as u32) as i32;
        let gx = rng.below(PATH_W as u32) as i32;
        let gy = rng.below(PATH_H as u32) as i32;
        if walls[(sy * PATH_W + sx) as usize] || walls[(gy * PATH_W + gx) as usize] {
            continue;
        }
        let Some(path) = astar((sx, sy), (gx, gy), path_is_blocked(&walls)) else {
            continue;
        };
        let sm = smooth_path(&path, path_is_blocked(&walls));
        assert_eq!(sm.first(), path.first(), "smooth must keep start");
        assert_eq!(sm.last(), path.last(), "smooth must keep goal");
        assert!(sm.len() <= path.len(), "smooth must not add waypoints");
        // Each smoothed segment's interior must be clear (Bresenham LOS).
        for w in sm.windows(2) {
            for &(x, y) in line(w[0], w[1]).iter() {
                assert!(
                    !(walls[(y * PATH_W + x) as usize]),
                    "smoothed segment {:?}→{:?} crosses wall at ({x},{y})",
                    w[0],
                    w[1]
                );
            }
        }
        smoothed += 1;
    }
    assert!(smoothed >= 100, "expected ≥100 smoothed paths, got {smoothed}");
}

// ── Dice properties ───────────────────────────────────────────────────────────

/// **Roll range invariant** — `roll()` must always produce a value in
/// `[min(), max()]`. A bug in the `rng.dice` accumulation or the modifier
/// arithmetic would violate this for some seed.
#[test]
fn prop_dice_roll_is_in_min_max_range() {
    let mut rng = SplitMix64::new(0x0D1C_E005);
    for _ in 0..ITERS {
        let count = rng.below(8) as u32; // 0..=7
        let sides = 1 + rng.below(20) as u32; // 1..=20
        let modifier = rng.range(-10, 11);
        let d = Dice::new(count, sides, modifier);

        let lo = d.min();
        let hi = d.max();
        assert!(lo <= hi, "min > max for {count}d{sides}{modifier:+}");

        let result = d.roll(&mut rng);
        assert!(
            result >= lo && result <= hi,
            "roll {result} outside [{lo},{hi}] for {count}d{sides}{modifier:+}"
        );
    }
}

/// **Advantage ≥ disadvantage** — `roll_advantage()` returns `max(a, b)` and
/// `roll_disadvantage()` returns `min(a, b)` from the SAME two draws. Since
/// the two dice objects are identical and draw from the same seeded stream,
/// `max(a, b) ≥ min(a, b)` is always true (no randomness affects this law).
#[test]
fn prop_dice_advantage_ge_disadvantage() {
    let mut rng = SplitMix64::new(0x0E2D_F005);
    for _ in 0..ITERS {
        let count = rng.below(5) as u32;
        let sides = 1 + rng.below(20) as u32;
        let modifier = rng.range(-5, 6);
        let d = Dice::new(count, sides, modifier);

        let mut rng_adv = rng.clone();
        let mut rng_dis = rng.clone();
        let adv = d.roll_advantage(&mut rng_adv);
        let dis = d.roll_disadvantage(&mut rng_dis);

        // Both consume two rolls from the same starting state.
        // max(a,b) ≥ min(a,b) unconditionally.
        assert!(
            adv >= dis,
            "advantage {adv} < disadvantage {dis} for {count}d{sides}{modifier:+}"
        );
        // Advance main rng past two rolls so each trial is independent.
        d.roll(&mut rng);
        d.roll(&mut rng);
    }
}

/// **min ≤ average ≤ max** — `average_x100()` must lie between `min()×100`
/// and `max()×100`. This is the defining property of an expected value for a
/// bounded distribution. A scaling or overflow bug in `average_x100` would
/// violate it for large count/sides.
#[test]
fn prop_dice_average_x100_is_between_min_and_max() {
    let mut rng = SplitMix64::new(0x0F3E_4005);
    for _ in 0..ITERS {
        let count = rng.below(16) as u32; // 0..=15
        let sides = 1 + rng.below(100) as u32; // 1..=100
        let modifier = rng.range(-100, 101);
        let d = Dice::new(count, sides, modifier);

        let avg = d.average_x100();
        let lo = d.min() as i64 * 100;
        let hi = d.max() as i64 * 100;

        assert!(
            avg >= lo && avg <= hi,
            "average_x100 {avg} outside [{lo},{hi}] for {count}d{sides}{modifier:+}"
        );
    }
}

// ── Mapgen (dungeon / cave / BSP) properties ──────────────────────────────────

/// Shared BFS 4-connectivity check: returns `true` when all floor cells in `d`
/// form a single connected region. Trivially `true` for 0- or 1-cell floors.
fn dungeon_is_connected(d: &Dungeon) -> bool {
    let floors = d.floor_cells();
    if floors.len() <= 1 {
        return true;
    }
    let w = d.width() as i32;
    let h = d.height() as i32;
    let idx = |x: i32, y: i32| (y * w + x) as usize;
    let mut visited = vec![false; (w * h) as usize];
    let start = floors[0];
    visited[idx(start.0, start.1)] = true;
    let mut queue = vec![start];
    let mut qi = 0;
    while qi < queue.len() {
        let (cx, cy) = queue[qi];
        qi += 1;
        for (dx, dy) in [(0i32, -1), (0, 1), (-1, 0), (1, 0)] {
            let (nx, ny) = (cx + dx, cy + dy);
            if nx >= 0 && ny >= 0 && nx < w && ny < h {
                let i = idx(nx, ny);
                if !visited[i] && d.is_floor(nx, ny) {
                    visited[i] = true;
                    queue.push((nx, ny));
                }
            }
        }
    }
    floors.iter().all(|&(x, y)| visited[idx(x, y)])
}

/// **Dungeon connectivity** — `generate_dungeon` wires every room to the
/// previous one with an L-shaped corridor, so the whole map forms one
/// 4-connected floor region. Verified over 200 (seed, size) pairs. A failure
/// means a room was placed without a connecting corridor or a corridor was
/// carved out of bounds.
#[test]
fn prop_generate_dungeon_is_fully_connected() {
    let mut rng = SplitMix64::new(0x0A4F_D005);
    let mut multi_room_count = 0usize;

    for _ in 0..200 {
        let seed = (rng.below(0x7FFF_FFFF) as u64) << 32 | rng.below(0x7FFF_FFFF) as u64 | 1;
        let w = 20 + rng.below(40) as u32;
        let h = 20 + rng.below(40) as u32;
        let dungeon = generate_dungeon(w, h, &mut SplitMix64::new(seed), GenParams::default());

        assert!(
            dungeon_is_connected(&dungeon),
            "dungeon not connected (seed={seed:#x}, w={w}, h={h}, rooms={})",
            dungeon.room_count()
        );

        if dungeon.room_count() >= 2 {
            multi_room_count += 1;
        }
    }

    assert!(
        multi_room_count >= 50,
        "expected ≥50 multi-room dungeons for non-vacuous coverage, got {multi_room_count}"
    );
}

/// **Dungeon determinism** — `generate_dungeon` is a pure function of
/// `(width, height, seed, params)`. Two calls with identical inputs must produce
/// identical maps. Any non-determinism (HashMap iteration, wall-clock reads)
/// would desync replays and violate this invariant.
#[test]
fn prop_generate_dungeon_is_deterministic() {
    use izanagi_kit::hash_state;
    let mut rng = SplitMix64::new(0x0B3E_C005);

    for _ in 0..200 {
        let seed = (rng.below(0x7FFF_FFFF) as u64) << 32 | rng.below(0x7FFF_FFFF) as u64 | 1;
        let w = 15 + rng.below(30) as u32;
        let h = 15 + rng.below(30) as u32;
        let params = GenParams::default();

        let a = generate_dungeon(w, h, &mut SplitMix64::new(seed), params);
        let b = generate_dungeon(w, h, &mut SplitMix64::new(seed), params);

        assert_eq!(
            hash_state(&a),
            hash_state(&b),
            "generate_dungeon not deterministic (seed={seed:#x}, w={w}, h={h})"
        );
    }
}

/// **Cave connectivity** — `generate_cave` ends every run with
/// `cull_to_largest_region`, which removes all floor cells outside the single
/// largest 4-connected component. The result is therefore always one connected
/// region (or all-wall). Verified over 200 (seed, size) triples.
#[test]
fn prop_generate_cave_is_fully_connected() {
    let mut rng = SplitMix64::new(0x0C5A_E005);
    let mut with_floor = 0usize;

    for _ in 0..200 {
        let seed = (rng.below(0x7FFF_FFFF) as u64) << 32 | rng.below(0x7FFF_FFFF) as u64 | 1;
        let w = 25 + rng.below(35) as u32;
        let h = 25 + rng.below(35) as u32;
        let dungeon = generate_cave(w, h, &mut SplitMix64::new(seed), CaveParams::default());

        assert!(
            dungeon_is_connected(&dungeon),
            "cave not connected (seed={seed:#x}, w={w}, h={h})"
        );

        if !dungeon.floor_cells().is_empty() {
            with_floor += 1;
        }
    }

    assert!(
        with_floor >= 100,
        "expected ≥100 caves with floor cells for non-vacuous coverage, got {with_floor}"
    );
}

/// **BSP dungeon connectivity** — `generate_bsp` joins each pair of child
/// partitions on the way back up the recursion tree, so the whole dungeon is
/// guaranteed connected. Verified over 200 (seed, size) pairs. A failure means
/// the corridor-stitching step in `bsp_build` broke connectivity.
#[test]
fn prop_generate_bsp_is_fully_connected() {
    let mut rng = SplitMix64::new(0x0D6B_F005);
    let mut with_floor = 0usize;

    for _ in 0..200 {
        let seed = (rng.below(0x7FFF_FFFF) as u64) << 32 | rng.below(0x7FFF_FFFF) as u64 | 1;
        let w = 25 + rng.below(35) as u32;
        let h = 25 + rng.below(35) as u32;
        let dungeon = generate_bsp(w, h, &mut SplitMix64::new(seed), BspParams::default());

        assert!(
            dungeon_is_connected(&dungeon),
            "BSP dungeon not connected (seed={seed:#x}, w={w}, h={h})"
        );

        if !dungeon.floor_cells().is_empty() {
            with_floor += 1;
        }
    }

    assert!(
        with_floor >= 100,
        "expected ≥100 BSP dungeons with floor cells for non-vacuous coverage, got {with_floor}"
    );
}

// ── Aabb (axis-aligned bounding box) properties ───────────────────────────────

/// Random `Aabb` with moderate coordinates (no saturation risk for small grows).
fn rand_aabb(rng: &mut SplitMix64) -> Aabb {
    let x = rng.range(-200, 201);
    let y = rng.range(-200, 201);
    let w = rng.range(0, 51);
    let h = rng.range(0, 51);
    Aabb::new(x, y, w, h)
}

/// **overlaps ↔ intersection.is_some()** — two independently-implemented
/// methods for "do these boxes share interior?" must always agree. An off-by-one
/// in either causes one to report overlap while the other doesn't.
/// Verified over 3000 random (a, b) pairs, including empty and touching boxes.
#[test]
fn prop_aabb_overlaps_iff_intersection_is_some() {
    let mut rng = SplitMix64::new(0x0E1A_B005);
    for _ in 0..ITERS {
        let a = rand_aabb(&mut rng);
        let b = rand_aabb(&mut rng);
        let ov = a.overlaps(&b);
        let ix = a.intersection(&b);
        assert_eq!(
            ov,
            ix.is_some(),
            "overlaps={ov} disagrees with intersection={ix:?} for a={a:?}, b={b:?}"
        );
    }
}

/// **overlaps and intersection are symmetric** — a.overlaps(b) == b.overlaps(a)
/// and a.intersection(b) == b.intersection(a). Any asymmetry produces collision
/// bugs where one object detects a hit but the other does not.
#[test]
fn prop_aabb_overlaps_and_intersection_are_symmetric() {
    let mut rng = SplitMix64::new(0x0F2B_C005);
    for _ in 0..ITERS {
        let a = rand_aabb(&mut rng);
        let b = rand_aabb(&mut rng);
        assert_eq!(
            a.overlaps(&b),
            b.overlaps(&a),
            "overlaps not symmetric for a={a:?}, b={b:?}"
        );
        assert_eq!(
            a.intersection(&b),
            b.intersection(&a),
            "intersection not symmetric for a={a:?}, b={b:?}"
        );
    }
}

/// **union is commutative; idempotent for non-empty boxes** —
/// a.union(b) == b.union(a) always, and a.union(a) == a when a is non-empty.
/// (When both inputs are empty the spec returns an empty box at the origin, so
/// idempotence is only claimed for non-empty boxes.) A commutativity failure
/// means bounding hierarchies depend on argument order.
#[test]
fn prop_aabb_union_is_commutative_and_idempotent() {
    let mut rng = SplitMix64::new(0x0A3C_D005);
    for _ in 0..ITERS {
        let a = rand_aabb(&mut rng);
        let b = rand_aabb(&mut rng);
        assert_eq!(
            a.union(&b),
            b.union(&a),
            "union not commutative: a={a:?}, b={b:?}"
        );
        if !a.is_empty() {
            assert_eq!(a.union(&a), a, "union not idempotent for non-empty a={a:?}");
        }
    }
}

/// **contains → overlaps** — if `a` contains `b`, `a` must also overlap `b`.
/// The converse need not hold (partial overlap ≠ containment), but the forward
/// implication is definitionally required.
///
/// Half the trials explicitly construct `b` inside `a` to guarantee non-vacuous
/// coverage (≥500 containment cases); the other half are fully random to test
/// boundary conditions.
#[test]
fn prop_aabb_contains_implies_overlaps() {
    let mut rng = SplitMix64::new(0x0B4D_E005);
    let mut found = 0usize;
    for trial in 0..ITERS {
        let a = Aabb::new(
            rng.range(-100, 101),
            rng.range(-100, 101),
            rng.range(10, 51),  // non-empty: w ∈ [10, 50]
            rng.range(10, 51),
        );
        let b = if trial % 2 == 0 {
            // Biased: b is strictly inside a (guaranteed containment).
            let inset = rng.range(1, 4);
            Aabb::new(
                a.x + inset,
                a.y + inset,
                (a.w - 2 * inset).max(1),
                (a.h - 2 * inset).max(1),
            )
        } else {
            // Unbiased: random box anywhere (exercises edge cases).
            rand_aabb(&mut rng)
        };
        if a.contains(&b) {
            found += 1;
            assert!(
                a.overlaps(&b),
                "contains but not overlaps: a={a:?}, b={b:?}"
            );
        }
    }
    assert!(found >= 500, "expected ≥500 containment cases for non-vacuous coverage, got {found}");
}

/// **iter_points count equals area** — every cell counted by `area()` must be
/// yielded by `iter_points()` — no skips, no extras, no off-by-one on empty.
#[test]
fn prop_aabb_iter_points_count_equals_area() {
    let mut rng = SplitMix64::new(0x0C5E_F005);
    for _ in 0..ITERS {
        let a = rand_aabb(&mut rng);
        let count = a.iter_points().count() as i32;
        assert_eq!(
            count,
            a.area(),
            "iter_points count {count} ≠ area {} for a={a:?}",
            a.area()
        );
    }
}

/// **split_v exact partition** — `left.w + right.w == a.w` for any split x,
/// the heights are preserved, and the two halves never overlap. This is the
/// invariant BSP dungeon partitioning relies on for non-overlapping rooms.
#[test]
fn prop_aabb_split_v_is_exact_partition() {
    let mut rng = SplitMix64::new(0x0D6F_A005);
    for _ in 0..ITERS {
        let a = rand_aabb(&mut rng);
        let split_x = rng.range(a.x - 5, a.x + a.w + 5);
        let (left, right) = a.split_v(split_x);
        assert_eq!(
            left.w + right.w,
            a.w,
            "split_v widths don't sum to original: a={a:?}, x={split_x}"
        );
        assert_eq!(left.h, a.h, "split_v changed height: a={a:?}");
        assert_eq!(right.h, a.h, "split_v changed height: a={a:?}");
        assert!(
            !left.overlaps(&right),
            "split_v halves overlap: a={a:?}, x={split_x}"
        );
    }
}

// ── Noise (deterministic value noise / fBm / hashing) properties ──────────────

/// A random Q16.16 fixed-point coordinate: a wide integer part (`x >> 16`) with
/// a random 16-bit fraction. Spans negatives so `rem_euclid` wrap paths and
/// `wrapping_shl` octave shifts are all exercised.
fn rand_q16(rng: &mut SplitMix64) -> i32 {
    (rng.range(-2000, 2000) << 16) | (rng.below(0x1_0000) as i32)
}

/// A non-zero, well-spread 64-bit seed from two 31-bit draws.
fn rand_seed(rng: &mut SplitMix64) -> u64 {
    (rng.below(0x7FFF_FFFF) as u64) << 32 | rng.below(0x7FFF_FFFF) as u64 | 1
}

fn mini_floor(seed: u64) -> Dungeon {
    generate_dungeon(
        20,
        14,
        &mut SplitMix64::new(seed),
        GenParams { max_rooms: 4, min_room: 3, max_room: 5 },
    )
}

/// **Output-range invariant** — every smooth/fBm noise function must return a
/// value in `[0, 65535]` for *all* inputs. The module doc fixes this range
/// (so two values multiply in `u32` without overflow); a regression in any
/// interpolation or normalization step would push a value past `65535`.
/// Exercised over 3000 random (coord, seed, octaves) tuples spanning negative
/// coordinates and octave counts up to 8.
#[test]
fn prop_noise_output_always_in_range() {
    use izanagi_kit::noise::{turbulence_1d, turbulence_2d};
    let mut rng = SplitMix64::new(0x0E70_A005);
    for _ in 0..ITERS {
        let x = rand_q16(&mut rng);
        let y = rand_q16(&mut rng);
        let z = rand_q16(&mut rng);
        let seed = rand_seed(&mut rng);
        let oct = rng.below(9); // 0..=8

        let samples = [
            value_noise_1d(x, seed),
            value_noise_2d(x, y, seed),
            value_noise_3d(x, y, z, seed),
            fbm_1d(x, seed, oct),
            fbm_2d(x, y, seed, oct),
            fbm_3d(x, y, z, seed, oct),
            ridge_noise_2d(x, y, seed, oct),
            turbulence_1d(x, seed, oct),
            turbulence_2d(x, y, seed, oct),
        ];
        for (k, &v) in samples.iter().enumerate() {
            assert!(
                v <= 65535,
                "noise sample #{k} out of range: {v} (x={x:#x}, y={y:#x}, seed={seed:#x}, oct={oct})"
            );
        }
    }
}

/// **Wrap-variant range** — the tileable functions must also honour the
/// `[0, 65535]` range for any period (including the degenerate `period == 0`,
/// which the doc treats as `1`). Periods are kept small so `period << shift`
/// stays well within `i32`.
#[test]
fn prop_noise_wrap_output_always_in_range() {
    let mut rng = SplitMix64::new(0x0F81_B005);
    for _ in 0..ITERS {
        let x = rand_q16(&mut rng);
        let y = rand_q16(&mut rng);
        let seed = rand_seed(&mut rng);
        let oct = rng.below(7);
        let period = rng.range(0, 33); // includes 0 (→ treated as 1)

        assert!(value_noise_1d_wrap(x, seed, period) <= 65535, "1d_wrap range");
        assert!(
            value_noise_2d_wrap(x, y, seed, period, period) <= 65535,
            "2d_wrap range"
        );
        assert!(fbm_1d_wrap(x, seed, oct, period.max(1)) <= 65535, "fbm_1d_wrap range");
        assert!(
            fbm_2d_wrap(x, y, seed, oct, period.max(1)) <= 65535,
            "fbm_2d_wrap range"
        );
    }
}

/// **Determinism** — the module's core promise ("pure, float-free, bit-identical
/// across targets"): the same inputs always produce the same output. Any hidden
/// state, float rounding, or address-dependent mixing would break replay.
#[test]
fn prop_noise_is_deterministic() {
    use izanagi_kit::noise::{turbulence_1d, turbulence_2d};
    let mut rng = SplitMix64::new(0x0A92_C005);
    for _ in 0..ITERS {
        let x = rand_q16(&mut rng);
        let y = rand_q16(&mut rng);
        let z = rand_q16(&mut rng);
        let seed = rand_seed(&mut rng);
        let oct = rng.below(7);

        assert_eq!(value_noise_2d(x, y, seed), value_noise_2d(x, y, seed), "vn2d");
        assert_eq!(
            value_noise_3d(x, y, z, seed),
            value_noise_3d(x, y, z, seed),
            "vn3d"
        );
        assert_eq!(fbm_2d(x, y, seed, oct), fbm_2d(x, y, seed, oct), "fbm2d");
        assert_eq!(fbm_3d(x, y, z, seed, oct), fbm_3d(x, y, z, seed, oct), "fbm3d");
        assert_eq!(
            ridge_noise_2d(x, y, seed, oct),
            ridge_noise_2d(x, y, seed, oct),
            "ridge"
        );
        assert_eq!(turbulence_1d(x, seed, oct), turbulence_1d(x, seed, oct), "turb1d");
        assert_eq!(
            turbulence_2d(x, y, seed, oct),
            turbulence_2d(x, y, seed, oct),
            "turb2d"
        );
    }
}

/// **Exact tileability** — the wrap variants must produce identical values at a
/// period boundary: `noise(0, y) == noise(period, y)` and `noise(x, 0) ==
/// noise(x, period)`, evaluated at integer coordinates. This is the seamless-
/// tiling guarantee for world-map wrapping; an off-by-one in the `rem_euclid`
/// corner selection would break it.
#[test]
fn prop_value_noise_wrap_tiles_at_period() {
    let mut rng = SplitMix64::new(0x0B03_D005);
    for _ in 0..ITERS {
        let seed = rand_seed(&mut rng);
        let period = rng.range(2, 33);
        let coord = rng.range(0, period); // integer cell within one tile

        // 1-D: noise(0) == noise(period).
        assert_eq!(
            value_noise_1d_wrap(0, seed, period),
            value_noise_1d_wrap(period << 16, seed, period),
            "1d wrap failed (seed={seed:#x}, period={period})"
        );

        // 2-D x-axis: noise(0, c) == noise(period, c).
        assert_eq!(
            value_noise_2d_wrap(0, coord << 16, seed, period, period),
            value_noise_2d_wrap(period << 16, coord << 16, seed, period, period),
            "2d x-wrap failed (seed={seed:#x}, period={period}, c={coord})"
        );

        // 2-D y-axis: noise(c, 0) == noise(c, period).
        assert_eq!(
            value_noise_2d_wrap(coord << 16, 0, seed, period, period),
            value_noise_2d_wrap(coord << 16, period << 16, seed, period, period),
            "2d y-wrap failed (seed={seed:#x}, period={period}, c={coord})"
        );
    }
}

/// **Integer-coordinate identity** — at a whole-number coordinate (zero
/// fraction), value noise returns exactly the corner hash `>> 16` (no
/// interpolation). Documents the boundary case the interpolation formula must
/// reduce to. Checked for 1-D, 2-D, and 3-D over random integer coords.
#[test]
fn prop_value_noise_at_integer_coords_equals_corner_hash() {
    let mut rng = SplitMix64::new(0x0C14_E005);
    for _ in 0..ITERS {
        let xi = rng.range(-5000, 5000);
        let yi = rng.range(-5000, 5000);
        let zi = rng.range(-5000, 5000);
        let seed = rand_seed(&mut rng);

        assert_eq!(
            value_noise_1d(xi << 16, seed),
            hash_1d(xi, seed) >> 16,
            "vn1d integer-coord mismatch (xi={xi}, seed={seed:#x})"
        );
        assert_eq!(
            value_noise_2d(xi << 16, yi << 16, seed),
            hash_2d(xi, yi, seed) >> 16,
            "vn2d integer-coord mismatch (xi={xi}, yi={yi}, seed={seed:#x})"
        );
        assert_eq!(
            value_noise_3d(xi << 16, yi << 16, zi << 16, seed),
            hash_3d(xi, yi, zi, seed) >> 16,
            "vn3d integer-coord mismatch (xi={xi}, yi={yi}, zi={zi}, seed={seed:#x})"
        );
    }
}

/// **`hash_range` bounds** — for `lo < hi`, `hash_range` always lands in the
/// half-open `[lo, hi)`; for a degenerate range (`lo >= hi`) it returns `lo`.
/// The wide-multiply mapping must never produce `hi` or exceed the range, which
/// would corrupt scatter-table indexing. Covers the full `u32` hash domain and
/// extreme `lo`/`hi` including `i32::MIN`/`i32::MAX`.
#[test]
fn prop_hash_range_is_within_half_open_bounds() {
    use izanagi_kit::noise::hash_range;
    let mut rng = SplitMix64::new(0x0D25_F005);
    for _ in 0..ITERS {
        let h = (rng.below(0x7FFF_FFFF) << 1) | rng.below(2); // full u32 spread
        let a = rng.range(-100_000, 100_000);
        let b = rng.range(-100_000, 100_000);

        // Ordered range → half-open containment.
        let (lo, hi) = (a.min(b), a.max(b));
        if lo < hi {
            let v = hash_range(h, lo, hi);
            assert!(
                v >= lo && v < hi,
                "hash_range({h:#x}, {lo}, {hi}) = {v} not in [{lo}, {hi})"
            );
        }

        // Degenerate range → lo.
        assert_eq!(hash_range(h, 7, 7), 7, "degenerate equal range");
        assert_eq!(hash_range(h, 9, 3), 9, "degenerate inverted range");

        // Extreme range must not overflow or escape the half-open upper bound
        // (`ev >= i32::MIN` is trivially true, so only the `< hi` edge matters).
        let ev = hash_range(h, i32::MIN, i32::MAX);
        assert!(ev < i32::MAX, "extreme-range escape: {ev}");
    }
}

/// **`normalize_noise` bounds** — for any noise value `v ∈ [0, 65535]` and
/// `lo < hi`, the mapped result lies in the closed `[lo, hi]`, with the
/// endpoints `v=0 → lo` and `v=65535 → hi`. Degenerate ranges return `lo`.
#[test]
fn prop_normalize_noise_is_within_closed_bounds() {
    let mut rng = SplitMix64::new(0x0E36_A005);
    for _ in 0..ITERS {
        let v = rng.below(0x1_0000); // 0..=65535
        let a = rng.range(-100_000, 100_000);
        let b = rng.range(-100_000, 100_000);
        let (lo, hi) = (a.min(b), a.max(b));

        if lo < hi {
            let r = normalize_noise(v, lo, hi);
            assert!(
                r >= lo && r <= hi,
                "normalize_noise({v}, {lo}, {hi}) = {r} not in [{lo}, {hi}]"
            );
            // Endpoints are exact.
            assert_eq!(normalize_noise(0, lo, hi), lo, "v=0 must map to lo");
            assert_eq!(normalize_noise(65535, lo, hi), hi, "v=65535 must map to hi");
        }

        assert_eq!(normalize_noise(v, 5, 5), 5, "degenerate equal range");
        assert_eq!(normalize_noise(v, 8, 2), 8, "degenerate inverted range");
    }
}

// ── SpatialHash properties ─────────────────────────────────────────────────────

/// **Insert → contains** — the fundamental membership invariant: after inserting
/// key K at (x, y), both `contains(&K, x, y)` and `query_cell(x, y).contains(&K)`
/// must return true for every cell size, coordinate (including negatives), and key.
#[test]
fn prop_spatial_hash_insert_then_contains() {
    let mut rng = SplitMix64::new(0x0A1B_2C3D);
    for _ in 0..ITERS {
        let cell_size = 1 + rng.below(32) as i32;
        let mut sh: SpatialHash<u32> = SpatialHash::new(cell_size);
        let x = rng.range(-500, 501);
        let y = rng.range(-500, 501);
        let key: u32 = rng.below(u32::MAX);
        sh.insert(key, x, y);
        assert!(sh.contains(&key, x, y), "key missing after insert at ({x},{y})");
        assert!(
            sh.query_cell(x, y).contains(&key),
            "query_cell missed key at ({x},{y})"
        );
    }
}

/// **Remove undoes insert** — after one insert + matching remove, `contains`
/// returns false and the cell is empty again. Dual of the insert invariant.
#[test]
fn prop_spatial_hash_remove_undoes_insert() {
    let mut rng = SplitMix64::new(0x1B2C_3D4E);
    for _ in 0..ITERS {
        let cell_size = 1 + rng.below(32) as i32;
        let mut sh: SpatialHash<u32> = SpatialHash::new(cell_size);
        let x = rng.range(-500, 501);
        let y = rng.range(-500, 501);
        let key: u32 = rng.below(0x8000_0000);
        sh.insert(key, x, y);
        sh.remove(&key, x, y);
        assert!(!sh.contains(&key, x, y), "key still present after remove at ({x},{y})");
        assert!(sh.query_cell(x, y).is_empty(), "cell non-empty after remove");
    }
}

/// **query_rect ⊇ query_cell** — every key found by `query_cell(px, py)` must
/// also appear in `query_rect` over any rectangle that contains `(px, py)`.
/// Verifies that the rect query never under-reports relative to the cell query.
/// One entity is always inserted exactly at the query point so every trial
/// exercises the subset check (non-vacuous by construction).
#[test]
fn prop_spatial_hash_query_rect_superset_of_query_cell() {
    use std::collections::HashSet;
    let mut rng = SplitMix64::new(0x2C3D_4E5F);
    for _ in 0..ITERS {
        let cell_size = 1 + rng.below(16) as i32;
        let mut sh: SpatialHash<u32> = SpatialHash::new(cell_size);
        let px = rng.range(-80, 81);
        let py = rng.range(-80, 81);
        // Always insert key 0 at the query point so query_cell is never empty.
        sh.insert(0u32, px, py);
        // Insert a few more at random positions.
        let n = rng.below(9) as u32;
        for k in 1..=n {
            sh.insert(k, rng.range(-100, 101), rng.range(-100, 101));
        }
        // Build a rect guaranteed to contain (px, py): offset in [0, rw-1].
        let rw = rng.range(1, 20);
        let rh = rng.range(1, 20);
        let rx = px - rng.range(0, rw);
        let ry = py - rng.range(0, rh);

        let cell_keys: HashSet<u32> = sh.query_cell(px, py).iter().copied().collect();
        let rect_keys: HashSet<u32> = sh.query_rect(rx, ry, rw, rh).into_iter().collect();
        assert!(
            cell_keys.is_subset(&rect_keys),
            "query_cell ⊄ query_rect: px={px},py={py}, rect=({rx},{ry},{rw},{rh})"
        );
    }
}

/// **Euclidean ⊆ Chebyshev** — every key within Euclidean distance r of the
/// query centre is also within Chebyshev distance r (the inscribed-circle law).
/// A cell whose closest point is within Euclidean r also overlaps the Chebyshev
/// square, so false negatives in `query_radius` can never hide a Euclidean hit.
#[test]
fn prop_spatial_hash_euclidean_subset_of_chebyshev() {
    use std::collections::HashSet;
    let mut rng = SplitMix64::new(0x3D4E_5F6A);
    for _ in 0..ITERS {
        let cell_size = 1 + rng.below(8) as i32;
        let mut sh: SpatialHash<u32> = SpatialHash::new(cell_size);
        let n = rng.below(15) as usize;
        for k in 0..n as u32 {
            sh.insert(k, rng.range(-50, 51), rng.range(-50, 51));
        }
        let cx = rng.range(-40, 41);
        let cy = rng.range(-40, 41);
        let radius = rng.range(0, 20);

        let eucl: HashSet<u32> = sh
            .query_radius_euclidean(cx, cy, radius)
            .into_iter()
            .collect();
        let cheb: HashSet<u32> = sh.query_radius(cx, cy, radius).into_iter().collect();
        assert!(
            eucl.is_subset(&cheb),
            "euclidean ⊄ chebyshev: center=({cx},{cy}), radius={radius}"
        );
    }
}

/// **query_rect_count == query_rect.len()** — the allocation-free count variant
/// must always agree with the allocating variant. A divergence would mean AI
/// budget checks silently disagree with actual collision results.
#[test]
fn prop_spatial_hash_count_matches_query_len() {
    let mut rng = SplitMix64::new(0x4E5F_6A7B);
    for _ in 0..ITERS {
        let cell_size = 1 + rng.below(16) as i32;
        let mut sh: SpatialHash<u32> = SpatialHash::new(cell_size);
        let n = rng.below(20) as usize;
        for k in 0..n as u32 {
            sh.insert(k, rng.range(-100, 101), rng.range(-100, 101));
        }
        let x = rng.range(-100, 101);
        let y = rng.range(-100, 101);
        let w = rng.range(1, 50);
        let h = rng.range(1, 50);
        assert_eq!(
            sh.query_rect_count(x, y, w, h),
            sh.query_rect(x, y, w, h).len(),
            "query_rect_count ≠ query_rect.len at ({x},{y},{w},{h})"
        );
    }
}

/// **move_entity transfers membership** — after moving key K from (ox,oy) to a
/// different cell (nx,ny), the key is found in the new cell and absent from the
/// old one. Uses an offset of ≥ cell_size to guarantee distinct cells.
#[test]
fn prop_spatial_hash_move_transfers_membership() {
    let mut rng = SplitMix64::new(0x5F6A_7B8C);
    for _ in 0..ITERS {
        let cell_size = 1 + rng.below(16) as i32;
        let mut sh: SpatialHash<u32> = SpatialHash::new(cell_size);
        let ox = rng.range(-100, 101);
        let oy = rng.range(-100, 101);
        // Offset by at least cell_size so the target cell is definitely different.
        let nx = ox.saturating_add(cell_size + rng.range(0, cell_size));
        let ny = oy.saturating_add(cell_size + rng.range(0, cell_size));
        let key: u32 = 42;
        sh.insert(key, ox, oy);
        sh.move_entity(key, ox, oy, nx, ny);
        assert!(sh.contains(&key, nx, ny), "key not in new cell after move ({ox},{oy})→({nx},{ny})");
        assert!(!sh.contains(&key, ox, oy), "key still in old cell after move ({ox},{oy})→({nx},{ny})");
    }
}

// ── PassabilityGrid properties ────────────────────────────────────────────────

/// **blocked + passable == len** — `blocked_count() + passable_count()` must
/// equal `len()` for every grid, including after mutations. A partition invariant:
/// no cell is double-counted or missed. Verified after construction and after a
/// random single-cell toggle.
#[test]
fn prop_passability_counts_sum_to_len() {
    let mut rng = SplitMix64::new(0x6A7B_8C9D);
    for _ in 0..ITERS {
        let w = rng.below(20) as i32;
        let h = rng.below(20) as i32;
        // Build a grid using a per-call rng draw so blocked density varies.
        let mut grid = PassabilityGrid::from_fn(w, h, |_, _| rng.below(4) == 0);
        assert_eq!(
            grid.blocked_count() + grid.passable_count(),
            grid.len(),
            "counts don't sum to len (w={w}, h={h})"
        );
        // Invariant must also hold after a random mutation.
        if w > 0 && h > 0 {
            let x = rng.below(w as u32) as i32;
            let y = rng.below(h as u32) as i32;
            grid.set_blocked(x, y, !grid.is_blocked(x, y));
            assert_eq!(
                grid.blocked_count() + grid.passable_count(),
                grid.len(),
                "counts don't sum to len after toggle at ({x},{y})"
            );
        }
    }
}

/// **from_fn predicate round-trip** — `from_fn(w, h, pred)` must store
/// `pred(x, y)` in every cell so that `is_blocked(x, y) == pred(x, y)` for all
/// in-bounds `(x, y)`. Any index-mapping bug in the row-major formula breaks this.
#[test]
fn prop_passability_from_fn_matches_predicate() {
    let mut rng = SplitMix64::new(0x7B8C_9D0E);
    for _ in 0..ITERS {
        let w = rng.below(12) as i32;
        let h = rng.below(12) as i32;
        // Deterministic arithmetic predicate so we can re-evaluate it without rng.
        let salt = rng.below(5);
        let pred = |x: i32, y: i32| {
            x.wrapping_mul(3)
                .wrapping_add(y.wrapping_mul(7))
                .rem_euclid(5) as u32
                == salt
        };
        let grid = PassabilityGrid::from_fn(w, h, |x, y| pred(x, y));
        for y in 0..h {
            for x in 0..w {
                assert_eq!(
                    grid.is_blocked(x, y),
                    pred(x, y),
                    "from_fn mismatch at ({x},{y}) for w={w}, h={h}"
                );
            }
        }
    }
}

/// **invert is an involution** — `invert()` applied twice is the identity: every
/// cell returns to its original state and `blocked_count` is unchanged. Any
/// off-by-one in the cell indexing during inversion would scramble the grid.
#[test]
fn prop_passability_invert_is_involution() {
    let mut rng = SplitMix64::new(0x8C9D_0E1F);
    for _ in 0..ITERS {
        let w = rng.below(15) as i32;
        let h = rng.below(15) as i32;
        let mut grid = PassabilityGrid::from_fn(w, h, |_, _| rng.below(3) == 0);
        let before = grid.blocked_count();
        grid.invert();
        grid.invert();
        assert_eq!(
            grid.blocked_count(),
            before,
            "double-invert changed blocked_count (w={w}, h={h})"
        );
        // Spot-check a random cell's state.
        if w > 0 && h > 0 {
            let x = rng.below(w as u32) as i32;
            let y = rng.below(h as u32) as i32;
            let original = grid.is_blocked(x, y);
            grid.invert();
            grid.invert();
            assert_eq!(
                grid.is_blocked(x, y),
                original,
                "double-invert changed cell ({x},{y})"
            );
        }
    }
}

/// **set_region covers the rectangle** — every cell in the axis-aligned
/// inclusive rectangle `[x1,x2] × [y1,y2]` (with x1/x2 and y1/y2 swapped as
/// needed) must have the target value after `set_region`. The invariant that BSP
/// and room-carving code relies on: bulk-writing a region leaves no gaps.
#[test]
fn prop_passability_set_region_covers_rectangle() {
    let mut rng = SplitMix64::new(0x9D0E_1F2A);
    for _ in 0..ITERS {
        let w = 2 + rng.below(18) as i32; // 2..=19
        let h = 2 + rng.below(18) as i32;
        let mut grid = PassabilityGrid::new(w, h); // all passable
        // Random inclusive rectangle (set_region handles x1>x2 internally).
        let x1 = rng.range(0, w); // [0, w-1]
        let y1 = rng.range(0, h);
        let x2 = rng.range(0, w);
        let y2 = rng.range(0, h);
        grid.set_region(x1, y1, x2, y2, true);
        let (xs, xe) = (x1.min(x2), x1.max(x2));
        let (ys, ye) = (y1.min(y2), y1.max(y2));
        for y in ys..=ye {
            for x in xs..=xe {
                assert!(
                    grid.is_blocked(x, y),
                    "set_region missed ({x},{y}) in [{xs},{xe}]×[{ys},{ye}]"
                );
            }
        }
    }
}

// ── InfluenceMap properties ───────────────────────────────────────────────────

/// **Source cell gets full strength** — the cell at (sx, sy) is at Chebyshev
/// distance 0 from itself, so `add_source(sx, sy, strength, radius)` always
/// stores exactly `strength` there, regardless of radius. A scaling or off-by-one
/// bug in the falloff formula breaks this for all non-zero radii.
#[test]
fn prop_influence_add_source_origin_gets_full_strength() {
    let mut rng = SplitMix64::new(0x0E1F_2A3B);
    for _ in 0..ITERS {
        let w = 2 + rng.below(20) as i32;
        let h = 2 + rng.below(20) as i32;
        let sx = rng.below(w as u32) as i32;
        let sy = rng.below(h as u32) as i32;
        let strength = rng.range(1, 10001);
        let radius = rng.range(0, 10);
        let mut map = InfluenceMap::new(w, h);
        map.add_source(sx, sy, strength, radius);
        assert_eq!(
            map.get(sx, sy),
            Some(strength),
            "source cell ({sx},{sy}) has wrong value (strength={strength}, radius={radius})"
        );
    }
}

/// **No cell exceeds strength** — the linear falloff formula for `add_source`
/// (strength × (r−dist)/r for dist ≤ r) never produces a value larger than
/// `strength`. Verified over all cells of randomly-sized maps.
#[test]
fn prop_influence_add_source_never_exceeds_strength() {
    let mut rng = SplitMix64::new(0x1F2A_3B4C);
    for _ in 0..ITERS {
        let w = 2 + rng.below(25) as i32;
        let h = 2 + rng.below(25) as i32;
        let sx = rng.below(w as u32) as i32;
        let sy = rng.below(h as u32) as i32;
        let strength = rng.range(0, 10001);
        let radius = rng.range(0, 8);
        let mut map = InfluenceMap::new(w, h);
        map.add_source(sx, sy, strength, radius);
        for (_, _, v) in map.iter() {
            assert!(
                v >= 0 && v <= strength,
                "add_source cell out of [0,{strength}]: {v}"
            );
        }
    }
}

/// **decay reduces magnitudes** — after `decay(num, den)` with `0 ≤ num < den`,
/// every cell's absolute value can only decrease (integer truncation toward zero).
/// Verified by cloning the map before the decay and comparing cell-by-cell.
#[test]
fn prop_influence_decay_reduces_magnitudes() {
    let mut rng = SplitMix64::new(0x2A3B_4C5D);
    for _ in 0..ITERS {
        let w = 1 + rng.below(8) as i32;
        let h = 1 + rng.below(8) as i32;
        let mut map = InfluenceMap::new(w, h);
        for y in 0..h {
            for x in 0..w {
                map.set(x, y, rng.range(-1000, 1001));
            }
        }
        let before = map.clone();
        let den = 2 + rng.below(10) as i32; // 2..=11
        let num = rng.below(den as u32) as i32; // 0..=den-1 (<den)
        map.decay(num, den);
        for y in 0..h {
            for x in 0..w {
                let bv = before.get(x, y).unwrap();
                let av = map.get(x, y).unwrap();
                assert!(
                    av.abs() <= bv.abs(),
                    "decay({num}/{den}) increased magnitude {bv} → {av} at ({x},{y})"
                );
            }
        }
    }
}

/// **normalize pins the range endpoints** — after `normalize(lo, hi)` on a map
/// with at least two distinct values, `min_value() == lo` and `max_value() == hi`.
/// The i128 wide-multiply path must place the maximum input value exactly at hi
/// and the minimum exactly at lo, with no rounding drift.
#[test]
fn prop_influence_normalize_pins_range() {
    let mut rng = SplitMix64::new(0x3B4C_5D6E);
    let mut pinned = 0usize;
    for _ in 0..ITERS {
        let w = 2 + rng.below(8) as i32;
        let h = 2 + rng.below(8) as i32;
        let mut map = InfluenceMap::new(w, h);
        // At least two distinct values: fill with base, then spike one cell.
        let base = rng.range(-500, 501);
        let peak = base + rng.range(1, 201); // peak > base
        map.fill(base);
        let px = rng.below(w as u32) as i32;
        let py = rng.below(h as u32) as i32;
        map.set(px, py, peak);

        let a = rng.range(-200, 201);
        let b = rng.range(-200, 201);
        let (lo, hi) = (a.min(b), a.max(b));
        if lo >= hi {
            continue; // degenerate — skip, don't count
        }
        map.normalize(lo, hi);
        assert_eq!(
            map.min_value(),
            Some(lo),
            "normalize({lo},{hi}) did not pin min"
        );
        assert_eq!(
            map.max_value(),
            Some(hi),
            "normalize({lo},{hi}) did not pin max"
        );
        pinned += 1;
    }
    assert!(pinned >= 2000, "expected ≥2000 normalize trials, got {pinned}");
}

/// **find_peaks agrees with direct scan** — `find_peaks(threshold)` must return
/// exactly the cells where `get(x, y) >= threshold`, in the same row-major order
/// produced by iterating `y` then `x`. An indexing bug in the coordinate-recovery
/// formula `(x = i % w, y = i / w)` would shift coordinates and break this.
#[test]
fn prop_influence_find_peaks_agrees_with_direct_scan() {
    let mut rng = SplitMix64::new(0x4C5D_6E7F);
    for _ in 0..ITERS {
        let w = 1 + rng.below(10) as i32;
        let h = 1 + rng.below(10) as i32;
        let mut map = InfluenceMap::new(w, h);
        for y in 0..h {
            for x in 0..w {
                map.set(x, y, rng.range(-100, 101));
            }
        }
        let threshold = rng.range(-50, 51);
        let peaks = map.find_peaks(threshold);

        let expected: Vec<(i32, i32)> = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .filter(|&(x, y)| map.get(x, y).unwrap() >= threshold)
            .collect();

        assert_eq!(
            peaks, expected,
            "find_peaks({threshold}) disagrees with direct scan (w={w}, h={h})"
        );
    }
}

// ── RandomTable properties ─────────────────────────────────────────────────────

/// **total_weight is sum of entries** — the cached `total_weight()` field must
/// equal `iter().map(|(w,_)| w as u64).sum()` after any combination of `push`
/// calls. A book-keeping bug (forgetting to accumulate on push or clear on `clear`)
/// would produce biased selection or spurious `None` rolls without any panic.
#[test]
fn prop_random_table_total_weight_is_sum() {
    let mut rng = SplitMix64::new(0xA0B1_C2D3);
    for _ in 0..ITERS {
        let n = rng.below(10) as usize;
        let mut table: RandomTable<u32> = RandomTable::new();
        let mut expected: u64 = 0;
        for _ in 0..n {
            let w = rng.below(100) as u32;
            let v = rng.below(1000) as u32;
            table.push(w, v);
            expected += w as u64;
        }
        assert_eq!(table.total_weight(), expected, "total_weight ≠ push sum (n={n})");
        let iter_sum: u64 = table.iter().map(|(w, _)| w as u64).sum();
        assert_eq!(table.total_weight(), iter_sum, "total_weight ≠ iter sum");
    }
}

/// **Zero-weight entries are never selected** — the wide-multiply formula maps
/// weight-0 entries to a length-0 bucket. `roll` must never return the sentinel
/// value `u32::MAX` inserted with weight 0, even though it is stored in the
/// table. Every trial has at least one non-zero entry so `roll` always returns
/// `Some(_)` (non-vacuous by construction).
#[test]
fn prop_random_table_zero_weight_never_selected() {
    let mut rng = SplitMix64::new(0xB1C2_D3E4);
    for _ in 0..ITERS {
        let n_nonzero = 1 + rng.below(5) as u32;
        let mut table: RandomTable<u32> = RandomTable::new();
        table.push(0, u32::MAX); // zero-weight sentinel; can never appear in rolls
        for i in 0..n_nonzero {
            table.push(1 + rng.below(10) as u32, i); // values 0..n_nonzero — never u32::MAX
        }
        for _ in 0..10 {
            match table.roll(&mut rng) {
                Some(&u32::MAX) => panic!("zero-weight sentinel was selected"),
                Some(_) => {}
                None => panic!("non-empty table returned None"),
            }
        }
    }
}

/// **roll_n returns exactly n items** — for a non-empty table (all non-zero
/// weights), `roll_n(n, rng)` must return a `Vec` of length exactly `n`. The
/// underlying `filter_map(roll_owned)` iterator only skips empty/zero-total
/// tables; a premature break or off-by-one would shrink the output.
#[test]
fn prop_random_table_roll_n_returns_correct_count() {
    let mut rng = SplitMix64::new(0xC2D3_E4F5);
    for _ in 0..ITERS {
        let n_entries = 1 + rng.below(8) as u32;
        let mut table: RandomTable<u32> = RandomTable::new();
        for i in 0..n_entries {
            table.push(1 + rng.below(5) as u32, i); // all weights ≥ 1
        }
        let n = rng.below(20) as u32;
        let results = table.roll_n(n, &mut rng);
        assert_eq!(
            results.len(),
            n as usize,
            "roll_n({n}) returned {} items (table len={n_entries})",
            results.len()
        );
    }
}

/// **weighted_idx agrees with roll on same RNG state** — forking the RNG at
/// the same state, `weighted_idx` and `roll` must land on the same entry.
/// The two functions share identical wide-multiply logic; this catches any
/// divergence in the rounding fallback or loop termination condition.
#[test]
fn prop_random_table_weighted_idx_consistent_with_roll() {
    let mut rng = SplitMix64::new(0xD3E4_F506);
    for _ in 0..ITERS {
        let n = 2 + rng.below(7) as usize;
        let mut table: RandomTable<u32> = RandomTable::new();
        for i in 0..n as u32 {
            table.push(1 + rng.below(10) as u32, i * 10);
        }
        // Fork: both forks see the same RNG state at the point of selection.
        let seed = rand_seed(&mut rng);
        let mut rng_a = SplitMix64::new(seed);
        let mut rng_b = SplitMix64::new(seed);
        let idx = table.weighted_idx(&mut rng_a).expect("non-empty table");
        let val = table.roll(&mut rng_b).expect("non-empty table");
        let (_, &entry_val) = table.iter().nth(idx).expect("idx in range");
        assert_eq!(
            entry_val, *val,
            "weighted_idx({idx}) = {entry_val}, roll = {} (disagree)",
            *val
        );
    }
}

// ── Cooldown properties ───────────────────────────────────────────────────────

/// **tick is monotone non-increasing** — `remaining` can only decrease (or
/// stay at zero). The `saturating_sub` implementation must never produce a
/// value higher than the pre-tick remaining, even for large tick counts that
/// would overflow non-saturating subtraction.
#[test]
fn prop_cooldown_tick_is_monotone() {
    let mut rng = SplitMix64::new(0xE4F5_0617);
    for _ in 0..ITERS {
        let initial = rng.below(0x1_0000) as u32;
        let mut cd = Cooldown::new(initial);
        let steps = rng.below(12) as usize;
        let mut prev = cd.remaining;
        for _ in 0..steps {
            let t = rng.below(200) as u32;
            cd.tick(t);
            assert!(
                cd.remaining <= prev,
                "tick({t}) increased remaining {prev} → {}",
                cd.remaining
            );
            prev = cd.remaining;
        }
    }
}

/// **percent_remaining is in [0, 100]** — for any combination of `remaining`
/// and `original_ticks` (including degenerate cases where remaining > original
/// or original == 0), `percent_remaining` must stay within the closed interval
/// [0, 100] so UI progress bars can never overflow or underflow.
#[test]
fn prop_cooldown_percent_remaining_in_range() {
    let mut rng = SplitMix64::new(0xF506_1728);
    for _ in 0..ITERS {
        let remaining = rng.below(0x1_0000) as u32;
        let original = rng.below(0x1_0000) as u32; // intentionally may be < remaining
        let cd = Cooldown::new(remaining);
        let pct = cd.percent_remaining(original);
        assert!(
            pct <= 100,
            "percent_remaining({remaining}, orig={original}) = {pct} > 100"
        );
    }
}

/// **elapsed accounting identity** — `elapsed(n)` must equal
/// `n.saturating_sub(remaining)` for all `(remaining, n)` pairs, including
/// cases where `n < remaining`. A divergence would show incorrect consumed-tick
/// values in UI progress bars and AI countdown queries.
#[test]
fn prop_cooldown_elapsed_accounting() {
    let mut rng = SplitMix64::new(0x0617_2839);
    for _ in 0..ITERS {
        let remaining = rng.below(0x1_0000) as u32;
        let original = rng.below(0x2_0000) as u32; // may be less, equal, or more
        let cd = Cooldown::new(remaining);
        let expected = original.saturating_sub(remaining);
        assert_eq!(
            cd.elapsed(original),
            expected,
            "elapsed({original}) ≠ {original}.saturating_sub({remaining})"
        );
    }
}

/// **fractional_progress endpoints are exact** — a fully-elapsed cooldown
/// (`remaining = 0`) must report exactly `Fixed::ONE`, and a fresh cooldown
/// (`remaining = original`) must report exactly `Fixed::ZERO`. These endpoints
/// are used as animation lerp parameters; any rounding drift would corrupt
/// start/end frames.
#[test]
fn prop_cooldown_fractional_progress_endpoints() {
    let mut rng = SplitMix64::new(0x1728_394A);
    // Case A: remaining == 0 → fully elapsed → progress == ONE
    for _ in 0..(ITERS / 2) {
        let original = 1 + rng.below(0xFFFF) as u32;
        let cd = Cooldown::ready();
        assert_eq!(
            cd.fractional_progress(original),
            Fixed::ONE,
            "ready cooldown must have progress=1.0 (original={original})"
        );
    }
    // Case B: remaining == original → not started → elapsed = 0 → progress == ZERO
    for _ in 0..(ITERS / 2) {
        let original = 1 + rng.below(0xFFFF) as u32;
        let cd = Cooldown::new(original);
        assert_eq!(
            cd.fractional_progress(original),
            Fixed::from_int(0),
            "fresh cooldown must have progress=0.0 (original={original})"
        );
    }
}

// ── TimerQueue properties ─────────────────────────────────────────────────────

/// **peek_next equals iterator minimum** — `peek_next()` must return the same
/// value as `iter().map(|(r, _)| r).min()`. Both scan the same `entries` Vec;
/// this catches any future divergence if peek_next is ever cached or computed
/// via a different traversal order.
#[test]
fn prop_timer_queue_peek_next_is_minimum() {
    let mut rng = SplitMix64::new(0x2839_4A5B);
    for _ in 0..ITERS {
        let mut q: TimerQueue<u32> = TimerQueue::new();
        let n = rng.below(12) as usize;
        for k in 0..n as u32 {
            q.schedule(rng.below(100) as u32, k);
        }
        let expected = q.iter().map(|(r, _)| r).min();
        assert_eq!(q.peek_next(), expected, "peek_next ≠ iter min (n={n})");
    }
}

/// **advance fires all due events** — every one-shot event scheduled with
/// `delay ≤ max_delay` must appear in the `Vec` returned by `advance(max_delay)`.
/// The `remaining ≤ ticks` condition is inclusive, so no event at exactly
/// `max_delay` ticks should be skipped. Non-vacuous: at least 1 event per trial.
#[test]
fn prop_timer_queue_advance_fires_all_due_events() {
    let mut rng = SplitMix64::new(0x394A_5B6C);
    for _ in 0..ITERS {
        let mut q: TimerQueue<u32> = TimerQueue::new();
        let n = 1 + rng.below(10) as u32;
        let max_delay = rng.below(50) as u32;
        for k in 0..n {
            // delay ∈ [0, max_delay] — guaranteed to fire in advance(max_delay)
            let delay = rng.below(max_delay.saturating_add(1)) as u32;
            q.schedule(delay, k);
        }
        let fired = q.advance(max_delay);
        assert_eq!(
            fired.len(),
            n as usize,
            "advance({max_delay}) fired {}, want {n}",
            fired.len()
        );
    }
}

/// **non-repeating entries removed after firing** — after `advance` fires all
/// due one-shot events, they must be absent from the queue. Repeating entries
/// (scheduled with `schedule_repeat`) survive by re-enqueuing themselves.
/// Every trial has ≥ 1 one-shot event (non-vacuous coverage of the removal path).
#[test]
fn prop_timer_queue_non_repeating_removed_after_fire() {
    let mut rng = SplitMix64::new(0x4A5B_6C7D);
    for _ in 0..ITERS {
        let mut q: TimerQueue<u32> = TimerQueue::new();
        let max_delay = rng.below(30) as u32;
        let n_oneshot = 1 + rng.below(5) as usize; // always ≥ 1
        let n_repeat = rng.below(4) as usize;
        for k in 0..n_oneshot as u32 {
            let delay = rng.below(max_delay.saturating_add(1)) as u32;
            q.schedule(delay, k);
        }
        for k in 0..n_repeat as u32 {
            let delay = rng.below(max_delay.saturating_add(1)) as u32;
            let period = 1 + rng.below(20) as u32;
            q.schedule_repeat(delay, period, k + 100);
        }
        let fired = q.advance(max_delay);
        assert_eq!(
            fired.len(),
            n_oneshot + n_repeat,
            "advance({max_delay}) fired {}, want {}",
            fired.len(),
            n_oneshot + n_repeat
        );
        // One-shot entries must be gone; only repeating entries survive.
        assert_eq!(
            q.len(),
            n_repeat,
            "after advance: expected {n_repeat} repeating, got {}",
            q.len()
        );
    }
}

// ── MultiMap properties ───────────────────────────────────────────────────────

/// **link_floors creates a bidirectional connector pair** — `link_floors(a, …,
/// b, …)` must add exactly one exit on floor `a` leading to `b` and exactly
/// one exit on floor `b` leading back to `a`. A unidirectional-only
/// implementation would break the standard staircase round-trip contract.
#[test]
fn prop_multimap_link_floors_is_bidirectional() {
    let mut rng = SplitMix64::new(0x5B6C_7D8E);
    for _ in 0..ITERS {
        let fa = mini_floor(rand_seed(&mut rng));
        let fb = mini_floor(rand_seed(&mut rng));
        let mut mm = MultiMap::new(vec![fa, fb], 0);
        assert!(mm.exits_from(0).is_empty(), "fresh floor 0 must have no exits");
        assert!(mm.exits_from(1).is_empty(), "fresh floor 1 must have no exits");
        mm.link_floors(0, 3, 3, 1, 5, 5);
        let exits0 = mm.exits_from(0);
        let exits1 = mm.exits_from(1);
        assert_eq!(exits0.len(), 1, "floor 0 must have exactly 1 exit after link_floors");
        assert_eq!(exits1.len(), 1, "floor 1 must have exactly 1 exit after link_floors");
        assert_eq!(exits0[0].to_floor, 1, "floor 0 exit must point to floor 1");
        assert_eq!(exits1[0].to_floor, 0, "floor 1 exit must point back to floor 0");
    }
}

/// **move_down then move_up is identity** — if `move_down()` succeeds from
/// floor `f`, then `move_up()` must return `true` and restore `current_floor`
/// to `f`. This is the canonical "staircase round-trip" used by all multi-floor
/// navigation code.
#[test]
fn prop_multimap_move_down_then_up_is_identity() {
    let mut rng = SplitMix64::new(0x6C7D_8E9F);
    for _ in 0..ITERS {
        let n = 2 + rng.below(4) as usize; // 2..5 floors
        let floors: Vec<Dungeon> = (0..n)
            .map(|i| mini_floor(rand_seed(&mut rng) ^ i as u64))
            .collect();
        let start = rng.below(n as u32) as u32;
        let mut mm = MultiMap::new(floors, start);
        let initial = mm.current_floor();
        if mm.move_down() {
            // Successfully moved to a deeper floor — move_up must restore us.
            assert!(mm.move_up(), "move_up must succeed after successful move_down");
            assert_eq!(
                mm.current_floor(),
                initial,
                "move_down+move_up did not restore floor {initial}"
            );
        } else {
            // Already at the last floor — current_floor must be unchanged.
            assert_eq!(
                mm.current_floor(),
                initial,
                "failed move_down changed current_floor"
            );
        }
    }
}

/// **find_floor_path to same floor returns empty path** — `find_floor_path(f, f)`
/// must return `Some(vec![])` for any valid floor index `f`. The range-check
/// in the BFS comes before the same-floor short-circuit, so out-of-range
/// indices still return `None` — but all in-range same-floor queries are free.
#[test]
fn prop_multimap_same_floor_path_is_empty() {
    let mut rng = SplitMix64::new(0x7D8E_9FA0);
    for _ in 0..ITERS {
        let n = 1 + rng.below(5) as usize; // 1..5 floors
        let floors: Vec<Dungeon> = (0..n).map(|_| mini_floor(rand_seed(&mut rng))).collect();
        let target = rng.below(n as u32) as u32;
        let mm = MultiMap::new(floors, 0);
        let path = mm.find_floor_path(target, target);
        assert_eq!(
            path,
            Some(Vec::new()),
            "find_floor_path({target},{target}) returned {path:?}, want Some([])"
        );
    }
}

// ---------------------------------------------------------------------------
// damage::ResistanceProfile — typed-damage algebraic laws
//
// The resistance model is integer-only and order-deterministic (no float, no
// HashMap), so its post-resistance damage obeys clean algebraic laws:
//   - non-negativity: the result is never negative for any resist (incl. < 0);
//   - True bypass: `DamageType::True` is the identity on `max(0, dmg)`;
//   - boundary pins: resist == 0 → unchanged, resist >= 100 → 0;
//   - monotonicity: result rises with damage, falls as resistance rises;
//   - vulnerability: resist < 0 amplifies (result >= dmg);
//   - oracle agreement: for resist in [0,100] it equals combat::apply_resistance.
// ---------------------------------------------------------------------------

/// The six resistible damage types in canonical order (everything but `True`).
const RESISTIBLE: [DamageType; 6] = [
    DamageType::Physical,
    DamageType::Fire,
    DamageType::Cold,
    DamageType::Lightning,
    DamageType::Poison,
    DamageType::Arcane,
];

fn rand_damage_type(rng: &mut SplitMix64) -> DamageType {
    DamageType::ALL[rng.below(DamageType::COUNT as u32) as usize]
}

fn rand_resistible_type(rng: &mut SplitMix64) -> DamageType {
    RESISTIBLE[rng.below(RESISTIBLE.len() as u32) as usize]
}

/// A resistance percentage spanning the whole meaningful range: deep
/// vulnerability, the [0,100] soak band, over-immunity, and the saturating
/// extremes — so laws that must hold "for any i32 resist" are actually probed
/// there, not just in the pretty range.
fn rand_resist(rng: &mut SplitMix64) -> i32 {
    match rng.below(8) {
        0 => i32::MIN,
        1 => i32::MAX,
        2 => -(rng.range(1, 500)),
        3 => 100,
        4 => 0,
        5 => rng.range(101, 100_000),
        _ => rng.range(0, 101),
    }
}

/// **apply is non-negative and True is the identity** — for *any* profile,
/// damage, and type, the post-resistance amount is `>= 0` (over-immunity never
/// becomes healing, and a huge vulnerability clamps at `i32::MAX` rather than
/// wrapping). For `DamageType::True` specifically, `apply` must return
/// `damage.max(0)` unchanged regardless of the profile.
#[test]
fn prop_damage_apply_non_negative_and_true_is_identity() {
    let mut rng = SplitMix64::new(0x0DA1_1A6E);
    for _ in 0..ITERS {
        let mut profile = ResistanceProfile::new();
        for ty in DamageType::ALL {
            profile.set(ty, rand_resist(&mut rng));
        }
        let damage = match rng.below(4) {
            0 => i32::MAX,
            1 => -(rng.range(0, 1000)),
            _ => rng.range(-50, 100_000),
        };
        let ty = rand_damage_type(&mut rng);
        let out = profile.apply(damage, ty);
        assert!(
            out >= 0,
            "apply({damage}, {ty:?}) = {out} is negative (resist={})",
            profile.get(ty)
        );
        // True is unconditionally the identity on the clamped input.
        assert_eq!(
            profile.apply(damage, DamageType::True),
            damage.max(0),
            "True damage must bypass the profile entirely"
        );
    }
}

/// **boundary pins: resist 0 is the identity, resist >= 100 is full immunity** —
/// a resistible type with resistance exactly 0 takes full (clamped) damage, and
/// any resistance `>= 100` (including over-immune 150 and saturating MAX) takes
/// exactly 0. `is_immune` must agree with the `>= 100` boundary.
#[test]
fn prop_damage_apply_boundary_pins() {
    let mut rng = SplitMix64::new(0x0DAB_011D);
    for _ in 0..ITERS {
        let ty = rand_resistible_type(&mut rng);
        let damage = rng.range(0, 100_000);

        let zero = ResistanceProfile::new().with(ty, 0);
        assert_eq!(
            zero.apply(damage, ty),
            damage,
            "resist=0 must pass {ty:?} damage through unchanged"
        );

        let immune_pct = match rng.below(3) {
            0 => 100,
            1 => rng.range(101, 100_000),
            _ => i32::MAX,
        };
        let immune = ResistanceProfile::new().with(ty, immune_pct);
        assert_eq!(
            immune.apply(damage, ty),
            0,
            "resist={immune_pct} (>=100) must fully soak {ty:?} damage"
        );
        assert!(
            immune.is_immune(ty),
            "is_immune must report true for resist={immune_pct}"
        );
    }
}

/// **monotonicity in damage and in resistance** — for a fixed resistible type:
///   (a) raising the incoming damage never lowers the post-resistance amount;
///   (b) raising the resistance percentage never raises the amount taken.
/// Integer truncation toward zero preserves both `<=` relations.
#[test]
fn prop_damage_apply_is_monotone() {
    let mut rng = SplitMix64::new(0x0DA3_070E);
    for _ in 0..ITERS {
        let ty = rand_resistible_type(&mut rng);

        // (a) monotone non-decreasing in damage at a fixed resistance.
        let resist = rand_resist(&mut rng);
        let profile = ResistanceProfile::new().with(ty, resist);
        let d0 = rng.range(0, 50_000);
        let d1 = d0 + rng.range(0, 50_000); // d1 >= d0
        let r0 = profile.apply(d0, ty);
        let r1 = profile.apply(d1, ty);
        assert!(
            r1 >= r0,
            "more damage gave less: apply({d1})={r1} < apply({d0})={r0} (resist={resist}, {ty:?})"
        );

        // (b) monotone non-increasing in resistance at a fixed damage.
        let damage = rng.range(0, 100_000);
        let lo = rng.range(-200, 100);
        let hi = lo + rng.range(0, 300); // hi >= lo
        let taken_lo = ResistanceProfile::new().with(ty, lo).apply(damage, ty);
        let taken_hi = ResistanceProfile::new().with(ty, hi).apply(damage, ty);
        assert!(
            taken_hi <= taken_lo,
            "more resistance let more through: resist {hi}->{taken_hi} > resist {lo}->{taken_lo} \
             (damage={damage}, {ty:?})"
        );
    }
}

/// **vulnerability amplifies; over-soak attenuates** — a negative resistance
/// (`is_vulnerable`) makes the target take at least as much as the raw damage,
/// while a positive resistance in (0,100] takes at most the raw damage. Both
/// stay within the `[0, i32::MAX]` clamp.
#[test]
fn prop_damage_vulnerability_amplifies_resistance_attenuates() {
    let mut rng = SplitMix64::new(0x0DAF_EE11);
    for _ in 0..ITERS {
        let ty = rand_resistible_type(&mut rng);
        let damage = rng.range(0, 40_000);

        let vuln_pct = -(rng.range(1, 400));
        let vuln = ResistanceProfile::new().with(ty, vuln_pct);
        assert!(vuln.is_vulnerable(ty), "resist={vuln_pct} should be vulnerable");
        assert!(
            vuln.apply(damage, ty) >= damage,
            "vulnerability must amplify: apply({damage})={} < {damage} (resist={vuln_pct}, {ty:?})",
            vuln.apply(damage, ty)
        );

        let soak_pct = rng.range(1, 101); // 1..=100
        let soak = ResistanceProfile::new().with(ty, soak_pct);
        assert!(
            soak.apply(damage, ty) <= damage,
            "soak must attenuate: apply({damage})={} > {damage} (resist={soak_pct}, {ty:?})",
            soak.apply(damage, ty)
        );
    }
}

/// **oracle agreement with combat::apply_resistance over [0,100]** — for the
/// percentage band the two subsystems share, typed damage must equal the flat
/// primitive exactly, so a creature can carry a `ResistanceProfile` while old
/// code calls `apply_resistance` and they never diverge. Also checks the
/// `index`/`from_index` round-trip that this agreement relies on for indexing.
#[test]
fn prop_damage_apply_matches_combat_oracle_and_index_roundtrips() {
    let mut rng = SplitMix64::new(0x0DA0_AC1E);
    for _ in 0..ITERS {
        let ty = rand_resistible_type(&mut rng);
        let resist = rng.range(0, 101) as u32; // 0..=100, the shared band
        let damage = rng.range(0, 100_000);
        let typed = ResistanceProfile::new()
            .with(ty, resist as i32)
            .apply(damage, ty);
        let flat = apply_resistance(damage, resist);
        assert_eq!(
            typed, flat,
            "typed/flat divergence: apply({damage},{ty:?},resist={resist})={typed} vs {flat}"
        );
        // index round-trips for every type, underpinning per-type array access.
        assert_eq!(DamageType::from_index(ty.index()), Some(ty));
    }
    assert_eq!(DamageType::from_index(DamageType::COUNT), None);
}

/// **`add` is saturating and never flips sign** — layering buffs/debuffs onto a
/// base profile with `add` must clamp at `i32::MIN`/`i32::MAX` rather than
/// wrapping (a wrap would silently turn a stack of resistances into a
/// vulnerability). The post-`add` value must lie on the same side of the true
/// (i64) sum, i.e. equal `saturating_add`.
#[test]
fn prop_damage_add_is_saturating() {
    let mut rng = SplitMix64::new(0x0DAA_DD5A);
    for _ in 0..ITERS {
        let ty = rand_resistible_type(&mut rng);
        let base = rand_resist(&mut rng);
        let delta = rand_resist(&mut rng);
        let mut profile = ResistanceProfile::new().with(ty, base);
        profile.add(ty, delta);
        let expected = (base as i64 + delta as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
        assert_eq!(
            profile.get(ty),
            expected,
            "add({base}, {delta}) on {ty:?} = {} != saturating {expected}",
            profile.get(ty)
        );
    }
}

// ---------------------------------------------------------------------------
// combat::Stats / StatsModifier — HP-bookkeeping and stat-modifier laws
//
// The combat primitives are all integer and saturating, so they obey clean
// invariants regardless of how extreme the inputs are:
//   - HP stays within [0, max_hp] across any sequence of mutators;
//   - overkill conserves damage (applied + overkill == requested);
//   - `modified` saturates and never heals;
//   - `base_damage` floors at 1 and is monotone in attack/defense;
//   - `melee_attack` deals exactly `base_damage`, clamped at 0 HP;
//   - `hp_percent` stays in [0,100] and tracks HP monotonically;
//   - `roll_damage` lands in [base, base+variance] and respects the draw contract;
//   - `splash_attack` deals >= 1 to each target, decreasing with falloff;
//   - `critical_strike` never deals less than `base_damage`.
// ---------------------------------------------------------------------------

/// A combatant with a moderate, possibly-damaged stat block. `attack`/`defense`
/// span negatives too, so the saturating/`max(1,_)` paths are exercised.
fn rand_stats(rng: &mut SplitMix64) -> Stats {
    let max_hp = rng.range(0, 500);
    let mut s = Stats::new(max_hp, rng.range(-50, 200), rng.range(-50, 200));
    s.take_damage(rng.range(0, max_hp + 1)); // leave HP somewhere in [0, max_hp]
    s
}

/// **HP stays in `[0, max_hp]` across any mutator sequence** — applying a random
/// mix of `take_damage` / `heal` / `set_max_hp` / `restore` / `clamp_hp` can
/// never leave `hp` negative or above the (current) `max_hp`. This is the core
/// safety invariant every other combat helper relies on.
#[test]
fn prop_stats_hp_stays_in_bounds() {
    let mut rng = SplitMix64::new(0x0C0A_B017);
    for _ in 0..ITERS {
        let mut s = rand_stats(&mut rng);
        for _ in 0..8 {
            match rng.below(5) {
                0 => s.take_damage(rng.range(-100, 1000)),
                1 => s.heal(rng.range(-100, 1000)),
                2 => s.set_max_hp(rng.range(-100, 600)),
                3 => s.restore(),
                _ => s.clamp_hp(),
            }
            assert!(
                s.hp >= 0 && s.hp <= s.max_hp,
                "hp {} out of [0, {}]",
                s.hp,
                s.max_hp
            );
            assert!(s.max_hp >= 0, "max_hp went negative: {}", s.max_hp);
        }
    }
}

/// **overkill conserves damage** — `take_overkill_damage(a)` floors HP at 0 and
/// returns the excess, so for the requested (clamped) amount `a' = max(0,a)`:
/// `applied + overkill == a'`, where `applied = hp_before − hp_after`. Negative
/// requests are a no-op (`applied = overkill = 0`).
#[test]
fn prop_stats_overkill_conserves_damage() {
    let mut rng = SplitMix64::new(0x0C0_0E711);
    for _ in 0..ITERS {
        let mut s = rand_stats(&mut rng);
        let before = s.hp;
        let amount = rng.range(-100, 1000);
        let overkill = s.take_overkill_damage(amount);
        let applied = before - s.hp;
        assert!(s.hp >= 0, "hp went negative: {}", s.hp);
        assert!(overkill >= 0, "overkill negative: {overkill}");
        assert_eq!(
            applied + overkill,
            amount.max(0),
            "applied {applied} + overkill {overkill} != requested {}",
            amount.max(0)
        );
    }
}

/// **`modified` saturates and never heals** — the resulting block has
/// `attack`/`defense` equal to the saturating add of base + modifier, `max_hp`
/// clamped to `>= 0`, and current `hp` clamped to the new ceiling and never
/// above the original `hp` (a modifier raises the ceiling but does not fill it).
#[test]
fn prop_stats_modified_saturates_and_never_heals() {
    let mut rng = SplitMix64::new(0x0C0_3D1F1);
    for _ in 0..ITERS {
        let base = rand_stats(&mut rng);
        let m = StatsModifier {
            attack: rng.range(i32::MIN / 2, i32::MAX / 2),
            defense: rng.range(i32::MIN / 2, i32::MAX / 2),
            max_hp: rng.range(-2000, 2000),
        };
        let out = base.modified(&m);
        assert_eq!(out.attack, base.attack.saturating_add(m.attack));
        assert_eq!(out.defense, base.defense.saturating_add(m.defense));
        let want_max = base.max_hp.saturating_add(m.max_hp).max(0);
        assert_eq!(out.max_hp, want_max, "max_hp not saturating-clamped");
        assert!(out.max_hp >= 0, "max_hp negative");
        assert!(out.hp <= out.max_hp, "hp {} above new max {}", out.hp, out.max_hp);
        assert!(out.hp <= base.hp, "modifier healed: {} > {}", out.hp, base.hp);
    }
}

/// **`base_damage` floors at 1 and is monotone** — at least 1 damage always,
/// and higher attack never lowers it while higher defense never raises it.
/// Uses i64 to predict the saturating subtraction without overflow.
#[test]
fn prop_base_damage_min_one_and_monotone() {
    let mut rng = SplitMix64::new(0x0C0_BA5ED);
    for _ in 0..ITERS {
        let att = rand_stats(&mut rng);
        let def = rand_stats(&mut rng);
        let d = base_damage(&att, &def);
        assert!(d >= 1, "base_damage {d} below floor 1");
        let predicted = (att.attack as i64 - def.defense as i64)
            .clamp(i32::MIN as i64, i32::MAX as i64)
            .max(1) as i32;
        assert_eq!(d, predicted, "base_damage diverged from saturating model");

        // Monotone: +attack never decreases, +defense never increases.
        let mut stronger = att.clone();
        stronger.attack = att.attack.saturating_add(rng.range(0, 100));
        assert!(base_damage(&stronger, &def) >= d, "more attack lowered damage");
        let mut tougher = def.clone();
        tougher.defense = def.defense.saturating_add(rng.range(0, 100));
        assert!(base_damage(&att, &tougher) <= d, "more defense raised damage");
    }
}

/// **`melee_attack` deals exactly `base_damage`, clamped at 0 HP** — the return
/// value equals the pre-attack `base_damage`, and the defender's HP drops by
/// `min(hp_before, dmg)` (never below 0).
#[test]
fn prop_melee_attack_deals_base_damage() {
    let mut rng = SplitMix64::new(0x0C0_3E1EE);
    for _ in 0..ITERS {
        let att = rand_stats(&mut rng);
        let mut def = rand_stats(&mut rng);
        let before = def.hp;
        let expected = base_damage(&att, &def);
        let dealt = melee_attack(&att, &mut def);
        assert_eq!(dealt, expected, "melee dealt != base_damage");
        assert_eq!(def.hp, (before - expected).max(0), "HP not clamped at 0");
    }
}

/// **`hp_percent` is bounded and HP-monotone** — always in `[0,100]`, and for a
/// fixed positive `max_hp`, a higher HP never yields a lower percentage.
#[test]
fn prop_hp_percent_bounded_and_monotone() {
    let mut rng = SplitMix64::new(0x0C0_9E2CE);
    for _ in 0..ITERS {
        let max_hp = rng.range(1, 100_000);
        let mut a = Stats::new(max_hp, 1, 1);
        let mut b = Stats::new(max_hp, 1, 1);
        let h0 = rng.range(0, max_hp + 1);
        let h1 = rng.range(0, max_hp + 1);
        let (lo, hi) = if h0 <= h1 { (h0, h1) } else { (h1, h0) };
        a.hp = lo;
        b.hp = hi;
        let pa = a.hp_percent();
        let pb = b.hp_percent();
        assert!(pa <= 100 && pb <= 100, "hp_percent exceeded 100: {pa}/{pb}");
        assert!(pb >= pa, "hp_percent not monotone: hp {hi}->{pb} < hp {lo}->{pa}");
    }
}

/// **`roll_damage` lands in `[base, base+variance]` and honours the draw
/// contract** — with `base >= 0` the result sits between `base` and
/// `base+variance` inclusive, and `variance == 0` consumes no RNG draw.
#[test]
fn prop_roll_damage_in_range_and_draw_contract() {
    let mut rng = SplitMix64::new(0x0C0_D1CE0);
    for _ in 0..ITERS {
        let base = rng.range(0, 1000);
        let variance = rng.range(0, 500) as u32;
        let dmg = roll_damage(&mut rng, base, variance);
        assert!(
            dmg >= base && dmg <= base + variance as i32,
            "roll_damage {dmg} outside [{base}, {}]",
            base + variance as i32
        );
        // Zero variance must not draw.
        let probe = rng.state();
        let d0 = roll_damage(&mut rng, base, 0);
        assert_eq!(d0, base, "zero-variance roll != base");
        assert_eq!(rng.state(), probe, "zero-variance roll consumed a draw");
    }
}

/// **`splash_attack` deals >= 1 and decays with falloff** — every target takes
/// at least 1 damage, the result length matches the target count, and with a
/// uniform defense the per-target damage is non-increasing (the falloff makes
/// later targets take no more than earlier ones).
#[test]
fn prop_splash_attack_min_one_and_non_increasing() {
    let mut rng = SplitMix64::new(0x0C05_71A5);
    for _ in 0..ITERS {
        let att = Stats::new(rng.range(0, 100), rng.range(0, 200), 0);
        let falloff = rng.range(0, 30);
        let n = rng.below(6) as usize;
        let def = rng.range(0, 50);
        let mut targets: Vec<Stats> = (0..n).map(|_| Stats::new(1000, 1, def)).collect();
        let dmgs = splash_attack(&att, &mut targets, falloff);
        assert_eq!(dmgs.len(), n, "result length != target count");
        for w in dmgs.windows(2) {
            assert!(w[1] <= w[0], "splash damage rose with falloff: {:?}", dmgs);
        }
        assert!(dmgs.iter().all(|&d| d >= 1), "a target took < 1 damage: {dmgs:?}");
    }
}

/// **`critical_strike` never deals less than `base_damage`** — a non-crit deals
/// exactly base, a crit multiplies by `max(1, mult)` (clamped to `i32::MAX`),
/// and `crit_chance >= 100` is always a crit while `<= 0` never is (and draws
/// no RNG in either degenerate case).
#[test]
fn prop_critical_strike_at_least_base_and_draw_contract() {
    let mut rng = SplitMix64::new(0x0C0_C211C);
    for _ in 0..ITERS {
        let att = rand_stats(&mut rng);
        let mut def = rand_stats(&mut rng);
        let base = base_damage(&att, &def);
        let mult = rng.range(-2, 5);

        // Degenerate chances: deterministic and drawless.
        let chance = if rng.below(2) == 0 { 200 } else { -5 };
        let probe = rng.state();
        let r = critical_strike(&mut rng, &att, &mut def, chance, mult);
        assert_eq!(rng.state(), probe, "degenerate crit chance consumed a draw");
        assert_eq!(r.critical, chance >= 100, "crit flag wrong for chance {chance}");
        assert!(r.damage >= base, "crit_strike {} below base {base}", r.damage);
        if r.critical {
            let want = (base as i64 * mult.max(1) as i64).min(i32::MAX as i64) as i32;
            assert_eq!(r.damage, want, "crit damage != base*max(1,mult) clamped");
        } else {
            assert_eq!(r.damage, base, "non-crit damage != base");
        }
    }
}

// ---------------------------------------------------------------------------
// turn::Scheduler — energy/speed turn-order laws
//
// The scheduler banks `speed` energy per time unit and lets an actor act once
// it reaches ACTION_COST, carrying the remainder over. Because time advances by
// closed-form integer ceil-division (no float), the queue obeys exact laws:
//   - a non-empty scheduler always yields an actor (no zero-speed stall);
//   - peek_next_turn is a non-destructive preview that matches next_turn and
//     picks the smallest ready id;
//   - time_until_ready equals the ceil-division formula and pins readiness;
//   - pending_count and actors_ready partition the actor set;
//   - energy is conserved: energy_i + cost·count_i == speed_i · U for a single
//     shared unit count U — the exact fairness identity;
//   - the turn sequence is fully deterministic for identical inputs.
// ---------------------------------------------------------------------------

/// Build a scheduler with `n` distinct `u32` ids (0..n) at random speeds.
fn rand_scheduler(rng: &mut SplitMix64, n: u32, max_speed: i32) -> Scheduler<u32> {
    let mut s = Scheduler::new();
    for id in 0..n {
        s.add(id, rng.range(1, max_speed + 1));
    }
    s
}

/// **a non-empty scheduler always yields a registered actor** — regardless of
/// speeds or (possibly negative) banked energy, `next_turn` advances time until
/// someone is ready and never returns `None` while actors remain; the id it
/// returns is always one that is registered. A zero/negative speed can never
/// stall the queue (speed is clamped to >= 1 on `add`).
#[test]
fn prop_scheduler_non_empty_always_acts() {
    let mut rng = SplitMix64::new(0x0701_AC75);
    for _ in 0..ITERS {
        let n = 1 + rng.below(5); // 1..=5 actors
        let mut s = rand_scheduler(&mut rng, n, 400);
        // Perturb starting energy, including negatives and over-ready values.
        for id in 0..n {
            if rng.below(2) == 0 {
                s.set_energy(id, rng.range(-300, 300));
            }
        }
        for _ in 0..40 {
            let acted = s.next_turn();
            assert!(acted.is_some(), "non-empty scheduler returned None");
            let id = acted.unwrap();
            assert!(id < n, "scheduler returned unregistered id {id}");
        }
    }
}

/// **peek_next_turn is a non-destructive, smallest-id-first preview** — peeking
/// never changes any actor's banked energy, returns `Some` iff some actor is
/// ready, picks the minimum ready id, and (when ready) equals the id the
/// following `next_turn` pops.
#[test]
fn prop_scheduler_peek_matches_next_and_is_nondestructive() {
    let mut rng = SplitMix64::new(0x0701_9EE5);
    for _ in 0..ITERS {
        let n = 1 + rng.below(5);
        let mut s = rand_scheduler(&mut rng, n, 300);
        for id in 0..n {
            s.set_energy(id, rng.range(-50, 250));
        }
        let before: Vec<Option<i32>> = (0..n).map(|id| s.energy(id)).collect();
        let peeked = s.peek_next_turn();
        let after: Vec<Option<i32>> = (0..n).map(|id| s.energy(id)).collect();
        assert_eq!(before, after, "peek_next_turn mutated banked energy");

        // peek is Some iff at least one actor is ready, and is the min ready id.
        let min_ready = (0..n)
            .filter(|&id| s.energy(id).unwrap() >= ACTION_COST)
            .min();
        assert_eq!(peeked, min_ready, "peek did not pick the smallest ready id");

        if let Some(p) = peeked {
            assert_eq!(s.next_turn(), Some(p), "next_turn disagreed with peek");
        }
    }
}

/// **time_until_ready matches the ceil-division formula and pins readiness** —
/// for a registered actor it equals `ceil(max(0, cost − energy) / speed)`, is
/// `0` exactly when the actor is already ready, and is `None` for an unknown id.
#[test]
fn prop_scheduler_time_until_ready_formula() {
    let mut rng = SplitMix64::new(0x0701_71ED);
    for _ in 0..ITERS {
        let n = 1 + rng.below(5);
        let mut s = rand_scheduler(&mut rng, n, 250);
        for id in 0..n {
            s.set_energy(id, rng.range(-200, 250));
        }
        for id in 0..n {
            let energy = s.energy(id).unwrap();
            let speed = s.speed(id).unwrap();
            let deficit = ACTION_COST - energy;
            let expected = if deficit <= 0 {
                0
            } else {
                (deficit + speed - 1) / speed
            };
            assert_eq!(s.time_until_ready(id), Some(expected), "formula mismatch");
            assert_eq!(
                s.time_until_ready(id) == Some(0),
                energy >= ACTION_COST,
                "time_until_ready==0 must match readiness for id {id}"
            );
        }
        assert_eq!(s.time_until_ready(n + 100), None, "unknown id must be None");
    }
}

/// **pending and ready partition the actor set** — at every point each actor is
/// either pending (energy < cost) or ready (energy >= cost), never both and
/// never neither, so `pending_count() + actors_ready().len() == len()` holds
/// across an arbitrary mix of mutations and turns.
#[test]
fn prop_scheduler_pending_ready_partition() {
    let mut rng = SplitMix64::new(0x0701_9A57);
    for _ in 0..ITERS {
        let n = 1 + rng.below(5);
        let mut s = rand_scheduler(&mut rng, n, 300);
        for _ in 0..10 {
            match rng.below(4) {
                0 => {
                    s.set_energy(rng.below(n), rng.range(-100, 250));
                }
                1 => {
                    s.next_turn();
                }
                2 => {
                    s.set_speed(rng.below(n), rng.range(1, 300));
                }
                _ => {
                    s.reset_actor(rng.below(n));
                }
            }
            assert_eq!(
                s.pending_count() + s.actors_ready().len(),
                s.len(),
                "pending + ready != total"
            );
        }
    }
}

/// **energy is conserved — the exact fairness identity** — starting every actor
/// at energy 0 and running `T` turns advances all actors by the *same* unit
/// count `U`, so `energy_i + cost·count_i == speed_i·U` for each actor. Hence
/// for any pair `(i, j)`:
/// `speed_j·(energy_i + cost·count_i) == speed_i·(energy_j + cost·count_j)`.
/// This is the precise statement of "faster actors act proportionally more".
/// Speeds are kept moderate so no saturation perturbs the identity.
#[test]
fn prop_scheduler_energy_is_conserved() {
    let mut rng = SplitMix64::new(0x0701_C047);
    for _ in 0..ITERS {
        let n = 2 + rng.below(4); // 2..=5 actors, all fresh at energy 0
        let mut s = rand_scheduler(&mut rng, n, 100);
        let mut counts = vec![0i64; n as usize];
        for _ in 0..300 {
            let id = s.next_turn().unwrap();
            counts[id as usize] += 1;
        }
        // bank_i = energy_i + cost*count_i = speed_i * U (same U for all actors).
        let bank: Vec<i64> = (0..n)
            .map(|id| s.energy(id).unwrap() as i64 + ACTION_COST as i64 * counts[id as usize])
            .collect();
        let speeds: Vec<i64> = (0..n).map(|id| s.speed(id).unwrap() as i64).collect();
        for i in 0..n as usize {
            for j in 0..n as usize {
                assert_eq!(
                    bank[i] * speeds[j],
                    bank[j] * speeds[i],
                    "energy conservation broken between {i} and {j}"
                );
            }
        }
    }
}

/// **the turn sequence is fully deterministic** — two schedulers built from the
/// same ids/speeds and driven the same number of times produce identical
/// output sequences (the lockstep-replay guarantee for turn order).
#[test]
fn prop_scheduler_sequence_is_deterministic() {
    let mut rng = SplitMix64::new(0x0701_DE73);
    for _ in 0..(ITERS / 4) {
        let n = 1 + rng.below(5);
        let speeds: Vec<i32> = (0..n).map(|_| rng.range(1, 400)).collect();
        let build = || {
            let mut s = Scheduler::new();
            for (id, &sp) in speeds.iter().enumerate() {
                s.add(id as u32, sp);
            }
            s
        };
        let mut a = build();
        let mut b = build();
        let seq_a: Vec<Option<u32>> = (0..80).map(|_| a.next_turn()).collect();
        let seq_b: Vec<Option<u32>> = (0..80).map(|_| b.next_turn()).collect();
        assert_eq!(seq_a, seq_b, "identical schedulers diverged");
    }
}

// ---------------------------------------------------------------------------
// status::StatusSet — timed buff/debuff bookkeeping laws
//
// Effects carry an unsigned remaining duration and a signed magnitude. Because
// tick subtracts whole units and the accumulators are saturating, the set obeys
// exact laws:
//   - tick conserves the actor set (survivors + expired == before) and lowers
//     each survivor's remaining by exactly the tick amount;
//   - ticks compose: tick(a) then tick(b) leaves the same surviving durations
//     as tick(a+b);
//   - re-apply stacks to the max duration and replaces the magnitude;
//   - stats_modifier is the per-target saturating sum of magnitudes;
//   - dot_total is the non-negative saturating sum over DoT keys;
//   - the canonical hash and aggregate queries are application-order-independent.
// ---------------------------------------------------------------------------

/// Map a `u32` key to a stat target by `key % 4` (the 4th class is a pure DoT
/// with no stat target), used by the `stats_modifier` law below.
fn status_target_of(k: &u32) -> Option<StatTarget> {
    match k % 4 {
        0 => Some(StatTarget::Attack),
        1 => Some(StatTarget::Defense),
        2 => Some(StatTarget::MaxHp),
        _ => None,
    }
}

/// Build a status set with `n` distinct keys (0..n), random durations >= 1 and
/// signed magnitudes.
fn rand_status(rng: &mut SplitMix64, n: u32) -> StatusSet<u32> {
    let mut s = StatusSet::new();
    for k in 0..n {
        s.apply(k, 1 + rng.below(60), rng.range(-100, 100));
    }
    s
}

/// **tick conserves the set and decrements survivors exactly** — after
/// `tick(n)`: the returned expired keys are exactly those whose remaining was
/// `<= n` (in application order), every survivor's remaining drops by exactly
/// `n` and stays `> 0`, and `survivors + expired == original count`.
#[test]
fn prop_status_tick_duration_accounting() {
    let mut rng = SplitMix64::new(0x57A7_05A0);
    for _ in 0..ITERS {
        let n = 1 + rng.below(8);
        let mut s = rand_status(&mut rng, n);
        let before: Vec<(u32, u32)> = s.iter().map(|(k, e)| (*k, e.remaining)).collect();
        let ticks = rng.below(70);
        let expired = s.tick(ticks);

        let expected_expired: Vec<u32> = before
            .iter()
            .filter(|(_, r)| *r <= ticks)
            .map(|(k, _)| *k)
            .collect();
        assert_eq!(expired, expected_expired, "expired set/order wrong");

        for (k, r) in &before {
            if *r > ticks {
                assert_eq!(s.remaining_of(k), r - ticks, "survivor not decremented by n");
                assert!(s.remaining_of(k) > 0, "survivor remaining hit 0");
            } else {
                assert!(!s.is_active(k), "expired effect still active");
            }
        }
        assert_eq!(s.len() + expired.len(), before.len(), "count not conserved");
    }
}

/// **ticks compose** — splitting a tick into two (`tick(a)` then `tick(b)`)
/// leaves the same surviving key→remaining map as a single `tick(a+b)`, since an
/// effect survives both iff its remaining exceeds `a+b`. A metamorphic law that
/// pins the duration arithmetic independent of how time is chunked.
#[test]
fn prop_status_tick_composition() {
    let mut rng = SplitMix64::new(0x57A7_C0F0);
    let sorted_map = |s: &StatusSet<u32>| -> Vec<(u32, u32)> {
        let mut v: Vec<(u32, u32)> = s.iter().map(|(k, e)| (*k, e.remaining)).collect();
        v.sort_unstable();
        v
    };
    for _ in 0..ITERS {
        let n = 1 + rng.below(8);
        let base = rand_status(&mut rng, n);
        let a = rng.below(40);
        let b = rng.below(40);

        let mut split = base.clone();
        split.tick(a);
        split.tick(b);

        let mut whole = base.clone();
        whole.tick(a + b);

        assert_eq!(
            sorted_map(&split),
            sorted_map(&whole),
            "tick(a) then tick(b) != tick(a+b)"
        );
    }
}

/// **re-apply stacks to the max duration and replaces the magnitude** — applying
/// an existing key never shortens its remaining (takes the max of old and new)
/// and adopts the newest magnitude, leaving the active-effect count unchanged.
#[test]
fn prop_status_apply_refresh_takes_max_duration() {
    let mut rng = SplitMix64::new(0x57A7_3EF5);
    for _ in 0..ITERS {
        let mut s = StatusSet::new();
        let d1 = 1 + rng.below(50);
        let m1 = rng.range(-50, 50);
        let d2 = 1 + rng.below(50);
        let m2 = rng.range(-50, 50);
        s.apply(7u32, d1, m1);
        s.apply(7u32, d2, m2);
        assert_eq!(s.len(), 1, "re-apply must not duplicate the key");
        assert_eq!(s.remaining_of(&7), d1.max(d2), "duration must be the max");
        assert_eq!(s.magnitude_of(&7), m2, "magnitude must be the newest");
    }
}

/// **stats_modifier is the per-target saturating sum** — each field of the
/// folded `StatsModifier` equals the saturating sum of the magnitudes of all
/// active effects mapped to that target; `None`-mapped keys (DoTs) contribute
/// nothing.
#[test]
fn prop_status_stats_modifier_is_grouped_sum() {
    let mut rng = SplitMix64::new(0x57A7_5717);
    for _ in 0..ITERS {
        let n = 1 + rng.below(8);
        let s = rand_status(&mut rng, n);
        let m = s.stats_modifier(status_target_of);

        let want = |t: StatTarget| -> i32 {
            s.iter()
                .filter(|(k, _)| status_target_of(k) == Some(t))
                .fold(0i32, |acc, (_, e)| acc.saturating_add(e.magnitude))
        };
        assert_eq!(m.attack, want(StatTarget::Attack), "attack sum wrong");
        assert_eq!(m.defense, want(StatTarget::Defense), "defense sum wrong");
        assert_eq!(m.max_hp, want(StatTarget::MaxHp), "max_hp sum wrong");
    }
}

/// **dot_total is the non-negative saturating sum over DoT keys** — it equals
/// `max(0, Σ magnitude)` over the keys for which `is_dot` is true, and is never
/// negative (a stray negative-magnitude DoT can never heal through this path).
#[test]
fn prop_status_dot_total_non_negative_sum() {
    let mut rng = SplitMix64::new(0x57A7_D077);
    for _ in 0..ITERS {
        let n = 1 + rng.below(8);
        let s = rand_status(&mut rng, n);
        let is_dot = |k: &u32| *k % 2 == 0;
        let total = s.dot_total(is_dot);

        let raw = s
            .iter()
            .filter(|(k, _)| is_dot(k))
            .fold(0i32, |acc, (_, e)| acc.saturating_add(e.magnitude));
        assert_eq!(total, raw.max(0), "dot_total != max(0, sum)");
        assert!(total >= 0, "dot_total negative: {total}");
    }
}

/// **canonical hash and aggregates are application-order-independent** — applying
/// the same distinct-key effects in forward vs reversed order yields an equal
/// `DetHash` and equal `total_magnitude` / `magnitude_range` / `min`/`max`
/// remaining (the order-canonicalisation guarantee for replay checksums).
#[test]
fn prop_status_hash_and_aggregates_order_independent() {
    use izanagi_kit::hash_state;
    let mut rng = SplitMix64::new(0x57A7_0DE7);
    for _ in 0..(ITERS / 2) {
        let n = 1 + rng.below(8);
        let effects: Vec<(u32, u32, i32)> = (0..n)
            .map(|k| (k, 1 + rng.below(60), rng.range(-100, 100)))
            .collect();

        let mut fwd = StatusSet::new();
        for &(k, d, m) in &effects {
            fwd.apply(k, d, m);
        }
        let mut rev = StatusSet::new();
        for &(k, d, m) in effects.iter().rev() {
            rev.apply(k, d, m);
        }

        assert_eq!(hash_state(&fwd), hash_state(&rev), "hash depends on apply order");
        assert_eq!(fwd.total_magnitude(), rev.total_magnitude(), "total differs");
        assert_eq!(fwd.magnitude_range(), rev.magnitude_range(), "range differs");
        assert_eq!(fwd.max_remaining(), rev.max_remaining(), "max_remaining differs");
        assert_eq!(fwd.min_remaining(), rev.min_remaining(), "min_remaining differs");
    }
}

// ---------------------------------------------------------------------------
// inventory::Inventory — slot-based item bookkeeping laws
//
// Items live in fixed-capacity optional slots; add fills the first free slot,
// remove leaves a gap, and the slot layout is stable. The container obeys exact
// laws regardless of fill/remove history:
//   - the occupancy counters all agree and stay within [0, capacity];
//   - add fills first_empty_slot and increments len (or returns None when full);
//   - add-then-remove of the returned slot is an exact round-trip (hash-stable);
//   - swap is an involution and preserves the item multiset;
//   - move_to_slot succeeds exactly on (in-bounds, from occupied, to free) and
//     never mutates state on failure;
//   - remove_where_indexed agrees with find + remove.
// ---------------------------------------------------------------------------

/// An inventory of `cap` `u32` slots churned with a random add/remove history,
/// so it carries gaps and is rarely full or empty.
fn rand_inventory(rng: &mut SplitMix64, cap: usize) -> Inventory<u32> {
    let mut inv = Inventory::new(cap);
    for _ in 0..(cap * 2 + 2) {
        if rng.below(3) == 0 {
            inv.remove(rng.below(cap as u32) as usize);
        } else {
            inv.add(rng.below(1000));
        }
    }
    inv
}

/// **occupancy counters agree and stay in `[0, capacity]`** — across an
/// arbitrary add/remove history, `len == count_occupied == filled_slots().len()
/// == iter().count()`, the count never leaves `[0, capacity]`, the boolean
/// predicates match the count, and `filled_slots` is exactly the ascending list
/// of occupied indices.
#[test]
fn prop_inventory_occupancy_invariants() {
    let mut rng = SplitMix64::new(0x141E_0000);
    for _ in 0..ITERS {
        let cap = 1 + rng.below(8) as usize;
        let inv = rand_inventory(&mut rng, cap);
        let len = inv.len();
        assert_eq!(len, inv.count_occupied(), "len != count_occupied");
        assert_eq!(len, inv.iter().count(), "len != iter count");
        let filled = inv.filled_slots();
        assert_eq!(len, filled.len(), "len != filled_slots len");
        assert!(len <= cap, "len {len} exceeds capacity {cap}");
        assert_eq!(inv.is_empty(), len == 0, "is_empty mismatch");
        assert_eq!(inv.is_full(), len == cap, "is_full mismatch");
        assert_eq!(inv.has_space(), len < cap, "has_space mismatch");
        // filled_slots ascending and every listed slot is occupied.
        for w in filled.windows(2) {
            assert!(w[0] < w[1], "filled_slots not ascending: {filled:?}");
        }
        for &i in &filled {
            assert!(inv.get(i).is_some(), "filled slot {i} is empty");
        }
    }
}

/// **add fills `first_empty_slot` and increments len; full inventories reject** —
/// when there is space, `add` returns exactly the index `first_empty_slot`
/// reported, stores the item there, and raises `len` by one; when full, `add`
/// returns `None` and leaves `len` untouched.
#[test]
fn prop_inventory_add_fills_first_empty() {
    let mut rng = SplitMix64::new(0x141E_ADD1);
    for _ in 0..ITERS {
        let cap = 1 + rng.below(8) as usize;
        let mut inv = rand_inventory(&mut rng, cap);
        let len_before = inv.len();
        let pre_empty = inv.first_empty_slot();
        let item = rng.below(1000);
        let result = inv.add(item);
        if let Some(slot) = result {
            assert_eq!(Some(slot), pre_empty, "add did not use first_empty_slot");
            assert_eq!(inv.get(slot), Some(&item), "item not stored at returned slot");
            assert_eq!(inv.len(), len_before + 1, "len did not increment");
        } else {
            assert!(inv.is_full(), "add returned None on a non-full inventory");
            assert_eq!(inv.len(), len_before, "rejected add changed len");
            assert_eq!(pre_empty, None, "first_empty_slot disagreed with full add");
        }
    }
}

/// **add-then-remove is an exact, hash-stable round-trip** — adding an item then
/// removing the slot `add` returned restores the original item and leaves the
/// inventory bit-identical (same `DetHash`) to before the add, because `add`
/// fills the first empty slot and `remove` clears exactly that slot.
#[test]
fn prop_inventory_add_remove_round_trip() {
    use izanagi_kit::hash_state;
    let mut rng = SplitMix64::new(0x141E_3217);
    for _ in 0..ITERS {
        let cap = 1 + rng.below(8) as usize;
        let mut inv = rand_inventory(&mut rng, cap);
        if inv.is_full() {
            continue; // no free slot to round-trip through
        }
        let before = hash_state(&inv);
        let item = rng.below(1000);
        let slot = inv.add(item).expect("non-full inventory must accept add");
        assert_eq!(inv.remove(slot), Some(item), "remove did not return the item");
        assert_eq!(hash_state(&inv), before, "add+remove was not identity");
    }
}

/// **swap is an involution and preserves the item multiset** — `swap(a,b)` keeps
/// the occupied count and the sorted multiset of items unchanged (it only moves
/// items between slots), and applying it twice restores the exact state.
/// Out-of-bounds indices are clamped, so the law holds for any indices.
#[test]
fn prop_inventory_swap_involution_and_multiset() {
    use izanagi_kit::hash_state;
    let mut rng = SplitMix64::new(0x141E_57A9);
    for _ in 0..ITERS {
        let cap = 1 + rng.below(8) as usize;
        let mut inv = rand_inventory(&mut rng, cap);
        let before = hash_state(&inv);
        let mut items_before: Vec<u32> = inv.iter().map(|(_, &v)| v).collect();
        items_before.sort_unstable();
        let len_before = inv.len();

        let a = rng.below((cap + 2) as u32) as usize;
        let b = rng.below((cap + 2) as u32) as usize;
        inv.swap(a, b);
        assert_eq!(inv.len(), len_before, "swap changed occupancy");
        let mut items_after: Vec<u32> = inv.iter().map(|(_, &v)| v).collect();
        items_after.sort_unstable();
        assert_eq!(items_before, items_after, "swap altered the item multiset");

        inv.swap(a, b); // involution
        assert_eq!(hash_state(&inv), before, "swap was not its own inverse");
    }
}

/// **move_to_slot succeeds exactly on a legal move and is a no-op otherwise** —
/// it returns `true` iff both indices are in bounds, `from` is occupied, and
/// `to` is either equal to `from` or empty. On success the item relocates,
/// `from` becomes empty (unless `from == to`), and `len` is unchanged. On
/// failure the inventory is left bit-identical.
#[test]
fn prop_inventory_move_to_slot_semantics() {
    use izanagi_kit::hash_state;
    let mut rng = SplitMix64::new(0x141E_30E5);
    for _ in 0..ITERS {
        let cap = 1 + rng.below(8) as usize;
        let mut inv = rand_inventory(&mut rng, cap);
        let from = rng.below((cap + 2) as u32) as usize;
        let to = rng.below((cap + 2) as u32) as usize;

        let in_bounds = from < cap && to < cap;
        let from_item = inv.get(from).copied();
        let to_empty = to < cap && inv.get(to).is_none();
        let expected = in_bounds && from_item.is_some() && (from == to || to_empty);

        let before = hash_state(&inv);
        let len_before = inv.len();
        let ok = inv.move_to_slot(from, to);
        assert_eq!(ok, expected, "move_to_slot({from},{to}) result wrong");

        if ok {
            assert_eq!(inv.len(), len_before, "successful move changed len");
            assert_eq!(inv.get(to).copied(), from_item, "item not at destination");
            if from != to {
                assert!(inv.get(from).is_none(), "source slot not vacated");
            }
        } else {
            assert_eq!(hash_state(&inv), before, "failed move mutated state");
        }
    }
}

/// **remove_where_indexed agrees with find + remove** — it returns `(slot, item)`
/// where `slot` is exactly `find(pred)`, leaves that slot empty, drops `len` by
/// one, and the returned item is the one `get(slot)` held; a non-match returns
/// `None` and leaves the inventory bit-identical.
#[test]
fn prop_inventory_remove_where_indexed_matches_find() {
    use izanagi_kit::hash_state;
    let mut rng = SplitMix64::new(0x141E_F147);
    for _ in 0..ITERS {
        let cap = 1 + rng.below(8) as usize;
        let mut inv = rand_inventory(&mut rng, cap);
        let threshold = rng.below(1000);
        let pred = |v: &u32| *v >= threshold;

        let expected_slot = inv.find(pred);
        let expected_item = expected_slot.and_then(|s| inv.get(s).copied());
        let len_before = inv.len();
        let before = hash_state(&inv);

        match inv.remove_where_indexed(pred) {
            Some((slot, item)) => {
                assert_eq!(Some(slot), expected_slot, "removed slot != find()");
                assert_eq!(Some(item), expected_item, "removed item != get(slot)");
                assert!(inv.get(slot).is_none(), "slot not emptied after removal");
                assert_eq!(inv.len(), len_before - 1, "len did not drop by one");
            }
            None => {
                assert_eq!(expected_slot, None, "None despite a matching item");
                assert_eq!(hash_state(&inv), before, "no-match removal mutated state");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// tilemap::TileMap / LayeredMap — grid structural laws
//
// A TileMap is a row-major grid with panic-free OOB handling and a set of
// geometric transforms. These obey exact laws:
//   - flip_h / flip_v are involutions that preserve the cell multiset;
//   - rotating CW four times (or CW then CCW) is the identity; each rotation
//     swaps the dimensions and preserves the cell multiset;
//   - copy_region then paste_region round-trips a sub-rectangle exactly;
//   - iter is row-major and consistent with get/contains/count/find/any/all;
//   - in-bounds set is observable, OOB set is a hash-stable no-op, and swap is
//     an involution that preserves the multiset;
//   - LayeredMap layers are independent and share one set of dimensions.
// ---------------------------------------------------------------------------

/// A small random tile grid (1..6 per side) with cell values in 0..5.
fn rand_tilemap(rng: &mut SplitMix64) -> TileMap<u8> {
    let w = 1 + rng.below(6);
    let h = 1 + rng.below(6);
    let mut m = TileMap::new(w, h, 0u8);
    for (_, _, c) in m.iter_mut() {
        *c = rng.below(5) as u8;
    }
    m
}

/// The sorted multiset of a map's cells — invariant under any pure rearrangement
/// (flip, rotate, swap).
fn tile_multiset(m: &TileMap<u8>) -> Vec<u8> {
    let mut v: Vec<u8> = m.iter().map(|(_, _, &c)| c).collect();
    v.sort_unstable();
    v
}

/// **flip_h and flip_v are multiset-preserving involutions** — flipping twice on
/// either axis restores the exact grid (same `DetHash`), dimensions never
/// change, and a single flip only rearranges cells (the sorted multiset is
/// unchanged).
#[test]
fn prop_tilemap_flip_involutions() {
    use izanagi_kit::hash_state;
    let mut rng = SplitMix64::new(0x7113_F11D);
    for _ in 0..ITERS {
        let mut m = rand_tilemap(&mut rng);
        let (w, h) = (m.width(), m.height());
        let before = hash_state(&m);
        let ms = tile_multiset(&m);

        m.flip_h();
        assert_eq!((m.width(), m.height()), (w, h), "flip_h changed dimensions");
        assert_eq!(tile_multiset(&m), ms, "flip_h altered the cell multiset");
        m.flip_h();
        assert_eq!(hash_state(&m), before, "flip_h is not an involution");

        m.flip_v();
        assert_eq!(tile_multiset(&m), ms, "flip_v altered the cell multiset");
        m.flip_v();
        assert_eq!(hash_state(&m), before, "flip_v is not an involution");
    }
}

/// **rotation is a dimension-swapping, multiset-preserving symmetry** — rotating
/// 90° CW four times returns the original grid, CW followed by CCW is the
/// identity, every single rotation swaps `(w, h)` and keeps the cell multiset.
#[test]
fn prop_tilemap_rotation_round_trips() {
    use izanagi_kit::hash_state;
    let mut rng = SplitMix64::new(0x7113_0747);
    for _ in 0..ITERS {
        let m = rand_tilemap(&mut rng);
        let before = hash_state(&m);
        let (w, h) = (m.width(), m.height());
        let ms = tile_multiset(&m);

        let once = m.rotated_cw();
        assert_eq!((once.width(), once.height()), (h, w), "CW did not swap dims");
        assert_eq!(tile_multiset(&once), ms, "CW altered the multiset");

        let four = once.rotated_cw().rotated_cw().rotated_cw();
        assert_eq!(hash_state(&four), before, "four CW rotations != identity");

        let there_and_back = m.rotated_cw().rotated_ccw();
        assert_eq!(hash_state(&there_and_back), before, "CW then CCW != identity");
    }
}

/// **copy_region then paste_region round-trips a sub-rectangle** — copying an
/// in-bounds `rw×rh` region, pasting it into a fresh `rw×rh` map at the origin,
/// and copying it back reproduces the original data exactly, and each datum
/// matches the source cell it came from.
#[test]
fn prop_tilemap_copy_paste_round_trip() {
    let mut rng = SplitMix64::new(0x7113_C0FA);
    for _ in 0..ITERS {
        let m = rand_tilemap(&mut rng);
        let rw = 1 + rng.below(m.width());
        let rh = 1 + rng.below(m.height());
        let x = rng.below(m.width() - rw + 1) as i32;
        let y = rng.below(m.height() - rh + 1) as i32;

        let data = m.copy_region(x, y, rw as i32, rh as i32, 0);
        assert_eq!(data.len(), (rw * rh) as usize, "copied region wrong length");
        // each datum matches the source cell.
        for row in 0..rh as i32 {
            for col in 0..rw as i32 {
                assert_eq!(
                    data[(row * rw as i32 + col) as usize],
                    *m.get(x + col, y + row).unwrap(),
                    "copied cell mismatch at ({col},{row})"
                );
            }
        }

        let mut dst = TileMap::new(rw, rh, 0u8);
        dst.paste_region(0, 0, rw as i32, rh as i32, &data);
        let back = dst.copy_region(0, 0, rw as i32, rh as i32, 0);
        assert_eq!(data, back, "copy→paste→copy was not identity");
    }
}

/// **iter is row-major and consistent with the query API** — `iter` yields
/// exactly `w·h` cells whose coordinates are in bounds and round-trip through
/// `get`; `find_all` equals the row-major filtered coordinates, `count_where`
/// equals that length, `any_where`/`all_where` agree with it, and `find_first`
/// is the first matching coordinate.
#[test]
fn prop_tilemap_iter_consistency() {
    let mut rng = SplitMix64::new(0x7113_17E5);
    for _ in 0..ITERS {
        let m = rand_tilemap(&mut rng);
        let n = (m.width() * m.height()) as usize;
        assert_eq!(m.iter().count(), n, "iter count != w*h");
        assert_eq!(m.len(), n, "len != w*h");
        for (x, y, t) in m.iter() {
            assert_eq!(m.get(x, y), Some(t), "iter/get disagree at ({x},{y})");
            assert!(m.contains(x, y), "iter yielded OOB coord ({x},{y})");
        }

        let threshold = rng.below(5) as u8;
        let pred = |t: &u8| *t >= threshold;
        let expected: Vec<(i32, i32)> = m
            .iter()
            .filter(|(_, _, t)| pred(t))
            .map(|(x, y, _)| (x, y))
            .collect();
        assert_eq!(m.find_all(pred), expected, "find_all != row-major filter");
        assert_eq!(m.count_where(pred), expected.len(), "count_where != match count");
        assert_eq!(m.any_where(pred), !expected.is_empty(), "any_where mismatch");
        assert_eq!(m.all_where(pred), expected.len() == n, "all_where mismatch");
        assert_eq!(m.find_first(pred), expected.first().copied(), "find_first mismatch");
    }
}

/// **set is observable in bounds, a hash-stable no-op out of bounds, and swap is
/// a multiset-preserving involution** — an in-bounds `set` is read back by
/// `get`; OOB sets leave the grid bit-identical and `get` returns `None`;
/// swapping a pair of cells twice restores the grid and never changes the
/// multiset.
#[test]
fn prop_tilemap_set_get_swap_laws() {
    use izanagi_kit::hash_state;
    let mut rng = SplitMix64::new(0x7113_5E75);
    for _ in 0..ITERS {
        let mut m = rand_tilemap(&mut rng);
        let w = m.width() as i32;
        let h = m.height() as i32;

        let x = rng.below(m.width()) as i32;
        let y = rng.below(m.height()) as i32;
        let v = rng.below(200) as u8;
        m.set(x, y, v);
        assert_eq!(m.get(x, y), Some(&v), "in-bounds set not observable");

        let before_oob = hash_state(&m);
        m.set(-1, y, 250);
        m.set(x, h, 250);
        m.set(w + 5, h + 5, 250);
        assert_eq!(hash_state(&m), before_oob, "OOB set mutated the grid");
        assert!(m.get(-1, 0).is_none() && m.get(w, 0).is_none(), "OOB get not None");

        let ms = tile_multiset(&m);
        let before_swap = hash_state(&m);
        let x2 = rng.below(m.width()) as i32;
        let y2 = rng.below(m.height()) as i32;
        m.swap(x, y, x2, y2);
        assert_eq!(tile_multiset(&m), ms, "swap altered the multiset");
        m.swap(x, y, x2, y2);
        assert_eq!(hash_state(&m), before_swap, "swap is not an involution");
    }
}

/// **LayeredMap layers are independent and uniformly sized** — writing to one
/// layer never changes another layer's cell at the same position; `get` routes
/// to the right layer; an out-of-range layer index is `None`; every layer shares
/// the map's `(width, height)`; and `fill_all` writes through to every layer.
#[test]
fn prop_layered_map_layers_independent() {
    let mut rng = SplitMix64::new(0x7113_1A7E);
    for _ in 0..ITERS {
        let w = 1 + rng.below(5);
        let h = 1 + rng.below(5);
        let lc = 1 + rng.below(4) as usize;
        let mut m = LayeredMap::new(w, h, lc, 0u8);

        let li = rng.below(lc as u32) as usize;
        let x = rng.below(w) as i32;
        let y = rng.below(h) as i32;
        m.set(li, x, y, 9);
        assert_eq!(m.get(li, x, y), Some(&9), "set/get on target layer disagree");
        for other in 0..lc {
            if other != li {
                assert_eq!(m.get(other, x, y), Some(&0), "layer {other} leaked a write");
            }
            let layer = m.layer(other).expect("layer in range");
            assert_eq!((layer.width(), layer.height()), (w, h), "layer dims differ");
        }
        assert!(m.layer(lc).is_none(), "out-of-range layer not None");
        assert!(m.get(lc, 0, 0).is_none(), "out-of-range get not None");

        m.fill_all(3);
        for i in 0..lc {
            assert_eq!(m.get(i, x, y), Some(&3), "fill_all skipped layer {i}");
        }
    }
}

// ---------------------------------------------------------------------------
// dice::Dice — tabletop dice-notation laws
//
// A Dice expression rolls `count` dice of `sides` faces plus a flat modifier.
// Because rolls draw from a seeded SplitMix64 and the bound/average maths is
// integer (i128 internally), the type obeys exact laws:
//   - every roll lands in [min(), max()] and is seed-deterministic;
//   - average_x100 matches its closed form and sits between 100·min and 100·max;
//   - span == max − min == count·(sides−1), independent of the modifier;
//   - advantage dominates disadvantage over the same draws, both in bounds;
//   - keep-highest sums lie in [keep·min, keep·max] and the degenerate case
//     returns the modifier without drawing;
//   - to_string round-trips through parse;
//   - a flat die (sides <= 1) is constant at min == max.
// ---------------------------------------------------------------------------

/// A dice expression with a small count, a real (>= 1) face count, and a signed
/// modifier — chosen to exercise negative results without saturation.
fn rand_dice(rng: &mut SplitMix64) -> Dice {
    Dice::new(rng.below(20), 1 + rng.below(50), rng.range(-100, 100))
}

/// **every roll lands in `[min, max]` and is seed-deterministic** — across many
/// rolls of random expressions the result never escapes the static bounds, and
/// two RNGs seeded identically produce the same roll sequence (replay safety).
#[test]
fn prop_dice_roll_within_bounds_and_deterministic() {
    let mut rng = SplitMix64::new(0x0D1C_E001);
    for _ in 0..ITERS {
        let d = rand_dice(&mut rng);
        let seed = rand_seed(&mut rng);
        let mut r1 = SplitMix64::new(seed);
        let mut r2 = SplitMix64::new(seed);
        for _ in 0..10 {
            let a = d.roll(&mut r1);
            assert!(
                a >= d.min() && a <= d.max(),
                "roll {a} out of [{}, {}] for {d}",
                d.min(),
                d.max()
            );
            assert_eq!(a, d.roll(&mut r2), "roll not deterministic for seed");
        }
    }
}

/// **average and span match their closed forms** — `average_x100` equals
/// `count·(sides+1)·50 + modifier·100` and lies between `100·min` and `100·max`;
/// `span` equals `max − min == count·(sides−1)` and is independent of the
/// modifier (which cancels).
#[test]
fn prop_dice_average_and_span_formulas() {
    let mut rng = SplitMix64::new(0x0D1C_EA06);
    for _ in 0..ITERS {
        let d = rand_dice(&mut rng);
        let count = d.count as i64;
        let sides = d.sides as i64;
        let modi = d.modifier as i64;

        let expected_avg = count * (sides + 1) * 50 + modi * 100;
        assert_eq!(d.average_x100(), expected_avg, "average_x100 formula wrong");
        assert!(
            d.average_x100() >= 100 * d.min() as i64 && d.average_x100() <= 100 * d.max() as i64,
            "average outside [min, max] for {d}"
        );

        // span == max - min == count*(sides-1); modifier cancels.
        assert_eq!(d.span() as i64, d.max() as i64 - d.min() as i64, "span != max-min");
        assert_eq!(d.span() as i64, count * (sides - 1), "span != count*(sides-1)");
        let shifted = Dice::new(d.count, d.sides, d.modifier.wrapping_add(7));
        assert_eq!(d.span(), shifted.span(), "span depends on modifier");
    }
}

/// **advantage dominates disadvantage over identical draws** — seeding two RNGs
/// the same makes `roll_advantage` and `roll_disadvantage` see the same pair of
/// rolls, so advantage (the max) is always `>=` disadvantage (the min), and
/// both stay within `[min, max]`.
#[test]
fn prop_dice_advantage_dominates_disadvantage() {
    let mut rng = SplitMix64::new(0x0D1C_EADD);
    for _ in 0..ITERS {
        let d = rand_dice(&mut rng);
        let seed = rand_seed(&mut rng);
        let mut ra = SplitMix64::new(seed);
        let mut rb = SplitMix64::new(seed);
        let adv = d.roll_advantage(&mut ra);
        let dis = d.roll_disadvantage(&mut rb);
        assert!(adv >= dis, "advantage {adv} < disadvantage {dis} for {d}");
        assert!(adv >= d.min() && adv <= d.max(), "advantage out of bounds");
        assert!(dis >= d.min() && dis <= d.max(), "disadvantage out of bounds");
    }
}

/// **keep-highest sums stay in `[keep·min, keep·max]` and the degenerate case is
/// drawless** — `roll_n_keep_highest` clamps `keep` to `n`, sums that many rolls
/// each in `[min, max]`, and when `n == 0` or `keep == 0` returns the modifier
/// alone without consuming any RNG draw.
#[test]
fn prop_dice_keep_highest_bounds_and_draw_contract() {
    let mut rng = SplitMix64::new(0x0D1C_EE47);
    for _ in 0..ITERS {
        let d = rand_dice(&mut rng);
        let n = rng.below(8);
        let keep = rng.below(10);
        let seed = rand_seed(&mut rng);
        let mut rr = SplitMix64::new(seed);
        let v = d.roll_n_keep_highest(n, keep, &mut rr);

        if n == 0 || keep == 0 {
            assert_eq!(v, d.modifier, "degenerate keep-highest != modifier");
            assert_eq!(rr.state(), SplitMix64::new(seed).state(), "degenerate case drew");
        } else {
            let k = keep.min(n) as i64;
            let lo = k * d.min() as i64;
            let hi = k * d.max() as i64;
            assert!(
                (v as i64) >= lo && (v as i64) <= hi,
                "keep-highest {v} outside [{lo}, {hi}] for {d} (n={n}, keep={keep})"
            );
        }
    }
}

/// **to_string round-trips through parse** — formatting a Dice and parsing the
/// result recovers the original expression exactly, for any `count`, real
/// `sides >= 1`, and signed modifier (the authoring-format invariant).
#[test]
fn prop_dice_display_parse_round_trip() {
    let mut rng = SplitMix64::new(0x0D1C_ED15);
    for _ in 0..ITERS {
        let d = Dice::new(rng.below(1000), 1 + rng.below(1000), rng.range(-10_000, 10_000));
        assert_eq!(Dice::parse(&d.to_string()), Some(d), "round-trip failed for {d}");
    }
}

/// **a flat die is constant at min == max** — when `sides <= 1` the expression
/// is deterministic regardless of the RNG: `min == max`, `span == 0`, `is_flat`
/// is true, and every roll equals `min`. A real (`sides >= 2`) die is not flat.
#[test]
fn prop_dice_flat_die_is_constant() {
    let mut rng = SplitMix64::new(0x0D1C_EF1A);
    for _ in 0..ITERS {
        let count = rng.below(10);
        let sides = rng.below(2); // 0 or 1 → flat
        let modi = rng.range(-50, 50);
        let d = Dice::new(count, sides, modi);
        assert!(d.is_flat(), "sides<=1 must be flat: {d}");
        assert_eq!(d.min(), d.max(), "flat die has a spread: {d}");
        assert_eq!(d.span(), 0, "flat die span != 0");
        let mut rr = SplitMix64::new(rand_seed(&mut rng));
        for _ in 0..5 {
            assert_eq!(d.roll(&mut rr), d.min(), "flat die roll varied");
        }

        let real = Dice::new(1 + rng.below(5), 2 + rng.below(20), 0);
        assert!(!real.is_flat(), "sides>=2 must not be flat: {real}");
    }
}

// ---------------------------------------------------------------------------
// camera::Camera — integer world↔screen coordinate-mapping laws
//
// The camera maps a world-space rectangle to a screen viewport with pure
// integer arithmetic, so it obeys exact laws:
//   - screen_to_world then world_to_screen is the identity on visible cells;
//   - the viewport stays clamped within the world after any movement;
//   - visibility is one predicate: world_to_screen.is_some() == is_visible ==
//     inside world_rect == distance_to_edge >= 0;
//   - clamp_world_to_screen always lands on screen and matches world_to_screen
//     for visible points;
//   - world_rect / center / viewport_area / contains_rect are mutually
//     consistent;
//   - screen_distance is a Chebyshev metric and chebyshev_to_center matches it.
// ---------------------------------------------------------------------------

/// A random camera: small world and viewport, focus possibly off-world so the
/// clamping paths are exercised.
fn rand_camera(rng: &mut SplitMix64) -> Camera {
    Camera::new(
        rng.range(-20, 80),
        rng.range(-20, 80),
        1 + rng.below(30),
        1 + rng.below(30),
        1 + rng.below(60),
        1 + rng.below(60),
    )
}

/// **screen_to_world then world_to_screen is the identity on the viewport** —
/// every screen cell maps to a world point that maps straight back to the same
/// cell, and out-of-viewport screen coordinates clamp to the edge cell.
#[test]
fn prop_camera_screen_world_round_trip() {
    let mut rng = SplitMix64::new(0x0CA3_7717);
    for _ in 0..ITERS {
        let c = rand_camera(&mut rng);
        let sx = rng.below(c.screen_w);
        let sy = rng.below(c.screen_h);
        let (wx, wy) = c.screen_to_world(sx, sy);
        assert_eq!(
            c.world_to_screen(wx, wy),
            Some((sx, sy)),
            "screen→world→screen not identity"
        );
        // OOB screen coords clamp to the last valid cell.
        let (cwx, cwy) = c.screen_to_world(c.screen_w + 5, c.screen_h + 5);
        let (ewx, ewy) = c.screen_to_world(c.screen_w - 1, c.screen_h - 1);
        assert_eq!((cwx, cwy), (ewx, ewy), "OOB screen coords did not clamp to edge");
    }
}

/// **the viewport stays clamped within the world** — after construction and any
/// sequence of recenter / pan / set_screen_size against a fixed world, the
/// top-left origin stays in `[0, max(0, world − screen)]` on both axes, so the
/// viewport never scrolls off the world.
#[test]
fn prop_camera_viewport_stays_within_world() {
    let mut rng = SplitMix64::new(0x0CA3_C1A3);
    for _ in 0..ITERS {
        let world_w = 1 + rng.below(60);
        let world_h = 1 + rng.below(60);
        let mut c = Camera::new(
            rng.range(-20, 80),
            rng.range(-20, 80),
            1 + rng.below(30),
            1 + rng.below(30),
            world_w,
            world_h,
        );
        for _ in 0..6 {
            match rng.below(3) {
                0 => c.recenter(rng.range(-40, 100), rng.range(-40, 100), world_w, world_h),
                1 => c.pan(rng.range(-50, 50), rng.range(-50, 50), world_w, world_h),
                _ => c.set_screen_size(1 + rng.below(30), 1 + rng.below(30), world_w, world_h),
            }
            let max_x = (world_w as i64 - c.screen_w as i64).max(0);
            let max_y = (world_h as i64 - c.screen_h as i64).max(0);
            assert!(
                (c.top_left_x as i64) >= 0 && (c.top_left_x as i64) <= max_x,
                "top_left_x {} outside [0, {max_x}]",
                c.top_left_x
            );
            assert!(
                (c.top_left_y as i64) >= 0 && (c.top_left_y as i64) <= max_y,
                "top_left_y {} outside [0, {max_y}]",
                c.top_left_y
            );
        }
    }
}

/// **visibility is a single coherent predicate** — for any world point,
/// `world_to_screen(..).is_some()`, `is_visible`, membership in the half-open
/// `world_rect`, and `distance_to_edge >= 0` all agree.
#[test]
fn prop_camera_visibility_equivalences() {
    let mut rng = SplitMix64::new(0x0CA3_715B);
    for _ in 0..ITERS {
        let c = rand_camera(&mut rng);
        let (l, t, r, b) = c.world_rect();
        let wx = rng.range(-30, 90);
        let wy = rng.range(-30, 90);
        let inside = wx >= l && wx < r && wy >= t && wy < b;
        assert_eq!(c.world_to_screen(wx, wy).is_some(), inside, "world_to_screen vs rect");
        assert_eq!(c.is_visible(wx, wy), inside, "is_visible vs rect");
        assert_eq!(c.distance_to_edge(wx, wy) >= 0, inside, "distance_to_edge sign vs rect");
    }
}

/// **clamp_world_to_screen always lands on screen and agrees when visible** —
/// the result is always within `[0, screen_w) × [0, screen_h)`, and for a
/// visible point it equals the `world_to_screen` mapping.
#[test]
fn prop_camera_clamp_world_to_screen_in_bounds() {
    let mut rng = SplitMix64::new(0x0CA3_C1A9);
    for _ in 0..ITERS {
        let c = rand_camera(&mut rng);
        let wx = rng.range(-30, 90);
        let wy = rng.range(-30, 90);
        let (sx, sy) = c.clamp_world_to_screen(wx, wy);
        assert!(sx < c.screen_w && sy < c.screen_h, "clamp result off-screen");
        if let Some(visible) = c.world_to_screen(wx, wy) {
            assert_eq!((sx, sy), visible, "clamp disagreed with world_to_screen for visible point");
        }
    }
}

/// **world_rect, center, area and contains_rect are mutually consistent** — the
/// rect's right/bottom equal origin plus size, `viewport_area == w·h`, the
/// centre is the integer midpoint, and `contains_rect` equals the standard AABB
/// overlap of a rectangle with the viewport (empty rects never overlap).
#[test]
fn prop_camera_world_rect_and_overlap_consistency() {
    let mut rng = SplitMix64::new(0x0CA3_3EC7);
    for _ in 0..ITERS {
        let c = rand_camera(&mut rng);
        let (l, t, r, b) = c.world_rect();
        assert_eq!(r, c.top_left_x + c.screen_w as i32, "world_rect right wrong");
        assert_eq!(b, c.top_left_y + c.screen_h as i32, "world_rect bottom wrong");
        assert_eq!(c.viewport_area(), c.screen_w * c.screen_h, "area != w*h");
        assert_eq!(c.center(), (l + c.screen_w as i32 / 2, t + c.screen_h as i32 / 2), "center wrong");

        let rl = rng.range(-10, 70);
        let rt = rng.range(-10, 70);
        let rr = rl + 1 + rng.below(20) as i32;
        let rb = rt + 1 + rng.below(20) as i32;
        let overlap = rl < r && rr > l && rt < b && rb > t;
        assert_eq!(c.contains_rect(rl, rt, rr, rb), overlap, "contains_rect != AABB overlap");
        assert!(!c.contains_rect(5, 5, 5, 9), "empty rect overlapped");
    }
}

/// **screen_distance is a Chebyshev metric and chebyshev_to_center matches it**
/// — `screen_distance` is symmetric, zero iff the cells coincide, and satisfies
/// the triangle inequality; `chebyshev_to_center` equals `max(|Δx|, |Δy|)` from
/// the viewport centre.
#[test]
fn prop_camera_chebyshev_metric_axioms() {
    let mut rng = SplitMix64::new(0x0CA3_DDE7);
    for _ in 0..ITERS {
        let a = (rng.below(50), rng.below(50));
        let b = (rng.below(50), rng.below(50));
        let p = (rng.below(50), rng.below(50));
        let dab = Camera::screen_distance(a.0, a.1, b.0, b.1);
        let dba = Camera::screen_distance(b.0, b.1, a.0, a.1);
        let dbp = Camera::screen_distance(b.0, b.1, p.0, p.1);
        let dap = Camera::screen_distance(a.0, a.1, p.0, p.1);
        assert_eq!(dab, dba, "screen_distance not symmetric");
        assert_eq!(dab == 0, a == b, "screen_distance zero must mean equal cells");
        assert!(dap <= dab + dbp, "triangle inequality violated");

        let c = rand_camera(&mut rng);
        let (ccx, ccy) = c.center();
        let wx = rng.range(-30, 90);
        let wy = rng.range(-30, 90);
        let expected = (ccx - wx).unsigned_abs().max((ccy - wy).unsigned_abs());
        assert_eq!(c.chebyshev_to_center(wx, wy), expected, "chebyshev_to_center wrong");
    }
}

// ---------------------------------------------------------------------------
// visibility::VisibilityMap — fog-of-war / exploration-memory laws
//
// The tri-state map (Unseen < Remembered < Visible) is the memory layer above
// FOV. Under the normal turn loop (begin_frame then mark_visible) it obeys:
//   - exploration is monotone: an explored cell never reverts to Unseen;
//   - the three states partition the grid and explored == visible + remembered;
//   - begin_frame demotes exactly the visible cells to remembered and leaves
//     the explored set unchanged.
// ---------------------------------------------------------------------------

/// **exploration is monotone under the turn loop** — running random
/// `begin_frame` / `mark_visible` turns can only ever grow the explored set; a
/// cell that has been observed never falls back to `Unseen`, and visibility is
/// always `>=` `Remembered` once explored.
#[test]
fn prop_visibility_exploration_is_monotone() {
    let mut rng = SplitMix64::new(0x715B_303E);
    for _ in 0..ITERS {
        let w = 1 + rng.below(8);
        let h = 1 + rng.below(8);
        let mut v = VisibilityMap::new(w, h);
        let mut explored = vec![false; (w * h) as usize];

        for _ in 0..6 {
            v.begin_frame();
            // mark a random handful of cells visible this "turn".
            let marks = rng.below(w * h + 1);
            for _ in 0..marks {
                let x = rng.below(w) as i32;
                let y = rng.below(h) as i32;
                v.mark_visible(x, y);
                explored[(y as u32 * w + x as u32) as usize] = true;
            }
            // every cell ever marked must still be explored (never Unseen).
            for (i, &was) in explored.iter().enumerate() {
                let x = (i as u32 % w) as i32;
                let y = (i as u32 / w) as i32;
                if was {
                    assert!(v.is_explored(x, y), "explored cell ({x},{y}) reverted");
                    assert!(v.get(x, y) >= Visibility::Remembered, "rank dropped below Remembered");
                }
            }
        }
    }
}

/// **the three states partition the grid** — `unseen + remembered + visible ==
/// len`, `explored == visible + remembered`, `visible <= explored <= len`, and
/// `explored_percent == explored*100/len`.
#[test]
fn prop_visibility_counts_partition() {
    let mut rng = SplitMix64::new(0x715B_C04E);
    for _ in 0..ITERS {
        let w = 1 + rng.below(8);
        let h = 1 + rng.below(8);
        let mut v = VisibilityMap::new(w, h);
        // Drive it into a mixed state.
        for _ in 0..rng.below(20) {
            match rng.below(3) {
                0 => v.begin_frame(),
                _ => v.mark_visible(rng.below(w) as i32, rng.below(h) as i32),
            }
        }
        let len = v.len();
        let unseen = v.count(Visibility::Unseen);
        let remembered = v.count(Visibility::Remembered);
        let visible = v.visible_count();
        assert_eq!(unseen + remembered + visible, len, "states do not partition");
        assert_eq!(v.explored_count(), visible + remembered, "explored != vis+rem");
        assert!(visible <= v.explored_count() && v.explored_count() <= len, "ordering broken");
        assert_eq!(
            v.explored_percent(),
            (v.explored_count() as u64 * 100 / len as u64) as u32,
            "explored_percent formula wrong"
        );
    }
}

/// **begin_frame demotes exactly the visible cells** — after `begin_frame`,
/// every cell that was `Visible` is now `Remembered`, every other cell is
/// unchanged, the visible count is zero, and the explored set is preserved.
#[test]
fn prop_visibility_begin_frame_demotes_exactly() {
    let mut rng = SplitMix64::new(0x715B_DE10);
    for _ in 0..ITERS {
        let w = 1 + rng.below(8);
        let h = 1 + rng.below(8);
        let mut v = VisibilityMap::new(w, h);
        for _ in 0..rng.below(15) {
            match rng.below(3) {
                0 => v.begin_frame(),
                _ => v.mark_visible(rng.below(w) as i32, rng.below(h) as i32),
            }
        }
        let before: Vec<(i32, i32, Visibility)> = v.iter().collect();
        let explored_before = v.explored_count();
        v.begin_frame();
        for (x, y, was) in before {
            let now = v.get(x, y);
            match was {
                Visibility::Visible => assert_eq!(now, Visibility::Remembered, "visible not demoted"),
                other => assert_eq!(now, other, "non-visible cell changed"),
            }
        }
        assert_eq!(v.visible_count(), 0, "visible remained after begin_frame");
        assert_eq!(v.explored_count(), explored_before, "explored set changed");
    }
}

// ── ShuffleBag ────────────────────────────────────────────────────────────────

/// **One full cycle is a permutation of the template.**
/// Draw exactly `cycle_len()` items; sorted they must equal the sorted template.
#[test]
fn prop_shufflebag_cycle_is_permutation() {
    let mut rng = SplitMix64::new(0xBA65_0001);
    for _ in 0..ITERS {
        let n = 1 + rng.below(8) as usize;
        let contents: Vec<u32> = (0..n).map(|_| rng.below(16)).collect();
        let mut bag = ShuffleBag::new(contents.clone());
        let drawn: Vec<u32> = (0..n).map(|_| bag.draw(&mut rng).unwrap()).collect();
        let mut expected = contents.clone();
        let mut got = drawn.clone();
        expected.sort_unstable();
        got.sort_unstable();
        assert_eq!(got, expected, "one cycle must be a permutation of the template");
        assert!(bag.cycle_exhausted(), "bag must be empty after exactly one cycle");
    }
}

/// **Two consecutive cycles are both permutations, and counts are balanced.**
/// Over two full cycles each item appears exactly twice its multiplicity.
#[test]
fn prop_shufflebag_two_cycles_balanced() {
    let mut rng = SplitMix64::new(0xBA65_0002);
    for _ in 0..ITERS {
        let n = 1 + rng.below(6) as usize;
        let contents: Vec<u32> = (0..n).map(|_| rng.below(8)).collect();
        let mut bag = ShuffleBag::new(contents.clone());
        let drawn: Vec<u32> = (0..2 * n).map(|_| bag.draw(&mut rng).unwrap()).collect();
        // Every value in `contents` must appear exactly twice its count.
        let mut expected = contents.clone();
        expected.extend_from_slice(&contents);
        expected.sort_unstable();
        let mut got = drawn.clone();
        got.sort_unstable();
        assert_eq!(got, expected, "two cycles must double each template count");
    }
}

/// **Determinism**: identical seed ⇒ identical draw sequence.
#[test]
fn prop_shufflebag_draw_sequence_deterministic() {
    let mut seed_rng = SplitMix64::new(0xBA65_0003);
    for _ in 0..ITERS {
        let n = 1 + seed_rng.below(6) as usize;
        let contents: Vec<u32> = (0..n).map(|_| seed_rng.below(16)).collect();
        let seed = seed_rng.next_u64();
        let draws = 2 * n + seed_rng.below(4) as usize;
        let seq = |s: u64| {
            let mut bag = ShuffleBag::new(contents.clone());
            let mut r = SplitMix64::new(s);
            (0..draws).map(|_| bag.draw(&mut r).unwrap()).collect::<Vec<_>>()
        };
        assert_eq!(seq(seed), seq(seed), "same seed must produce same sequence");
    }
}

/// **Empty bag always returns None and never panics.**
#[test]
fn prop_shufflebag_empty_returns_none() {
    let mut rng = SplitMix64::new(0xBA65_0004);
    for _ in 0..ITERS {
        let mut bag: ShuffleBag<u32> = ShuffleBag::new(vec![]);
        assert!(bag.is_empty());
        assert_eq!(bag.draw(&mut rng), None);
        assert_eq!(bag.remaining(), 0);
    }
}

/// **Size-1 bag never consumes an RNG draw** (degenerate-bound contract).
#[test]
fn prop_shufflebag_size1_no_rng_draw() {
    let mut rng = SplitMix64::new(0xBA65_0005);
    for _ in 0..ITERS {
        let item: u32 = rng.below(256);
        let mut bag = ShuffleBag::new(vec![item]);
        let state_before = rng.state();
        for _ in 0..8 {
            assert_eq!(bag.draw(&mut rng), Some(item));
        }
        assert_eq!(rng.state(), state_before, "size-1 bag must consume no RNG state");
    }
}

/// **`add` grows both template and live bag by one.**
#[test]
fn prop_shufflebag_add_grows_both() {
    let mut rng = SplitMix64::new(0xBA65_0006);
    for _ in 0..ITERS {
        let n = rng.below(5) as usize;
        let contents: Vec<u32> = (0..n).map(|_| rng.below(16)).collect();
        let mut bag = ShuffleBag::new(contents.clone());
        let extra: u32 = rng.below(32);
        bag.add(extra);
        assert_eq!(bag.cycle_len(), n + 1, "cycle_len must grow by 1 after add");
        assert_eq!(bag.remaining(), n + 1, "remaining must grow by 1 after add");
    }
}

// ── Equipment ─────────────────────────────────────────────────────────────────

/// Build a random loadout of `(slot, modifier)` pairs and return the equipment
/// plus the multiset of equipped modifiers (in canonical slot order).
fn rand_equipment(rng: &mut SplitMix64) -> Equipment<StatsModifier> {
    let mut gear = Equipment::new();
    for &slot in EquipSlot::ALL.iter() {
        if rng.below(2) == 1 {
            let attack = rng.range(-5, 6);
            let defense = rng.range(-5, 6);
            let max_hp = rng.range(-10, 11);
            gear.equip(slot, StatsModifier { attack, defense, max_hp });
        }
    }
    gear
}

/// **occupied + empty == slot_count**, always; and `is_empty` ⇔ occupied == 0.
#[test]
fn prop_equipment_occupancy_partition() {
    let mut rng = SplitMix64::new(0xE901_0001);
    for _ in 0..ITERS {
        let gear = rand_equipment(&mut rng);
        assert_eq!(gear.occupied_count() + gear.empty_count(), gear.slot_count());
        assert_eq!(gear.is_empty(), gear.occupied_count() == 0);
        // iter yields exactly occupied_count items, all in canonical order.
        let slots: Vec<EquipSlot> = gear.iter().map(|(s, _)| s).collect();
        assert_eq!(slots.len(), gear.occupied_count());
        let mut sorted = slots.clone();
        sorted.sort();
        assert_eq!(slots, sorted, "iter must visit slots in canonical order");
    }
}

/// **aggregate equals the manual field-wise saturating fold over worn items**,
/// visited in canonical order.
#[test]
fn prop_equipment_aggregate_matches_manual_fold() {
    let mut rng = SplitMix64::new(0xE901_0002);
    for _ in 0..ITERS {
        let gear = rand_equipment(&mut rng);
        let mut expect = StatsModifier::default();
        for &slot in EquipSlot::ALL.iter() {
            if let Some(m) = gear.get(slot) {
                expect.attack = expect.attack.saturating_add(m.attack);
                expect.defense = expect.defense.saturating_add(m.defense);
                expect.max_hp = expect.max_hp.saturating_add(m.max_hp);
            }
        }
        assert_eq!(gear.aggregate(|&item| item), expect);
    }
}

/// **equip/unequip round-trip**: equipping an item then unequipping that slot
/// returns the same item and restores the prior occupancy exactly.
#[test]
fn prop_equipment_equip_unequip_round_trip() {
    let mut rng = SplitMix64::new(0xE901_0003);
    for _ in 0..ITERS {
        let mut gear = rand_equipment(&mut rng);
        let slot = EquipSlot::ALL[rng.below(9) as usize];
        let before = gear.get(slot).copied();
        let item = StatsModifier { attack: rng.range(-3, 4), defense: 0, max_hp: 0 };
        let displaced = gear.equip(slot, item);
        assert_eq!(displaced, before, "equip must return the prior occupant");
        assert_eq!(gear.get(slot), Some(&item), "slot now holds the new item");
        // Unequip and restore whatever was there before.
        let taken = gear.unequip(slot);
        assert_eq!(taken, Some(item), "unequip returns what we equipped");
        assert!(!gear.is_equipped(slot), "slot empty after unequip");
        if let Some(b) = before {
            gear.equip(slot, b);
            assert_eq!(gear.get(slot), Some(&b), "restored to original state");
        }
    }
}

/// **equip conserves total count to within one** (swap = net 0, fill = +1) and
/// never affects any other slot.
#[test]
fn prop_equipment_equip_locality_and_count() {
    let mut rng = SplitMix64::new(0xE901_0004);
    for _ in 0..ITERS {
        let mut gear = rand_equipment(&mut rng);
        let slot = EquipSlot::ALL[rng.below(9) as usize];
        let was_occupied = gear.is_equipped(slot);
        let count_before = gear.occupied_count();
        let others: Vec<(EquipSlot, Option<StatsModifier>)> = EquipSlot::ALL
            .iter()
            .filter(|&&s| s != slot)
            .map(|&s| (s, gear.get(s).copied()))
            .collect();
        gear.equip(slot, StatsModifier::default());
        let count_after = gear.occupied_count();
        if was_occupied {
            assert_eq!(count_after, count_before, "swap keeps count");
        } else {
            assert_eq!(count_after, count_before + 1, "fill grows count by one");
        }
        for (s, prev) in others {
            assert_eq!(gear.get(s).copied(), prev, "equip must not touch other slots");
        }
    }
}

/// **Determinism / hash sensitivity**: same construction ⇒ same hash; clearing
/// yields the empty-loadout hash; aggregate is reproducible.
#[test]
fn prop_equipment_deterministic_and_hashable() {
    use izanagi_kit::world_hash::hash_state;
    let empty_hash = hash_state(&Equipment::<StatsModifier>::new());
    for seed in 0u64..ITERS as u64 {
        let mut a = SplitMix64::new(0xE901_0005 ^ seed);
        let mut b = SplitMix64::new(0xE901_0005 ^ seed);
        let ga = rand_equipment(&mut a);
        let mut gb = rand_equipment(&mut b);
        assert_eq!(hash_state(&ga), hash_state(&gb), "same seed ⇒ same hash");
        assert_eq!(ga.aggregate(|&m| m), gb.aggregate(|&m| m), "aggregate reproducible");
        gb.clear();
        assert_eq!(hash_state(&gb), empty_hash, "cleared loadout hashes as empty");
        assert_eq!(gb.aggregate(|&m| m), StatsModifier::default());
    }
}

// ── Progression ───────────────────────────────────────────────────────────────

/// A random non-degenerate level curve.
fn rand_curve(rng: &mut SplitMix64) -> LevelCurve {
    let base = 1 + rng.below(500) as u64;
    let step = rng.below(200) as u64;
    let max_level = 1 + rng.below(60);
    LevelCurve::new(base, step, max_level)
}

/// **xp_to_reach is strictly monotone over reachable levels, and level_at is its
/// inverse** — `level_at(xp_to_reach(L)) == L` for every `1 ≤ L ≤ max_level`.
#[test]
fn prop_progression_threshold_round_trip() {
    let mut rng = SplitMix64::new(0x9209_0001);
    for _ in 0..ITERS {
        let c = rand_curve(&mut rng);
        let mut prev = 0u64;
        for l in 1..=c.max_level() {
            let t = c.xp_to_reach(l);
            if l > 1 {
                assert!(t >= prev, "thresholds must be non-decreasing");
            }
            assert_eq!(c.level_at(t), l, "level_at must invert xp_to_reach at {l}");
            prev = t;
        }
    }
}

/// **Threshold boundary**: one XP below a level's threshold is the previous
/// level, exactly at the threshold is that level (when costs are positive).
#[test]
fn prop_progression_threshold_boundary() {
    let mut rng = SplitMix64::new(0x9209_0002);
    for _ in 0..ITERS {
        // Force positive per-level cost so thresholds are strictly increasing.
        let base = 1 + rng.below(500) as u64;
        let step = rng.below(200) as u64;
        let c = LevelCurve::new(base, step, 1 + rng.below(60));
        for l in 2..=c.max_level() {
            let t = c.xp_to_reach(l);
            assert_eq!(c.level_at(t), l, "at threshold");
            assert_eq!(c.level_at(t - 1), l - 1, "just below threshold {l}");
        }
    }
}

/// **level is monotone in total XP**: more experience never lowers the level.
#[test]
fn prop_progression_level_monotone_in_xp() {
    let mut rng = SplitMix64::new(0x9209_0003);
    for _ in 0..ITERS {
        let c = rand_curve(&mut rng);
        let x1 = rng.next_u64() % 1_000_000;
        let x2 = rng.next_u64() % 1_000_000;
        let (lo, hi) = (x1.min(x2), x1.max(x2));
        assert!(c.level_at(lo) <= c.level_at(hi), "level must be monotone in xp");
    }
}

/// **add_xp conserves experience and derives level purely from the total**:
/// total after == saturating(before + amount), level == level_at(total), and
/// the returned levels-gained == new level − old level.
#[test]
fn prop_progression_add_xp_conserves_and_derives() {
    let mut rng = SplitMix64::new(0x9209_0004);
    for _ in 0..ITERS {
        let c = rand_curve(&mut rng);
        let start = rng.next_u64() % 500_000;
        let mut p = Progression::with_xp(c, start);
        let before_level = p.level();
        let before_xp = p.total_xp();
        let amount = rng.next_u64() % 500_000;
        let gained = p.add_xp(amount);
        assert_eq!(p.total_xp(), before_xp.saturating_add(amount), "xp conserved");
        assert_eq!(p.level(), c.level_at(p.total_xp()), "level is pure fn of total");
        assert_eq!(gained, p.level() - before_level, "gained == delta level");
        assert!(p.level() <= c.max_level(), "level never exceeds cap");
    }
}

/// **xp_into_level + xp_to_next == cost_of_current_level**, except at the cap
/// where both `xp_to_next` is zero and `is_max_level` holds.
#[test]
fn prop_progression_within_level_accounting() {
    let mut rng = SplitMix64::new(0x9209_0005);
    for _ in 0..ITERS {
        let c = rand_curve(&mut rng);
        let total = rng.next_u64() % 1_000_000;
        let p = Progression::with_xp(c, total);
        // xp_into_level is always total minus the current threshold.
        assert_eq!(p.xp_into_level(), total - c.xp_to_reach(p.level()));
        if p.is_max_level() {
            assert_eq!(p.xp_to_next(), 0, "no next level at cap");
        } else {
            let cost = c.cost_of_level_up(p.level());
            assert_eq!(
                p.xp_into_level() + p.xp_to_next(),
                cost,
                "into + to_next must equal the level's cost"
            );
            assert!(p.xp_into_level() < cost, "progress stays within the level");
        }
    }
}
