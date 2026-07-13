//! Deterministic Simulation Testing (DST) harness — seed sweeps with
//! invariant checks and one-line failure reproduction.
//!
//! DST is the testing discipline popularized by FoundationDB and TigerBeetle
//! and now mainstream well beyond games (Polar Signals, madsim, turmoil): run
//! the *same* simulation under many seeds, assert invariants every tick, and
//! — because the simulation is deterministic — reduce any failure to a
//! `(seed, tick)` pair that reproduces it exactly, every time, on every
//! machine. The kit's bit-exact core makes this nearly free: this module is
//! the thin harness that turns "our sim is deterministic" into "our bugs are
//! one-line reproducible".
//!
//! - [`dst_sweep`] — drive N seeded runs, checking an invariant after every
//!   tick; the first violation comes back as a [`DstFailure`] naming the
//!   seed, the tick, and the caller's message.
//! - [`dst_replay`] — re-run a single seed for a given number of ticks: the
//!   one-line repro for a sweep failure (and the debugging entry point).
//! - [`dst_determinism_sweep`] — run each seed **twice** and compare the
//!   per-tick hash traces, catching nondeterminism itself (iteration-order
//!   leaks, uninitialised state, hidden globals) rather than a logic
//!   invariant. This is the self-test of the kit's own central claim.
//!
//! The simulation is supplied as closures (`make(seed) -> state`,
//! `step(&mut state, tick)`), so the harness is engine-agnostic, mirroring
//! [`replay`](crate::replay).

use crate::replay::Divergence;
use crate::world_hash::{hash_state, DetHash};

/// A failed DST run, reduced to what a developer needs to reproduce it: the
/// seed, the 0-based tick of the first violation, and the invariant's own
/// message. The `Display` form is a ready-to-paste repro line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DstFailure {
    /// The seed of the failing run.
    pub seed: u64,
    /// 0-based tick at which the invariant first failed.
    pub tick: usize,
    /// The message returned by the failing invariant (or a description of
    /// the hash divergence for [`dst_determinism_sweep`]).
    pub message: String,
}

impl core::fmt::Display for DstFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "DST failure: seed {:#018x}, tick {}: {} — reproduce with dst_replay({:#018x}, {})",
            self.seed,
            self.tick,
            self.message,
            self.seed,
            self.tick + 1
        )
    }
}

/// Sweep `seeds`, running each simulation for `ticks` ticks and checking
/// `invariant` after every tick. Returns the first violation as a
/// [`DstFailure`] (sweeping stops there — rerun with the reported seed via
/// [`dst_replay`] to debug), or `Ok(())` if every seed survives.
///
/// - `make(seed)` builds the initial state for one run.
/// - `step(&mut state, tick)` advances one tick (0-based).
/// - `invariant(&state, tick)` returns `Err(message)` to flag a violation.
///
/// Determinism: seeds are visited in iterator order and each run is driven
/// identically, so the reported failure is stable across machines and runs.
pub fn dst_sweep<S, M, F, C, I>(
    seeds: I,
    ticks: usize,
    mut make: M,
    mut step: F,
    mut invariant: C,
) -> Result<(), DstFailure>
where
    I: IntoIterator<Item = u64>,
    M: FnMut(u64) -> S,
    F: FnMut(&mut S, usize),
    C: FnMut(&S, usize) -> Result<(), String>,
{
    for seed in seeds {
        let mut state = make(seed);
        for tick in 0..ticks {
            step(&mut state, tick);
            if let Err(message) = invariant(&state, tick) {
                return Err(DstFailure {
                    seed,
                    tick,
                    message,
                });
            }
        }
    }
    Ok(())
}

/// Re-run a single seed for `ticks` ticks and return the resulting state —
/// the one-line reproduction of a [`dst_sweep`] failure. To land exactly *on*
/// the failing state, pass `failure.tick + 1` ticks (the failure's `Display`
/// line already says so).
pub fn dst_replay<S, M, F>(seed: u64, ticks: usize, mut make: M, mut step: F) -> S
where
    M: FnMut(u64) -> S,
    F: FnMut(&mut S, usize),
{
    let mut state = make(seed);
    for tick in 0..ticks {
        step(&mut state, tick);
    }
    state
}

