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
use izanagi_kit::pathfinding::{astar, weighted_astar};
use izanagi_kit::turn::{Scheduler, ACTION_COST};
use izanagi_kit::wfc::wfc_solve_backtrack;
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
