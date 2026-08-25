//! Metamorphic-relation test perspective.
//!
//! The golden, property, model-based, and differential lenses each need
//! *something to compare against* — a pinned value, an algebraic self-relation,
//! a reference model, or an `f64` oracle. But the kit's richest algorithms —
//! pathfinding, field-of-view, map transforms — have no tractable oracle: you
//! cannot cheaply state "the FOV of this map is exactly these cells".
//!
//! Metamorphic testing sidesteps the missing oracle. Instead of checking an
//! output, it transforms an input by a symmetry that *should* induce a known
//! change in the output, and asserts that **relation** holds. The claim under
//! test here: these spatial algorithms depend only on *relative* geometry, not
//! absolute coordinates — so translating the whole world translates the result,
//! and rotating a map permutes its cells without losing any. A coordinate-
//! dependent or symmetry-breaking bug surfaces even with no ground truth.
//!
//! Deterministic via `SplitMix64`; reproducible in CI.

use izanagi_kit::fov::can_see;
use izanagi_kit::{astar, fov_to_vec, is_reachable, path_cost, SplitMix64, TileMap, TimerQueue};
use std::collections::HashSet;

const CASES: usize = 400;

fn rand_walls(rng: &mut SplitMix64, n: u32, w: i32, h: i32) -> HashSet<(i32, i32)> {
    (0..n).map(|_| (rng.range(0, w), rng.range(0, h))).collect()
}

/// Pathfinding depends only on relative geometry: translating the bounded world
/// (walls + the box that bounds the search) by `d` must preserve both
/// reachability and optimal path cost. Cost rather than the path itself, since
/// A*'s tie-break among equal-cost paths need not be translation-equivariant —
/// but the *optimal cost* is a pure function of the relative layout.
#[test]
fn pathfinding_is_translation_invariant() {
    const N: i32 = 12;
    let mut rng = SplitMix64::new(0x9A74F1);
    for _ in 0..CASES {
        let mut walls = rand_walls(&mut rng, 24, N, N);
        let start = (0, 0);
        let goal = (N - 1, N - 1);
        walls.remove(&start);
        walls.remove(&goal);

        let in_box =
            |x: i32, y: i32, ox: i32, oy: i32| x >= ox && x < ox + N && y >= oy && y < oy + N;

        // Original world: out-of-box and wall cells are blocked.
        let blocked = |x: i32, y: i32| !in_box(x, y, 0, 0) || walls.contains(&(x, y));
        let reach1 = is_reachable(start, goal, blocked);
        let cost1 = astar(start, goal, blocked).map(|p| path_cost(&p));

        // Translate the entire world by d.
        let (dx, dy) = (rng.range(-50, 51), rng.range(-50, 51));
        let blocked_t = |x: i32, y: i32| !in_box(x, y, dx, dy) || walls.contains(&(x - dx, y - dy));
        let start_t = (start.0 + dx, start.1 + dy);
        let goal_t = (goal.0 + dx, goal.1 + dy);
        let reach2 = is_reachable(start_t, goal_t, blocked_t);
        let cost2 = astar(start_t, goal_t, blocked_t).map(|p| path_cost(&p));

        assert_eq!(
            reach1, reach2,
            "reachability changed under translation by ({dx},{dy})"
        );
        assert_eq!(
            cost1, cost2,
            "optimal path cost changed under translation by ({dx},{dy})"
        );
    }
}

