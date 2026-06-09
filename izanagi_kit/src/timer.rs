//! Deterministic tick-based timers and cooldowns.
//!
//! Time is measured in **ticks** (integers) rather than wall-clock seconds so
//! the scheduler is fully deterministic and replay-safe — there is no float,
//! no OS clock, and no `Duration`. One tick is whatever the caller defines
//! (a fixed-timestep frame, a sim step, a roguelike "turn", …).
//!
//! # Two main types
//!
//! * [`Cooldown`] — a simple "not-yet-ready" counter. Decrement each tick;
//!   it's ready when it hits zero. Used for per-actor ability/attack delays.
//! * [`TimerQueue<E>`] — a collection of future events, each scheduled to fire
//!   after a given number of ticks. `advance(n)` fires all events whose delay
//!   expires within `n` ticks and returns them in firing order. Repeat timers
//!   re-enqueue themselves automatically. Generic over the event type `E`.

use crate::fixed::Fixed;
use crate::world_hash::{DetHash, Fnv1a};

// ---------------------------------------------------------------------------
// Cooldown
// ---------------------------------------------------------------------------

/// A simple countdown: starts at `remaining` ticks and decrements to zero.
/// `is_ready()` is true when it hits zero. `reset(n)` re-arms for another
/// `n` ticks. Saturates at zero (never underflows).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cooldown {
    pub remaining: u32,
}

impl Cooldown {
    /// Create a new cooldown already in the ready state.
    pub const fn ready() -> Self {
        Cooldown { remaining: 0 }
    }

    /// Create a new cooldown that needs `ticks` more ticks before it's ready.
    pub const fn new(ticks: u32) -> Self {
        Cooldown { remaining: ticks }
    }

    /// Whether the cooldown has expired (remaining == 0).
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.remaining == 0
    }

    /// Advance by `ticks`. Saturates at zero; returns `true` if it just
    /// became ready (transitioned from non-zero to zero in this call).
    #[inline]
    pub fn tick(&mut self, ticks: u32) -> bool {
        let was_ready = self.is_ready();
        self.remaining = self.remaining.saturating_sub(ticks);
        !was_ready && self.is_ready()
    }

    /// Re-arm for `ticks` more ticks. If already ready, arms from zero.
    #[inline]
    pub fn reset(&mut self, ticks: u32) {
        self.remaining = ticks;
    }

    /// Instantly mark the cooldown as ready (`remaining = 0`). Use when an
    /// ability should be available immediately (e.g. on level-up or a "haste"
    /// effect that clears all cooldowns).
    #[inline]
    pub fn set_ready(&mut self) {
        self.remaining = 0;
    }

    /// Ticks consumed from an original charge of `original`. Returns
    /// `original.saturating_sub(remaining)` so calls against a ready cooldown
    /// always return `original`. Useful for progress bars and UI countdowns.
    #[inline]
    pub fn elapsed(&self, original: u32) -> u32 {
        original.saturating_sub(self.remaining)
    }

    /// Percentage of the cooldown still remaining as an integer in `[0, 100]`.
    /// `original_ticks == 0` always returns `0` (ready). Saturates: if
    /// `remaining > original_ticks`, returns 100. The inverse of `elapsed`.
    /// Useful for "80% remaining on block cooldown" progress bars.
    #[inline]
    pub fn percent_remaining(&self, original_ticks: u32) -> u32 {
        if original_ticks == 0 {
            return 0;
        }
        (self.remaining.min(original_ticks) as u64 * 100 / original_ticks as u64) as u32
    }

    /// Extend the cooldown by `extra` ticks. Saturates at `u32::MAX` rather
    /// than overflowing. Use for "slow" or "anti-haste" effects that push back
    /// an ability without replacing its current remaining time.
    #[inline]
    pub fn extend(&mut self, extra: u32) {
        self.remaining = self.remaining.saturating_add(extra);
    }

    /// Fractional progress through the cooldown as a [`Fixed`]-point value in
    /// `[0, 1]`: `0` means just started, `1` means ready.
    ///
    /// `original_ticks == 0` always returns `Fixed::ONE` (already ready).
    /// If `remaining > original_ticks`, progress saturates at `0`.
    ///
    /// Useful as an animation lerp parameter or smooth progress-bar fill
    /// without converting to float or percent-integer.
    pub fn fractional_progress(&self, original_ticks: u32) -> Fixed {
        if original_ticks == 0 {
            return Fixed::ONE;
        }
        Fixed::from_ratio(self.elapsed(original_ticks) as i32, original_ticks as i32)
    }
}

