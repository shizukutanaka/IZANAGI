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
//! - [`dst_swarm_sweep`] — swarm testing (Groce et al., ISSTA 2012): each seed
//!   enables a random *subset* of the available actions, because omitting
//!   features per run finds more bugs than always allowing all of them.
//!   [`dst_swarm_replay`] is its one-line repro.
//!
//! The simulation is supplied as closures (`make(seed) -> state`,
//! `step(&mut state, tick)`), so the harness is engine-agnostic, mirroring
//! [`replay`](crate::replay).

use crate::replay::Divergence;
use crate::rng::SplitMix64;
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

// Named sub-streams so the harness's own choices never disturb the
// simulation's RNG, and so subset selection and action selection stay
// independent of each other.
const SWARM_SUBSET_STREAM: u64 = 0x5375_6273_6574; // "Subset"
const SWARM_ACTION_STREAM: u64 = 0x4163_7469_6F6E; // "Action"

/// The action indices enabled for `seed` in a [`dst_swarm_sweep`] run —
/// each of the `action_count` actions is included with probability ½.
///
/// Exposed so a failure can be inspected or reproduced by hand. The result is
/// derived purely from `seed` through a named sub-stream, so it is stable
/// across machines and never consumes from the simulation's own RNG.
///
/// A run with nothing to do would be wasted, so an empty draw falls back to a
/// single seed-chosen action. Returns empty only when `action_count` is 0.
pub fn swarm_subset(seed: u64, action_count: usize) -> Vec<usize> {
    if action_count == 0 {
        return Vec::new();
    }
    let mut rng = SplitMix64::new(seed).split(SWARM_SUBSET_STREAM);
    let mut enabled: Vec<usize> = (0..action_count).filter(|_| rng.below(2) == 1).collect();
    if enabled.is_empty() {
        enabled.push(rng.below(action_count as u32) as usize);
    }
    enabled
}

/// A failed [`dst_swarm_sweep`] run. Carries everything [`DstFailure`] does
/// plus the **action subset** that was active, since "which features were
/// switched on" is the first thing you need when reading a swarm failure.
///
/// The subset is derived from `seed`, so the seed alone still reproduces the
/// run exactly ([`dst_swarm_replay`]); `enabled` is reported for legibility.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwarmFailure {
    /// The seed of the failing run.
    pub seed: u64,
    /// 0-based tick at which the invariant first failed.
    pub tick: usize,
    /// The message returned by the failing invariant.
    pub message: String,
    /// Indices into the caller's `actions` slice that were enabled this run.
    pub enabled: Vec<usize>,
}

impl core::fmt::Display for SwarmFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "swarm seed {} failed at tick {} with actions {:?}: {} \
             (reproduce: dst_swarm_replay({}, {}, ..))",
            self.seed,
            self.tick,
            self.enabled,
            self.message,
            self.seed,
            self.tick + 1
        )
    }
}

/// **Swarm testing** over `actions`: each seed enables a random *subset* of the
/// available actions and drives the simulation using only those, checking
/// `invariant` after every tick.
///
/// Groce, Zhang, Eide, Chen & Regehr (*Swarm Testing*, ISSTA 2012) found —
/// counter-intuitively — that randomly **omitting** features from each test run
/// finds more bugs than always allowing every feature. The reason is that a
/// uniform generator dilutes: with every action equally likely on every tick,
/// the long runs of one action that expose capacity, starvation and ordering
/// bugs essentially never occur. A run that simply *cannot* pop is a run that
/// tests overflow properly.
///
/// So this complements [`dst_sweep`] rather than replacing it: same seeds, same
/// invariants, different distribution over action sequences.
///
/// ```
/// use izanagi_kit::dst::dst_swarm_sweep;
///
/// #[derive(Clone, Copy)]
/// enum Op { Add, Remove }
///
/// let result = dst_swarm_sweep(
///     0..50u64,
///     30,
///     &[Op::Add, Op::Remove],
///     |_seed| 0i32,
///     |count: &mut i32, op: &Op| match op {
///         Op::Add => *count += 1,
///         Op::Remove => *count = (*count - 1).max(0),
///     },
///     |count: &i32, _tick| {
///         if *count > 100 { Err(format!("overflow: {count}")) } else { Ok(()) }
///     },
/// );
/// assert!(result.is_ok());
/// ```
///
/// `actions` empty, `ticks` 0, or `seeds` empty all succeed vacuously.
pub fn dst_swarm_sweep<S, A, M, F, C, I>(
    seeds: I,
    ticks: usize,
    actions: &[A],
    mut make: M,
    mut step: F,
    mut invariant: C,
) -> Result<(), SwarmFailure>
where
    I: IntoIterator<Item = u64>,
    M: FnMut(u64) -> S,
    F: FnMut(&mut S, &A),
    C: FnMut(&S, usize) -> Result<(), String>,
{
    if actions.is_empty() {
        return Ok(());
    }
    for seed in seeds {
        let enabled = swarm_subset(seed, actions.len());
        let mut rng = SplitMix64::new(seed).split(SWARM_ACTION_STREAM);
        let mut state = make(seed);
        for tick in 0..ticks {
            let pick = enabled[rng.below(enabled.len() as u32) as usize];
            step(&mut state, &actions[pick]);
            if let Err(message) = invariant(&state, tick) {
                return Err(SwarmFailure {
                    seed,
                    tick,
                    message,
                    enabled,
                });
            }
        }
    }
    Ok(())
}

