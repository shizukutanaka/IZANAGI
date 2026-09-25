//! Dual numbers — forward-mode automatic differentiation in a
//! two-tuple `(v, d)`: value and derivative carried together
//! through every operation, so `f'(x)` falls out of evaluating
//! `f` once on a `Dual` seeded with `d = 1`. No tapes, no graphs
//! — just the chain rule applied mechanically:
//! `(u, u')·(v, v') = (u·v, u·v' + u'·v)`.
//!
//! This is the analytic derivative primitive the kit's numerical
//! tools (`roots`, `pid`, `kalman`) can lean on when a hand-coded
//! Jacobian isn't worth it. All arithmetic is `Fixed`, so a given
//! function evaluates its derivative bit-exactly.
//!
//! ```
//! use izanagi_kit::dual::Dual;
//! use izanagi_kit::fixed::Fixed;
//!
//! // f(x) = x² + 3x  ⇒  f'(2) = 7
//! let x = Dual::var(Fixed::from_int(2));
//! let f = x.mul(x) + x * Fixed::from_int(3);
//! assert_eq!(f.deriv(), Fixed::from_int(7));
//! ```

use crate::fixed::Fixed;

/// A `Fixed` value paired with its derivative.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dual {
    v: Fixed,
    d: Fixed,
}

impl Dual {
    /// Constant (derivative 0).
    pub fn new(v: Fixed) -> Self {
        Dual { v, d: Fixed::ZERO }
    }

    /// Independent variable seeded with derivative 1.
    pub fn var(v: Fixed) -> Self {
        Dual { v, d: Fixed::ONE }
    }

    /// The value part.
    pub fn value(self) -> Fixed {
        self.v
    }

    /// The derivative part.
    pub fn deriv(self) -> Fixed {
        self.d
    }

    /// `u · v` — product rule.
    #[allow(clippy::should_implement_trait)]
    pub fn mul(self, rhs: Dual) -> Dual {
        Dual {
            v: self.v.mul(rhs.v),
            d: self.v.mul(rhs.d) + self.d.mul(rhs.v),
        }
    }

    /// `u / v` — quotient rule; `v = 0` saturates via `Fixed::div`.
    #[allow(clippy::should_implement_trait)]
    pub fn div(self, rhs: Dual) -> Dual {
        Dual {
            v: self.v.div(rhs.v),
            d: (self.d.mul(rhs.v) - self.v.mul(rhs.d)).div(rhs.v.mul(rhs.v)),
        }
    }

    /// `u^n` for integer `n` (negative allowed via `div`).
    pub fn powi(self, n: i32) -> Dual {
        if n == 0 {
            return Dual::new(Fixed::ONE);
        }
        let mut acc = Dual::new(Fixed::ONE);
        let base = if n < 0 {
            Dual::new(Fixed::ONE).div(self)
        } else {
            self
        };
        for _ in 0..n.unsigned_abs() {
            acc = acc.mul(base);
        }
        acc
    }

    /// `e^u` — `(eᵘ, eᵘ·u')`.
    pub fn exp(self) -> Dual {
        let e = self.v.exp();
        Dual {
            v: e,
            d: e.mul(self.d),
        }
    }

    /// `ln u` — `(ln u, u'/u)`; `u ≤ 0` saturates via `Fixed::ln`.
    pub fn ln(self) -> Dual {
        Dual {
            v: self.v.ln(),
            d: self.d.div(self.v),
        }
    }

    /// `√u` — `(√u, u'/(2√u))`.
    pub fn sqrt(self) -> Dual {
        let r = self.v.sqrt();
        Dual {
            v: r,
            d: if r.is_zero() {
                Fixed::ZERO
            } else {
                self.d.div(r.mul(Fixed::from_int(2)))
            },
        }
    }

    /// `sin u` — `(sin u, u'·cos u)` via `Fixed::sin_cos`.
    pub fn sin(self) -> Dual {
        let (s, c) = self.v.sin_cos();
        Dual {
            v: s,
            d: c.mul(self.d),
        }
    }

    /// `cos u` — `(cos u, −u'·sin u)`.
    pub fn cos(self) -> Dual {
        let (s, c) = self.v.sin_cos();
        Dual {
            v: c,
            d: -s.mul(self.d),
        }
    }
}

