//! Input buffer with hold-detection and key-repeat for roguelike input.
//!
//! `InputBuffer<K>` tracks which keys are currently held and for how many
//! ticks. It drives the standard roguelike hold-to-repeat pattern:
//!
//! 1. On the first tick a key is pressed, it fires immediately (initial press).
//! 2. After `initial_delay` ticks of continuous hold, it begins repeating every
//!    `repeat_period` ticks.
//!
//! The caller feeds raw "key down" / "key up" events via `press` and `release`,
//! then calls `tick(n)` once per simulation step to advance time and collect
//! the list of keys that fire this tick (initial presses + repeats).
//!
//! All timing is in simulation ticks (integers). No OS clock, no float.
//!
//! `DetHash` (gated on `K: DetHash + Ord`) folds all held-key states in
//! canonical key order so the buffer participates in world-hash / replay checks.

use crate::world_hash::{DetHash, Fnv1a};

/// State for a single held key.
#[derive(Clone, Debug)]
struct HeldKey<K> {
    key: K,
    held_ticks: u32,
    fired_initial: bool,
}

/// Input buffer with configurable hold-to-repeat timing.
///
/// `initial_delay`: ticks before repeat begins after the initial press.
/// `repeat_period`: ticks between repeat fires once repeating.
#[derive(Clone, Debug)]
pub struct InputBuffer<K> {
    held: Vec<HeldKey<K>>,
    initial_delay: u32,
    repeat_period: u32,
}

impl<K: Eq + Clone> InputBuffer<K> {
    /// Create a buffer with the given hold timing.
    ///
    /// `initial_delay = 0` means repeat fires on the same tick as the initial
    /// press. `repeat_period = 0` is clamped to 1 (avoids infinite firing).
    pub fn new(initial_delay: u32, repeat_period: u32) -> Self {
        InputBuffer {
            held: Vec::new(),
            initial_delay,
            repeat_period: repeat_period.max(1),
        }
    }

    /// Register a key-down event.  If the key is already held this is a no-op
    /// (de-bouncing: duplicate presses don't reset the hold counter).
    pub fn press(&mut self, key: K) {
        if self.held.iter().any(|h| h.key == key) {
            return;
        }
        self.held.push(HeldKey {
            key,
            held_ticks: 0,
            fired_initial: false,
        });
    }

    /// Register a key-up event.  Unknown keys are silently ignored.
    pub fn release(&mut self, key: &K) {
        self.held.retain(|h| &h.key != key);
    }

    /// True if `key` is currently held.
    pub fn is_held(&self, key: &K) -> bool {
        self.held.iter().any(|h| &h.key == key)
    }

    /// Advance time by `ticks` and return all keys that fire this step.
    ///
    /// A key fires on the tick it was pressed (initial press) and then again
    /// every `repeat_period` ticks after `initial_delay` ticks of holding.
    pub fn tick(&mut self, ticks: u32) -> Vec<K> {
        let mut fired: Vec<K> = Vec::new();
        for h in &mut self.held {
            // Initial press fires immediately (held_ticks == 0 before increment).
            if !h.fired_initial {
                fired.push(h.key.clone());
                h.fired_initial = true;
                h.held_ticks = h.held_ticks.saturating_add(ticks);
                continue;
            }

            h.held_ticks = h.held_ticks.saturating_add(ticks);

            // Check how many repeats fall in this tick window.
            if h.held_ticks > self.initial_delay {
                let repeat_ticks = h.held_ticks - self.initial_delay;
                let prev_ticks = repeat_ticks.saturating_sub(ticks);
                // Number of repeats fired so far (before this tick).
                let prev_count = prev_ticks / self.repeat_period;
                let new_count = repeat_ticks / self.repeat_period;
                for _ in prev_count..new_count {
                    fired.push(h.key.clone());
                }
            }
        }
        fired
    }

    /// Drop all held keys (e.g. on focus loss).
    pub fn clear(&mut self) {
        self.held.clear();
    }

    /// Number of currently held keys.
    pub fn held_count(&self) -> usize {
        self.held.len()
    }

    /// Ticks the given key has been held for, or `None` if not held.
    pub fn held_ticks(&self, key: &K) -> Option<u32> {
        self.held
            .iter()
            .find(|h| &h.key == key)
            .map(|h| h.held_ticks)
    }

    /// Iterate all currently held key values. Useful for modifier queries:
    /// "is Shift / Ctrl held while I process another key event?"
    pub fn all_held(&self) -> impl Iterator<Item = &K> {
        self.held.iter().map(|h| &h.key)
    }
}

