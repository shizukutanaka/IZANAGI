//! Utility AI scoring (Dave Mark / "Infinite Axis" style) — response-curve
//! considerations whose product scores each candidate action; the
//! [`choose`] argmax decides. A deterministic alternative to `goap` plans
//! and behavior trees for continuous trade-offs.
//!
//! A [`Consideration`] maps a normalized input `x ∈ [0,1]` through a
//! [`Curve`] to a desirability in `[0,1]`; `score` multiplies them with the
//! classic compensation factor `score + score·m·(1-score)`, where
//! `m = 1 - 1/n`, so a single weak consideration still leaves the action
//! viable. All math is [`Fixed`] — the same inputs always pick the same
//! action, with lowest-index ties.
//!
//! ```
//! use izanagi_kit::{utility::{self, Consideration, Curve}, fixed::Fixed};
//! let cons = [
//!     Consideration { input: Fixed::ONE, curve: Curve::Linear, weight: Fixed::ONE },
//!     Consideration { input: Fixed::ONE, curve: Curve::Quad, weight: Fixed::ONE },
//! ];
//! assert_eq!(utility::score(&cons), Fixed::ONE);
//! assert_eq!(utility::choose(&[Fixed::ZERO, Fixed::ONE]), Some(1));
//! ```

use crate::fixed::Fixed;

/// A response curve mapping normalized input → normalized desirability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Curve {
    /// `y = x`
    Linear,
    /// `y = x²` — rewards strong inputs, suppresses weak ones.
    Quad,
    /// `y = 1 - x` — inverse response.
    Inverse,
    /// `y = 1 / (1 + e^(-k·(x - c)))` — logistic centered at `center` with
    /// steepness `steep` (e.g. steep=10 centers the slope sharply).
    Logistic {
        /// Sigmoid midpoint (normalized input).
        center: Fixed,
        /// Slope factor; larger = sharper transition.
        steep: Fixed,
    },
    /// `y = 1` when `x ≥ at`, else `0` — binary threshold.
    Step {
        /// Threshold input.
        at: Fixed,
    },
}

fn clamp01(x: Fixed) -> Fixed {
    if x.raw() <= 0 {
        Fixed::ZERO
    } else if x >= Fixed::ONE {
        Fixed::ONE
    } else {
        x
    }
}

/// Evaluate `curve` at `x` (input clamped to `[0,1]`; output in `[0,1]`).
pub fn eval(curve: &Curve, x: Fixed) -> Fixed {
    let x = clamp01(x);
    match *curve {
        Curve::Linear => x,
        Curve::Quad => x.mul(x),
        Curve::Inverse => Fixed::ONE - x,
        Curve::Logistic { center, steep } => {
            // 1/(1 + e^(steep·(center - x)))
            let z = steep.mul(center - x);
            Fixed::ONE.div(Fixed::ONE + Fixed::exp(z))
        }
        Curve::Step { at } => {
            if x >= at {
                Fixed::ONE
            } else {
                Fixed::ZERO
            }
        }
    }
}

/// One scored axis of an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Consideration {
    /// Normalized input in `[0,1]` (clamped at evaluation).
    pub input: Fixed,
    /// Response curve applied to `input`.
    pub curve: Curve,
    /// Weight raised into the product (`curve(input)^weight`, usually 1).
    pub weight: Fixed,
}

/// Combined utility of an action: the product of each consideration's
/// `eval(curve, input)^weight`, lifted by Dave Mark's compensation factor
/// `score·(1 + m·(1 - score))` with `m = 1 - 1/n` — this keeps an action
/// with one poor factor from collapsing entirely to zero. `weight` applies
/// as a power when positive; 0 means "ignore this consideration".
pub fn score(cons: &[Consideration]) -> Fixed {
    let mut product = Fixed::ONE;
    let mut n = 0usize;
    for c in cons {
        if c.weight.raw() <= 0 {
            continue;
        }
        n += 1;
        let base = eval(&c.curve, c.input);
        let contrib = if c.weight == Fixed::ONE {
            base
        } else {
            // base^weight via exp(weight·ln base); base 0 → 0.
            if base.raw() <= 0 {
                Fixed::ZERO
            } else {
                Fixed::exp(c.weight.mul(Fixed::ln(base)))
            }
        };
        product = product.mul(contrib);
    }
    if n == 0 {
        return Fixed::ZERO;
    }
    if n == 1 {
        return product;
    }
    // m = 1 - 1/n in raw integers (exact).
    let m = Fixed::from_ratio((n - 1) as i32, n as i32);
    product + product.mul(m).mul(Fixed::ONE - product)
}

