//! Sparse-set component storage.
//!
//! Per the arxiv comparison (Cox et al., CGVC 2025): sparse-set storage makes
//! entity composition changes cheap while keeping lookup O(1); archetype
//! storage wins on large-scale iteration but pays on composition churn. For a
//! terminal engine with frequent spawn/despawn and mid-scale entity counts,
//! sparse-set is the right v1 default. Promote to archetype only after
//! profiling shows iteration dominates (measure-first; do not pre-optimise).
//!
//! Layout: `dense` holds packed `(Entity, T)` for cache-friendly iteration;
//! `sparse[index] -> position in dense`. Removal is swap-remove (O(1)).

use crate::entity::Entity;

/// Dense component pool keyed by entity index.
pub struct SparseSet<T> {
    sparse: Vec<Option<u32>>,
    dense_entities: Vec<Entity>,
    dense_values: Vec<T>,
}

impl<T> Default for SparseSet<T> {
    fn default() -> Self {
        Self {
            sparse: Vec::new(),
            dense_entities: Vec::new(),
            dense_values: Vec::new(),
        }
    }
}

impl<T> SparseSet<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.dense_values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dense_values.is_empty()
    }

    #[inline]
    fn slot(&self, entity: Entity) -> Option<u32> {
        self.sparse.get(entity.index() as usize).copied().flatten()
    }

    /// Inserts or overwrites the component for `entity`. O(1) amortised.
    pub fn insert(&mut self, entity: Entity, value: T) {
        let idx = entity.index() as usize;
        if idx >= self.sparse.len() {
            self.sparse.resize(idx + 1, None);
        }
        if let Some(pos) = self.sparse[idx] {
            self.dense_values[pos as usize] = value;
            self.dense_entities[pos as usize] = entity;
            return;
        }
        self.sparse[idx] = Some(self.dense_values.len() as u32);
        self.dense_entities.push(entity);
        self.dense_values.push(value);
    }

    #[inline]
    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.slot(entity).map(|p| &self.dense_values[p as usize])
    }

    #[inline]
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        match self.slot(entity) {
            Some(p) => Some(&mut self.dense_values[p as usize]),
            None => None,
        }
    }

    pub fn contains(&self, entity: Entity) -> bool {
        self.slot(entity).is_some()
    }

    /// Remove all entries for which `pred(entity, &value)` returns `false`.
    /// Useful for bulk-removing dead entities at the end of a frame. O(n).
    /// Remove all entries for which `pred(entity, &value)` returns `true`,
    /// returning the count removed. Semantically the inverse of `retain` —
    /// use when the natural phrasing is "remove entities that satisfy X"
    /// rather than "keep entities that do NOT satisfy X".
    pub fn remove_where<F: Fn(Entity, &T) -> bool>(&mut self, pred: F) -> usize {
        let mut removed = 0usize;
        let mut i = 0;
        while i < self.dense_entities.len() {
            let entity = self.dense_entities[i];
            if pred(entity, &self.dense_values[i]) {
                self.remove(entity);
                removed += 1;
            } else {
                i += 1;
            }
        }
        removed
    }

    /// Each surviving entry is visited once; each removed entry pays one
    /// swap-remove (the last element moves into the vacated slot, so after a
    /// removal the same index is re-examined without advancing).
    pub fn retain<F: Fn(Entity, &T) -> bool>(&mut self, pred: F) {
        let mut i = 0;
        while i < self.dense_entities.len() {
            let entity = self.dense_entities[i];
            if !pred(entity, &self.dense_values[i]) {
                self.remove(entity);
            } else {
                i += 1;
            }
        }
    }

    /// Removes via swap-remove, returning the value. O(1).
    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        let pos = self.slot(entity)? as usize;
        let last = self.dense_values.len() - 1;
        self.dense_entities.swap(pos, last);
        self.dense_values.swap(pos, last);
        let moved = self.dense_entities[pos];
        self.sparse[moved.index() as usize] = Some(pos as u32);
        self.sparse[entity.index() as usize] = None;
        self.dense_entities.pop();
        self.dense_values.pop()
    }

    /// Remove all entries. The sparse index is also cleared so no stale slots
    /// remain. Equivalent to `retain(|_,_| false)` but avoids per-element
    /// overhead.
    pub fn clear(&mut self) {
        for e in &self.dense_entities {
            if let Some(slot) = self.sparse.get_mut(e.index() as usize) {
                *slot = None;
            }
        }
        self.dense_entities.clear();
        self.dense_values.clear();
    }

    /// Iterate entity handles only, in dense (insertion) order.
    pub fn entities(&self) -> impl Iterator<Item = Entity> + '_ {
        self.dense_entities.iter().copied()
    }

    /// Iterate component values only, in dense (insertion) order.
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.dense_values.iter()
    }

    /// Iterate mutable component values only, in dense (insertion) order.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.dense_values.iter_mut()
    }

    /// Dense iteration (fast). Order follows insert/swap history — stable for a
    /// fixed op sequence, but not canonical. Use `iter_sorted` when a
    /// deterministic-across-content order is required.
    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.dense_entities
            .iter()
            .copied()
            .zip(self.dense_values.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        self.dense_entities
            .iter()
            .copied()
            .zip(self.dense_values.iter_mut())
    }

    /// Canonical iteration order: ascending entity index. Removes iteration
    /// order as a source of non-determinism (arxiv: stable iteration order).
    pub fn iter_sorted(&self) -> Vec<(Entity, &T)> {
        let mut pairs: Vec<(Entity, &T)> = self.iter().collect();
        pairs.sort_unstable_by_key(|(e, _)| e.index());
        pairs
    }

    /// Exchange the components of `a` and `b` in place. Returns `true` when both
    /// entities are present; returns `false` (no change) if either is absent.
    /// O(1) — both dense positions are looked up via the sparse index and the
    /// values are swapped without searching. Useful for trading inventory items
    /// or applying a permutation to a component pool.
    pub fn swap(&mut self, a: Entity, b: Entity) -> bool {
        let pa = match self.slot(a) {
            Some(p) => p as usize,
            None => return false,
        };
        let pb = match self.slot(b) {
            Some(p) => p as usize,
            None => return false,
        };
        if pa != pb {
            self.dense_values.swap(pa, pb);
        }
        true
    }

    /// Count entries for which `pred` returns `true`. Non-allocating alternative
    /// to `.iter().filter(|(_,v)| pred(v)).count()`.
    pub fn count_matching<F: Fn(&T) -> bool>(&self, pred: F) -> usize {
        self.values().filter(|v| pred(v)).count()
    }

    /// Return the first entity whose component satisfies `pred`, or `None` if
    /// no component matches. Scans in dense (insertion) order, so when several
    /// match the result depends on insert history; use [`iter_sorted`](Self::iter_sorted)
    /// at the call site if a canonical pick is required. The component-query
    /// complement to `count_matching`.
    pub fn find_entity_where<F: Fn(&T) -> bool>(&self, pred: F) -> Option<Entity> {
        self.iter()
            .find_map(|(entity, value)| if pred(value) { Some(entity) } else { None })
    }
}

