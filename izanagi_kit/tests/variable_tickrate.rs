//! Variable-tickrate property: render pacing must not change the simulation.
//!
//! RESEARCH.md section 5, candidate 6. `FixedTimestep` exists so that a
//! variable render frame rate cannot change simulation results (Fiedler,
//! "Fix Your Timestep!"). The unit tests in `timestep.rs` pin the step
//! *count* for two chunkings of the same total time; this file pins the
//! stronger claim the engine actually relies on: the per-tick world-hash
//! *trace* of an input-driven simulation is identical under different
//! render pacings.
//!
//! The claim holds under precise preconditions, and only under them:
//!
//! - the simulation rate is fixed (60 Hz here) — changing the rate changes
//!   `step_ns` itself, so traces across rates are not comparable;
//! - every pacing delivers the same total time and the same per-tick input
//!   sequence (inputs are indexed by tick, never by frame);
//! - **no frame ever triggers the death-spiral clamp**. The clamp discards
//!   accumulated sim time by design, so a pacing that clamps runs fewer
//!   ticks and its trace legitimately diverges. Each property run below
//!   therefore asserts `dropped_steps() == 0`, and
//!   `clamp_boundary_marks_where_invariance_ends` pins the other side of
//!   the boundary: exactly one dropped step, exactly one tick fewer.
//!
//! Without clamping, the accumulator loses no time, so the executed tick
//! sequence is a pure function of cumulative delivered time — frame
//! boundaries cannot leak into the simulation. All frame durations are
//! data in this file; nothing reads a wall clock.

use izanagi_kit::{
    DetHash, Entity, EntityAllocator, Fixed, FixedTimestep, Fnv1a, SparseSet, SplitMix64,
};

#[derive(Clone, Copy)]
struct Position {
    x: Fixed,
    y: Fixed,
}

#[derive(Clone, Copy)]
struct Velocity {
    dx: Fixed,
    dy: Fixed,
}

impl DetHash for Position {
    fn det_hash(&self, h: &mut Fnv1a) {
        h.write_i32(self.x.raw());
        h.write_i32(self.y.raw());
    }
}

/// A small input-driven world: a hero entity steered by the per-tick input,
/// plus seeded-RNG population churn (the same shape as `determinism.rs`,
/// with the hero kept out of the churn list so it is never despawned).
struct World {
    alloc: EntityAllocator,
    positions: SparseSet<Position>,
    velocities: SparseSet<Velocity>,
    live: Vec<Entity>,
    hero: Entity,
    rng: SplitMix64,
}

impl World {
    fn new(seed: u64) -> Self {
        let mut alloc = EntityAllocator::new();
        let hero = alloc.allocate();
        let mut positions = SparseSet::new();
        let mut velocities = SparseSet::new();
        positions.insert(
            hero,
            Position {
                x: Fixed::ZERO,
                y: Fixed::ZERO,
            },
        );
        velocities.insert(
            hero,
            Velocity {
                dx: Fixed::ZERO,
                dy: Fixed::ZERO,
            },
        );
        Self {
            alloc,
            positions,
            velocities,
            live: Vec::new(),
            hero,
            rng: SplitMix64::new(seed),
        }
    }

    fn spawn(&mut self) {
        let e = self.alloc.allocate();
        let vx = self.rng.below(7) as i32 - 3;
        let vy = self.rng.below(7) as i32 - 3;
        self.positions.insert(
            e,
            Position {
                x: Fixed::ZERO,
                y: Fixed::ZERO,
            },
        );
        self.velocities.insert(
            e,
            Velocity {
                dx: Fixed::from_int(vx),
                dy: Fixed::from_int(vy),
            },
        );
        self.live.push(e);
    }

    fn despawn_oldest(&mut self) {
        if self.live.is_empty() {
            return;
        }
        let e = self.live.remove(0);
        self.positions.remove(e);
        self.velocities.remove(e);
        self.alloc.free(e);
    }

