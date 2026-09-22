//! Normalized rational arithmetic — `Frac` keeps every value as a
//! reduced `(num, den)` pair in `i128` so comparisons and arithmetic
//! never drift. The shared scalar behind [`gauss`](crate::gauss)'s
//! solutions and [`bezier`](crate::bezier)'s curve points, and the
//! right currency for probability trees, drop rates, and any place a
//! fraction must stay *exact* across rounds of arithmetic.
//!
//! ```
//! use izanagi_kit::frac::Frac;
//! let half = Frac::new(1, 2);
//! let third = Frac::new(1, 3);
//! assert_eq!(half + third, Frac::new(5, 6));
//! assert_eq!(half.cmp_frac(&Frac::new(2, 4)), std::cmp::Ordering::Equal);
//! ```

/// Reduced rational `num/den` with `den > 0` and `gcd(|num|, den) == 1`
/// maintained as an invariant — equal rationals are structurally equal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Frac {
    /// Numerator (sign lives here).
    pub num: i128,
    /// Denominator, always positive.
    pub den: i128,
}

/// Euclidean gcd on `u128` — exact at the full width (unlike an
/// `i64`-narrowed helper).
fn gcd128(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

impl Frac {
    /// `num/den` reduced to lowest terms, `den` normalized positive.
    /// `den == 0` is the only nonsensical input — it clamps to `0/1`
    /// so the constructor stays total (division-by-zero is checked at
    /// [`checked_div`](Frac::checked_div), which returns `None`).
    pub fn new(num: i128, den: i128) -> Frac {
        if den == 0 {
            return Frac { num: 0, den: 1 };
        }
        let g = gcd128(num.unsigned_abs(), den.unsigned_abs()).max(1) as i128;
        let (mut n, mut d) = (num / g, den / g);
        if d < 0 {
            n = -n;
            d = -d;
        }
        Frac { num: n, den: d }
    }

    /// `n/1`.
    pub fn from_int(n: i128) -> Frac {
        Frac { num: n, den: 1 }
    }

    /// `self / rhs` — `None` on `rhs == 0`.
    pub fn checked_div(self, rhs: Frac) -> Option<Frac> {
        if rhs.num == 0 {
            return None;
        }
        Some(
            self * Frac {
                num: rhs.den,
                den: rhs.num,
            },
        )
    }

    /// `|self|`.
    pub fn abs(self) -> Frac {
        Frac {
            num: self.num.unsigned_abs() as i128,
            den: self.den,
        }
    }

    /// Three-way compare — `i128`-exact via cross-multiplication.
    pub fn cmp_frac(&self, rhs: &Frac) -> std::cmp::Ordering {
        (self.num * rhs.den).cmp(&(rhs.num * self.den))
    }

    /// `self` as a `(whole, frac)` split with `|frac| < 1` —
    /// e.g. `7/3 → (2, 1/3)`, `-7/3 → (-2, -1/3)`.
    pub fn split_mixed(&self) -> (i128, Frac) {
        let w = self.num / self.den;
        (
            w,
            Frac {
                num: self.num - w * self.den,
                den: self.den,
            },
        )
    }
}

impl std::ops::Add for Frac {
    type Output = Frac;
    /// `self + rhs` — exact, reduced.
    fn add(self, rhs: Frac) -> Frac {
        let n = self.num * rhs.den + rhs.num * self.den;
        Frac::new(n, self.den * rhs.den)
    }
}
impl std::ops::Sub for Frac {
    type Output = Frac;
    fn sub(self, rhs: Frac) -> Frac {
        self + (-rhs)
    }
}
impl std::ops::Mul for Frac {
    type Output = Frac;
    /// `self * rhs` — cross-reduced before multiplying.
    fn mul(self, rhs: Frac) -> Frac {
        let a = Frac::new(self.num, rhs.den);
        let b = Frac::new(rhs.num, self.den);
        Frac::new(a.num * b.num, a.den * b.den)
    }
}
impl std::ops::Neg for Frac {
    type Output = Frac;
    fn neg(self) -> Frac {
        Frac {
            num: -self.num,
            den: self.den,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn arithmetic_matches_pair_oracle() {
        let mut rng = SplitMix64::new(0xFAC7_10AA);
        for _ in 0..2000 {
            let (a, b, c, d) = (
                (rng.below(1000) as i128) - 500,
                (rng.below(500) as i128) + 1,
                (rng.below(1000) as i128) - 500,
                (rng.below(500) as i128) + 1,
            );
            let fa = Frac::new(a, b);
            let fb = Frac::new(c, d);
            // Reduction invariant on every produced value.
            for f in [fa, fb, fa + fb, fa * fb] {
                assert!(f.den > 0);
                assert_eq!(gcd128(f.num.unsigned_abs(), f.den as u128), 1, "{f:?}");
            }
            // add matches the raw-pair truth up to reduction:
            //   (a/b + c/d) == (ad' + bd)/dd'  ⟺  cross-multiply equal.
            let s = fa + fb;
            let (tn, td) = (fa.num * fb.den + fb.num * fa.den, fa.den * fb.den);
            assert_eq!(s.num * td, tn * s.den);
            // a/b - a/b == 0.
            assert_eq!((fa - fa).num, 0);
            // a/b * b/a == 1 when a != 0.
            if a != 0 {
                assert_eq!(fa * Frac::new(b, a), Frac::from_int(1));
            }
            // cmp agrees with cross-multiplied truth.
            let want = (fa.num * fb.den).cmp(&(fb.num * fa.den));
            assert_eq!(fa.cmp_frac(&fb), want);
            // div round-trips: (fa / fb) * fb == fa when fb != 0.
            if c != 0 {
                if let Some(q) = fa.checked_div(fb) {
                    assert_eq!(q * fb, fa);
                } else {
                    panic!("div failed on nonzero divisor");
                }
            }
        }
        // Mixed split: w + f == original, |f| < 1.
        let (w, f) = Frac::new(7, 3).split_mixed();
        assert_eq!(w, 2);
        assert_eq!(f, Frac::new(1, 3));
        let (w2, f2) = Frac::new(-7, 3).split_mixed();
        assert_eq!(w2, -2);
        assert_eq!(f2, Frac::new(-1, 3));
        // Norms and edge cases.
        assert_eq!(Frac::new(1, 0), Frac::new(0, 1));
        assert_eq!(Frac::new(-6, 8), Frac::new(-3, 4));
        assert_eq!(Frac::new(6, -8), Frac::new(-3, 4));
        assert_eq!(Frac::new(-6, -8), Frac::new(3, 4));
        assert_eq!(Frac::new(1, 2).abs(), Frac::new(1, 2));
        assert_eq!(Frac::new(-1, 2).abs(), Frac::new(1, 2));
        assert_eq!(-Frac::new(2, 5), Frac::new(-2, 5));
        assert!(Frac::new(1, 2).checked_div(Frac::new(0, 5)).is_none());
        assert_eq!(
            Frac::new(1, 2).checked_div(Frac::new(3, 4)),
            Some(Frac::new(2, 3))
        );
        assert_eq!(Frac::from_int(-9).num, -9);
    }
}
