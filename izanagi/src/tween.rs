//! Time-based animation and timers.
//!
//! [`Tween`] interpolates a value from A to B over a duration using an
//! easing function. [`Timer`] counts down or repeats. Both are driven by
//! calling `.tick(dt)` every frame.

/// A one-shot animation from one f32 value to another.
pub struct Tween {
    from: f32,
    to: f32,
    duration: f32,
    elapsed: f32,
    ease: fn(f32) -> f32,
    done: bool,
}

impl Tween {
    /// Animate from `from` to `to` over `duration` seconds, using `ease`.
    pub fn new(from: f32, to: f32, duration: f32, ease: fn(f32) -> f32) -> Self {
        Self {
            from,
            to,
            duration: duration.max(f32::EPSILON),
            elapsed: 0.0,
            ease,
            done: false,
        }
    }

    /// Advance by `dt` seconds. Returns `true` when complete.
    pub fn tick(&mut self, dt: f32) -> bool {
        if self.done {
            return true;
        }
        self.elapsed = (self.elapsed + dt).min(self.duration);
        self.done = self.elapsed >= self.duration;
        self.done
    }

    /// Current interpolated value.
    pub fn value(&self) -> f32 {
        let t = (self.ease)((self.elapsed / self.duration).clamp(0.0, 1.0));
        self.from + (self.to - self.from) * t
    }

    /// 0.0 at start, 1.0 at end.
    pub fn progress(&self) -> f32 {
        (self.elapsed / self.duration).clamp(0.0, 1.0)
    }

    /// Has the animation completed?
    pub fn done(&self) -> bool {
        self.done
    }

    /// Restart from the beginning.
    pub fn restart(&mut self) {
        self.elapsed = 0.0;
        self.done = false;
    }

    /// Reverse direction and restart.
    pub fn reverse(&mut self) {
        std::mem::swap(&mut self.from, &mut self.to);
        self.restart();
    }
}

/// A countdown or repeating timer.
pub struct Timer {
    duration: f32,
    elapsed: f32,
    repeating: bool,
    just_finished: bool,
    finished: bool,
}

impl Timer {
    /// One-shot: fires once after `duration` seconds.
    pub fn once(duration: f32) -> Self {
        Self {
            duration: duration.max(f32::EPSILON),
            elapsed: 0.0,
            repeating: false,
            just_finished: false,
            finished: false,
        }
    }

    /// Repeating: fires every `duration` seconds.
    pub fn every(duration: f32) -> Self {
        Self {
            duration: duration.max(f32::EPSILON),
            elapsed: 0.0,
            repeating: true,
            just_finished: false,
            finished: false,
        }
    }

    /// Advance by `dt`. Returns `true` the frame the timer fires.
    pub fn tick(&mut self, dt: f32) -> bool {
        self.just_finished = false;
        if self.finished {
            return false;
        }
        self.elapsed += dt;
        if self.elapsed >= self.duration {
            self.just_finished = true;
            if self.repeating {
                self.elapsed -= self.duration;
            } else {
                self.elapsed = self.duration;
                self.finished = true;
            }
        }
        self.just_finished
    }

    /// True only on the frame the timer fires.
    pub fn just_finished(&self) -> bool {
        self.just_finished
    }

    /// True after a one-shot timer completes.
    pub fn finished(&self) -> bool {
        self.finished
    }

    /// 0.0 at start, 1.0 when due.
    pub fn fraction(&self) -> f32 {
        (self.elapsed / self.duration).clamp(0.0, 1.0)
    }

    /// Reset elapsed time.
    pub fn reset(&mut self) {
        self.elapsed = 0.0;
        self.finished = false;
        self.just_finished = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ease;

    #[test]
    fn tween_value_at_endpoints() {
        let t = Tween::new(0.0, 100.0, 1.0, ease::linear);
        assert_eq!(t.value(), 0.0);
        let mut t = t;
        t.tick(1.0);
        assert!((t.value() - 100.0).abs() < 1e-4);
        assert!(t.done());
    }

    #[test]
    fn tween_midpoint_linear() {
        let mut t = Tween::new(0.0, 100.0, 1.0, ease::linear);
        t.tick(0.5);
        assert!((t.value() - 50.0).abs() < 1e-3);
    }

    #[test]
    fn tween_clamps_past_end() {
        let mut t = Tween::new(0.0, 100.0, 1.0, ease::linear);
        t.tick(10.0);
        assert!((t.value() - 100.0).abs() < 1e-4);
    }

    #[test]
    fn tween_reverse() {
        let mut t = Tween::new(0.0, 100.0, 1.0, ease::linear);
        t.tick(1.0);
        t.reverse();
        t.tick(0.5);
        assert!((t.value() - 50.0).abs() < 1e-3);
    }

    #[test]
    fn timer_once_fires_exactly_once() {
        let mut timer = Timer::once(0.5);
        let mut fires = 0;
        for _ in 0..100 {
            if timer.tick(0.02) {
                fires += 1;
            }
        }
        assert_eq!(fires, 1);
        assert!(timer.finished());
    }

    #[test]
    fn timer_every_fires_repeatedly() {
        let mut timer = Timer::every(0.1);
        let mut fires = 0i32;
        for _ in 0..100 {
            if timer.tick(0.02) {
                fires += 1;
            }
        }
        assert!((fires - 20).abs() <= 1);
    }

    #[test]
    fn timer_reset_works() {
        let mut t = Timer::once(0.1);
        t.tick(1.0);
        assert!(t.finished());
        t.reset();
        assert!(!t.finished());
    }
}
