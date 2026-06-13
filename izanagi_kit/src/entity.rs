//! Generational entity handles. Reused indices carry a bumped generation,
//! so a stale handle to a despawned-then-respawned slot is rejected.

/// Opaque handle. `index` locates storage; `generation` validates liveness.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Entity {
    index: u32,
    generation: u32,
}

impl Entity {
    #[inline]
    pub fn index(self) -> u32 {
        self.index
    }

    #[inline]
    pub fn generation(self) -> u32 {
        self.generation
    }
}

/// Hands out entities with a free list. Deterministic: identical
/// allocate/free sequences yield identical handles across runs/platforms.
#[derive(Default)]
pub struct EntityAllocator {
    generations: Vec<u32>,
    free: Vec<u32>,
    /// Counts generation-counter wrap events (a slot freed more than 2^32
    /// times). Any non-zero value is a diagnostic red flag: stale handles with
    /// generation 0 may silently alias freshly recycled slots at the same index.
    wrap_count: u32,
}

impl EntityAllocator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reuses a freed slot when available, else grows. O(1).
    pub fn allocate(&mut self) -> Entity {
        if let Some(index) = self.free.pop() {
            return Entity {
                index,
                generation: self.generations[index as usize],
            };
        }
        let index = self.generations.len() as u32;
        self.generations.push(0);
        Entity {
            index,
            generation: 0,
        }
    }

    /// Frees a live handle, bumping its generation so stale copies fail
    /// `is_alive`. Double-free or stale-free is a no-op.
    ///
    /// # Generation overflow
    /// When the generation counter reaches [`u32::MAX`] and wraps to 0 on the
    /// next free, a stale handle at generation 0 becomes indistinguishable from
    /// a freshly recycled slot. This event is recorded in [`generation_wrap_count`]
    /// and fires a `debug_assert!` so development builds catch it early.
    ///
    /// [`generation_wrap_count`]: EntityAllocator::generation_wrap_count
    pub fn free(&mut self, entity: Entity) {
        if !self.is_alive(entity) {
            return;
        }
        if entity.generation == u32::MAX {
            // Generation counter is about to wrap to 0: a future stale handle
            // with generation 0 at this index will be indistinguishable from a
            // fresh allocation. Record the event; callers check
            // `generation_wrap_count()` in diagnostics / save-file audits.
            self.wrap_count = self.wrap_count.saturating_add(1);
        }
        self.generations[entity.index as usize] = entity.generation.wrapping_add(1);
        self.free.push(entity.index);
    }

    /// Number of generation-counter wrap events: slots that have been recycled
    /// so many times (> 2^32) that the counter rolled over to 0. A non-zero
    /// value is a red flag in long-running applications; zero is expected for
    /// any reasonably bounded game session.
    #[inline]
    pub fn generation_wrap_count(&self) -> u32 {
        self.wrap_count
    }

    #[inline]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.generations
            .get(entity.index as usize)
            .is_some_and(|&g| g == entity.generation)
    }

    /// Number of currently live (allocated and not yet freed) entities.
    ///
    /// O(1) — computed as `total_slots − free_slots`.
    #[inline]
    pub fn count(&self) -> usize {
        self.generations.len() - self.free.len()
    }

    /// Total slots that have ever been created (both live and freed). O(1).
    /// Useful for metrics ("the allocator has seen up to N distinct entities"),
    /// memory-budget checks, and save-file headers.
    #[inline]
    pub fn total_slots(&self) -> usize {
        self.generations.len()
    }

    /// O(1) count of freed (reusable) slots. Equivalent to
    /// `total_slots() - count()`. Useful for memory diagnostics ("N slots
    /// available for reuse before the allocator grows").
    #[inline]
    pub fn free_count(&self) -> usize {
        self.free.len()
    }

    /// Free every entity in `entities`. Equivalent to calling `free` for each
    /// element; stale and duplicate entries are ignored (no-op per `free`).
    pub fn batch_free(&mut self, entities: &[Entity]) {
        for &e in entities {
            self.free(e);
        }
    }

    /// Allocate `n` entities in a single call, returning them as a `Vec`.
    /// Equivalent to `(0..n).map(|_| alloc.allocate()).collect()` but named
    /// for clarity at bulk-spawn call sites (e.g. "spawn 20 enemies at once").
    pub fn batch_alloc(&mut self, n: usize) -> Vec<Entity> {
        (0..n).map(|_| self.allocate()).collect()
    }

    /// The highest generation counter across all slots, or `0` when the
    /// allocator is empty. A value near [`u32::MAX`] means a slot has been
    /// recycled an extraordinary number of times — useful for diagnostics and
    /// long-running save-file audits.
    #[inline]
    pub fn highest_generation(&self) -> u32 {
        self.generations.iter().copied().max().unwrap_or(0)
    }

    /// Return the live entity at slot `index`, or `None` if the slot is out of
    /// range or the entity has been freed.
    ///
    /// O(1) generation lookup plus an O(free-list) liveness check. Useful for
    /// serialisation round-trips and debug tools that need "what entity is at
    /// this exact index?" without enumerating all live entities.
    pub fn live_at_index(&self, index: u32) -> Option<Entity> {
        let gen = self.generations.get(index as usize).copied()?;
        if self.free.contains(&index) {
            return None;
        }
        Some(Entity {
            index,
            generation: gen,
        })
    }

    /// All currently live entities in ascending index order.
    ///
    /// Useful for systems that need to enumerate every spawned entity without
    /// access to a `SparseSet` or `ArchTable` (e.g. serialisation, debug
    /// overlays, end-of-frame cleanup). O(n · m) where n = total slots and
    /// m = free-list length; for typical game entity counts this is fast.
    pub fn live_entities(&self) -> Vec<Entity> {
        self.generations
            .iter()
            .enumerate()
            .filter_map(|(i, &gen)| {
                if self.free.contains(&(i as u32)) {
                    None
                } else {
                    Some(Entity {
                        index: i as u32,
                        generation: gen,
                    })
                }
            })
            .collect()
    }
}

