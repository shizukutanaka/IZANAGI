//! Time-driven value interpolation — eased tweens over a tick span.
//!
//! [`easing`](crate::easing) supplies interpolation *curves* (`fn(Fixed) ->
//! Fixed` over the unit interval) and [`fixed::Fixed::lerp`](crate::fixed::Fixed::lerp)
//! interpolates between two endpoints, but nothing held the *state* needed to
//! play an interpolation out over time: a start, an end, a duration, and how
//! many ticks have elapsed. [`status`](crate::status) tracks effect durations,
//! [`timer`](crate::timer) tracks cooldowns, and [`pool`](crate::pool) tracks
//! linear regeneration — but none yield "the eased value *right now*, N ticks
//! into a D-tick animation." [`Tween`] is that primitive.
//!
//! Following the same decoupling as [`recipe`](crate::recipe), the easing
//! curve is **not stored** in the tween (function pointers are neither
//! deterministically hashable nor part of the simulation state). Instead it is
//! supplied at sample time, so a tween's [`DetHash`](crate::world_hash::DetHash)
//! covers only its time state (`start`, `end`, `duration`, `elapsed`) and the
//! same tween can be sampled through any curve.
//!
//! ```
//! use izanagi_kit::tween::Tween;
//! use izanagi_kit::fixed::Fixed;
//! use izanagi_kit::easing::{linear, ease_in_out_quad};
//!
//! // Slide a value from 0 to 100 over 10 ticks.
//! let mut t = Tween::new(Fixed::from_int(0), Fixed::from_int(100), 10);
//! assert!(!t.is_done());
//!
//! t.advance(5);                       // halfway through
//! assert_eq!(t.value(linear).to_int_round(), 50);
//!
//! t.advance(5);                       // finished
//! assert!(t.is_done());
//! assert_eq!(t.value(ease_in_out_quad).to_int_round(), 100);
//! ```
//!
//! Determinism: time is integer ticks and the value is computed with
//! [`Fixed`](crate::fixed::Fixed) (Q16.16), so a tween is bit-identical across
//! targets and folds safely into the replay checksum.

use crate::fixed::Fixed;
use crate::world_hash::{DetHash, Fnv1a};

/// A value interpolating from `start` to `end` over `duration` ticks.
///
/// The current value is obtained by sampling an easing curve at the tween's
/// linear time progress; see [`value`](Tween::value) and
/// [`value_linear`](Tween::value_linear).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tween {
    start: Fixed,
    end: Fixed,
    duration: u32,
    elapsed: u32,
}

impl Tween {
    /// Create a tween from `start` to `end` over `duration` ticks, beginning at
    /// elapsed 0. A `duration` of `0` is a tween that is immediately complete
    /// (its value is always `end`).
    pub fn new(start: Fixed, end: Fixed, duration: u32) -> Self {
        Tween {
            start,
            end,
            duration,
            elapsed: 0,
        }
    }

    /// Create a tween resumed at a given elapsed tick count (clamped to
    /// `duration`). Useful when restoring from a save.
    pub fn with_elapsed(start: Fixed, end: Fixed, duration: u32, elapsed: u32) -> Self {
        Tween {
            start,
            end,
            duration,
            elapsed: elapsed.min(duration),
        }
    }

    /// The starting value.
    #[inline]
    pub fn start(&self) -> Fixed {
        self.start
    }

    /// The target value.
    #[inline]
    pub fn end(&self) -> Fixed {
        self.end
    }

    /// The total duration in ticks.
    #[inline]
    pub fn duration(&self) -> u32 {
        self.duration
    }

    /// Ticks elapsed so far (never exceeds `duration`).
    #[inline]
    pub fn elapsed(&self) -> u32 {
        self.elapsed
    }

    /// Ticks remaining until completion: `duration - elapsed`.
    #[inline]
    pub fn remaining(&self) -> u32 {
        self.duration - self.elapsed
    }

    /// `true` once `elapsed >= duration` (a zero-duration tween starts done).
    #[inline]
    pub fn is_done(&self) -> bool {
        self.elapsed >= self.duration
    }

    /// Advance the tween by `ticks` (saturating, capped at `duration`).
    /// Returns `true` if the tween is complete after advancing.
    pub fn advance(&mut self, ticks: u32) -> bool {
        self.elapsed = self.elapsed.saturating_add(ticks).min(self.duration);
        self.is_done()
    }

