//! Critically damped spring — the "SmoothDamp" behavior — via the
//! *exact* closed-form solution of `ẍ = −ω²(x−T) − 2ωẋ` evaluated in
//! `Fixed` (GPG4/Eiserloh is the standard reference; we use the true
//! exponential rather than Unity's 1/(1+x+0.48x²+0.235x³) approximation,
//! since `Fixed::exp` is available).
//!
//! With `Δ = x₀ − T`: `x(t) = (Δ + (v₀+ωΔ)·t)·e^(−ωt)`,
//! `v(t) = (v₀ − ω(v₀+ωΔ)·t)·e^(−ωt)`. No overshoot for `ω > 0`,
//! `dt ≥ 0`; the response is frame-rate independent — two `dt` half
//! steps land on the same `x` as one `dt` full step (up to `Fixed`
//! rounding of `e^(−ωt)` vs `e^(−ωt)²`).

use crate::fixed::Fixed;
use crate::world_hash::{DetHash, Fnv1a};

/// One exact critically damped step: returns `(x', v')` after `dt`
/// seconds toward `target` at angular frequency `omega`. `omega ≤ 0`
/// or `dt ≤ 0` returns the inputs unchanged (no motion to compute).
///
/// ```
/// use izanagi_kit::{fixed::Fixed, spring::step};
/// let (x, _v) = step(Fixed::from_int(10), Fixed::ZERO, Fixed::ZERO,
///                   Fixed::from_int(4), Fixed::ONE);
/// // Heavily decayed toward 0 but not overshot.
/// assert!(x > Fixed::ZERO && x < Fixed::from_int(3));
/// ```
pub fn step(x: Fixed, v: Fixed, target: Fixed, omega: Fixed, dt: Fixed) -> (Fixed, Fixed) {
    if omega <= Fixed::ZERO || dt <= Fixed::ZERO {
        return (x, v);
    }
    let delta = x - target;
    let b = v + omega.mul(delta); // (v₀ + ωΔ)
    let e = Fixed::exp(Fixed::ZERO - omega.mul(dt));
    let x2 = target + (delta + b.mul(dt)).mul(e);
    let v2 = (v - omega.mul(b).mul(dt)).mul(e);
    (x2, v2)
}

/// Stateful driver around [`step`]: holds position, velocity and target
/// so callers can re-aim mid-flight without discontinuities (the whole
/// point of SmoothDamp-style springs — retargeting is smooth because
/// `v` carries over).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Spring {
    /// Current position.
    pub x: Fixed,
    /// Current velocity.
    pub v: Fixed,
    /// Position the spring relaxes toward.
    pub target: Fixed,
    /// Angular frequency `ω`: larger is stiffer/faster. `ω = 2/t` gives
    /// roughly Unity's `SmoothDamp(x, v, T, t)` feel.
    pub omega: Fixed,
}

impl Spring {
    /// New spring at rest on `x`, tracking `x` itself as the target.
    pub fn new(x: Fixed, omega: Fixed) -> Self {
        Self {
            x,
            v: Fixed::ZERO,
            target: x,
            omega,
        }
    }
    /// Re-aim at a new target; position and velocity are untouched.
    pub fn set_target(&mut self, target: Fixed) {
        self.target = target;
    }
    /// Advance `dt` seconds.
    pub fn advance(&mut self, dt: Fixed) {
        let (x, v) = step(self.x, self.v, self.target, self.omega, dt);
        self.x = x;
        self.v = v;
    }
}

impl DetHash for Spring {
    fn det_hash(&self, h: &mut Fnv1a) {
        self.x.det_hash(h);
        self.v.det_hash(h);
        self.target.det_hash(h);
        self.omega.det_hash(h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(v: i32) -> Fixed {
        Fixed::from_int(v)
    }

    #[test]
    fn converges_without_overshoot_from_rest() {
        let mut s = Spring::new(f(0), f(4));
        s.set_target(f(10));
        let mut prev = Fixed::ZERO;
        for _ in 0..600 {
            s.advance(Fixed::from_ratio(1, 60));
            // From rest below the target, a critically damped spring
            // never crosses: x is monotonically non-decreasing ≤ target.
            assert!(s.x >= prev && s.x <= f(10));
            prev = s.x;
        }
        assert!(s.x > Fixed::from_ratio(199, 20), "x={:?}", s.x.raw());
        assert!(s.v.abs() < Fixed::from_ratio(1, 100));
    }

    #[test]
    fn matches_f64_closed_form() {
        // Oracle: same closed form in f64; compare within Fixed resolution.
        for &(x0, v0, t, om, dt) in &[
            (3.0, 0.0, 0.0, 2.0, 0.25),
            (-4.0, 2.0, 5.0, 3.5, 0.1),
            (0.0, -7.0, -2.0, 1.25, 0.7),
        ] {
            let d = x0 - t;
            let b = v0 + om * d;
            let e = f64::exp(-om * dt);
            let wx = t + (d + b * dt) * e;
            let wv = (v0 - om * b * dt) * e;
            let (gx, gv) = step(
                Fixed::from_ratio((x0 * 1000.0) as i32, 1000),
                Fixed::from_ratio((v0 * 1000.0) as i32, 1000),
                Fixed::from_ratio((t * 1000.0) as i32, 1000),
                Fixed::from_ratio((om * 1000.0) as i32, 1000),
                Fixed::from_ratio((dt * 1000.0) as i32, 1000),
            );
            let ex = Fixed::from_ratio((wx * 1000.0) as i32, 1000);
            let ev = Fixed::from_ratio((wv * 1000.0) as i32, 1000);
            assert!(
                (gx - ex).abs() < Fixed::from_ratio(1, 20),
                "{gx:?} vs {ex:?}"
            );
            assert!(
                (gv - ev).abs() < Fixed::from_ratio(1, 10),
                "{gv:?} vs {ev:?}"
            );
        }
    }

    #[test]
    fn frame_rate_independence_approximately() {
        // One 1s step vs ten 0.1s steps: differ only by exp rounding noise.
        let (x1, v1) = step(f(0), Fixed::ZERO, f(8), f(3), Fixed::ONE);
        let mut s = Spring::new(f(0), f(3));
        s.set_target(f(8));
        for _ in 0..10 {
            s.advance(Fixed::from_ratio(1, 10));
        }
        assert!((s.x - x1).abs() < Fixed::from_ratio(1, 10));
        assert!((s.v - v1).abs() < Fixed::from_ratio(1, 5));
    }

    #[test]
    fn zero_and_negative_dt_are_noop() {
        assert_eq!(step(f(3), f(1), f(0), f(4), Fixed::ZERO), (f(3), f(1)));
        assert_eq!(
            step(f(3), f(1), f(0), Fixed::ZERO, Fixed::ONE),
            (f(3), f(1))
        );
    }

    #[test]
    fn deterministic_twice() {
        let a = step(f(7), f(-2), f(1), f(2), Fixed::from_ratio(3, 10));
        let b = step(f(7), f(-2), f(1), f(2), Fixed::from_ratio(3, 10));
        assert_eq!(a, b);
    }
}
