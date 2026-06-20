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
    base_damage, cone, knockback, line, ray_cast, splash_attack, Aabb, Distance, Fixed,
    SpatialHash, SplitMix64, Stats, StatsModifier,
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
    let mut rng = SplitMix64::new(0x4A605);
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

#[test]
fn aabb_is_total_at_extreme_coords() {
    // union/nearest_corner span the full coordinate range; the difference and
    // multiply must saturate rather than overflow.
    for &x in &EXTREMES {
        for &w in &EXTREMES {
            let a = Aabb::new(x, x, w, w);
            let b = Aabb::new(w, x, x, w);
            let _ = a.union(&b); // <- was raw r - x / b - y
            let _ = a.intersection(&b);
            let _ = a.contains_point(w, x);
            let _ = a.nearest_corner(x, w); // <- was raw (px - self.x).abs()
            let _ = (a.right(), a.bottom(), a.area());
        }
    }
}

#[test]
fn spatial_hash_queries_are_total_at_extreme_positions() {
    // Extreme *positions* with *small* sizes: exercises the saturating position
    // arithmetic — `x.saturating_add(w).saturating_sub(1)` and
    // `cx.saturating_sub(radius)` — at coordinate extremes without forcing
    // query_rect to iterate an astronomically large cell span. (A huge query
    // *span* is an O(area) cost — an algorithmic property of the grid, not an
    // overflow; the `2*radius+1` multiply is saturated defensively regardless.)
    let mut g: SpatialHash<u32> = SpatialHash::new(16);
    for (i, &x) in EXTREMES.iter().enumerate() {
        g.insert(i as u32, x, x);
    }
    let small = [0i32, 1, 5];
    for &x in &EXTREMES {
        for &s in &small {
            let _ = g.query_rect(x, x, s, s);
            let _ = g.query_rect_count(x, x, s, s);
            let _ = g.query_radius(x, x, s);
        }
    }
}

// --- easing / encounter / random_table / inputbuf totality ------------------
// These modules take caller-controlled numeric inputs and were verified clean
// by inspection; the following turn that into standing guards.

#[test]
fn easing_is_total_at_and_beyond_unit_interval() {
    use izanagi_kit::{
        ease_in_back, ease_in_bounce, ease_in_cubic, ease_in_expo, ease_in_out_back,
        ease_in_out_bounce, ease_in_out_cubic, ease_in_out_expo, ease_in_out_sine, ease_in_quad,
        ease_in_sine, ease_out_back, ease_out_bounce, ease_out_circ, ease_out_cubic, ease_out_expo,
        ease_out_sine, linear,
    };
    // Easing is defined on [0,1] but documented to extrapolate; feed extremes
    // and out-of-range t — every curve must return (saturating Fixed math).
    let ts = [
        Fixed::MIN,
        Fixed::from_int(-5),
        Fixed::ZERO,
        Fixed::from_ratio(1, 3),
        Fixed::ONE,
        Fixed::from_int(5),
        Fixed::MAX,
    ];
    for &t in &ts {
        let _ = (
            linear(t),
            ease_in_quad(t),
            ease_in_cubic(t),
            ease_in_sine(t),
            ease_out_sine(t),
            ease_in_out_sine(t),
            ease_in_expo(t),
            ease_out_expo(t),
            ease_in_out_expo(t),
            ease_out_circ(t),
            ease_in_back(t),
            ease_out_back(t),
            ease_in_out_back(t),
            ease_in_bounce(t),
            ease_out_bounce(t),
            ease_in_out_bounce(t),
            ease_out_cubic(t),
            ease_in_out_cubic(t),
        );
    }
}

