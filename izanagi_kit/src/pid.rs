//! Discrete PID controller on `Fixed`, with output clamping, integral
//! anti-windup (clamped-integration form), and derivative-on-measurement
//! to avoid setpoint kick (Åström–Hägglund, "PID Controllers" — the
//! standard textbook form, integer-only throughout).
//!
//! `u = clamp(kp·e + ki·∫e + kd·(−dm/dt), lo, hi)` where `e = target − m`.
//! Derivative acts on the *measurement* `m`, not the error, so a step
//! change of `target` does not produce a derivative spike.

use crate::fixed::Fixed;
use crate::world_hash::{DetHash, Fnv1a};

/// One PID loop. `Clone + Copy`: checkpoint/restore is trivial and the
/// whole filter is deterministic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Pid {
    /// Proportional gain.
    pub kp: Fixed,
    /// Integral gain (per second of accumulated error).
    pub ki: Fixed,
    /// Derivative gain (multiplies `-dm/dt`).
    pub kd: Fixed,
    /// Integral accumulator `∫e·dt`, clamped each update.
    pub integral: Fixed,
    /// Previous measurement, for derivative-on-measurement.
    pub prev_m: Fixed,
    /// Whether `prev_m` holds a real sample yet.
    primed: bool,
    /// Output clamp, lower bound.
    pub out_min: Fixed,
    /// Output clamp, upper bound.
    pub out_max: Fixed,
}

impl Pid {
    /// Controller with gains and output bounds; `out_min ≤ out_max`.
    /// Swaps them if passed inverted so the clamp is never broken.
    pub fn new(kp: Fixed, ki: Fixed, kd: Fixed, out_min: Fixed, out_max: Fixed) -> Self {
        let (lo, hi) = if out_min <= out_max {
            (out_min, out_max)
        } else {
            (out_max, out_min)
        };
        Self {
            kp,
            ki,
            kd,
            integral: Fixed::ZERO,
            prev_m: Fixed::ZERO,
            primed: false,
            out_min: lo,
            out_max: hi,
        }
    }

    /// Proportional-only shorthand with no clamp.
    pub fn proportional(kp: Fixed) -> Self {
        Self::new(kp, Fixed::ZERO, Fixed::ZERO, Fixed::MIN, Fixed::MAX)
    }

    /// Forget accumulated state (integral, measurement history); gains
    /// and bounds are kept.
    pub fn reset(&mut self) {
        self.integral = Fixed::ZERO;
        self.prev_m = Fixed::ZERO;
        self.primed = false;
    }

    /// Advance the loop by `dt` with a fresh `measurement` against
    /// `target`; returns the clamped control effort. `dt ≤ 0` returns
    /// the proportional-only clamped output without touching state.
    ///
    /// ```
    /// use izanagi_kit::{fixed::Fixed, pid::Pid};
    /// let mut pid = Pid::new(Fixed::ONE, Fixed::ZERO, Fixed::ZERO,
    ///                        Fixed::from_int(-10), Fixed::from_int(10));
    /// // P-only: u = 1·(target − m) = 3.
    /// assert_eq!(pid.update(Fixed::from_int(2), Fixed::from_int(5), Fixed::ONE),
    ///            Fixed::from_int(3));
    /// ```
    pub fn update(&mut self, measurement: Fixed, target: Fixed, dt: Fixed) -> Fixed {
        let e = target - measurement;
        if dt <= Fixed::ZERO {
            return (self.kp.mul(e)).clamp(self.out_min, self.out_max);
        }
        // Integrate then cap ∫e so ki·∫e stays inside the output range
        // — a saturated actuator can't wind the integral to infinity.
        // ki = 0 skips integration entirely (the accumulator would be
        // dead state); ki < 0 accumulates but clamps nothing.
        if self.ki != Fixed::ZERO {
            self.integral = self.integral + e.mul(dt);
        }
        if self.ki > Fixed::ZERO {
            let cap_lo = self.out_min.div(self.ki);
            let cap_hi = self.out_max.div(self.ki);
            self.integral = self.integral.clamp(cap_lo, cap_hi);
        }
        // Derivative on measurement; kick-free on the first sample and
        // on setpoint steps.
        let d_term = if self.primed {
            -self.kd.mul((measurement - self.prev_m).div(dt))
        } else {
            Fixed::ZERO
        };
        self.prev_m = measurement;
        self.primed = true;
        let u = self.kp.mul(e) + self.ki.mul(self.integral) + d_term;
        u.clamp(self.out_min, self.out_max)
    }
}

