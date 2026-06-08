//! Archetype-based component storage (C4).
//!
//! An **archetype** groups entities that share the same set of component types
//! into a tightly-packed parallel array (struct-of-arrays). Compared to
//! independent `SparseSet` columns, this layout improves cache utilisation when
//! iterating over multi-component queries: all data for one entity is fetched in
//! the same cache line, and the entity list walks a single dense array.
//!
//! This module provides:
//!
//! - [`ArchTable<Row>`] — a single archetype: one dense `Vec<(Entity, Row)>` with
//!   a secondary `HashMap<Entity, usize>` for O(1) lookup. `Row` is a
//!   caller-defined struct that bundles the component fields for this archetype.
//! - [`DetHash`] — canonical hash (entity-index order) so archetypes participate
//!   in replay state checksums.
//!
//! # Design notes
//!
//! *Multiple archetypes* (an archetype-set registry) are naturally built on top:
//! the caller maintains several `ArchTable` instances, each corresponding to one
//! component combination. `Entity` migration between archetypes is O(1) for the
//! swap-remove step in the source table and a push in the destination. That
//! higher-level orchestration is left to the game engine layer.

use std::collections::HashMap;

use crate::{
    entity::Entity,
    world_hash::{DetHash, Fnv1a},
};

/// A densely-packed archetype table.
///
/// Stores `(Entity, Row)` pairs in a contiguous array for O(n) cache-friendly
/// iteration. A secondary index (`HashMap<Entity, usize>`) provides O(1)
/// insert/remove/get. Removing an entry uses **swap-remove** to keep the array
/// dense — callers must not cache slot indices across mutations.
pub struct ArchTable<Row: Clone> {
    dense: Vec<(Entity, Row)>,
    index: HashMap<Entity, usize>,
}

impl<Row: Clone> Default for ArchTable<Row> {
    fn default() -> Self {
        Self {
            dense: Vec::new(),
            index: HashMap::new(),
        }
    }
}

impl<Row: Clone> ArchTable<Row> {
    /// Create an empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert `entity` with row data `row`. Returns `true` if newly inserted,
    /// `false` if the entity was already present (row is left unchanged).
    pub fn insert(&mut self, entity: Entity, row: Row) -> bool {
        if self.index.contains_key(&entity) {
            return false;
        }
        let slot = self.dense.len();
        self.dense.push((entity, row));
        self.index.insert(entity, slot);
        true
    }

    /// Remove `entity` and return its row, or `None` if absent.
    /// Uses swap-remove: O(1) but does not preserve order.
    pub fn remove(&mut self, entity: Entity) -> Option<Row> {
        let slot = *self.index.get(&entity)?;
        let last = self.dense.len() - 1;
        if slot != last {
            // Swap with the last element and update its index entry.
            self.dense.swap(slot, last);
            let moved_entity = self.dense[slot].0;
            self.index.insert(moved_entity, slot);
        }
        self.index.remove(&entity);
        Some(self.dense.pop().unwrap().1)
    }

    /// Borrow the row for `entity`, or `None` if absent.
    pub fn get(&self, entity: Entity) -> Option<&Row> {
        let &slot = self.index.get(&entity)?;
        Some(&self.dense[slot].1)
    }

    /// Mutably borrow the row for `entity`, or `None` if absent.
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut Row> {
        let &slot = self.index.get(&entity)?;
        Some(&mut self.dense[slot].1)
    }

    /// Whether `entity` is present in this table.
    pub fn contains(&self, entity: Entity) -> bool {
        self.index.contains_key(&entity)
    }

    /// Number of entities in this table.
    pub fn len(&self) -> usize {
        self.dense.len()
    }

    /// Whether the table is empty.
    pub fn is_empty(&self) -> bool {
        self.dense.is_empty()
    }

    /// Iterate `(Entity, &Row)` in dense array order (fast, cache-friendly).
    /// Order is insertion order up to any intervening removes.
    pub fn iter(&self) -> impl Iterator<Item = (Entity, &Row)> {
        self.dense.iter().map(|(e, r)| (*e, r))
    }

    /// Iterate `(Entity, &mut Row)` in dense array order.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut Row)> {
        self.dense.iter_mut().map(|(e, r)| (*e, r))
    }

    /// Remove all entries.
    pub fn clear(&mut self) {
        self.dense.clear();
        self.index.clear();
    }
}