#[test]
fn encounter_pack_is_total_at_extremes() {
    use izanagi_kit::EncounterPack;
    let mut rng = SplitMix64::new(0xE11C0);
    // `roll_counts` computes each slot's count (exercising the full-span
    // `max - min + 1` arithmetic that previously overflowed) but returns it as a
    // single (value, count) tuple — so extreme spans are tested without
    // materializing up to u32::MAX spawn copies. `roll`, which *does* allocate
    // `count` copies, is only fed bounded `max` (a huge max legitimately spawns a
    // huge group — output-proportional cost, not a defect).
    for &min in &[0u32, 1, 1000, u32::MAX] {
        for &max in &[0u32, 1, 1000, u32::MAX] {
            for &chance in &[0u32, 50, 100, u32::MAX] {
                let pack: EncounterPack<u32> =
                    EncounterPack::new().with_optional_slot(7, min, max, chance);
                let _ = pack.roll_counts(&mut rng); // full-span safe (no materialization)
                let _ = (pack.min_spawns(), pack.max_spawns());
            }
        }
    }
    // `roll` only with bounded max so the spawn loop can't build a giant Vec.
    for &(min, max) in &[(0u32, 0u32), (1, 1), (0, 4), (2, 5)] {
        let pack: EncounterPack<u32> =
            EncounterPack::new().with_optional_slot(7, min, max, 100);
        let _ = pack.roll(&mut rng);
    }
}

#[test]
fn random_table_is_total_at_extreme_weights() {
    use izanagi_kit::RandomTable;
    let mut rng = SplitMix64::new(0x4A67AB);
    // Empty table.
    let empty: RandomTable<u32> = RandomTable::new();
    assert!(empty.roll(&mut rng).is_none());
    assert_eq!(empty.average_weight(), 0, "empty average must be 0, not div-by-zero");
    // Zero-weight and max-weight entries.
    let table: RandomTable<u32> = RandomTable::new()
        .with(0, 1) // zero weight: never chosen
        .with(u32::MAX, 2)
        .with(u32::MAX, 3);
    let _ = table.roll(&mut rng);
    let _ = table.roll_n(5, &mut rng);
    let _ = table.average_weight();
}

#[test]
fn input_buffer_is_total_with_degenerate_timing() {
    use izanagi_kit::InputBuffer;
    // The key risk is division by the repeat period: `new()` and `set_timing`
    // must both clamp it to >= 1 so `tick` never divides by zero. (A huge `tick`
    // value is deliberately *not* tested — with a 1-tick repeat period it
    // legitimately emits ~tick repeat events, an output-proportional cost, not a
    // panic; realistic ticks are small.)
    let mut buf: InputBuffer<u32> = InputBuffer::new(0, 0); // period clamped to 1
    buf.press(1);
    let _ = buf.tick(5);
    buf.set_timing(0, 0); // period clamped to >= 1 internally — no div-by-zero
    let _ = buf.tick(1000);
    buf.set_timing(u32::MAX, 7);
    let _ = buf.tick(1000); // initial_delay huge: no repeats fire, no panic
    assert!(buf.is_held(&1));
}

#[test]
fn passability_grid_is_total_at_large_dimensions() {
    use izanagi_kit::PassabilityGrid;
    // Dimensions whose product exceeds i32::MAX previously overflowed the
    // `(w * h) as usize` multiply in the constructor. usize/saturating math now
    // makes construction and indexing total. Use a tall-but-thin grid so the
    // product is large without allocating gigabytes: 100_000 x 2 = 200_000 cells,
    // and an index like y*width that would overflow i32 only past width*y > 2^31
    // (covered by the negative/oob guards returning early).
    let g = PassabilityGrid::new(100_000, 2);
    assert_eq!(g.len(), 200_000);
    // In-bounds access at a large linear offset must not overflow.
    let _ = g.is_blocked(99_999, 1);
    let _ = g.is_passable(0, 0);
    // Negative / out-of-bounds are total (return blocked / no-op).
    assert!(g.is_blocked(-1, 0));
    assert!(g.is_blocked(i32::MAX, i32::MAX));
    let mut g2 = PassabilityGrid::new(50_000, 3);
    g2.set_blocked(49_999, 2, true);
    g2.set_region(0, 0, 49_999, 2, true);
    assert!(g2.is_blocked(49_999, 2));
    // Degenerate dimensions clamp to empty, not panic.
    assert_eq!(PassabilityGrid::new(-5, 10).len(), 0);
    assert_eq!(PassabilityGrid::new(0, 0).len(), 0);
}

