//! Desync forensics — attributing a replay divergence to its cause.
//!
//! [`crate::replay`] answers *where* two runs first disagree: [`first_divergence`]
//! names the tick and the two state hashes. That is the starting point of a
//! desync hunt, not its end — the question that actually decides the fix is
//! *whose fault* the divergence is. Two peers can produce different hashes
//! because their simulations genuinely stepped differently (a build, platform
//! or step-function bug), or because they were simply **fed different
//! inputs** (a delivery bug upstream of the simulation, which no amount of
//! simulation debugging will find). A bare hash comparison cannot tell those
//! apart; comparing the input streams as well can.
//!
//! This module is that comparison. Each side brings its recorded evidence —
//! a [`TraceEvidence`]: the run's seed, its per-tick input frames when they
//! were recorded, and its per-tick state hashes. [`analyze_traces`] then
//! produces a [`ForensicsReport`]: the first diverging tick (via
//! [`first_divergence`]), the first diverging input, how much of the run
//! agreed before that, whether the runs later reconverged, and a
//! [`ForensicsVerdict`] attributing the divergence. The report is a value,
//! never a panic — it implements [`Display`](core::fmt::Display) so it can be
//! pasted into a bug report or a CI log as-is, the operational lesson
//! Factorio's automatic desync reports taught (and the point where a
//! panic-on-mismatch sync test stops).
//!
//! The evidence triple is exactly what the persistent replay-trace format
//! stores (seed, per-tick input frames, expected hashes — see `ReplayTrace`
//! and `encode_trace` / `decode_trace` in [`crate::replay`]), so a decoded trace
//! file drops straight into a [`TraceEvidence`] with no conversion.
//!
//! **Search strategy.** The first divergence is found by a linear scan over
//! the recorded hash columns, not a binary search. Bisection pays off when
//! each probe costs a re-simulation (the Gaffer-style workflow of re-running
//! to a midpoint); here both columns are already in memory, so a single pass
//! is cheaper than the bookkeeping of a search — and it also yields the
//! matched prefix, the total divergent-tick count and the reconvergence tick
//! in the same pass structure.
//!
//! **Limits.** Attribution is only as strong as the evidence. With no input
//! frames recorded, a hash divergence is honestly reported as
//! [`ForensicsVerdict::UnexplainedDivergence`] rather than blamed on the
//! simulation; likewise, input evidence only clears the simulation for the
//! ticks it actually covers. And a "hash" here is the kit's FNV-1a world
//! hash: equality means the canonical state bytes agreed, subject to that
//! hash's collision stance (see [`crate::world_hash`]).
//!
//! ```
//! use izanagi_kit::trace_forensics::{analyze_traces, ForensicsVerdict, TraceEvidence};
//!
//! let inputs: Vec<Vec<u8>> = vec![vec![1], vec![2], vec![3]];
//! let hashes = [0xA, 0xB, 0xC];
//! let mut diverged = hashes;
//! diverged[1] = 0xFF;
//! let expected = TraceEvidence {
//!     seed: 7,
//!     inputs: Some(&inputs),
//!     hashes: &hashes,
//! };
//! let actual = TraceEvidence {
//!     seed: 7,
//!     inputs: Some(&inputs),
//!     hashes: &diverged,
//! };
//! let report = analyze_traces(&expected, &actual);
//! // Same seed, same inputs, different state: the simulation itself diverged.
//! assert_eq!(report.verdict, ForensicsVerdict::SimulationDivergence);
//! assert_eq!(report.matched_prefix, 1);
//! ```

use crate::replay::{count_divergences, first_divergence, Divergence};

/// One side's recorded evidence for a forensic comparison: everything a run
/// can present to explain its state hashes.
///
/// - `seed` — the run's identity, as in [`crate::replay::DesyncReport`].
///   Two traces with different seeds are different runs, and comparing their
///   hashes is meaningless; [`analyze_traces`] reports that first.
/// - `inputs` — the input frame for each tick, in tick order (frame *i* was
///   applied at tick *i* and produced hash *i*, the [`crate::replay::record_trace`]
///   alignment), or `None` when the inputs were not recorded. Attribution
///   needs them: without input evidence a divergence cannot be cleared of
///   being the simulation's fault, nor blamed on it.
/// - `hashes` — the per-tick state hashes, in tick order.
#[derive(Clone, Copy, Debug)]
pub struct TraceEvidence<'a, I> {
    /// The run's seed, echoed verbatim.
    pub seed: u64,
    /// The per-tick input frames, if they were recorded.
    pub inputs: Option<&'a [I]>,
    /// The per-tick state hashes, in tick order.
    pub hashes: &'a [u64],
}

