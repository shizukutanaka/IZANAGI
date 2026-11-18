//! Unit-test emulator: drive a simulation one tick at a time, by hand.
//!
//! The kit's replay tooling is batch-shaped: [`record_trace`](crate::replay::record_trace) runs a whole
//! input list and hands back the finished trace, [`check_trace`](crate::replay::check_trace) re-runs the
//! whole list and only then reports where it diverged. That is the right
//! shape for CI gates and peer comparison, but the wrong shape for a unit
//! test that wants to *watch* a simulation: step once, look at the state,
//! assert the hash, feed a different input than the script had planned,
//! stop exactly on the tick that breaks.
//!
//! [`SimEmulator`] is that missing handle. It owns the state and the step
//! function, advances **one tick per call**, and returns each tick's world
//! hash as it is produced — so a test can interleave stepping, inspection
//! and assertions in any order, and when an expectation fails the emulator
//! is left standing on the diverging tick with the diverged state still in
//! hand, instead of a finished run that has to be re-driven to the failure
//! point. This is the role Photon Quantum's FrameEmulator plays for its
//! (commercial, cloud-priced) deterministic stack; here it is a thin
//! wrapper over the kit's existing [`DetHash`] machinery, with no new
//! simulation semantics of its own.
//!
//! Relationship to the batch API, stated plainly:
//!
//! - [`record_trace`](crate::replay::record_trace) — "run everything, give me the trace."
//! - [`check_trace`](crate::replay::check_trace) — "run everything, then tell me the first bad tick."
//! - [`SimEmulator::verify`] — the same check as `check_trace`, but it
//!   stops *at* the first bad tick and keeps the state there.
//! - [`SimEmulator::step`] / [`SimEmulator::step_expect`] — no batch at
//!   all; the test is the driver.
//!
//! ```
//! use izanagi_kit::sim_emulator::SimEmulator;
//! use izanagi_kit::world_hash::{DetHash, Fnv1a};
//!
//! #[derive(Clone, PartialEq, Debug)]
//! struct Counter { value: i64 }
//! impl DetHash for Counter {
//!     fn det_hash(&self, h: &mut Fnv1a) { h.write_i64(self.value); }
//! }
//!
//! let mut emu = SimEmulator::new(Counter { value: 0 }, |s: &mut Counter, i: &i64| {
//!     s.value += *i;
//! });
//! let hash = emu.step(&10);
//! assert_eq!(emu.tick(), 1);
//! assert_eq!(emu.state().value, 10);
//! assert_eq!(hash, emu.current_hash());
//! ```

use std::marker::PhantomData;

use crate::replay::Divergence;
use crate::world_hash::{hash_state, DetHash};

/// A hand-driven simulation session for unit tests: owns a state and its
/// step function, advances one tick at a time, and records the world hash
/// of every tick it produces.
///
/// The step function has the same shape as [`record_trace`](crate::replay::record_trace)'s
/// (`FnMut(&mut S, &I)`), and a [`Simulation`](crate::sim::Simulation) is
/// driven by passing `|s, i| s.step(i)`. The emulator adds no behaviour of
/// its own — the hashes it returns are exactly the ones
/// [`record_trace`](crate::replay::record_trace) would produce for the same state and input sequence —
/// it only changes *when* the caller gets them: immediately, one tick at a
/// time, with the live state available for inspection between ticks.
pub struct SimEmulator<S, I, F> {
    state: S,
    step_fn: F,
    trace: Vec<u64>,
    input: PhantomData<fn(&I)>,
}