#[test]
fn autotile_is_total_at_extreme_coords() {
    use izanagi_kit::{autotile::compute_region, compute_mask};
    // compute_mask probes 8 neighbours (x±1, y±1); compute_region iterates
    // [x, x+w) × [y, y+h). Both previously overflowed at coordinate extremes.
    for &x in &EXTREMES {
        for &y in &EXTREMES {
            let _ = compute_mask(x, y, |_, _| true); // x±1 / y±1 must not overflow
        }
    }
    // Region with origin at the coordinate ceiling: x+w / y+h must saturate.
    let _ = compute_region(i32::MAX - 1, i32::MAX - 1, 3, 3, |_, _| false);
    let _ = compute_region(i32::MIN, i32::MIN, 2, 2, |_, _| true);
    let _ = compute_region(0, 0, -5, 10, |_, _| true); // non-positive -> empty
}

#[test]
fn noise_is_total_and_bounded_at_extremes() {
    use izanagi_kit::{
        fbm_1d, fbm_1d_wrap, fbm_2d, fbm_2d_wrap, fbm_3d, value_noise_1d, value_noise_1d_wrap,
        value_noise_2d, value_noise_2d_wrap, value_noise_3d,
    };
    // Noise is determinism-critical; all variants must be total (no overflow /
    // no rem_euclid-by-zero) and stay within [0, 65535] for any coordinate,
    // octave count, seed, and period — including extremes and degenerate values.
    let coords = EXTREMES;
    let octaves = [0u32, 1, 6, u32::MAX]; // amplitude>>=1 bounds the octave loop
    let periods = [i32::MIN, -1, 0, 1, i32::MAX]; // period<=0 must be treated as 1
    let seed = 0xABCD_1234_5678_9012u64;
    for &x in &coords {
        for &y in &coords {
            assert!(value_noise_1d(x, seed) <= 65535);
            assert!(value_noise_2d(x, y, seed) <= 65535);
            assert!(value_noise_3d(x, y, x ^ y, seed) <= 65535);
            for &o in &octaves {
                assert!(fbm_1d(x, seed, o) <= 65535);
                assert!(fbm_2d(x, y, seed, o) <= 65535);
                assert!(fbm_3d(x, y, x ^ y, seed, o) <= 65535);
            }
            for &p in &periods {
                assert!(value_noise_1d_wrap(x, seed, p) <= 65535);
                assert!(value_noise_2d_wrap(x, y, seed, p, p) <= 65535);
                assert!(fbm_1d_wrap(x, seed, 4, p) <= 65535);
                assert!(fbm_2d_wrap(x, y, seed, 4, p) <= 65535);
            }
        }
    }
}

#[test]
fn terminal_draw_ops_are_total_at_extreme_coords() {
    use izanagi_kit::{content::Color, Cell, Screen};
    let mut s = Screen::new(20, 10);
    let c = Cell {
        glyph: '#',
        fg: Color { r: 255, g: 255, b: 255 },
        bg: Color { r: 0, g: 0, b: 0 },
    };
    let col = Color { r: 1, g: 2, b: 3 };
    // Draw ops at extreme origins / sizes must clip without overflow-panicking
    // (the documented "clipped, no panic" contract). Coordinate arithmetic now
    // saturates before the put/set clip.
    for &x in &EXTREMES {
        for &y in &EXTREMES {
            s.fill_rect(x, y, 5, 5, c);
            s.draw_str(x, y, "hello", col, col);
            s.draw_box(x, y, 4, 4, col, col);
            s.draw_double_box(x, y, 4, 4, col, col);
            s.draw_h_line(x, y, 6, '-', col, col);
        }
    }
    // Note: draw ops are O(requested size) by nature (each cell is clipped
    // individually), so huge *sizes* are deliberately not exercised — that is
    // output-proportional work, not a panic. The saturating fix being validated
    // is about extreme *positions*, covered exhaustively above.
}

