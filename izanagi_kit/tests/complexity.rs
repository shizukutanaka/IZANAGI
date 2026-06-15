//! Complexity / work-bound test perspective.
//!
//! The other lenses all ask "is the answer correct?". None asks "how much work
//! does it take to get there?" — yet an algorithm whose cost scales with an
//! incidental magnitude (absolute coordinates, query span) rather than its
//! relevant input (radius, path length, explored region) is a real defect: it
//! can hang or DoS a deterministic engine even while returning the right answer.
//!
//! This lens measures work *deterministically* — no flaky wall-clock timing — by
//! instrumenting the predicate closures these algorithms already take and
//! counting invocations. The Socratic claim under test: the number of
//! `is_opaque` / `is_blocked` calls is bounded by the algorithm's logical input,
//! and in particular is *independent of absolute position*.
//!
//! Deterministic via `SplitMix64`.

use izanagi_kit::{astar, compute_fov, flood_fill, is_reachable, ray_cast, SpatialHash, SplitMix64};
use std::cell::Cell;

/// Count `is_opaque` invocations for a `compute_fov` call at `origin`.
fn fov_opaque_calls(origin: (i32, i32), radius: i32) -> usize {
    let count = Cell::new(0usize);
    compute_fov(
        origin,
        radius,
        |_x, _y| {
            count.set(count.get() + 1);
            false
        },
        |_x, _y| {},
    );
    count.get()
}

#[test]
fn fov_work_is_independent_of_absolute_position() {
    // The strongest form of the bound: FOV explores a fixed neighbourhood shape,
    // so the work is *identical* regardless of where in the coordinate space the
    // origin sits. A position-dependent or unbounded FOV breaks this.
    let r = 8;
    let near = fov_opaque_calls((0, 0), r);
    let far = fov_opaque_calls((1_000_000, -2_000_000), r);
    let other = fov_opaque_calls((i32::MIN / 2, i32::MAX / 2), r);
    assert!(near > 0, "FOV did no work");
    assert_eq!(near, far, "FOV work depends on absolute position");
    assert_eq!(near, other, "FOV work depends on absolute position");
}

#[test]
fn fov_work_grows_with_radius_within_the_radius_square() {
    // Work is bounded by the (2r+1)² cells of the radius square and grows with r.
    for r in [1, 4, 8, 16] {
        let calls = fov_opaque_calls((50, 50), r);
        // Shadowcasting re-probes cells on octant boundaries (axes/diagonals), so
        // the count is a small constant factor over the (2r+1)² square — but it
        // is still O(r²): bounded by a constant multiple of the radius square,
        // never by absolute coordinates. (Observed ratio ≤ 1.33×.)
        let bound = 2 * ((2 * r + 1) * (2 * r + 1)) as usize;
        assert!(calls > 0 && calls <= bound, "r={r}: {calls} calls exceed O(r²) bound {bound}");
    }
    assert!(
        fov_opaque_calls((0, 0), 16) > fov_opaque_calls((0, 0), 4),
        "larger radius must do more work"
    );
}

#[test]
fn ray_cast_tests_each_cell_after_origin_exactly_once() {
    // is_blocked is called once per traversed cell *after* the origin — never
    // the origin, never twice. The tightest possible work bound.
    for &(tx, ty) in &[(10, 0), (0, 7), (9, 5), (-6, -3), (12, 12)] {
        let count = Cell::new(0usize);
        let path = ray_cast((0, 0), (tx, ty), |_x, _y| {
            count.set(count.get() + 1);
            false
        });
        assert_eq!(
            count.get(),
            path.len() - 1,
            "ray_cast to ({tx},{ty}) tested {} cells for a {}-cell path",
            count.get(),
            path.len()
        );
    }
}

