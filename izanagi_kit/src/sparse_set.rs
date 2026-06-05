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
}
