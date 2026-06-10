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

    /// Remove all active effects immediately.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Magnitude of the effect at `key`, or `0` if not active.
    ///
    /// Shorthand for `get(key).map_or(0, |e| e.magnitude)`.
    #[inline]
    pub fn magnitude_of(&self, key: &K) -> i32 {
        self.get(key).map_or(0, |e| e.magnitude)
    }

    /// Remaining duration of the effect at `key`, or `0` if not active.
    ///
    /// Shorthand for `get(key).map_or(0, |e| e.remaining)`.
    #[inline]
    pub fn remaining_of(&self, key: &K) -> u32 {
        self.get(key).map_or(0, |e| e.remaining)
    }

    /// The longest remaining duration across all active effects.
    /// Returns `0` when no effects are active.
    pub fn max_remaining(&self) -> u32 {
        self.entries
            .iter()
            .map(|(_, e)| e.remaining)
            .max()
            .unwrap_or(0)
    }

    /// The shortest remaining duration across all active effects.
    /// Returns `0` when no effects are active.
    ///
    /// Mirrors [`max_remaining`](Self::max_remaining). Useful for "how soon
    /// will a buff wear off?" or "apply debuff only if its duration exceeds the
    /// shortest existing effect" queries.
    pub fn min_remaining(&self) -> u32 {
        self.entries
            .iter()
            .map(|(_, e)| e.remaining)
            .min()
            .unwrap_or(0)
    }

    /// Add `added_ticks` to the remaining duration of the effect keyed by `key`.
    /// No-op if the key is not currently active. Saturating on overflow.
    pub fn extend_duration(&mut self, key: &K, added_ticks: u32) {
        if let Some((_, e)) = self.entries.iter_mut().find(|(k, _)| k == key) {
            e.remaining = e.remaining.saturating_add(added_ticks);
        }
    }

    /// Count effects for which `pred(key, &effect)` returns `true`.
    pub fn count_with<F: Fn(&K, &Effect) -> bool>(&self, pred: F) -> usize {
        self.entries.iter().filter(|(k, e)| pred(k, e)).count()
    }

    /// The key and remaining ticks of the effect that will expire soonest
    /// (lowest `remaining`). Returns `None` when no effects are active.
    ///
    /// Useful for "time until next status change" UI and AI predictions.
    pub fn first_expiring(&self) -> Option<(&K, u32)> {
        self.entries
            .iter()
            .min_by_key(|(_, e)| e.remaining)
            .map(|(k, e)| (k, e.remaining))
    }

    /// The `(min, max)` magnitude across all active effects.
    /// Returns `(0, 0)` when no effects are active.
    ///
    /// Useful for "net buff range" queries and AI tuning: apply a debuff only
    /// if it would extend the current range below the current minimum.
    pub fn magnitude_range(&self) -> (i32, i32) {
        if self.entries.is_empty() {
            return (0, 0);
        }
        let min = self.entries.iter().map(|(_, e)| e.magnitude).min().unwrap();
        let max = self.entries.iter().map(|(_, e)| e.magnitude).max().unwrap();
        (min, max)
    }

    /// All keys that currently have an active effect, in application order.
    /// Returns an empty `Vec` when no effects are active.
    pub fn active_keys(&self) -> Vec<&K> {
        self.entries.iter().map(|(k, _)| k).collect()
    }

    /// Apply multiple effects at once. Equivalent to calling `apply` for each
    /// `(key, Effect)` pair in order. Useful for equipping items and area
    /// spells that apply several buffs/debuffs simultaneously.
    pub fn apply_all(&mut self, effects: &[(K, Effect)]) {
        for (k, e) in effects {
            self.apply(k.clone(), e.remaining, e.magnitude);
        }
    }

    /// Extend all currently active effects by `extra_ticks`. Saturating on
    /// overflow. No-op when no effects are active. Useful for a "haste" or
    /// "duration boost" mechanic that prolongs everything at once.
    pub fn extend_all(&mut self, extra_ticks: u32) {
        for (_, e) in &mut self.entries {
            e.remaining = e.remaining.saturating_add(extra_ticks);
        }
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

    #[test]
    fn test_clear_removes_all_effects() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 5, 10);
        s.apply(2, 3, -5);
        s.clear();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_magnitude_of_returns_zero_when_inactive() {
        let s: StatusSet<u32> = StatusSet::new();
        assert_eq!(s.magnitude_of(&99), 0);
    }

    #[test]
    fn test_magnitude_of_returns_correct_value() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 5, -7);
        assert_eq!(s.magnitude_of(&1), -7);
    }

    #[test]
    fn test_remaining_of_returns_zero_when_inactive() {
        let s: StatusSet<u32> = StatusSet::new();
        assert_eq!(s.remaining_of(&42), 0);
    }

    #[test]
    fn test_remaining_of_decrements_with_tick() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 8, 5);
        s.tick(3);
        assert_eq!(s.remaining_of(&1), 5);
    }

    #[test]
    fn test_max_remaining_empty_returns_zero() {
        let s: StatusSet<u32> = StatusSet::new();
        assert_eq!(s.max_remaining(), 0);
    }

    #[test]
    fn test_max_remaining_returns_longest() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 3, 1);
        s.apply(2, 10, 1);
        s.apply(3, 7, 1);
        assert_eq!(s.max_remaining(), 10);
    }

    #[test]
    fn test_max_remaining_after_tick() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 5, 1);
        s.apply(2, 2, 1);
        s.tick(3); // effect 2 expires
        assert_eq!(s.max_remaining(), 2); // 5 - 3 = 2
    }

    #[test]
    fn test_first_expiring_empty_returns_none() {
        let s: StatusSet<u32> = StatusSet::new();
        assert!(s.first_expiring().is_none());
    }

    #[test]
    fn test_first_expiring_returns_shortest() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 10, 1);
        s.apply(2, 2, 1);
        s.apply(3, 7, 1);
        let (k, remaining) = s.first_expiring().unwrap();
        assert_eq!(*k, 2);
        assert_eq!(remaining, 2);
    }

    #[test]
    fn test_first_expiring_single_effect() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(5, 4, 3);
        let (k, r) = s.first_expiring().unwrap();
        assert_eq!(*k, 5);
        assert_eq!(r, 4);
    }

    #[test]
    fn test_extend_duration_active_effect() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 5, 10);
        s.extend_duration(&1, 3);
        assert_eq!(s.remaining_of(&1), 8);
    }

    #[test]
    fn test_extend_duration_inactive_key_is_noop() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.extend_duration(&99, 10); // not active — no panic, no effect
        assert!(s.is_empty());
    }

    #[test]
    fn test_extend_duration_saturates() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, u32::MAX - 1, 1);
        s.extend_duration(&1, 10); // saturates
        assert_eq!(s.remaining_of(&1), u32::MAX);
    }

    #[test]
    fn test_count_with_matches_predicate() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 5, 10); // positive magnitude
        s.apply(2, 3, -5); // negative magnitude (debuff)
        s.apply(3, 7, 8); // positive magnitude
        let buffs = s.count_with(|_, e| e.magnitude > 0);
        assert_eq!(buffs, 2);
    }

    #[test]
    fn test_count_with_no_matches() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 5, 3);
        assert_eq!(s.count_with(|_, e| e.magnitude < 0), 0);
    }

    #[test]
    fn test_magnitude_range_empty_is_zero_zero() {
        let s: StatusSet<u32> = StatusSet::new();
        assert_eq!(s.magnitude_range(), (0, 0));
    }

    #[test]
    fn test_magnitude_range_single_effect() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 5, -7);
        assert_eq!(s.magnitude_range(), (-7, -7));
    }

    #[test]
    fn test_magnitude_range_mixed_effects() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 5, 10);
        s.apply(2, 3, -3);
        s.apply(3, 8, 5);
        assert_eq!(s.magnitude_range(), (-3, 10));
    }

    #[test]
    fn test_active_keys_empty_set() {
        let s: StatusSet<u32> = StatusSet::new();
        assert!(s.active_keys().is_empty());
    }

    #[test]
    fn test_active_keys_returns_all_keys() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 10, 5);
        s.apply(2, 5, -3);
        let keys = s.active_keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&&1u32));
        assert!(keys.contains(&&2u32));
    }

    #[test]
    fn test_active_keys_removes_expired() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 3, 0);
        s.apply(2, 10, 0);
        s.tick(3);
        let keys = s.active_keys();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], &2u32);
    }

    #[test]
    fn test_min_remaining_empty_returns_zero() {
        let s: StatusSet<u32> = StatusSet::new();
        assert_eq!(s.min_remaining(), 0);
    }

    #[test]
    fn test_min_remaining_returns_shortest() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 3, 1);
        s.apply(2, 10, 1);
        s.apply(3, 7, 1);
        assert_eq!(s.min_remaining(), 3);
    }

    #[test]
    fn test_min_remaining_after_tick() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 5, 1);
        s.apply(2, 8, 1);
        s.tick(2); // both still active: 3 and 6 remaining
        assert_eq!(s.min_remaining(), 3);
    }

    #[test]
    fn test_apply_all_inserts_all_effects() {
        let mut s: StatusSet<u32> = StatusSet::new();
        let effects = vec![
            (
                1u32,
                Effect {
                    remaining: 3,
                    magnitude: 5,
                },
            ),
            (
                2u32,
                Effect {
                    remaining: 5,
                    magnitude: -2,
                },
            ),
        ];
        s.apply_all(&effects);
        assert!(s.is_active(&1));
        assert!(s.is_active(&2));
        assert_eq!(s.get(&1).unwrap().magnitude, 5);
        assert_eq!(s.get(&2).unwrap().magnitude, -2);
    }

    #[test]
    fn test_apply_all_empty_slice_is_noop() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply_all(&[]);
        assert!(s.is_empty());
    }

    #[test]
    fn test_apply_all_uses_max_duration_on_existing_key() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 10, 5);
        let effects = vec![(
            1u32,
            Effect {
                remaining: 3,
                magnitude: 5,
            },
        )];
        s.apply_all(&effects);
        assert_eq!(s.get(&1).unwrap().remaining, 10); // max(10, 3)
    }

    #[test]
    fn test_extend_all_extends_every_effect() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, 3, 1);
        s.apply(2, 5, -1);
        s.extend_all(2);
        assert_eq!(s.get(&1).unwrap().remaining, 5);
        assert_eq!(s.get(&2).unwrap().remaining, 7);
    }

    #[test]
    fn test_extend_all_empty_is_noop() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.extend_all(100);
        assert!(s.is_empty());
    }

    #[test]
    fn test_extend_all_saturates_at_max() {
        let mut s: StatusSet<u32> = StatusSet::new();
        s.apply(1, u32::MAX, 0);
        s.extend_all(1);
        assert_eq!(s.get(&1).unwrap().remaining, u32::MAX);
    }
}