/// The attribution a [`ForensicsReport`] reaches — whose fault a divergence
/// is, or why that cannot be told from the evidence presented.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ForensicsVerdict {
    /// The traces agree everywhere: same seed, identical hashes, and —
    /// where recorded — identical inputs. There is nothing to explain.
    InSync,
    /// The seeds differ: the two traces are not the same run at all, so
    /// their hashes were never expected to agree. (The report's remaining
    /// fields still describe the hash columns factually.)
    DifferentRuns,
    /// The input streams themselves first diverged at or before the first
    /// diverging hash (or the hashes never diverged at all): the runs were
    /// fed different inputs, which fully explains different states. Look
    /// upstream — input recording, delivery or prediction — not at the
    /// simulation.
    InputDivergence,
    /// The inputs agree through the diverging tick — including the input
    /// that produced it — yet the states differ: a true desync. The
    /// simulation, its build, or its platform stepped differently.
    SimulationDivergence,
    /// The hashes genuinely diverge, but the evidence cannot attribute it:
    /// the input frames were not recorded, or they end before the diverging
    /// tick. Record the inputs (or more of them) and compare again.
    UnexplainedDivergence,
    /// Every shared tick agrees, but one trace has more ticks than the
    /// other: one run simply lasted longer, or its recording was cut short.
    /// Neither simulation is implicated.
    LengthMismatch,
}

impl core::fmt::Display for ForensicsVerdict {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            ForensicsVerdict::InSync => "in sync",
            ForensicsVerdict::DifferentRuns => "different runs",
            ForensicsVerdict::InputDivergence => "input divergence",
            ForensicsVerdict::SimulationDivergence => "simulation divergence",
            ForensicsVerdict::UnexplainedDivergence => "unexplained divergence",
            ForensicsVerdict::LengthMismatch => "length mismatch",
        })
    }
}

/// The result of comparing two traces: the verdict, plus every fact it was
/// reached from, so a reader (or a CI gate) can check the attribution rather
/// than trust it. Built by [`analyze_traces`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForensicsReport {
    /// Whose fault the divergence is — see [`ForensicsVerdict`].
    pub verdict: ForensicsVerdict,
    /// The expected (reference) trace's seed.
    pub expected_seed: u64,
    /// The actual (observed) trace's seed.
    pub actual_seed: u64,
    /// How many ticks of hashes the expected trace holds.
    pub expected_ticks: usize,
    /// How many ticks of hashes the actual trace holds.
    pub actual_ticks: usize,
    /// How many input frames the expected trace recorded, if it recorded
    /// any.
    pub expected_input_frames: Option<usize>,
    /// How many input frames the actual trace recorded, if it recorded any.
    pub actual_input_frames: Option<usize>,
    /// The number of leading ticks whose hashes agree — the length of the
    /// run that is *not* in question. Equals the first diverging tick when
    /// there is one, and the full trace length when there is none.
    pub matched_prefix: usize,
    /// The first tick where the state hashes disagree (a missing hash counts
    /// as a disagreement), exactly as [`first_divergence`] reports it;
    /// `None` iff the hash columns are identical.
    pub first_divergence: Option<Divergence>,
    /// The first tick where the recorded input frames disagree (a missing
    /// frame counts), as [`first_input_divergence`] reports it; `None` when
    /// the inputs are identical or were not recorded on both sides.
    pub first_input_divergence: Option<usize>,
    /// How many ticks' hashes disagree in total, counting ticks present in
    /// only one trace (the [`count_divergences`] convention).
    pub divergent_ticks: usize,
    /// The first tick *after* the first divergence where the hashes agree
    /// again, if any: a divergence that heals is a transient (one bad frame,
    /// since corrected), one that never heals is a fork. `None` when there
    /// is no divergence, or none heals within both traces' lengths.
    pub reconverged_at: Option<usize>,
}