/// FOV depends only on relative geometry: the set of visible cells *relative to
/// the origin* is unchanged when the origin and the opacity map are translated
/// together. Equivalently, `fov(o, M)` translated by `d` equals
/// `fov(o + d, M translated by d)`.
#[test]
fn fov_is_translation_invariant() {
    const N: i32 = 16;
    let radius = 6;
    let mut rng = SplitMix64::new(0xF0F4E2);
    for _ in 0..CASES {
        let opaque = rand_walls(&mut rng, 30, N, N);
        let origin = (rng.range(0, N), rng.range(0, N));

        let is_op = |x: i32, y: i32| opaque.contains(&(x, y));
        let vis1: Vec<(i32, i32)> = fov_to_vec(origin, radius, is_op);

        let (dx, dy) = (rng.range(-40, 41), rng.range(-40, 41));
        let is_op_t = |x: i32, y: i32| opaque.contains(&(x - dx, y - dy));
        let vis2: HashSet<(i32, i32)> = fov_to_vec((origin.0 + dx, origin.1 + dy), radius, is_op_t)
            .into_iter()
            .collect();

        // vis1 shifted by d must equal vis2 exactly.
        let vis1_shifted: HashSet<(i32, i32)> =
            vis1.iter().map(|&(x, y)| (x + dx, y + dy)).collect();
        assert_eq!(
            vis1_shifted, vis2,
            "FOV visible set not translation-invariant by ({dx},{dy}) from origin {origin:?}"
        );
    }
}

/// Rotating a `TileMap` 90° is a permutation of its cells: the multiset of tile
/// values is preserved, the dimensions swap, and four rotations return the
/// original map. No value can be created, lost, or duplicated by a rotation.
#[test]
fn tilemap_rotation_preserves_cell_multiset() {
    let mut rng = SplitMix64::new(0x707A7E);
    for _ in 0..CASES {
        let w = rng.range(1, 7) as u32;
        let h = rng.range(1, 7) as u32;
        let mut map: TileMap<u32> = TileMap::new(w, h, 0);
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                map.set(x, y, rng.range(0, 1000) as u32);
            }
        }

        let multiset = |m: &TileMap<u32>| -> Vec<u32> {
            let mut v = Vec::new();
            for y in 0..m.height() as i32 {
                for x in 0..m.width() as i32 {
                    v.push(*m.get(x, y).unwrap());
                }
            }
            v.sort_unstable();
            v
        };

        let cw = map.rotated_cw();
        // Dimensions swap under a 90° rotation.
        assert_eq!((cw.width(), cw.height()), (h, w), "rotated dims must swap");
        // The multiset of values is preserved (rotation is a bijection on cells).
        assert_eq!(
            multiset(&cw),
            multiset(&map),
            "rotation changed the value multiset"
        );

        // Four 90° rotations return the original map exactly.
        let back = map.rotated_cw().rotated_cw().rotated_cw().rotated_cw();
        assert_eq!(
            (back.width(), back.height()),
            (w, h),
            "4x rotation must restore dims"
        );
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                assert_eq!(
                    back.get(x, y),
                    map.get(x, y),
                    "4x rotation must restore cell ({x},{y})"
                );
            }
        }
    }
}

/// Equal iff same dimensions and same cell at every coordinate (TileMap has no
/// derived PartialEq).
fn maps_equal(a: &TileMap<u32>, b: &TileMap<u32>) -> bool {
    if a.width() != b.width() || a.height() != b.height() {
        return false;
    }
    for y in 0..a.height() as i32 {
        for x in 0..a.width() as i32 {
            if a.get(x, y) != b.get(x, y) {
                return false;
            }
        }
    }
    true
}

/// TileMap transforms obey their group laws: cw and ccw rotations are inverses,
/// the two flips are involutions, and flip_h∘flip_v equals a 180° rotation
/// (== cw twice). These are metamorphic relations — two routes to the same map.
#[test]
fn tilemap_transforms_round_trip_and_compose() {
    let mut rng = SplitMix64::new(0x717E_3A05);
    for _ in 0..CASES {
        let w = rng.range(1, 7) as u32;
        let h = rng.range(1, 7) as u32;
        let mut m: TileMap<u32> = TileMap::new(w, h, 0);
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                m.set(x, y, rng.range(0, 1000) as u32);
            }
        }

        // Rotations are inverse pairs.
        assert!(
            maps_equal(&m.rotated_cw().rotated_ccw(), &m),
            "cw∘ccw != identity"
        );
        assert!(
            maps_equal(&m.rotated_ccw().rotated_cw(), &m),
            "ccw∘cw != identity"
        );

        // Flips are involutions (applying twice restores the original).
        let mut fh = m.clone();
        fh.flip_h();
        fh.flip_h();
        assert!(maps_equal(&fh, &m), "flip_h twice != identity");
        let mut fv = m.clone();
        fv.flip_v();
        fv.flip_v();
        assert!(maps_equal(&fv, &m), "flip_v twice != identity");

        // flip_h ∘ flip_v == 180° rotation == rotate_cw applied twice.
        let mut fhv = m.clone();
        fhv.flip_h();
        fhv.flip_v();
        let rot180 = m.rotated_cw().rotated_cw();
        assert!(maps_equal(&fhv, &rot180), "flip_h∘flip_v != rotate 180");
    }
}