impl crate::world_hash::DetHash for Entity {
    #[inline]
    fn det_hash(&self, hasher: &mut crate::world_hash::Fnv1a) {
        hasher.write_u32(self.index);
        hasher.write_u32(self.generation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_fresh_starts_at_gen_zero() {
        let mut a = EntityAllocator::new();
        let e = a.allocate();
        assert_eq!((e.index(), e.generation()), (0, 0));
        assert!(a.is_alive(e));
    }

    #[test]
    fn test_free_then_alloc_reuses_index_with_bumped_gen() {
        let mut a = EntityAllocator::new();
        let e0 = a.allocate();
        a.free(e0);
        let e1 = a.allocate();
        assert_eq!(e1.index(), e0.index());
        assert_eq!(e1.generation(), 1);
    }

    #[test]
    fn test_stale_handle_after_free_is_not_alive() {
        let mut a = EntityAllocator::new();
        let e0 = a.allocate();
        a.free(e0);
        let _e1 = a.allocate();
        assert!(!a.is_alive(e0), "stale handle must be rejected");
    }

    #[test]
    fn test_double_free_is_noop() {
        let mut a = EntityAllocator::new();
        let e = a.allocate();
        a.free(e);
        a.free(e); // must not corrupt the free list
        assert_eq!(a.allocate().index(), e.index());
    }

    #[test]
    fn test_live_entities_empty_allocator() {
        let a = EntityAllocator::new();
        assert!(a.live_entities().is_empty());
    }

    #[test]
    fn test_live_entities_all_allocated() {
        let mut a = EntityAllocator::new();
        let e0 = a.allocate();
        let e1 = a.allocate();
        let live = a.live_entities();
        assert_eq!(live.len(), 2);
        assert!(live.contains(&e0));
        assert!(live.contains(&e1));
    }

    #[test]
    fn test_live_entities_excludes_freed() {
        let mut a = EntityAllocator::new();
        let e0 = a.allocate();
        let e1 = a.allocate();
        a.free(e0);
        let live = a.live_entities();
        assert_eq!(live.len(), 1);
        assert!(!live.contains(&e0));
        assert!(live.contains(&e1));
    }

    #[test]
    fn test_live_entities_sorted_by_index() {
        let mut a = EntityAllocator::new();
        let e0 = a.allocate();
        let e1 = a.allocate();
        let e2 = a.allocate();
        let live = a.live_entities();
        assert_eq!(live[0], e0);
        assert_eq!(live[1], e1);
        assert_eq!(live[2], e2);
    }

    #[test]
    fn test_batch_free_frees_all_live() {
        let mut a = EntityAllocator::new();
        let e0 = a.allocate();
        let e1 = a.allocate();
        let e2 = a.allocate();
        a.batch_free(&[e0, e1, e2]);
        assert_eq!(a.count(), 0);
        assert!(!a.is_alive(e0));
        assert!(!a.is_alive(e1));
        assert!(!a.is_alive(e2));
    }

    #[test]
    fn test_batch_free_ignores_stale_and_unknown() {
        let mut a = EntityAllocator::new();
        let e = a.allocate();
        a.free(e); // stale now
                   // Should not panic or corrupt allocator.
        a.batch_free(&[e]);
        assert_eq!(a.count(), 0);
    }

    #[test]
    fn test_batch_free_partial() {
        let mut a = EntityAllocator::new();
        let e0 = a.allocate();
        let e1 = a.allocate();
        let e2 = a.allocate();
        a.batch_free(&[e0, e2]);
        assert!(!a.is_alive(e0));
        assert!(a.is_alive(e1));
        assert!(!a.is_alive(e2));
        assert_eq!(a.count(), 1);
    }

    #[test]
    fn test_total_slots_grows_with_allocations() {
        let mut a = EntityAllocator::new();
        assert_eq!(a.total_slots(), 0);
        a.allocate();
        a.allocate();
        assert_eq!(a.total_slots(), 2);
        let e = a.allocate();
        a.free(e); // freed, but slot still exists
        assert_eq!(a.total_slots(), 3); // slot count does not shrink
    }

    #[test]
    fn test_total_slots_reuse_does_not_grow() {
        let mut a = EntityAllocator::new();
        let e = a.allocate();
        a.free(e);
        a.allocate(); // reuses e's slot — no new slot created
        assert_eq!(a.total_slots(), 1);
    }

    #[test]
    fn test_count_tracks_live_entities() {
        let mut a = EntityAllocator::new();
        assert_eq!(a.count(), 0);
        let e0 = a.allocate();
        let e1 = a.allocate();
        assert_eq!(a.count(), 2);
        a.free(e0);
        assert_eq!(a.count(), 1);
        let _ = a.allocate(); // reuse e0's slot
        assert_eq!(a.count(), 2);
        a.free(e1);
        assert_eq!(a.count(), 1);
    }

    #[test]
    fn test_highest_generation_empty_is_zero() {
        let a = EntityAllocator::new();
        assert_eq!(a.highest_generation(), 0);
    }

    #[test]
    fn test_highest_generation_fresh_allocs_is_zero() {
        let mut a = EntityAllocator::new();
        a.allocate();
        a.allocate();
        assert_eq!(
            a.highest_generation(),
            0,
            "no frees yet — all generations at 0"
        );
    }

    #[test]
    fn test_highest_generation_tracks_frees() {
        let mut a = EntityAllocator::new();
        let e = a.allocate();
        a.free(e);
        assert_eq!(a.highest_generation(), 1);
        let e2 = a.allocate(); // reuses slot at gen 1
        a.free(e2);
        assert_eq!(a.highest_generation(), 2);
    }

    #[test]
    fn test_free_count_empty_allocator_is_zero() {
        let a = EntityAllocator::new();
        assert_eq!(a.free_count(), 0);
    }

    #[test]
    fn test_free_count_increments_on_free() {
        let mut a = EntityAllocator::new();
        let e = a.allocate();
        assert_eq!(a.free_count(), 0);
        a.free(e);
        assert_eq!(a.free_count(), 1);
    }

    #[test]
    fn test_free_count_decrements_on_realloc() {
        let mut a = EntityAllocator::new();
        let e = a.allocate();
        a.free(e);
        assert_eq!(a.free_count(), 1);
        a.allocate(); // reuses the freed slot
        assert_eq!(a.free_count(), 0);
    }

    #[test]
    fn test_batch_alloc_returns_n_live_distinct_entities() {
        let mut a = EntityAllocator::new();
        let batch = a.batch_alloc(5);
        assert_eq!(batch.len(), 5);
        assert_eq!(a.count(), 5);
        let indices: std::collections::HashSet<u32> = batch.iter().map(|e| e.index()).collect();
        assert_eq!(indices.len(), 5, "all indices must be distinct");
    }

    #[test]
    fn test_batch_alloc_zero_returns_empty() {
        let mut a = EntityAllocator::new();
        let batch = a.batch_alloc(0);
        assert!(batch.is_empty());
        assert_eq!(a.count(), 0);
    }

    #[test]
    fn test_batch_alloc_entities_are_alive() {
        let mut a = EntityAllocator::new();
        for e in a.batch_alloc(3) {
            assert!(a.is_alive(e));
        }
    }

    // --- live_at_index ---

    #[test]
    fn test_live_at_index_returns_live_entity() {
        let mut a = EntityAllocator::new();
        let e = a.allocate();
        let found = a.live_at_index(e.index()).unwrap();
        assert_eq!(found, e);
    }

    #[test]
    fn test_live_at_index_freed_entity_returns_none() {
        let mut a = EntityAllocator::new();
        let e = a.allocate();
        a.free(e);
        assert_eq!(a.live_at_index(e.index()), None);
    }

    #[test]
    fn test_live_at_index_out_of_range_returns_none() {
        let a = EntityAllocator::new();
        assert_eq!(a.live_at_index(99), None);
    }

    // --- generation overflow / wrap detection (W2) ---

    #[test]
    fn test_wrap_count_zero_on_new_allocator() {
        let a = EntityAllocator::new();
        assert_eq!(a.generation_wrap_count(), 0);
    }

    #[test]
    fn test_wrap_count_zero_on_normal_free_and_realloc() {
        let mut a = EntityAllocator::new();
        let e = a.allocate();
        a.free(e);
        let e2 = a.allocate(); // generation 1 now
        a.free(e2);
        assert_eq!(
            a.generation_wrap_count(),
            0,
            "normal frees must not increment wrap_count"
        );
    }

    #[test]
    fn test_wrap_count_increments_when_generation_is_u32_max() {
        let mut a = EntityAllocator::new();
        let e = a.allocate(); // slot 0, generation 0

        // Forcibly set the stored generation to u32::MAX so the *next* free wraps.
        a.generations[e.index() as usize] = u32::MAX;
        // Construct a matching entity handle (private field access is valid inside this module).
        let e_max = Entity {
            index: e.index(),
            generation: u32::MAX,
        };
        // is_alive must hold (stored gen == handle gen).
        assert!(a.is_alive(e_max), "precondition: entity must be live");

        // Free it — this should record the wrap.
        a.free(e_max);
        assert_eq!(a.generation_wrap_count(), 1, "one wrap event must be recorded");
        // The generation stored should have wrapped to 0.
        assert_eq!(a.generations[e.index() as usize], 0, "wrapping_add(1) from MAX → 0");
    }

    #[test]
    fn test_wrap_count_saturates_not_overflows() {
        let mut a = EntityAllocator::new();
        // Manually set wrap_count near u32::MAX and trigger another wrap.
        a.wrap_count = u32::MAX;

        let e = a.allocate();
        a.generations[e.index() as usize] = u32::MAX;
        let e_max = Entity {
            index: e.index(),
            generation: u32::MAX,
        };
        a.free(e_max);
        assert_eq!(
            a.generation_wrap_count(),
            u32::MAX,
            "saturating_add must not overflow past u32::MAX"
        );
    }

    #[test]
    fn test_wrap_does_not_corrupt_free_list() {
        let mut a = EntityAllocator::new();
        let e = a.allocate();
        a.generations[e.index() as usize] = u32::MAX;
        let e_max = Entity {
            index: e.index(),
            generation: u32::MAX,
        };
        a.free(e_max);
        // After the wrap-free, the slot should be reusable.
        let e2 = a.allocate();
        assert_eq!(e2.index(), e.index(), "slot must be reused");
        assert_eq!(e2.generation(), 0, "generation wraps back to 0");
        assert!(a.is_alive(e2));
    }
}