    /// Linear time progress in `[0, 1]` as a [`Fixed`]: `elapsed / duration`.
    /// A zero-duration tween reports `1` (complete).
    pub fn progress(&self) -> Fixed {
        if self.duration == 0 || self.elapsed >= self.duration {
            return Fixed::ONE;
        }
        Fixed::from_ratio(self.elapsed as i32, self.duration as i32)
    }

    /// The current value using **linear** interpolation (no easing curve):
    /// `lerp(start, end, progress)`.
    pub fn value_linear(&self) -> Fixed {
        Fixed::lerp(self.start, self.end, self.progress())
    }

    /// The current value, sampling `easing` at the linear progress. The eased
    /// parameter is clamped to `[0, 1]` before interpolation so overshooting
    /// curves (e.g. `ease_out_back`) still interpolate between the endpoints
    /// rather than beyond them; pass [`value_overshoot`](Tween::value_overshoot)
    /// if you want the overshoot preserved.
    pub fn value(&self, easing: fn(Fixed) -> Fixed) -> Fixed {
        let t = easing(self.progress()).clamp01();
        Fixed::lerp(self.start, self.end, t)
    }

    /// Like [`value`](Tween::value) but does **not** clamp the eased parameter,
    /// allowing curves that overshoot or undershoot the `[0, 1]` range (back,
    /// elastic) to push the value past the endpoints for anticipation effects.
    pub fn value_overshoot(&self, easing: fn(Fixed) -> Fixed) -> Fixed {
        Fixed::lerp(self.start, self.end, easing(self.progress()))
    }

    /// Restart from elapsed 0 (replay the same tween).
    #[inline]
    pub fn reset(&mut self) {
        self.elapsed = 0;
    }

    /// Swap `start` and `end` and restart, so the tween plays back toward its
    /// original starting value. Useful for ping-pong / yo-yo animations.
    pub fn reverse(&mut self) {
        std::mem::swap(&mut self.start, &mut self.end);
        self.elapsed = 0;
    }
}

impl DetHash for Tween {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        self.start.det_hash(hasher);
        self.end.det_hash(hasher);
        hasher.write_u32(self.duration);
        hasher.write_u32(self.elapsed);
    }
}

/// A chain of [`Tween`]s played back-to-back under a single clock.
///
/// Playing several tweens **concurrently** needs no new type — a
/// `Vec<Tween>` advanced with `iter_mut().for_each(|t| t.advance(dt))` (e.g. a
/// UI bar fill and an unrelated sound fade ticking side by side) already
/// works. What was missing is **sequential** playback: a walk cycle of N
/// frame-tweens, a cutscene of several eased legs, a UI element that slides in
/// then holds then fades — where one clock should drive whichever step is
/// current and roll any leftover ticks into the next step the instant one
/// completes, so a single large `advance` (a slow frame, a fast-forward)
/// correctly fast-forwards through several short steps instead of stalling on
/// the first.
///
/// ```
/// use izanagi_kit::tween::{Tween, TweenSequence};
/// use izanagi_kit::fixed::Fixed;
/// use izanagi_kit::easing::linear;
///
/// fn fi(n: i32) -> Fixed { Fixed::from_int(n) }
///
/// let mut seq = TweenSequence::new(vec![
///     Tween::new(fi(0), fi(10), 5),  // slide in
///     Tween::new(fi(10), fi(10), 3), // hold
///     Tween::new(fi(10), fi(0), 5),  // slide out
/// ]);
///
/// seq.advance(7); // 5 ticks finish step 0; the other 2 carry into step 1
/// assert_eq!(seq.current_index(), Some(1));
/// assert_eq!(seq.value(linear).to_int_round(), 10);
///
/// seq.advance(100); // fast-forwards through the hold and the slide-out
/// assert!(seq.is_done());
/// assert_eq!(seq.value(linear).to_int_round(), 0, "settles on the last step's end value");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TweenSequence {
    steps: Vec<Tween>,
    current: usize,
}

impl TweenSequence {
    /// Create a sequence from `steps`, played in order starting at the first.
    /// An empty sequence is immediately done.
    pub fn new(steps: Vec<Tween>) -> Self {
        TweenSequence { steps, current: 0 }
    }

    /// The number of steps in the chain.
    #[inline]
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// `true` if the chain has no steps.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// The index of the current step, or `None` for an empty sequence. Once
    /// the sequence completes this stays at the last valid index rather than
    /// running off the end, so [`current_step`](Self::current_step) and
    /// [`value`](Self::value) keep reporting the final step's end value.
    #[inline]
    pub fn current_index(&self) -> Option<usize> {
        if self.steps.is_empty() {
            None
        } else {
            Some(self.current)
        }
    }