/// For one-shot timers, the multiset of fired events does not depend on how the
/// total advance is divided — `advance(total)` fires the same events as
/// advancing in arbitrary chunks summing to `total`. (Recurring timers fire once
/// per `advance` call, so this invariant is one-shot-only.) This exercises the
/// delay accounting without reimplementing `advance` as an oracle.
#[test]
fn timer_queue_one_shot_fires_are_split_advance_invariant() {
    let mut rng = SplitMix64::new(0x717E_9A05);
    for _ in 0..CASES {
        let n = rng.range(1, 12) as usize;
        let timers: Vec<(u32, u32)> = (0..n)
            .map(|i| (rng.range(0, 50) as u32, i as u32))
            .collect();
        let total = rng.range(0, 60) as u32;

        // One big advance.
        let single = {
            let mut q: TimerQueue<u32> = TimerQueue::new();
            for &(d, e) in &timers {
                q.schedule(d, e);
            }
            q.advance(total)
        };

        // The same total, split into random chunks.
        let chunked = {
            let mut q: TimerQueue<u32> = TimerQueue::new();
            for &(d, e) in &timers {
                q.schedule(d, e);
            }
            let mut out = Vec::new();
            let mut remaining = total;
            while remaining > 0 {
                let chunk = rng.range(1, (remaining + 1) as i32) as u32;
                out.extend(q.advance(chunk));
                remaining -= chunk;
            }
            if total == 0 {
                out.extend(q.advance(0)); // delay-0 timers fire on advance(0)
            }
            out
        };

        let mut a = single.clone();
        let mut b = chunked;
        a.sort_unstable();
        b.sort_unstable();
        assert_eq!(
            a, b,
            "one-shot fired multiset differs: single vs chunked advance"
        );

        // Each one-shot with delay <= total fires exactly once.
        let expected = timers.iter().filter(|(d, _)| *d <= total).count();
        assert_eq!(single.len(), expected, "wrong one-shot fire count");
    }
}

/// The *defining* relation of symmetric shadowcasting: for two transparent
/// cells A and B within radius, `A can see B` ⇔ `B can see A` — the very claim
/// `can_see`'s docs make ("A sees B ⟺ B sees A"). The existing unit test pins
/// it on a single hand-built map; this exercises the public `can_see` API over
/// hundreds of random wall layouts and endpoint pairs, where an asymmetry in
/// the slope-window logic (the `is_symmetric` centre test) would surface as a
/// pair that sees one-way only. Walls are opaque endpoints are skipped — the
/// guarantee is stated for transparent cells.
#[test]
fn fov_visibility_is_symmetric_across_random_maps() {
    const N: i32 = 12;
    const RADIUS: i32 = 8;
    let mut rng = SplitMix64::new(0x5EE_C0DE);
    let mut checked = 0u64;
    for _ in 0..CASES {
        let walls = rand_walls(&mut rng, 30, N, N);
        let is_opaque =
            |x: i32, y: i32| x < 0 || y < 0 || x >= N || y >= N || walls.contains(&(x, y));

        let a = (rng.range(0, N), rng.range(0, N));
        let b = (rng.range(0, N), rng.range(0, N));
        if is_opaque(a.0, a.1) || is_opaque(b.0, b.1) {
            continue; // symmetry is guaranteed for transparent endpoints
        }
        let ab = can_see(a, b, RADIUS, is_opaque);
        let ba = can_see(b, a, RADIUS, is_opaque);
        assert_eq!(ab, ba, "FOV asymmetry: {a:?}<->{b:?} ab={ab} ba={ba}");
        checked += 1;
    }
    assert!(checked > 0, "no transparent endpoint pairs were exercised");
}