impl<T: crate::world_hash::DetHash> SparseSet<T> {
    /// Folds the set into a hasher in **canonical order** (ascending entity
    /// index), independent of insert/swap history (invariant G6). The length is
    /// folded first so `{}` and a set whose first element hashes to the offset
    /// basis cannot collide. Pairs each entity handle with its component value.
    pub fn det_hash(&self, hasher: &mut crate::world_hash::Fnv1a) {
        use crate::world_hash::DetHash;
        hasher.write_u32(self.len() as u32);
        for (entity, value) in self.iter_sorted() {
            entity.det_hash(hasher);
            value.det_hash(hasher);
        }
    }
}

/// Canonical inner join of two component stores: every entity present in
/// **both**, in ascending-index order, paired with a reference to each
/// component. This is the basic multi-component query — e.g. "every entity with
/// a `Position` *and* a `Render`".
///
/// The smaller store is iterated and the other probed (O(min) lookups); the
/// result is sorted by entity index so iteration order is canonical and
/// deterministic regardless of either store's insert history (invariant G6).
pub fn join<'a, A, B>(a: &'a SparseSet<A>, b: &'a SparseSet<B>) -> Vec<(Entity, &'a A, &'a B)> {
    let mut out = Vec::new();
    if a.len() <= b.len() {
        for (entity, av) in a.iter() {
            if let Some(bv) = b.get(entity) {
                out.push((entity, av, bv));
            }
        }
    } else {
        for (entity, bv) in b.iter() {
            if let Some(av) = a.get(entity) {
                out.push((entity, av, bv));
            }
        }
    }
    out.sort_unstable_by_key(|(e, _, _)| e.index());
    out
}

