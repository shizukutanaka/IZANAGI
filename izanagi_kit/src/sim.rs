//! One canonical simulation interface — and a single call that audits it.
//!
//! ## Why this exists
//!
//! Schneider's state-machine approach (*Implementing Fault-Tolerant Services
//! Using the State Machine Approach: A Tutorial*, ACM Computing Surveys 22(4),
//! 1990) is the formal statement of what this whole kit rests on: a
//! **deterministic state machine** plus an **ordered log of inputs** fully
//! determines the execution. Every verification tool here — trace recording,
//! rollback resimulation, sync testing, input planning — is a different way of
//! exercising that one abstraction.
//!
//! But they grew up separately, and each spelled the step function
//! differently: [`replay::record_trace`](crate::replay::record_trace),
//! [`replay::resimulate`](crate::replay::resimulate) and
//! [`rollback::sync_test`](crate::rollback::sync_test) take
//! `FnMut(&mut S, &I)`; [`plan::plan_inputs`](crate::plan::plan_inputs) takes a
//! pure `Fn(&S, &I) -> S`; [`dst::dst_sweep`](crate::dst::dst_sweep) takes a
//! tick-driven `FnMut(&mut S, usize)`. A user with one simulation had to write
//! it three ways to get all the checks — friction with no conceptual
//! justification, since the underlying object is the same state machine each
//! time.
//!
//! [`Simulation`] is that one abstraction, and the adapters below feed it to
//! every tool. Nothing here changes the existing functions: they remain the
//! low-level API for closures, and these are thin wrappers over them.
//!
//! ## The audit
//!
//! [`audit`] answers "is my simulation actually deterministic?" in one call by
//! running three *independent* checks that fail on different bug classes:
//!
//! 1. **Double run** — simulate the same inputs twice from the same start and
//!    compare hash traces. Catches nondeterminism that is stable within a run
//!    but varies across runs: hash-map iteration order, pointer-address
//!    dependence, uninitialised reads.
//! 2. **Sync test** — roll back and resimulate every frame
//!    ([`rollback::sync_test`](crate::rollback::sync_test)). Catches
//!    order-of-operations bugs that only appear on *partial* re-execution from
//!    a mid-stream snapshot — the path a rollback netcode session actually
//!    takes, which a straight double run never exercises.
//! 3. **Final hash** — reported so callers can pin it as a regression value.
//!
//! This is the same layering as the DST discipline in
//! [`dst`](crate::dst) (FoundationDB, TigerBeetle): cheap checks that run
//! constantly, each aimed at a failure mode the others cannot see.
//!
//! ```
//! use izanagi_kit::sim::{audit, Simulation};
//! use izanagi_kit::world_hash::{DetHash, Fnv1a};
//!
//! #[derive(Clone)]
//! struct Counter { value: i64 }
//!
//! impl Simulation for Counter {
//!     type Input = i64;
//!     fn step(&mut self, input: &i64) { self.value += *input; }
//! }
//! impl DetHash for Counter {
//!     fn det_hash(&self, h: &mut Fnv1a) { h.write_i64(self.value); }
//! }
//!
//! let report = audit(&Counter { value: 0 }, &[1, 2, 3, 4], 2);
//! assert!(report.is_deterministic());
//! assert_eq!(report.ticks, 4);
//! ```

use crate::replay::{first_divergence, record_trace, Divergence};
use crate::rollback::{sync_test as rollback_sync_test, SyncTestFailure};
use crate::world_hash::{hash_state, DetHash};

/// A deterministic state machine: state that advances by applying one input.
///
/// This is the interface every verification tool in the kit consumes (see the
/// module docs). Implementing it means `step` must be a *pure function of
/// `(self, input)`* — no wall-clock, no thread ids, no floats in the
/// simulation path, no iteration over unordered containers. Those are exactly
/// the violations [`audit`] is built to catch.
pub trait Simulation {
    /// The per-step input this simulation consumes.
    type Input;

    /// Advance the state by one step, applying `input`.
    fn step(&mut self, input: &Self::Input);
}

