//! Typed asset handle management.
//!
//! `AssetStore<T>` is a handle-indexed store of assets of type `T`. Handles
//! are opaque `AssetHandle<T>` values; inserting an asset yields a handle,
//! and that handle is the only way to retrieve, replace, or remove the asset.
//!
//! Handles are generational: after an asset is removed its slot can be reused
//! for a new asset, and the old (stale) handle will no longer resolve. This
//! prevents use-after-free bugs where old code holds a handle to a removed
//! asset and silently reads a different one's data.
//!
//! The design mirrors `EntityAllocator`/`SparseSet` but is typed by `T` so
//! different asset types (textures, sounds, tile definitions) live in separate
//! stores and handles can't be confused across types.
//!
//! `DetHash` (gated on `T: DetHash`) folds occupied handles + values in
//! ascending index order so the store participates in world/replay hashes.

use crate::world_hash::{DetHash, Fnv1a};
use std::marker::PhantomData;

/// Opaque handle to an asset of type `T` stored in `AssetStore<T>`.
#[derive(Debug)]
pub struct AssetHandle<T> {
    index: u32,
    generation: u32,
    _marker: PhantomData<fn() -> T>,
}

// Manual impls so T doesn't need to be Clone/Copy/PartialEq itself.
impl<T> Clone for AssetHandle<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for AssetHandle<T> {}
impl<T> PartialEq for AssetHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}
impl<T> Eq for AssetHandle<T> {}

impl<T> AssetHandle<T> {
    /// Internal index (for ordered iteration / hashing).
    #[inline]
    pub fn index(self) -> u32 {
        self.index
    }
}

struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

/// Generational handle-indexed asset store.
pub struct AssetStore<T> {
    slots: Vec<Slot<T>>,
    free: Vec<u32>,
}

impl<T> Default for AssetStore<T> {
    fn default() -> Self {
        AssetStore {
            slots: Vec::new(),
            free: Vec::new(),
        }
    }
}