/// Like [`join`], but yields a mutable reference to the first store's component
/// (and a shared reference to the second) — for systems that update `A` using
/// `B` (e.g. advance `Position` by `Velocity`). Canonical ascending-index order.
pub fn join_mut<'a, A, B>(
    a: &'a mut SparseSet<A>,
    b: &'a SparseSet<B>,
) -> Vec<(Entity, &'a mut A, &'a B)> {
    let mut out = Vec::new();
    for (entity, av) in a.iter_mut() {
        if let Some(bv) = b.get(entity) {
            out.push((entity, av, bv));
        }
    }
    out.sort_unstable_by_key(|(e, _, _)| e.index());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityAllocator;

    fn three() -> (EntityAllocator, [Entity; 3]) {
        let mut a = EntityAllocator::new();
        let es = [a.allocate(), a.allocate(), a.allocate()];
        (a, es)
    }

    #[test]
    fn test_insert_then_get_returns_value() {
        let (_, es) = three();
        let mut s = SparseSet::new();
        s.insert(es[1], 42u32);
        assert_eq!(s.get(es[1]), Some(&42));
        assert_eq!(s.get(es[0]), None);
    }

    #[test]
    fn test_insert_existing_overwrites_not_duplicates() {
        let (_, es) = three();
        let mut s = SparseSet::new();
        s.insert(es[0], 1u32);
        s.insert(es[0], 2u32);
        assert_eq!(s.len(), 1);
        assert_eq!(s.get(es[0]), Some(&2));
    }

    #[test]
    fn test_remove_middle_keeps_others_via_swap() {
        let (_, es) = three();
        let mut s = SparseSet::new();
        for (i, &e) in es.iter().enumerate() {
            s.insert(e, i as u32);
        }
        assert_eq!(s.remove(es[0]), Some(0));
        assert_eq!(s.len(), 2);
        assert_eq!(s.get(es[1]), Some(&1));
        assert_eq!(s.get(es[2]), Some(&2));
        assert_eq!(s.get(es[0]), None);
    }

    #[test]
    fn test_iter_sorted_is_ascending_regardless_of_history() {
        let (_, es) = three();
        let mut s = SparseSet::new();
        s.insert(es[2], 20u32);
        s.insert(es[0], 0u32);
        s.insert(es[1], 10u32);
        let order: Vec<u32> = s.iter_sorted().iter().map(|(e, _)| e.index()).collect();
        assert_eq!(order, vec![0, 1, 2]);
    }

    #[test]
    fn test_det_hash_ignores_insert_order_but_tracks_values() {
        use crate::world_hash::Fnv1a;
        let (_, es) = three();

        let hash_of = |pairs: &[(Entity, u32)]| {
            let mut s = SparseSet::new();
            for &(e, v) in pairs {
                s.insert(e, v);
            }
            let mut h = Fnv1a::new();
            s.det_hash(&mut h);
            h.finish()
        };

        // Same contents, different insert order → identical canonical hash.
        let forward = hash_of(&[(es[0], 0), (es[1], 10), (es[2], 20)]);
        let shuffled = hash_of(&[(es[2], 20), (es[0], 0), (es[1], 10)]);
        assert_eq!(forward, shuffled, "canonical hash must ignore insert order");

        // A changed value must change the hash.
        let changed = hash_of(&[(es[0], 0), (es[1], 10), (es[2], 21)]);
        assert_ne!(forward, changed, "value change must be observable");

        // A different population (length) must change the hash.
        let shorter = hash_of(&[(es[0], 0), (es[1], 10)]);
        assert_ne!(forward, shorter, "length must be folded in");
    }

    #[test]
    fn test_join_returns_only_entities_in_both_canonically() {
        let mut alloc = EntityAllocator::new();
        let es: Vec<Entity> = (0..4).map(|_| alloc.allocate()).collect();
        let mut pos: SparseSet<u32> = SparseSet::new();
        let mut vel: SparseSet<i32> = SparseSet::new();
        // Insert in scrambled order; only es[1] and es[3] are in both.
        pos.insert(es[3], 30);
        pos.insert(es[0], 0);
        pos.insert(es[1], 10);
        vel.insert(es[1], -1);
        vel.insert(es[3], -3);
        vel.insert(es[2], -2); // not in pos

        let joined = join(&pos, &vel);
        let ids: Vec<u32> = joined.iter().map(|(e, _, _)| e.index()).collect();
        assert_eq!(ids, vec![1, 3], "join is the intersection, ascending index");
        assert_eq!(joined[0].1, &10);
        assert_eq!(joined[0].2, &-1);
    }

    #[test]
    fn test_join_order_independent_of_which_store_is_smaller() {
        let mut alloc = EntityAllocator::new();
        let es: Vec<Entity> = (0..3).map(|_| alloc.allocate()).collect();
        let mut a: SparseSet<u32> = SparseSet::new();
        let mut b: SparseSet<u32> = SparseSet::new();
        for &e in &es {
            a.insert(e, e.index());
        }
        // b is the smaller store (drives the other branch of `join`).
        b.insert(es[2], 2);
        b.insert(es[0], 0);
        let ids: Vec<u32> = join(&a, &b).iter().map(|(e, _, _)| e.index()).collect();
        let ids_rev: Vec<u32> = join(&b, &a).iter().map(|(e, _, _)| e.index()).collect();
        assert_eq!(ids, vec![0, 2]);
        assert_eq!(ids, ids_rev, "join order must not depend on argument order");
    }

    #[test]
    fn test_retain_keeps_matching() {
        let (_, es) = three();
        let mut s: SparseSet<u32> = SparseSet::new();
        s.insert(es[0], 1);
        s.insert(es[1], 2);
        s.insert(es[2], 3);
        // Keep only even values.
        s.retain(|_, v| v % 2 == 0);
        assert_eq!(s.len(), 1);
        assert_eq!(s.get(es[1]), Some(&2));
        assert_eq!(s.get(es[0]), None);
        assert_eq!(s.get(es[2]), None);
    }

    #[test]
    fn test_retain_false_clears_all() {
        let (_, es) = three();
        let mut s: SparseSet<u32> = SparseSet::new();
        for (i, &e) in es.iter().enumerate() {
            s.insert(e, i as u32);
        }
        s.retain(|_, _| false);
        assert!(s.is_empty());
    }

    #[test]
    fn test_retain_true_keeps_all() {
        let (_, es) = three();
        let mut s: SparseSet<u32> = SparseSet::new();
        for (i, &e) in es.iter().enumerate() {
            s.insert(e, i as u32);
        }
        s.retain(|_, _| true);
        assert_eq!(s.len(), 3);
        // All original values survive.
        for (i, &e) in es.iter().enumerate() {
            assert_eq!(s.get(e), Some(&(i as u32)));
        }
    }

    #[test]
    fn test_clear_removes_all_entries() {
        let mut alloc = EntityAllocator::new();
        let mut s: SparseSet<i32> = SparseSet::new();
        let e0 = alloc.allocate();
        let e1 = alloc.allocate();
        s.insert(e0, 1);
        s.insert(e1, 2);
        s.clear();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
        assert_eq!(s.get(e0), None);
        assert_eq!(s.get(e1), None);
    }

    #[test]
    fn test_clear_then_reinsert() {
        let mut alloc = EntityAllocator::new();
        let mut s: SparseSet<i32> = SparseSet::new();
        let e = alloc.allocate();
        s.insert(e, 10);
        s.clear();
        s.insert(e, 99);
        assert_eq!(s.get(e), Some(&99));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_entities_iterator() {
        let mut alloc = EntityAllocator::new();
        let mut s: SparseSet<i32> = SparseSet::new();
        let e0 = alloc.allocate();
        let e1 = alloc.allocate();
        s.insert(e0, 1);
        s.insert(e1, 2);
        let ents: Vec<Entity> = s.entities().collect();
        assert!(ents.contains(&e0));
        assert!(ents.contains(&e1));
        assert_eq!(ents.len(), 2);
    }

    #[test]
    fn test_values_iterator() {
        let mut alloc = EntityAllocator::new();
        let mut s: SparseSet<i32> = SparseSet::new();
        let e0 = alloc.allocate();
        let e1 = alloc.allocate();
        s.insert(e0, 10);
        s.insert(e1, 20);
        let vals: Vec<i32> = s.values().copied().collect();
        assert!(vals.contains(&10));
        assert!(vals.contains(&20));
        assert_eq!(vals.len(), 2);
    }

    #[test]
    fn test_values_mut_modifies_components() {
        let mut alloc = EntityAllocator::new();
        let mut s: SparseSet<i32> = SparseSet::new();
        let e0 = alloc.allocate();
        let e1 = alloc.allocate();
        s.insert(e0, 3);
        s.insert(e1, 5);
        for v in s.values_mut() {
            *v *= 2;
        }
        assert_eq!(s.get(e0), Some(&6));
        assert_eq!(s.get(e1), Some(&10));
    }

    #[test]
    fn test_join_mut_can_update_a_using_b() {
        let mut alloc = EntityAllocator::new();
        let es: Vec<Entity> = (0..3).map(|_| alloc.allocate()).collect();
        let mut pos: SparseSet<i32> = SparseSet::new();
        let mut vel: SparseSet<i32> = SparseSet::new();
        for (i, &e) in es.iter().enumerate() {
            pos.insert(e, (i as i32 + 1) * 100);
        }
        vel.insert(es[0], 5);
        vel.insert(es[2], 7); // es[1] has no velocity

        for (_, p, v) in join_mut(&mut pos, &vel) {
            *p += *v;
        }
        assert_eq!(pos.get(es[0]), Some(&105));
        assert_eq!(pos.get(es[1]), Some(&200), "no velocity → unchanged");
        assert_eq!(pos.get(es[2]), Some(&307));
    }

    #[test]
    fn test_count_matching_zero_on_empty() {
        let s: SparseSet<i32> = SparseSet::new();
        assert_eq!(s.count_matching(|_| true), 0);
    }

    #[test]
    fn test_count_matching_all() {
        let (_, es) = three();
        let mut s: SparseSet<i32> = SparseSet::new();
        s.insert(es[0], 1);
        s.insert(es[1], 2);
        s.insert(es[2], 3);
        assert_eq!(s.count_matching(|_| true), 3);
    }

    #[test]
    fn test_count_matching_none() {
        let (_, es) = three();
        let mut s: SparseSet<i32> = SparseSet::new();
        s.insert(es[0], 1);
        s.insert(es[1], 3);
        assert_eq!(s.count_matching(|v| v % 2 == 0), 0);
    }

    #[test]
    fn test_count_matching_partial() {
        let (_, es) = three();
        let mut s: SparseSet<i32> = SparseSet::new();
        s.insert(es[0], 10);
        s.insert(es[1], 7);
        s.insert(es[2], 4);
        assert_eq!(s.count_matching(|v| *v > 5), 2);
    }

    #[test]
    fn test_find_entity_where_returns_matching_entity() {
        let (_, es) = three();
        let mut s: SparseSet<i32> = SparseSet::new();
        s.insert(es[0], 10);
        s.insert(es[1], 20);
        s.insert(es[2], 30);
        assert_eq!(s.find_entity_where(|v| *v == 20), Some(es[1]));
    }

    #[test]
    fn test_find_entity_where_no_match_returns_none() {
        let (_, es) = three();
        let mut s: SparseSet<i32> = SparseSet::new();
        s.insert(es[0], 1);
        s.insert(es[1], 2);
        assert_eq!(s.find_entity_where(|v| *v > 100), None);
    }

    #[test]
    fn test_find_entity_where_empty_returns_none() {
        let s: SparseSet<i32> = SparseSet::new();
        assert_eq!(s.find_entity_where(|_| true), None);
    }

    #[test]
    fn test_remove_where_returns_count_removed() {
        let (_, es) = three();
        let mut s: SparseSet<i32> = SparseSet::new();
        s.insert(es[0], 1);
        s.insert(es[1], 2);
        s.insert(es[2], 3);
        let removed = s.remove_where(|_, v| *v > 1);
        assert_eq!(removed, 2);
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_remove_where_no_match_returns_zero() {
        let (_, es) = three();
        let mut s: SparseSet<i32> = SparseSet::new();
        s.insert(es[0], 1);
        let removed = s.remove_where(|_, v| *v > 100);
        assert_eq!(removed, 0);
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_remove_where_all_match_clears_set() {
        let (_, es) = three();
        let mut s: SparseSet<i32> = SparseSet::new();
        s.insert(es[0], 1);
        s.insert(es[1], 2);
        let removed = s.remove_where(|_, _| true);
        assert_eq!(removed, 2);
        assert!(s.is_empty());
    }

    #[test]
    fn test_swap_exchanges_component_values() {
        let (_, es) = three();
        let mut s: SparseSet<u32> = SparseSet::new();
        s.insert(es[0], 10);
        s.insert(es[1], 20);
        assert!(s.swap(es[0], es[1]));
        assert_eq!(s.get(es[0]), Some(&20));
        assert_eq!(s.get(es[1]), Some(&10));
    }

    #[test]
    fn test_swap_returns_false_when_either_absent() {
        let (_, es) = three();
        let mut s: SparseSet<u32> = SparseSet::new();
        s.insert(es[0], 1);
        assert!(!s.swap(es[0], es[1]), "es[1] absent → false");
        assert!(!s.swap(es[2], es[1]), "both absent → false");
        assert_eq!(s.get(es[0]), Some(&1), "no mutation on failure");
    }

    #[test]
    fn test_swap_same_entity_is_noop_and_returns_true() {
        let (_, es) = three();
        let mut s: SparseSet<u32> = SparseSet::new();
        s.insert(es[0], 42);
        assert!(s.swap(es[0], es[0]));
        assert_eq!(s.get(es[0]), Some(&42));
    }
}
