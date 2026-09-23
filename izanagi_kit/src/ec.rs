//! Elliptic-curve point arithmetic over GF(p) — the short
//! Weierstrass group law `y² = x³ + a·x + b`, computed with
//! `i128` intermediates so coordinates may range over any
//! prime `p < 2⁶²`.
//!
//! [`Point`] is the additive group: `Inf` is the identity,
//! `add`/`double`/`neg` follow the affine chord-tangent
//! formulas, and `mul` is scalar multiplication by
//! double-and-add.  is provided because affine
//! coordinates from the caller may not lie on the curve at
//! all — the module never silently treats an off-curve point
//! as a group element (callers check; `add` etc. still return
//! a `Point` whose coordinates are reduced mod `p`).
//!
//! ```
//! use izanagi_kit::ec::{Curve, Point};
//! // y² = x³ + 2x + 2 over GF(17) — the famous small curve
//! let c = Curve::new(2, 2, 17).unwrap();
//! let p = Point::Affine(5, 1);
//! assert!(c.on_curve(p));
//! assert_eq!(c.mul(p, 19), Point::Inf); // #E(F_17) = 19
//! ```
//!
//! References: standard affine group law (Cohen & Frey,
//! *Handbook of Elliptic and Hyperelliptic Curve
//! Cryptography*, §13.2).

/// An affine curve `y² = x³ + a·x + b` over the field `GF(p)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Curve {
    /// `a` reduced mod `p`.
    pub a: i64,
    /// `b` reduced mod `p`.
    pub b: i64,
    /// Field characteristic (must be a prime ≥ 5 for the
    /// chord-tangent law; `p = 2, 3` need the extended law,
    /// which this module does not implement — `new` rejects).
    pub p: i64,
}

/// A point on (or near) the curve: the group identity
/// `Inf` or affine coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Point {
    /// Point at infinity — the additive identity.
    Inf,
    /// Affine point `(x, y)` reduced mod `p`.
    Affine(i64, i64),
}

fn mmul(x: i64, y: i64, p: i64) -> i64 {
    (i128::from(x) * i128::from(y)).rem_euclid(i128::from(p)) as i64
}

fn madd(x: i64, y: i64, p: i64) -> i64 {
    (i128::from(x) + i128::from(y)).rem_euclid(i128::from(p)) as i64
}

impl Curve {
    /// Curve `y² = x³ + a·x + b` over `GF(p)`, `a`/`b` reduced
    /// mod `p`. `None` when `p < 5` (the affine law needs the
    /// extended group law there) or `p` is composite-prime
    /// agnostic — composite `p` still forms a ring where the
    /// formulas apply but the chord-tangent *group* law can
    /// fail; primality is the caller's contract and is checked
    /// only up to honesty: use [`Curve::is_field_prime`] if
    /// needed.
    pub fn new(a: i64, b: i64, p: i64) -> Option<Curve> {
        if p < 5 {
            return None;
        }
        Some(Curve {
            a: madd(a, 0, p),
            b: madd(b, 0, p),
            p,
        })
    }

    /// Whether `self.p` is prime (deterministic trial
    /// division — `p < 2⁶²`, fine for the intended sizes).
    pub fn is_field_prime(&self) -> bool {
        let p = self.p;
        if p < 2 {
            return false;
        }
        let mut d = 2i64;
        while i128::from(d) * i128::from(d) <= i128::from(p) {
            if p % d == 0 {
                return false;
            }
            d += 1;
        }
        true
    }

    /// Whether `q` lies on the curve (or is `Inf`).
    pub fn on_curve(&self, q: Point) -> bool {
        match q {
            Point::Inf => true,
            Point::Affine(x, y) => {
                mmul(y, y, self.p)
                    == madd(
                        madd(
                            mmul(x, mmul(x, x, self.p), self.p),
                            mmul(self.a, x, self.p),
                            self.p,
                        ),
                        self.b,
                        self.p,
                    )
            }
        }
    }

    /// `q1 + q2` under the group law.
    pub fn add(&self, q1: Point, q2: Point) -> Point {
        match (q1, q2) {
            (Point::Inf, _) => q2,
            (_, Point::Inf) => q1,
            (Point::Affine(x1, y1), Point::Affine(x2, y2)) => {
                if x1 == x2 {
                    if madd(y1, y2, self.p) == 0 {
                        return Point::Inf;
                    }
                    return self.double(q1);
                }
                // slope s = (y2 - y1)/(x2 - x1)
                let num = madd(y2, -y1, self.p);
                let den = madd(x2, -x1, self.p);
                let inv = crate::ntheory::mod_inv(den, self.p);
                match inv {
                    None => Point::Inf, // non-field p: honest fallback
                    Some(dinv) => {
                        let s = mmul(num, dinv, self.p);
                        let x3 = madd(madd(mmul(s, s, self.p), -x1, self.p), -x2, self.p);
                        let y3 = madd(mmul(s, madd(x1, -x3, self.p), self.p), -y1, self.p);
                        Point::Affine(x3, y3)
                    }
                }
            }
        }
    }

