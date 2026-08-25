//! Replay harness integration.
//!
//! The unit tests in `src/replay.rs` verify `record_trace`, `check_trace`, and
//! `resimulate` with a trivial two-field struct. This file proves the same
//! claims against a simulation built from real kit modules — `Scheduler<u32>`
//! driven by `SplitMix64` — where the interaction between the two creates the
//! genuine replay risk.
//!
//! Three Socratic claims under test:
//!
//! 1. **Trace reproducibility** (`check_trace` round-trip): identical seed +
//!    identical input sequence → `check_trace` returns `Ok(())` every time.
//! 2. **Divergence detection** (fault injection): changing one input at tick K
//!    causes `check_trace` to report a divergence at exactly tick K — proven by
//!    injecting a `SetSpeed` event that was absent from the recorded run.
//! 3. **Rollback fidelity** (`resimulate`): replaying the tail inputs from a
//!    mid-run `Clone` snapshot produces the same final state hash as the
//!    unbroken run, and leaves the snapshot itself unchanged — the invariant
//!    rollback netcode depends on.
//!
//! Deterministic via `SplitMix64`; seeds use the `0xHHHH_HHHH` four-group
//! convention adopted across the test suite.

use izanagi_kit::turn::ACTION_COST;
use izanagi_kit::{
    check_trace, first_divergence, hash_state, record_trace, resimulate, DetHash, Fnv1a, Scheduler,
    SplitMix64,
};

const SEED: u64 = 0xD4CE_0501;
const TRIALS: usize = 200;

// ── Simulation types ──────────────────────────────────────────────────────────

/// External input supplied by the caller at each replay tick.
#[derive(Clone)]
enum TickInput {
    /// Advance the turn queue by one step: pop the ready actor, apply a
    /// randomised outcome to `score`, and advance the RNG stream.
    Tick,
    /// Change actor `id`'s speed to `speed` without advancing the queue.
    SetSpeed { id: u32, speed: i32 },
}

/// Simulation state. `Clone`-able for `resimulate`/rollback.
#[derive(Clone)]
struct TickSim {
    sched: Scheduler<u32>,
    rng: SplitMix64,
    /// Accumulated outcome — depends on both turn order and RNG, so any
    /// divergence in either propagates into the hash.
    score: i64,
}

impl TickSim {
    fn new(seed: u64) -> Self {
        let mut sched: Scheduler<u32> = Scheduler::new();
        // Three actors with distinct, fixed speeds so the turn order is
        // non-trivial and consistently interleaved across the whole trace.
        sched.add(0, ACTION_COST);
        sched.add(1, ACTION_COST * 3 / 2); // 1.5× — acts more often than 0
        sched.add(2, ACTION_COST * 2); // 2× — fastest
        TickSim {
            sched,
            rng: SplitMix64::new(seed),
            score: 0,
        }
    }
}

impl DetHash for TickSim {
    fn det_hash(&self, h: &mut Fnv1a) {
        // `Scheduler<u32>` exposes `det_hash` as a plain method (not the
        // `DetHash` trait — it sorts actors by id internally, so the hash is
        // insertion-order-independent and replay-safe).
        self.sched.det_hash(h);
        // `SplitMix64` implements `DetHash` as a trait.
        self.rng.det_hash(h);
        h.write_i64(self.score);
    }
}

