//! Frame timing — variable dt, fixed-step accumulator, wall-clock.
//!
//! Most game logic runs on the variable [`Time::dt`]. Physics that needs
//! determinism wraps its update in a fixed-step accumulator:
//!
//! ```
//! use izanagi::Time;
//! let mut t = Time::new();
//! t.set_fixed_dt(1.0 / 60.0);
//! t.advance(0.05); // simulate 50 ms variable
//! while t.fixed_step() {
//!     // physics_tick(t.fixed_dt());  -- runs at exact 60 Hz
//! }
//! ```

use std::time::{SystemTime, UNIX_EPOCH};

/// Tracks elapsed time and last-frame delta.
pub struct Time {
    elapsed: f32,
    dt: f32,
    fixed_dt: f32,
    accumulator: f32,
    fixed_steps_this_frame: u32,
}

impl Time {
    /// Create a timer starting at zero. Fixed step defaults to 1/60.
    pub fn new() -> Self {
        Self {
            elapsed: 0.0,
            dt: 0.0,
            fixed_dt: 1.0 / 60.0,
            accumulator: 0.0,
            fixed_steps_this_frame: 0,
        }
    }

    /// Advance by `dt` seconds. Called by the engine.
    pub fn advance(&mut self, dt: f32) {
        // Clamp pathological values (debugger pauses, OS hiccups).
        let dt = dt.clamp(0.0, 0.25);
        self.dt = dt;
        self.elapsed += dt;
        self.accumulator += dt;
        self.fixed_steps_this_frame = 0;
    }

    /// Seconds elapsed since start.
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// Delta of the last `advance()` in seconds.
    pub fn dt(&self) -> f32 {
        self.dt
    }

    /// Configure the fixed-step duration (default 1/60).
    pub fn set_fixed_dt(&mut self, dt: f32) {
        self.fixed_dt = dt.max(1e-4);
    }

    /// The fixed-step duration.
    pub fn fixed_dt(&self) -> f32 {
        self.fixed_dt
    }

    /// Drain one fixed step from the accumulator.
    ///
    /// Use in a `while` loop: it returns `true` for each fixed step that
    /// has accumulated since the last frame, then `false` to exit.
    /// Caps at 8 steps per frame to prevent the spiral-of-death on slow
    /// frames.
    pub fn fixed_step(&mut self) -> bool {
        if self.fixed_steps_this_frame >= 8 {
            self.accumulator = 0.0;
            return false;
        }
        if self.accumulator >= self.fixed_dt {
            self.accumulator -= self.fixed_dt;
            self.fixed_steps_this_frame += 1;
            true
        } else {
            false
        }
    }

    /// 0..1 interpolation factor for rendering between fixed steps.
    /// Multiply by `fixed_dt()` to get the partial frame time.
    pub fn alpha(&self) -> f32 {
        (self.accumulator / self.fixed_dt).clamp(0.0, 1.0)
    }

    /// Wall-clock seconds since UNIX epoch. Useful for save timestamps.
    /// Returns 0.0 if the system clock is misconfigured.
    pub fn wall_clock_seconds() -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    }
}

impl Default for Time {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advances() {
        let mut t = Time::new();
        t.advance(0.016);
        t.advance(0.016);
        assert!((t.elapsed() - 0.032).abs() < 1e-5);
        assert!((t.dt() - 0.016).abs() < 1e-5);
    }

    #[test]
    fn advance_clamps_huge_dt() {
        let mut t = Time::new();
        t.advance(10.0); // OS hiccup
        assert!(t.dt() <= 0.25);
        assert!(t.elapsed() <= 0.25);
    }

    #[test]
    fn fixed_step_runs_correct_count() {
        let mut t = Time::new();
        t.set_fixed_dt(0.01);
        t.advance(0.034); // 3 full steps + remainder
        let mut steps = 0;
        while t.fixed_step() {
            steps += 1;
        }
        assert_eq!(steps, 3);
    }

    #[test]
    fn fixed_step_caps_at_8() {
        let mut t = Time::new();
        t.set_fixed_dt(0.001);
        t.advance(1.0); // would be 1000 steps without the cap
        let mut steps = 0;
        while t.fixed_step() {
            steps += 1;
        }
        assert_eq!(steps, 8);
    }

    #[test]
    fn alpha_in_range() {
        let mut t = Time::new();
        t.set_fixed_dt(0.01);
        t.advance(0.005);
        let a = t.alpha();
        assert!((0.0..=1.0).contains(&a));
        assert!((a - 0.5).abs() < 1e-3);
    }

    #[test]
    fn wall_clock_returns_recent_value() {
        let now = Time::wall_clock_seconds();
        // Sometime in 2024 or later (1.7 billion seconds since epoch).
        assert!(now > 1_700_000_000.0);
    }
}