impl DetHash for Cooldown {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.remaining);
    }
}

// ---------------------------------------------------------------------------
// TimerQueue
// ---------------------------------------------------------------------------

/// An entry in the timer queue.
#[derive(Clone, Debug)]
struct Entry<E> {
    /// Ticks remaining until this entry fires.
    remaining: u32,
    /// If `Some(period)`, re-enqueue with `remaining = period` after firing.
    period: Option<u32>,
    event: E,
}

/// A collection of future events scheduled to fire after a given tick delay.
///
/// `advance(n)` consumes `n` ticks and returns all events that fired (in the
/// order they would have fired — ties break by insertion order). Repeating
/// entries requeue themselves with their original period.
#[derive(Clone, Debug, Default)]
pub struct TimerQueue<E> {
    entries: Vec<Entry<E>>,
}

impl<E: Clone> TimerQueue<E> {
    pub fn new() -> Self {
        TimerQueue {
            entries: Vec::new(),
        }
    }

    /// Schedule `event` to fire after `delay` ticks. `delay == 0` fires on
    /// the next `advance` call (even with `ticks == 0`).
    pub fn schedule(&mut self, delay: u32, event: E) {
        self.entries.push(Entry {
            remaining: delay,
            period: None,
            event,
        });
    }

    /// Schedule a repeating event: fires after `delay` ticks, then again
    /// every `period` ticks. `period == 0` fires every `advance` call.
    pub fn schedule_repeat(&mut self, delay: u32, period: u32, event: E) {
        self.entries.push(Entry {
            remaining: delay,
            period: Some(period),
            event,
        });
    }

    /// Cancel all pending events. Returns the number of entries removed.
    pub fn cancel_all(&mut self) -> usize {
        let n = self.entries.len();
        self.entries.clear();
        n
    }

    /// Cancel all pending entries whose event satisfies `pred`. Both one-shot
    /// and repeating entries are eligible. Returns how many were removed.
    pub fn cancel_where<P: Fn(&E) -> bool>(&mut self, pred: P) -> usize {
        let before = self.entries.len();
        self.entries.retain(|e| !pred(&e.event));
        before - self.entries.len()
    }

    /// How many entries (including repeating) are currently scheduled.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The minimum number of ticks until the next event fires, or `None` if the
    /// queue is empty. Useful for UI countdowns ("next ability in N turns") and
    /// for skipping ahead in headless simulations.
    pub fn peek_next(&self) -> Option<u32> {
        self.entries.iter().map(|e| e.remaining).min()
    }

    /// Number of repeating entries (those scheduled with `schedule_repeat`).
    pub fn count_repeating(&self) -> usize {
        self.entries.iter().filter(|e| e.period.is_some()).count()
    }

    /// Cancel the first pending entry matching `pred` and re-schedule it at
    /// `new_delay` ticks from now (one-shot). Returns `true` if an entry was
    /// found and rescheduled, `false` if no entry matched.
    ///
    /// Use this to "reset" a pending ability or patrol timer without losing
    /// track of which event to reschedule. The rescheduled entry is always
    /// one-shot regardless of whether the original was repeating.
    pub fn reschedule<P: Fn(&E) -> bool>(&mut self, pred: P, new_delay: u32) -> bool {
        if let Some(pos) = self.entries.iter().position(|e| pred(&e.event)) {
            let event = self.entries.remove(pos).event;
            self.schedule(new_delay, event);
            true
        } else {
            false
        }
    }

