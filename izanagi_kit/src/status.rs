//! Timed status effects — buff/debuff tracking for roguelike gameplay.
//!
//! A `StatusSet<K>` holds a collection of active effects keyed by a
//! caller-defined discriminant `K` (typically an enum). Each effect has a
//! remaining duration in ticks and a signed integer magnitude (positive for
//! buffs, negative for debuffs — e.g. +10 speed, −5 defense).
//!
//! `tick(n)` advances all active effects and removes expired ones.
//! Stacking policy: re-applying an existing key takes the maximum of the
//! current and new durations (does not reset to the new duration, and does
//! not add magnitudes). Callers that want different stacking semantics can
//! call `remove` + `apply` explicitly.
//!
//! No float, no OS clock. `DetHash` folds effects in sorted-key order so the
//! hash is canonical regardless of application order.

use crate::world_hash::{DetHash, Fnv1a};

/// A single active status effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Effect {
    /// Remaining duration in ticks. Decremented by `tick`. Removed at zero.
    pub remaining: u32,
    /// Signed magnitude (+buff / -debuff). Callers interpret this field.
    pub magnitude: i32,
}

/// A set of timed status effects keyed by `K`.
///
/// Internally stored as a `Vec` of `(K, Effect)` pairs — small enough for the
/// handful of effects typically active at once that a linear scan beats a
/// `HashMap` (no hashing, no heap reallocation on insert).
#[derive(Clone, Debug, Default)]
pub struct StatusSet<K> {
    entries: Vec<(K, Effect)>,
}

impl<K: Eq + Clone> StatusSet<K> {
    pub fn new() -> Self {
        StatusSet {
            entries: Vec::new(),
        }
    }

    /// Apply (or refresh) an effect. If the key already exists, the duration
    /// is extended to the maximum of the current and new values, and the
    /// magnitude is replaced with the new one. If not present, it is inserted.
    pub fn apply(&mut self, key: K, duration: u32, magnitude: i32) {
        if let Some((_, e)) = self.entries.iter_mut().find(|(k, _)| *k == key) {
            e.remaining = e.remaining.max(duration);
            e.magnitude = magnitude;
        } else {
            self.entries.push((
                key,
                Effect {
                    remaining: duration,
                    magnitude,
                },
            ));
        }
    }

    /// Remove an effect immediately. No-op if absent.
    pub fn remove(&mut self, key: &K) {
        self.entries.retain(|(k, _)| k != key);
    }

    /// Whether an effect with this key is currently active.
    pub fn is_active(&self, key: &K) -> bool {
        self.entries.iter().any(|(k, _)| k == key)
    }

    /// Get the effect for a key, if active.
    pub fn get(&self, key: &K) -> Option<&Effect> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, e)| e)
    }

    /// Advance all effects by `ticks`, removing any that expire. Returns the
    /// keys of effects that expired in this call (in order they appeared).
    pub fn tick(&mut self, ticks: u32) -> Vec<K> {
        let mut expired = Vec::new();
        self.entries.retain(|(k, e)| {
            if e.remaining <= ticks {
                expired.push(k.clone());
                false
            } else {
                true
            }
        });
        for (_, e) in &mut self.entries {
            e.remaining -= ticks;
        }
        expired
    }

    /// Number of currently active effects.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Sum of all active magnitudes. Useful for computing a net modifier.
    pub fn total_magnitude(&self) -> i32 {
        self.entries.iter().map(|(_, e)| e.magnitude).sum()
    }

    /// Iterate active effects in application order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &Effect)> {
        self.entries.iter().map(|(k, e)| (k, e))
    }
}

impl<K: Eq + Clone + Ord + DetHash> DetHash for StatusSet<K> {
    /// Folds effects in sorted-key order so the hash is canonical regardless
    /// of the order they were applied.
    fn det_hash(&self, hasher: &mut Fnv1a) {
        let mut ordered: Vec<&(K, Effect)> = self.entries.iter().collect();
        ordered.sort_by(|(a, _), (b, _)| a.cmp(b));
        hasher.write_u32(ordered.len() as u32);
        for (k, e) in ordered {
            k.det_hash(hasher);
            hasher.write_u32(e.remaining);
            hasher.write_i32(e.magnitude);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[test]
    fn test_apply_and_is_active() {
        let mut s: StatusSet<u32> = StatusSet::new();
        assert!(!s.is_active(&1));
        s.apply(1, 3, 10);
        assert!(s.is_active(&1));
    }

    #[test]
    fn test_tick_decrements_duration() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 5, 10);
        let expired = s.tick(2);
        assert!(expired.is_empty());
        assert_eq!(s.get(&1).unwrap().remaining, 3);
    }

    #[test]
    fn test_tick_removes_expired_and_returns_keys() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 2, 10);
        s.apply(2, 5, -5);
        let expired = s.tick(3); // effect 1 expires (2 <= 3)
        assert_eq!(expired, vec![1]);
        assert!(!s.is_active(&1));
        assert!(s.is_active(&2));
    }

    #[test]
    fn test_remove_is_immediate() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(7, 10, 3);
        s.remove(&7);
        assert!(!s.is_active(&7));
        assert!(s.is_empty());
    }

    #[test]
    fn test_apply_refresh_takes_max_duration() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 5, 10);
        s.apply(1, 3, 10); // shorter — should not shorten
        assert_eq!(s.get(&1).unwrap().remaining, 5);
        s.apply(1, 10, 10); // longer — should extend
        assert_eq!(s.get(&1).unwrap().remaining, 10);
    }

    #[test]
    fn test_total_magnitude_sums_all() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 5, 10);
        s.apply(2, 5, -3);
        s.apply(3, 5, 7);
        assert_eq!(s.total_magnitude(), 14);
    }

    #[test]
    fn test_len_and_is_empty() {
        let mut s: StatusSet<u32> = StatusSet::new();
        assert!(s.is_empty());
        s.apply(1, 5, 1);
        assert_eq!(s.len(), 1);
        s.apply(2, 5, 1);
        assert_eq!(s.len(), 2);
        s.remove(&1);
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_det_hash_canonical_regardless_of_apply_order() {
        let mut a: StatusSet<u32> = StatusSet::new();
        a.apply(2, 5, 1);
        a.apply(1, 3, 2);

        let mut b: StatusSet<u32> = StatusSet::new();
        b.apply(1, 3, 2);
        b.apply(2, 5, 1);

        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_det_hash_changes_after_tick() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 10, 5);
        let h1 = hash_state(&s);
        s.tick(1);
        let h2 = hash_state(&s);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_get_returns_correct_effect() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(42, 8, -15);
        let e = s.get(&42).unwrap();
        assert_eq!(e.remaining, 8);
        assert_eq!(e.magnitude, -15);
    }

    #[test]
    fn test_tick_zero_changes_nothing() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 5, 10);
        let expired = s.tick(0);
        assert!(expired.is_empty());
        assert_eq!(s.get(&1).unwrap().remaining, 5);
    }
}