impl core::ops::Add for Dual {
    type Output = Dual;
    fn add(self, rhs: Dual) -> Dual {
        Dual {
            v: self.v + rhs.v,
            d: self.d + rhs.d,
        }
    }
}

impl core::ops::Sub for Dual {
    type Output = Dual;
    fn sub(self, rhs: Dual) -> Dual {
        Dual {
            v: self.v - rhs.v,
            d: self.d - rhs.d,
        }
    }
}

impl core::ops::Neg for Dual {
    type Output = Dual;
    fn neg(self) -> Dual {
        Dual {
            v: -self.v,
            d: -self.d,
        }
    }
}

/// `Dual · scalar` — scale both parts.
impl core::ops::Mul<Fixed> for Dual {
    type Output = Dual;
    fn mul(self, rhs: Fixed) -> Dual {
        Dual {
            v: self.v.mul(rhs),
            d: self.d.mul(rhs),
        }
    }
}

/// `scalar · Dual` — mirror of `Dual · scalar`.
impl core::ops::Mul<Dual> for Fixed {
    type Output = Dual;
    fn mul(self, rhs: Dual) -> Dual {
        rhs * self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(x: i32) -> Dual {
        Dual::var(Fixed::from_int(x))
    }

    #[test]
    fn polynomial_rules() {
        // x² + 3x at 2 → value 10, deriv 7.
        let x = d(2);
        let f = x.mul(x) + x * Fixed::from_int(3);
        assert_eq!(f.value(), Fixed::from_int(10));
        assert_eq!(f.deriv(), Fixed::from_int(7));
        // Quotient: f = x/(x+1) at 1 → v=1/2, f'=1/4.
        let f = d(1).div(d(1) + Dual::new(Fixed::ONE));
        assert_eq!(f.value(), Fixed::from_ratio(1, 2));
        assert_eq!(f.deriv(), Fixed::from_ratio(1, 4));
        // Negative exponent: x⁻¹ at 2 → v=1/2, d=−1/4.
        let f = d(2).powi(-1);
        assert_eq!(f.deriv(), -Fixed::from_ratio(1, 4));
    }

    #[test]
    fn transcendental_rules() {
        let eps = Fixed::from_ratio(1, 50);
        // exp(x) is its own derivative.
        let f = Dual::var(Fixed::ONE).exp();
        assert!(f.deriv() >= f.value() - eps && f.deriv() <= f.value() + eps);
        // ln(x): d = 1/x at x=2.
        let f = Dual::var(Fixed::from_int(2)).ln();
        assert!(f.deriv() >= Fixed::from_ratio(1, 2) - eps);
        assert!(f.deriv() <= Fixed::from_ratio(1, 2) + eps);
        // sin(0)' = cos(0) = 1; cos(0)' = −sin(0) ≈ 0 (CORDIC epsilon).
        assert_eq!(Dual::var(Fixed::ZERO).sin().deriv(), Fixed::ONE);
        let cd = Dual::var(Fixed::ZERO).cos().deriv();
        assert!(cd.abs() < eps, "cos' at 0 = {cd:?}");
        // sqrt(x²) at 3 → |x| = 3, d = 1.
        let f = d(3).powi(2).sqrt();
        assert_eq!(f.value(), Fixed::from_int(3));
    }

    #[test]
    fn chain_rule_composition() {
        // f = sin(2x): f'(0) = 2.
        let f = (d(0) * Fixed::from_int(2)).sin();
        assert_eq!(f.deriv(), Fixed::from_int(2));
        // f = exp(−x²/2) at 0 → 1, d = 0.
        let x = d(0).powi(2) * -Fixed::from_ratio(1, 2);
        let f = x.exp();
        assert_eq!(f.value(), Fixed::ONE);
        assert_eq!(f.deriv(), Fixed::ZERO);
    }

    #[test]
    fn constants_have_zero_deriv() {
        let f = Dual::new(Fixed::from_int(9)).mul(d(3));
        assert_eq!(f.deriv(), Fixed::from_int(9));
    }

    #[test]
    fn deterministic_twice() {
        let g = |x: i32| {
            Dual::var(Fixed::from_int(x))
                .mul(Dual::var(Fixed::from_int(x)))
                .exp()
        };
        assert_eq!(g(1), g(1));
    }
}
