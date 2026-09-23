//! Signed arbitrary-precision integer — sign-magnitude over
//! little-endian `u64` limbs, normalized so no top limb is
//! zero and zero itself is always non-negative. Covers
//! `add`/`sub`/`neg`/`cmp`/`mul`/`pow` exactly, no floats and
//! no allocation-dependent results: two equal values have
//! byte-identical limb vectors (canonical form).
//!
//! `karatsuba` multiplies *unsigned* limb slices; this module
//! gives the signed composite type around a schoolbook kernel —
//! the right scope for sizes games actually reach (score
//! counters, resource hashes, big modular precomputes), while
//! `conv`/`karatsuba` stay the kernels for bulk products.
//!
//! ```
//! use izanagi_kit::bigint::BigInt;
//! let a = BigInt::from_i64(-7);
//! let b = BigInt::from_i64(19);
//! assert_eq!(a.mul(&b).to_i128(), Some(-133));
//! assert!(BigInt::from_limbs(&[!0; 4]).to_i128().is_none());
//! ```
//!
//! References: Knuth TAOCP 4.3.1 (classical algorithms),
//! cp-algorithms bigint for the sign-magnitude layout.

use std::cmp::Ordering;

/// Three-way comparison, sign then magnitude.
fn cmp_bigint(a: &BigInt, b: &BigInt) -> Ordering {
    match (a.neg, b.neg) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => cmp_mag(&a.mag, &b.mag),
        (true, true) => cmp_mag(&b.mag, &a.mag),
    }
}

impl Ord for BigInt {
    fn cmp(&self, rhs: &BigInt) -> Ordering {
        cmp_bigint(self, rhs)
    }
}

impl PartialOrd for BigInt {
    fn partial_cmp(&self, rhs: &BigInt) -> Option<Ordering> {
        Some(self.cmp(rhs))
    }
}

/// Signed arbitrary-precision integer, canonical
/// sign-magnitude form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BigInt {
    /// `false` for `≥ 0`. Invariant: `mag` holds no trailing
    /// zero limb, and `neg` is `false` whenever `mag` is empty.
    neg: bool,
    /// Little-endian `u64` magnitude.
    mag: Vec<u64>,
}

fn norm(mut b: BigInt) -> BigInt {
    while b.mag.last() == Some(&0) {
        b.mag.pop();
    }
    if b.mag.is_empty() {
        b.neg = false;
    }
    b
}

fn cmp_mag(a: &[u64], b: &[u64]) -> Ordering {
    if a.len() != b.len() {
        return a.len().cmp(&b.len());
    }
    for i in (0..a.len()).rev() {
        match a[i].cmp(&b[i]) {
            Ordering::Equal => {}
            o => return o,
        }
    }
    Ordering::Equal
}

fn add_mag(a: &[u64], b: &[u64]) -> Vec<u64> {
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let mut out = Vec::with_capacity(long.len() + 1);
    let mut carry = 0u64;
    for i in 0..long.len() {
        let x = long[i];
        let y = if i < short.len() { short[i] } else { 0 };
        let (s1, c1) = x.overflowing_add(y);
        let (s2, c2) = s1.overflowing_add(carry);
        out.push(s2);
        carry = u64::from(c1 || c2);
    }
    if carry != 0 {
        out.push(carry);
    }
    out
}

/// `a − b` for `a ≥ b` magnitudes.
fn sub_mag(a: &[u64], b: &[u64]) -> Vec<u64> {
    let mut out = Vec::with_capacity(a.len());
    let mut borrow = 0u64;
    for i in 0..a.len() {
        let x = a[i];
        let y = if i < b.len() { b[i] } else { 0 };
        let (d1, b1) = x.overflowing_sub(y);
        let (d2, b2) = d1.overflowing_sub(borrow);
        out.push(d2);
        borrow = u64::from(b1 || b2);
    }
    while out.last() == Some(&0) {
        out.pop();
    }
    out
}