impl<T> AssetStore<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert `asset` and return a handle. O(1) amortised.
    pub fn insert(&mut self, asset: T) -> AssetHandle<T> {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.value = Some(asset);
            AssetHandle {
                index,
                generation: slot.generation,
                _marker: PhantomData,
            }
        } else {
            let index = self.slots.len() as u32;
            self.slots.push(Slot {
                generation: 0,
                value: Some(asset),
            });
            AssetHandle {
                index,
                generation: 0,
                _marker: PhantomData,
            }
        }
    }

    /// Get a reference to the asset, or `None` if the handle is stale/invalid.
    pub fn get(&self, handle: AssetHandle<T>) -> Option<&T> {
        let slot = self.slots.get(handle.index as usize)?;
        if slot.generation == handle.generation {
            slot.value.as_ref()
        } else {
            None
        }
    }

    /// Get a mutable reference to the asset.
    pub fn get_mut(&mut self, handle: AssetHandle<T>) -> Option<&mut T> {
        let slot = self.slots.get_mut(handle.index as usize)?;
        if slot.generation == handle.generation {
            slot.value.as_mut()
        } else {
            None
        }
    }

    /// Replace the asset at `handle`, returning the old value.
    /// Returns `None` if the handle is stale/invalid.
    pub fn replace(&mut self, handle: AssetHandle<T>, new_asset: T) -> Option<T> {
        let slot = self.slots.get_mut(handle.index as usize)?;
        if slot.generation != handle.generation {
            return None;
        }
        slot.value.replace(new_asset)
    }

    /// Remove the asset at `handle`. Returns `None` for a stale handle.
    /// The slot's generation is bumped so the handle is permanently invalidated.
    pub fn remove(&mut self, handle: AssetHandle<T>) -> Option<T> {
        let slot = self.slots.get_mut(handle.index as usize)?;
        if slot.generation != handle.generation {
            return None;
        }
        slot.generation = slot.generation.wrapping_add(1);
        let val = slot.value.take();
        self.free.push(handle.index);
        val
    }

    /// True if `handle` still refers to a live asset.
    pub fn is_live(&self, handle: AssetHandle<T>) -> bool {
        self.slots
            .get(handle.index as usize)
            .is_some_and(|s| s.generation == handle.generation && s.value.is_some())
    }

    /// Number of currently live assets.
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    pub fn is_empty(&self) -> bool {
        self.iter().next().is_none()
    }

    /// Iterate `(handle, &asset)` for all live assets in ascending index order.
    pub fn iter(&self) -> impl Iterator<Item = (AssetHandle<T>, &T)> {
        self.slots.iter().enumerate().filter_map(|(i, s)| {
            s.value.as_ref().map(|v| {
                (
                    AssetHandle {
                        index: i as u32,
                        generation: s.generation,
                        _marker: PhantomData,
                    },
                    v,
                )
            })
        })
    }

    /// Iterate `(handle, &mut asset)` for all live assets in ascending index order.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (AssetHandle<T>, &mut T)> + '_ {
        self.slots.iter_mut().enumerate().filter_map(|(i, s)| {
            let gen = s.generation;
            s.value.as_mut().map(|v| {
                (
                    AssetHandle {
                        index: i as u32,
                        generation: gen,
                        _marker: PhantomData,
                    },
                    v,
                )
            })
        })
    }

    /// Remove all assets. All existing handles are permanently invalidated.
    pub fn clear(&mut self) {
        for slot in &mut self.slots {
            if slot.value.take().is_some() {
                slot.generation = slot.generation.wrapping_add(1);
            }
        }
        self.free.clear();
        self.free.extend(0..self.slots.len() as u32);
    }

    /// Return the handle of the first live asset for which `pred(&asset)` is
    /// `true`, searching in ascending index order. Returns `None` if no match.
    pub fn find_by<F: Fn(&T) -> bool>(&self, pred: F) -> Option<AssetHandle<T>> {
        self.iter().find(|(_, asset)| pred(asset)).map(|(h, _)| h)
    }

    /// Return handles of **all** live assets for which `pred(&asset)` is
    /// `true`, in ascending index order. Returns an empty `Vec` when no
    /// assets match or the store is empty. The plural companion to `find_by`
    /// — use when multiple assets can satisfy a condition (e.g. "all loaded
    /// textures for layer 3") and you need every handle, not just the first.
    pub fn find_all_by<F: Fn(&T) -> bool>(&self, pred: F) -> Vec<AssetHandle<T>> {
        self.iter()
            .filter(|(_, asset)| pred(asset))
            .map(|(h, _)| h)
            .collect()
    }

    /// Iterate over live asset handles without their values.
    /// Useful when you need a list of handles for bulk operations (remove, pass
    /// to another system) without borrowing the asset values at the same time.
    pub fn handles(&self) -> impl Iterator<Item = AssetHandle<T>> + '_ {
        self.iter().map(|(h, _)| h)
    }

    /// Remove all assets for which `pred(&asset)` returns `true`. Returns the
    /// count of assets removed. Unlike `retain` this takes only the value (no
    /// handle) so it is simpler for "remove all expired items" patterns.
    pub fn remove_where<F: Fn(&T) -> bool>(&mut self, pred: F) -> usize {
        let to_remove = self.find_all_by(pred);
        let count = to_remove.len();
        for handle in to_remove {
            self.remove(handle);
        }
        count
    }

    /// Retain only assets for which `pred(handle, &asset)` returns `true`.
    /// Assets failing the predicate are removed and their handles invalidated.
    pub fn retain<F: Fn(AssetHandle<T>, &T) -> bool>(&mut self, pred: F) {
        let to_remove: Vec<AssetHandle<T>> = self
            .iter()
            .filter(|&(h, asset)| !pred(h, asset))
            .map(|(h, _)| h)
            .collect();
        for handle in to_remove {
            self.remove(handle);
        }
    }

    /// Count live assets for which `pred(&asset)` returns `true`. Allocation-
    /// free alternative to `find_all_by(pred).len()` — avoids building the
    /// handle `Vec` when only the count is needed.
    pub fn count_by<F: Fn(&T) -> bool>(&self, pred: F) -> usize {
        self.iter().filter(|(_, a)| pred(a)).count()
    }

    /// Whether any live asset satisfies `pred`. Short-circuits on the first
    /// match. Lighter than `find_by(pred).is_some()` (same semantics).
    pub fn any_by<F: Fn(&T) -> bool>(&self, pred: F) -> bool {
        self.iter().any(|(_, a)| pred(a))
    }
}

