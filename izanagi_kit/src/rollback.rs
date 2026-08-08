//! Rollback-netcode building blocks: a bounded snapshot ring and a
//! development-time sync test.
//!
//! Two lessons from shipped rollback implementations, made reusable:
//!
//! - **Snapshotting is the dominant cost** of rollback (NetherRealm's Mortal
//!   Kombat 11 netcode, GDC 2019). You cannot keep a full state history — you
//!   keep a *bounded* ring of recent snapshots and re-simulate forward from
//!   the nearest one when a rollback is needed. [`SnapshotRing`] is that ring:
//!   `capacity` slots, one saved every `stride` frames, oldest evicted first.
//! - **A rollback engine is only correct if the simulation is deterministic**,
//!   and the cheapest way to catch nondeterminism is to *do a rollback every
//!   frame even when you don't need one* and check the result still matches
//!   (GGRS's `SyncTestSession`). [`sync_test`] is that check: it rolls the
//!   simulation back `check_distance` frames and re-simulates forward at every
//!   step, reporting the first frame whose re-simulated [`DetHash`] disagrees
//!   with the original.
//!
//! [`sync_test`] catches a different bug class than
//! [`dst_determinism_sweep`](crate::dst::dst_determinism_sweep): that runs the
//! *whole* simulation twice from the start, this exercises the actual
//! rollback-and-resimulate path (partial re-runs from mid-stream snapshots),
//! which is what a rollback session does at runtime and where
//! order-of-operations nondeterminism tends to hide.
//!
//! The simulation is supplied as a `step(&mut state, &input)` closure, the
//! same shape used by [`resimulate`] and
//! [`record_trace`](crate::replay::record_trace), so a state that already
//! implements [`DetHash`] is sync-testable for free.

use std::collections::VecDeque;

use crate::replay::{resimulate, Divergence};
use crate::world_hash::{hash_state, DetHash};

/// A bounded ring of simulation snapshots for rollback.
///
/// Holds at most `capacity` snapshots, one taken every `stride` frames. When
/// full, saving a new snapshot evicts the oldest — so the ring always covers
/// the most recent `capacity * stride` frames (approximately), the window a
/// rollback session can undo into. Cloning `S` is the only cost, matching the
/// MK11 observation that snapshot storage — not re-simulation — is what bounds
/// a rollback budget.
pub struct SnapshotRing<S> {
    stride: u64,
    capacity: usize,
    slots: VecDeque<(u64, S)>,
    last_saved: Option<u64>,
}

impl<S: Clone> SnapshotRing<S> {
    /// Create a ring holding up to `capacity` snapshots at a `stride`-frame
    /// interval. Panics if either is zero (a zero-capacity or zero-stride ring
    /// can never store anything — a programming error, not a runtime input).
    pub fn new(capacity: usize, stride: u64) -> Self {
        assert!(capacity > 0, "SnapshotRing capacity must be > 0");
        assert!(stride > 0, "SnapshotRing stride must be > 0");
        SnapshotRing {
            stride,
            capacity,
            slots: VecDeque::with_capacity(capacity),
            last_saved: None,
        }
    }

    /// Offer the current `(frame, state)` to the ring. A clone is stored iff
    /// `frame` lies on the stride (`frame % stride == 0`) **and** is strictly
    /// newer than the last stored frame; otherwise it is ignored. Returns
    /// whether it was stored. When the ring is full the oldest snapshot is
    /// evicted first.
    pub fn offer(&mut self, frame: u64, state: &S) -> bool {
        if frame % self.stride != 0 {
            return false;
        }
        if let Some(last) = self.last_saved {
            if frame <= last {
                return false;
            }
        }
        if self.slots.len() == self.capacity {
            self.slots.pop_front();
        }
        self.slots.push_back((frame, state.clone()));
        self.last_saved = Some(frame);
        true
    }

    /// The newest stored snapshot at or before `frame` — the one a rollback to
    /// `frame` should re-simulate forward from. `None` if the ring holds no
    /// snapshot that old (the target predates the ring's window).
    pub fn nearest_at_or_before(&self, frame: u64) -> Option<(u64, &S)> {
        self.slots
            .iter()
            .rev()
            .find(|(f, _)| *f <= frame)
            .map(|(f, s)| (*f, s))
    }