fn mul_mag(a: &[u64], b: &[u64]) -> Vec<u64> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0u64; a.len() + b.len()];
    for (i, &x) in a.iter().enumerate() {
        let mut carry = 0u64;
        for (j, &y) in b.iter().enumerate() {
            let (lo, hi) = mul_wide(x, y);
            let (s1, c1) = out[i + j].overflowing_add(lo);
            let (s2, c2) = s1.overflowing_add(carry);
            out[i + j] = s2;
            carry = hi + u64::from(c1 || c2);
        }
        let mut k = i + b.len();
        while carry != 0 {
            let (s, c) = out[k].overflowing_add(carry);
            out[k] = s;
            carry = u64::from(c);
            k += 1;
        }
    }
    while out.last() == Some(&0) {
        out.pop();
    }
    out
}

/// Full `u64 × u64 → u128` split into `(lo, hi)` — avoids
/// `to_be_bytes`-adjacent patterns by keeping the product in
/// one `u128`.
fn mul_wide(x: u64, y: u64) -> (u64, u64) {
    let p = (x as u128) * (y as u128);
    (p as u64, (p >> 64) as u64)
}

impl BigInt {
    /// Zero.
    pub fn zero() -> BigInt {
        BigInt {
            neg: false,
            mag: Vec::new(),
        }
    }

    /// From an `i64` — exact.
    pub fn from_i64(v: i64) -> BigInt {
        if v == 0 {
            return BigInt::zero();
        }
        BigInt {
            neg: v < 0,
            mag: vec![v.unsigned_abs()],
        }
    }

    /// From an `i128` — exact.
    pub fn from_i128(v: i128) -> BigInt {
        if v == 0 {
            return BigInt::zero();
        }
        let m = v.unsigned_abs();
        BigInt {
            neg: v < 0,
            mag: vec![m as u64, (m >> 64) as u64],
        }
        .normalized()
    }

    /// Positive BigInt from raw little-endian limbs —
    /// normalized (top zeros dropped).
    pub fn from_limbs(mag: &[u64]) -> BigInt {
        norm(BigInt {
            neg: false,
            mag: mag.to_vec(),
        })
    }

    /// `self == 0`.
    pub fn is_zero(&self) -> bool {
        self.mag.is_empty()
    }

    /// `-self`.
    pub fn neg(&self) -> BigInt {
        if self.is_zero() {
            self.clone()
        } else {
            BigInt {
                neg: !self.neg,
                mag: self.mag.clone(),
            }
        }
    }

    /// `|self|`.
    pub fn abs(&self) -> BigInt {
        BigInt {
            neg: false,
            mag: self.mag.clone(),
        }
    }

    /// `self < 0`.
    pub fn is_negative(&self) -> bool {
        self.neg
    }

    /// `self + rhs`.
    pub fn add(&self, rhs: &BigInt) -> BigInt {
        match (self.neg, rhs.neg) {
            (a, b) if a == b => norm(BigInt {
                neg: a,
                mag: add_mag(&self.mag, &rhs.mag),
            }),
            _ => match cmp_mag(&self.mag, &rhs.mag) {
                Ordering::Equal => BigInt::zero(),
                Ordering::Greater => norm(BigInt {
                    neg: self.neg,
                    mag: sub_mag(&self.mag, &rhs.mag),
                }),
                Ordering::Less => norm(BigInt {
                    neg: rhs.neg,
                    mag: sub_mag(&rhs.mag, &self.mag),
                }),
            },
        }
    }

    /// `self − rhs`.
    pub fn sub(&self, rhs: &BigInt) -> BigInt {
        self.add(&rhs.neg())
    }

    /// `self × rhs` — schoolbook `O(n·m)`; fine to the sizes
    /// counters and hashes reach.
    pub fn mul(&self, rhs: &BigInt) -> BigInt {
        norm(BigInt {
            neg: self.neg != rhs.neg,
            mag: mul_mag(&self.mag, &rhs.mag),
        })
    }

    /// `self^e` by binary exponentiation — `e` is a `u64`
    /// exponent, `self^0 = 1`.
    pub fn pow(&self, mut e: u64) -> BigInt {
        let mut base = self.clone();
        let mut acc = BigInt::from_i64(1);
        while e > 0 {
            if e & 1 == 1 {
                acc = acc.mul(&base);
            }
            base = base.mul(&base);
            e >>= 1;
        }
        acc
    }

    /// Little-endian magnitude limbs — `&[]` for zero.
    pub fn limbs(&self) -> &[u64] {
        &self.mag
    }