    fn integrate(&mut self) {
        // Canonical (sorted) iteration order, as in `determinism.rs`.
        let updates: Vec<(Entity, Velocity)> = self
            .velocities
            .iter_sorted()
            .iter()
            .map(|(e, v)| (*e, **v))
            .collect();
        for (e, v) in updates {
            if let Some(p) = self.positions.get_mut(e) {
                p.x = p.x + v.dx;
                p.y = p.y + v.dy;
            }
        }
    }

    fn step(&mut self, input: u8) {
        // The per-tick input steers the hero. Every input value moves the
        // velocity (dx cycles with `input % 5`, dy with `(input / 5) % 5`),
        // so no input is silently ignored — the sensitivity test below
        // relies on this.
        let hero_v = self
            .velocities
            .get_mut(self.hero)
            .expect("the hero always has a velocity");
        hero_v.dx = Fixed::from_int((input % 5) as i32 - 2);
        hero_v.dy = Fixed::from_int(((input / 5) % 5) as i32 - 2);
        // RNG-driven population churn, then physics. Fixed op order.
        if self.rng.below(100) < 60 {
            self.spawn();
        }
        if self.rng.below(100) < 25 {
            self.despawn_oldest();
        }
        self.integrate();
    }

    /// Same projection digest as `determinism.rs`: sorted positions plus
    /// the RNG stream position.
    fn hash(&self) -> u64 {
        let mut h = Fnv1a::new();
        for (e, p) in self.positions.iter_sorted() {
            h.write_u32(e.index());
            h.write_u32(e.generation());
            p.det_hash(&mut h);
        }
        h.write_u64(self.rng.state());
        h.finish()
    }
}

/// What one paced run produced: the per-tick hash trace plus the final
/// timestep counters, so equality covers the clock as well as the world.
struct Outcome {
    trace: Vec<u64>,
    total_steps: u64,
    dropped_steps: u64,
    accumulator_ns: u64,
}

/// Drives `world` through `frames_ns`, consuming `inputs` one per tick.
/// A pacing that delivers more steps than there are inputs is a bug in
/// the test data, so it panics loudly instead of indexing past the end.
fn run_paced(seed: u64, inputs: &[u8], frames_ns: &[u64], max_steps: u32) -> Outcome {
    let mut ts = FixedTimestep::new(60, max_steps);
    let mut world = World::new(seed);
    let mut trace = Vec::with_capacity(inputs.len());
    let mut tick = 0usize;
    for &frame_ns in frames_ns {
        let steps = ts.advance(frame_ns);
        for _ in 0..steps {
            assert!(
                tick < inputs.len(),
                "pacing delivered more steps than there are inputs"
            );
            world.step(inputs[tick]);
            tick += 1;
            trace.push(world.hash());
        }
    }
    Outcome {
        trace,
        total_steps: ts.total_steps(),
        dropped_steps: ts.dropped_steps(),
        accumulator_ns: ts.accumulator_ns(),
    }
}

// ── frame pacings ────────────────────────────────────────────────────────────
// All pacings deliver exactly `TICKS * step_ns` in total. `step_ns` at
// 60 Hz is 16_666_666 ns — even — so half-step units below are exact.

const TICKS: usize = 240;

fn step_ns() -> u64 {
    FixedTimestep::new(60, 5).step_ns()
}

/// One frame per tick: render rate equals sim rate.
fn pacing_matched() -> Vec<u64> {
    vec![step_ns(); TICKS]
}

/// One frame per two ticks: render at half the sim rate.
fn pacing_half_rate() -> Vec<u64> {
    vec![step_ns() * 2; TICKS / 2]
}

/// Two frames per tick: render at double the sim rate.
fn pacing_double_rate() -> Vec<u64> {
    vec![step_ns() / 2; TICKS * 2]
}

/// Irregular cycle including zero-length frames; sums to 6 steps per cycle
/// (12 half-step units). The largest frame is 2 steps, far under the
/// `max_steps` used by the property runs, so the clamp never fires.
fn pacing_irregular() -> Vec<u64> {
    let unit = step_ns() / 2;
    let cycle = [unit, 3 * unit, 0, 2 * unit, 4 * unit, 2 * unit];
    let mut frames = Vec::new();
    for _ in 0..TICKS / 6 {
        frames.extend_from_slice(&cycle);
    }
    frames
}