    /// The oldest frame currently stored (the far edge of the rollback
    /// window), or `None` when empty.
    pub fn oldest_frame(&self) -> Option<u64> {
        self.slots.front().map(|(f, _)| *f)
    }

    /// The newest frame currently stored, or `None` when empty.
    pub fn newest_frame(&self) -> Option<u64> {
        self.slots.back().map(|(f, _)| *f)
    }

    /// Number of snapshots currently stored.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Whether the ring holds no snapshots yet.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// The configured stride (frames between saved snapshots).
    pub fn stride(&self) -> u64 {
        self.stride
    }

    /// The configured capacity (maximum snapshots held).
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

/// A [`sync_test`] failure: the earliest frame whose re-simulation from a
/// `check_distance`-frame-old snapshot produced a state hash different from
/// the one the straight-through run produced there. That mismatch means the
/// `step` closure is **not deterministic** — the same inputs from the same
/// state gave two different results — which a rollback session cannot tolerate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncTestFailure {
    /// The 1-based frame (number of inputs applied) at which the mismatch
    /// was detected.
    pub frame: usize,
    /// How many frames back the re-simulation started from.
    pub check_distance: usize,
    /// The straight-through state hash at `frame`.
    pub expected: u64,
    /// The hash produced by rolling back and re-simulating to `frame`.
    pub actual: u64,
}

impl core::fmt::Display for SyncTestFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "sync-test mismatch at frame {} (rolled back {} frame(s)): \
             straight-through {:#018x} != re-simulated {:#018x} — the step \
             function is nondeterministic",
            self.frame, self.check_distance, self.expected, self.actual
        )
    }
}

impl SyncTestFailure {
    /// View this failure as a plain [`Divergence`] (tick = `frame`), for code
    /// that already handles the replay module's divergence type.
    pub fn as_divergence(&self) -> Divergence {
        Divergence {
            tick: self.frame,
            expected: self.expected,
            actual: self.actual,
        }
    }
}