impl DetHash for Pid {
    fn det_hash(&self, h: &mut Fnv1a) {
        self.kp.det_hash(h);
        self.ki.det_hash(h);
        self.kd.det_hash(h);
        self.integral.det_hash(h);
        self.prev_m.det_hash(h);
        self.primed.det_hash(h);
        self.out_min.det_hash(h);
        self.out_max.det_hash(h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(v: i32) -> Fixed {
        Fixed::from_int(v)
    }
    fn r(n: i32, d: i32) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    #[test]
    fn proportional_shorthand_is_unclamped_p_only() {
        let mut p = Pid::proportional(f(2));
        assert_eq!(p.update(f(0), f(10), Fixed::ONE), f(20));
        assert_eq!(p.integral, Fixed::ZERO);
    }

    #[test]
    fn proportional_hits_and_clamps() {
        let mut p = Pid::new(f(2), Fixed::ZERO, Fixed::ZERO, f(-5), f(5));
        // e = 2 → u = 4.
        assert_eq!(p.update(f(1), f(3), Fixed::ONE), f(4));
        // e = 5 → u = 10 clamped to 5.
        assert_eq!(p.update(f(0), f(5), Fixed::ONE), f(5));
        // e = −7 → u = −14 clamped to −5.
        assert_eq!(p.update(f(9), f(2), Fixed::ONE), f(-5));
    }

    #[test]
    fn integral_winds_then_unwinds() {
        // ki = 1, bounds ±4: sustained error 2·dt accumulates.
        let mut p = Pid::new(Fixed::ZERO, Fixed::ONE, Fixed::ZERO, f(-4), f(4));
        for _ in 0..10 {
            p.update(f(0), f(2), Fixed::ONE); // e=2 → integral hits cap 4
        }
        // Anti-windup: integral pinned at +4, output at +4.
        assert_eq!(p.update(f(0), f(2), Fixed::ONE), f(4));
        // Reverse the error; integral unwinds by 2 per step, not from ∞.
        assert_eq!(p.update(f(4), f(2), Fixed::ONE), f(2)); // 4 − 2·1 = 2
        assert_eq!(p.update(f(4), f(2), Fixed::ONE), Fixed::ZERO);
    }

    #[test]
    fn derivative_on_measurement_avoids_setpoint_kick() {
        // kd only: at constant measurement a target step gives u = 0
        // (no derivative spike), vs. derivative-on-error which would
        // spike to ±∞ for a step.
        let mut p = Pid::new(Fixed::ZERO, Fixed::ZERO, Fixed::ONE, f(-9), f(9));
        assert_eq!(p.update(f(0), f(0), Fixed::ONE), Fixed::ZERO);
        assert_eq!(p.update(f(0), f(7), Fixed::ONE), Fixed::ZERO);
        // Measurement moves +2/s → u = kd·(−dm/dt) = −2.
        assert_eq!(p.update(f(2), f(7), Fixed::ONE), f(-2));
    }

    #[test]
    fn combined_terms_add_up() {
        let mut p = Pid::new(f(1), r(1, 2), r(1, 4), f(-100), f(100));
        // Step 1: m=0→1? Use m=0, T=4, dt=2: e=4, P=4.
        let u1 = p.update(f(0), f(4), f(2));
        assert_eq!(u1, f(4) + f(4)); // P=4·1, I=0.5·(4·2)=4, D=0
                                     // Step 2: m=2, e=2, dt=2: P=2, I=0.5·(8+4)=6, D=0.25·(−1)=−0.25.
        let u2 = p.update(f(2), f(4), f(2));
        assert_eq!(u2, f(2) + f(6) - r(1, 4));
    }

    #[test]
    fn zero_dt_returns_p_only() {
        let mut p = Pid::new(f(2), f(9), f(9), f(-8), f(8));
        assert_eq!(p.update(f(1), f(4), Fixed::ZERO), f(6));
        // State untouched: integral stays 0.
        assert_eq!(p.integral, Fixed::ZERO);
    }

    #[test]
    fn reset_clears_history_not_gains() {
        let mut p = Pid::new(f(1), f(1), f(1), f(-9), f(9));
        p.update(f(0), f(5), Fixed::ONE);
        p.reset();
        assert_eq!(p.integral, Fixed::ZERO);
        // After reset the derivative sees no prior sample → no kick.
        assert_eq!(p.update(f(9), f(0), Fixed::ONE), f(-9));
    }

    #[test]
    fn inverted_bounds_are_swapped() {
        let mut p = Pid::new(f(1), Fixed::ZERO, Fixed::ZERO, f(5), f(-5));
        assert!(p.out_min <= p.out_max);
        assert_eq!(p.update(f(0), f(7), Fixed::ONE), f(5));
    }
}