fn step(sim: &mut TickSim, input: &TickInput) {
    match input {
        TickInput::Tick => {
            if let Some(id) = sim.sched.next_turn() {
                // A randomised outcome scaled by actor id makes score depend on
                // both *who* acted and *when*, so speed changes cascade
                // visibly into the score (and hence the hash).
                let outcome = sim.rng.below(20) as i64;
                sim.score = sim.score.wrapping_add(outcome * id as i64 + 1);
            }
        }
        TickInput::SetSpeed { id, speed } => {
            sim.sched.set_speed(*id, *speed);
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Combine two `below(2^31)` draws into a `u64` seed (avoids reaching for
/// platform-dependent constructs while staying within what `SplitMix64`
/// accepts).
fn gen_seed(rng: &mut SplitMix64) -> u64 {
    let hi = rng.below(0x7FFF_FFFF) as u64;
    let lo = rng.below(0x7FFF_FFFF) as u64;
    hi << 32 | lo | 1 // always non-zero; SplitMix64 handles any u64
}

/// Build the canonical mixed input sequence used by several tests below.
fn mixed_inputs(n: usize) -> Vec<TickInput> {
    (0..n)
        .map(|i| {
            if i % 20 == 9 {
                // Inject one speed change every 20 ticks — keeps turn order
                // interesting without drowning out the Tick steps.
                TickInput::SetSpeed {
                    id: (i / 20 % 3) as u32,
                    speed: ACTION_COST + (i * 7) as i32,
                }
            } else {
                TickInput::Tick
            }
        })
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Claim 1 (basic round-trip): recording and re-running the same seed + inputs
/// always produces an identical trace.
#[test]
fn replay_trace_is_reproducible_with_scheduler_and_rng() {
    let inputs = mixed_inputs(120);
    let a = record_trace(&mut TickSim::new(SEED), &inputs, step);
    let b = record_trace(&mut TickSim::new(SEED), &inputs, step);
    assert_eq!(a, b, "identical seed + inputs must reproduce the trace");
    assert_eq!(a.len(), inputs.len(), "trace length must match input count");
}

/// `check_trace` round-trip: a faithful replay is always accepted.
#[test]
fn check_trace_accepts_faithful_replay_over_scheduler_state() {
    let inputs: Vec<TickInput> = (0..80).map(|_| TickInput::Tick).collect();
    let expected = record_trace(&mut TickSim::new(SEED), &inputs, step);
    assert_eq!(
        check_trace(&mut TickSim::new(SEED), &inputs, &expected, step),
        Ok(()),
        "check_trace must accept a faithful replay"
    );
}

/// Claim 2 (fault injection): injecting a `SetSpeed` at tick 20 that was
/// absent from the recorded run must cause `check_trace` to report a divergence
/// at exactly tick 20 — not before, not silently, not at a later tick.
#[test]
fn check_trace_detects_divergence_at_the_mutated_tick() {
    let mut inputs: Vec<TickInput> = (0..60).map(|_| TickInput::Tick).collect();
    let expected = record_trace(&mut TickSim::new(SEED), &inputs, step);

    // Poison tick 20 with a speed change that was not in the original.
    inputs[20] = TickInput::SetSpeed {
        id: 1,
        speed: ACTION_COST * 3,
    };
    match check_trace(&mut TickSim::new(SEED), &inputs, &expected, step) {
        Err(d) => assert_eq!(
            d.tick,
            20,
            "divergence must be at the mutated tick, not at {tick}",
            tick = d.tick
        ),
        Ok(()) => panic!("check_trace accepted a replay with a mutated input at tick 20"),
    }
}

/// A second fault-injection variant: mutating a later tick (tick 40) produces
/// a divergence there, confirming the detection is tick-accurate, not just
/// "some divergence somewhere".
#[test]
fn check_trace_divergence_is_tick_accurate_not_just_present() {
    let mut inputs: Vec<TickInput> = (0..80).map(|_| TickInput::Tick).collect();
    let expected = record_trace(&mut TickSim::new(SEED), &inputs, step);

    inputs[40] = TickInput::SetSpeed {
        id: 2,
        speed: ACTION_COST / 4 + 1,
    };
    match check_trace(&mut TickSim::new(SEED), &inputs, &expected, step) {
        Err(d) => assert_eq!(
            d.tick,
            40,
            "divergence must be at the injected tick 40, not {tick}",
            tick = d.tick
        ),
        Ok(()) => panic!("check_trace accepted a replay with a mutated input at tick 40"),
    }
}

/// Claim 3 (rollback fidelity): `resimulate` from a mid-run snapshot produces
/// the same final state hash as the unbroken inline run.
#[test]
fn resimulate_from_snapshot_matches_inline_continuation() {
    let inputs = mixed_inputs(100);
    const SNAP_TICK: usize = 30;

    // Inline run: advance to SNAP_TICK, take a snapshot, finish.
    let mut inline = TickSim::new(SEED);
    for input in &inputs[..SNAP_TICK] {
        step(&mut inline, input);
    }
    let snapshot = inline.clone();
    for input in &inputs[SNAP_TICK..] {
        step(&mut inline, input);
    }
    let inline_final = hash_state(&inline);

    // Rollback path: resimulate the tail from the snapshot.
    let rolled = resimulate(&snapshot, &inputs[SNAP_TICK..], step);
    assert_eq!(
        hash_state(&rolled),
        inline_final,
        "resimulate must produce the same final state as the inline continuation"
    );
}

/// `resimulate` must not mutate the snapshot it was given — the caller must be
/// able to re-use the same snapshot for multiple rollback attempts.
#[test]
fn resimulate_leaves_snapshot_state_unchanged() {
    let inputs: Vec<TickInput> = (0..50).map(|_| TickInput::Tick).collect();
    const SNAP_TICK: usize = 20;

    let mut sim = TickSim::new(SEED);
    for input in &inputs[..SNAP_TICK] {
        step(&mut sim, input);
    }
    let snap_hash_before = hash_state(&sim);

    let _rolled = resimulate(&sim, &inputs[SNAP_TICK..], step);

    assert_eq!(
        hash_state(&sim),
        snap_hash_before,
        "resimulate must not mutate the snapshot"
    );
}

/// Two simulations that differ only in one actor's initial speed diverge
/// immediately — `first_divergence` reports a mismatch somewhere in the trace.
/// This confirms that the hash actually captures scheduler energy/speed state.
#[test]
fn different_actor_speeds_cause_trace_divergence() {
    let inputs: Vec<TickInput> = (0..60).map(|_| TickInput::Tick).collect();

    let trace_a = record_trace(&mut TickSim::new(SEED), &inputs, step);

    // Sim B: override actor 2's speed to 5×, altering the turn order.
    let mut sim_b = TickSim::new(SEED);
    sim_b.sched.set_speed(2, ACTION_COST * 5);
    let trace_b = record_trace(&mut sim_b, &inputs, step);

    assert!(
        first_divergence(&trace_a, &trace_b).is_err(),
        "different actor speeds must produce diverging hash traces"
    );
}

/// Property (200 random seeds × random input lengths): `check_trace` always
/// accepts a faithful replay — no seed or sequence length causes a spurious
/// divergence in an unmodified replay.
#[test]
fn replay_check_is_consistent_over_random_seeds_and_inputs() {
    let mut rng = SplitMix64::new(0x0C4E_4B05);
    for _ in 0..TRIALS {
        let n = rng.range(20, 80) as usize;
        let inputs: Vec<TickInput> = (0..n)
            .map(|_| {
                if rng.below(5) == 0 {
                    TickInput::SetSpeed {
                        id: rng.below(3),
                        speed: ACTION_COST + rng.below(200) as i32,
                    }
                } else {
                    TickInput::Tick
                }
            })
            .collect();
        let seed = gen_seed(&mut rng);
        let expected = record_trace(&mut TickSim::new(seed), &inputs, step);
        assert_eq!(
            check_trace(&mut TickSim::new(seed), &inputs, &expected, step),
            Ok(()),
            "replay round-trip must always be consistent (seed={seed:#x})"
        );
    }
}

/// Property (200 random seeds): `resimulate` from a randomly chosen snapshot
/// tick always matches the inline continuation — `Clone` faithfully captures
/// all relevant simulation state regardless of how much time has passed.
#[test]
fn resimulate_fidelity_holds_over_random_snapshot_ticks() {
    let mut rng = SplitMix64::new(0x074E_C505);
    for _ in 0..TRIALS {
        let n = rng.range(40, 100) as usize;
        let snap_at = rng.below(n as u32 / 2) as usize + 5;
        let inputs: Vec<TickInput> = (0..n).map(|_| TickInput::Tick).collect();
        let seed = gen_seed(&mut rng);

        // Inline run.
        let mut inline = TickSim::new(seed);
        for input in &inputs[..snap_at] {
            step(&mut inline, input);
        }
        let snapshot = inline.clone();
        for input in &inputs[snap_at..] {
            step(&mut inline, input);
        }

        // Rollback path.
        let rolled = resimulate(&snapshot, &inputs[snap_at..], step);
        assert_eq!(
            hash_state(&rolled),
            hash_state(&inline),
            "resimulate must match inline continuation (seed={seed:#x}, snap_at={snap_at})"
        );
    }
}