/// Index of the maximum score; `None` for an empty slice. Ties go to the
/// lowest index.
pub fn choose(scores: &[Fixed]) -> Option<usize> {
    if scores.is_empty() {
        return None;
    }
    let mut best = 0;
    for (i, s) in scores.iter().enumerate() {
        if *s > scores[best] {
            best = i;
        }
    }
    Some(best)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(n: i32, d: i32) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    #[test]
    fn curves_hit_reference_points() {
        assert_eq!(eval(&Curve::Linear, f(1, 4)), f(1, 4));
        assert_eq!(eval(&Curve::Quad, f(1, 2)), f(1, 4));
        assert_eq!(eval(&Curve::Inverse, f(1, 4)), f(3, 4));
        assert_eq!(eval(&Curve::Step { at: f(1, 2) }, f(1, 4)), Fixed::ZERO);
        assert_eq!(eval(&Curve::Step { at: f(1, 2) }, f(3, 4)), Fixed::ONE);
        // Logistic at its center is exactly ½.
        assert_eq!(
            eval(
                &Curve::Logistic {
                    center: f(1, 2),
                    steep: Fixed::from_int(10)
                },
                f(1, 2)
            ),
            Fixed::ONE.div(Fixed::from_int(2))
        );
        // Logistic is monotonic increasing.
        let s = Curve::Logistic {
            center: f(1, 2),
            steep: Fixed::from_int(10),
        };
        assert!(eval(&s, f(1, 4)) < eval(&s, f(1, 2)));
        assert!(eval(&s, f(3, 4)) > eval(&s, f(1, 2)));
    }

    #[test]
    fn clamped_inputs() {
        assert_eq!(eval(&Curve::Linear, Fixed::from_int(-1)), Fixed::ZERO);
        assert_eq!(eval(&Curve::Linear, Fixed::from_int(5)), Fixed::ONE);
    }

    #[test]
    fn score_product_and_compensation() {
        // Two full-strength considerations → 1.
        let c = [
            Consideration {
                input: Fixed::ONE,
                curve: Curve::Linear,
                weight: Fixed::ONE,
            },
            Consideration {
                input: Fixed::ONE,
                curve: Curve::Quad,
                weight: Fixed::ONE,
            },
        ];
        assert_eq!(score(&c), Fixed::ONE);
        // A single zero factor: raw product 0 stays 0 even compensated.
        let bad = [
            c[0],
            Consideration {
                input: Fixed::ZERO,
                curve: Curve::Linear,
                weight: Fixed::ONE,
            },
        ];
        assert_eq!(score(&bad), Fixed::ZERO);
        // Half × half, n=2 → compensated ¼·(1 + ½·¾) = ¼·1.375 = 0.34375.
        let half = [Consideration {
            input: f(1, 2),
            curve: Curve::Linear,
            weight: Fixed::ONE,
        }; 2];
        assert_eq!(score(&half), f(11, 32));
        // Zero-weight factors are skipped entirely.
        let mut skip = Vec::from(half);
        skip.push(Consideration {
            input: Fixed::ZERO,
            curve: Curve::Linear,
            weight: Fixed::ZERO,
        });
        assert_eq!(score(&skip), f(11, 32));
        // Empty → 0.
        assert_eq!(score(&[]), Fixed::ZERO);
    }

    #[test]
    fn weight_exponent() {
        // weight 2 squares the factor's contribution.
        let c = [Consideration {
            input: f(1, 2),
            curve: Curve::Linear,
            weight: Fixed::from_int(2),
        }];
        assert_eq!(score(&c), f(1, 4));
    }

    #[test]
    fn choose_argmax_deterministic() {
        assert_eq!(choose(&[]), None);
        assert_eq!(choose(&[Fixed::ZERO, Fixed::ONE]), Some(1));
        // Tie → lowest index.
        assert_eq!(choose(&[Fixed::ONE, Fixed::ONE]), Some(0));
    }
}