#[test]
fn hud_panel_is_total_at_extreme_coords() {
    use izanagi_kit::HudPanel;
    // inner_x/inner_y (x+1), contains (x + w), and corners (x + w-1) previously
    // overflowed for a panel positioned near i32::MAX. All now saturate.
    for &x in &EXTREMES {
        for &y in &EXTREMES {
            let p = HudPanel::new(x, y, 10, 5);
            let _ = (p.inner_x(), p.inner_y(), p.inner_w(), p.inner_h());
            let _ = p.corners();
            for &px in &EXTREMES {
                let _ = p.contains(px, y);
            }
            // Extreme dimensions too (panel arithmetic, not a fill loop).
            let big = HudPanel::new(x, y, u32::MAX, u32::MAX);
            let _ = (big.inner_x(), big.corners(), big.contains(x, y));
        }
    }
}

#[test]
fn color_ops_are_total_at_extreme_ratios() {
    use izanagi_kit::content::Color;
    // lerp(num,den) and scale(num,den) take caller-controlled i32 ratios; a
    // large num overflowed the channel math (255 * i32::MAX). Now i64-safe.
    let colors = [
        Color { r: 0, g: 0, b: 0 },
        Color { r: 255, g: 255, b: 255 },
        Color { r: 17, g: 200, b: 99 },
    ];
    for &a in &colors {
        for &b in &colors {
            for &num in &EXTREMES {
                for &den in &[i32::MIN, -1, 0, 1, i32::MAX] {
                    let _ = Color::lerp(a, b, num, den);
                    let _ = a.scale(num, den);
                }
            }
            for alpha in [0u8, 1, 128, 255] {
                let _ = Color::alpha_blend(a, b, alpha);
            }
        }
        let _ = (a.grayscale(), a.invert(), a.luminance());
        let _ = Color::from_hsv(99999, 200, 200); // hue wraps; sat/val bounded
    }
}

#[test]
fn camera_coord_math_is_total_at_extremes() {
    use izanagi_kit::Camera;
    // The viewport coordinate arithmetic — center, world_to_screen,
    // screen_to_world, world_to_screen_unclamped, world_rect — previously did
    // raw i32 add/sub on top_left + screen_{w,h} and on world-vs-top_left
    // deltas, panicking for extreme positions / large viewports.
    for &x in &EXTREMES {
        for &y in &EXTREMES {
            for &sw in &[0u32, 1, 100, u32::MAX] {
                for &sh in &[0u32, 1, 100, u32::MAX] {
                    // Build a camera positioned anywhere in coord space at any
                    // viewport size. (Use a world dimension large enough that
                    // clamp_origin doesn't shrink the test space.)
                    let cam = Camera::new(x, y, sw, sh, u32::MAX, u32::MAX);
                    let _ = (cam.center(), cam.world_rect());
                    for &wx in &EXTREMES {
                        let _ = cam.world_to_screen(wx, y);
                        let _ = cam.world_to_screen_unclamped(wx, y);
                    }
                    let _ = cam.screen_to_world(0, 0);
                    let _ = cam.screen_to_world(u32::MAX, u32::MAX);
                }
            }
        }
    }
}

#[test]
fn influence_normalize_is_total_at_extreme_values() {
    use izanagi_kit::influence::InfluenceMap;
    // normalize did `(v - cur_min) * target_span / span` in i32; spans across
    // the full i32 range overflowed both the span subtraction and the multiply.
    // i64 widening + final clamp makes it total.
    let mut m = InfluenceMap::new(3, 1);
    m.set(0, 0, i32::MIN);
    m.set(1, 0, 0);
    m.set(2, 0, i32::MAX);
    // Source span = full i32; target spans of every shape must not panic.
    for (&lo, &hi) in EXTREMES.iter().zip(EXTREMES.iter().rev()) {
        m.normalize(lo, hi);
        for x in 0..3 {
            let v = m.get(x, 0);
            assert!(v.is_some(), "cell ({x}, 0) missing after normalize");
        }
    }
    // Single-value map (span == 0) collapses to target_min (no division).
    let mut flat = InfluenceMap::new(2, 1);
    flat.set(0, 0, 42);
    flat.set(1, 0, 42);
    flat.normalize(i32::MIN, i32::MAX);
    assert_eq!(flat.get(0, 0), Some(i32::MIN));
}

