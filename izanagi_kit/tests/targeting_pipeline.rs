//! Integration coverage for the ranged-combat geometry suite.
//!
//! Exercises the public re-exports (`ray_cast`, `ray_blocked_at`, `cone`,
//! `knockback`, `line_of_sight`, `vec_toward`) the way a game resolves a ranged
//! attack end-to-end: trace a bolt to its impact, detonate a wall-aware cone at
//! the impact, then knock a caught entity back. The whole pipeline is integer-
//! only and deterministic, so the asserted cells are exact.

use izanagi_kit::{cone, knockback, line_of_sight, ray_blocked_at, ray_cast, vec_toward};

/// A tiny arena: open floor except a vertical wall segment at x == 4 for
/// 1 <= y <= 5. `(x, y)` is a wall when this returns true.
fn is_wall(x: i32, y: i32) -> bool {
    x == 4 && (1..=5).contains(&y)
}

#[test]
fn test_bolt_blocked_by_wall_short_of_target() {
    // Shooter at (0,3) fires at (8,3); the wall at (4,3) stops the bolt.
    let origin = (0, 3);
    let target = (8, 3);
    let path = ray_cast(origin, target, is_wall);
    assert_eq!(path.last(), Some(&(4, 3)), "bolt is absorbed by the wall");
    assert_eq!(
        ray_blocked_at(origin, target, is_wall),
        Some((4, 3)),
        "targeting query agrees on the blocking cell"
    );
    // Interior pass-through cells are all open floor before the wall.
    for &(x, y) in &path[1..path.len() - 1] {
        assert!(!is_wall(x, y), "pass-through cell ({x},{y}) must be open");
    }
}

#[test]
fn test_clear_bolt_reaches_target_then_detonates_cone() {
    // Fire along an open corridor (y = 8, below the wall) to a clear target.
    let origin = (0, 8);
    let target = (6, 8);
    assert_eq!(ray_blocked_at(origin, target, is_wall), None, "clear shot");
    let path = ray_cast(origin, target, is_wall);
    let impact = *path.last().unwrap();
    assert_eq!(impact, target, "bolt lands on the intended target");

    // Detonate a cone breath continuing in the bolt's travel direction.
    let facing = vec_toward(origin, impact);
    assert_eq!(facing, (1, 0));
    let blast = cone(impact, facing, 3);
    assert!(!blast.is_empty());
    // Every blasted cell is in front of the impact (east of it).
    assert!(blast.iter().all(|&(x, _)| x > impact.0));
}

#[test]
fn test_wall_aware_cone_excludes_shadowed_cells() {
    // A cone fired into the wall: filtering by line-of-sight from the burst
    // point drops cells the wall occludes, so a pure-shape cone and the
    // wall-aware cone differ when the wall actually blocks some cells.
    let burst = (2, 3);
    let facing = (1, 0); // toward the wall at x == 4
    let shape: Vec<(i32, i32)> = cone(burst, facing, 4);
    let visible: Vec<(i32, i32)> = shape
        .iter()
        .copied()
        .filter(|&cell| line_of_sight(burst, cell, is_wall))
        .collect();
    assert!(
        visible.len() < shape.len(),
        "the wall must occlude at least one cone cell"
    );
    // Cells beyond the wall on the central axis are not line-of-sight visible.
    assert!(
        shape.contains(&(6, 3)),
        "(6,3) is within the pure cone shape"
    );
    assert!(
        !visible.contains(&(6, 3)),
        "(6,3) sits behind the wall and must be culled"
    );
}

#[test]
fn test_knockback_away_from_burst_stops_at_wall() {
    // An entity at (3,3) is knocked away from a burst at (1,3): it is pushed
    // east and slams into the wall at (4,3), halting on (3,3) — it cannot move.
    let burst = (1, 3);
    let victim = (3, 3);
    let dir = vec_toward(burst, victim);
    assert_eq!(dir, (1, 0));
    let landing = knockback(victim, dir, 5, is_wall);
    assert_eq!(landing, (3, 3), "wall at (4,3) blocks the very first step");
}

#[test]
fn test_knockback_into_open_space_travels_full_distance() {
    // Same burst, but a victim in the open corridor (y = 8) flies the full push.
    let burst = (1, 8);
    let victim = (3, 8);
    let dir = vec_toward(burst, victim);
    let landing = knockback(victim, dir, 4, is_wall);
    assert_eq!(landing, (7, 8), "no wall: pushed the full 4 cells east");
}

#[test]
fn test_full_pipeline_is_deterministic() {
    // Re-running the whole resolve must produce byte-identical results.
    let run = || {
        let path = ray_cast((0, 8), (6, 8), is_wall);
        let impact = *path.last().unwrap();
        let blast = cone(impact, vec_toward((0, 8), impact), 3);
        let kb = knockback(impact, (1, 0), 3, is_wall);
        (path, blast, kb)
    };
    assert_eq!(run(), run(), "deterministic ranged-attack resolution");
}
