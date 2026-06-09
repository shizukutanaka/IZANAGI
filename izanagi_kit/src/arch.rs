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

    /// Create an empty table pre-allocated for `n` entities, avoiding
    /// reallocation for the first `n` inserts.
    pub fn with_capacity(n: usize) -> Self {
        ArchTable {
            dense: Vec::with_capacity(n),
            index: HashMap::with_capacity(n),
        }
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

    /// Insert `entity` with `row` if not present; overwrite the existing row if
    /// already present. The upsert-on-move pattern (common in position systems):
    /// call `upsert(entity, new_pos)` instead of remove+insert.
    pub fn upsert(&mut self, entity: Entity, row: Row) {
        if let Some(&slot) = self.index.get(&entity) {
            self.dense[slot].1 = row;
        } else {
            let slot = self.dense.len();
            self.dense.push((entity, row));
            self.index.insert(entity, slot);
        }
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

    /// Retain only entries for which `keep(entity, row)` returns `true`;
    /// drop the rest. Iterates the dense array in reverse to amortise
    /// swap-removes, keeping one array pass without extra allocation.
    pub fn retain<F>(&mut self, mut keep: F)
    where
        F: FnMut(Entity, &Row) -> bool,
    {
        let mut i = self.dense.len();
        while i > 0 {
            i -= 1;
            let (e, ref r) = self.dense[i];
            if !keep(e, r) {
                let last = self.dense.len() - 1;
                if i != last {
                    self.dense.swap(i, last);
                    let moved = self.dense[i].0;
                    self.index.insert(moved, i);
                }
                self.index.remove(&e);
                self.dense.pop();
            }
        }
    }

    /// Iterate just the row values (no entity handles), in dense array order.
    pub fn values(&self) -> impl Iterator<Item = &Row> {
        self.dense.iter().map(|(_, r)| r)
    }

    /// Mutably iterate just the row values, in dense array order.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Row> {
        self.dense.iter_mut().map(|(_, r)| r)
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
    fn test_upsert_inserts_when_absent() {
        let mut a = alloc();
        let e = a.allocate();
        let mut table: ArchTable<Pos> = ArchTable::new();
        table.upsert(e, Pos { x: 1, y: 2 });
        assert_eq!(table.get(e), Some(&Pos { x: 1, y: 2 }));
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn test_upsert_overwrites_when_present() {
        let mut a = alloc();
        let e = a.allocate();
        let mut table: ArchTable<Pos> = ArchTable::new();
        table.insert(e, Pos { x: 0, y: 0 });
        table.upsert(e, Pos { x: 99, y: 99 });
        assert_eq!(table.get(e), Some(&Pos { x: 99, y: 99 }));
        assert_eq!(table.len(), 1); // no duplicate
    }

    #[test]
    fn test_upsert_does_not_duplicate() {
        let mut a = alloc();
        let e = a.allocate();
        let mut table: ArchTable<Pos> = ArchTable::new();
        for i in 0..5 {
            table.upsert(e, Pos { x: i, y: 0 });
        }
        assert_eq!(table.len(), 1);
        assert_eq!(table.get(e).unwrap().x, 4);
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
    fn test_retain_keeps_matching_entries() {
        let mut a = alloc();
        let mut table: ArchTable<Pos> = ArchTable::new();
        let e0 = a.allocate();
        let e1 = a.allocate();
        let e2 = a.allocate();
        table.insert(e0, Pos { x: 1, y: 0 });
        table.insert(e1, Pos { x: 2, y: 0 });
        table.insert(e2, Pos { x: 3, y: 0 });
        table.retain(|_, r| r.x != 2); // drop e1
        assert_eq!(table.len(), 2);
        assert!(!table.contains(e1));
        assert!(table.contains(e0));
        assert!(table.contains(e2));
    }

    #[test]
    fn test_retain_all_true_changes_nothing() {
        let mut a = alloc();
        let mut table: ArchTable<Pos> = ArchTable::new();
        let e = a.allocate();
        table.insert(e, Pos { x: 5, y: 5 });
        table.retain(|_, _| true);
        assert_eq!(table.len(), 1);
        assert!(table.contains(e));
    }

    #[test]
    fn test_retain_all_false_clears_table() {
        let mut a = alloc();
        let mut table: ArchTable<Pos> = ArchTable::new();
        for _ in 0..4 {
            let e = a.allocate();
            table.insert(e, Pos { x: 0, y: 0 });
        }
        table.retain(|_, _| false);
        assert!(table.is_empty());
    }

    #[test]
    fn test_values_and_values_mut() {
        let mut a = alloc();
        let mut table: ArchTable<Pos> = ArchTable::new();
        let e0 = a.allocate();
        let e1 = a.allocate();
        table.insert(e0, Pos { x: 1, y: 0 });
        table.insert(e1, Pos { x: 2, y: 0 });
        let xs: Vec<i32> = table.values().map(|r| r.x).collect();
        assert_eq!(xs.len(), 2);
        assert!(xs.contains(&1) && xs.contains(&2));
        for r in table.values_mut() {
            r.x *= 10;
        }
        let xs2: Vec<i32> = table.values().map(|r| r.x).collect();
        assert!(xs2.contains(&10) && xs2.contains(&20));
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

    #[test]
    fn test_with_capacity_starts_empty() {
        let table: ArchTable<Pos> = ArchTable::with_capacity(64);
        assert!(table.is_empty());
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn test_with_capacity_behaves_like_new() {
        let mut a = alloc();
        let e = a.allocate();
        let mut t1: ArchTable<Pos> = ArchTable::new();
        let mut t2: ArchTable<Pos> = ArchTable::with_capacity(10);
        t1.insert(e, Pos { x: 5, y: 7 });
        t2.insert(e, Pos { x: 5, y: 7 });
        assert_eq!(hash_state(&t1), hash_state(&t2));
    }

    #[test]
    fn test_with_capacity_zero_is_valid() {
        let table: ArchTable<Pos> = ArchTable::with_capacity(0);
        assert!(table.is_empty());
    }
}
