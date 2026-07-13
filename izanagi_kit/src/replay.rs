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
    /// 0-based step index where the traces first disagreed.
    pub tick: usize,
    /// The expected (reference) state hash at that tick.
    pub expected: u64,
    /// The actual (observed) state hash at that tick.
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

/// Percentage of ticks (0–100) at which the two traces diverge.
///
/// Ticks beyond the shorter trace count as divergences. Returns `0` when both
/// traces are empty. Result is integer-rounded toward zero — use this for
/// "reject if > 5% of ticks diverged" CI gates and replay-quality dashboards
/// without floating-point arithmetic.
pub fn divergence_percent(expected: &[u64], actual: &[u64]) -> u32 {
    let total = expected.len().max(actual.len());
    if total == 0 {
        return 0;
    }
    let diverged = count_divergences(expected, actual);
    (diverged.saturating_mul(100) / total) as u32
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

// ─────────────────────────────────────────────────────────────────────────
// Subsystem-localized divergence
// ─────────────────────────────────────────────────────────────────────────

/// Where two *labeled* replay traces first disagree — the tick **and the
/// subsystem** within it. This is the localization a plain [`Divergence`]
/// cannot give: a per-tick `u64` says "tick 4207 differs", a labeled trace
/// says "tick 4207, subsystem `enemies` differs" — the difference between an
/// afternoon of bisection and a one-line fix.
///
/// A labeled trace is one [`LabeledDigest`](crate::world_hash::LabeledDigest)'s
/// `parts()` per tick: `&[&[(&'static str, u64)]]`. Build each tick's digest by
/// hashing your subsystems under stable labels (see `LabeledDigest`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LabeledDivergence {
    /// 0-based tick where the traces first disagreed.
    pub tick: usize,
    /// The first subsystem (by position within the tick) that differed.
    /// `"<missing>"` if one trace had fewer subsystems at this tick;
    /// `"<label-mismatch>"` if the two traces labeled position *i* differently
    /// (a structural mismatch — the traces don't describe the same world).
    pub subsystem: &'static str,
    /// The expected (reference) subsystem hash (`0` if absent).
    pub expected: u64,
    /// The actual (observed) subsystem hash (`0` if absent).
    pub actual: u64,
}

impl core::fmt::Display for LabeledDivergence {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "replay divergence at tick {}, subsystem '{}': expected {:#018x}, got {:#018x}",
            self.tick, self.subsystem, self.expected, self.actual
        )
    }
}

