//! Slot-based inventory for roguelike items.
//!
//! `Inventory<T>` holds up to `capacity` items of type `T` in a fixed-size
//! array of optional slots. Items are added to the first free slot; removed
//! by slot index; retrieved by index or by a predicate. The slot layout is
//! stable — removing an item leaves a gap, which is the expected roguelike
//! model (item order reflects acquisition order, not compaction).
//!
//! The item type is generic so the caller defines what an item is (a struct,
//! an enum, or even an `Entity` reference into a separate component store).
//! `DetHash` is gated on `T: DetHash` and folds slot indices and values in
//! canonical order so the hash is independent of internal Vec layout.

use crate::world_hash::{DetHash, Fnv1a};

/// A fixed-capacity slot-based inventory.
#[derive(Clone, Debug)]
pub struct Inventory<T> {
    slots: Vec<Option<T>>,
}

impl<T: Clone> Inventory<T> {
    /// Create an empty inventory with the given capacity. All slots are empty.
    pub fn new(capacity: usize) -> Self {
        Inventory {
            slots: (0..capacity).map(|_| None).collect(),
        }
    }

    /// Total number of slots (including empty ones).
    #[inline]
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// Number of items currently held.
    pub fn len(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.slots.iter().all(|s| s.is_none())
    }

    /// Whether there is at least one free slot.
    pub fn has_space(&self) -> bool {
        self.slots.iter().any(|s| s.is_none())
    }

    /// Add `item` to the first free slot. Returns the slot index on success,
    /// or `None` if the inventory is full.
    pub fn add(&mut self, item: T) -> Option<usize> {
        if let Some((i, slot)) = self.slots.iter_mut().enumerate().find(|(_, s)| s.is_none()) {
            *slot = Some(item);
            Some(i)
        } else {
            None
        }
    }

    /// Remove and return the item at `slot`. Returns `None` if the slot is
    /// empty or out of bounds.
    pub fn remove(&mut self, slot: usize) -> Option<T> {
        self.slots.get_mut(slot)?.take()
    }

    /// Borrow the item at `slot`, or `None` if empty / out-of-bounds.
    pub fn get(&self, slot: usize) -> Option<&T> {
        self.slots.get(slot)?.as_ref()
    }

    /// Find the first slot for which `pred(item)` is true. Returns the slot
    /// index or `None` if no matching item is present.
    pub fn find<F: Fn(&T) -> bool>(&self, pred: F) -> Option<usize> {
        self.slots.iter().enumerate().find_map(|(i, s)| {
            s.as_ref()
                .and_then(|item| if pred(item) { Some(i) } else { None })
        })
    }

    /// Iterate `(slot_index, &item)` for all occupied slots in index order.
    pub fn iter(&self) -> impl Iterator<Item = (usize, &T)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.as_ref().map(|item| (i, item)))
    }

    /// Swap items at `a` and `b` (both may be empty — swapping two empty slots
    /// is a no-op). Out-of-bounds indices are silently clamped to the last slot.
    pub fn swap(&mut self, a: usize, b: usize) {
        let len = self.slots.len();
        if len == 0 {
            return;
        }
        let a = a.min(len - 1);
        let b = b.min(len - 1);
        self.slots.swap(a, b);
    }
}

impl<T: Clone + DetHash> DetHash for Inventory<T> {
    /// Folds `(slot_index, item)` pairs for occupied slots in ascending index
    /// order, plus the capacity, so two inventories with the same items at the
    /// same slots hash identically regardless of how they were filled.
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.slots.len() as u32);
        for (i, item) in self.iter() {
            hasher.write_u32(i as u32);
            item.det_hash(hasher);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[test]
    fn test_new_is_empty() {
        let inv: Inventory<u32> = Inventory::new(5);
        assert!(inv.is_empty());
        assert_eq!(inv.len(), 0);
        assert_eq!(inv.capacity(), 5);
    }

    #[test]
    fn test_add_returns_slot_index() {
        let mut inv: Inventory<u32> = Inventory::new(4);
        let s = inv.add(10).unwrap();
        assert_eq!(s, 0); // first free slot
        let s2 = inv.add(20).unwrap();
        assert_eq!(s2, 1);
    }

    #[test]
    fn test_add_when_full_returns_none() {
        let mut inv: Inventory<u32> = Inventory::new(2);
        inv.add(1);
        inv.add(2);
        assert_eq!(inv.add(3), None);
        assert!(!inv.has_space());
    }

    #[test]
    fn test_remove_clears_slot() {
        let mut inv: Inventory<u32> = Inventory::new(4);
        inv.add(42);
        let item = inv.remove(0);
        assert_eq!(item, Some(42));
        assert!(inv.is_empty());
    }

    #[test]
    fn test_remove_empty_slot_returns_none() {
        let mut inv: Inventory<u32> = Inventory::new(4);
        assert_eq!(inv.remove(0), None);
    }

    #[test]
    fn test_remove_out_of_bounds_returns_none() {
        let mut inv: Inventory<u32> = Inventory::new(2);
        assert_eq!(inv.remove(99), None);
    }

    #[test]
    fn test_get_returns_item() {
        let mut inv: Inventory<u32> = Inventory::new(4);
        inv.add(55);
        assert_eq!(inv.get(0), Some(&55));
        assert_eq!(inv.get(1), None);
    }

    #[test]
    fn test_find_returns_correct_slot() {
        let mut inv: Inventory<u32> = Inventory::new(4);
        inv.add(10);
        inv.add(20);
        inv.add(30);
        assert_eq!(inv.find(|&x| x == 20), Some(1));
        assert_eq!(inv.find(|&x| x == 99), None);
    }

    #[test]
    fn test_gap_after_remove_is_reused() {
        let mut inv: Inventory<u32> = Inventory::new(3);
        inv.add(1);
        inv.add(2);
        inv.remove(0);
        let s = inv.add(99).unwrap();
        assert_eq!(s, 0); // slot 0 is the first free slot again
    }

    #[test]
    fn test_iter_yields_occupied_in_order() {
        let mut inv: Inventory<u32> = Inventory::new(4);
        inv.add(10);
        inv.add(20);
        inv.remove(0);
        inv.add(30);
        let items: Vec<(usize, u32)> = inv.iter().map(|(i, &v)| (i, v)).collect();
        assert_eq!(items, [(0, 30), (1, 20)]);
    }

    #[test]
    fn test_swap_exchanges_items() {
        let mut inv: Inventory<u32> = Inventory::new(4);
        inv.add(1);
        inv.add(2);
        inv.swap(0, 1);
        assert_eq!(inv.get(0), Some(&2));
        assert_eq!(inv.get(1), Some(&1));
    }

    #[test]
    fn test_len_counts_occupied_slots() {
        let mut inv: Inventory<u32> = Inventory::new(4);
        inv.add(1);
        inv.add(2);
        inv.add(3);
        inv.remove(1);
        assert_eq!(inv.len(), 2);
    }

    #[test]
    fn test_det_hash_same_content_same_hash() {
        let mut a: Inventory<u32> = Inventory::new(4);
        let mut b: Inventory<u32> = Inventory::new(4);
        a.add(10);
        a.add(20);
        b.add(10);
        b.add(20);
        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_det_hash_after_remove_differs() {
        let mut a: Inventory<u32> = Inventory::new(4);
        let mut b: Inventory<u32> = Inventory::new(4);
        a.add(10);
        a.add(20);
        b.add(10);
        b.add(20);
        b.remove(0);
        assert_ne!(hash_state(&a), hash_state(&b));
    }
}