/// Record the state hash after each input — [`replay::record_trace`] for a
/// [`Simulation`]. Advances `state` in place and returns one hash per input.
///
/// [`replay::record_trace`]: crate::replay::record_trace
pub fn trace<S>(state: &mut S, inputs: &[S::Input]) -> Vec<u64>
where
    S: Simulation + DetHash,
{
    record_trace(state, inputs, |s: &mut S, i: &S::Input| s.step(i))
}

/// Replay `inputs` onto a clone of `snapshot`, leaving `snapshot` untouched —
/// [`replay::resimulate`] for a [`Simulation`]. This is the core rollback
/// operation: keep a confirmed-good snapshot, then re-run newer inputs.
///
/// [`replay::resimulate`]: crate::replay::resimulate
pub fn resimulate<S>(snapshot: &S, inputs: &[S::Input]) -> S
where
    S: Simulation + Clone,
{
    let mut state = snapshot.clone();
    for input in inputs {
        state.step(input);
    }
    state
}

/// Roll back `check_distance` frames and resimulate at every step, checking the
/// result still matches — [`rollback::sync_test`] for a [`Simulation`].
///
/// [`rollback::sync_test`]: crate::rollback::sync_test
pub fn sync_test<S>(
    initial: S,
    inputs: &[S::Input],
    check_distance: usize,
) -> Result<(), SyncTestFailure>
where
    S: Simulation + DetHash + Clone,
{
    rollback_sync_test(
        initial,
        inputs,
        check_distance,
        |s: &mut S, i: &S::Input| s.step(i),
    )
}

/// Search for the shortest input sequence reaching `goal` —
/// [`plan::plan_inputs`] for a [`Simulation`]. The pure `Fn(&S, &I) -> S` shape
/// that planner needs is derived here from `Clone` + [`Simulation::step`], so
/// implementors do not write the simulation a second way.
///
/// [`plan::plan_inputs`]: crate::plan::plan_inputs
pub fn plan<S, G>(
    start: S,
    inputs: &[S::Input],
    goal: G,
    max_states: usize,
) -> Option<Vec<S::Input>>
where
    S: Simulation + DetHash + Clone,
    S::Input: Clone,
    G: Fn(&S) -> bool,
{
    crate::plan::plan_inputs(
        start,
        inputs,
        |s: &S, i: &S::Input| {
            let mut next = s.clone();
            next.step(i);
            next
        },
        goal,
        max_states,
    )
}

/// What [`audit`] found. [`is_deterministic`](Self::is_deterministic) is the
/// one-line verdict; the individual fields say *how* it failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditReport {
    /// Number of inputs applied (steps simulated).
    pub ticks: usize,
    /// State hash after the last input, from the first of the two runs. Pin
    /// this as a regression value once the audit passes.
    pub final_hash: u64,
    /// Set when two identical runs produced different hash traces — the
    /// simulation is nondeterministic *across runs*.
    pub double_run: Option<Divergence>,
    /// Set when rolling back and resimulating produced a different state than
    /// running straight through — the simulation is nondeterministic under
    /// *partial re-execution*, which breaks rollback netcode.
    pub sync_test: Option<SyncTestFailure>,
}

impl AuditReport {
    /// `true` when every check passed: the simulation is deterministic across
    /// repeated runs *and* under rollback resimulation.
    pub fn is_deterministic(&self) -> bool {
        self.double_run.is_none() && self.sync_test.is_none()
    }
}

impl core::fmt::Display for AuditReport {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.is_deterministic() {
            return write!(
                f,
                "deterministic over {} tick(s); final hash {:#018x}",
                self.ticks, self.final_hash
            );
        }
        write!(f, "NONDETERMINISTIC over {} tick(s):", self.ticks)?;
        if let Some(d) = &self.double_run {
            write!(f, " double-run: {d};")?;
        }
        if let Some(s) = &self.sync_test {
            write!(f, " {s}")?;
        }
        Ok(())
    }
}

