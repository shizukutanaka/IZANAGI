//! p-adic integers, truncated — arithmetic in `Z_p` carried
//! out exactly modulo `p^k`. An [`Adic`] value `v mod p^k`
//! supports the ring operations plus `inv`/`div` on `p`-units
//! and the p-adic *valuation* of an integer.
//!
//! The valuation `v_p(x)` — the exponent of `p` in `x` —
//! is the p-adic size of an integer, and the norm `p^{-v_p(x)}`
//! is what makes the p-adic completion meaningful: elements
//! are compared *exactly* here because every value is a
//! residue, not a float.
//!
//! ```
//! use izanagi_kit::adic::{Adic, valuation};
//! // In Z_3 truncated mod 3⁴: 1 + 3 = 4, and 1/2 exists (2 is a 3-unit)
//! let a = Adic::new(1, 3, 4).unwrap();
//! let two = Adic::new(2, 3, 4).unwrap();
//! assert_eq!((a + two).residue(), 3);
//! let half = two.inv().unwrap(); // 2·41 = 82 ≡ 1 (mod 81)
//! assert_eq!((two * half).residue(), 1);
//! assert_eq!(valuation(108, 3), Some(3)); // 108 = 4·27
//! ```
//!
//! References: p-adic arithmetic as in Gouvêa, *p-adic
//! Numbers* (truncated to mod `p^k` precision).

/// The p-adic valuation of `x`: the largest `e` with
/// `pᵉ | x`. `None` for `x = 0` (infinite valuation) or
/// `p < 2`.
pub fn valuation(x: u64, p: u64) -> Option<u32> {
    if x == 0 || p < 2 {
        return None;
    }
    let mut v = 0;
    let mut n = x;
    while n % p == 0 {
        n /= p;
        v += 1;
    }
    Some(v)
}

fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// A truncated p-adic integer: `v mod pᵏ`, `k ≥ 1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Adic {
    p: u64,
    k: u32,
    v: u64,
    /// Cached `pᵏ`.
    modulus: u64,
}

impl Adic {
    /// `x mod pᵏ`. `None` when `p < 2`, `k == 0`, `k > 31`
    /// (keeps `pᵏ` inside `u64` for `p ≤ u64::MAX`), or `pᵏ`
    /// overflows `u64`.
    pub fn new(x: u64, p: u64, k: u32) -> Option<Adic> {
        if p < 2 || k == 0 || k > 31 {
            return None;
        }
        let modulus = p.checked_pow(k)?;
        if modulus > i64::MAX as u64 {
            // residues stay inside i64 so `inv` can reuse the
            // i64 extgcd in `ntheory`
            return None;
        }
        Some(Adic {
            p,
            k,
            v: x % modulus,
            modulus,
        })
    }

    /// The residue `v mod pᵏ`.
    pub fn residue(&self) -> u64 {
        self.v
    }

    /// The modulus `pᵏ`.
    pub fn modulus(&self) -> u64 {
        self.modulus
    }

    /// Whether `self` is a unit mod `pᵏ`: `gcd(v, p) = 1`.
    /// For prime `p` this is `v % p ≠ 0`; composite `p` is
    /// handled correctly (`v = 2, p = 4` is not a unit).
    pub fn is_unit(&self) -> bool {
        gcd_u64(self.v, self.p) == 1
    }

    /// `self` at higher precision — the same residue mod `pᵏ'`,
    /// `k' > k`. `None` on `k' > 31` or modulus overflow.
    pub fn lift(&self, k2: u32) -> Option<Adic> {
        if k2 <= self.k {
            return None;
        }
        Adic::new(self.v, self.p, k2)
    }

    /// `self` at lower precision `k' < k` (truncate residue).
    pub fn trunc(&self, k2: u32) -> Option<Adic> {
        if k2 == 0 || k2 >= self.k {
            return None;
        }
        Adic::new(self.v, self.p, k2)
    }
}

impl std::ops::Add for Adic {
    type Output = Adic;
    fn add(self, r: Adic) -> Adic {
        Adic {
            p: self.p,
            k: self.k,
            v: (self.v + r.v) % self.modulus,
            modulus: self.modulus,
        }
    }
}

impl std::ops::Sub for Adic {
    type Output = Adic;
    fn sub(self, r: Adic) -> Adic {
        Adic {
            p: self.p,
            k: self.k,
            v: (self.modulus + self.v - r.v) % self.modulus,
            modulus: self.modulus,
        }
    }
}

impl std::ops::Neg for Adic {
    type Output = Adic;
    fn neg(self) -> Adic {
        Adic {
            p: self.p,
            k: self.k,
            v: if self.v == 0 {
                0
            } else {
                self.modulus - self.v
            },
            modulus: self.modulus,
        }
    }
}

impl std::ops::Mul for Adic {
    type Output = Adic;
    fn mul(self, r: Adic) -> Adic {
        let prod = u128::from(self.v) * u128::from(r.v);
        Adic {
            p: self.p,
            k: self.k,
            v: (prod % u128::from(self.modulus)) as u64,
            modulus: self.modulus,
        }
    }
}

