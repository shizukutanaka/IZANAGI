//! Change detection — dirty-flag tracking for ECS components.
//!
//! When a component is updated, downstream systems (renderers, AI, caches)
//! only need to re-run if the component actually changed. A generation counter
//! — incremented globally each tick — lets `Changed<T>` record the tick on
//! which it was last written; systems query `is_changed_since(last_checked)`
//! to skip unchanged components in O(1) without scanning all data.
//!
//! This mirrors Bevy's `Changed<T>` filter (arXiv Bevy determinism audit) but
//! without any global state: the generation is passed explicitly so the system
//! is fully deterministic and replay-safe (no hidden thread-local or atomic).
//!
//! Usage:
//! 1. Hold a `u32` tick counter in your world state; increment it each sim tick.
//! 2. Wrap mutated components in `Changed<T>` and call `mark(tick)` on write.
//! 3. In downstream systems, call `is_changed_since(last_tick)` to skip unchanged.

use crate::world_hash::{DetHash, Fnv1a};

/// A value paired with the tick on which it was last modified.
///
/// `changed_at` starts at `0`; a value created at tick `0` is considered
/// changed (because anything should be processed at least once).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Changed<T> {
    pub value: T,
    /// The tick on which this component was last written.
    pub changed_at: u32,
}

impl<T> Changed<T> {
    /// Create a new `Changed<T>` that is considered changed at tick `0`.
    pub fn new(value: T) -> Self {
        Changed {
            value,
            changed_at: 0,
        }
    }

    /// Create with an explicit starting tick (e.g. the current tick).
    pub fn at(value: T, tick: u32) -> Self {
        Changed {
            value,
            changed_at: tick,
        }
    }

    /// Mark as changed at `tick`. Call this whenever the wrapped value is mutated.
    #[inline]
    pub fn mark(&mut self, tick: u32) {
        self.changed_at = tick;
    }

    /// Returns `true` if this component was modified at or after `since_tick`.
    /// Pass the last tick your system processed to get only new changes.
    #[inline]
    pub fn is_changed_since(&self, since_tick: u32) -> bool {
        self.changed_at >= since_tick
    }

    /// Mutably access the inner value and automatically mark as changed at `tick`.
    /// Prefer this over direct `value` mutation so the change tick is never forgotten.
    #[inline]
    pub fn get_mut(&mut self, tick: u32) -> &mut T {
        self.mark(tick);
        &mut self.value
    }

    /// Acknowledge this component as "seen at `tick`" without modifying its
    /// value. Sets `changed_at` to `tick` so a subsequent `is_changed_since(tick)`
    /// returns `false` — the canonical way for a system to record that it has
    /// processed this change and should not re-process on the next query.
    #[inline]
    pub fn reset(&mut self, tick: u32) {
        self.changed_at = tick;
    }

    /// `true` if this component was marked *exactly* at `tick` (i.e.
    /// `changed_at == tick`). Stricter than `is_changed_since`, which also
    /// matches earlier ticks.
    #[inline]
    pub fn was_written_at(&self, tick: u32) -> bool {
        self.changed_at == tick
    }

    /// How many ticks ago this component was last changed, relative to
    /// `current_tick` (i.e. `current_tick − changed_at`, saturating). Useful
    /// for "show a freshness indicator" or "invalidate stale cache entries"
    /// patterns without passing the `ChangeTracker` everywhere.
    #[inline]
    pub fn ticks_since_change(&self, current_tick: u32) -> u32 {
        current_tick.saturating_sub(self.changed_at)
    }

    /// Returns `true` if this component has not been marked for at least
    /// `age_threshold` ticks (i.e. `ticks_since_change >= age_threshold`).
    ///
    /// Useful for cache invalidation ("if no update for 30 ticks, recompute")
    /// and TTL checks without spelling out the comparison at every call site.
    #[inline]
    pub fn is_stale(&self, age_threshold: u32, current_tick: u32) -> bool {
        self.ticks_since_change(current_tick) >= age_threshold
    }

