//! Fixed-timestep accumulator — the simulation clock.
//!
//! Per the determinism literature (Fiedler "Fix Your Timestep!", and the game
//! engine determinism study arXiv:2104.06262): a variable render frame rate
//! must not change simulation results. The simulation advances in fixed `dt`
//! chunks; the renderer runs as fast as it likes and interpolates with the
//! leftover fraction (`alpha`). This decouples "how it looks" from "what
//! happens", so a replay driven by the same inputs is bit-identical regardless
//! of machine speed.
//!
//! Death-spiral guard: if a frame delivers more accumulated time than
//! `max_steps` chunks (a stall, a breakpoint, a slow machine), the surplus is
//! dropped. The sim then appears to slow down rather than trying to "catch up"
//! forever and falling further behind.
//!
//! Time is integer nanoseconds — never wall-clock floats — so the accumulator
//! itself introduces no nondeterminism.

/// Drives fixed-step simulation from variable real-frame durations.
#[derive(Clone, Debug)]
pub struct FixedTimestep {
    step_ns: u64,
    accumulator_ns: u64,
    max_steps: u32,
    total_steps: u64,
}

impl FixedTimestep {
    /// `steps_per_second` is the simulation tick rate (e.g. 60). `max_steps`
    /// caps catch-up work per frame (e.g. 5) to prevent the death spiral.
    pub fn new(steps_per_second: u32, max_steps: u32) -> Self {
        assert!(steps_per_second > 0, "steps_per_second must be > 0");
        assert!(max_steps > 0, "max_steps must be > 0");
        Self {
            step_ns: 1_000_000_000 / steps_per_second as u64,
            accumulator_ns: 0,
            max_steps,
            total_steps: 0,
        }
    }

    /// Standard 60 Hz sim, up to 5 catch-up steps per frame.
    pub fn sixty_hz() -> Self {
        Self::new(60, 5)
    }

    #[inline]
    pub fn step_ns(&self) -> u64 {
        self.step_ns
    }

    /// Monotonically increasing count of simulation steps taken. Folding this
    /// into a replay log lets a divergence be located by step index.
    #[inline]
    pub fn total_steps(&self) -> u64 {
        self.total_steps
    }

    /// Current sub-step time buffered in the accumulator (nanoseconds).
    /// Always in `[0, step_ns)`. Exposes the value normally hidden inside the
    /// struct so callers can serialise it alongside `total_steps` for an exact
    /// save/restore of the timestep state — or verify that two replay runs are
    /// at identical accumulator positions.
    #[inline]
    pub fn accumulator_ns(&self) -> u64 {
        self.accumulator_ns
    }

    /// Deposits one real frame's elapsed time and returns how many fixed steps
    /// to run now. Surplus beyond `max_steps` is discarded (death-spiral guard).
    pub fn advance(&mut self, frame_ns: u64) -> u32 {
        self.accumulator_ns = self.accumulator_ns.saturating_add(frame_ns);
        let mut steps = 0u32;
        while self.accumulator_ns >= self.step_ns && steps < self.max_steps {
            self.accumulator_ns -= self.step_ns;
            steps += 1;
        }
        if steps == self.max_steps && self.accumulator_ns >= self.step_ns {
            // Clamp: drop the backlog so we don't spiral. Keep sub-step remainder.
            self.accumulator_ns %= self.step_ns;
        }
        self.total_steps += steps as u64;
        steps
    }

    /// Interpolation factor in [0, 1): how far between the last and next step the
    /// renderer should draw. `numerator/denominator` form keeps it float-free.
    #[inline]
    pub fn alpha_ratio(&self) -> (u64, u64) {
        (self.accumulator_ns, self.step_ns)
    }

