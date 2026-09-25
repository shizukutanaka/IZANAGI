//! PCHIP — Fritsch–Carlson monotone cubic Hermite interpolation.
//! Unlike a natural cubic spline (which overshoots), PCHIP picks each
//! knot tangent as a weighted harmonic mean of the adjacent secant
//! slopes and zeroes it wherever the sign flips, so the interpolant
//! stays monotone wherever the data is. That's the property you want
//! for easing curves, envelopes, and lookup densification where a
//! wiggle is a bug, not a feature.
//!
//! Evaluation is the standard cubic Hermite basis on the containing
//! interval; outside the data range the end values are returned
//! (flat extrapolation — linear extrapolation would break monotonicity
//! on the outside).
//!
//! `Fixed` note: the Hermite basis needs `t³`, so keep `x` spans under
//! ~200 units to leave headroom.
//!
//! ```
//! use izanagi_kit::fixed::Fixed;
//! use izanagi_kit::pchip::Pchip;
//!
//! let xs: Vec<Fixed> = (0..4).map(Fixed::from_int).collect();
//! let ys = vec![
//!     Fixed::ZERO,
//!     Fixed::ONE,
//!     Fixed::ONE, // flat — must stay flat
//!     Fixed::from_int(2),
//! ];
//! let p = Pchip::new(&xs, &ys).unwrap();
//! // Knots are interpolated exactly; the flat segment stays flat.
//! assert_eq!(p.eval(Fixed::ONE), Fixed::ONE);
//! let mid = p.eval(Fixed::from_ratio(3, 2)); // x = 1.5, inside flat zone
//! assert!(mid >= Fixed::ONE && mid <= Fixed::from_int(2));
//! ```

use crate::fixed::Fixed;

/// Monotone cubic Hermite interpolant over strictly increasing `xs`.
#[derive(Clone, Debug)]
pub struct Pchip {
    xs: Vec<Fixed>,
    ys: Vec<Fixed>,
    /// Knot tangents `d_i` (secant-slope units).
    ds: Vec<Fixed>,
}

/// `max(sign,0)`-style helper: zero when `a` and `b` differ in sign.
fn same_sign(a: Fixed, b: Fixed) -> bool {
    (a > Fixed::ZERO && b > Fixed::ZERO) || (a < Fixed::ZERO && b < Fixed::ZERO)
}

impl Pchip {
    /// Build from `n ≥ 2` knots; `xs` must be strictly increasing.
    /// Returns `None` otherwise.
    pub fn new(xs: &[Fixed], ys: &[Fixed]) -> Option<Self> {
        let n = xs.len();
        if n < 2 || ys.len() != n {
            return None;
        }
        let mut h = Vec::with_capacity(n - 1);
        let mut delta = Vec::with_capacity(n - 1);
        for i in 0..n - 1 {
            let hi = xs[i + 1] - xs[i];
            if hi <= Fixed::ZERO {
                return None;
            }
            delta.push((ys[i + 1] - ys[i]).div(hi));
            h.push(hi);
        }
        let mut d = vec![Fixed::ZERO; n];
        // Interior: weighted harmonic mean, or 0 at a sign flip.
        for i in 1..n - 1 {
            let (d_prev, d_next) = (delta[i - 1], delta[i]);
            if !same_sign(d_prev, d_next) {
                d[i] = Fixed::ZERO;
            } else {
                let w1 = h[i].mul(Fixed::from_int(2)) + h[i - 1];
                let w2 = h[i] + h[i - 1].mul(Fixed::from_int(2));
                // (w1+w2) / (w1/δ_{i-1} + w2/δ_i)
                d[i] = (w1 + w2).div(w1.div(d_prev) + w2.div(d_next));
            }
        }
        // Endpoints: one-sided three-point estimate, then the
        // Fritsch–Carlson clamps (sign, then 3×-secant magnitude cap).
        d[0] = Self::edge(
            h[0],
            h.get(1).copied().unwrap_or(h[0]),
            delta[0],
            delta.get(1).copied().unwrap_or(delta[0]),
        );
        d[n - 1] = Self::edge(
            h[n - 2],
            h.get(n - 3).copied().unwrap_or(h[n - 2]),
            delta[n - 2],
            delta.get(n - 3).copied().unwrap_or(delta[n - 2]),
        );
        Some(Self {
            xs: xs.to_vec(),
            ys: ys.to_vec(),
            ds: d,
        })
    }

