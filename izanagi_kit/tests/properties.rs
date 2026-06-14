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
    rotate_90_cw, Fixed, SplitMix64,
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
    let mut rng = SplitMix64::new(0x_ADD_C0);
    for _ in 0..ITERS {
        let (a, b) = (rand_fixed_ext(&mut rng), rand_fixed_ext(&mut rng));
        assert_eq!(a + b, b + a, "add not commutative for {a:?},{b:?}");
    }
}

#[test]
fn prop_fixed_mul_is_commutative() {
    let mut rng = SplitMix64::new(0x_3_4C);
    for _ in 0..ITERS {
        let (a, b) = (rand_fixed_ext(&mut rng), rand_fixed_ext(&mut rng));
        assert_eq!(a.mul(b), b.mul(a), "mul not commutative for {a:?},{b:?}");
    }
}

#[test]
fn prop_fixed_identities() {
    let mut rng = SplitMix64::new(0x1_DE_77);
    for _ in 0..ITERS {
        let a = rand_fixed_ext(&mut rng);
        assert_eq!(a + Fixed::ZERO, a, "additive identity");
        assert_eq!(a.mul(Fixed::ONE), a, "multiplicative identity");
        assert_eq!(a.mul(Fixed::ZERO), Fixed::ZERO, "annihilation by zero");
    }
}

#[test]
fn prop_fixed_abs_is_non_negative() {
    let mut rng = SplitMix64::new(0x_AB5);
    for _ in 0..ITERS {
        let a = rand_fixed_ext(&mut rng);
        assert!(a.abs() >= Fixed::ZERO, "abs negative for {a:?}");
    }
}

#[test]
fn prop_fixed_double_negation_is_identity_except_min() {
    // -(-a) == a for every value except MIN (whose negation saturates to MAX).
    let mut rng = SplitMix64::new(0x_4E_61);
    for _ in 0..ITERS {
        let a = rand_fixed(&mut rng); // moderate range never hits MIN
        assert_eq!(-(-a), a, "double negation not identity for {a:?}");
    }
    // The documented MIN exception.
    assert_eq!(-Fixed::MIN, Fixed::MAX);
}

#[test]
fn prop_fixed_clamp_is_in_range_and_idempotent() {
    let mut rng = SplitMix64::new(0x_C1_A3);
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
    let mut rng = SplitMix64::new(0x_31_AC);
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
    let mut rng = SplitMix64::new(0x_1E_47);
    for _ in 0..ITERS {
        let (a, b) = (rand_fixed(&mut rng), rand_fixed(&mut rng));
        assert_eq!(Fixed::lerp(a, b, Fixed::ZERO), a, "lerp t=0 != a");
        assert_eq!(Fixed::lerp(a, b, Fixed::ONE), b, "lerp t=1 != b");
    }
}

#[test]
fn prop_fixed_sqrt_is_monotonic_and_non_negative() {
    let mut rng = SplitMix64::new(0x_5417);
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
    let mut rng = SplitMix64::new(0x_4_0_7);
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
    let mut rng = SplitMix64::new(0x_C_CC);
    for _ in 0..ITERS {
        let (x, y) = rand_coord(&mut rng);
        let (cx, cy) = rotate_90_cw(x, y);
        assert_eq!(rotate_90_ccw(cx, cy), (x, y), "ccw∘cw != id for ({x},{y})");
    }
}

#[test]
fn prop_reflect_point_is_an_involution() {
    let mut rng = SplitMix64::new(0x_4EF_1);
    for _ in 0..ITERS {
        let p = rand_coord(&mut rng);
        let c = rand_coord(&mut rng);
        assert_eq!(reflect_point(reflect_point(p, c), c), p, "reflect not involutive");
    }
}

#[test]
fn prop_distances_are_symmetric_and_ordered() {
    let mut rng = SplitMix64::new(0x_D157);
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
    let mut rng = SplitMix64::new(0x_11_E5);
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
    let mut rng = SplitMix64::new(0x_C8_AC);
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
    let mut rng = SplitMix64::new(0x_C0_4E);
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
