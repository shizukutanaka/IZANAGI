//! Order-independence (confluence) test perspective.
//!
//! The kit's headline promise is a *canonical* state hash: the same logical
//! state must hash identically no matter the order the operations that built it
//! were applied. That is exactly what lets replay/lockstep checks stay robust
//! when peers receive the same events in different orders. Several modules sort
//! their entries inside `det_hash` specifically to guarantee this, and each
//! ships a single two-element "apply order doesn't matter" example test.
//!
//! This lens stress-tests the guarantee broadly: it builds each structure from
//! the *same set* of operations under many random permutations and asserts every
//! permutation yields the identical `hash_state`. A module whose `det_hash`
//! forgot to canonicalize a collection — or sorted it unstably — diverges here
//! even though its own example test passes. This is orthogonal to the other
//! lenses: it tests the determinism mandate itself, not a value, law, model,
//! oracle, or symmetry.
//!
//! Operations use *distinct* keys/entities so the final logical state is
//! genuinely permutation-independent (e.g. `StatusSet::apply` deliberately has
//! last-writer-wins magnitude on a repeated key, which is order-*dependent*).
//! Deterministic via `SplitMix64`.

use izanagi_kit::{
    hash_state, EntityAllocator, Fnv1a, Relations, SparseSet, SpatialHash, SplitMix64, StatusSet,
};

const TRIALS: usize = 300;

/// `SparseSet` exposes `det_hash` as an inherent method (not a `DetHash` impl),
/// so fold it through a fresh hasher rather than `hash_state`.
fn sparse_set_hash(s: &SparseSet<u32>) -> u64 {
    let mut h = Fnv1a::new();
    s.det_hash(&mut h);
    h.finish()
}

/// Shuffle `0..n` into a fresh permutation.
fn permutation(rng: &mut SplitMix64, n: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..n).collect();
    rng.shuffle(&mut order);
    order
}

#[test]
fn sparse_set_hash_is_insertion_order_independent() {
    let mut rng = SplitMix64::new(0x_5A95_E7_01);
    let mut alloc = EntityAllocator::new();
    let items: Vec<(_, u32)> = (0..32).map(|i| (alloc.allocate(), i * 7 + 1)).collect();

    let reference = {
        let mut s: SparseSet<u32> = SparseSet::new();
        for &(e, v) in &items {
            s.insert(e, v);
        }
        sparse_set_hash(&s)
    };

    for _ in 0..TRIALS {
        let order = permutation(&mut rng, items.len());
        let mut s: SparseSet<u32> = SparseSet::new();
        for &i in &order {
            let (e, v) = items[i];
            s.insert(e, v);
        }
        assert_eq!(sparse_set_hash(&s), reference, "SparseSet hash depends on insertion order");
    }
}

#[test]
fn status_set_hash_is_apply_order_independent() {
    // Distinct keys, so each apply is independent (no last-writer-wins overlap).
    let mut rng = SplitMix64::new(0x_57A7_05_02);
    let items: Vec<(u32, u32, i32)> = (0..24)
        .map(|i| (i, (i % 9) + 1, (i as i32) - 12))
        .collect();

    let reference = {
        let mut s: StatusSet<u32> = StatusSet::new();
        for &(k, dur, mag) in &items {
            s.apply(k, dur, mag);
        }
        hash_state(&s)
    };

    for _ in 0..TRIALS {
        let order = permutation(&mut rng, items.len());
        let mut s: StatusSet<u32> = StatusSet::new();
        for &i in &order {
            let (k, dur, mag) = items[i];
            s.apply(k, dur, mag);
        }
        assert_eq!(hash_state(&s), reference, "StatusSet hash depends on apply order");
    }
}

#[test]
fn spatial_hash_hash_is_insertion_order_independent() {
    // Random positions deliberately collide in cells, exercising the per-bucket
    // canonicalization (buckets are sorted inside det_hash).
    let mut rng = SplitMix64::new(0x_5A_71_A1_03);
    let items: Vec<(u32, i32, i32)> = (0..40)
        .map(|k| (k, rng.range(0, 20), rng.range(0, 20)))
        .collect();

    let reference = {
        let mut g: SpatialHash<u32> = SpatialHash::new(4);
        for &(k, x, y) in &items {
            g.insert(k, x, y);
        }
        hash_state(&g)
    };

    for _ in 0..TRIALS {
        let order = permutation(&mut rng, items.len());
        let mut g: SpatialHash<u32> = SpatialHash::new(4);
        for &i in &order {
            let (k, x, y) = items[i];
            g.insert(k, x, y);
        }
        assert_eq!(hash_state(&g), reference, "SpatialHash hash depends on insertion order");
    }
}

#[test]
fn relations_hash_is_attach_order_independent() {
    // A forest of distinct children attached to a small set of shared parents.
    let mut rng = SplitMix64::new(0x_4E_1A_04);
    let mut alloc = EntityAllocator::new();
    let parents: Vec<_> = (0..4).map(|_| alloc.allocate()).collect();
    let edges: Vec<(_, _)> = (0..28)
        .map(|i| (alloc.allocate(), parents[i % parents.len()]))
        .collect();

    let reference = {
        let mut r = Relations::new();
        for &(child, parent) in &edges {
            r.attach(child, parent);
        }
        hash_state(&r)
    };

    for _ in 0..TRIALS {
        let order = permutation(&mut rng, edges.len());
        let mut r = Relations::new();
        for &i in &order {
            let (child, parent) = edges[i];
            r.attach(child, parent);
        }
        assert_eq!(hash_state(&r), reference, "Relations hash depends on attach order");
    }
}
