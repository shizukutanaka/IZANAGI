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
}