/// The irregular cycle's frames in reverse order within each cycle: the
/// same multiset of durations, delivered in a different order.
fn pacing_irregular_reversed() -> Vec<u64> {
    let unit = step_ns() / 2;
    let cycle = [2 * unit, 4 * unit, 2 * unit, 0, 3 * unit, unit];
    let mut frames = Vec::new();
    for _ in 0..TICKS / 6 {
        frames.extend_from_slice(&cycle);
    }
    frames
}

fn all_pacings() -> Vec<(&'static str, Vec<u64>)> {
    vec![
        ("matched 60 fps", pacing_matched()),
        ("half rate 30 fps", pacing_half_rate()),
        ("double rate 120 fps", pacing_double_rate()),
        ("irregular", pacing_irregular()),
        ("irregular reversed", pacing_irregular_reversed()),
    ]
}

// ── input sequences ──────────────────────────────────────────────────────────

fn rng_inputs(seed: u64) -> Vec<u8> {
    let mut rng = SplitMix64::new(seed);
    (0..TICKS).map(|_| (rng.next_u64() % 25) as u8).collect()
}

fn alternating_inputs() -> Vec<u8> {
    (0..TICKS).map(|i| (i % 25) as u8).collect()
}

// ── tests ────────────────────────────────────────────────────────────────────

#[test]
fn hash_trace_is_invariant_under_render_pacing() {
    let pacings = all_pacings();
    for (name, frames) in &pacings {
        assert_eq!(
            frames.iter().sum::<u64>(),
            TICKS as u64 * step_ns(),
            "pacing '{name}' must deliver exactly {TICKS} steps of time"
        );
    }
    let input_sets: Vec<Vec<u8>> = vec![
        rng_inputs(0x5EED),
        rng_inputs(0xBADC0DE),
        vec![0u8; TICKS],
        alternating_inputs(),
    ];
    // max_steps comfortably above the largest frame (2 steps), so the
    // clamp cannot fire and the invariance precondition holds.
    let max_steps = 8;
    for seed in [1u64, 7, 0xC0FFEE00] {
        for inputs in &input_sets {
            let reference = run_paced(seed, inputs, &pacings[0].1, max_steps);
            assert_eq!(reference.trace.len(), TICKS);
            assert_eq!(reference.total_steps, TICKS as u64);
            assert_eq!(
                reference.dropped_steps, 0,
                "precondition: the reference pacing must not clamp"
            );
            assert_eq!(reference.accumulator_ns, 0);
            // Non-vacuity: the world must actually evolve, or equal traces
            // would prove nothing.
            let unique: std::collections::HashSet<u64> = reference.trace.iter().copied().collect();
            assert!(
                unique.len() > 50,
                "simulation must actually evolve state ({} distinct hashes)",
                unique.len()
            );
            for (name, frames) in &pacings[1..] {
                let outcome = run_paced(seed, inputs, frames, max_steps);
                assert_eq!(
                    outcome.dropped_steps, 0,
                    "precondition: pacing '{name}' must not clamp"
                );
                assert_eq!(
                    outcome.trace, reference.trace,
                    "pacing '{name}' changed the simulation (seed {seed})"
                );
                assert_eq!(outcome.total_steps, reference.total_steps);
                assert_eq!(outcome.accumulator_ns, reference.accumulator_ns);
            }
        }
    }
}

#[test]
fn same_pacing_twice_is_bit_identical() {
    let inputs = rng_inputs(0x5EED);
    let frames = pacing_irregular();
    let a = run_paced(42, &inputs, &frames, 8);
    let b = run_paced(42, &inputs, &frames, 8);
    assert_eq!(a.trace, b.trace);
    assert_eq!(a.total_steps, b.total_steps);
    assert_eq!(a.dropped_steps, b.dropped_steps);
}