    /// `d = ((2h₀+h₁)·δ₀ − h₀·δ₁) / (h₀+h₁)` with FC clamping.
    fn edge(h0: Fixed, h1: Fixed, d0: Fixed, d1: Fixed) -> Fixed {
        let mut d = ((h0.mul(Fixed::from_int(2)) + h1).mul(d0) - h0.mul(d1)).div(h0 + h1);
        if !same_sign(d, d0) {
            d = Fixed::ZERO;
        } else if !same_sign(d0, d1) && d.abs() > d0.abs().mul(Fixed::from_int(3)) {
            d = d0.mul(Fixed::from_int(3));
        }
        d
    }

    /// Interpolate at `x`; flat extrapolation outside the knots.
    pub fn eval(&self, x: Fixed) -> Fixed {
        let n = self.xs.len();
        if x <= self.xs[0] {
            return self.ys[0];
        }
        if x >= self.xs[n - 1] {
            return self.ys[n - 1];
        }
        // Binary search for the containing interval.
        let (mut lo, mut hi) = (0usize, n - 1);
        while hi - lo > 1 {
            let mid = lo + (hi - lo) / 2;
            if self.xs[mid] <= x {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let h = self.xs[hi] - self.xs[lo];
        let t = (x - self.xs[lo]).div(h);
        let t2 = t.mul(t);
        let t3 = t2.mul(t);
        let h00 = t3.mul(Fixed::from_int(2)) - t2.mul(Fixed::from_int(3)) + Fixed::ONE;
        let h10 = t3 - t2.mul(Fixed::from_int(2)) + t;
        let h01 = Fixed::ZERO - t3.mul(Fixed::from_int(2)) + t2.mul(Fixed::from_int(3));
        let h11 = t3 - t2;
        h00.mul(self.ys[lo])
            + h10.mul(h.mul(self.ds[lo]))
            + h01.mul(self.ys[hi])
            + h11.mul(h.mul(self.ds[hi]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(x: i32) -> Fixed {
        Fixed::from_int(x)
    }
    fn to_f64(x: Fixed) -> f64 {
        x.raw() as f64 / 65536.0
    }

    /// f64 oracle PCHIP (same FC construction, float eval).
    struct F64Pchip {
        xs: Vec<f64>,
        ys: Vec<f64>,
        ds: Vec<f64>,
    }
    impl F64Pchip {
        fn new(xs: &[f64], ys: &[f64]) -> Self {
            let n = xs.len();
            let h: Vec<f64> = (0..n - 1).map(|i| xs[i + 1] - xs[i]).collect();
            let delta: Vec<f64> = (0..n - 1).map(|i| (ys[i + 1] - ys[i]) / h[i]).collect();
            let mut d = vec![0.0; n];
            for i in 1..n - 1 {
                if delta[i - 1] * delta[i] <= 0.0 {
                    d[i] = 0.0;
                } else {
                    let w1 = 2.0 * h[i] + h[i - 1];
                    let w2 = h[i] + 2.0 * h[i - 1];
                    d[i] = (w1 + w2) / (w1 / delta[i - 1] + w2 / delta[i]);
                }
            }
            let edge = |h0: f64, h1: f64, d0: f64, d1: f64| {
                let mut d = ((2.0 * h0 + h1) * d0 - h0 * d1) / (h0 + h1);
                if d * d0 <= 0.0 {
                    d = 0.0;
                } else if d0 * d1 < 0.0 && d.abs() > 3.0 * d0.abs() {
                    d = 3.0 * d0;
                }
                d
            };
            d[0] = edge(
                h[0],
                h.get(1).copied().unwrap_or(h[0]),
                delta[0],
                delta.get(1).copied().unwrap_or(delta[0]),
            );
            d[n - 1] = edge(
                h[n - 2],
                h.get(n - 3).copied().unwrap_or(h[n - 2]),
                delta[n - 2],
                delta.get(n - 3).copied().unwrap_or(delta[n - 2]),
            );
            Self {
                xs: xs.to_vec(),
                ys: ys.to_vec(),
                ds: d,
            }
        }
        fn eval(&self, x: f64) -> f64 {
            let n = self.xs.len();
            if x <= self.xs[0] {
                return self.ys[0];
            }
            if x >= self.xs[n - 1] {
                return self.ys[n - 1];
            }
            let i = (0..n - 1).find(|&i| x < self.xs[i + 1]).unwrap_or(n - 2);
            let h = self.xs[i + 1] - self.xs[i];
            let t = (x - self.xs[i]) / h;
            let (t2, t3) = (t * t, t * t * t);
            (2.0 * t3 - 3.0 * t2 + 1.0) * self.ys[i]
                + (t3 - 2.0 * t2 + t) * h * self.ds[i]
                + (-2.0 * t3 + 3.0 * t2) * self.ys[i + 1]
                + (t3 - t2) * h * self.ds[i + 1]
        }
    }

    #[test]
    fn hits_knots_and_stays_monotone() {
        let xs: Vec<Fixed> = (0..6).map(f).collect();
        let ys: Vec<Fixed> = [0, 1, 1, 2, 2, 3].iter().map(|&v| f(v)).collect();
        let p = Pchip::new(&xs, &ys).unwrap();
        for i in 0..6 {
            assert_eq!(p.eval(xs[i]), ys[i]);
        }
        // Dense sample: non-decreasing everywhere, flat over the flats.
        let mut prev = Fixed::MIN;
        for step in 0..=250 {
            let x = Fixed::from_ratio(5 * step, 250);
            let y = p.eval(x);
            assert!(y >= prev, "x={x:?}: {y:?} < {prev:?}");
            prev = y;
        }
        // Flat segment [1,2] stays within a small tolerance of 1
        // (interior tangents are 0 there but endpoints pull slightly).
        for step in 45..=85 {
            let x = Fixed::from_ratio(step, 50); // 0.9..1.7
            let y = to_f64(p.eval(x));
            assert!((y - 1.0).abs() < 0.15, "x={step}: {y}");
        }
    }

    #[test]
    fn matches_f64_oracle() {
        let xf = [0.0, 1.0, 2.5, 4.0, 7.0];
        let yf = [0.0, 3.0, 1.0, 4.0, 4.5];
        let xs: Vec<Fixed> = xf
            .iter()
            .map(|&v| Fixed::from_ratio((v * 2.0) as i32, 2))
            .collect();
        let ys: Vec<Fixed> = yf
            .iter()
            .map(|&v| Fixed::from_ratio((v * 2.0) as i32, 2))
            .collect();
        let (p, o) = (Pchip::new(&xs, &ys).unwrap(), F64Pchip::new(&xf, &yf));
        for step in 0..=140 {
            let x = 7.0 * step as f64 / 140.0;
            let got = to_f64(p.eval(Fixed::from_ratio((x * 20.0) as i32, 20)));
            let want = o.eval(x);
            assert!((got - want).abs() < 0.03, "x={x}: {got} vs {want}");
        }
    }

    #[test]
    fn no_overshoot_on_step_data() {
        // A step: spline would overshoot; pchip caps at the step ends.
        let xs: Vec<Fixed> = (0..4).map(f).collect();
        let ys: Vec<Fixed> = [0, 0, 2, 2].iter().map(|&v| f(v)).collect();
        let p = Pchip::new(&xs, &ys).unwrap();
        for step in 0..=200 {
            let x = Fixed::from_ratio(3 * step, 200);
            let y = p.eval(x);
            assert!(y >= Fixed::ZERO && y <= f(2), "x={x:?}: {y:?}");
        }
    }

    #[test]
    fn validation_and_extrapolation() {
        assert!(Pchip::new(&[f(0)], &[f(0)]).is_none());
        assert!(Pchip::new(&[f(0), f(0)], &[f(0), f(1)]).is_none());
        assert!(Pchip::new(&[f(0), f(1)], &[f(0)]).is_none());
        let p = Pchip::new(&[f(0), f(1), f(2)], &[f(0), f(1), f(4)]).unwrap();
        assert_eq!(p.eval(f(-5)), f(0));
        assert_eq!(p.eval(f(9)), f(4));
    }

    #[test]
    fn deterministic_twice() {
        let xs: Vec<Fixed> = (0..5).map(f).collect();
        let ys: Vec<Fixed> = [0, 2, 1, 3, 3].iter().map(|&v| f(v)).collect();
        let a = Pchip::new(&xs, &ys).unwrap();
        for step in 0..=80 {
            let x = Fixed::from_ratio(4 * step, 80);
            assert_eq!(a.eval(x), a.eval(x));
        }
    }
}
