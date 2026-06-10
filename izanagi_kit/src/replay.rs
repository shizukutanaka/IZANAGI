//! Deterministic replay & desync detection harness.
//!
//! The kit's reason for existing is bit-exact replay: the same initial state
//! driven by the same inputs must produce the same state, tick for tick. This
//! module turns that promise into reusable tooling, generalising what
//! `tests/determinism.rs` does by hand.
//!
//! - [`record_trace`] runs a simulation and returns the per-tick state-hash
//!   sequence (the "replay trace").
//! - [`check_trace`] re-runs against a recorded trace and reports the **first**
//!   diverging tick — the starting point for any desync hunt.
//! - [`first_divergence`] compares two traces directly (e.g. from two peers).
//! - [`resimulate`] clones a snapshot and replays inputs onto it — the basis of
//!   rollback netcode (snapshot a known-good tick, re-run newer inputs).
//!
//! The simulation is supplied as a `step(&mut state, &input)` closure, so this
//! is engine-agnostic. State is hashed via [`DetHash`], so any state built from
//! the kit's value types is replay-checkable for free.

use crate::world_hash::{hash_state, DetHash};

/// Where two replay traces first disagree. `tick` is the 0-based step index;
/// `expected`/`actual` are the state hashes there (0 if that trace was shorter).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Divergence {
    pub tick: usize,
    pub expected: u64,
    pub actual: u64,
}

impl core::fmt::Display for Divergence {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "replay divergence at tick {}: expected {:#018x}, got {:#018x}",
            self.tick, self.expected, self.actual
        )
    }
}

/// Advance `state` through `inputs` with `step`, recording the state hash after
/// each tick. The returned trace has length `inputs.len()`.
pub fn record_trace<S, I, F>(state: &mut S, inputs: &[I], mut step: F) -> Vec<u64>
where
    S: DetHash,
    F: FnMut(&mut S, &I),
{
    let mut trace = Vec::with_capacity(inputs.len());
    for input in inputs {
        step(state, input);
        trace.push(hash_state(state));
    }
    trace
}

/// Re-run `state` through `inputs` and compare against a previously recorded
/// `expected` trace, returning the first diverging tick if any. A length
/// mismatch counts as a divergence at the first missing tick.
pub fn check_trace<S, I, F>(
    state: &mut S,
    inputs: &[I],
    expected: &[u64],
    step: F,
) -> Result<(), Divergence>
where
    S: DetHash,
    F: FnMut(&mut S, &I),
{
    let actual = record_trace(state, inputs, step);
    first_divergence(expected, &actual)
}

/// Compare two state-hash traces tick by tick. `Ok(())` iff identical (same
/// length and values); otherwise the earliest disagreement. Useful for
/// comparing dumps from two peers / two builds to localise a desync.
pub fn first_divergence(expected: &[u64], actual: &[u64]) -> Result<(), Divergence> {
    let ticks = expected.len().max(actual.len());
    for tick in 0..ticks {
        let e = expected.get(tick).copied();
        let a = actual.get(tick).copied();
        if e != a {
            return Err(Divergence {
                tick,
                expected: e.unwrap_or(0),
                actual: a.unwrap_or(0),
            });
        }
    }
    Ok(())
}

/// Count the number of ticks where `expected` and `actual` disagree.
/// Ticks beyond the shorter trace count as divergences (the two runs produced
/// different lengths). Returns 0 for identical traces.
pub fn count_divergences(expected: &[u64], actual: &[u64]) -> usize {
    let ticks = expected.len().max(actual.len());
    (0..ticks)
        .filter(|&i| expected.get(i) != actual.get(i))
        .count()
}

/// Collect **all** ticks where `expected` and `actual` disagree into a
/// `Vec<Divergence>`. Unlike [`first_divergence`] (stops at the first mismatch)
/// or [`count_divergences`] (returns only a count), this returns every
/// divergence so that multiple desyncs can be inspected at once. Ticks beyond
/// the shorter trace are included as divergences (missing hash treated as `0`).
pub fn find_all_divergences(expected: &[u64], actual: &[u64]) -> Vec<Divergence> {
    let ticks = expected.len().max(actual.len());
    (0..ticks)
        .filter_map(|tick| {
            let e = expected.get(tick).copied();
            let a = actual.get(tick).copied();
            if e != a {
                Some(Divergence {
                    tick,
                    expected: e.unwrap_or(0),
                    actual: a.unwrap_or(0),
                })
            } else {
                None
            }
        })
        .collect()
}

/// Returns `true` when `expected` and `actual` are identical (no divergence).
/// Thin wrapper around [`first_divergence`] for callers that only need a boolean
/// answer — avoids `.is_ok()` boilerplate on the `Result`.
#[inline]
pub fn replay_ok(expected: &[u64], actual: &[u64]) -> bool {
    first_divergence(expected, actual).is_ok()
}