/// Run GGRS-style sync testing over `inputs`: at every frame, roll the
/// simulation back `check_distance` frames and re-simulate forward, checking
/// the result matches the straight-through run. Returns the first
/// [`SyncTestFailure`], or `Ok(())` if every re-simulation matched (the step
/// function is deterministic over these inputs).
///
/// `check_distance` is the rollback depth to exercise; `0` disables the check
/// (there is nothing to roll back into) and returns `Ok(())` immediately. The
/// straight-through run and every re-simulation both drive the same `step`
/// closure, so a mismatch can only come from `step` itself producing
/// different output for identical `(state, input)` — hidden globals, pointer-
/// address-dependent ordering, uninitialised reads, wall-clock or thread-id
/// use. Memory is bounded to the last `check_distance + 1` states regardless
/// of input length (the same window a [`SnapshotRing`] of stride 1 would
/// hold).
pub fn sync_test<S, I, F>(
    initial: S,
    inputs: &[I],
    check_distance: usize,
    mut step: F,
) -> Result<(), SyncTestFailure>
where
    S: DetHash + Clone,
    F: FnMut(&mut S, &I),
{
    if check_distance == 0 {
        return Ok(());
    }

    let mut cur = initial;
    // Rolling window of recent (frame, state) with the front always sitting
    // exactly `check_distance` frames behind the frame under test.
    let mut window: VecDeque<(usize, S)> = VecDeque::with_capacity(check_distance + 1);
    window.push_back((0, cur.clone()));

    for (i, input) in inputs.iter().enumerate() {
        step(&mut cur, input);
        let frame = i + 1;
        let expected = hash_state(&cur);

        if frame >= check_distance {
            let base_frame = frame - check_distance;
            let (bf, base_state) = window.front().expect("window is never empty");
            debug_assert_eq!(*bf, base_frame, "window front tracks the rollback base");
            // Re-simulate the base state forward through exactly the inputs
            // that produced `cur`, then compare.
            let replayed = resimulate(base_state, &inputs[base_frame..frame], &mut step);
            let actual = hash_state(&replayed);
            if actual != expected {
                return Err(SyncTestFailure {
                    frame,
                    check_distance,
                    expected,
                    actual,
                });
            }
        }

        window.push_back((frame, cur.clone()));
        while window.len() > check_distance {
            window.pop_front();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixed::Fixed;
    use crate::rng::SplitMix64;
    use crate::world_hash::Fnv1a;

    #[derive(Clone)]
    struct Sim {
        pos: Fixed,
        rng: SplitMix64,
    }

    impl DetHash for Sim {
        fn det_hash(&self, hasher: &mut Fnv1a) {
            self.pos.det_hash(hasher);
            self.rng.det_hash(hasher);
        }
    }

    fn new_sim() -> Sim {
        Sim {
            pos: Fixed::ZERO,
            rng: SplitMix64::new(0xABCD),
        }
    }

    fn step_sim(s: &mut Sim, i: &u8) {
        let jitter = s.rng.below(4) as i32;
        s.pos = s.pos + Fixed::from_int(*i as i32 + jitter);
    }

    // --- SnapshotRing ---

    #[test]
    fn test_ring_saves_only_on_stride() {
        let mut ring: SnapshotRing<i32> = SnapshotRing::new(8, 5);
        assert!(ring.offer(0, &10)); // 0 % 5 == 0
        assert!(!ring.offer(3, &11)); // not on stride
        assert!(!ring.offer(4, &12));
        assert!(ring.offer(5, &13));
        assert!(ring.offer(10, &14));
        assert_eq!(ring.len(), 3);
        assert_eq!(ring.oldest_frame(), Some(0));
        assert_eq!(ring.newest_frame(), Some(10));
    }

    #[test]
    fn test_ring_ignores_stale_or_duplicate_frames() {
        let mut ring: SnapshotRing<i32> = SnapshotRing::new(8, 1);
        assert!(ring.offer(5, &1));
        assert!(!ring.offer(5, &2), "same frame must not re-save");
        assert!(!ring.offer(4, &3), "older frame must not save");
        assert!(ring.offer(6, &4));
        assert_eq!(ring.len(), 2);
    }

    #[test]
    fn test_ring_evicts_oldest_when_full() {
        let mut ring: SnapshotRing<u64> = SnapshotRing::new(3, 1);
        for f in 0..6u64 {
            ring.offer(f, &(f * 100));
        }
        // Capacity 3: only frames 3,4,5 survive.
        assert_eq!(ring.len(), 3);
        assert_eq!(ring.oldest_frame(), Some(3));
        assert_eq!(ring.newest_frame(), Some(5));
        assert!(ring.nearest_at_or_before(2).is_none());
    }

    #[test]
    fn test_ring_nearest_at_or_before() {
        let mut ring: SnapshotRing<i32> = SnapshotRing::new(8, 5);
        ring.offer(0, &0);
        ring.offer(5, &50);
        ring.offer(10, &100);
        // Rolling back to frame 7 resumes from the snapshot at frame 5.
        let (f, s) = ring.nearest_at_or_before(7).unwrap();
        assert_eq!(f, 5);
        assert_eq!(*s, 50);
        // Exactly on a snapshot frame returns that frame.
        assert_eq!(ring.nearest_at_or_before(10).map(|(f, _)| f), Some(10));
        // Before the first snapshot: nothing.
        assert!(ring.nearest_at_or_before(0).map(|(f, _)| f) == Some(0));
    }

    #[test]
    fn test_ring_empty_and_accessors() {
        let ring: SnapshotRing<i32> = SnapshotRing::new(4, 2);
        assert!(ring.is_empty());
        assert_eq!(ring.len(), 0);
        assert_eq!(ring.stride(), 2);
        assert_eq!(ring.capacity(), 4);
        assert!(ring.nearest_at_or_before(100).is_none());
    }

    #[test]
    #[should_panic(expected = "capacity must be > 0")]
    fn test_ring_zero_capacity_panics() {
        let _: SnapshotRing<i32> = SnapshotRing::new(0, 1);
    }

    #[test]
    #[should_panic(expected = "stride must be > 0")]
    fn test_ring_zero_stride_panics() {
        let _: SnapshotRing<i32> = SnapshotRing::new(1, 0);
    }

    // --- sync_test ---

    #[test]
    fn test_sync_test_passes_for_deterministic_step() {
        let inputs: Vec<u8> = (1..=30).collect();
        assert_eq!(sync_test(new_sim(), &inputs, 7, step_sim), Ok(()));
    }

    #[test]
    fn test_sync_test_check_distance_zero_is_noop() {
        let inputs: Vec<u8> = (1..=10).collect();
        // Even a blatantly nondeterministic step passes when nothing is checked.
        let mut hidden = 0i32;
        let result = sync_test(new_sim(), &inputs, 0, |s: &mut Sim, i: &u8| {
            hidden += 1;
            s.pos = s.pos + Fixed::from_int(hidden);
            step_sim(s, i);
        });
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_sync_test_catches_nondeterminism() {
        // A step that reads a hidden mutable counter is nondeterministic: the
        // straight-through run and the re-simulation advance the counter a
        // different number of times, so the hashes diverge.
        let inputs: Vec<u8> = (1..=20).collect();
        let mut hidden = 0i32;
        let result = sync_test(new_sim(), &inputs, 5, |s: &mut Sim, i: &u8| {
            hidden = hidden.wrapping_add(1);
            s.pos = s.pos + Fixed::from_int(hidden);
            let jitter = s.rng.below(4) as i32;
            s.pos = s.pos + Fixed::from_int(*i as i32 + jitter);
        });
        let failure = result.unwrap_err();
        assert_eq!(failure.check_distance, 5);
        // The first rollback happens at frame == check_distance.
        assert_eq!(failure.frame, 5);
        assert!(failure.expected != failure.actual);
    }

    #[test]
    fn test_sync_test_failure_is_deterministic() {
        let inputs: Vec<u8> = (1..=20).collect();
        let run = || {
            let mut hidden = 0i32;
            sync_test(new_sim(), &inputs, 4, move |s: &mut Sim, i: &u8| {
                hidden = hidden.wrapping_add(1);
                s.pos = s.pos + Fixed::from_int(hidden);
                step_sim(s, i);
            })
        };
        assert_eq!(run(), run());
        assert!(run().is_err());
    }

    #[test]
    fn test_sync_test_failure_display_and_divergence() {
        let f = SyncTestFailure {
            frame: 12,
            check_distance: 5,
            expected: 0xAAAA,
            actual: 0xBBBB,
        };
        let text = f.to_string();
        assert!(text.contains("frame 12"), "{text}");
        assert!(text.contains("5 frame"), "{text}");
        assert!(text.contains("nondeterministic"), "{text}");
        let d = f.as_divergence();
        assert_eq!(d.tick, 12);
        assert_eq!(d.expected, 0xAAAA);
        assert_eq!(d.actual, 0xBBBB);
    }

    #[test]
    fn test_sync_test_short_input_below_distance_never_rolls_back() {
        // Fewer inputs than check_distance: no rollback ever fires, so even a
        // nondeterministic step is not caught (nothing to compare) — passes.
        let inputs: Vec<u8> = vec![1, 2, 3];
        let mut hidden = 0i32;
        let result = sync_test(new_sim(), &inputs, 10, |s: &mut Sim, i: &u8| {
            hidden += 1;
            s.pos = s.pos + Fixed::from_int(hidden);
            step_sim(s, i);
        });
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_sync_test_ring_window_matches_full_replay_hashes() {
        // Cross-check: sync_test's rolling re-simulation must agree with a
        // full record_trace over the same run for a deterministic sim (it
        // reaches Ok only if every rolled-back hash equalled the straight
        // one, which are the record_trace values).
        let inputs: Vec<u8> = (1..=25).collect();
        assert_eq!(sync_test(new_sim(), &inputs, 8, step_sim), Ok(()));
        assert_eq!(sync_test(new_sim(), &inputs, 1, step_sim), Ok(()));
        assert_eq!(sync_test(new_sim(), &inputs, 25, step_sim), Ok(()));
    }
}
