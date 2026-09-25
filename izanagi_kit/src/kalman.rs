//! Scalar Kalman filter on `Fixed` (Welch–Bishop formulation) — the
//! optimal linear estimator for a one-dimensional state under Gaussian
//! noise, exact integer arithmetic throughout.
//!
//! Model: state `x` with error covariance `p` (variance, not stddev).
//! Predict: `p += q` (process noise). Update with measurement `z` of
//! noise variance `r`: `K = p/(p+r)`, `x += K(z−x)`, `p *= (1−K)`.

use crate::fixed::Fixed;
use crate::world_hash::{DetHash, Fnv1a};

/// Scalar filter state: the estimate and its variance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Kalman {
    /// Current best estimate of the state.
    pub x: Fixed,
    /// Error covariance `P` — how wrong `x` might be, in units².
    pub p: Fixed,
}

impl Kalman {
    /// Fresh filter: initial estimate `x0` with covariance `p0`.
    /// `p0 < 0` is clamped to zero (a variance cannot be negative);
    /// pass a *large* `p0` when the initial guess is blind.
    ///
    /// ```
    /// use izanagi_kit::{fixed::Fixed, kalman::Kalman};
    /// let mut k = Kalman::new(Fixed::ZERO, Fixed::ONE);
    /// let est = k.update(Fixed::from_int(10), Fixed::from_int(3));
    /// // K = 1/(1+3) = 1/4 → est = 0 + (10−0)/4 = 2.5
    /// assert_eq!(est, Fixed::from_ratio(5, 2));
    /// ```
    pub fn new(x0: Fixed, p0: Fixed) -> Self {
        Self {
            x: x0,
            p: p0.max(Fixed::ZERO),
        }
    }

    /// Time update with no known motion: `P += q`. Negative `q` clamps
    /// to zero — process noise is a variance.
    pub fn predict(&mut self, q: Fixed) {
        self.p = self.p + q.max(Fixed::ZERO);
    }

    /// Time update with a known deterministic move `dx` (a "control
    /// input"): `x += dx`, `P += q`.
    pub fn predict_with(&mut self, dx: Fixed, q: Fixed) {
        self.x = self.x + dx;
        self.predict(q);
    }

    /// Measurement update: fold `z` (variance `r`) into the estimate and
    /// return it. `r ≤ 0` means a noise-free sensor — the measurement is
    /// trusted completely (`x := z`, `P := 0`).
    pub fn update(&mut self, z: Fixed, r: Fixed) -> Fixed {
        if r <= Fixed::ZERO {
            self.x = z;
            self.p = Fixed::ZERO;
            return self.x;
        }
        let denom = self.p + r;
        // K = P/(P+R); denom > 0 since r > 0 and P ≥ 0.
        let k = self.p.div(denom);
        self.x = self.x + k.mul(z - self.x);
        self.p = (Fixed::ONE - k).mul(self.p);
        self.x
    }
}

impl DetHash for Kalman {
    fn det_hash(&self, h: &mut Fnv1a) {
        self.x.det_hash(h);
        self.p.det_hash(h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(v: i32) -> Fixed {
        Fixed::from_int(v)
    }

    #[test]
    fn matches_scalar_kalman_equations() {
        // Oracle: same filter in f64, compare each step.
        let (mut fx, mut fp) = (0.0f64, 4.0f64);
        let mut g = Kalman::new(Fixed::ZERO, f(4));
        for &(z, q, r) in &[
            (10.0, 1.0, 2.0),
            (11.0, 1.0, 2.0),
            (9.0, 0.5, 1.0),
            (12.0, 0.5, 4.0),
        ] {
            // predict
            fp += q;
            g.predict(Fixed::from_ratio((q * 2.0) as i32, 2));
            // update
            let k = fp / (fp + r);
            fx += k * (z - fx);
            fp *= 1.0 - k;
            let est = g.update(
                Fixed::from_ratio((z * 8.0) as i32, 8),
                Fixed::from_ratio((r * 8.0) as i32, 8),
            );
            let want = Fixed::from_ratio((fx * 64.0) as i32, 64);
            assert!(
                (est - want).abs() < Fixed::from_ratio(1, 8),
                "{est:?} vs {want:?}"
            );
        }
    }

    #[test]
    fn converges_onto_a_noisy_signal() {
        // Constant true value 50, noisy ±8 measurements, q small, r=8:
        // the estimate should settle inside ±2 and P should shrink.
        let mut rng = crate::rng::SplitMix64::new(0xF11);
        let mut k = Kalman::new(Fixed::ZERO, f(100));
        for _ in 0..200 {
            k.predict(Fixed::from_ratio(1, 100));
            let z = f(50) + Fixed::from_int(rng.range(-8, 8));
            k.update(z, f(8));
        }
        assert!((k.x - f(50)).abs() < f(2), "x={:?}", k.x.raw());
        assert!(k.p < Fixed::ONE, "p={:?}", k.p.raw());
    }

    #[test]
    fn perfect_sensor_overrides_estimate() {
        let mut k = Kalman::new(f(-7), f(3));
        assert_eq!(k.update(f(42), Fixed::ZERO), f(42));
        assert_eq!(k.p, Fixed::ZERO);
    }

    #[test]
    fn negative_covariances_are_clamped() {
        let mut k = Kalman::new(Fixed::ZERO, f(-5));
        assert_eq!(k.p, Fixed::ZERO);
        k.predict(f(-1));
        assert_eq!(k.p, Fixed::ZERO);
    }

    #[test]
    fn known_motion_tracks() {
        // Moving +3/step with q=0.5 and exact-ish sensor r=1.
        let mut k = Kalman::new(Fixed::ZERO, f(10));
        for i in 1..=50 {
            k.predict_with(f(3), Fixed::from_ratio(1, 2));
            let z = f(3 * i);
            k.update(z, Fixed::ONE);
        }
        assert!((k.x - f(150)).abs() < Fixed::from_ratio(1, 10));
    }

    #[test]
    fn deterministic_twice() {
        let mut a = Kalman::new(f(1), f(1));
        let mut b = Kalman::new(f(1), f(1));
        for z in 0..40 {
            a.predict(Fixed::from_ratio(1, 4));
            b.predict(Fixed::from_ratio(1, 4));
            let ea = a.update(f(7) + f(z % 5), f(2));
            let eb = b.update(f(7) + f(z % 5), f(2));
            assert_eq!(ea, eb);
            assert_eq!(a, b);
        }
    }
}