/// Run every determinism check against a [`Simulation`] in one call — see the
/// module docs for what each check catches and why one is not enough.
///
/// `check_distance` is the rollback depth for the sync test (`0` skips it).
/// `initial` is cloned for each run and left untouched. With no inputs the
/// report is vacuously clean and `final_hash` is the hash of `initial`.
pub fn audit<S>(initial: &S, inputs: &[S::Input], check_distance: usize) -> AuditReport
where
    S: Simulation + DetHash + Clone,
{
    // 1. Two independent runs of the same inputs from the same start.
    let mut a = initial.clone();
    let trace_a = trace(&mut a, inputs);
    let mut b = initial.clone();
    let trace_b = trace(&mut b, inputs);
    let double_run = first_divergence(&trace_a, &trace_b).err();

    // 2. Rollback-and-resimulate at every frame.
    let sync = sync_test(initial.clone(), inputs, check_distance).err();

    AuditReport {
        ticks: inputs.len(),
        final_hash: *trace_a.last().unwrap_or(&hash_state(initial)),
        double_run,
        sync_test: sync,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixed::Fixed;
    use crate::rng::SplitMix64;
    use crate::world_hash::Fnv1a;

    #[derive(Clone)]
    struct Counter {
        value: i64,
        rng: SplitMix64,
    }

    impl Simulation for Counter {
        type Input = i64;
        fn step(&mut self, input: &i64) {
            let jitter = self.rng.below(4) as i64;
            self.value += *input + jitter;
        }
    }

    impl DetHash for Counter {
        fn det_hash(&self, h: &mut Fnv1a) {
            h.write_i64(self.value);
            self.rng.det_hash(h);
        }
    }

    fn counter() -> Counter {
        Counter {
            value: 0,
            rng: SplitMix64::new(0xC0FFEE),
        }
    }

    fn inputs() -> Vec<i64> {
        (1..=12).collect()
    }

    // --- adapters agree with the underlying low-level API ---

    #[test]
    fn test_trace_matches_record_trace_directly() {
        let mut via_trait = counter();
        let got = trace(&mut via_trait, &inputs());

        let mut direct = counter();
        let expect = record_trace(&mut direct, &inputs(), |s: &mut Counter, i: &i64| {
            let jitter = s.rng.below(4) as i64;
            s.value += *i + jitter;
        });
        assert_eq!(got, expect, "adapter must not change the hash sequence");
        assert_eq!(via_trait.value, direct.value);
    }

    #[test]
    fn test_resimulate_matches_running_forward() {
        let base = counter();
        let replayed = resimulate(&base, &inputs());
        let mut forward = base.clone();
        let _ = trace(&mut forward, &inputs());
        assert_eq!(hash_state(&replayed), hash_state(&forward));
        assert_eq!(base.value, 0, "snapshot must be left untouched");
    }

    #[test]
    fn test_sync_test_adapter_passes_for_deterministic_sim() {
        assert_eq!(sync_test(counter(), &inputs(), 4), Ok(()));
    }

    #[test]
    fn test_plan_adapter_finds_a_sequence() {
        // A tiny deterministic sim with no RNG so the planner's state space is
        // small and the goal is exactly reachable.
        #[derive(Clone, PartialEq)]
        struct Pos {
            x: i32,
        }
        impl Simulation for Pos {
            type Input = i32;
            fn step(&mut self, input: &i32) {
                self.x += *input;
            }
        }
        impl DetHash for Pos {
            fn det_hash(&self, h: &mut Fnv1a) {
                h.write_i32(self.x);
            }
        }
        let found = plan(Pos { x: 0 }, &[1, 2], |p: &Pos| p.x == 5, 10_000);
        let seq = found.expect("5 is reachable from 0 with steps of 1 and 2");
        // Verify by replaying the plan.
        let end = resimulate(&Pos { x: 0 }, &seq);
        assert_eq!(end.x, 5);
        // BFS finds a shortest sequence: 5 = 2+2+1, three steps.
        assert_eq!(seq.len(), 3, "must be a shortest solution");
    }

    // --- audit ---

    #[test]
    fn test_audit_passes_for_deterministic_sim() {
        let report = audit(&counter(), &inputs(), 5);
        assert!(report.is_deterministic(), "{report}");
        assert_eq!(report.ticks, 12);
        assert!(report.double_run.is_none() && report.sync_test.is_none());
        assert!(report.to_string().contains("deterministic over 12"));
    }

    #[test]
    fn test_audit_final_hash_matches_manual_replay() {
        let report = audit(&counter(), &inputs(), 0);
        let end = resimulate(&counter(), &inputs());
        assert_eq!(report.final_hash, hash_state(&end));
    }

    #[test]
    fn test_audit_empty_inputs_is_vacuously_clean() {
        let report = audit(&counter(), &[], 3);
        assert!(report.is_deterministic());
        assert_eq!(report.ticks, 0);
        assert_eq!(
            report.final_hash,
            hash_state(&counter()),
            "no inputs → hash of the initial state"
        );
    }

    /// A simulation whose result depends on a counter shared across runs — the
    /// classic "global mutable state" nondeterminism. Each `step` reads and
    /// bumps a process-wide value, so a second run of the same inputs produces
    /// different states.
    #[derive(Clone)]
    struct GlobalDependent {
        value: i64,
    }

    // A thread-local standing in for a hidden global. Not simulation state, so
    // it is not cloned with the struct — exactly the bug shape audit must find.
    thread_local! {
        static HIDDEN: core::cell::Cell<i64> = const { core::cell::Cell::new(0) };
    }

    impl Simulation for GlobalDependent {
        type Input = i64;
        fn step(&mut self, input: &i64) {
            let extra = HIDDEN.with(|h| {
                let v = h.get() + 1;
                h.set(v);
                v
            });
            self.value += *input + extra;
        }
    }

    impl DetHash for GlobalDependent {
        fn det_hash(&self, h: &mut Fnv1a) {
            h.write_i64(self.value);
        }
    }

    #[test]
    fn test_audit_catches_cross_run_nondeterminism() {
        HIDDEN.with(|h| h.set(0));
        let report = audit(&GlobalDependent { value: 0 }, &inputs(), 0);
        assert!(!report.is_deterministic(), "hidden global must be caught");
        let d = report.double_run.expect("double run must diverge");
        assert_eq!(d.tick, 0, "the very first step already differs");
        assert!(report.to_string().contains("NONDETERMINISTIC"));
    }

    #[test]
    fn test_audit_catches_rollback_only_nondeterminism() {
        // Same hidden-global bug, but now audited with a rollback distance: the
        // sync test must also flag it (it re-executes from mid-stream
        // snapshots, so the hidden counter advances a different number of
        // times).
        HIDDEN.with(|h| h.set(0));
        let report = audit(&GlobalDependent { value: 0 }, &inputs(), 3);
        assert!(!report.is_deterministic());
        assert!(
            report.sync_test.is_some(),
            "partial re-execution must diverge too"
        );
    }

    #[test]
    fn test_audit_is_deterministic_itself() {
        let a = audit(&counter(), &inputs(), 4);
        let b = audit(&counter(), &inputs(), 4);
        assert_eq!(a, b, "auditing twice must give the identical report");
    }

    #[test]
    fn test_audit_report_display_is_informative() {
        let clean = AuditReport {
            ticks: 7,
            final_hash: 0xABCD,
            double_run: None,
            sync_test: None,
        };
        let text = clean.to_string();
        assert!(text.contains("7 tick"), "{text}");
        assert!(text.contains("0x000000000000abcd"), "{text}");
    }

    #[test]
    fn test_simulation_with_fixed_point_state() {
        // The intended production shape: fixed-point state, no float anywhere.
        #[derive(Clone)]
        struct Body {
            pos: Fixed,
            vel: Fixed,
        }
        impl Simulation for Body {
            type Input = Fixed;
            fn step(&mut self, accel: &Fixed) {
                self.vel = self.vel + *accel;
                self.pos = self.pos + self.vel;
            }
        }
        impl DetHash for Body {
            fn det_hash(&self, h: &mut Fnv1a) {
                self.pos.det_hash(h);
                self.vel.det_hash(h);
            }
        }
        let accels: Vec<Fixed> = (1..=8).map(Fixed::from_int).collect();
        let report = audit(
            &Body {
                pos: Fixed::ZERO,
                vel: Fixed::ZERO,
            },
            &accels,
            3,
        );
        assert!(report.is_deterministic(), "{report}");
    }
}