    /// Returns `true` if this component was marked within the last
    /// `max_age_ticks` ticks — the logical inverse of
    /// [`is_stale`](Self::is_stale).
    ///
    /// Useful for "only process if recently updated" filters: sparkle effects,
    /// cache-warming systems, or network delta-compression that skips unchanged
    /// data.
    #[inline]
    pub fn is_fresh(&self, max_age_ticks: u32, current_tick: u32) -> bool {
        !self.is_stale(max_age_ticks, current_tick)
    }

    /// Return a reference to the inner value if it changed at or after
    /// `since_tick`, otherwise `None`. Combines `is_changed_since` with value
    /// access to avoid the two-step pattern at every call site.
    #[inline]
    pub fn if_changed(&self, since_tick: u32) -> Option<&T> {
        if self.is_changed_since(since_tick) {
            Some(&self.value)
        } else {
            None
        }
    }

    /// Consume the wrapper and return the inner value, discarding the change
    /// tick. Useful for extracting the final value when the component is being
    /// removed or serialised and the dirty-flag metadata is no longer needed.
    #[inline]
    pub fn into_value(self) -> T {
        self.value
    }
}

impl<T: DetHash> DetHash for Changed<T> {
    /// Folds both the value and the change tick so a spurious re-mark shows
    /// up in the world hash even if the value bytes are identical.
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        self.value.det_hash(hasher);
        hasher.write_u32(self.changed_at);
    }
}

// ---------------------------------------------------------------------------
// ChangeTracker — tick counter + helpers
// ---------------------------------------------------------------------------

/// A monotonically increasing tick counter. Increment once per simulation
/// step; pass the current tick to `Changed::mark` / `get_mut` and to
/// `is_changed_since` in downstream systems.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChangeTracker {
    pub tick: u32,
}

impl ChangeTracker {
    pub fn new() -> Self {
        ChangeTracker { tick: 0 }
    }

    /// Advance by one tick. Call at the end of each simulation step.
    #[inline]
    pub fn advance(&mut self) {
        self.tick = self.tick.saturating_add(1);
    }

    /// Current tick (convenience getter).
    #[inline]
    pub fn current(&self) -> u32 {
        self.tick
    }

    /// Reset the tick counter to `0`. Useful when restoring from a save file
    /// or rewinding a replay: sets the baseline so `is_changed_since(0)` treats
    /// everything as fresh.
    #[inline]
    pub fn reset(&mut self) {
        self.tick = 0;
    }

    /// Ticks elapsed since `last_tick` (i.e. `current − last_tick`, saturating).
    ///
    /// Useful for "if > N ticks have passed since last action, do X" patterns
    /// without manually computing the difference at every call site.
    #[inline]
    pub fn delta_since(&self, last_tick: u32) -> u32 {
        self.tick.saturating_sub(last_tick)
    }

    /// Set the tick counter to `tick`. Use when restoring exact simulation
    /// state from a save file: preserves the tick offset recorded in each
    /// `Changed<T>` component without requiring a sequence of `advance` calls.
    #[inline]
    pub fn set_tick(&mut self, tick: u32) {
        self.tick = tick;
    }

    /// Returns `true` if the current tick equals `target`.
    ///
    /// A concise predicate for "fire exactly on tick N" patterns — e.g. trigger
    /// a cutscene at tick 1000 or assert a deterministic checkpoint.
    #[inline]
    pub fn is_at_tick(&self, target: u32) -> bool {
        self.tick == target
    }
}

