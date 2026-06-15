//! API-equivalence test perspective.
//!
//! The kit documents several convenience/batch methods as *exactly equivalent*
//! to a primitive sequence — `apply_all` ≡ a loop of `apply`, `batch_free` ≡ a
//! loop of `free`, `extend_all` ≡ `extend_duration` on every key. Those are
//! claims, and batch/shorthand methods are precisely where an optimization
//! quietly diverges from the loop it replaces (a different stacking rule, a
//! skipped element, a saturation handled once instead of per-item).
//!
//! This lens mechanizes the equivalence claims: it builds one structure via the
//! shorthand and another via the documented primitive sequence over the *same*
//! random data, and asserts the two are indistinguishable — identical canonical
//! hash for `DetHash` types, identical observable state otherwise. It is
//! orthogonal to the other lenses: not a value, law, model, oracle, symmetry, or
//! ordering, but the equivalence of two *API surfaces* for the same intent.
//!
//! Deterministic via `SplitMix64`.

use izanagi_kit::{hash_state, Effect, EntityAllocator, SplitMix64, StatusSet};

const TRIALS: usize = 500;

/// `StatusSet::apply_all(&effects)` must equal calling `apply` for each entry,
/// in slice order (the documented definition).
#[test]
fn status_apply_all_equals_loop_of_apply() {
    let mut rng = SplitMix64::new(0x_A99_A11_01);
    for _ in 0..TRIALS {
        let n = rng.range(0, 20) as usize;
        let effects: Vec<(u32, Effect)> = (0..n)
            .map(|i| {
                (
                    i as u32,
                    Effect {
                        remaining: (rng.range(1, 50)) as u32,
                        magnitude: rng.range(-30, 31),
                    },
                )
            })
            .collect();

        let via_batch = {
            let mut s: StatusSet<u32> = StatusSet::new();
            s.apply_all(&effects);
            s
        };
        let via_loop = {
            let mut s: StatusSet<u32> = StatusSet::new();
            for (k, e) in &effects {
                s.apply(*k, e.remaining, e.magnitude);
            }
            s
        };
        assert_eq!(
            hash_state(&via_batch),
            hash_state(&via_loop),
            "apply_all diverged from a loop of apply"
        );
    }
}

/// `StatusSet::extend_all(n)` must equal calling `extend_duration(key, n)` on
/// every active key (saturating per item, exactly as the loop would).
#[test]
fn status_extend_all_equals_loop_of_extend_duration() {
    let mut rng = SplitMix64::new(0x_E47_E11_02);
    for _ in 0..TRIALS {
        let n = rng.range(0, 16) as usize;
        // Seed both sets identically with distinct keys (some near u32::MAX to
        // exercise the saturation path in both code paths).
        let seed: Vec<(u32, u32, i32)> = (0..n)
            .map(|i| {
                let rem = if rng.below(4) == 0 {
                    u32::MAX - rng.range(0, 5) as u32
                } else {
                    rng.range(1, 100) as u32
                };
                (i as u32, rem, rng.range(-10, 11))
            })
            .collect();
        let extra = rng.range(0, 1000) as u32;

        let mut via_all: StatusSet<u32> = StatusSet::new();
        let mut via_loop: StatusSet<u32> = StatusSet::new();
        for &(k, rem, mag) in &seed {
            via_all.apply(k, rem, mag);
            via_loop.apply(k, rem, mag);
        }
        via_all.extend_all(extra);
        let keys: Vec<u32> = via_loop.active_keys().into_iter().copied().collect();
        for k in keys {
            via_loop.extend_duration(&k, extra);
        }
        assert_eq!(
            hash_state(&via_all),
            hash_state(&via_loop),
            "extend_all diverged from a loop of extend_duration"
        );
    }
}

/// Observable allocator state is identical whether entities are allocated via
/// `batch_alloc(n)` or `n` individual `allocate()` calls.
#[test]
fn entity_batch_alloc_equals_loop_of_allocate() {
    let mut rng = SplitMix64::new(0x_A110C_03);
    for _ in 0..TRIALS {
        let n = rng.range(0, 40) as usize;

        let mut via_batch = EntityAllocator::new();
        let batch = via_batch.batch_alloc(n);

        let mut via_loop = EntityAllocator::new();
        let loop_es: Vec<_> = (0..n).map(|_| via_loop.allocate()).collect();

        assert_eq!(batch, loop_es, "batch_alloc handed out different entities");
        assert_eq!(via_batch.count(), via_loop.count());
        assert_eq!(via_batch.total_slots(), via_loop.total_slots());
        assert_eq!(via_batch.live_entities(), via_loop.live_entities());
    }
}

/// `batch_free(&es)` must equal freeing each entity individually — identical
/// liveness, counts, and generation bumps afterward.
#[test]
fn entity_batch_free_equals_loop_of_free() {
    let mut rng = SplitMix64::new(0x_F4EE_04);
    for _ in 0..TRIALS {
        let n = rng.range(1, 40) as usize;

        // Two allocators driven through the identical allocate sequence.
        let mut via_batch = EntityAllocator::new();
        let mut via_loop = EntityAllocator::new();
        let es_b: Vec<_> = (0..n).map(|_| via_batch.allocate()).collect();
        let es_l: Vec<_> = (0..n).map(|_| via_loop.allocate()).collect();
        assert_eq!(es_b, es_l, "precondition: identical allocate sequences");

        // Choose a random subset (with possible duplicates / stale repeats) to free.
        let to_free: Vec<_> = (0..rng.range(0, n as i32 + 5))
            .filter_map(|_| {
                let i = rng.below(n as u32) as usize;
                es_b.get(i).copied()
            })
            .collect();

        via_batch.batch_free(&to_free);
        for &e in &to_free {
            via_loop.free(e);
        }

        assert_eq!(via_batch.count(), via_loop.count(), "count diverged");
        assert_eq!(via_batch.free_count(), via_loop.free_count(), "free_count diverged");
        assert_eq!(
            via_batch.live_entities(),
            via_loop.live_entities(),
            "live set diverged"
        );
        // Re-allocation must also proceed identically (same recycled slots/gens).
        assert_eq!(via_batch.allocate(), via_loop.allocate(), "next allocate diverged");
    }
}