    /// The current step, or `None` for an empty sequence.
    #[inline]
    pub fn current_step(&self) -> Option<&Tween> {
        self.steps.get(self.current)
    }

    /// `true` once the last step has completed (or the sequence is empty).
    pub fn is_done(&self) -> bool {
        match self.current_step() {
            Some(step) => self.current + 1 == self.steps.len() && step.is_done(),
            None => true,
        }
    }

    /// Advance the chain by `ticks`, feeding them into the current step and
    /// rolling any leftover into the next step the moment one completes (so a
    /// single large advance can fast-forward through several short steps in
    /// one call). Ticks beyond the final step's completion are discarded, the
    /// same saturating behaviour as [`Tween::advance`]. Returns `true` if the
    /// chain is complete after advancing.
    pub fn advance(&mut self, mut ticks: u32) -> bool {
        while ticks > 0 && !self.is_done() {
            let idx = self.current;
            let step = &mut self.steps[idx];
            let take = ticks.min(step.remaining());
            step.advance(take);
            ticks -= take;
            if step.is_done() && idx + 1 < self.steps.len() {
                self.current += 1;
            }
        }
        self.is_done()
    }

    /// The current step's eased value, or [`Fixed::ZERO`] for an empty
    /// sequence (there is no meaningful value to report).
    pub fn value(&self, easing: fn(Fixed) -> Fixed) -> Fixed {
        self.current_step()
            .map(|s| s.value(easing))
            .unwrap_or(Fixed::ZERO)
    }

    /// Total duration of every step, summed (saturating).
    pub fn total_duration(&self) -> u32 {
        self.steps
            .iter()
            .fold(0u32, |acc, s| acc.saturating_add(s.duration()))
    }

    /// Ticks elapsed across the whole chain: every completed step's full
    /// duration plus the current step's own elapsed.
    pub fn elapsed_total(&self) -> u32 {
        let completed: u32 = self.steps[..self.current]
            .iter()
            .fold(0u32, |acc, s| acc.saturating_add(s.duration()));
        completed.saturating_add(self.current_step().map(|s| s.elapsed()).unwrap_or(0))
    }

    /// Overall progress across the whole chain in `[0, 1]`, analogous to
    /// [`Tween::progress`]. An empty (or zero-total-duration) sequence reports
    /// [`Fixed::ONE`] (complete).
    pub fn progress(&self) -> Fixed {
        let total = self.total_duration();
        if total == 0 {
            return Fixed::ONE;
        }
        Fixed::from_ratio(self.elapsed_total() as i32, total as i32)
    }

    /// Restart every step from elapsed 0 and rewind the cursor to the first
    /// step (replay the whole chain from the beginning).
    pub fn reset(&mut self) {
        self.current = 0;
        for step in &mut self.steps {
            step.reset();
        }
    }
}

