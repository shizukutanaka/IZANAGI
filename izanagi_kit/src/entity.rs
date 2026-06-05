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
    pub fn free(&mut self, entity: Entity) {
        if !self.is_alive(entity) {
            return;
        }
        self.generations[entity.index as usize] = entity.generation.wrapping_add(1);
        self.free.push(entity.index);
    }

    #[inline]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.generations
            .get(entity.index as usize)
            .is_some_and(|&g| g == entity.generation)
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
}