impl<S, I, F> SimEmulator<S, I, F>
where
    S: DetHash,
    F: FnMut(&mut S, &I),
{
    /// Start an emulator on `state`, before any tick. [`tick`](Self::tick)
    /// is 0 and the trace is empty; [`current_hash`](Self::current_hash)
    /// is the hash of the untouched initial state.
    pub fn new(state: S, step: F) -> Self {
        SimEmulator {
            state,
            step_fn: step,
            trace: Vec::new(),
            input: PhantomData,
        }
    }

    /// The current state — the initial state advanced by
    /// [`tick`](Self::tick) inputs. Borrow it freely between steps to
    /// assert on fields the hash alone cannot localise.
    pub fn state(&self) -> &S {
        &self.state
    }

    /// Number of ticks applied so far — also the length of
    /// [`trace`](Self::trace) and the index the next step will produce.
    pub fn tick(&self) -> usize {
        self.trace.len()
    }

    /// The world hash of every tick so far, in order: `trace()[t]` is the
    /// hash of the state after input `t` was applied. Identical to what
    /// [`record_trace`](crate::replay::record_trace) returns for the same run.
    pub fn trace(&self) -> &[u64] {
        &self.trace
    }

    /// The world hash of the state as it stands now. After at least one
    /// step this equals the last entry of [`trace`](Self::trace); before
    /// the first step it is the hash of the initial state (which is *not*
    /// a trace entry — traces record post-step hashes only).
    pub fn current_hash(&self) -> u64 {
        hash_state(&self.state)
    }

    /// Advance exactly one tick by applying `input`, record the resulting
    /// world hash, and return it. This is the core of the emulator: the
    /// caller decides when (and whether) each tick happens, and can
    /// inspect [`state`](Self::state) before and after every one.
    pub fn step(&mut self, input: &I) -> u64 {
        (self.step_fn)(&mut self.state, input);
        let hash = hash_state(&self.state);
        self.trace.push(hash);
        hash
    }

    /// Advance one tick and check the resulting hash against `expected`
    /// (the hash a reference run recorded for this tick).
    ///
    /// The step is applied either way: on mismatch the emulator is left
    /// standing on the diverging tick — [`state`](Self::state) is the
    /// diverged state and [`current_hash`](Self::current_hash) equals the
    /// returned [`Divergence::actual`] — so the test can inspect, log or
    /// snapshot it before deciding whether to keep stepping. On success
    /// the tick's hash is returned.
    pub fn step_expect(&mut self, input: &I, expected: u64) -> Result<u64, Divergence> {
        let tick = self.tick();
        let actual = self.step(input);
        if actual == expected {
            Ok(actual)
        } else {
            Err(Divergence {
                tick,
                expected,
                actual,
            })
        }
    }

    /// Apply every input in `inputs`, one tick each, continuing from the
    /// current tick (earlier steps are *not* restarted). Returns the full
    /// accumulated [`trace`](Self::trace), including any ticks stepped
    /// before this call.
    pub fn run(&mut self, inputs: &[I]) -> &[u64] {
        for input in inputs {
            self.step(input);
        }
        &self.trace
    }

    /// Step through `inputs`, comparing each tick's hash against
    /// `expected` as it is produced, and stop at the first divergence.
    ///
    /// This is [`check_trace`](crate::replay::check_trace) with the machine left in the caller's
    /// hands: the verdict is the same (including the length rule — an
    /// `expected` trace longer than the run diverges at the first missing
    /// tick, with `actual` 0), but `check_trace` always consumes the whole
    /// input list before answering, while `verify` returns the moment a
    /// tick disagrees, with the emulator still on that tick. Ticks are
    /// indexed absolutely: if the emulator has already stepped `k` ticks,
    /// the next input is compared against `expected[k]`.
    pub fn verify(&mut self, inputs: &[I], expected: &[u64]) -> Result<(), Divergence> {
        for input in inputs {
            let tick = self.tick();
            let actual = self.step(input);
            let want = expected.get(tick).copied().unwrap_or(0);
            if actual != want {
                return Err(Divergence {
                    tick,
                    expected: want,
                    actual,
                });
            }
        }
        if expected.len() > self.tick() {
            // Same length rule as check_trace / first_divergence: a longer
            // expected trace diverges at the first tick the run never made.
            let tick = self.tick();
            return Err(Divergence {
                tick,
                expected: expected.get(tick).copied().unwrap_or(0),
                actual: 0,
            });
        }
        Ok(())
    }

    /// Discard the current run and start over on a fresh `state`: the
    /// trace is cleared and [`tick`](Self::tick) returns to 0. Lets one
    /// emulator (and its step closure) serve many scenarios in a test.
    pub fn reset(&mut self, state: S) {
        self.state = state;
        self.trace.clear();
    }

    /// Consume the emulator and return the state as it stands — the
    /// end-of-test counterpart to [`resimulate`](crate::replay::resimulate)
    /// for a run that was driven by hand.
    pub fn into_state(self) -> S {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixed::Fixed;
    use crate::replay::{check_trace, record_trace};
    use crate::rng::SplitMix64;
    use crate::world_hash::Fnv1a;

    /// The same tiny simulation the replay tests use: a fixed-point
    /// position driven by a seeded RNG, built only from kit value types.
    /// (`SplitMix64` has no `PartialEq`, so states are compared by hash —
    /// the same convention the replay tests use.)
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

    fn step(s: &mut Sim, input: &i32) {
        let jitter = s.rng.below(3) as i32;
        s.pos = s.pos + Fixed::from_int(*input + jitter);
    }

    fn inputs() -> Vec<i32> {
        vec![1, -2, 3, 0, 5, -4, 2, 2]
    }

    fn emulator() -> SimEmulator<Sim, i32, fn(&mut Sim, &i32)> {
        SimEmulator::new(new_sim(), step)
    }

    // --- the oracle: hand-stepping must equal the batch trace, tick for tick

    #[test]
    fn test_step_returns_record_trace_hashes_one_at_a_time() {
        let expected = record_trace(&mut new_sim(), &inputs(), step);
        let mut emu = emulator();
        for (tick, input) in inputs().iter().enumerate() {
            let hash = emu.step(input);
            assert_eq!(
                hash, expected[tick],
                "tick {tick} must match the batch trace"
            );
            assert_eq!(emu.tick(), tick + 1);
        }
        assert_eq!(emu.trace(), expected.as_slice());
    }

    #[test]
    fn test_state_is_inspectable_between_ticks() {
        // Drive two ticks by hand, compare against a manually stepped
        // state, then drive the rest: pausing to look must not change the
        // run.
        let mut emu = emulator();
        emu.step(&inputs()[0]);
        emu.step(&inputs()[1]);
        let mut manual = new_sim();
        step(&mut manual, &inputs()[0]);
        step(&mut manual, &inputs()[1]);
        assert_eq!(hash_state(emu.state()), hash_state(&manual));
        assert_eq!(emu.state().pos, manual.pos);
        assert_eq!(emu.current_hash(), hash_state(&manual));

        let mut full = new_sim();
        let expected = record_trace(&mut full, &inputs(), step);
        emu.run(&inputs()[2..]);
        assert_eq!(emu.trace(), expected.as_slice());
        assert_eq!(hash_state(emu.state()), hash_state(&full));
    }

    #[test]
    fn test_interleaved_emulators_agree() {
        // Two emulators stepped in alternating order (one runs ahead, the
        // other catches up) must produce identical traces: *when* a tick
        // is requested cannot affect what it computes.
        let mut a = emulator();
        let mut b = emulator();
        let ins = inputs();
        a.step(&ins[0]);
        a.step(&ins[1]);
        a.step(&ins[2]);
        b.step(&ins[0]);
        a.step(&ins[3]);
        b.step(&ins[1]);
        b.step(&ins[2]);
        b.step(&ins[3]);
        assert_eq!(a.trace(), b.trace());
        assert_eq!(hash_state(a.state()), hash_state(b.state()));
    }

    // --- step_expect ---

    #[test]
    fn test_step_expect_accepts_the_recorded_hash() {
        let expected = record_trace(&mut new_sim(), &inputs(), step);
        let mut emu = emulator();
        for (input, &want) in inputs().iter().zip(&expected) {
            assert_eq!(emu.step_expect(input, want), Ok(want));
        }
        assert_eq!(emu.tick(), inputs().len());
    }

    #[test]
    fn test_step_expect_reports_the_tick_and_keeps_the_state_there() {
        let mut expected = record_trace(&mut new_sim(), &inputs(), step);
        expected[2] ^= 0xFFFF;
        let mut emu = emulator();
        assert!(emu.step_expect(&inputs()[0], expected[0]).is_ok());
        assert!(emu.step_expect(&inputs()[1], expected[1]).is_ok());
        let d = emu.step_expect(&inputs()[2], expected[2]).unwrap_err();
        assert_eq!(d.tick, 2);
        assert_eq!(d.expected, expected[2]);
        assert_eq!(
            d.actual,
            emu.current_hash(),
            "the diverged state is still in hand"
        );
        assert_eq!(
            emu.tick(),
            3,
            "the diverging step was applied, not rolled back"
        );
        // And the run can continue from the diverging tick.
        let rest = emu.run(&inputs()[3..]).to_vec();
        let reference = record_trace(&mut new_sim(), &inputs(), step);
        assert_eq!(rest, reference);
    }

    // --- verify: same verdict as check_trace, but it stops at the failure ---

    #[test]
    fn test_verify_matches_check_trace_verdicts() {
        let faithful = record_trace(&mut new_sim(), &inputs(), step);
        assert_eq!(emulator().verify(&inputs(), &faithful), Ok(()));
        assert_eq!(
            check_trace(&mut new_sim(), &inputs(), &faithful, step),
            Ok(())
        );

        let mut corrupt = faithful.clone();
        corrupt[5] ^= 0xABCD;
        let via_emulator = emulator().verify(&inputs(), &corrupt);
        let via_batch = check_trace(&mut new_sim(), &inputs(), &corrupt, step);
        assert_eq!(
            via_emulator, via_batch,
            "same divergence, whichever driver ran it"
        );
    }

    #[test]
    fn test_verify_stops_at_the_first_divergence() {
        let mut expected = record_trace(&mut new_sim(), &inputs(), step);
        expected[1] ^= 0xFFFF;
        let mut emu = emulator();
        let d = emu.verify(&inputs(), &expected).unwrap_err();
        assert_eq!(d.tick, 1);
        // check_trace would have consumed all 8 inputs; the emulator
        // stopped on tick 1, so only 2 ticks exist and the state is the
        // diverged one — not the end-of-run state.
        assert_eq!(emu.tick(), 2);
        assert_eq!(emu.trace().len(), 2);
        let mut full = new_sim();
        record_trace(&mut full, &inputs(), step);
        assert_ne!(hash_state(emu.state()), hash_state(&full));
        assert_eq!(emu.current_hash(), d.actual);
    }

    #[test]
    fn test_verify_flags_expected_longer_than_the_run() {
        let mut expected = record_trace(&mut new_sim(), &inputs(), step);
        expected.push(0xDEAD);
        let mut emu = emulator();
        let d = emu.verify(&inputs(), &expected).unwrap_err();
        assert_eq!(d.tick, inputs().len());
        assert_eq!(d.expected, 0xDEAD);
        assert_eq!(d.actual, 0);
        assert_eq!(
            emu.tick(),
            inputs().len(),
            "all real ticks were consumed first"
        );
    }

    #[test]
    fn test_verify_continues_from_the_current_tick() {
        // Absolute tick indexing: pre-step two ticks by hand, then verify
        // the remainder against the full reference trace.
        let expected = record_trace(&mut new_sim(), &inputs(), step);
        let mut emu = emulator();
        emu.step(&inputs()[0]);
        emu.step(&inputs()[1]);
        assert_eq!(emu.verify(&inputs()[2..], &expected), Ok(()));
        assert_eq!(emu.trace(), expected.as_slice());
    }

    // --- boundaries ---

    #[test]
    fn test_empty_run_is_vacuous() {
        let mut emu = emulator();
        assert_eq!(emu.tick(), 0);
        assert!(emu.trace().is_empty());
        assert_eq!(emu.current_hash(), hash_state(&new_sim()));
        assert!(emu.run(&[]).is_empty());
        assert_eq!(emu.verify(&[], &[]), Ok(()));
        // An expectation with no run at all diverges at tick 0.
        let d = emu.verify(&[], &[0x1234]).unwrap_err();
        assert_eq!(d.tick, 0);
        assert_eq!(d.actual, 0);
    }

    #[test]
    fn test_reset_restarts_from_a_fresh_state() {
        let mut emu = emulator();
        emu.run(&inputs()[..3]);
        assert_eq!(emu.tick(), 3);
        emu.reset(new_sim());
        assert_eq!(emu.tick(), 0);
        assert!(emu.trace().is_empty());
        assert_eq!(hash_state(emu.state()), hash_state(&new_sim()));
        let expected = record_trace(&mut new_sim(), &inputs(), step);
        assert_eq!(emu.run(&inputs()), expected.as_slice());
    }

    #[test]
    fn test_into_state_returns_the_advanced_state() {
        let mut emu = emulator();
        emu.run(&inputs());
        let end = emu.into_state();
        let mut reference = new_sim();
        record_trace(&mut reference, &inputs(), step);
        assert_eq!(hash_state(&end), hash_state(&reference));
        assert_eq!(end.pos, reference.pos);
    }

    #[test]
    fn test_drives_a_simulation_trait_impl_via_the_step_adapter() {
        // The intended pairing: a `sim::Simulation` is driven by the
        // one-line closure adapter, and the emulator's trace equals the
        // batch adapter's.
        use crate::sim::{trace as sim_trace, Simulation};

        #[derive(Clone, PartialEq, Debug)]
        struct Counter {
            value: i64,
        }
        impl Simulation for Counter {
            type Input = i64;
            fn step(&mut self, input: &i64) {
                self.value += *input;
            }
        }
        impl DetHash for Counter {
            fn det_hash(&self, h: &mut Fnv1a) {
                h.write_i64(self.value);
            }
        }

        let ins = [3, -1, 10, 0, 7];
        let mut emu = SimEmulator::new(Counter { value: 0 }, |s: &mut Counter, i: &i64| {
            s.step(i);
        });
        emu.run(&ins);
        let mut via_batch = Counter { value: 0 };
        let expected = sim_trace(&mut via_batch, &ins);
        assert_eq!(emu.trace(), expected.as_slice());
        assert_eq!(emu.state().value, 19);
    }
}