    /// Exact `i128` conversion — `None` when `|self| > i128::MAX`
    /// (and `None` for `i128::MIN` magnitude `2^127` only when
    /// `neg`, which *does* fit — that case is returned).
    pub fn to_i128(&self) -> Option<i128> {
        if self.mag.len() > 2 {
            return None;
        }
        let lo = *self.mag.first().unwrap_or(&0) as u128;
        let hi = *self.mag.get(1).unwrap_or(&0) as u128;
        let m = (hi << 64) | lo;
        if self.neg {
            if m == (1u128 << 127) {
                Some(i128::MIN)
            } else if m < (1u128 << 127) {
                Some(-(m as i128))
            } else {
                None
            }
        } else if m <= i128::MAX as u128 {
            Some(m as i128)
        } else {
            None
        }
    }

    /// `self` as `u64` — `None` when negative or > `u64::MAX`.
    pub fn to_u64(&self) -> Option<u64> {
        if self.neg || self.mag.len() > 1 {
            return None;
        }
        Some(self.mag.first().copied().unwrap_or(0))
    }

    fn normalized(self) -> BigInt {
        norm(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn big_of(v: i128) -> BigInt {
        BigInt::from_i128(v)
    }

    #[test]
    fn basics() {
        assert_eq!(BigInt::zero().to_i128(), Some(0));
        assert!(!BigInt::zero().is_negative());
        assert_eq!(
            BigInt::from_i64(-7).mul(&BigInt::from_i64(19)).to_i128(),
            Some(-133)
        );
        assert_eq!(BigInt::from_i64(0).neg().to_i128(), Some(0));
        // canonical: no trailing zero limbs
        let b = BigInt::from_i128(1i128 << 100);
        assert_eq!(b.limbs().len(), 2);
    }

    #[test]
    fn i128_oracle() {
        let mut rng = SplitMix64::new(77);
        for _ in 0..2000 {
            let a = rng.next_u64() as i128 * (rng.below(3) as i128 - 1);
            let b = rng.next_u64() as i128 * (rng.below(3) as i128 - 1);
            let (x, y) = (big_of(a), big_of(b));
            assert_eq!(x.add(&y).to_i128(), a.checked_add(b));
            assert_eq!(x.sub(&y).to_i128(), a.checked_sub(b));
            assert_eq!(x.cmp(&y), a.cmp(&b), "a={a} b={b}");
            assert_eq!(x.eq(&y), a == b);
            assert_eq!(x.neg().to_i128(), Some(-a));
        }
    }

    #[test]
    fn mul_oracle() {
        let mut rng = SplitMix64::new(78);
        for _ in 0..500 {
            // keep products inside i128 for the oracle
            let a = (rng.next_u64() as i128) * (rng.below(3) as i128 - 1);
            let b = rng.below(1_000_000) as i128 * (rng.below(3) as i128 - 1);
            assert_eq!(big_of(a).mul(&big_of(b)).to_i128(), a.checked_mul(b));
        }
        // carry chains at limb boundaries: (2^128−1)² = 2^256−2^129+1
        assert_eq!(
            BigInt::from_limbs(&[!0u64; 2])
                .mul(&BigInt::from_limbs(&[!0u64; 2]))
                .limbs(),
            &[1, 0, !0u64 - 1, !0u64]
        );
        assert_eq!(
            BigInt::from_i64(-1).mul(&BigInt::from_i64(-1)).to_i128(),
            Some(1)
        );
    }

    #[test]
    fn pow_and_bounds() {
        assert_eq!(BigInt::from_i64(3).pow(0).to_i128(), Some(1));
        assert_eq!(BigInt::from_i64(3).pow(4).to_i128(), Some(81));
        let neg2_65 = BigInt::from_i64(-2).pow(65);
        assert_eq!(neg2_65.limbs(), &[0, 2]);
        assert!(neg2_65.is_negative());
        // to_i128 boundaries
        assert_eq!(BigInt::from_i128(i128::MAX).to_i128(), Some(i128::MAX));
        assert_eq!(BigInt::from_i128(i128::MIN).to_i128(), Some(i128::MIN));
        assert_eq!(BigInt::from_limbs(&[!0u64; 3]).to_i128(), None);
        assert_eq!(BigInt::from_limbs(&[!0u64; 2]).to_i128(), None);
        assert_eq!(BigInt::from_i64(-1).to_u64(), None);
        assert_eq!(BigInt::from_i64(-1).abs().to_u64(), Some(1));
    }
}