impl ForensicsReport {
    /// Returns `true` when the comparison found nothing to explain
    /// ([`ForensicsVerdict::InSync`]).
    #[inline]
    pub fn is_in_sync(&self) -> bool {
        self.verdict == ForensicsVerdict::InSync
    }
}

impl core::fmt::Display for ForensicsReport {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "desync forensics: {}", self.verdict)?;
        if self.expected_seed == self.actual_seed {
            writeln!(f, "seed: {:#018x} (both traces)", self.expected_seed)?;
        } else {
            writeln!(
                f,
                "seeds: expected {:#018x}, actual {:#018x} (different runs)",
                self.expected_seed, self.actual_seed
            )?;
        }
        writeln!(
            f,
            "ticks: expected {}, actual {} (matched prefix {})",
            self.expected_ticks, self.actual_ticks, self.matched_prefix
        )?;
        match self.first_divergence {
            Some(d) => writeln!(f, "first divergence: {d}")?,
            None => writeln!(f, "first divergence: none")?,
        }
        match (self.expected_input_frames, self.actual_input_frames) {
            (Some(e), Some(a)) => {
                write!(f, "inputs: expected {e} frame(s), actual {a} frame(s)")?;
                match self.first_input_divergence {
                    Some(tick) => writeln!(f, "; first input divergence at tick {tick}")?,
                    None => writeln!(f, "; inputs identical")?,
                }
            }
            (Some(e), None) => writeln!(f, "inputs: expected {e} frame(s), actual not recorded")?,
            (None, Some(a)) => writeln!(f, "inputs: expected not recorded, actual {a} frame(s)")?,
            (None, None) => writeln!(f, "inputs: not recorded")?,
        }
        write!(f, "divergent ticks: {}", self.divergent_ticks)?;
        if let Some(tick) = self.reconverged_at {
            write!(f, "; reconverged at tick {tick}")?;
        } else if self.first_divergence.is_some() {
            write!(f, "; reconverged: never")?;
        }
        Ok(())
    }
}

/// The first tick where two input streams disagree: either the frames at
/// that tick differ, or one stream has ended (a missing frame counts, the
/// same convention [`first_divergence`] uses for hashes). `None` iff the
/// streams are identical — same length, every frame equal.
pub fn first_input_divergence<I: PartialEq>(expected: &[I], actual: &[I]) -> Option<usize> {
    let ticks = expected.len().max(actual.len());
    (0..ticks).find(|&tick| expected.get(tick) != actual.get(tick))
}

/// The first tick at which both streams *have* a frame and the frames
/// differ. Unlike [`first_input_divergence`], one stream simply ending is
/// not a value divergence — an absent frame cannot explain a different
/// state the way a different frame can.
fn first_input_value_divergence<I: PartialEq>(expected: &[I], actual: &[I]) -> Option<usize> {
    let ticks = expected.len().min(actual.len());
    (0..ticks).find(|&tick| expected[tick] != actual[tick])
}