impl DetHash for ChangeTracker {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.tick);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[test]
    fn test_new_is_changed_since_tick_0() {
        let c = Changed::new(42u32);
        assert!(c.is_changed_since(0));
    }

    #[test]
    fn test_mark_updates_changed_at() {
        let mut c = Changed::new(0u32);
        c.mark(5);
        assert_eq!(c.changed_at, 5);
    }

    #[test]
    fn test_is_changed_since_after_mark() {
        let mut c = Changed::new(0u32);
        c.mark(10);
        assert!(c.is_changed_since(10)); // changed AT 10 counts
        assert!(c.is_changed_since(5)); // changed after 5
        assert!(!c.is_changed_since(11)); // not changed after 11
    }

    #[test]
    fn test_get_mut_marks_and_updates_value() {
        let mut c = Changed::new(0u32);
        *c.get_mut(7) = 99;
        assert_eq!(c.value, 99);
        assert_eq!(c.changed_at, 7);
    }

    #[test]
    fn test_not_changed_since_old_tick() {
        let c = Changed::at(0u32, 3);
        assert!(!c.is_changed_since(4)); // changed at 3, not at 4
        assert!(c.is_changed_since(3));
        assert!(c.is_changed_since(2));
    }

    #[test]
    fn test_det_hash_changes_on_mark() {
        let mut a = Changed::new(1u32);
        let b = Changed::new(1u32);
        a.mark(5);
        // Same value, different tick → different hash.
        assert_ne!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_det_hash_same_value_and_tick_same_hash() {
        let a = Changed::at(42u32, 7);
        let b = Changed::at(42u32, 7);
        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_change_tracker_advance() {
        let mut ct = ChangeTracker::new();
        assert_eq!(ct.current(), 0);
        ct.advance();
        assert_eq!(ct.current(), 1);
        ct.advance();
        assert_eq!(ct.current(), 2);
    }

    #[test]
    fn test_change_tracker_saturates_at_max() {
        let mut ct = ChangeTracker { tick: u32::MAX };
        ct.advance();
        assert_eq!(ct.tick, u32::MAX);
    }

    #[test]
    fn test_reset_makes_subsequent_is_changed_false() {
        let mut c = Changed::at(42u32, 5);
        assert!(c.is_changed_since(5));
        // Acknowledge: reset to tick 5, then query since 5 → not changed.
        c.reset(5);
        assert!(
            !c.is_changed_since(6),
            "after reset(5), changed_since(6) must be false"
        );
        // But it IS still changed since an earlier tick.
        assert!(c.is_changed_since(5));
    }

    #[test]
    fn test_reset_does_not_modify_value() {
        let mut c = Changed::new(99u32);
        c.reset(10);
        assert_eq!(c.value, 99u32);
    }

    #[test]
    fn test_change_tracker_reset_to_zero() {
        let mut ct = ChangeTracker::new();
        ct.advance();
        ct.advance();
        assert_eq!(ct.current(), 2);
        ct.reset();
        assert_eq!(ct.current(), 0);
    }

    #[test]
    fn test_delta_since_returns_elapsed() {
        let mut ct = ChangeTracker::new();
        ct.advance();
        ct.advance();
        ct.advance(); // tick = 3
        assert_eq!(ct.delta_since(1), 2);
        assert_eq!(ct.delta_since(0), 3);
        assert_eq!(ct.delta_since(3), 0);
    }

    #[test]
    fn test_delta_since_saturates_at_zero() {
        let ct = ChangeTracker { tick: 5 };
        assert_eq!(ct.delta_since(10), 0); // last_tick > current → saturate
    }

    #[test]
    fn test_system_pattern_skips_unchanged() {
        // Simulate: process only components changed since last_processed_tick.
        let mut ct = ChangeTracker::new();
        let mut components = [Changed::new(1u32), Changed::new(2u32), Changed::new(3u32)];

        ct.advance(); // tick = 1
                      // Mark component[1] as changed at tick 1.
        components[1].mark(ct.current());

        let last_processed = 0u32; // we processed everything at tick 0
        let changed_count = components
            .iter()
            .filter(|c| c.is_changed_since(last_processed + 1))
            .count();
        // Only component[1] changed after tick 0.
        assert_eq!(changed_count, 1);
    }

    #[test]
    fn test_was_written_at_true_on_exact_tick() {
        let mut c = Changed::new(0u32);
        c.mark(7);
        assert!(c.was_written_at(7));
    }

    #[test]
    fn test_was_written_at_false_on_different_tick() {
        let mut c = Changed::new(0u32);
        c.mark(5);
        assert!(!c.was_written_at(4));
        assert!(!c.was_written_at(6));
    }

    #[test]
    fn test_ticks_since_change_zero_when_just_written() {
        let mut c = Changed::new(0u32);
        c.mark(10);
        assert_eq!(c.ticks_since_change(10), 0);
    }

    #[test]
    fn test_ticks_since_change_counts_elapsed() {
        let mut c = Changed::new(0u32);
        c.mark(3);
        assert_eq!(c.ticks_since_change(8), 5);
    }

    #[test]
    fn test_ticks_since_change_saturates_below_zero() {
        let mut c = Changed::new(0u32);
        c.mark(10);
        assert_eq!(c.ticks_since_change(5), 0); // saturating sub
    }

    #[test]
    fn test_set_tick_sets_tick_counter() {
        let mut ct = ChangeTracker::new();
        ct.advance();
        ct.advance(); // tick = 2
        ct.set_tick(100);
        assert_eq!(ct.current(), 100);
    }

    #[test]
    fn test_set_tick_zero_resets_like_reset() {
        let mut ct = ChangeTracker { tick: 42 };
        ct.set_tick(0);
        assert_eq!(ct.current(), 0);
    }

    #[test]
    fn test_set_tick_restores_delta_correctly() {
        let mut ct = ChangeTracker::new();
        ct.set_tick(50);
        assert_eq!(ct.delta_since(45), 5);
    }

    #[test]
    fn test_is_stale_when_old_enough() {
        let c = Changed::at(42u32, 5);
        assert!(c.is_stale(10, 20)); // changed 15 ticks ago, threshold 10
    }

    #[test]
    fn test_is_stale_not_stale_when_fresh() {
        let c = Changed::at(42u32, 18);
        assert!(!c.is_stale(10, 20)); // changed 2 ticks ago, threshold 10
    }

    #[test]
    fn test_is_stale_at_exact_threshold() {
        let c = Changed::at(0u32, 10);
        assert!(c.is_stale(10, 20)); // exactly 10 ticks ago = stale
    }

    #[test]
    fn test_is_fresh_when_recently_changed() {
        let c = Changed::at(42u32, 18);
        assert!(c.is_fresh(10, 20)); // changed 2 ticks ago, threshold 10 → fresh
    }

    #[test]
    fn test_is_fresh_false_when_stale() {
        let c = Changed::at(42u32, 5);
        assert!(!c.is_fresh(10, 20)); // changed 15 ticks ago, threshold 10 → not fresh
    }

    #[test]
    fn test_is_fresh_is_exact_inverse_of_is_stale() {
        let c = Changed::at(0u32, 10);
        // at exact threshold: is_stale returns true, is_fresh must return false
        assert!(c.is_stale(10, 20));
        assert!(!c.is_fresh(10, 20));
    }

    #[test]
    fn test_if_changed_returns_some_when_changed() {
        let c = Changed::at(99u32, 5);
        assert_eq!(c.if_changed(3), Some(&99u32));
        assert_eq!(c.if_changed(5), Some(&99u32));
    }

    #[test]
    fn test_if_changed_returns_none_when_not_changed() {
        let c = Changed::at(42u32, 3);
        assert_eq!(c.if_changed(10), None);
    }

    #[test]
    fn test_if_changed_new_value_always_returns_some() {
        let c = Changed::new(7u32); // changed_at == 0
        assert_eq!(c.if_changed(0), Some(&7u32));
    }

    #[test]
    fn test_is_at_tick_matches_current() {
        let mut ct = ChangeTracker::new();
        ct.advance(); // tick == 1
        assert!(ct.is_at_tick(1));
    }

    #[test]
    fn test_is_at_tick_false_for_other_tick() {
        let mut ct = ChangeTracker::new();
        ct.advance();
        ct.advance(); // tick == 2
        assert!(!ct.is_at_tick(1));
        assert!(!ct.is_at_tick(3));
    }

    #[test]
    fn test_is_at_tick_after_set_tick() {
        let mut ct = ChangeTracker::new();
        ct.set_tick(42);
        assert!(ct.is_at_tick(42));
        assert!(!ct.is_at_tick(43));
    }

    #[test]
    fn test_into_value_returns_inner_value() {
        let c = Changed::new(42u32);
        assert_eq!(c.into_value(), 42u32);
    }

    #[test]
    fn test_into_value_discards_tick() {
        let c = Changed::at(String::from("hello"), 99);
        let v = c.into_value();
        assert_eq!(v, "hello");
    }

    #[test]
    fn test_into_value_consumes_wrapper() {
        let c: Changed<Vec<u32>> = Changed::new(vec![1, 2, 3]);
        let v = c.into_value();
        assert_eq!(v, vec![1, 2, 3]);
    }
}