#[test]
fn astar_exploration_is_bounded_by_the_reachable_box_not_coordinates() {
    // A* on a bounded open box must probe a bounded number of cells — and the
    // same number whether the box sits at the origin or far away (position
    // independence). An unbounded or coordinate-scaled search fails this.
    let probe = |ox: i32, oy: i32| -> usize {
        let n = 25;
        let count = Cell::new(0usize);
        let blocked = |x: i32, y: i32| {
            count.set(count.get() + 1);
            !(ox..ox + n).contains(&x) || !(oy..oy + n).contains(&y)
        };
        let _ = astar((ox, oy), (ox + n - 1, oy + n - 1), blocked);
        count.get()
    };
    let near = probe(0, 0);
    let far = probe(500_000, -700_000);
    assert!(near > 0, "A* did no work");
    assert_eq!(near, far, "A* work depends on absolute position");
    // Generous structural bound: each of n*n cells is probed a small constant
    // number of times (≤ 8 neighbours + bookkeeping).
    assert!(near <= 25 * 25 * 9, "A* explored unboundedly: {near}");
}

#[test]
fn flood_fill_and_reachable_work_is_position_independent() {
    let probe_fill = |ox: i32, oy: i32| -> usize {
        let count = Cell::new(0usize);
        let blocked = |x: i32, y: i32| {
            count.set(count.get() + 1);
            !(ox..ox + 12).contains(&x) || !(oy..oy + 12).contains(&y)
        };
        let _ = flood_fill((ox, oy), 6, blocked);
        count.get()
    };
    assert_eq!(probe_fill(0, 0), probe_fill(900_000, 900_000), "flood_fill position-dependent");

    let probe_reach = |ox: i32, oy: i32| -> usize {
        let count = Cell::new(0usize);
        let blocked = |x: i32, y: i32| {
            count.set(count.get() + 1);
            !(ox..ox + 15).contains(&x) || !(oy..oy + 15).contains(&y)
        };
        let _ = is_reachable((ox, oy), (ox + 14, oy + 14), blocked);
        count.get()
    };
    assert_eq!(probe_reach(0, 0), probe_reach(-800_000, 800_000), "is_reachable position-dependent");
}

#[test]
fn fov_smoke_uses_seeded_rng_for_opacity() {
    // A randomized-opacity FOV still does bounded, position-independent work.
    let mut rng = SplitMix64::new(0x_C0FFEE);
    let walls: Vec<(i32, i32)> = (0..40).map(|_| (rng.range(0, 20), rng.range(0, 20))).collect();
    let calls_at = |ox: i32, oy: i32| -> usize {
        let count = Cell::new(0usize);
        compute_fov(
            (ox + 10, oy + 10),
            7,
            |x, y| {
                count.set(count.get() + 1);
                walls.contains(&(x - ox, y - oy))
            },
            |_x, _y| {},
        );
        count.get()
    };
    assert_eq!(calls_at(0, 0), calls_at(300_000, 300_000), "randomized FOV work position-dependent");
}

#[test]
fn spatial_hash_whole_world_query_is_bounded_and_correct() {
    // A query whose span covers (nearly) the entire coordinate space must NOT
    // iterate the O(area) span — it must fall back to scanning the populated
    // cells. Before the sparse-path fix this hung; now it returns instantly with
    // the correct result. The test *completing* is the work-bound proof; the
    // assertions pin correctness and dense/sparse-path agreement.
    let mut g: SpatialHash<u32> = SpatialHash::new(8);
    let pts = [(0, 0), (100, 5), (-40, 30), (7, -9), (1000, 1000)];
    for (i, &(x, y)) in pts.iter().enumerate() {
        g.insert(i as u32, x, y);
    }

    // Whole-world query (huge span -> sparse path). Must return every key.
    let huge = g.query_rect(i32::MIN / 2, i32::MIN / 2, i32::MAX, i32::MAX);
    assert_eq!(huge.len(), pts.len(), "whole-world query lost or duplicated keys");
    for k in 0..pts.len() as u32 {
        assert!(huge.contains(&k), "key {k} missing from whole-world query");
    }
    assert_eq!(
        g.query_rect_count(i32::MIN / 2, i32::MIN / 2, i32::MAX, i32::MAX),
        pts.len(),
        "count disagrees on whole-world query"
    );

    // Exercise the DENSE path: a tiny query (span ≤ populated-cell count) around
    // (0,0) takes the span-walking branch and must return exactly key 0.
    let dense = g.query_rect(-1, -1, 4, 4); // ~1 cell span → dense path
    assert_eq!(dense, vec![0], "dense-path query returned the wrong cell");
    // Consistency: every key the dense query found is also in the sparse result.
    assert!(dense.iter().all(|k| huge.contains(k)), "dense ⊄ sparse result");
}