/// Compare two recorded traces and attribute their first divergence.
///
/// The verdict is reached in a fixed order, so it can be reasoned about:
///
/// 1. Different seeds ⇒ [`DifferentRuns`](ForensicsVerdict::DifferentRuns):
///    the traces were never the same run.
/// 2. Identical hash columns ⇒ [`InSync`](ForensicsVerdict::InSync), unless
///    the recorded inputs differ, which is itself the finding:
///    [`InputDivergence`](ForensicsVerdict::InputDivergence) (the inputs
///    disagreed; the states, over the recorded span, did not).
/// 3. A first hash divergence *beyond* every shared tick (one trace is a
///    strict prefix of the other) ⇒
///    [`LengthMismatch`](ForensicsVerdict::LengthMismatch).
/// 4. A genuine in-span hash divergence at tick *t*: if both sides recorded
///    inputs and the frames first *differ* (not merely end) at or before
///    *t*, the inputs explain it ⇒
///    [`InputDivergence`](ForensicsVerdict::InputDivergence). If the frames
///    agree through *t* — the input that produced the diverging state
///    included — ⇒ [`SimulationDivergence`](ForensicsVerdict::SimulationDivergence).
///    If the input evidence is absent, or ends at or before *t*, the
///    divergence cannot honestly be attributed ⇒
///    [`UnexplainedDivergence`](ForensicsVerdict::UnexplainedDivergence).
///
/// The analysis is a pure function of the two evidence values: same traces,
/// same report, on every platform.
pub fn analyze_traces<I: PartialEq>(
    expected: &TraceEvidence<'_, I>,
    actual: &TraceEvidence<'_, I>,
) -> ForensicsReport {
    let hash_divergence = first_divergence(expected.hashes, actual.hashes).err();
    let input_divergence = match (expected.inputs, actual.inputs) {
        (Some(e), Some(a)) => first_input_divergence(e, a),
        _ => None,
    };
    let matched_prefix = match hash_divergence {
        Some(d) => d.tick,
        None => expected.hashes.len(),
    };
    let divergent_ticks = count_divergences(expected.hashes, actual.hashes);
    let reconverged_at = match hash_divergence {
        Some(d) => {
            let overlap = expected.hashes.len().min(actual.hashes.len());
            (d.tick.saturating_add(1)..overlap)
                .find(|&tick| expected.hashes[tick] == actual.hashes[tick])
        }
        None => None,
    };
    let verdict = classify(expected, actual, hash_divergence, input_divergence);
    ForensicsReport {
        verdict,
        expected_seed: expected.seed,
        actual_seed: actual.seed,
        expected_ticks: expected.hashes.len(),
        actual_ticks: actual.hashes.len(),
        expected_input_frames: expected.inputs.map(|frames| frames.len()),
        actual_input_frames: actual.inputs.map(|frames| frames.len()),
        matched_prefix,
        first_divergence: hash_divergence,
        first_input_divergence: input_divergence,
        divergent_ticks,
        reconverged_at,
    }
}