    /// Advance by `ticks`. Fires (and returns) every event whose delay expires
    /// within those ticks, in firing order (earliest first; ties preserve
    /// insertion order). Repeating entries requeue themselves.
    ///
    /// Each fired entry is cloned into the output before potential requeue, so
    /// the caller always owns the returned events.
    pub fn advance(&mut self, ticks: u32) -> Vec<E> {
        let mut fired: Vec<E> = Vec::new();
        let mut requeue: Vec<Entry<E>> = Vec::new();

        for entry in self.entries.drain(..) {
            if entry.remaining <= ticks {
                fired.push(entry.event.clone());
                if let Some(period) = entry.period {
                    // Remaining ticks after the first fire; re-arm with period.
                    let leftover = ticks - entry.remaining;
                    // A period-0 repeating timer fires once per advance; we
                    // requeue with remaining=0 so it fires next call too.
                    let next = Entry {
                        remaining: period.saturating_sub(leftover),
                        period: entry.period,
                        event: entry.event,
                    };
                    requeue.push(next);
                }
                // Non-repeating entries are simply dropped.
            } else {
                requeue.push(Entry {
                    remaining: entry.remaining - ticks,
                    ..entry
                });
            }
        }

        self.entries = requeue;
        fired
    }
}

impl<E: DetHash + Clone> DetHash for TimerQueue<E> {
    /// Folds every pending entry (remaining + event) in insertion order and the
    /// total count. Two queues with the same schedule hash identically.
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.entries.len() as u32);
        for entry in &self.entries {
            hasher.write_u32(entry.remaining);
            entry.event.det_hash(hasher);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    // --- Cooldown -----------------------------------------------------------

    #[test]
    fn test_cooldown_ready_from_start() {
        let cd = Cooldown::ready();
        assert!(cd.is_ready());
    }

    #[test]
    fn test_cooldown_ticks_down_to_ready() {
        let mut cd = Cooldown::new(3);
        assert!(!cd.is_ready());
        assert!(!cd.tick(1));
        assert!(!cd.tick(1));
        let just_ready = cd.tick(1);
        assert!(just_ready);
        assert!(cd.is_ready());
    }

    #[test]
    fn test_cooldown_saturates_at_zero() {
        let mut cd = Cooldown::new(2);
        cd.tick(10);
        assert_eq!(cd.remaining, 0);
    }

    #[test]
    fn test_cooldown_tick_returns_true_only_on_transition() {
        let mut cd = Cooldown::new(1);
        let t1 = cd.tick(1); // transitions to 0 → true
        let t2 = cd.tick(1); // already 0 → false
        assert!(t1);
        assert!(!t2);
    }

    #[test]
    fn test_cooldown_reset_rearms() {
        let mut cd = Cooldown::ready();
        cd.reset(5);
        assert!(!cd.is_ready());
        assert_eq!(cd.remaining, 5);
    }

    #[test]
    fn test_cooldown_det_hash_changes_on_mutation() {
        let a = Cooldown::new(3);
        let b = Cooldown::new(4);
        assert_ne!(hash_state(&a), hash_state(&b));
    }

    // --- TimerQueue ---------------------------------------------------------

    #[test]
    fn test_single_fire_after_delay() {
        let mut q: TimerQueue<u32> = TimerQueue::new();
        q.schedule(3, 42);
        assert!(q.advance(2).is_empty());
        let fired = q.advance(1);
        assert_eq!(fired, vec![42]);
        assert!(q.is_empty());
    }

    #[test]
    fn test_multiple_fires_in_one_advance() {
        let mut q: TimerQueue<u32> = TimerQueue::new();
        q.schedule(1, 10);
        q.schedule(2, 20);
        q.schedule(3, 30);
        let fired = q.advance(3);
        // All three fire; insertion order.
        assert_eq!(fired, vec![10, 20, 30]);
    }

    #[test]
    fn test_zero_delay_fires_immediately() {
        let mut q: TimerQueue<u32> = TimerQueue::new();
        q.schedule(0, 99);
        let fired = q.advance(0);
        assert_eq!(fired, vec![99]);
    }

    #[test]
    fn test_repeating_timer_requeues() {
        let mut q: TimerQueue<u32> = TimerQueue::new();
        q.schedule_repeat(2, 2, 1);
        let f1 = q.advance(2);
        assert_eq!(f1, vec![1]);
        assert_eq!(q.len(), 1); // re-queued
        let f2 = q.advance(2);
        assert_eq!(f2, vec![1]);
    }

    #[test]
    fn test_cancel_all() {
        let mut q: TimerQueue<u32> = TimerQueue::new();
        q.schedule(5, 1);
        q.schedule(10, 2);
        let removed = q.cancel_all();
        assert_eq!(removed, 2);
        assert!(q.is_empty());
        let fired = q.advance(100);
        assert!(fired.is_empty());
    }

    #[test]
    fn test_cancel_where_removes_matching_entries() {
        let mut q: TimerQueue<u32> = TimerQueue::new();
        q.schedule(1, 10);
        q.schedule(2, 20);
        q.schedule(3, 30);
        let removed = q.cancel_where(|&e| e >= 20);
        assert_eq!(removed, 2);
        assert_eq!(q.len(), 1);
        // Only event 10 should fire.
        let fired = q.advance(10);
        assert_eq!(fired, vec![10]);
    }

    #[test]
    fn test_cancel_where_none_removed_when_no_match() {
        let mut q: TimerQueue<u32> = TimerQueue::new();
        q.schedule(1, 5);
        q.schedule(2, 6);
        let removed = q.cancel_where(|&e| e > 100);
        assert_eq!(removed, 0);
        assert_eq!(q.len(), 2);
    }

    #[test]
    fn test_non_repeating_does_not_requeue() {
        let mut q: TimerQueue<u32> = TimerQueue::new();
        q.schedule(1, 7);
        let _ = q.advance(5);
        assert!(q.is_empty());
    }

    #[test]
    fn test_det_hash_same_schedule_same_hash() {
        let mut a: TimerQueue<u32> = TimerQueue::new();
        let mut b: TimerQueue<u32> = TimerQueue::new();
        a.schedule(3, 1);
        a.schedule(5, 2);
        b.schedule(3, 1);
        b.schedule(5, 2);
        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_cooldown_set_ready() {
        let mut cd = Cooldown::new(10);
        cd.set_ready();
        assert!(cd.is_ready());
        assert_eq!(cd.remaining, 0);
    }

    #[test]
    fn test_cooldown_elapsed() {
        let cd = Cooldown::new(3);
        assert_eq!(cd.elapsed(10), 7); // 10 - 3 = 7 ticks consumed
    }

    #[test]
    fn test_cooldown_elapsed_ready() {
        let cd = Cooldown::ready();
        assert_eq!(cd.elapsed(5), 5); // fully consumed
    }

    #[test]
    fn test_timer_queue_peek_next_empty() {
        let q: TimerQueue<u32> = TimerQueue::new();
        assert_eq!(q.peek_next(), None);
    }

    #[test]
    fn test_timer_queue_peek_next_returns_min() {
        let mut q: TimerQueue<u32> = TimerQueue::new();
        q.schedule(5, 1);
        q.schedule(2, 2);
        q.schedule(8, 3);
        assert_eq!(q.peek_next(), Some(2));
    }

    #[test]
    fn test_timer_queue_count_repeating() {
        let mut q: TimerQueue<u32> = TimerQueue::new();
        q.schedule(1, 10);
        q.schedule_repeat(2, 3, 20);
        q.schedule_repeat(4, 5, 30);
        assert_eq!(q.count_repeating(), 2);
    }

    #[test]
    fn test_timer_queue_count_repeating_zero_when_all_oneshot() {
        let mut q: TimerQueue<u32> = TimerQueue::new();
        q.schedule(1, 1);
        q.schedule(2, 2);
        assert_eq!(q.count_repeating(), 0);
    }

    #[test]
    fn test_det_hash_different_remaining_differs() {
        let mut a: TimerQueue<u32> = TimerQueue::new();
        let mut b: TimerQueue<u32> = TimerQueue::new();
        a.schedule(3, 1);
        b.schedule(4, 1);
        assert_ne!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_reschedule_returns_false_when_not_found() {
        let mut q: TimerQueue<u32> = TimerQueue::new();
        q.schedule(5, 10);
        assert!(!q.reschedule(|&e| e == 99, 2));
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn test_reschedule_returns_true_and_fires_at_new_delay() {
        let mut q: TimerQueue<u32> = TimerQueue::new();
        q.schedule(10, 42);
        let found = q.reschedule(|&e| e == 42, 3);
        assert!(found);
        assert_eq!(q.len(), 1);
        let not_yet = q.advance(2);
        assert!(not_yet.is_empty());
        let fired = q.advance(1);
        assert_eq!(fired, vec![42]);
    }

    #[test]
    fn test_reschedule_repeating_becomes_oneshot() {
        let mut q: TimerQueue<u32> = TimerQueue::new();
        q.schedule_repeat(5, 5, 7);
        assert_eq!(q.count_repeating(), 1);
        q.reschedule(|&e| e == 7, 2);
        // After reschedule the rescheduled entry is one-shot.
        assert_eq!(q.count_repeating(), 0);
        let fired = q.advance(2);
        assert_eq!(fired, vec![7]);
        assert!(q.is_empty());
    }

    #[test]
    fn test_reschedule_on_empty_queue_returns_false() {
        let mut q: TimerQueue<u32> = TimerQueue::new();
        assert!(!q.reschedule(|_| true, 1));
    }

    #[test]
    fn test_percent_remaining_full() {
        let cd = Cooldown::new(100);
        assert_eq!(cd.percent_remaining(100), 100);
    }

    #[test]
    fn test_percent_remaining_ready_returns_zero() {
        let cd = Cooldown::ready();
        assert_eq!(cd.percent_remaining(100), 0);
    }

    #[test]
    fn test_percent_remaining_half() {
        let cd = Cooldown::new(50);
        assert_eq!(cd.percent_remaining(100), 50);
    }

    #[test]
    fn test_percent_remaining_original_zero_returns_zero() {
        let cd = Cooldown::new(5);
        assert_eq!(cd.percent_remaining(0), 0);
    }

    #[test]
    fn test_fractional_progress_ready_is_one() {
        use crate::fixed::Fixed;
        let cd = Cooldown::ready();
        assert_eq!(cd.fractional_progress(10), Fixed::ONE);
    }

    #[test]
    fn test_fractional_progress_original_zero_is_one() {
        use crate::fixed::Fixed;
        let cd = Cooldown::new(5);
        assert_eq!(cd.fractional_progress(0), Fixed::ONE);
    }

    #[test]
    fn test_cooldown_extend_adds_ticks() {
        let mut cd = Cooldown::new(5);
        cd.extend(3);
        assert_eq!(cd.remaining, 8);
    }

    #[test]
    fn test_cooldown_extend_on_ready_arms_it() {
        let mut cd = Cooldown::ready();
        cd.extend(10);
        assert!(!cd.is_ready());
        assert_eq!(cd.remaining, 10);
    }

    #[test]
    fn test_cooldown_extend_saturates() {
        let mut cd = Cooldown::new(u32::MAX);
        cd.extend(1); // must not overflow
        assert_eq!(cd.remaining, u32::MAX);
    }

    #[test]
    fn test_fractional_progress_half() {
        use crate::fixed::Fixed;
        // original=10, remaining=5 → elapsed=5/10 = 0.5
        let cd = Cooldown::new(5);
        let p = cd.fractional_progress(10);
        let half = Fixed::from_ratio(1, 2);
        assert_eq!(p, half);
    }
}
