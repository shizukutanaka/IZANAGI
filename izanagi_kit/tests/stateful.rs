//! Model-based (stateful) test perspective.
//!
//! A lens distinct from both the example-based unit tests and the *stateless*
//! algebraic laws in `tests/properties.rs`. Here a structure is driven through a
//! long, random sequence of operations alongside an independent **reference
//! model**, and after *every* step the structure is asserted to agree with the
//! model and to uphold its public invariants.
//!
//! This is where history-dependent bugs live — the kind that only appear after a
//! particular operation sequence (e.g. free → recycle an index → insert), which
//! neither a fixed example nor a single-input property can reach. The
//! `SparseSet` generational-aliasing fix earlier in this branch is exactly such
//! a bug; the model below would have caught it (the model rejects a stale handle
//! at a recycled index, so a structure that did not would diverge immediately).
//!
//! Deterministic via the kit's own `SplitMix64`, so every run is reproducible in
//! CI without a `proptest`/`quickcheck` dependency.

use izanagi_kit::{Entity, EntityAllocator, SparseSet, SplitMix64};
use std::collections::{HashMap, HashSet};

// Each op is followed by a full-pool consistency scan, so the work is
// quadratic in OPS; this count keeps the sweep thorough (thousands of
// recycle/insert/remove cycles) while staying well under a second.
const OPS: usize = 3500;

fn pick<T: Copy>(pool: &[T], rng: &mut SplitMix64) -> Option<T> {
    if pool.is_empty() {
        None
    } else {
        Some(pool[rng.below(pool.len() as u32) as usize])
    }
}

#[test]
fn sparse_set_matches_index_slot_model_under_random_ops() {
    let mut rng = SplitMix64::new(0xB0A5_E7_1234);
    let mut alloc = EntityAllocator::new();
    let mut set: SparseSet<u32> = SparseSet::new();

    // Reference model of `SparseSet` semantics: each *index* holds at most one
    // (owning entity, value). insert overwrites the slot and rebinds its owner;
    // get/contains/remove require the FULL entity (index AND generation) to
    // match the slot's current owner — the generational guard.
    let mut model: HashMap<u32, (Entity, u32)> = HashMap::new();

    // Every entity ever handed out (live or stale) — the check surface.
    let mut pool: Vec<Entity> = Vec::new();
    let mut next_value: u32 = 0;

    let model_get = |model: &HashMap<u32, (Entity, u32)>, e: Entity| -> Option<u32> {
        model
            .get(&e.index())
            .filter(|(owner, _)| *owner == e)
            .map(|(_, v)| *v)
    };

    for _ in 0..OPS {
        match rng.below(7) {
            0 => {
                // Allocate a fresh entity (often reusing a freed index at a
                // bumped generation — the generational stress case).
                let e = alloc.allocate();
                pool.push(e);
            }
            1 => {
                // Free a live entity so its index can later be recycled.
                if let Some(e) = pick(&pool, &mut rng) {
                    alloc.free(e); // no-op if already stale
                }
            }
            2 | 3 => {
                // Insert at a pool entity (which may be stale or freshly recycled).
                if let Some(e) = pick(&pool, &mut rng) {
                    next_value = next_value.wrapping_add(1);
                    set.insert(e, next_value);
                    model.insert(e.index(), (e, next_value)); // overwrite slot + owner
                }
            }
            4 => {
                // Remove must agree on the returned value, and only succeed when
                // the full handle matches the slot owner.
                if let Some(e) = pick(&pool, &mut rng) {
                    let got = set.remove(e);
                    let expected = match model.get(&e.index()) {
                        Some((owner, _)) if *owner == e => model.remove(&e.index()).map(|(_, v)| v),
                        _ => None,
                    };
                    assert_eq!(got, expected, "remove({e:?}) returned {got:?}, model {expected:?}");
                }
            }
            5 => {
                if rng.below(12) == 0 {
                    set.clear();
                    model.clear();
                }
            }
            _ => {
                // Point-read agreement on a random handle (incl. stale ones).
                if let Some(e) = pick(&pool, &mut rng) {
                    assert_eq!(set.get(e).copied(), model_get(&model, e), "get({e:?})");
                    assert_eq!(
                        set.contains(e),
                        model_get(&model, e).is_some(),
                        "contains({e:?})"
                    );
                }
            }
        }

        // Whole-structure invariants after every operation.
        assert_eq!(set.len(), model.len(), "len disagrees with model");
        assert_eq!(set.is_empty(), model.is_empty(), "is_empty disagrees");
        for &e in &pool {
            assert_eq!(set.get(e).copied(), model_get(&model, e), "get({e:?}) post-op");
            assert_eq!(
                set.contains(e),
                model_get(&model, e).is_some(),
                "contains({e:?}) post-op"
            );
        }
        // iter() must yield exactly the model's current (entity, value) slots.
        let mut from_iter: Vec<(u32, u32, u32)> =
            set.iter().map(|(e, &v)| (e.index(), e.generation(), v)).collect();
        let mut from_model: Vec<(u32, u32, u32)> = model
            .values()
            .map(|(e, v)| (e.index(), e.generation(), *v))
            .collect();
        from_iter.sort_unstable();
        from_model.sort_unstable();
        assert_eq!(from_iter, from_model, "iter contents disagree with model");
    }
}

#[test]
fn entity_allocator_matches_live_set_model_under_random_ops() {
    let mut rng = SplitMix64::new(0x_A110C_5EED);
    let mut alloc = EntityAllocator::new();

    // Reference model: the set of currently-live entities.
    let mut live: HashSet<Entity> = HashSet::new();
    let mut pool: Vec<Entity> = Vec::new();

    for _ in 0..OPS {
        match rng.below(3) {
            0 => {
                let e = alloc.allocate();
                assert!(live.insert(e), "allocate handed out a duplicate live entity {e:?}");
                pool.push(e);
            }
            1 => {
                if let Some(e) = pick(&pool, &mut rng) {
                    let was_live = live.contains(&e);
                    alloc.free(e);
                    if was_live {
                        live.remove(&e);
                    }
                    // Freeing a stale/duplicate handle must be a no-op for the
                    // model too (already absent).
                }
            }
            _ => {
                // batch free a couple of pool entries.
                for _ in 0..2 {
                    if let Some(e) = pick(&pool, &mut rng) {
                        let was_live = live.contains(&e);
                        alloc.free(e);
                        if was_live {
                            live.remove(&e);
                        }
                    }
                }
            }
        }

        // Liveness and counts must mirror the model after every operation.
        assert_eq!(alloc.count(), live.len(), "live count disagrees with model");
        for &e in &pool {
            assert_eq!(alloc.is_alive(e), live.contains(&e), "is_alive({e:?}) disagrees");
        }
        let reported: HashSet<Entity> = alloc.live_entities().into_iter().collect();
        assert_eq!(reported, live, "live_entities() disagrees with model");
    }
}