#[test]
fn zero_duration_frames_change_nothing() {
    // A bare clock first: a 0 ns frame emits no step and buffers nothing.
    let mut ts = FixedTimestep::new(60, 5);
    assert_eq!(ts.advance(0), 0);
    assert_eq!(ts.total_steps(), 0);
    assert_eq!(ts.accumulator_ns(), 0);

    // Then through the sim: zero frames interleaved before every real
    // frame must leave the trace untouched.
    let inputs = rng_inputs(0x5EED);
    let mut with_zeros = Vec::new();
    for frame in pacing_matched() {
        with_zeros.push(0);
        with_zeros.push(frame);
    }
    let reference = run_paced(7, &inputs, &pacing_matched(), 8);
    let outcome = run_paced(7, &inputs, &with_zeros, 8);
    assert_eq!(outcome.trace, reference.trace);
    assert_eq!(outcome.total_steps, TICKS as u64);
    assert_eq!(outcome.dropped_steps, 0);
}

#[test]
fn frame_order_permutation_preserves_the_trace() {
    // Named separately from the pacing sweep: delivery *order* of the same
    // durations is not allowed to matter, only the cumulative sum.
    let inputs = rng_inputs(0xBADC0DE);
    let forward = run_paced(7, &inputs, &pacing_irregular(), 8);
    let reversed = run_paced(7, &inputs, &pacing_irregular_reversed(), 8);
    assert_eq!(forward.dropped_steps, 0);
    assert_eq!(reversed.dropped_steps, 0);
    assert_eq!(forward.trace, reversed.trace);
}

#[test]
fn frames_just_below_the_clamp_boundary_stay_exact() {
    // With max_steps = 5, a frame of exactly 5 steps — and one carrying
    // 5 steps plus a sub-step remainder — runs its steps and drops
    // nothing: the remainder is buffered time, not backlog.
    let step = step_ns();
    let mut frames = Vec::new();
    for _ in 0..TICKS / 10 {
        frames.push(5 * step + (step - 1));
        frames.push(5 * step - (step - 1));
    }
    assert_eq!(frames.iter().sum::<u64>(), TICKS as u64 * step);
    let inputs = rng_inputs(0x5EED);
    let reference = run_paced(7, &inputs, &pacing_matched(), 5);
    let outcome = run_paced(7, &inputs, &frames, 5);
    assert_eq!(
        outcome.dropped_steps, 0,
        "no frame may clamp at the boundary"
    );
    assert_eq!(outcome.trace, reference.trace);
    assert_eq!(outcome.accumulator_ns, 0);
}

#[test]
fn clamp_boundary_marks_where_invariance_ends() {
    // One frame of 6 steps against max_steps = 5: the guard runs 5 steps
    // and discards exactly 1 step of sim time. The run therefore executes
    // one tick fewer than the same total time delivered without clamping —
    // this is the documented edge of the invariance claim, pinned so the
    // precondition in the property test cannot silently rot.
    let step = step_ns();
    let mut frames = vec![6 * step];
    frames.extend(vec![step; TICKS - 6]);
    assert_eq!(frames.iter().sum::<u64>(), TICKS as u64 * step);
    let inputs = rng_inputs(0x5EED);
    let outcome = run_paced(7, &inputs, &frames, 5);
    assert_eq!(outcome.dropped_steps, 1);
    assert_eq!(outcome.total_steps, TICKS as u64 - 1);
    assert_eq!(outcome.trace.len(), TICKS - 1);
}

#[test]
fn a_single_changed_input_changes_the_trace() {
    // Sensitivity guard: if inputs stopped reaching the simulation, the
    // invariance tests above would pass vacuously. Flipping one input at
    // tick 100 must leave the trace identical before that tick and move
    // the hash from that tick on.
    let a_inputs = vec![0u8; TICKS];
    let mut b_inputs = vec![0u8; TICKS];
    b_inputs[100] = 1;
    let frames = pacing_matched();
    let a = run_paced(7, &a_inputs, &frames, 8);
    let b = run_paced(7, &b_inputs, &frames, 8);
    assert_eq!(a.trace[..100], b.trace[..100]);
    assert_ne!(a.trace[100], b.trace[100]);
    assert_ne!(
        a.trace.last(),
        b.trace.last(),
        "the divergence must persist to the final hash"
    );
}