/// The verdict half of [`analyze_traces`], kept separate so the decision
/// order documented there is the code, in one place.
fn classify<I: PartialEq>(
    expected: &TraceEvidence<'_, I>,
    actual: &TraceEvidence<'_, I>,
    hash_divergence: Option<Divergence>,
    input_divergence: Option<usize>,
) -> ForensicsVerdict {
    if expected.seed != actual.seed {
        return ForensicsVerdict::DifferentRuns;
    }
    let divergence = match hash_divergence {
        Some(d) => d,
        None => {
            // Identical hash columns (first_divergence's Ok means same
            // length, same values). Differing inputs are then the only
            // finding there is.
            return match input_divergence {
                Some(_) => ForensicsVerdict::InputDivergence,
                None => ForensicsVerdict::InSync,
            };
        }
    };
    // A divergence at the first tick one trace does not have, with every
    // shared tick in agreement, is a difference in length, not in states.
    let overlap = expected.hashes.len().min(actual.hashes.len());
    if divergence.tick >= overlap {
        return ForensicsVerdict::LengthMismatch;
    }
    match (expected.inputs, actual.inputs) {
        (Some(e), Some(a)) => match first_input_value_divergence(e, a) {
            Some(tick) if tick <= divergence.tick => ForensicsVerdict::InputDivergence,
            _ => {
                // No differing frame at or before the diverging tick. The
                // simulation is cleared only for the span the input evidence
                // actually covers; past its end the honest verdict is that
                // the evidence ran out.
                let covered = e.len().min(a.len());
                if divergence.tick < covered {
                    ForensicsVerdict::SimulationDivergence
                } else {
                    ForensicsVerdict::UnexplainedDivergence
                }
            }
        },
        _ => ForensicsVerdict::UnexplainedDivergence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixed::Fixed;
    use crate::replay::record_trace;
    use crate::rng::SplitMix64;
    use crate::world_hash::{DetHash, Fnv1a};

    /// Two traces over the same five ticks, identical except where a test
    /// perturbs them.
    fn evidence<'a>(
        seed: u64,
        inputs: Option<&'a [Vec<u8>]>,
        hashes: &'a [u64],
    ) -> TraceEvidence<'a, Vec<u8>> {
        TraceEvidence {
            seed,
            inputs,
            hashes,
        }
    }

    fn frames(values: &[u8]) -> Vec<Vec<u8>> {
        values.iter().map(|v| vec![*v]).collect()
    }

    #[test]
    fn identical_traces_are_in_sync() {
        let inputs = frames(&[1, 2, 3, 4, 5]);
        let hashes = [0xA, 0xB, 0xC, 0xD, 0xE];
        let report = analyze_traces(
            &evidence(9, Some(&inputs), &hashes),
            &evidence(9, Some(&inputs), &hashes),
        );
        assert_eq!(report.verdict, ForensicsVerdict::InSync);
        assert!(report.is_in_sync());
        assert_eq!(report.matched_prefix, 5);
        assert_eq!(report.first_divergence, None);
        assert_eq!(report.first_input_divergence, None);
        assert_eq!(report.divergent_ticks, 0);
        assert_eq!(report.reconverged_at, None);
        assert_eq!(report.expected_ticks, 5);
        assert_eq!(report.actual_ticks, 5);
        assert_eq!(report.expected_input_frames, Some(5));
        assert_eq!(report.actual_input_frames, Some(5));
    }

    #[test]
    fn empty_traces_are_in_sync() {
        let report = analyze_traces(&evidence(1, None, &[]), &evidence(1, None, &[]));
        assert_eq!(report.verdict, ForensicsVerdict::InSync);
        assert_eq!(report.matched_prefix, 0);
        assert_eq!(report.expected_input_frames, None);
    }

    #[test]
    fn same_inputs_different_state_is_a_simulation_divergence() {
        let inputs = frames(&[1, 2, 3, 4, 5]);
        let expected_hashes = [0xA, 0xB, 0xC, 0xD, 0xE];
        // From tick 2 on the states never agree again: a fork, not a blip.
        let actual_hashes = [0xA, 0xB, 0x1C, 0x1D, 0x1E];
        let report = analyze_traces(
            &evidence(9, Some(&inputs), &expected_hashes),
            &evidence(9, Some(&inputs), &actual_hashes),
        );
        assert_eq!(report.verdict, ForensicsVerdict::SimulationDivergence);
        assert!(!report.is_in_sync());
        assert_eq!(
            report.first_divergence,
            Some(Divergence {
                tick: 2,
                expected: 0xC,
                actual: 0x1C,
            })
        );
        assert_eq!(report.matched_prefix, 2);
        assert_eq!(report.first_input_divergence, None);
        assert_eq!(report.divergent_ticks, 3);
        assert_eq!(report.reconverged_at, None);
    }

    #[test]
    fn transient_divergence_reports_its_reconvergence() {
        let inputs = frames(&[1, 2, 3, 4, 5]);
        let expected_hashes = [0xA, 0xB, 0xC, 0xD, 0xE];
        let actual_hashes = [0xA, 0xB, 0xFF, 0xD, 0xE];
        let report = analyze_traces(
            &evidence(9, Some(&inputs), &expected_hashes),
            &evidence(9, Some(&inputs), &actual_hashes),
        );
        assert_eq!(report.verdict, ForensicsVerdict::SimulationDivergence);
        assert_eq!(report.divergent_ticks, 1);
        assert_eq!(report.reconverged_at, Some(3));
    }

    #[test]
    fn earlier_input_divergence_explains_the_hash_divergence() {
        let expected_inputs = frames(&[1, 2, 3, 4, 5]);
        let actual_inputs = frames(&[1, 9, 3, 4, 5]);
        let expected_hashes = [0xA, 0xB, 0xC, 0xD, 0xE];
        // The hash only shows the damage two ticks after the bad input.
        let actual_hashes = [0xA, 0xB, 0xC, 0x1D, 0x1E];
        let report = analyze_traces(
            &evidence(9, Some(&expected_inputs), &expected_hashes),
            &evidence(9, Some(&actual_inputs), &actual_hashes),
        );
        assert_eq!(report.verdict, ForensicsVerdict::InputDivergence);
        assert_eq!(report.first_input_divergence, Some(1));
        assert_eq!(
            report.first_divergence.map(|d| d.tick),
            Some(3),
            "the state divergence is real — it is just not the simulation's fault"
        );
    }

    #[test]
    fn input_divergence_at_the_diverging_tick_still_explains_it() {
        let expected_inputs = frames(&[1, 2, 3]);
        let actual_inputs = frames(&[1, 2, 8]);
        let expected_hashes = [0xA, 0xB, 0xC];
        let actual_hashes = [0xA, 0xB, 0x1C];
        let report = analyze_traces(
            &evidence(9, Some(&expected_inputs), &expected_hashes),
            &evidence(9, Some(&actual_inputs), &actual_hashes),
        );
        // The hash at tick 2 is the state *after* input 2 was applied, and
        // input 2 is exactly what differed.
        assert_eq!(report.verdict, ForensicsVerdict::InputDivergence);
        assert_eq!(report.first_input_divergence, Some(2));
    }

    #[test]
    fn input_divergence_without_hash_divergence_is_still_reported() {
        let expected_inputs = frames(&[1, 2, 3]);
        let actual_inputs = frames(&[1, 7, 3]);
        let hashes = [0xA, 0xB, 0xC];
        let report = analyze_traces(
            &evidence(9, Some(&expected_inputs), &hashes),
            &evidence(9, Some(&actual_inputs), &hashes),
        );
        // The differing input happened not to move the state (a cosmetic
        // input, say) — the finding is the input mismatch itself.
        assert_eq!(report.verdict, ForensicsVerdict::InputDivergence);
        assert_eq!(report.first_divergence, None);
        assert_eq!(report.first_input_divergence, Some(1));
        assert_eq!(report.matched_prefix, 3);
    }

    #[test]
    fn seed_mismatch_means_different_runs() {
        let hashes_a = [0xA, 0xB];
        let hashes_b = [0xA, 0x1B];
        let report = analyze_traces(&evidence(1, None, &hashes_a), &evidence(2, None, &hashes_b));
        assert_eq!(report.verdict, ForensicsVerdict::DifferentRuns);
        assert_eq!(report.expected_seed, 1);
        assert_eq!(report.actual_seed, 2);
        // The factual fields still describe the columns as they are.
        assert_eq!(report.first_divergence.map(|d| d.tick), Some(1));
    }

    #[test]
    fn a_prefix_trace_is_a_length_mismatch_not_a_desync() {
        let inputs = frames(&[1, 2, 3, 4, 5]);
        let short_inputs = frames(&[1, 2, 3]);
        let hashes = [0xA, 0xB, 0xC, 0xD, 0xE];
        let report = analyze_traces(
            &evidence(9, Some(&inputs), &hashes),
            &evidence(9, Some(&short_inputs), &hashes[..3]),
        );
        assert_eq!(report.verdict, ForensicsVerdict::LengthMismatch);
        assert_eq!(report.matched_prefix, 3);
        assert_eq!(
            report.first_divergence,
            Some(Divergence {
                tick: 3,
                expected: 0xD,
                actual: 0,
            })
        );
        assert_eq!(report.reconverged_at, None);
    }

    #[test]
    fn missing_input_evidence_leaves_the_divergence_unexplained() {
        let expected_hashes = [0xA, 0xB, 0xC];
        let actual_hashes = [0xA, 0x1B, 0xC];
        let report = analyze_traces(
            &evidence(9, None, &expected_hashes),
            &evidence(9, None, &actual_hashes),
        );
        assert_eq!(report.verdict, ForensicsVerdict::UnexplainedDivergence);
        assert_eq!(report.first_input_divergence, None);
        assert_eq!(report.expected_input_frames, None);
    }

    #[test]
    fn input_evidence_ending_before_the_divergence_is_unexplained() {
        // Inputs recorded for the first two ticks only; the states diverge
        // at tick 4, past the end of the evidence.
        let inputs = frames(&[1, 2]);
        let expected_hashes = [0xA, 0xB, 0xC, 0xD, 0xE];
        let actual_hashes = [0xA, 0xB, 0xC, 0xD, 0x1E];
        let report = analyze_traces(
            &evidence(9, Some(&inputs), &expected_hashes),
            &evidence(9, Some(&inputs), &actual_hashes),
        );
        assert_eq!(report.verdict, ForensicsVerdict::UnexplainedDivergence);
    }

    #[test]
    fn first_input_divergence_cases() {
        assert_eq!(first_input_divergence(&[1, 2, 3], &[1, 2, 3]), None);
        assert_eq!(first_input_divergence(&[1, 2, 3], &[1, 9, 3]), Some(1));
        // A stream that runs out diverges at the first missing frame.
        assert_eq!(first_input_divergence(&[1, 2, 3], &[1, 2]), Some(2));
        assert_eq!(first_input_divergence(&[1, 2], &[1, 2, 3]), Some(2));
        let empty: &[i32] = &[];
        assert_eq!(first_input_divergence(empty, empty), None);
        // Byte frames — the persisted trace's frame type — behave the same.
        let a = frames(&[5, 6]);
        let b = frames(&[5, 7]);
        assert_eq!(first_input_divergence(&a, &b), Some(1));
    }

    #[test]
    fn analysis_is_a_pure_function_of_the_evidence() {
        let inputs = frames(&[1, 2, 3]);
        let expected_hashes = [0xA, 0xB, 0xC];
        let actual_hashes = [0xA, 0x1B, 0xC];
        let first = analyze_traces(
            &evidence(9, Some(&inputs), &expected_hashes),
            &evidence(9, Some(&inputs), &actual_hashes),
        );
        let second = analyze_traces(
            &evidence(9, Some(&inputs), &expected_hashes),
            &evidence(9, Some(&inputs), &actual_hashes),
        );
        assert_eq!(first, second, "same traces in, same report out");
    }

    #[test]
    fn report_display_names_the_verdict_tick_and_hashes() {
        let inputs = frames(&[1, 2, 3, 4, 5]);
        let expected_hashes = [0xA, 0xB, 0xC, 0xD, 0xE];
        let actual_hashes = [0xA, 0xB, 0x1C, 0x1D, 0x1E];
        let report = analyze_traces(
            &evidence(0x1234, Some(&inputs), &expected_hashes),
            &evidence(0x1234, Some(&inputs), &actual_hashes),
        );
        let text = report.to_string();
        assert!(text.contains("simulation divergence"), "got: {text}");
        assert!(text.contains("0x0000000000001234"), "got: {text}");
        assert!(text.contains("tick 2"), "got: {text}");
        assert!(text.contains("0x000000000000001c"), "got: {text}");
        assert!(text.contains("reconverged: never"), "got: {text}");

        let in_sync = analyze_traces(
            &evidence(3, Some(&inputs), &expected_hashes),
            &evidence(3, Some(&inputs), &expected_hashes),
        );
        let text = in_sync.to_string();
        assert!(text.contains("in sync"), "got: {text}");
        assert!(text.contains("first divergence: none"), "got: {text}");
    }

    /// The tiny simulation from `replay`'s own tests: a fixed-point position
    /// driven by a seeded RNG, built only from kit value types.
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

    fn step(s: &mut Sim, input: &i32) {
        let jitter = s.rng.range(0, 3);
        s.pos = s.pos + Fixed::from_int(*input + jitter);
    }

    #[test]
    fn recorded_traces_attribute_as_expected() {
        let inputs = [1, 2, 3, 4, 5];
        let new_sim = || Sim {
            pos: Fixed::ZERO,
            rng: SplitMix64::new(0x1234),
        };
        let expected_hashes = record_trace(&mut new_sim(), &inputs, step);

        // Same inputs, one recorded hash corrupted afterwards: the runs were
        // fed identically, so the divergence belongs to the record/simulation
        // side — the forensic verdict for a tampered or desynced trace.
        let mut corrupted = expected_hashes.clone();
        corrupted[3] ^= 0xFFFF;
        let report = analyze_traces(
            &TraceEvidence {
                seed: 0x1234,
                inputs: Some(&inputs),
                hashes: &expected_hashes,
            },
            &TraceEvidence {
                seed: 0x1234,
                inputs: Some(&inputs),
                hashes: &corrupted,
            },
        );
        assert_eq!(report.verdict, ForensicsVerdict::SimulationDivergence);
        assert_eq!(report.first_divergence.map(|d| d.tick), Some(3));

        // A genuinely different input stream: the states diverge, but the
        // inputs explain it.
        let mut other_inputs = inputs;
        other_inputs[2] = 30;
        let other_hashes = record_trace(&mut new_sim(), &other_inputs, step);
        let report = analyze_traces(
            &TraceEvidence {
                seed: 0x1234,
                inputs: Some(&inputs),
                hashes: &expected_hashes,
            },
            &TraceEvidence {
                seed: 0x1234,
                inputs: Some(&other_inputs),
                hashes: &other_hashes,
            },
        );
        assert_eq!(report.verdict, ForensicsVerdict::InputDivergence);
        assert_eq!(report.first_input_divergence, Some(2));
    }
}
