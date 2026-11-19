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
//!
//! Marker (tag) components — types that carry no data, where only presence
//! matters — use [`TagSet`] instead: the same sparse/dense index pair with no
//! per-element value storage at all.

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
    /// Create an empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of components currently stored.
    pub fn len(&self) -> usize {
        self.dense_values.len()
    }

    /// `true` if no components are stored.
    pub fn is_empty(&self) -> bool {
        self.dense_values.is_empty()
    }

    #[inline]
    fn slot(&self, entity: Entity) -> Option<u32> {
        let pos = self
            .sparse
            .get(entity.index() as usize)
            .copied()
            .flatten()?;
        // Generational safety: the dense slot is keyed by index, but a freed
        // index can be recycled at a bumped generation. Reject a handle whose
        // generation no longer matches the entity actually occupying the slot,
        // so a stale handle can never alias (read/mutate/remove) a recycled
        // slot's component. `pos` always indexes a valid dense entry.
        if self.dense_entities[pos as usize] == entity {
            Some(pos)
        } else {
            None
        }
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

    /// Look up the component for `entity`, if it has one.
    #[inline]
    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.slot(entity).map(|p| &self.dense_values[p as usize])
    }

    /// Mutably look up the component for `entity`, if it has one.
    #[inline]
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        match self.slot(entity) {
            Some(p) => Some(&mut self.dense_values[p as usize]),
            None => None,
        }
    }

    /// `true` if `entity` has a component in this set.
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

    /// Remove all entries and return them as a `Vec<(Entity, T)>` in dense
    /// (insertion) order. Equivalent to collecting `iter()` and then calling
    /// `clear()`, but avoids cloning by taking ownership of the values.
    /// Useful for batch-despawn after a frame: drain all dead entities and
    /// process their components without a separate collect+remove loop.
    pub fn drain(&mut self) -> Vec<(Entity, T)> {
        for e in &self.dense_entities {
            if let Some(slot) = self.sparse.get_mut(e.index() as usize) {
                *slot = None;
            }
        }
        let entities = core::mem::take(&mut self.dense_entities);
        let values = core::mem::take(&mut self.dense_values);
        entities.into_iter().zip(values).collect()
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

    /// Mutable dense iteration. Same ordering caveat as [`Self::iter`].
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

    /// Return all entities whose component satisfies `pred`, in dense order.
    /// Use [`iter_sorted`](Self::iter_sorted) if a canonical ascending-index
    /// order is required.
    pub fn find_all_entities<F: Fn(&T) -> bool>(&self, pred: F) -> Vec<Entity> {
        self.iter()
            .filter_map(|(e, v)| if pred(v) { Some(e) } else { None })
            .collect()
    }

    /// Remove every entry for which `pred(entity, &value)` is `true`, returning
    /// the removed `(entity, value)` pairs in removal order. Like
    /// [`remove_where`](Self::remove_where) but the caller receives the values
    /// — use for "cull dead entities and trigger despawn callbacks with their
    /// components."
    pub fn remove_where_returning<F: Fn(Entity, &T) -> bool>(
        &mut self,
        pred: F,
    ) -> Vec<(Entity, T)> {
        let to_remove: Vec<Entity> = self
            .iter()
            .filter_map(|(e, v)| if pred(e, v) { Some(e) } else { None })
            .collect();
        to_remove
            .into_iter()
            .filter_map(|e| self.remove(e).map(|v| (e, v)))
            .collect()
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

    /// Returns `true` if any component satisfies `pred`. Short-circuits on the
    /// first match. Non-allocating — prefer this over `count_matching > 0` in
    /// "does anyone have a status effect?" or "is any inventory slot empty?" checks.
    pub fn any<F: Fn(&T) -> bool>(&self, pred: F) -> bool {
        self.values().any(pred)
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

/// Presence-only component storage for marker (tag) components.
///
/// A tag component carries no data — the component's *type* is the whole
/// payload, and the only question a system asks is "does this entity have
/// it?" (examples: `Frozen`, `PlayerControlled`, `PendingDespawn`). Storing
/// such a component in a [`SparseSet`] would still allocate and maintain a
/// dense value array whose elements can never differ, so `TagSet` keeps only
/// the sparse index and the dense entity list: membership is the storage.
///
/// The semantics mirror [`SparseSet`] exactly — O(1) insert / lookup /
/// swap-remove, the same generational safety (a stale handle never aliases a
/// recycled index), the same canonical ascending-index order for hashing —
/// so swapping a marker's store from `SparseSet<Marker>` to `TagSet` changes
/// no observable behaviour, including the world hash: for the same entity
/// population, [`TagSet::det_hash`] folds exactly what
/// `SparseSet<Marker>::det_hash` folds for a marker whose own hash contribution
/// is empty.
#[derive(Default)]
pub struct TagSet {
    sparse: Vec<Option<u32>>,
    dense_entities: Vec<Entity>,
}

impl TagSet {
    /// Create an empty set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of entities currently tagged.
    pub fn len(&self) -> usize {
        self.dense_entities.len()
    }

    /// `true` if no entity is tagged.
    pub fn is_empty(&self) -> bool {
        self.dense_entities.is_empty()
    }

    #[inline]
    fn slot(&self, entity: Entity) -> Option<u32> {
        let pos = self
            .sparse
            .get(entity.index() as usize)
            .copied()
            .flatten()?;
        // Generational safety, identical to `SparseSet`: reject a handle
        // whose generation no longer matches the entity occupying the slot,
        // so a stale handle can never alias a recycled index.
        if self.dense_entities[pos as usize] == entity {
            Some(pos)
        } else {
            None
        }
    }

    /// Tag `entity`. Returns `true` if the tag was newly added, `false` if
    /// this exact handle was already tagged. O(1) amortised.
    pub fn insert(&mut self, entity: Entity) -> bool {
        let idx = entity.index() as usize;
        if idx >= self.sparse.len() {
            self.sparse.resize(idx + 1, None);
        }
        if let Some(pos) = self.sparse[idx] {
            // The index is already occupied: refresh the stored handle (the
            // previous occupant may be a stale generation) and report whether
            // this exact handle is new to the set.
            let was_present = self.dense_entities[pos as usize] == entity;
            self.dense_entities[pos as usize] = entity;
            return !was_present;
        }
        self.sparse[idx] = Some(self.dense_entities.len() as u32);
        self.dense_entities.push(entity);
        true
    }

    /// `true` if `entity` is tagged. A stale handle (generation mismatch)
    /// reports `false`, exactly as [`SparseSet::contains`].
    pub fn contains(&self, entity: Entity) -> bool {
        self.slot(entity).is_some()
    }

    /// Remove the tag from `entity` via swap-remove, returning `true` if it
    /// was present. O(1).
    pub fn remove(&mut self, entity: Entity) -> bool {
        let pos = match self.slot(entity) {
            Some(p) => p as usize,
            None => return false,
        };
        let last = self.dense_entities.len() - 1;
        self.dense_entities.swap(pos, last);
        let moved = self.dense_entities[pos];
        self.sparse[moved.index() as usize] = Some(pos as u32);
        self.sparse[entity.index() as usize] = None;
        self.dense_entities.pop();
        true
    }

    /// Untag every entity for which `pred(entity)` returns `false`. O(n).
    pub fn retain<F: Fn(Entity) -> bool>(&mut self, pred: F) {
        let mut i = 0;
        while i < self.dense_entities.len() {
            let entity = self.dense_entities[i];
            if !pred(entity) {
                self.remove(entity);
            } else {
                i += 1;
            }
        }
    }

    /// Remove all tags. The sparse index is also cleared so no stale slots
    /// remain.
    pub fn clear(&mut self) {
        for e in &self.dense_entities {
            if let Some(slot) = self.sparse.get_mut(e.index() as usize) {
                *slot = None;
            }
        }
        self.dense_entities.clear();
    }

    /// Remove all tags and return the tagged entities in dense (insertion)
    /// order.
    pub fn drain(&mut self) -> Vec<Entity> {
        for e in &self.dense_entities {
            if let Some(slot) = self.sparse.get_mut(e.index() as usize) {
                *slot = None;
            }
        }
        core::mem::take(&mut self.dense_entities)
    }

    /// Iterate tagged entities in dense (insertion) order.
    pub fn entities(&self) -> impl Iterator<Item = Entity> + '_ {
        self.dense_entities.iter().copied()
    }

    /// Tagged entities in canonical order: ascending entity index. Removes
    /// iteration order as a source of non-determinism.
    pub fn entities_sorted(&self) -> Vec<Entity> {
        let mut out = self.dense_entities.clone();
        out.sort_unstable_by_key(|e| e.index());
        out
    }

    /// Folds the set into a hasher in **canonical order** (ascending entity
    /// index), independent of insert/swap history (invariant G6). The length
    /// is folded first, mirroring [`SparseSet::det_hash`]; a tag has no value
    /// to fold, so each entry contributes its entity handle only.
    pub fn det_hash(&self, hasher: &mut crate::world_hash::Fnv1a) {
        use crate::world_hash::DetHash;
        hasher.write_u32(self.len() as u32);
        for entity in self.entities_sorted() {
            entity.det_hash(hasher);
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

/// Canonical inner join of **three** component stores: every entity present in
/// all three, in ascending-index order, paired with references to each component
/// (W1 in `STRENGTHS_WEAKNESSES.md`).
///
/// Iterates the smallest store and probes the other two — O(min · log(max))
/// with the existing `get` O(1) lookup. Canonical ascending-index order is
/// guaranteed regardless of insertion history.
///
/// # Example
/// ```
/// use izanagi_kit::{sparse_set::{join3, SparseSet}, entity::EntityAllocator};
///
/// let mut alloc = EntityAllocator::new();
/// let e = alloc.allocate();
///
/// let mut pos: SparseSet<i32> = SparseSet::new();
/// let mut vel: SparseSet<i32> = SparseSet::new();
/// let mut hp:  SparseSet<i32> = SparseSet::new();
/// pos.insert(e, 10);
/// vel.insert(e, 1);
/// hp.insert(e, 100);
///
/// let results = join3(&pos, &vel, &hp);
/// assert_eq!(results.len(), 1);
/// assert_eq!(*results[0].1, 10); // pos
/// assert_eq!(*results[0].2,  1); // vel
/// assert_eq!(*results[0].3, 100); // hp
/// ```
pub fn join3<'a, A, B, C>(
    a: &'a SparseSet<A>,
    b: &'a SparseSet<B>,
    c: &'a SparseSet<C>,
) -> Vec<(Entity, &'a A, &'a B, &'a C)> {
    let mut out = Vec::new();
    let min_len = a.len().min(b.len()).min(c.len());
    if a.len() == min_len {
        for (entity, av) in a.iter() {
            if let (Some(bv), Some(cv)) = (b.get(entity), c.get(entity)) {
                out.push((entity, av, bv, cv));
            }
        }
    } else if b.len() == min_len {
        for (entity, bv) in b.iter() {
            if let (Some(av), Some(cv)) = (a.get(entity), c.get(entity)) {
                out.push((entity, av, bv, cv));
            }
        }
    } else {
        for (entity, cv) in c.iter() {
            if let (Some(av), Some(bv)) = (a.get(entity), b.get(entity)) {
                out.push((entity, av, bv, cv));
            }
        }
    }
    out.sort_unstable_by_key(|(e, _, _, _)| e.index());
    out
}

/// Like [`join3`], but yields a mutable reference to the first store's
/// component. Useful for systems that update `A` based on `B` and `C` — e.g.
/// `apply_velocity(pos_mut, vel, floor)`.
pub fn join3_mut<'a, A, B, C>(
    a: &'a mut SparseSet<A>,
    b: &'a SparseSet<B>,
    c: &'a SparseSet<C>,
) -> Vec<(Entity, &'a mut A, &'a B, &'a C)> {
    let mut out = Vec::new();
    for (entity, av) in a.iter_mut() {
        if let (Some(bv), Some(cv)) = (b.get(entity), c.get(entity)) {
            out.push((entity, av, bv, cv));
        }
    }
    out.sort_unstable_by_key(|(e, _, _, _)| e.index());
    out
}

/// Canonical filter of a component store by a tag set: every entity that has
/// the component *and* the tag, in ascending-index order, paired with a
/// reference to the component. This is the tag half of the multi-component
/// query — e.g. "every `Position` whose entity is `Frozen`".
///
/// The smaller side is iterated and the other probed, as in [`join`]; the
/// result is sorted by entity index so the order is canonical regardless of
/// either store's insert history (invariant G6).
pub fn join_tagged<'a, T>(set: &'a SparseSet<T>, tags: &TagSet) -> Vec<(Entity, &'a T)> {
    let mut out = Vec::new();
    if set.len() <= tags.len() {
        for (entity, value) in set.iter() {
            if tags.contains(entity) {
                out.push((entity, value));
            }
        }
    } else {
        for entity in tags.entities() {
            if let Some(value) = set.get(entity) {
                out.push((entity, value));
            }
        }
    }
    out.sort_unstable_by_key(|(e, _)| e.index());
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

    // --- find_all_entities / remove_where_returning ---

    #[test]
    fn test_find_all_entities_returns_all_matching() {
        let (_, es) = three();
        let mut s: SparseSet<i32> = SparseSet::new();
        s.insert(es[0], 10);
        s.insert(es[1], 20);
        s.insert(es[2], 30);
        let mut found = s.find_all_entities(|v| *v >= 20);
        found.sort_by_key(|e| e.index());
        assert_eq!(found, vec![es[1], es[2]]);
    }

    #[test]
    fn test_find_all_entities_empty_when_no_match() {
        let (_, es) = three();
        let mut s: SparseSet<i32> = SparseSet::new();
        s.insert(es[0], 5);
        assert!(s.find_all_entities(|v| *v > 100).is_empty());
    }

    #[test]
    fn test_find_all_entities_all_match() {
        let (_, es) = three();
        let mut s: SparseSet<i32> = SparseSet::new();
        s.insert(es[0], 1);
        s.insert(es[1], 2);
        s.insert(es[2], 3);
        assert_eq!(s.find_all_entities(|_| true).len(), 3);
    }

    #[test]
    fn test_remove_where_returning_returns_removed_values() {
        let (_, es) = three();
        let mut s: SparseSet<i32> = SparseSet::new();
        s.insert(es[0], 10);
        s.insert(es[1], 20);
        s.insert(es[2], 30);
        let mut removed = s.remove_where_returning(|_, v| *v >= 20);
        removed.sort_by_key(|(e, _)| e.index());
        assert_eq!(removed, vec![(es[1], 20), (es[2], 30)]);
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_remove_where_returning_empty_when_no_match() {
        let (_, es) = three();
        let mut s: SparseSet<i32> = SparseSet::new();
        s.insert(es[0], 5);
        let removed = s.remove_where_returning(|_, v| *v > 100);
        assert!(removed.is_empty());
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_remove_where_returning_all_match_clears_set() {
        let (_, es) = three();
        let mut s: SparseSet<i32> = SparseSet::new();
        s.insert(es[0], 1);
        s.insert(es[1], 2);
        let removed = s.remove_where_returning(|_, _| true);
        assert_eq!(removed.len(), 2);
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

    // --- drain ---

    #[test]
    fn test_drain_returns_all_entries_and_empties_set() {
        let (_, es) = three();
        let mut s: SparseSet<u32> = SparseSet::new();
        s.insert(es[0], 10);
        s.insert(es[1], 20);
        s.insert(es[2], 30);
        let drained = s.drain();
        assert_eq!(drained.len(), 3);
        assert!(s.is_empty());
    }

    #[test]
    fn test_drain_values_match_what_was_inserted() {
        let (_, es) = three();
        let mut s: SparseSet<u32> = SparseSet::new();
        s.insert(es[0], 7);
        s.insert(es[1], 13);
        let drained = s.drain();
        let vals: Vec<u32> = drained.iter().map(|(_, v)| *v).collect();
        assert!(vals.contains(&7));
        assert!(vals.contains(&13));
    }

    #[test]
    fn test_drain_empty_set_returns_empty_vec() {
        let mut s: SparseSet<u32> = SparseSet::new();
        let drained = s.drain();
        assert!(drained.is_empty());
    }

    #[test]
    fn test_drain_then_reinsert_works() {
        let (_, es) = three();
        let mut s: SparseSet<u32> = SparseSet::new();
        s.insert(es[0], 1);
        s.drain();
        s.insert(es[0], 99);
        assert_eq!(s.get(es[0]), Some(&99));
    }

    // --- any ---

    #[test]
    fn test_any_returns_true_on_match() {
        let (alloc, es) = three();
        let _ = alloc;
        let mut s: SparseSet<u32> = SparseSet::new();
        s.insert(es[0], 10);
        s.insert(es[1], 20);
        assert!(s.any(|&v| v == 20));
    }

    #[test]
    fn test_any_returns_false_when_no_match() {
        let (alloc, es) = three();
        let _ = alloc;
        let mut s: SparseSet<u32> = SparseSet::new();
        s.insert(es[0], 5);
        assert!(!s.any(|&v| v > 100));
    }

    #[test]
    fn test_any_empty_set_returns_false() {
        let s: SparseSet<u32> = SparseSet::new();
        assert!(!s.any(|_| true));
    }

    // --- join3 / join3_mut (W1) ---

    fn four() -> (EntityAllocator, [Entity; 4]) {
        let mut a = EntityAllocator::new();
        let es = [a.allocate(), a.allocate(), a.allocate(), a.allocate()];
        (a, es)
    }

    #[test]
    fn test_join3_entity_in_all_three_returns_triple() {
        let (_, es) = four();
        let mut pos: SparseSet<i32> = SparseSet::new();
        let mut vel: SparseSet<i32> = SparseSet::new();
        let mut hp: SparseSet<i32> = SparseSet::new();
        pos.insert(es[0], 10);
        vel.insert(es[0], 1);
        hp.insert(es[0], 100);
        let result = join3(&pos, &vel, &hp);
        assert_eq!(result.len(), 1);
        assert_eq!((result[0].1, result[0].2, result[0].3), (&10, &1, &100));
    }

    #[test]
    fn test_join3_entity_missing_in_one_excluded() {
        let (_, es) = four();
        let mut pos: SparseSet<i32> = SparseSet::new();
        let mut vel: SparseSet<i32> = SparseSet::new();
        let mut hp: SparseSet<i32> = SparseSet::new();
        pos.insert(es[0], 1);
        pos.insert(es[1], 2);
        vel.insert(es[0], 3);
        // es[1] has no hp — must be excluded
        hp.insert(es[0], 4);
        let result = join3(&pos, &vel, &hp);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, es[0]);
    }

    #[test]
    fn test_join3_empty_store_yields_empty() {
        let (_, es) = four();
        let mut pos: SparseSet<i32> = SparseSet::new();
        let vel: SparseSet<i32> = SparseSet::new(); // empty
        let mut hp: SparseSet<i32> = SparseSet::new();
        pos.insert(es[0], 1);
        hp.insert(es[0], 2);
        let result = join3(&pos, &vel, &hp);
        assert!(result.is_empty());
    }

    #[test]
    fn test_join3_ascending_index_order() {
        let (_, es) = four();
        let mut pos: SparseSet<u32> = SparseSet::new();
        let mut vel: SparseSet<u32> = SparseSet::new();
        let mut hp: SparseSet<u32> = SparseSet::new();
        // Insert in non-ascending order.
        for &e in &es[0..3] {
            pos.insert(e, 0);
            vel.insert(e, 0);
            hp.insert(e, 0);
        }
        let result = join3(&pos, &vel, &hp);
        assert_eq!(result.len(), 3);
        let indices: Vec<u32> = result.iter().map(|(e, _, _, _)| e.index()).collect();
        let mut sorted = indices.clone();
        sorted.sort_unstable();
        assert_eq!(
            indices, sorted,
            "join3 result must be in ascending entity index order"
        );
    }

    #[test]
    fn test_join3_mut_can_mutate_first_component() {
        let (_, es) = four();
        let mut pos: SparseSet<i32> = SparseSet::new();
        let mut vel: SparseSet<i32> = SparseSet::new();
        let mut hp: SparseSet<i32> = SparseSet::new();
        pos.insert(es[0], 0);
        vel.insert(es[0], 5);
        hp.insert(es[0], 100);
        for (_, p, v, _h) in join3_mut(&mut pos, &vel, &hp) {
            *p += *v;
        }
        assert_eq!(pos.get(es[0]), Some(&5), "position advanced by velocity");
    }

    #[test]
    fn test_join3_mut_order_matches_join3() {
        let (_, es) = four();
        let mut pos: SparseSet<u32> = SparseSet::new();
        let mut vel: SparseSet<u32> = SparseSet::new();
        let mut hp: SparseSet<u32> = SparseSet::new();
        for &e in &es[0..3] {
            pos.insert(e, 0);
            vel.insert(e, 0);
            hp.insert(e, 0);
        }
        let immut: Vec<Entity> = join3(&pos, &vel, &hp)
            .into_iter()
            .map(|(e, _, _, _)| e)
            .collect();
        let mut_: Vec<Entity> = join3_mut(&mut pos, &vel, &hp)
            .into_iter()
            .map(|(e, _, _, _)| e)
            .collect();
        assert_eq!(
            immut, mut_,
            "join3 and join3_mut must produce the same entity order"
        );
    }

    // --- generational safety: a stale handle must never alias a recycled slot ---

    #[test]
    fn test_stale_handle_does_not_alias_recycled_slot() {
        let mut alloc = EntityAllocator::new();
        let mut set: SparseSet<u32> = SparseSet::new();
        let e0 = alloc.allocate(); // idx 0, gen 0
        set.insert(e0, 100);
        alloc.free(e0);
        let e1 = alloc.allocate(); // idx 0, gen 1 — recycled slot
        assert_eq!(e1.index(), e0.index());
        assert_ne!(e1.generation(), e0.generation());
        set.insert(e1, 200); // e1's component overwrites the slot
        assert_eq!(
            set.get(e0),
            None,
            "stale handle (gen 0) must not read the recycled slot's component"
        );
        assert_eq!(
            set.get(e1),
            Some(&200),
            "live handle reads its own component"
        );
    }

    #[test]
    fn test_fresh_handle_at_recycled_index_has_no_component() {
        let mut alloc = EntityAllocator::new();
        let mut set: SparseSet<u32> = SparseSet::new();
        let e0 = alloc.allocate();
        set.insert(e0, 100);
        alloc.free(e0);
        let e1 = alloc.allocate(); // recycles idx 0 at gen 1; no component inserted
        assert_eq!(
            set.get(e1),
            None,
            "a fresh entity reusing an index must not inherit the prior occupant's component"
        );
        assert!(!set.contains(e1));
    }

    #[test]
    fn test_stale_handle_get_mut_and_remove_rejected() {
        let mut alloc = EntityAllocator::new();
        let mut set: SparseSet<u32> = SparseSet::new();
        let e0 = alloc.allocate();
        set.insert(e0, 1);
        alloc.free(e0);
        let e1 = alloc.allocate();
        set.insert(e1, 2);
        // Mutation and removal via the stale handle must both be rejected,
        // leaving e1's component intact.
        assert_eq!(set.get_mut(e0), None);
        assert_eq!(set.remove(e0), None);
        assert_eq!(set.get(e1), Some(&2), "e1's component survives stale ops");
        assert!(!set.contains(e0));
    }

    // --- TagSet: presence-only storage for marker components ---

    fn tag_hash(tags: &TagSet) -> u64 {
        use crate::world_hash::Fnv1a;
        let mut h = Fnv1a::new();
        tags.det_hash(&mut h);
        h.finish()
    }

    #[test]
    fn test_tag_insert_then_contains_and_len() {
        let (_, es) = three();
        let mut tags = TagSet::new();
        assert!(tags.is_empty());
        assert!(tags.insert(es[1]));
        assert!(tags.contains(es[1]));
        assert!(!tags.contains(es[0]));
        assert_eq!(tags.len(), 1);
        assert!(!tags.is_empty());
    }

    #[test]
    fn test_tag_insert_same_handle_twice_is_idempotent() {
        let (_, es) = three();
        let mut tags = TagSet::new();
        assert!(tags.insert(es[0]), "first insert is new");
        assert!(
            !tags.insert(es[0]),
            "second insert of the same handle is not"
        );
        assert_eq!(tags.len(), 1);
    }

    #[test]
    fn test_tag_default_is_empty() {
        let tags = TagSet::default();
        assert!(tags.is_empty());
        assert_eq!(tags.len(), 0);
    }

    #[test]
    fn test_tag_remove_present_and_absent() {
        let (_, es) = three();
        let mut tags = TagSet::new();
        tags.insert(es[0]);
        tags.insert(es[2]);
        assert!(tags.remove(es[0]), "present tag removes");
        assert!(!tags.contains(es[0]));
        assert!(tags.contains(es[2]), "the other tag survives");
        assert!(!tags.remove(es[0]), "removing twice reports absence");
        assert!(!tags.remove(es[1]), "never-tagged entity reports absence");
        assert_eq!(tags.len(), 1);
    }

    #[test]
    fn test_tag_remove_middle_keeps_others_via_swap() {
        let (_, es) = three();
        let mut tags = TagSet::new();
        for &e in &es {
            tags.insert(e);
        }
        // Removing the first entry swap-moves the last into its slot; the
        // moved entity must still be found through the sparse index.
        assert!(tags.remove(es[0]));
        assert_eq!(tags.len(), 2);
        assert!(tags.contains(es[1]));
        assert!(tags.contains(es[2]), "swap-moved entity stays reachable");
        assert!(!tags.contains(es[0]));
    }

    #[test]
    fn test_tag_entities_sorted_is_ascending_regardless_of_history() {
        let (_, es) = three();
        let mut tags = TagSet::new();
        tags.insert(es[2]);
        tags.insert(es[0]);
        tags.insert(es[1]);
        let order: Vec<u32> = tags.entities_sorted().iter().map(|e| e.index()).collect();
        assert_eq!(order, vec![0, 1, 2]);
        // Dense iteration holds every tagged entity exactly once.
        let mut dense: Vec<u32> = tags.entities().map(|e| e.index()).collect();
        dense.sort_unstable();
        assert_eq!(dense, vec![0, 1, 2]);
    }

    #[test]
    fn test_tag_clear_and_drain() {
        let (_, es) = three();
        let mut tags = TagSet::new();
        for &e in &es {
            tags.insert(e);
        }
        let drained = tags.drain();
        assert_eq!(drained.len(), 3);
        assert!(tags.is_empty());
        assert!(!tags.contains(es[0]), "no stale slot survives a drain");

        // Re-tagging after drain, then clear, behaves the same.
        tags.insert(es[1]);
        tags.clear();
        assert!(tags.is_empty());
        assert!(!tags.contains(es[1]), "no stale slot survives a clear");
        assert!(tags.insert(es[1]), "set is reusable after clear");
    }

    #[test]
    fn test_tag_retain_keeps_matching() {
        let (_, es) = three();
        let mut tags = TagSet::new();
        for &e in &es {
            tags.insert(e);
        }
        tags.retain(|e| e.index() != 1);
        assert_eq!(tags.len(), 2);
        assert!(tags.contains(es[0]));
        assert!(!tags.contains(es[1]));
        assert!(tags.contains(es[2]));
        tags.retain(|_| false);
        assert!(tags.is_empty(), "retain(false) untags everything");
    }

    #[test]
    fn test_tag_empty_set_queries() {
        let mut tags = TagSet::new();
        let (_, es) = three();
        assert_eq!(tags.len(), 0);
        assert!(tags.is_empty());
        assert!(!tags.contains(es[0]));
        assert!(!tags.remove(es[0]));
        assert!(tags.entities_sorted().is_empty());
        assert!(tags.drain().is_empty());
        // The empty population hashes to a fixed value: two independently
        // built empty sets agree.
        assert_eq!(tag_hash(&tags), tag_hash(&TagSet::new()));
    }

    #[test]
    fn test_tag_many_entities_churn() {
        let mut alloc = EntityAllocator::new();
        let pool: Vec<Entity> = (0..10_000).map(|_| alloc.allocate()).collect();
        let mut tags = TagSet::new();
        for &e in &pool {
            assert!(tags.insert(e));
        }
        assert_eq!(tags.len(), 10_000);
        // Remove every odd index; swap-remove churn must keep the sparse
        // index exact for the survivors.
        for &e in pool.iter().filter(|e| e.index() % 2 == 1) {
            assert!(tags.remove(e));
        }
        assert_eq!(tags.len(), 5_000);
        for &e in &pool {
            assert_eq!(
                tags.contains(e),
                e.index() % 2 == 0,
                "membership must be exact after churn at index {}",
                e.index()
            );
        }
    }

    #[test]
    fn test_tag_stale_handle_does_not_alias_recycled_index() {
        let mut alloc = EntityAllocator::new();
        let mut tags = TagSet::new();
        let e0 = alloc.allocate(); // idx 0, gen 0
        tags.insert(e0);
        alloc.free(e0);
        let e1 = alloc.allocate(); // idx 0, gen 1 — recycled index
        assert_eq!(e1.index(), e0.index());
        assert_ne!(e1.generation(), e0.generation());
        assert!(
            !tags.contains(e1),
            "a fresh entity reusing an index must not inherit the prior tag"
        );
        // Until the slot is refreshed, the store still holds e0's tag: the
        // allocator's `free` is invisible to the store, and `SparseSet`
        // behaves identically in this state (its slot still yields e0's
        // component — `test_fresh_handle_at_recycled_index_has_no_component`
        // asserts only the fresh handle's view). Removal via the stored
        // handle therefore succeeds here.
        assert!(tags.contains(e0));
        assert!(tags.remove(e0), "the stored handle's tag removes normally");
        assert!(tags.is_empty());
        // Once the live handle occupies the slot, the stale handle is
        // rejected for reads and removal alike — the generational guarantee,
        // mirroring `test_stale_handle_get_mut_and_remove_rejected`.
        assert!(tags.insert(e1), "tagging the live handle is a new insert");
        assert!(tags.contains(e1));
        assert!(
            !tags.contains(e0),
            "the stale handle stays rejected after the slot is refreshed"
        );
        assert!(
            !tags.remove(e0),
            "removal via the stale handle must be rejected"
        );
        assert!(tags.remove(e1));
        assert!(tags.is_empty());
    }

    #[test]
    fn test_tag_det_hash_ignores_insert_order_but_tracks_population() {
        let (_, es) = three();
        let hash_of = |order: &[Entity]| {
            let mut tags = TagSet::new();
            for &e in order {
                tags.insert(e);
            }
            tag_hash(&tags)
        };
        let forward = hash_of(&[es[0], es[1], es[2]]);
        let shuffled = hash_of(&[es[2], es[0], es[1]]);
        assert_eq!(forward, shuffled, "canonical hash must ignore insert order");
        let shorter = hash_of(&[es[0], es[1]]);
        assert_ne!(forward, shorter, "population (length) must be folded in");
        let different = hash_of(&[es[0], es[2]]);
        assert_ne!(shorter, different, "membership must be folded in");
    }

    #[test]
    fn test_tag_det_hash_matches_sparse_set_of_unit() {
        // Oracle: `()` folds nothing, so a `SparseSet<()>` over the same
        // population is the reference behaviour — if TagSet folded anything
        // more or less (or in a different order), the hashes would diverge.
        use crate::world_hash::Fnv1a;
        let mut alloc = EntityAllocator::new();
        let pool: Vec<Entity> = (0..6).map(|_| alloc.allocate()).collect();
        let mut tags = TagSet::new();
        let mut reference: SparseSet<()> = SparseSet::new();
        // Scrambled insert order, then a removal, on both stores.
        for &i in &[3usize, 0, 5, 1] {
            tags.insert(pool[i]);
            reference.insert(pool[i], ());
        }
        tags.remove(pool[0]);
        reference.remove(pool[0]);

        let mut h = Fnv1a::new();
        reference.det_hash(&mut h);
        assert_eq!(
            tag_hash(&tags),
            h.finish(),
            "TagSet must fold exactly what SparseSet<()> folds"
        );
    }

    #[test]
    fn test_tag_same_op_sequence_twice_gives_identical_state() {
        let build = || {
            let mut alloc = EntityAllocator::new();
            let pool: Vec<Entity> = (0..8).map(|_| alloc.allocate()).collect();
            let mut tags = TagSet::new();
            for &i in &[5usize, 2, 7, 2, 0] {
                tags.insert(pool[i]);
            }
            tags.remove(pool[5]);
            tags.insert(pool[5]);
            tags
        };
        let a = build();
        let b = build();
        assert_eq!(a.entities_sorted(), b.entities_sorted());
        assert_eq!(tag_hash(&a), tag_hash(&b), "same input → same hash, twice");
    }

    #[test]
    fn test_tag_model_oracle_random_ops() {
        use crate::rng::SplitMix64;
        let mut alloc = EntityAllocator::new();
        let pool: Vec<Entity> = (0..24).map(|_| alloc.allocate()).collect();
        let mut present = [false; 24];
        let mut tags = TagSet::new();
        let mut rng = SplitMix64::new(0x7a65);
        for step in 0..600 {
            let i = rng.below(24) as usize;
            if rng.below(3) == 2 {
                assert_eq!(
                    tags.remove(pool[i]),
                    present[i],
                    "remove result must match the model at step {step}"
                );
                present[i] = false;
            } else {
                assert_eq!(
                    tags.insert(pool[i]),
                    !present[i],
                    "insert result must match the model at step {step}"
                );
                present[i] = true;
            }
            if step % 100 == 0 {
                assert_eq!(tags.len(), present.iter().filter(|&&p| p).count());
            }
        }
        // Final state must equal the model exactly: membership, canonical
        // order, and hash (via the SparseSet<()> oracle).
        let expected: Vec<Entity> = pool
            .iter()
            .copied()
            .filter(|e| present[e.index() as usize])
            .collect();
        assert_eq!(tags.entities_sorted(), expected);
        let mut reference: SparseSet<()> = SparseSet::new();
        for &e in &expected {
            reference.insert(e, ());
        }
        use crate::world_hash::Fnv1a;
        let mut h = Fnv1a::new();
        reference.det_hash(&mut h);
        assert_eq!(tag_hash(&tags), h.finish());
    }

    // --- join_tagged: TagSet composed with the existing join API family ---

    #[test]
    fn test_join_tagged_returns_tagged_subset_canonically() {
        let mut alloc = EntityAllocator::new();
        let es: Vec<Entity> = (0..4).map(|_| alloc.allocate()).collect();
        let mut pos: SparseSet<i32> = SparseSet::new();
        let mut tags = TagSet::new();
        // Scrambled inserts; es[1] and es[3] have both, es[0] lacks the tag,
        // es[2] has the tag but no component.
        pos.insert(es[3], 30);
        pos.insert(es[0], 0);
        pos.insert(es[1], 10);
        tags.insert(es[1]);
        tags.insert(es[3]);
        tags.insert(es[2]);

        let joined = join_tagged(&pos, &tags);
        let ids: Vec<u32> = joined.iter().map(|(e, _)| e.index()).collect();
        assert_eq!(ids, vec![1, 3], "intersection only, ascending index");
        assert_eq!(joined[0].1, &10);
        assert_eq!(joined[1].1, &30);
    }

    #[test]
    fn test_join_tagged_empty_when_either_side_empty() {
        let (_, es) = three();
        let mut pos: SparseSet<i32> = SparseSet::new();
        pos.insert(es[0], 1);
        let no_tags = TagSet::new();
        assert!(join_tagged(&pos, &no_tags).is_empty());

        let empty_set: SparseSet<i32> = SparseSet::new();
        let mut tags = TagSet::new();
        tags.insert(es[0]);
        assert!(join_tagged(&empty_set, &tags).is_empty());
    }

    #[test]
    fn test_join_tagged_result_independent_of_which_side_is_smaller() {
        let mut alloc = EntityAllocator::new();
        let es: Vec<Entity> = (0..5).map(|_| alloc.allocate()).collect();
        let mut pos: SparseSet<i32> = SparseSet::new();
        let mut tags = TagSet::new();
        // Component store smaller than the tag set (iterates the store).
        pos.insert(es[4], 40);
        pos.insert(es[1], 10);
        for &e in &es {
            tags.insert(e);
        }
        let ids: Vec<u32> = join_tagged(&pos, &tags)
            .iter()
            .map(|(e, _)| e.index())
            .collect();
        assert_eq!(ids, vec![1, 4]);

        // Tag set smaller than the component store (iterates the tags).
        let mut pos2: SparseSet<i32> = SparseSet::new();
        for (i, &e) in es.iter().enumerate() {
            pos2.insert(e, i as i32);
        }
        let mut tags2 = TagSet::new();
        tags2.insert(es[3]);
        tags2.insert(es[0]);
        let ids2: Vec<u32> = join_tagged(&pos2, &tags2)
            .iter()
            .map(|(e, _)| e.index())
            .collect();
        assert_eq!(ids2, vec![0, 3]);
    }
}