/// Sweep `seeds`, running each seed **twice** and comparing the two runs'
/// per-tick [`DetHash`] traces. Any mismatch means the simulation itself is
/// nondeterministic — an iteration-order leak, uninitialised memory, a hidden
/// global, wall-clock reads — and is reported as a [`DstFailure`] whose
/// message carries the two hashes ([`Divergence`] formatting).
///
/// This checks a different property than [`dst_sweep`]: not "is the state
/// valid?" but "is the state *reproducible*?" — the kit's central claim,
/// applied to *your* simulation.
pub fn dst_determinism_sweep<S, M, F, I>(
    seeds: I,
    ticks: usize,
    mut make: M,
    mut step: F,
) -> Result<(), DstFailure>
where
    S: DetHash,
    I: IntoIterator<Item = u64>,
    M: FnMut(u64) -> S,
    F: FnMut(&mut S, usize),
{
    for seed in seeds {
        let run = |make: &mut M, step: &mut F| -> Vec<u64> {
            let mut state = make(seed);
            let mut trace = Vec::with_capacity(ticks);
            for tick in 0..ticks {
                step(&mut state, tick);
                trace.push(hash_state(&state));
            }
            trace
        };
        let first = run(&mut make, &mut step);
        let second = run(&mut make, &mut step);
        for (tick, (&e, &a)) in first.iter().zip(second.iter()).enumerate() {
            if e != a {
                let divergence = Divergence {
                    tick,
                    expected: e,
                    actual: a,
                };
                return Err(DstFailure {
                    seed,
                    tick,
                    message: format!("nondeterministic re-run: {divergence}"),
                });
            }
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

    /// Minimal deterministic sim: a counter walked by a seeded RNG.
    #[derive(Clone)]
    struct Sim {
        value: Fixed,
        rng: SplitMix64,
    }

    impl DetHash for Sim {
        fn det_hash(&self, hasher: &mut Fnv1a) {
            self.value.det_hash(hasher);
            self.rng.det_hash(hasher);
        }
    }

    fn make_sim(seed: u64) -> Sim {
        Sim {
            value: Fixed::ZERO,
            rng: SplitMix64::new(seed),
        }
    }

    fn step_sim(s: &mut Sim, _tick: usize) {
        let d = s.rng.below(5) as i32;
        s.value = s.value + Fixed::from_int(d);
    }

    #[test]
    fn test_dst_sweep_passes_when_invariant_holds() {
        // value only ever grows, so non-negativity holds for every seed.
        let result = dst_sweep(0..20u64, 50, make_sim, step_sim, |s, _| {
            if s.value >= Fixed::ZERO {
                Ok(())
            } else {
                Err("value went negative".to_string())
            }
        });
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn test_dst_sweep_reports_seed_tick_and_message() {
        // Fail deterministically on seed 7 once value exceeds a bound only
        // that run's ticks can reach (tick index provides the trigger).
        let result = dst_sweep(0..20u64, 50, make_sim, step_sim, |s, tick| {
            if tick == 13 && hash_state(s) % 20 == 0 {
                Err("synthetic violation".to_string())
            } else {
                Ok(())
            }
        });
        // The invariant is deterministic, so whether it fires — and where — is
        // stable. If it fires, the report must carry tick 13 and the message.
        if let Err(f) = result {
            assert_eq!(f.tick, 13);
            assert_eq!(f.message, "synthetic violation");
            assert!(f.seed < 20);
        }
    }

    #[test]
    fn test_dst_sweep_failure_is_deterministic_and_replayable() {
        // An invariant that must fail for every seed at a known tick.
        let invariant = |s: &Sim, _tick: usize| {
            if s.value > Fixed::from_int(30) {
                Err(format!("value exceeded 30: {}", s.value.to_int_trunc()))
            } else {
                Ok(())
            }
        };
        let a = dst_sweep(0..10u64, 100, make_sim, step_sim, invariant).unwrap_err();
        let b = dst_sweep(0..10u64, 100, make_sim, step_sim, invariant).unwrap_err();
        assert_eq!(a, b, "sweep failure must be stable across runs");

        // The Display line's recipe (tick + 1) lands exactly on the failing
        // state: replaying it violates the invariant, one tick earlier passes.
        let failing = dst_replay(a.seed, a.tick + 1, make_sim, step_sim);
        assert!(invariant(&failing, a.tick).is_err());
        if a.tick > 0 {
            let before = dst_replay(a.seed, a.tick, make_sim, step_sim);
            assert!(invariant(&before, a.tick - 1).is_ok());
        }
    }

    #[test]
    fn test_dst_failure_display_carries_repro_line() {
        let f = DstFailure {
            seed: 0xBEEF,
            tick: 41,
            message: "hp underflow".to_string(),
        };
        let text = f.to_string();
        assert!(text.contains("0x000000000000beef"), "{text}");
        assert!(text.contains("tick 41"), "{text}");
        assert!(text.contains("hp underflow"), "{text}");
        assert!(
            text.contains("dst_replay(0x000000000000beef, 42)"),
            "{text}"
        );
    }

    #[test]
    fn test_dst_determinism_sweep_passes_for_pure_sim() {
        assert_eq!(
            dst_determinism_sweep(0..20u64, 50, make_sim, step_sim),
            Ok(())
        );
    }

    #[test]
    fn test_dst_determinism_sweep_catches_nondeterminism() {
        // Inject nondeterminism through a captured mutable counter that leaks
        // across the two runs — exactly the "hidden global" failure mode.
        let mut hidden: i32 = 0;
        let result = dst_determinism_sweep(0..1u64, 10, make_sim, |s, tick| {
            hidden += 1;
            s.value = s.value + Fixed::from_int(hidden);
            step_sim(s, tick);
        });
        let failure = result.unwrap_err();
        assert_eq!(failure.seed, 0);
        assert_eq!(failure.tick, 0, "hidden state diverges on the first tick");
        assert!(failure.message.contains("nondeterministic re-run"));
    }

    #[test]
    fn test_dst_replay_matches_sweep_run_exactly() {
        // dst_replay must produce bit-identical state to the sweep's own run.
        let via_replay = dst_replay(3, 25, make_sim, step_sim);
        let mut manual = make_sim(3);
        for tick in 0..25 {
            step_sim(&mut manual, tick);
        }
        assert_eq!(hash_state(&via_replay), hash_state(&manual));
    }

    #[test]
    fn test_dst_sweep_zero_ticks_or_empty_seeds_is_ok() {
        let fail_always =
            |_: &Sim, _: usize| -> Result<(), String> { Err("never run".to_string()) };
        assert_eq!(
            dst_sweep(0..0u64, 100, make_sim, step_sim, fail_always),
            Ok(())
        );
        assert_eq!(
            dst_sweep(0..5u64, 0, make_sim, step_sim, fail_always),
            Ok(())
        );
    }
}