impl<K: Eq + Clone + Ord + DetHash> DetHash for InputBuffer<K> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        // Sort by key for canonical order.
        let mut sorted: Vec<&HeldKey<K>> = self.held.iter().collect();
        sorted.sort_by(|a, b| a.key.cmp(&b.key));
        hasher.write_u32(sorted.len() as u32);
        hasher.write_u32(self.initial_delay);
        hasher.write_u32(self.repeat_period);
        for h in sorted {
            h.key.det_hash(hasher);
            hasher.write_u32(h.held_ticks);
            hasher.write_u32(h.fired_initial as u32);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    fn buf() -> InputBuffer<u32> {
        InputBuffer::new(3, 2)
    }

    #[test]
    fn test_press_fires_immediately() {
        let mut b = buf();
        b.press(1);
        let fired = b.tick(1);
        assert_eq!(fired, vec![1]);
    }

    #[test]
    fn test_release_stops_firing() {
        let mut b = buf();
        b.press(1);
        b.tick(1); // initial fire
        b.release(&1);
        let fired = b.tick(1);
        assert!(fired.is_empty());
    }

    #[test]
    fn test_no_repeat_before_initial_delay() {
        let mut b = buf(); // initial_delay=3
        b.press(1);
        b.tick(1); // initial fire
        let f2 = b.tick(1);
        let f3 = b.tick(1);
        assert!(f2.is_empty());
        assert!(f3.is_empty());
    }

    #[test]
    fn test_repeat_fires_after_delay() {
        let mut b = buf(); // initial_delay=3, repeat_period=2
        b.press(1);
        b.tick(1); // tick 1: initial fire, held_ticks=1
        b.tick(1); // tick 2
        b.tick(1); // tick 3
        let f = b.tick(1); // tick 4: held_ticks=4, repeat_ticks=1, prev=0, new=0 → no
        assert!(f.is_empty());
        let f = b.tick(1); // tick 5: held_ticks=5, repeat_ticks=2, prev=0, new=1 → fire
        assert_eq!(f, vec![1]);
        let f = b.tick(1); // tick 6: held_ticks=6, repeat_ticks=3, prev=1, new=1 → no
        assert!(f.is_empty());
        let f = b.tick(1); // tick 7: held_ticks=7, repeat_ticks=4, prev=1, new=2 → fire
        assert_eq!(f, vec![1]);
    }

    #[test]
    fn test_duplicate_press_is_noop() {
        let mut b = buf();
        b.press(1);
        b.press(1); // duplicate — should not reset
        b.tick(1); // initial fire
        assert_eq!(b.held_count(), 1);
    }

    #[test]
    fn test_release_unknown_key_is_noop() {
        let mut b: InputBuffer<u32> = InputBuffer::new(0, 1);
        b.release(&99); // should not panic
        assert_eq!(b.held_count(), 0);
    }

    #[test]
    fn test_is_held() {
        let mut b = buf();
        assert!(!b.is_held(&1));
        b.press(1);
        assert!(b.is_held(&1));
        b.release(&1);
        assert!(!b.is_held(&1));
    }

    #[test]
    fn test_clear_drops_all_keys() {
        let mut b = buf();
        b.press(1);
        b.press(2);
        b.clear();
        assert_eq!(b.held_count(), 0);
    }

    #[test]
    fn test_held_ticks_returns_duration() {
        let mut b = buf();
        b.press(1);
        b.tick(1);
        b.tick(3);
        assert_eq!(b.held_ticks(&1), Some(4));
    }

    #[test]
    fn test_held_ticks_unknown_key_returns_none() {
        let b = buf();
        assert_eq!(b.held_ticks(&99), None);
    }

    #[test]
    fn test_zero_initial_delay_repeats_immediately() {
        let mut b: InputBuffer<u32> = InputBuffer::new(0, 1);
        b.press(1);
        b.tick(1); // initial fire
        let f = b.tick(1); // repeat_ticks=1, new=1 → fire
        assert_eq!(f, vec![1]);
    }

    #[test]
    fn test_multiple_keys_fire_independently() {
        let mut b: InputBuffer<u32> = InputBuffer::new(0, 1);
        b.press(1);
        b.press(2);
        let fired = b.tick(1);
        assert_eq!(fired.len(), 2);
        assert!(fired.contains(&1));
        assert!(fired.contains(&2));
    }

    #[test]
    fn test_all_held_returns_held_keys() {
        let mut b: InputBuffer<u32> = InputBuffer::new(3, 2);
        b.press(1);
        b.press(3);
        let held: Vec<u32> = b.all_held().copied().collect();
        assert_eq!(held.len(), 2);
        assert!(held.contains(&1) && held.contains(&3));
    }

    #[test]
    fn test_all_held_empty_when_nothing_pressed() {
        let b: InputBuffer<u32> = InputBuffer::new(3, 2);
        assert_eq!(b.all_held().count(), 0);
    }

    #[test]
    fn test_det_hash_same_state_same_hash() {
        let mut a: InputBuffer<u32> = InputBuffer::new(3, 2);
        let mut b: InputBuffer<u32> = InputBuffer::new(3, 2);
        a.press(5);
        b.press(5);
        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_det_hash_different_state_different_hash() {
        let mut a: InputBuffer<u32> = InputBuffer::new(3, 2);
        let mut b: InputBuffer<u32> = InputBuffer::new(3, 2);
        a.press(1);
        b.press(2);
        assert_ne!(hash_state(&a), hash_state(&b));
    }
}