impl DetHash for TweenSequence {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.steps.len() as u32);
        for step in &self.steps {
            step.det_hash(hasher);
        }
        hasher.write_u32(self.current as u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::easing::{ease_in_quad, ease_out_back, linear};
    use crate::world_hash::hash_state;

    fn fi(n: i32) -> Fixed {
        Fixed::from_int(n)
    }

    #[test]
    fn test_new_starts_at_zero_elapsed() {
        let t = Tween::new(fi(0), fi(100), 10);
        assert_eq!(t.elapsed(), 0);
        assert_eq!(t.remaining(), 10);
        assert!(!t.is_done());
        assert_eq!(t.value_linear().to_int_round(), 0, "value at t=0 is start");
    }

    #[test]
    fn test_linear_midpoint() {
        let mut t = Tween::new(fi(0), fi(100), 10);
        t.advance(5);
        assert_eq!(t.value_linear().to_int_round(), 50);
        assert_eq!(
            t.value(linear).to_int_round(),
            50,
            "linear easing == value_linear"
        );
    }

    #[test]
    fn test_completes_at_end_value() {
        let mut t = Tween::new(fi(20), fi(80), 4);
        assert!(t.advance(4));
        assert!(t.is_done());
        assert_eq!(t.value_linear().to_int_round(), 80, "ends at end value");
        assert_eq!(t.value(ease_in_quad).to_int_round(), 80);
    }

    #[test]
    fn test_advance_caps_at_duration() {
        let mut t = Tween::new(fi(0), fi(10), 5);
        assert!(t.advance(100));
        assert_eq!(t.elapsed(), 5, "elapsed capped at duration");
        assert_eq!(t.remaining(), 0);
    }

    #[test]
    fn test_zero_duration_is_immediately_done() {
        let t = Tween::new(fi(0), fi(100), 0);
        assert!(t.is_done());
        assert_eq!(t.progress(), Fixed::ONE);
        assert_eq!(t.value_linear().to_int_round(), 100);
    }

    #[test]
    fn test_with_elapsed_clamps() {
        let t = Tween::with_elapsed(fi(0), fi(100), 10, 50);
        assert_eq!(t.elapsed(), 10, "elapsed clamped to duration");
        assert!(t.is_done());
    }

    #[test]
    fn test_progress_range() {
        let mut t = Tween::new(fi(0), fi(1), 8);
        assert_eq!(t.progress(), Fixed::ZERO);
        t.advance(4);
        assert_eq!(t.progress(), Fixed::from_ratio(1, 2));
        t.advance(4);
        assert_eq!(t.progress(), Fixed::ONE);
    }

    #[test]
    fn test_reset() {
        let mut t = Tween::new(fi(0), fi(100), 10);
        t.advance(7);
        t.reset();
        assert_eq!(t.elapsed(), 0);
        assert_eq!(t.value_linear().to_int_round(), 0);
    }

    #[test]
    fn test_reverse_swaps_and_resets() {
        let mut t = Tween::new(fi(0), fi(100), 10);
        t.advance(10);
        t.reverse();
        assert_eq!(t.elapsed(), 0);
        assert_eq!(t.start().to_int_round(), 100);
        assert_eq!(t.end().to_int_round(), 0);
        assert_eq!(
            t.value_linear().to_int_round(),
            100,
            "now starts at old end"
        );
    }

    #[test]
    fn test_value_clamps_overshoot_by_default() {
        // ease_out_back overshoots above 1.0 near the end; value() clamps it.
        let mut t = Tween::new(fi(0), fi(100), 10);
        t.advance(9);
        let clamped = t.value(ease_out_back).to_int_round();
        assert!(
            clamped <= 100,
            "default value() clamps overshoot to end ({clamped})"
        );
    }

    #[test]
    fn test_value_overshoot_preserves_back_curve() {
        let mut t = Tween::new(fi(0), fi(100), 100);
        t.advance(80);
        let overshoot = t.value_overshoot(ease_out_back).to_int_round();
        let clamped = t.value(ease_out_back).to_int_round();
        // The overshoot variant can exceed the clamped one near the tail.
        assert!(
            overshoot >= clamped,
            "overshoot >= clamped ({overshoot} vs {clamped})"
        );
    }

    #[test]
    fn test_det_hash_canonical_and_sensitive() {
        let a = Tween::with_elapsed(fi(0), fi(100), 10, 3);
        let b = Tween::with_elapsed(fi(0), fi(100), 10, 3);
        assert_eq!(hash_state(&a), hash_state(&b), "same state, same hash");
        let c = Tween::with_elapsed(fi(0), fi(100), 10, 4);
        assert_ne!(
            hash_state(&a),
            hash_state(&c),
            "different elapsed → different hash"
        );
        let d = Tween::with_elapsed(fi(0), fi(101), 10, 3);
        assert_ne!(
            hash_state(&a),
            hash_state(&d),
            "different end → different hash"
        );
    }

    // --- TweenSequence --------------------------------------------------

    fn seq3() -> TweenSequence {
        TweenSequence::new(vec![
            Tween::new(fi(0), fi(10), 5),
            Tween::new(fi(10), fi(10), 3),
            Tween::new(fi(10), fi(0), 5),
        ])
    }

    #[test]
    fn test_sequence_new_starts_at_first_step() {
        let seq = seq3();
        assert_eq!(seq.current_index(), Some(0));
        assert_eq!(seq.len(), 3);
        assert!(!seq.is_empty());
        assert!(!seq.is_done());
        assert_eq!(seq.value(linear).to_int_round(), 0);
    }

    #[test]
    fn test_sequence_empty_is_immediately_done() {
        let seq = TweenSequence::new(vec![]);
        assert!(seq.is_empty());
        assert!(seq.is_done());
        assert_eq!(seq.current_index(), None);
        assert_eq!(seq.current_step(), None);
        assert_eq!(seq.value(linear), Fixed::ZERO);
        assert_eq!(seq.progress(), Fixed::ONE);
    }

    #[test]
    fn test_sequence_advance_within_one_step_does_not_roll_over() {
        let mut seq = seq3();
        seq.advance(3);
        assert_eq!(seq.current_index(), Some(0), "3 < step 0's duration of 5");
        assert_eq!(seq.value(linear).to_int_round(), 6);
        assert!(!seq.is_done());
    }

    #[test]
    fn test_sequence_advance_rolls_leftover_into_next_step() {
        let mut seq = seq3();
        // 5 ticks finish step 0 exactly; 2 more roll into step 1.
        seq.advance(7);
        assert_eq!(seq.current_index(), Some(1));
        assert_eq!(seq.current_step().unwrap().elapsed(), 2);
        assert_eq!(seq.value(linear).to_int_round(), 10, "step 1 holds at 10");
    }

    #[test]
    fn test_sequence_large_advance_fast_forwards_through_all_steps() {
        let mut seq = seq3();
        assert!(seq.advance(1000));
        assert!(seq.is_done());
        assert_eq!(
            seq.current_index(),
            Some(2),
            "cursor settles on the last step, never runs off the end"
        );
        assert_eq!(
            seq.value(linear).to_int_round(),
            0,
            "settles on last step's end value"
        );
    }

    #[test]
    fn test_sequence_advance_exactly_to_boundary() {
        let mut seq = seq3();
        // Exactly the total duration (5+3+5=13) must land exactly on done.
        assert!(seq.advance(13));
        assert!(seq.is_done());
        assert_eq!(seq.elapsed_total(), 13);
    }

    #[test]
    fn test_sequence_zero_duration_step_is_skipped_without_stalling() {
        let mut seq = TweenSequence::new(vec![
            Tween::new(fi(0), fi(5), 5),
            Tween::new(fi(5), fi(5), 0), // zero-duration middle step
            Tween::new(fi(5), fi(0), 5),
        ]);
        seq.advance(10); // exactly finishes step 0 and step 2, skipping step 1
        assert!(seq.is_done());
        assert_eq!(seq.value(linear).to_int_round(), 0);
    }

    #[test]
    fn test_sequence_single_step_matches_bare_tween() {
        let mut seq = TweenSequence::new(vec![Tween::new(fi(0), fi(100), 10)]);
        let mut bare = Tween::new(fi(0), fi(100), 10);
        seq.advance(4);
        bare.advance(4);
        assert_eq!(seq.value(linear), bare.value(linear));
        assert_eq!(seq.is_done(), bare.is_done());
    }

    #[test]
    fn test_sequence_total_duration_sums_steps() {
        let seq = seq3();
        assert_eq!(seq.total_duration(), 13);
    }

    #[test]
    fn test_sequence_progress_tracks_elapsed_over_total() {
        let mut seq = seq3();
        assert_eq!(seq.progress(), Fixed::ZERO);
        seq.advance(13);
        assert_eq!(seq.progress(), Fixed::ONE);
    }

    #[test]
    fn test_sequence_reset_rewinds_every_step_and_cursor() {
        let mut seq = seq3();
        seq.advance(13);
        assert!(seq.is_done());
        seq.reset();
        assert_eq!(seq.current_index(), Some(0));
        assert!(!seq.is_done());
        assert_eq!(seq.elapsed_total(), 0);
        assert_eq!(seq.value(linear).to_int_round(), 0);
    }

    #[test]
    fn test_sequence_advance_zero_ticks_is_noop() {
        let mut seq = seq3();
        seq.advance(3);
        let before = seq.clone();
        seq.advance(0);
        assert_eq!(seq, before);
    }

    #[test]
    fn test_sequence_det_hash_sensitive_to_position() {
        let mut a = seq3();
        let b = seq3();
        assert_eq!(
            hash_state(&a),
            hash_state(&b),
            "identical fresh sequences hash equal"
        );
        a.advance(1);
        assert_ne!(hash_state(&a), hash_state(&b), "advancing changes the hash");
    }

    #[test]
    fn test_sequence_det_hash_sensitive_to_step_count() {
        let a = TweenSequence::new(vec![Tween::new(fi(0), fi(1), 5)]);
        let b = TweenSequence::new(vec![
            Tween::new(fi(0), fi(1), 5),
            Tween::new(fi(1), fi(2), 5),
        ]);
        assert_ne!(
            hash_state(&a),
            hash_state(&b),
            "different step count → different hash"
        );
    }
}
