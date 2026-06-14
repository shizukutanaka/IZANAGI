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

use izanagi_kit::{astar, fov_to_vec, is_reachable, path_cost, SplitMix64, TileMap};
use std::collections::HashSet;

const CASES: usize = 400;

fn rand_walls(rng: &mut SplitMix64, n: u32, w: i32, h: i32) -> HashSet<(i32, i32)> {
    (0..n)
        .map(|_| (rng.range(0, w), rng.range(0, h)))
        .collect()
}

/// Pathfinding depends only on relative geometry: translating the bounded world
/// (walls + the box that bounds the search) by `d` must preserve both
/// reachability and optimal path cost. Cost rather than the path itself, since
/// A*'s tie-break among equal-cost paths need not be translation-equivariant —
/// but the *optimal cost* is a pure function of the relative layout.
#[test]
fn pathfinding_is_translation_invariant() {
    const N: i32 = 12;
    let mut rng = SplitMix64::new(0x_9A7_4F1);
    for _ in 0..CASES {
        let mut walls = rand_walls(&mut rng, 24, N, N);
        let start = (0, 0);
        let goal = (N - 1, N - 1);
        walls.remove(&start);
        walls.remove(&goal);

        let in_box = |x: i32, y: i32, ox: i32, oy: i32| {
            x >= ox && x < ox + N && y >= oy && y < oy + N
        };

        // Original world: out-of-box and wall cells are blocked.
        let blocked = |x: i32, y: i32| !in_box(x, y, 0, 0) || walls.contains(&(x, y));
        let reach1 = is_reachable(start, goal, blocked);
        let cost1 = astar(start, goal, blocked).map(|p| path_cost(&p));

        // Translate the entire world by d.
        let (dx, dy) = (rng.range(-50, 51), rng.range(-50, 51));
        let blocked_t = |x: i32, y: i32| {
            !in_box(x, y, dx, dy) || walls.contains(&(x - dx, y - dy))
        };
        let start_t = (start.0 + dx, start.1 + dy);
        let goal_t = (goal.0 + dx, goal.1 + dy);
        let reach2 = is_reachable(start_t, goal_t, blocked_t);
        let cost2 = astar(start_t, goal_t, blocked_t).map(|p| path_cost(&p));

        assert_eq!(reach1, reach2, "reachability changed under translation by ({dx},{dy})");
        assert_eq!(cost1, cost2, "optimal path cost changed under translation by ({dx},{dy})");
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
    let mut rng = SplitMix64::new(0x_F0F_4E2);
    for _ in 0..CASES {
        let opaque = rand_walls(&mut rng, 30, N, N);
        let origin = (rng.range(0, N), rng.range(0, N));

        let is_op = |x: i32, y: i32| opaque.contains(&(x, y));
        let vis1: Vec<(i32, i32)> = fov_to_vec(origin, radius, is_op);

        let (dx, dy) = (rng.range(-40, 41), rng.range(-40, 41));
        let is_op_t = |x: i32, y: i32| opaque.contains(&(x - dx, y - dy));
        let vis2: HashSet<(i32, i32)> =
            fov_to_vec((origin.0 + dx, origin.1 + dy), radius, is_op_t)
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
    let mut rng = SplitMix64::new(0x_70_7A7E);
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
        assert_eq!(multiset(&cw), multiset(&map), "rotation changed the value multiset");

        // Four 90° rotations return the original map exactly.
        let back = map.rotated_cw().rotated_cw().rotated_cw().rotated_cw();
        assert_eq!((back.width(), back.height()), (w, h), "4x rotation must restore dims");
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                assert_eq!(back.get(x, y), map.get(x, y), "4x rotation must restore cell ({x},{y})");
            }
        }
    }
}