impl Adic {
    /// p-adic inverse — `None` when `self` is not a p-unit.
    /// Computed as a mod-`pᵏ` inverse via `extgcd` — exact.
    pub fn inv(&self) -> Option<Adic> {
        if !self.is_unit() {
            return None;
        }
        // modulus ≤ i64::MAX by `new`'s bound
        let i = crate::ntheory::mod_inv(self.v as i64, self.modulus as i64)?;
        Adic::new(i as u64, self.p, self.k)
    }

    /// `self / r` — `None` when `r` is not a p-unit.
    pub fn div(&self, r: &Adic) -> Option<Adic> {
        Some(*self * r.inv()?)
    }

    /// The p-adic valuation of this truncated value —
    /// `v_p(v)` when `v ≠ 0 mod pᵏ`; `None` when `v ≡ 0`
    /// (the truncated zero has valuation ≥ k — the true
    /// valuation is unknown at this precision).
    pub fn val(&self) -> Option<u32> {
        if self.v == 0 {
            return None;
        }
        let mut v = self.v;
        let mut e = 0;
        while v % self.p == 0 {
            v /= self.p;
            e += 1;
        }
        Some(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basics() {
        let a = Adic::new(1, 3, 4).unwrap();
        let two = Adic::new(2, 3, 4).unwrap();
        assert_eq!((a + two).residue(), 3);
        let h = two.inv().unwrap();
        assert_eq!((two * h).residue(), 1);
        assert_eq!(valuation(108, 3), Some(3));
        assert_eq!(valuation(0, 3), None);
        assert_eq!(valuation(5, 3), Some(0));
        assert!(Adic::new(1, 1, 4).is_none());
        assert!(Adic::new(1, 3, 0).is_none());
        // non-unit has no inverse mod 3^4
        assert!(Adic::new(3, 3, 4).unwrap().inv().is_none());
        assert!(!Adic::new(3, 3, 4).unwrap().is_unit());
        // composite p: units are gcd(v,p)=1 — 2 is not a unit
        // mod 4^k even though 2 % 4 != 0; 3 is a unit (3*11=33=1 mod 16)
        assert!(!Adic::new(2, 4, 2).unwrap().is_unit());
        assert_eq!(Adic::new(2, 4, 2).unwrap().inv(), None);
        assert_eq!(Adic::new(3, 4, 2).unwrap().inv().unwrap().residue(), 11);
        let three = Adic::new(3, 3, 4).unwrap();
        assert_eq!(three.val(), Some(1));
        assert_eq!(Adic::new(0, 3, 4).unwrap().val(), None);
    }

    /// Shadow oracle: every op must agree with plain
    /// `mod pᵏ` arithmetic on the residues.
    #[test]
    fn oracle_ring_laws() {
        let mut rng = SplitMix64::new(0xAD1C);
        for _ in 0..200 {
            let p = 2 + u64::from(rng.below(5));
            let k = 1 + rng.below(6);
            let m = p.pow(k);
            let x = u64::from(rng.below(m as u32));
            let y = u64::from(rng.below(m as u32));
            let a = Adic::new(x, p, k).unwrap();
            let b = Adic::new(y, p, k).unwrap();
            assert_eq!((a + b).residue(), (x + y) % m);
            assert_eq!((a - b).residue(), (m + x - y) % m);
            assert_eq!(
                (a * b).residue(),
                ((u128::from(x) * u128::from(y)) % u128::from(m)) as u64
            );
            assert_eq!((-a).residue(), if x == 0 { 0 } else { m - x });
            // inverses iff gcd(x, p) == 1
            let xu = gcd_u64(x, p) == 1;
            let yu = gcd_u64(y, p) == 1;
            assert_eq!(a.inv().is_some(), xu);
            if xu {
                let inv = a.inv().unwrap();
                assert_eq!((a * inv).residue(), 1 % m);
                assert_eq!(a.div(&b).is_some(), yu);
            }
            // v mod p^k ≠ 0 implies v_p(x) < k, so the
            // truncated valuation equals the global one
            if x % m != 0 {
                assert_eq!(a.val(), valuation(x, p));
            }
        }
    }

    /// Hensel-flavored check: lifting a root of `x² ≡ −1`
    /// mod 5 to mod 25 — the p-adic machinery treats residues
    /// at higher precision as extensions.
    #[test]
    fn lift_and_trunc() {
        let a = Adic::new(2, 5, 1).unwrap(); // 2² = 4 ≡ −1 mod 5
        let b = a.lift(2).unwrap();
        assert_eq!(b.residue(), 2);
        assert_eq!(b.modulus(), 25);
        let t = b.trunc(1).unwrap();
        assert_eq!(t.residue(), 2);
        assert_eq!(t.modulus(), 5);
        assert!(a.trunc(1).is_none());
        assert!(b.lift(1).is_none());
    }
}