/// Replay `inputs` onto a **clone** of `snapshot`, returning the resulting
/// state and leaving `snapshot` untouched. This is the core rollback operation:
/// keep a confirmed-good snapshot, then re-simulate the inputs received since.
pub fn resimulate<S, I, F>(snapshot: &S, inputs: &[I], mut step: F) -> S
where
    S: Clone,
    F: FnMut(&mut S, &I),
{
    let mut state = snapshot.clone();
    for input in inputs {
        step(&mut state, input);
    }
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixed::Fixed;
    use crate::rng::SplitMix64;
    use crate::world_hash::Fnv1a;

    /// A tiny but representative simulation state: a fixed-point position driven
    /// by a seeded RNG. Built only from kit value types, so it hashes for free.
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
            rng: SplitMix64::new(0x1234),
        }
    }

    // Step: move by `input` units plus a small random jitter from the stream.
    fn step(s: &mut Sim, input: &i32) {
        let jitter = s.rng.range(0, 3);
        s.pos = s.pos + Fixed::from_int(*input + jitter);
    }

    #[test]
    fn test_divergence_display_contains_tick_and_hashes() {
        let d = Divergence {
            tick: 7,
            expected: 0xDEAD_BEEF_0000_1234,
            actual: 0xCAFE_BABE_5678_9ABC,
        };
        let s = d.to_string();
        assert!(s.contains("7"), "tick must appear in output");
        assert!(s.contains("0xdeadbeef00001234") || s.contains("0xDEADBEEF00001234"));
    }

    #[test]
    fn test_record_trace_is_reproducible() {
        let inputs = [1, -2, 3, 0, 5];
        let a = record_trace(&mut new_sim(), &inputs, step);
        let b = record_trace(&mut new_sim(), &inputs, step);
        assert_eq!(a, b, "same seed + inputs must reproduce the trace");
        assert_eq!(a.len(), inputs.len());
    }

    #[test]
    fn test_check_trace_accepts_a_faithful_replay() {
        let inputs = [4, 4, 4, 4];
        let expected = record_trace(&mut new_sim(), &inputs, step);
        assert_eq!(
            check_trace(&mut new_sim(), &inputs, &expected, step),
            Ok(())
        );
    }

    #[test]
    fn test_check_trace_localises_a_divergence() {
        let inputs = [1, 2, 3, 4, 5];
        let mut expected = record_trace(&mut new_sim(), &inputs, step);
        // Corrupt the hash at tick 2 → divergence must be reported there.
        expected[2] ^= 0xFFFF;
        match check_trace(&mut new_sim(), &inputs, &expected, step) {
            Err(d) => assert_eq!(d.tick, 2),
            Ok(()) => panic!("expected a divergence at tick 2"),
        }
    }

    #[test]
    fn test_first_divergence_edge_cases() {
        assert_eq!(first_divergence(&[1, 2, 3], &[1, 2, 3]), Ok(()));
        assert_eq!(
            first_divergence(&[1, 2, 3], &[1, 9, 3]),
            Err(Divergence {
                tick: 1,
                expected: 2,
                actual: 9
            })
        );
        // Shorter actual diverges at the first missing tick.
        assert_eq!(
            first_divergence(&[1, 2, 3], &[1, 2]),
            Err(Divergence {
                tick: 2,
                expected: 3,
                actual: 0
            })
        );
    }

    #[test]
    fn test_resimulate_matches_inline_run_and_preserves_snapshot() {
        let inputs = [2, 7, -3];
        // Run inline to tick 1, snapshot, finish inline.
        let mut inline = new_sim();
        step(&mut inline, &inputs[0]);
        let snapshot = inline.clone();
        for input in &inputs[1..] {
            step(&mut inline, input);
        }
        // Rollback path: resimulate the tail from the snapshot.
        let rolled = resimulate(&snapshot, &inputs[1..], step);
        assert_eq!(hash_state(&rolled), hash_state(&inline));
        // Snapshot itself is untouched (still at tick 1).
        assert_ne!(hash_state(&snapshot), hash_state(&inline));
    }

    #[test]
    fn test_count_divergences_identical_traces_is_zero() {
        let trace = vec![1u64, 2, 3, 4];
        assert_eq!(count_divergences(&trace, &trace.clone()), 0);
    }

    #[test]
    fn test_count_divergences_all_differ() {
        let a = vec![1u64, 2, 3];
        let b = vec![4u64, 5, 6];
        assert_eq!(count_divergences(&a, &b), 3);
    }

    #[test]
    fn test_count_divergences_length_mismatch_counts_extra() {
        let longer = vec![1u64, 2, 3, 4];
        let shorter = vec![1u64, 2];
        // ticks 2 and 3 are in longer but not shorter → 2 divergences
        assert_eq!(count_divergences(&longer, &shorter), 2);
    }

    #[test]
    fn test_find_all_divergences_identical_is_empty() {
        let t = vec![1u64, 2, 3];
        assert!(find_all_divergences(&t, &t).is_empty());
    }

    #[test]
    fn test_find_all_divergences_returns_all() {
        let a = vec![1u64, 2, 3];
        let b = vec![1u64, 9, 3]; // tick 1 diverges
        let divs = find_all_divergences(&a, &b);
        assert_eq!(divs.len(), 1);
        assert_eq!(divs[0].tick, 1);
        assert_eq!(divs[0].expected, 2);
        assert_eq!(divs[0].actual, 9);
    }

    #[test]
    fn test_find_all_divergences_length_mismatch_included() {
        let longer = vec![1u64, 2, 3];
        let shorter = vec![1u64];
        let divs = find_all_divergences(&longer, &shorter);
        // ticks 1 and 2 are missing from shorter → 2 divergences
        assert_eq!(divs.len(), 2);
        assert_eq!(divs[0].tick, 1);
        assert_eq!(divs[1].tick, 2);
    }

    #[test]
    fn test_replay_ok_identical_traces() {
        let trace = vec![1u64, 2, 3, 4];
        assert!(replay_ok(&trace, &trace));
    }

    #[test]
    fn test_replay_ok_false_on_divergence() {
        let a = vec![1u64, 2, 3];
        let b = vec![1u64, 9, 3];
        assert!(!replay_ok(&a, &b));
    }

    #[test]
    fn test_replay_ok_empty_traces_are_ok() {
        assert!(replay_ok(&[], &[]));
    }
}