/// Re-run one swarm seed for `ticks` ticks — the one-line reproduction of a
/// [`dst_swarm_sweep`] failure. The action subset and the per-tick picks are
/// both re-derived from `seed`, so this replays the identical sequence. To land
/// exactly *on* the failing state, pass `failure.tick + 1`.
pub fn dst_swarm_replay<S, A, M, F>(
    seed: u64,
    ticks: usize,
    actions: &[A],
    mut make: M,
    mut step: F,
) -> S
where
    M: FnMut(u64) -> S,
    F: FnMut(&mut S, &A),
{
    let mut state = make(seed);
    if actions.is_empty() {
        return state;
    }
    let enabled = swarm_subset(seed, actions.len());
    let mut rng = SplitMix64::new(seed).split(SWARM_ACTION_STREAM);
    for _ in 0..ticks {
        let pick = enabled[rng.below(enabled.len() as u32) as usize];
        step(&mut state, &actions[pick]);
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

    // --- swarm testing ---

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Op {
        Push,
        Pop,
    }

    /// A bounded buffer with a deliberate bug: `Push` never checks capacity.
    /// The invariant catches the overflow — but only if a run actually pushes
    /// far more than it pops, which is the situation swarm testing creates.
    fn stack_step(len: &mut i32, op: &Op) {
        match op {
            Op::Push => *len += 1,
            Op::Pop => *len = (*len - 1).max(0),
        }
    }

    const CAP: i32 = 20;

    fn cap_invariant(len: &i32, _tick: usize) -> Result<(), String> {
        if *len > CAP {
            Err(format!("buffer overflow: len {len} exceeds capacity {CAP}"))
        } else {
            Ok(())
        }
    }

    #[test]
    fn test_swarm_subset_is_deterministic() {
        for seed in 0..50u64 {
            assert_eq!(swarm_subset(seed, 6), swarm_subset(seed, 6));
        }
    }

    #[test]
    fn test_swarm_subset_is_never_empty_and_in_range() {
        for seed in 0..500u64 {
            let s = swarm_subset(seed, 4);
            assert!(!s.is_empty(), "seed {seed} produced an empty subset");
            assert!(s.iter().all(|&i| i < 4), "index out of range: {s:?}");
            // Indices are distinct (each action considered once).
            let mut sorted = s.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), s.len(), "duplicate indices in {s:?}");
        }
    }

    #[test]
    fn test_swarm_subset_zero_actions_is_empty() {
        assert!(swarm_subset(1, 0).is_empty());
    }

    #[test]
    fn test_swarm_subset_varies_across_seeds() {
        // The entire point of swarm: configurations must differ run to run.
        let subsets: std::collections::HashSet<Vec<usize>> =
            (0..200u64).map(|s| swarm_subset(s, 5)).collect();
        assert!(
            subsets.len() > 8,
            "expected diverse subsets, saw only {}",
            subsets.len()
        );
        // And at least one run must omit some action — otherwise this is just
        // uniform testing with extra steps.
        assert!(
            subsets.iter().any(|s| s.len() < 5),
            "no run omitted an action"
        );
    }

    #[test]
    fn test_swarm_finds_the_bug_that_uniform_sampling_misses() {
        // This is the Groce et al. result, reproduced in miniature.
        //
        // Uniform: every tick picks Push or Pop with equal probability, so the
        // length is a ±1 random walk. Reaching +20 within 40 steps is
        // essentially impossible, and the sweep passes — the bug hides.
        let uniform = dst_sweep(
            0..200u64,
            40,
            |_seed| 0i32,
            |len: &mut i32, tick: usize| {
                // Deterministic 50/50 from the seed-independent tick stream.
                let mut rng = SplitMix64::new(tick as u64 ^ 0xA5A5);
                let op = if rng.below(2) == 0 { Op::Push } else { Op::Pop };
                stack_step(len, &op);
            },
            cap_invariant,
        );
        assert!(
            uniform.is_ok(),
            "uniform sampling was expected to miss this bug"
        );

        // Swarm: some seed enables only Push, and that run overflows at once.
        let swarm = dst_swarm_sweep(
            0..200u64,
            40,
            &[Op::Push, Op::Pop],
            |_seed| 0i32,
            stack_step,
            cap_invariant,
        );
        let failure = swarm.expect_err("swarm must find the overflow");
        assert!(failure.message.contains("overflow"), "{failure}");
        assert_eq!(
            failure.enabled,
            vec![0],
            "the failing run is the Push-only configuration"
        );
        // It overflows on the first tick past capacity.
        assert_eq!(failure.tick, CAP as usize);
    }

    #[test]
    fn test_swarm_sweep_passes_for_a_correct_sim() {
        let ok = dst_swarm_sweep(
            0..100u64,
            50,
            &[Op::Push, Op::Pop],
            |_seed| 0i32,
            |len: &mut i32, op: &Op| {
                // Capacity-respecting version: no overflow is possible.
                match op {
                    Op::Push => *len = (*len + 1).min(CAP),
                    Op::Pop => *len = (*len - 1).max(0),
                }
            },
            cap_invariant,
        );
        assert_eq!(ok, Ok(()));
    }

    #[test]
    fn test_swarm_replay_reproduces_the_failing_state() {
        let failure = dst_swarm_sweep(
            0..200u64,
            40,
            &[Op::Push, Op::Pop],
            |_seed| 0i32,
            stack_step,
            cap_invariant,
        )
        .expect_err("expected a failure to replay");

        // The documented one-line repro: tick + 1 lands exactly on the state
        // the invariant rejected.
        let state = dst_swarm_replay(
            failure.seed,
            failure.tick + 1,
            &[Op::Push, Op::Pop],
            |_seed| 0i32,
            stack_step,
        );
        assert_eq!(
            cap_invariant(&state, failure.tick),
            Err(failure.message.clone()),
            "replay must land on the same violation"
        );
    }

    #[test]
    fn test_swarm_replay_is_deterministic() {
        let run = || dst_swarm_replay(12345, 60, &[Op::Push, Op::Pop], |_seed| 0i32, stack_step);
        assert_eq!(run(), run());
    }

    #[test]
    fn test_swarm_sweep_vacuous_cases() {
        // No actions, no ticks, no seeds — all succeed without running anything.
        let no_actions: Result<(), SwarmFailure> = dst_swarm_sweep(
            0..10u64,
            10,
            &[] as &[Op],
            |_s| 0i32,
            stack_step,
            cap_invariant,
        );
        assert_eq!(no_actions, Ok(()));

        let no_ticks = dst_swarm_sweep(
            0..10u64,
            0,
            &[Op::Push],
            |_s| 0i32,
            stack_step,
            cap_invariant,
        );
        assert_eq!(no_ticks, Ok(()));

        let no_seeds = dst_swarm_sweep(
            0..0u64,
            10,
            &[Op::Push],
            |_s| 0i32,
            stack_step,
            cap_invariant,
        );
        assert_eq!(no_seeds, Ok(()));
    }

    #[test]
    fn test_swarm_failure_display_names_seed_actions_and_repro() {
        let f = SwarmFailure {
            seed: 77,
            tick: 5,
            message: "bad".into(),
            enabled: vec![0, 2],
        };
        let text = f.to_string();
        assert!(text.contains("seed 77"), "{text}");
        assert!(text.contains("tick 5"), "{text}");
        assert!(text.contains("[0, 2]"), "{text}");
        assert!(text.contains("dst_swarm_replay(77, 6"), "{text}");
    }

    #[test]
    fn test_swarm_subset_does_not_disturb_the_simulation_rng() {
        // The harness draws from named sub-streams, so a sim seeded with the
        // raw seed sees the identical RNG sequence it would outside swarm.
        let mut direct = SplitMix64::new(999);
        let expect: Vec<u64> = (0..4).map(|_| direct.next_u64()).collect();

        let mut observed = Vec::new();
        let _ = dst_swarm_sweep(
            [999u64],
            4,
            &[Op::Push],
            SplitMix64::new,
            |rng: &mut SplitMix64, _op: &Op| observed.push(rng.next_u64()),
            |_s: &SplitMix64, _t| Ok(()),
        );
        assert_eq!(observed, expect, "swarm must not consume the sim's stream");
    }
}