    /// Discard any accumulated sub-step time, returning the nanoseconds dropped.
    /// Call on resume after a pause, a breakpoint, or a level load so the first
    /// new frame does not replay buffered time as a burst of catch-up steps.
    /// Leaves `total_steps` and the configured rate untouched.
    #[inline]
    pub fn reset_accumulator(&mut self) -> u64 {
        let dropped = self.accumulator_ns;
        self.accumulator_ns = 0;
        dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_one_step_per_frame_at_matching_rate() {
        let mut ts = FixedTimestep::new(60, 5);
        let frame = 1_000_000_000 / 60;
        assert_eq!(ts.advance(frame), 1);
    }

    #[test]
    fn test_small_frames_accumulate_until_a_step() {
        let mut ts = FixedTimestep::new(60, 5);
        // Deliver the step in fifths; integer division leaves no remainder gap
        // (step_ns is divisible enough that 5 fifths >= one step is exact here
        // only if we add the true step_ns total, so feed exact fractions).
        let step = ts.step_ns();
        let part = step / 5;
        for _ in 0..4 {
            assert_eq!(ts.advance(part), 0, "not enough accumulated yet");
        }
        // 4 parts so far = 4/5 step. Deliver the remainder to cross the line.
        let delivered = part * 4;
        assert_eq!(
            ts.advance(step - delivered),
            1,
            "remainder completes one step"
        );
    }

    #[test]
    fn test_big_frame_runs_multiple_steps() {
        let mut ts = FixedTimestep::new(60, 5);
        let three = ts.step_ns() * 3;
        assert_eq!(ts.advance(three), 3);
    }

    #[test]
    fn test_death_spiral_is_clamped() {
        let mut ts = FixedTimestep::new(60, 5);
        // A 1-second stall would be 60 steps; max_steps caps it at 5.
        let huge = 1_000_000_000;
        assert_eq!(ts.advance(huge), 5, "must clamp to max_steps");
        // Backlog dropped: next normal frame yields exactly one step.
        assert_eq!(ts.advance(ts.step_ns()), 1, "no catch-up debt remains");
    }

    #[test]
    fn test_total_steps_accumulates() {
        let mut ts = FixedTimestep::new(60, 5);
        ts.advance(ts.step_ns() * 2);
        ts.advance(ts.step_ns());
        assert_eq!(ts.total_steps(), 3);
    }

    #[test]
    fn test_reset_accumulator_returns_dropped_time() {
        let mut ts = FixedTimestep::new(60, 5);
        let part = ts.step_ns() / 2;
        assert_eq!(ts.advance(part), 0); // below one step
        assert_eq!(ts.reset_accumulator(), part, "must return the dropped ns");
        // Accumulator now empty: re-dropping yields 0.
        assert_eq!(ts.reset_accumulator(), 0);
    }

    #[test]
    fn test_reset_accumulator_prevents_carryover() {
        let mut ts = FixedTimestep::new(60, 5);
        let step = ts.step_ns();
        ts.advance(step - 1); // 0 steps, accumulator = step-1
        ts.reset_accumulator();
        // Without the reset, (step-1)+2 >= step would fire a step; after reset it must not.
        assert_eq!(ts.advance(2), 0, "reset must clear buffered time");
    }

    #[test]
    fn test_reset_accumulator_keeps_total_steps() {
        let mut ts = FixedTimestep::new(60, 5);
        ts.advance(ts.step_ns() * 2);
        assert_eq!(ts.total_steps(), 2);
        ts.reset_accumulator();
        assert_eq!(ts.total_steps(), 2, "reset must not touch total_steps");
    }

    #[test]
    fn test_accumulator_ns_starts_zero() {
        let ts = FixedTimestep::new(60, 5);
        assert_eq!(ts.accumulator_ns(), 0);
    }

    #[test]
    fn test_accumulator_ns_tracks_partial_frame() {
        let mut ts = FixedTimestep::new(60, 5);
        let half = ts.step_ns() / 2;
        ts.advance(half);
        assert_eq!(ts.accumulator_ns(), half);
    }

    #[test]
    fn test_accumulator_ns_clears_after_reset() {
        let mut ts = FixedTimestep::new(60, 5);
        ts.advance(ts.step_ns() / 3);
        ts.reset_accumulator();
        assert_eq!(ts.accumulator_ns(), 0);
    }

    #[test]
    fn test_determinism_independent_of_frame_pacing() {
        // Same total time delivered in different chunkings => same step count.
        let total = FixedTimestep::new(60, 1000).step_ns() * 100;
        let mut a = FixedTimestep::new(60, 1000);
        a.advance(total); // one big frame
        let mut b = FixedTimestep::new(60, 1000);
        for _ in 0..200 {
            b.advance(total / 200); // many small frames
        }
        assert_eq!(a.total_steps(), b.total_steps());
    }
}