impl<Row: Clone + DetHash> DetHash for ArchTable<Row> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.dense.len() as u32);
        // Sort by entity index for canonical ordering independent of
        // insert/remove history (swap-remove makes dense order unstable).
        let mut sorted: Vec<(Entity, &Row)> = self.dense.iter().map(|(e, r)| (*e, r)).collect();
        sorted.sort_by_key(|(e, _)| e.index());
        for (e, r) in sorted {
            hasher.write_u32(e.index());
            hasher.write_u32(e.generation());
            r.det_hash(hasher);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{entity::EntityAllocator, world_hash::hash_state};

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Pos {
        x: i32,
        y: i32,
    }

    impl DetHash for Pos {
        fn det_hash(&self, hasher: &mut Fnv1a) {
            hasher.write_i32(self.x);
            hasher.write_i32(self.y);
        }
    }

    fn alloc() -> EntityAllocator {
        EntityAllocator::new()
    }

    #[test]
    fn test_insert_and_get() {
        let mut a = alloc();
        let e = a.allocate();
        let mut table: ArchTable<Pos> = ArchTable::new();
        assert!(table.insert(e, Pos { x: 1, y: 2 }));
        assert_eq!(table.get(e), Some(&Pos { x: 1, y: 2 }));
    }

    #[test]
    fn test_insert_duplicate_returns_false() {
        let mut a = alloc();
        let e = a.allocate();
        let mut table: ArchTable<Pos> = ArchTable::new();
        assert!(table.insert(e, Pos { x: 0, y: 0 }));
        assert!(!table.insert(e, Pos { x: 9, y: 9 }));
        // Original row is unchanged.
        assert_eq!(table.get(e), Some(&Pos { x: 0, y: 0 }));
    }

    #[test]
    fn test_remove_returns_row() {
        let mut a = alloc();
        let e = a.allocate();
        let mut table: ArchTable<Pos> = ArchTable::new();
        table.insert(e, Pos { x: 3, y: 4 });
        let removed = table.remove(e);
        assert_eq!(removed, Some(Pos { x: 3, y: 4 }));
        assert!(!table.contains(e));
    }

    #[test]
    fn test_remove_absent_returns_none() {
        let mut a = alloc();
        let e = a.allocate();
        let mut table: ArchTable<Pos> = ArchTable::new();
        assert!(table.remove(e).is_none());
    }

    #[test]
    fn test_remove_middle_keeps_table_dense() {
        let mut a = alloc();
        let e0 = a.allocate();
        let e1 = a.allocate();
        let e2 = a.allocate();
        let mut table: ArchTable<Pos> = ArchTable::new();
        table.insert(e0, Pos { x: 0, y: 0 });
        table.insert(e1, Pos { x: 1, y: 1 });
        table.insert(e2, Pos { x: 2, y: 2 });
        table.remove(e0);
        assert_eq!(table.len(), 2);
        // e1 and e2 still accessible.
        assert!(table.contains(e1));
        assert!(table.contains(e2));
    }

    #[test]
    fn test_get_mut_modifies_row() {
        let mut a = alloc();
        let e = a.allocate();
        let mut table: ArchTable<Pos> = ArchTable::new();
        table.insert(e, Pos { x: 1, y: 1 });
        if let Some(r) = table.get_mut(e) {
            r.x = 99;
        }
        assert_eq!(table.get(e).unwrap().x, 99);
    }

    #[test]
    fn test_contains() {
        let mut a = alloc();
        let e = a.allocate();
        let absent = a.allocate();
        let mut table: ArchTable<Pos> = ArchTable::new();
        table.insert(e, Pos { x: 0, y: 0 });
        assert!(table.contains(e));
        assert!(!table.contains(absent));
    }

    #[test]
    fn test_len_and_is_empty() {
        let mut a = alloc();
        let mut table: ArchTable<Pos> = ArchTable::new();
        assert!(table.is_empty());
        let e = a.allocate();
        table.insert(e, Pos { x: 0, y: 0 });
        assert_eq!(table.len(), 1);
        assert!(!table.is_empty());
    }

    #[test]
    fn test_iter_visits_all() {
        let mut a = alloc();
        let mut table: ArchTable<Pos> = ArchTable::new();
        let entities: Vec<Entity> = (0..5).map(|_| a.allocate()).collect();
        for (i, &e) in entities.iter().enumerate() {
            table.insert(e, Pos { x: i as i32, y: 0 });
        }
        let iterated: Vec<Entity> = table.iter().map(|(e, _)| e).collect();
        assert_eq!(iterated.len(), 5);
        for e in &entities {
            assert!(iterated.contains(e));
        }
    }

    #[test]
    fn test_iter_mut_modifies_all() {
        let mut a = alloc();
        let mut table: ArchTable<Pos> = ArchTable::new();
        for _ in 0..4 {
            let e = a.allocate();
            table.insert(e, Pos { x: 0, y: 0 });
        }
        for (_, r) in table.iter_mut() {
            r.x = 7;
        }
        assert!(table.iter().all(|(_, r)| r.x == 7));
    }

    #[test]
    fn test_clear() {
        let mut a = alloc();
        let mut table: ArchTable<Pos> = ArchTable::new();
        for _ in 0..3 {
            let e = a.allocate();
            table.insert(e, Pos { x: 0, y: 0 });
        }
        table.clear();
        assert!(table.is_empty());
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn test_det_hash_same_state() {
        let mut a = alloc();
        let e = a.allocate();
        let mut t1: ArchTable<Pos> = ArchTable::new();
        let mut t2: ArchTable<Pos> = ArchTable::new();
        t1.insert(e, Pos { x: 5, y: 6 });
        t2.insert(e, Pos { x: 5, y: 6 });
        assert_eq!(hash_state(&t1), hash_state(&t2));
    }

    #[test]
    fn test_det_hash_differs_on_row_change() {
        let mut a = alloc();
        let e = a.allocate();
        let mut t1: ArchTable<Pos> = ArchTable::new();
        let mut t2: ArchTable<Pos> = ArchTable::new();
        t1.insert(e, Pos { x: 1, y: 2 });
        t2.insert(e, Pos { x: 9, y: 9 });
        assert_ne!(hash_state(&t1), hash_state(&t2));
    }

    #[test]
    fn test_det_hash_canonical_regardless_of_insert_order() {
        let mut a = alloc();
        let e0 = a.allocate();
        let e1 = a.allocate();
        let mut t1: ArchTable<Pos> = ArchTable::new();
        let mut t2: ArchTable<Pos> = ArchTable::new();
        // Insert in different orders.
        t1.insert(e0, Pos { x: 1, y: 0 });
        t1.insert(e1, Pos { x: 2, y: 0 });
        t2.insert(e1, Pos { x: 2, y: 0 });
        t2.insert(e0, Pos { x: 1, y: 0 });
        assert_eq!(hash_state(&t1), hash_state(&t2));
    }
}
