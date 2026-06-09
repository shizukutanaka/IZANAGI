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

    /// Release every currently held key — semantic alias for `clear()`, named
    /// for focus-loss handlers where the intent is "simulate a key-up event for
    /// every pressed key." Callers that want explicit key-up semantics should
    /// prefer this over `clear` for readability.
    pub fn release_all(&mut self) {
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

    /// Reset the hold counter for `key` to `0` without releasing it.
    ///
    /// The key stays held but the repeat timer restarts: the next repeat will
    /// not fire until `initial_delay + repeat_period` ticks have passed again.
    /// Useful for interrupts where the action should pause mid-repeat (e.g. a
    /// player jumps while a movement key is held). No-op if the key is not held.
    pub fn reset_hold(&mut self, key: &K) {
        if let Some(h) = self.held.iter_mut().find(|h| &h.key == key) {
            h.held_ticks = 0;
            h.fired_initial = false;
        }
    }

    /// True if `key` is held **and** has passed the `initial_delay` threshold —
    /// i.e. it is in the repeating phase rather than the initial-press phase.
    /// Returns `false` if the key is not held or has not yet reached repeat.
    pub fn is_repeating(&self, key: &K) -> bool {
        self.held
            .iter()
            .find(|h| &h.key == key)
            .map(|h| h.fired_initial && h.held_ticks > self.initial_delay)
            .unwrap_or(false)
    }

    /// Update the hold-repeat timing parameters without clearing the buffer.
    ///
    /// The new timing takes effect on the next `tick` call. Held keys are not
    /// released and their hold counters continue from where they were.
    /// `repeat_period` is clamped to at least 1.
    ///
    /// Useful for "haste" power-ups that halve repeat period, or accessibility
    /// settings that let players configure auto-repeat speed mid-session.
    pub fn set_timing(&mut self, initial_delay: u32, repeat_period: u32) {
        self.initial_delay = initial_delay;
        self.repeat_period = repeat_period.max(1);
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
    fn test_reset_hold_restarts_repeat_timer() {
        let mut b: InputBuffer<u32> = InputBuffer::new(0, 2);
        b.press(1);
        b.tick(1); // initial fire, held_ticks=1
        b.tick(1); // held_ticks=2, repeat fires
        b.reset_hold(&1); // restart timer
                          // Next tick is the new initial fire.
        let fired = b.tick(1);
        assert_eq!(fired, vec![1]); // re-fires as initial
    }

    #[test]
    fn test_reset_hold_noop_for_unheld_key() {
        let mut b = buf();
        b.reset_hold(&99); // should not panic
        assert_eq!(b.held_count(), 0);
    }

    #[test]
    fn test_reset_hold_keeps_key_held() {
        let mut b = buf();
        b.press(5u32);
        b.tick(1);
        b.reset_hold(&5);
        assert!(b.is_held(&5)); // still held
        assert_eq!(b.held_ticks(&5), Some(0));
    }

    #[test]
    fn test_release_all_drops_all_held_keys() {
        let mut b = buf();
        b.press(1);
        b.press(2);
        b.tick(1);
        b.release_all();
        assert_eq!(b.held_count(), 0);
        assert!(!b.is_held(&1));
        assert!(!b.is_held(&2));
    }

    #[test]
    fn test_release_all_empty_is_noop() {
        let mut b: InputBuffer<u32> = InputBuffer::new(1, 2);
        b.release_all(); // no panic
        assert_eq!(b.held_count(), 0);
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

    #[test]
    fn test_set_timing_updates_delay_and_period() {
        let mut b: InputBuffer<u32> = InputBuffer::new(10, 5);
        b.set_timing(2, 1);
        // Verify the new period by observing faster repeats.
        b.press(1);
        b.tick(1); // initial fire
        let f = b.tick(1); // held_ticks=2, repeat_ticks=2-2=0, threshold crossed
                           // With initial_delay=2, repeat_period=1: repeat_ticks=0 → no repeat yet
                           // tick to 3: repeat_ticks=1, new=1 → fire
        let _ = f;
        let f2 = b.tick(1);
        assert_eq!(f2, vec![1]);
    }

    #[test]
    fn test_set_timing_keeps_held_keys() {
        let mut b: InputBuffer<u32> = InputBuffer::new(10, 5);
        b.press(7);
        b.tick(1);
        b.set_timing(2, 2);
        assert!(b.is_held(&7), "key must remain held after set_timing");
        assert_eq!(b.held_count(), 1);
    }

    #[test]
    fn test_set_timing_clamps_period_to_one() {
        let mut b: InputBuffer<u32> = InputBuffer::new(0, 1);
        b.set_timing(0, 0); // period 0 should become 1
        b.press(3u32);
        b.tick(1); // initial fire
                   // With repeat_period clamped to 1: held_ticks=1, repeat_ticks=1, new=1 → fire
        let f = b.tick(1);
        assert_eq!(f, vec![3]);
    }

    #[test]
    fn test_is_repeating_false_on_initial_press() {
        // initial_delay=3, repeat_period=2
        let mut b: InputBuffer<u32> = InputBuffer::new(3, 2);
        b.press(1u32);
        b.tick(1); // fires initial, held_ticks=1 ≤ initial_delay
        assert!(!b.is_repeating(&1u32), "not repeating yet");
    }

    #[test]
    fn test_is_repeating_true_after_delay() {
        let mut b: InputBuffer<u32> = InputBuffer::new(2, 1);
        b.press(5u32);
        b.tick(1); // initial fire, held_ticks=1
        b.tick(2); // held_ticks=3 > initial_delay(2) → repeat phase
        assert!(b.is_repeating(&5u32));
    }

    #[test]
    fn test_is_repeating_false_for_unheld_key() {
        let b: InputBuffer<u32> = InputBuffer::new(0, 1);
        assert!(!b.is_repeating(&99u32));
    }
}