/// Compare two labeled traces tick by tick, and within each tick subsystem by
/// subsystem (in order), returning the earliest disagreement.
///
/// Each element of a trace is the `parts()` of that tick's
/// [`LabeledDigest`](crate::world_hash::LabeledDigest). At each tick the two
/// part-lists are walked in parallel:
/// - a differing hash at the same label ⇒ that subsystem diverged;
/// - the same position labeled differently ⇒ `"<label-mismatch>"` (the traces
///   aren't describing the same set of subsystems — itself a bug);
/// - one list shorter than the other ⇒ `"<missing>"` at the extra position.
///
/// A tick present in only one trace (length mismatch between the two traces)
/// reports `"<missing>"` at that tick. `Ok(())` iff every tick's part-list is
/// identical in labels, order, and hashes.
pub fn first_divergence_labeled(
    expected: &[&[(&'static str, u64)]],
    actual: &[&[(&'static str, u64)]],
) -> Result<(), LabeledDivergence> {
    let ticks = expected.len().max(actual.len());
    for tick in 0..ticks {
        match (expected.get(tick), actual.get(tick)) {
            (Some(e), Some(a)) => {
                let n = e.len().max(a.len());
                for i in 0..n {
                    match (e.get(i), a.get(i)) {
                        (Some(&(el, eh)), Some(&(al, ah))) => {
                            if el != al {
                                return Err(LabeledDivergence {
                                    tick,
                                    subsystem: "<label-mismatch>",
                                    expected: eh,
                                    actual: ah,
                                });
                            }
                            if eh != ah {
                                return Err(LabeledDivergence {
                                    tick,
                                    subsystem: el,
                                    expected: eh,
                                    actual: ah,
                                });
                            }
                        }
                        (Some(&(el, eh)), None) => {
                            return Err(LabeledDivergence {
                                tick,
                                subsystem: el,
                                expected: eh,
                                actual: 0,
                            });
                        }
                        (None, Some(&(al, ah))) => {
                            return Err(LabeledDivergence {
                                tick,
                                subsystem: al,
                                expected: 0,
                                actual: ah,
                            });
                        }
                        (None, None) => unreachable!("i < max(len) so at least one side is Some"),
                    }
                }
            }
            // One trace ran longer than the other: the whole tick is missing.
            (Some(_), None) | (None, Some(_)) => {
                return Err(LabeledDivergence {
                    tick,
                    subsystem: "<missing>",
                    expected: 0,
                    actual: 0,
                });
            }
            (None, None) => unreachable!("tick < max(len) so at least one side is Some"),
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Production desync reporting
// ─────────────────────────────────────────────────────────────────────────

/// A self-contained desync **repro bundle** — For Honor's operational lesson
/// (GDC 2019, "Networking For Honor"): a desync you cannot reproduce is a
/// desync you cannot fix, so the moment one is detected, capture everything a
/// developer needs to replay it *in the same artifact that reports it*.
///
/// The bundle carries the divergence itself, the subsystem localization when
/// labeled traces were available, the run's seed, and the window of inputs
/// leading up to (and including) the diverging tick. With the kit's
/// determinism guarantee, `seed + recent_inputs` replayed from the preceding
/// snapshot reproduces the desync exactly.
///
/// Build one with [`desync_report`] (plain traces) or
/// [`desync_report_labeled`] (subsystem-labeled traces).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesyncReport<I> {
    /// Where the traces first disagreed (tick + expected/actual hashes).
    pub divergence: Divergence,
    /// The first diverging subsystem, when the traces were labeled
    /// ([`desync_report_labeled`]); `None` for plain `u64` traces.
    pub subsystem: Option<&'static str>,
    /// The run's identity — typically the world seed. Echoed verbatim from
    /// the caller so the report is replayable standalone.
    pub seed: u64,
    /// The inputs for the ticks leading up to and **including** the diverging
    /// tick (at most the `window` most recent; input *i* produced trace entry
    /// *i*, matching [`record_trace`]).
    pub recent_inputs: Vec<I>,
    /// The tick index of `recent_inputs[0]` — where the captured window
    /// starts. Replaying the window from a snapshot of tick
    /// `first_input_tick - 1` reproduces the divergence.
    pub first_input_tick: usize,
}

impl<I> core::fmt::Display for DesyncReport<I> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.divergence)?;
        if let Some(subsystem) = self.subsystem {
            write!(f, " (subsystem '{}')", subsystem)?;
        }
        write!(
            f,
            "; seed {:#018x}; {} input(s) captured from tick {}",
            self.seed,
            self.recent_inputs.len(),
            self.first_input_tick
        )
    }
}

/// Clip the input window for a divergence at `tick`: at most `window` inputs
/// ending at (and including) the diverging tick.
fn clip_input_window<I: Clone>(inputs: &[I], tick: usize, window: usize) -> (Vec<I>, usize) {
    // Inclusive end: input `tick` produced the diverging trace entry. The
    // divergence may lie beyond the inputs we hold (length-mismatch case).
    let end = (tick + 1).min(inputs.len());
    let first = end.saturating_sub(window);
    (inputs[first..end].to_vec(), first)
}

/// Compare two plain replay traces and, if they diverge, bundle the evidence
/// into a [`DesyncReport`]: the divergence, the run's `seed`, and the at most
/// `window` most recent inputs up to and including the diverging tick.
/// Returns `None` iff the traces are identical.
///
/// `inputs` is the same slice the traces were recorded from ([`record_trace`]
/// alignment: input *i* → trace entry *i*). A `window` of `0` captures no
/// inputs (the report still localizes the tick).
pub fn desync_report<I: Clone>(
    expected: &[u64],
    actual: &[u64],
    seed: u64,
    inputs: &[I],
    window: usize,
) -> Option<DesyncReport<I>> {
    let divergence = first_divergence(expected, actual).err()?;
    let (recent_inputs, first_input_tick) = clip_input_window(inputs, divergence.tick, window);
    Some(DesyncReport {
        divergence,
        subsystem: None,
        seed,
        recent_inputs,
        first_input_tick,
    })
}

/// [`desync_report`] for **labeled** traces: additionally pins the first
/// diverging subsystem (via [`first_divergence_labeled`]), so the report
/// names both the tick and the subsystem to look at.
pub fn desync_report_labeled<I: Clone>(
    expected: &[&[(&'static str, u64)]],
    actual: &[&[(&'static str, u64)]],
    seed: u64,
    inputs: &[I],
    window: usize,
) -> Option<DesyncReport<I>> {
    let labeled = first_divergence_labeled(expected, actual).err()?;
    let (recent_inputs, first_input_tick) = clip_input_window(inputs, labeled.tick, window);
    Some(DesyncReport {
        divergence: Divergence {
            tick: labeled.tick,
            expected: labeled.expected,
            actual: labeled.actual,
        },
        subsystem: Some(labeled.subsystem),
        seed,
        recent_inputs,
        first_input_tick,
    })
}

/// What a session should do about a confirmed desync — the recovery
/// vocabulary from For Honor's production postmortem (GDC 2019). The kit
/// deliberately ships the *vocabulary*, not the decision: which policy
/// applies is a game/netcode judgement (how authoritative is each peer, can
/// state be re-sent, how disruptive is a kick), so the session layer picks a
/// variant and acts on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DesyncPolicy {
    /// Re-send authoritative state to the diverged peer and resume — least
    /// disruptive; requires a trusted source of truth and transferable state.
    Resync,
    /// Remove the diverged peer and continue the session for everyone else.
    Kick,
    /// End the session for all peers — the honest option when no peer is
    /// authoritative (pure lockstep) and the simulations cannot be reconciled.
    Disband,
}

impl core::fmt::Display for DesyncPolicy {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            DesyncPolicy::Resync => "resync",
            DesyncPolicy::Kick => "kick",
            DesyncPolicy::Disband => "disband",
        })
    }
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

    #[test]
    fn test_divergence_percent_zero_for_identical_traces() {
        let t = vec![1u64, 2, 3, 4];
        assert_eq!(divergence_percent(&t, &t), 0);
    }

    #[test]
    fn test_divergence_percent_100_for_all_diverged() {
        let a = vec![1u64, 2, 3];
        let b = vec![9u64, 8, 7];
        assert_eq!(divergence_percent(&a, &b), 100);
    }

    #[test]
    fn test_divergence_percent_empty_returns_zero() {
        assert_eq!(divergence_percent(&[], &[]), 0);
    }

    // --- first_divergence_labeled ---

    #[test]
    fn test_labeled_identical_traces_are_ok() {
        let t0: &[(&str, u64)] = &[("pos", 1), ("hp", 2)];
        let t1: &[(&str, u64)] = &[("pos", 3), ("hp", 4)];
        let trace = [t0, t1];
        assert_eq!(first_divergence_labeled(&trace, &trace), Ok(()));
    }

    #[test]
    fn test_labeled_localizes_diverging_subsystem() {
        // Same tick, same labels/order, but the `hp` hash differs at tick 1.
        let e0: &[(&str, u64)] = &[("pos", 1), ("hp", 2)];
        let e1: &[(&str, u64)] = &[("pos", 3), ("hp", 4)];
        let a0: &[(&str, u64)] = &[("pos", 1), ("hp", 2)];
        let a1: &[(&str, u64)] = &[("pos", 3), ("hp", 99)];
        let got = first_divergence_labeled(&[e0, e1], &[a0, a1]);
        assert_eq!(
            got,
            Err(LabeledDivergence {
                tick: 1,
                subsystem: "hp",
                expected: 4,
                actual: 99,
            })
        );
    }

    #[test]
    fn test_labeled_reports_first_subsystem_by_position() {
        // Both `pos` and `hp` differ; the earlier position (`pos`) wins.
        let e: &[(&str, u64)] = &[("pos", 1), ("hp", 2)];
        let a: &[(&str, u64)] = &[("pos", 5), ("hp", 6)];
        let got = first_divergence_labeled(&[e], &[a]);
        assert_eq!(got.unwrap_err().subsystem, "pos");
    }

    #[test]
    fn test_labeled_detects_label_mismatch() {
        // Same position labeled differently: the traces don't describe the
        // same subsystem set — a structural bug, flagged distinctly.
        let e: &[(&str, u64)] = &[("pos", 1)];
        let a: &[(&str, u64)] = &[("velocity", 1)];
        let got = first_divergence_labeled(&[e], &[a]);
        assert_eq!(got.unwrap_err().subsystem, "<label-mismatch>");
    }

    #[test]
    fn test_labeled_detects_missing_subsystem_within_tick() {
        let e: &[(&str, u64)] = &[("pos", 1), ("hp", 2)];
        let a: &[(&str, u64)] = &[("pos", 1)]; // hp absent
        let got = first_divergence_labeled(&[e], &[a]).unwrap_err();
        assert_eq!(got.subsystem, "hp");
        assert_eq!(got.expected, 2);
        assert_eq!(got.actual, 0);
    }

    #[test]
    fn test_labeled_detects_missing_tick() {
        let e0: &[(&str, u64)] = &[("pos", 1)];
        let e1: &[(&str, u64)] = &[("pos", 2)];
        let a0: &[(&str, u64)] = &[("pos", 1)];
        let got = first_divergence_labeled(&[e0, e1], &[a0]).unwrap_err();
        assert_eq!(got.tick, 1);
        assert_eq!(got.subsystem, "<missing>");
    }

    #[test]
    fn test_labeled_integrates_with_labeled_digest() {
        use crate::world_hash::LabeledDigest;
        // Build two ticks from LabeledDigest, mutate one subsystem, confirm
        // the localizer names exactly it. This is the intended end-to-end use.
        let mut good = LabeledDigest::new();
        good.add("terrain", &[1u32, 2, 3][..])
            .add("actors", &[10u32, 20][..]);
        let good_parts = good.parts().to_vec();

        let mut bad = LabeledDigest::new();
        bad.add("terrain", &[1u32, 2, 3][..])
            .add("actors", &[10u32, 21][..]); // actors moved
        let bad_parts = bad.parts().to_vec();

        let expected = [good_parts.as_slice()];
        let actual = [bad_parts.as_slice()];
        let got = first_divergence_labeled(&expected, &actual).unwrap_err();
        assert_eq!(got.subsystem, "actors");
        assert_eq!(got.tick, 0);
    }

    // --- DesyncReport ---

    #[test]
    fn test_desync_report_none_when_traces_agree() {
        let trace = [1u64, 2, 3];
        let inputs = [10u8, 20, 30];
        assert!(desync_report(&trace, &trace, 0xABCD, &inputs, 8).is_none());
    }

    #[test]
    fn test_desync_report_captures_window_ending_at_divergence() {
        let expected = [1u64, 2, 3, 4, 5];
        let actual = [1u64, 2, 3, 99, 5]; // diverges at tick 3
        let inputs = [10u8, 11, 12, 13, 14];
        let report = desync_report(&expected, &actual, 7, &inputs, 3).unwrap();
        assert_eq!(report.divergence.tick, 3);
        assert_eq!(report.divergence.expected, 4);
        assert_eq!(report.divergence.actual, 99);
        assert_eq!(report.subsystem, None);
        assert_eq!(report.seed, 7);
        // Window of 3 ending at (and including) the diverging tick's input.
        assert_eq!(report.recent_inputs, vec![11, 12, 13]);
        assert_eq!(report.first_input_tick, 1);
    }

    #[test]
    fn test_desync_report_window_larger_than_history_starts_at_zero() {
        let expected = [1u64, 2];
        let actual = [1u64, 9];
        let inputs = [10u8, 11];
        let report = desync_report(&expected, &actual, 0, &inputs, 100).unwrap();
        assert_eq!(report.recent_inputs, vec![10, 11]);
        assert_eq!(report.first_input_tick, 0);
    }

    #[test]
    fn test_desync_report_zero_window_captures_no_inputs() {
        let expected = [1u64, 2];
        let actual = [1u64, 9];
        let inputs = [10u8, 11];
        let report = desync_report(&expected, &actual, 0, &inputs, 0).unwrap();
        assert!(report.recent_inputs.is_empty());
        assert_eq!(report.first_input_tick, 2);
    }

    #[test]
    fn test_desync_report_divergence_beyond_inputs_is_clamped() {
        // Length-mismatch divergence at a tick we hold no input for.
        let expected = [1u64, 2, 3];
        let actual = [1u64, 2];
        let inputs = [10u8, 11]; // only 2 inputs held
        let report = desync_report(&expected, &actual, 0, &inputs, 8).unwrap();
        assert_eq!(report.divergence.tick, 2);
        assert_eq!(report.recent_inputs, vec![10, 11]);
        assert_eq!(report.first_input_tick, 0);
    }

    #[test]
    fn test_desync_report_labeled_names_subsystem() {
        let e: &[(&str, u64)] = &[("terrain", 1), ("actors", 2)];
        let a: &[(&str, u64)] = &[("terrain", 1), ("actors", 5)];
        let ok: &[(&str, u64)] = &[("terrain", 1), ("actors", 2)];
        let inputs = [10u8, 11];
        let report = desync_report_labeled(&[ok, e], &[ok, a], 42, &inputs, 8).unwrap();
        assert_eq!(report.divergence.tick, 1);
        assert_eq!(report.subsystem, Some("actors"));
        assert_eq!(report.divergence.expected, 2);
        assert_eq!(report.divergence.actual, 5);
        assert_eq!(report.recent_inputs, vec![10, 11]);
    }

    #[test]
    fn test_desync_report_labeled_none_when_identical() {
        let t: &[(&str, u64)] = &[("terrain", 1)];
        let inputs: [u8; 1] = [10];
        assert!(desync_report_labeled(&[t], &[t], 0, &inputs, 4).is_none());
    }

    #[test]
    fn test_desync_report_display_mentions_tick_seed_and_subsystem() {
        let e: &[(&str, u64)] = &[("actors", 2)];
        let a: &[(&str, u64)] = &[("actors", 5)];
        let inputs = [10u8];
        let report = desync_report_labeled(&[e], &[a], 0xBEEF, &inputs, 4).unwrap();
        let text = report.to_string();
        assert!(text.contains("tick 0"), "{text}");
        assert!(text.contains("actors"), "{text}");
        assert!(text.contains("0x000000000000beef"), "{text}");
        assert!(text.contains("1 input(s) captured from tick 0"), "{text}");
    }

    #[test]
    fn test_desync_policy_display() {
        assert_eq!(DesyncPolicy::Resync.to_string(), "resync");
        assert_eq!(DesyncPolicy::Kick.to_string(), "kick");
        assert_eq!(DesyncPolicy::Disband.to_string(), "disband");
    }

    #[test]
    fn test_desync_report_replays_to_reproduce_the_divergence() {
        // End-to-end: record a good trace, corrupt one input on the "peer",
        // build the report, then use ONLY the report's window + a snapshot of
        // the tick before it to re-derive the diverging hash — proving the
        // bundle is a sufficient repro artifact.
        let inputs: Vec<u8> = (1..=10).collect();
        let mut reference = new_sim();
        let step = |s: &mut Sim, i: &u8| {
            let jitter = s.rng.below(3) as i32;
            s.pos = s.pos + Fixed::from_int(*i as i32 + jitter);
        };
        let expected = record_trace(&mut reference, &inputs, step);

        let mut peer_inputs = inputs.clone();
        peer_inputs[6] = 99; // corruption at tick 6
        let mut peer = new_sim();
        let actual = record_trace(&mut peer, &peer_inputs, step);

        let report = desync_report(&expected, &actual, 0x1234, &peer_inputs, 4).unwrap();
        assert_eq!(report.divergence.tick, 6);

        // Rebuild the pre-window snapshot by replaying the prefix, then replay
        // just the report's window: the final hash must equal the "actual"
        // (diverged) hash the report captured.
        let mut snapshot = new_sim();
        for input in &peer_inputs[..report.first_input_tick] {
            step(&mut snapshot, input);
        }
        let replayed = resimulate(&snapshot, &report.recent_inputs, step);
        assert_eq!(hash_state(&replayed), report.divergence.actual);
    }
}