    /// `2·q` under the group law.
    pub fn double(&self, q: Point) -> Point {
        match q {
            Point::Inf => Point::Inf,
            Point::Affine(x, y) => {
                if y == 0 {
                    return Point::Inf;
                }
                // tangent slope s = (3x² + a)/(2y)
                let num = madd(mmul(3, mmul(x, x, self.p), self.p), self.a, self.p);
                let den = mmul(2, y, self.p);
                match crate::ntheory::mod_inv(den, self.p) {
                    None => Point::Inf,
                    Some(dinv) => {
                        let s = mmul(num, dinv, self.p);
                        let x3 = madd(mmul(s, s, self.p), -mmul(2, x, self.p), self.p);
                        let y3 = madd(mmul(s, madd(x, -x3, self.p), self.p), -y, self.p);
                        Point::Affine(x3, y3)
                    }
                }
            }
        }
    }

    /// `−q`.
    pub fn neg(&self, q: Point) -> Point {
        match q {
            Point::Inf => Point::Inf,
            Point::Affine(x, y) => Point::Affine(x, madd(-y, 0, self.p)),
        }
    }

    /// `k·q` by double-and-add over the scalar bits.
    pub fn mul(&self, q: Point, mut k: u64) -> Point {
        let mut acc = Point::Inf;
        let mut cur = q;
        while k > 0 {
            if k & 1 == 1 {
                acc = self.add(acc, cur);
            }
            cur = self.double(cur);
            k >>= 1;
        }
        acc
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// The textbook curve y² = x³ + 2x + 2 over F17:
    /// |E(F17)| = 19, every nonzero element generates.
    fn tiny() -> Curve {
        Curve::new(2, 2, 17).unwrap()
    }

    #[test]
    fn basics() {
        let c = tiny();
        assert!(c.is_field_prime());
        let g = Point::Affine(5, 1);
        assert!(c.on_curve(g));
        assert_eq!(c.mul(g, 19), Point::Inf);
        assert_eq!(c.mul(g, 20), g);
        assert_eq!(c.add(g, c.neg(g)), Point::Inf);
        assert_eq!(c.add(g, Point::Inf), g);
        assert!(!c.on_curve(Point::Affine(0, 0)));
        assert!(Curve::new(1, 1, 3).is_none());
    }

    /// The whole group table of the 19-element curve matches
    /// brute enumeration: group is cyclic, so kP is a bijection
    /// and every point is `on_curve`.
    #[test]
    fn group_table_is_cyclic() {
        let c = tiny();
        let g = Point::Affine(5, 1);
        let mut seen = std::collections::BTreeSet::new();
        for k in 0..19 {
            let p = c.mul(g, k);
            assert!(c.on_curve(p));
            seen.insert(p);
        }
        assert_eq!(seen.len(), 19); // 18 affine + Inf
                                    // group law agrees with the scalar form: kP+lP=(k+l)P
        let mut rng = SplitMix64::new(7);
        for _ in 0..200 {
            let k = u64::from(rng.below(19));
            let l = u64::from(rng.below(19));
            let got = c.add(c.mul(g, k), c.mul(g, l));
            let want = c.mul(g, (k + l) % 19);
            assert_eq!(got, want);
        }
    }

    /// Larger prime: associativity spot-checks and
    /// `k·P + l·P = (k+l)·P` on a non-cyclic-sized curve.
    #[test]
    fn law_holds_on_larger_prime() {
        // y² = x³ + x + 1 over F23 — |E| = 28
        let c = Curve::new(1, 1, 23).unwrap();
        let g = Point::Affine(0, 1); // 0² = 0 + 0 + 1 ✓
        assert!(c.on_curve(g));
        let mut rng = SplitMix64::new(11);
        for _ in 0..300 {
            let k = u64::from(rng.below(29));
            let l = u64::from(rng.below(29));
            let m = u64::from(rng.below(29));
            assert_eq!(c.add(c.mul(g, k), c.mul(g, l)), c.mul(g, k + l),);
            // associativity: (kP+lP)+mP = kP+(lP+mP)
            let p_k = c.mul(g, k);
            let p_l = c.mul(g, l);
            let p_m = c.mul(g, m);
            assert_eq!(c.add(c.add(p_k, p_l), p_m), c.add(p_k, c.add(p_l, p_m)),);
        }
        // group order divides |E|: 28·P = Inf iff ord | 28 —
        // the actual order of (0,1) on this curve is 28
        assert_eq!(c.mul(g, 28), Point::Inf);
    }

    /// `y = 0` points are their own inverses.
    #[test]
    fn two_torsion() {
        // y² = x³ over F7: (0,0) is on the curve (a=b=0 — a
        // singular example; the law still computes, but for an
        // honest 2-torsion use a non-singular curve)
        let c = Curve::new(0, 0, 7).unwrap();
        let p0 = Point::Affine(0, 0);
        assert!(c.on_curve(p0));
        assert_eq!(c.double(p0), Point::Inf);
    }
}