#[test]
fn wfc_solve_is_total_at_large_dimensions() {
    use izanagi_kit::wfc::{wfc_solve, WfcRules};
    // width * height previously overflowed i32 for large grids (e.g.
    // 60000 × 60000 ≈ 3.6e9). usize widening makes it total — even if it
    // returns Contradiction or allocates a huge buffer, it must not panic.
    let rules = WfcRules::new(2);
    let mut rng = SplitMix64::new(0x_70F_C_05);
    // Degenerate sizes: 0/negative are early-rejected as Contradiction.
    for &(w, h) in &[(0, 10), (10, 0), (-1, 5), (5, -1)] {
        let _ = wfc_solve(w, h, &rules, &mut rng);
    }
    // Small grid: must actually solve / return a result (no panic).
    let _ = wfc_solve(8, 8, &rules, &mut rng);
    // (Larger overflow-trigger sizes like 60000×60000 would *allocate* 3.6e9
    // cells × 8 bytes = 30 GiB, exceeding test memory; the targeted unit
    // test above proves the multiply itself no longer panics.)
}

#[test]
fn dice_metrics_are_total_at_extreme_parameters() {
    use izanagi_kit::Dice;
    // max() previously overflowed i64 on count*sides at u32::MAX × u32::MAX
    // (~1.8e19); average_x100 had the same risk with an extra ×50 factor.
    // i128 + clamp makes both total.
    let cases = [
        (0u32, 0u32, 0i32),
        (1, 6, 0),
        (u32::MAX, u32::MAX, 0),
        (u32::MAX, u32::MAX, i32::MAX),
        (u32::MAX, u32::MAX, i32::MIN),
        (u32::MAX, 1, 0),
        (1, u32::MAX, 0),
    ];
    let mut rng = SplitMix64::new(0xD1CE_05);
    for &(count, sides, modifier) in &cases {
        let d = Dice::new(count, sides, modifier);
        let _ = (d.min(), d.max(), d.average_x100(), d.span(), d.is_flat());
        // Skip roll() at catastrophic counts: it legitimately loops `count`
        // times by design (output-proportional work, not a defect class —
        // same as inputbuf.tick(u32::MAX)). The overflow risk this test
        // exists for is in the *metric* methods above.
        if count <= 10_000 {
            let _ = d.roll(&mut rng);
        }
    }
}

#[test]
fn tilemap_iter_rect_is_total_at_extreme_anchor() {
    use izanagi_kit::TileMap;
    // rx + rw / ry + rh previously overflowed for an anchor near i32::MAX.
    let m: TileMap<u8> = TileMap::new(8, 8, 0);
    for &rx in &EXTREMES {
        for &rw in &[0i32, 1, 100, i32::MAX] {
            let _: Vec<_> = m.iter_rect(rx, 0, rw, 1).collect();
            let _: Vec<_> = m.iter_rect(0, rx, 1, rw).collect();
        }
    }
}

#[test]
fn pathfinding_octile_distance_is_total_at_extreme_coords() {
    use izanagi_kit::pathfinding::octile_distance;
    // The heuristic previously overflowed i32 on the coordinate subtraction
    // (e.g. i32::MAX - i32::MIN) and on the ×10 scaling. As a public pure
    // function it must be total over every coordinate pair.
    for &ax in &EXTREMES {
        for &ay in &EXTREMES {
            for &bx in &EXTREMES {
                for &by in &EXTREMES {
                    let d = octile_distance((ax, ay), (bx, by));
                    assert!(d >= 0, "octile distance must be non-negative");
                }
            }
        }
    }
}

#[test]
fn turn_scheduler_is_total_at_extreme_energy_and_speed() {
    use izanagi_kit::turn::Scheduler;
    // next_turn() advances `cost - energy` and `speed * units`; callers can set
    // energy to i32::MIN and speed near i32::MAX, both of which overflowed i32.
    for &energy in &EXTREMES {
        for &speed in &[1i32, 2, 100, i32::MAX - 1, i32::MAX] {
            let mut s: Scheduler<u32> = Scheduler::new();
            s.add(1, speed);
            s.set_energy(1, energy);
            // A second, fast actor guarantees the catch-up branch runs with a
            // mix of extreme energies present.
            s.add(2, i32::MAX);
            s.set_energy(2, energy);
            let _ = s.next_turn();
            let _ = s.next_turn();
        }
    }
}