impl<T: DetHash> DetHash for AssetStore<T> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.len() as u32);
        for (handle, asset) in self.iter() {
            hasher.write_u32(handle.index);
            hasher.write_u32(handle.generation);
            asset.det_hash(hasher);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[test]
    fn test_insert_and_get() {
        let mut s: AssetStore<u32> = AssetStore::new();
        let h = s.insert(42);
        assert_eq!(s.get(h), Some(&42));
    }

    #[test]
    fn test_remove_invalidates_handle() {
        let mut s: AssetStore<u32> = AssetStore::new();
        let h = s.insert(10);
        assert_eq!(s.remove(h), Some(10));
        assert_eq!(s.get(h), None);
        assert!(!s.is_live(h));
    }

    #[test]
    fn test_stale_handle_after_slot_reuse() {
        let mut s: AssetStore<u32> = AssetStore::new();
        let h0 = s.insert(1);
        s.remove(h0);
        let h1 = s.insert(2); // reuses slot 0
        assert_eq!(h1.index, 0);
        assert_eq!(s.get(h0), None); // h0 is stale
        assert_eq!(s.get(h1), Some(&2));
    }

    #[test]
    fn test_get_mut() {
        let mut s: AssetStore<u32> = AssetStore::new();
        let h = s.insert(5);
        *s.get_mut(h).unwrap() = 99;
        assert_eq!(s.get(h), Some(&99));
    }

    #[test]
    fn test_replace() {
        let mut s: AssetStore<u32> = AssetStore::new();
        let h = s.insert(7);
        let old = s.replace(h, 77);
        assert_eq!(old, Some(7));
        assert_eq!(s.get(h), Some(&77));
    }

    #[test]
    fn test_replace_stale_handle_returns_none() {
        let mut s: AssetStore<u32> = AssetStore::new();
        let h = s.insert(1);
        s.remove(h);
        assert_eq!(s.replace(h, 99), None);
    }

    #[test]
    fn test_is_live() {
        let mut s: AssetStore<u32> = AssetStore::new();
        let h = s.insert(1);
        assert!(s.is_live(h));
        s.remove(h);
        assert!(!s.is_live(h));
    }

    #[test]
    fn test_len() {
        let mut s: AssetStore<u32> = AssetStore::new();
        let h0 = s.insert(1);
        let _h1 = s.insert(2);
        assert_eq!(s.len(), 2);
        s.remove(h0);
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_is_empty_on_new() {
        let s: AssetStore<u32> = AssetStore::new();
        assert!(s.is_empty());
    }

    #[test]
    fn test_iter_ascending_index_order() {
        let mut s: AssetStore<u32> = AssetStore::new();
        let h0 = s.insert(10);
        let h1 = s.insert(20);
        let items: Vec<u32> = s.iter().map(|(_, &v)| v).collect();
        assert_eq!(items, vec![10, 20]);
        let _ = (h0, h1);
    }

    #[test]
    fn test_multiple_independent_handles() {
        let mut s: AssetStore<u32> = AssetStore::new();
        let h0 = s.insert(1);
        let h1 = s.insert(2);
        let h2 = s.insert(3);
        assert_ne!(h0, h1);
        assert_ne!(h1, h2);
        assert_eq!(s.get(h0), Some(&1));
        assert_eq!(s.get(h1), Some(&2));
        assert_eq!(s.get(h2), Some(&3));
    }

    #[test]
    fn test_iter_mut_modifies_assets() {
        let mut s: AssetStore<u32> = AssetStore::new();
        let h0 = s.insert(1);
        let h1 = s.insert(2);
        for (_, v) in s.iter_mut() {
            *v *= 10;
        }
        assert_eq!(s.get(h0), Some(&10));
        assert_eq!(s.get(h1), Some(&20));
    }

    #[test]
    fn test_iter_mut_empty_store_is_noop() {
        let mut s: AssetStore<u32> = AssetStore::new();
        assert_eq!(s.iter_mut().count(), 0);
    }

    #[test]
    fn test_retain_keeps_matching() {
        let mut s: AssetStore<u32> = AssetStore::new();
        let h0 = s.insert(1);
        let h1 = s.insert(2);
        let h2 = s.insert(3);
        s.retain(|_, &v| v > 1);
        assert_eq!(s.get(h0), None);
        assert_eq!(s.get(h1), Some(&2));
        assert_eq!(s.get(h2), Some(&3));
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn test_retain_false_removes_all() {
        let mut s: AssetStore<u32> = AssetStore::new();
        s.insert(1);
        s.insert(2);
        s.retain(|_, _| false);
        assert!(s.is_empty());
    }

    #[test]
    fn test_retain_true_keeps_all() {
        let mut s: AssetStore<u32> = AssetStore::new();
        s.insert(1);
        s.insert(2);
        s.retain(|_, _| true);
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn test_det_hash_same_content_same_hash() {
        let mut a: AssetStore<u32> = AssetStore::new();
        let mut b: AssetStore<u32> = AssetStore::new();
        a.insert(42);
        b.insert(42);
        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_det_hash_differs_after_change() {
        let mut a: AssetStore<u32> = AssetStore::new();
        let mut b: AssetStore<u32> = AssetStore::new();
        let h = a.insert(1);
        b.insert(2);
        let _ = h;
        assert_ne!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_clear_removes_all_assets() {
        let mut s: AssetStore<u32> = AssetStore::new();
        s.insert(1);
        s.insert(2);
        s.insert(3);
        s.clear();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_clear_invalidates_existing_handles() {
        let mut s: AssetStore<u32> = AssetStore::new();
        let h = s.insert(42);
        s.clear();
        assert_eq!(s.get(h), None);
    }

    #[test]
    fn test_find_by_returns_matching_handle() {
        let mut s: AssetStore<u32> = AssetStore::new();
        s.insert(10);
        let h = s.insert(20);
        s.insert(30);
        let found = s.find_by(|&v| v == 20).unwrap();
        assert_eq!(found.index(), h.index());
    }

    #[test]
    fn test_find_by_no_match_returns_none() {
        let mut s: AssetStore<u32> = AssetStore::new();
        s.insert(1);
        s.insert(2);
        assert!(s.find_by(|&v| v == 99).is_none());
    }

    #[test]
    fn test_handles_returns_all_live_handles() {
        let mut s: AssetStore<u32> = AssetStore::new();
        let h0 = s.insert(10);
        let h1 = s.insert(20);
        let h2 = s.insert(30);
        let handles: Vec<_> = s.handles().collect();
        assert_eq!(handles.len(), 3);
        assert!(handles.iter().any(|h| h.index() == h0.index()));
        assert!(handles.iter().any(|h| h.index() == h1.index()));
        assert!(handles.iter().any(|h| h.index() == h2.index()));
    }

    #[test]
    fn test_handles_excludes_removed() {
        let mut s: AssetStore<u32> = AssetStore::new();
        let h0 = s.insert(1);
        let h1 = s.insert(2);
        s.remove(h0);
        let handles: Vec<_> = s.handles().collect();
        assert_eq!(handles.len(), 1);
        assert_eq!(handles[0].index(), h1.index());
    }

    #[test]
    fn test_handles_empty_store_returns_empty() {
        let s: AssetStore<u32> = AssetStore::new();
        assert!(s.handles().next().is_none());
    }

    #[test]
    fn test_find_all_by_no_match_returns_empty() {
        let mut s: AssetStore<u32> = AssetStore::new();
        s.insert(10);
        s.insert(20);
        let handles = s.find_all_by(|&v| v == 99);
        assert!(handles.is_empty());
    }

    #[test]
    fn test_find_all_by_returns_all_matching() {
        let mut s: AssetStore<u32> = AssetStore::new();
        let h1 = s.insert(5);
        s.insert(99);
        let h3 = s.insert(5);
        let handles = s.find_all_by(|&v| v == 5);
        assert_eq!(handles.len(), 2);
        assert!(handles.contains(&h1));
        assert!(handles.contains(&h3));
    }

    #[test]
    fn test_find_all_by_empty_store_returns_empty() {
        let s: AssetStore<u32> = AssetStore::new();
        assert!(s.find_all_by(|_| true).is_empty());
    }

    #[test]
    fn test_remove_where_removes_matching() {
        let mut s: AssetStore<u32> = AssetStore::new();
        s.insert(1);
        s.insert(2);
        s.insert(3);
        let removed = s.remove_where(|&v| v % 2 == 0);
        assert_eq!(removed, 1);
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn test_remove_where_none_match_returns_zero() {
        let mut s: AssetStore<u32> = AssetStore::new();
        s.insert(1);
        s.insert(3);
        let removed = s.remove_where(|&v| v == 99);
        assert_eq!(removed, 0);
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn test_remove_where_all_match_empties_store() {
        let mut s: AssetStore<u32> = AssetStore::new();
        s.insert(10);
        s.insert(20);
        let removed = s.remove_where(|_| true);
        assert_eq!(removed, 2);
        assert!(s.is_empty());
    }

    #[test]
    fn test_count_by_returns_matching_count() {
        let mut s: AssetStore<u32> = AssetStore::new();
        s.insert(1);
        s.insert(2);
        s.insert(3);
        assert_eq!(s.count_by(|&v| v > 1), 2);
    }

    #[test]
    fn test_count_by_empty_store_returns_zero() {
        let s: AssetStore<u32> = AssetStore::new();
        assert_eq!(s.count_by(|_| true), 0);
    }

    #[test]
    fn test_any_by_true_when_match_exists() {
        let mut s: AssetStore<u32> = AssetStore::new();
        s.insert(5);
        s.insert(10);
        assert!(s.any_by(|&v| v == 10));
    }

    #[test]
    fn test_any_by_false_when_no_match() {
        let mut s: AssetStore<u32> = AssetStore::new();
        s.insert(1);
        assert!(!s.any_by(|&v| v == 99));
    }

    #[test]
    fn test_any_by_false_on_empty_store() {
        let s: AssetStore<u32> = AssetStore::new();
        assert!(!s.any_by(|_| true));
    }
}
