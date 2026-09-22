//! Integer number theory — gcd, lcm, extended Euclid, modular
//! inverse/power, and the Chinese Remainder Theorem. The arithmetic
//! backbone for fixed-point-friendly exactness: cycle detection,
//! coprime scheduling, hash-function design, and unit conversion all
//! land here.
//!
//! Every function is total and deterministic: impossible inputs
//! (zero modulus, non-invertible, pairwise non-coprime CRT moduli)
//! return `None`, never panic.
//!
//! ```
//! use izanagi_kit::ntheory::{gcd, lcm, extgcd, mod_inv, mod_pow, crt2};
//! assert_eq!(gcd(48, 36), 12);
//! assert_eq!(lcm(21, 6), 42);
//! // Bezout: 3·3 + 8·(−1) = gcd(3,8) = 1.
//! assert_eq!(extgcd(3, 8), (1, 3, -1));
//! assert_eq!(mod_inv(3, 11), Some(4)); // 3·4 ≡ 1 (mod 11)
//! assert_eq!(mod_pow(2, 10, 1024), Some(0)); // 1024 mod 1024
//! // x ≡ 2 mod 3, x ≡ 3 mod 5 → x ≡ 8 mod 15.
//! assert_eq!(crt2(2, 3, 3, 5), Some((8, 15)));
//! ```

/// Greatest common divisor of `|a|` and `|b|` — always non-negative;
/// `gcd(0, 0) = 0`.
pub fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.unsigned_abs(), b.unsigned_abs());
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a as i64
}

/// Least common multiple — `lcm(0, anything) = 0`. Non-negative.
/// Divides first so `lcm(a, b)` never overflows before `a·b` would.
pub fn lcm(a: i64, b: i64) -> i64 {
    if a == 0 || b == 0 {
        return 0;
    }
    let g = gcd(a, b);
    (a / g).checked_mul(b).map(|v| v.abs()).unwrap_or(0)
}

/// Extended Euclidean algorithm: `(g, x, y)` with `g = gcd(a, b)`
/// and `a·x + b·y = g`. `g` is non-negative.
pub fn extgcd(a: i64, b: i64) -> (i64, i64, i64) {
    let (mut r0, mut r1) = (a, b);
    let (mut x0, mut x1) = (1i64, 0i64);
    let (mut y0, mut y1) = (0i64, 1i64);
    while r1 != 0 {
        let q = r0 / r1;
        (r0, r1) = (r1, r0 - q * r1);
        (x0, x1) = (x1, x0 - q * x1);
        (y0, y1) = (y1, y0 - q * y1);
    }
    if r0 < 0 {
        (-r0, -x0, -y0)
    } else {
        (r0, x0, y0)
    }
}

/// Multiplicative inverse of `a` modulo `m`, in `[0, m)` — `Some`
/// iff `gcd(a, m) == 1`. `m <= 0` or `m == 1` → `None`.
pub fn mod_inv(a: i64, m: i64) -> Option<i64> {
    if m <= 1 {
        return None;
    }
    let (g, x, _) = extgcd(a, m);
    if g != 1 {
        return None;
    }
    Some(x.rem_euclid(m))
}

/// `base^exp mod m` — square-and-multiply with `u128` intermediates so
/// results stay exact while `m·m < 2^128`. `m <= 0` or `exp < 0` →
/// `None`; `m == 1` → `Some(0)` (everything is 0 mod 1).
pub fn mod_pow(base: i64, exp: i64, m: i64) -> Option<i64> {
    if m <= 0 || exp < 0 {
        return None;
    }
    let m = m as u128;
    let mut b = base.rem_euclid(m as i64) as u128;
    let mut e = exp as u64;
    let mut acc = 1u128 % m;
    while e > 0 {
        if e & 1 == 1 {
            acc = acc * b % m;
        }
        b = b * b % m;
        e >>= 1;
    }
    Some(acc as i64)
}

/// Two-congruence CRT: solves `x ≡ r1 (m1)`, `x ≡ r2 (m2)` →
/// `(x, lcm(m1, m2))` with `x` the smallest non-negative solution,
/// `Some` iff one exists (`r1 ≡ r2 mod gcd(m1, m2)`). Moduli must
/// be ≥ 1; modulus 1 is the trivial congruence (always satisfied).
pub fn crt2(r1: i64, m1: i64, r2: i64, m2: i64) -> Option<(i64, i64)> {
    if m1 < 1 || m2 < 1 {
        return None;
    }
    if m1 == 1 {
        return Some((r2.rem_euclid(m2), m2));
    }
    if m2 == 1 {
        return Some((r1.rem_euclid(m1), m1));
    }
    let r1 = r1.rem_euclid(m1);
    let r2 = r2.rem_euclid(m2);
    let g = gcd(m1, m2);
    if (r2 - r1).rem_euclid(g) != 0 {
        return None;
    }
    // x = r1 + m1·t, solve t ≡ ((r2-r1)/g)·inv(m1/g) (mod m2/g).
    let m1r = m1 / g;
    let m2r = m2 / g;
    let inv = mod_inv(m1r.rem_euclid(m2r), m2r).unwrap_or(1);
    let t = ((r2 - r1) / g)
        .rem_euclid(m2r)
        .checked_mul(inv)?
        .rem_euclid(m2r);
    let mm = m1.checked_mul(m2r)?;
    let x = r1.checked_add(m1.checked_mul(t)?)?.rem_euclid(mm);
    Some((x, mm))
}

/// General CRT over `(residue, modulus)` pairs — `Some((x, M))` with
/// `M` the lcm of the moduli, or `None` if any pair is inconsistent
/// or a modulus is < 2. Empty list → `Some((0, 1))`.
pub fn crt(terms: &[(i64, i64)]) -> Option<(i64, i64)> {
    let mut acc = (0i64, 1i64);
    for &(r, m) in terms {
        acc = crt2(acc.0, acc.1, r, m)?;
    }
    Some(acc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Naive gcd oracle: largest d dividing both (direct scan).
    fn naive_gcd(a: i64, b: i64) -> i64 {
        if a == 0 && b == 0 {
            return 0;
        }
        let mut best = 0i64;
        for d in 1..=a.unsigned_abs().max(b.unsigned_abs()) as i64 {
            if a % d == 0 && b % d == 0 {
                best = d;
            }
        }
        best
    }

    #[test]
    fn gcd_lcm_match_oracle_and_identities() {
        let mut rng = SplitMix64::new(0x6CD);
        for _ in 0..2_000 {
            let a = rng.range(-2_000, 2_001) as i64;
            let b = rng.range(-2_000, 2_001) as i64;
            let g = gcd(a, b);
            assert_eq!(g, naive_gcd(a, b));
            assert!(g >= 0);
            if g > 0 {
                assert_eq!(a % g, 0);
                assert_eq!(b % g, 0);
                let l = lcm(a, b);
                if let Some(p) = a.checked_mul(b) {
                    assert_eq!(g.checked_mul(l).map(|v| v.abs()), Some(p.abs()));
                }
            }
        }
        assert_eq!(gcd(0, 0), 0);
        assert_eq!(gcd(-12, 8), 4);
        assert_eq!(lcm(0, 5), 0);
    }

    #[test]
    fn extgcd_satisfies_bezout() {
        let mut rng = SplitMix64::new(0xB20);
        for _ in 0..2_000 {
            let a = rng.range(-5_000, 5_001) as i64;
            let b = rng.range(-5_000, 5_001) as i64;
            if a == 0 && b == 0 {
                continue;
            }
            let (g, x, y) = extgcd(a, b);
            assert_eq!(g, gcd(a, b));
            assert_eq!(
                a.checked_mul(x).and_then(|ax| ax.checked_add(b * y)),
                Some(g)
            );
        }
    }

    #[test]
    fn mod_inv_round_trips_and_mod_pow() {
        let mut rng = SplitMix64::new(0xA11);
        for _ in 0..2_000 {
            let m = 2 + rng.below(500) as i64;
            let a = rng.range(-10_000, 10_001) as i64;
            match mod_inv(a, m) {
                Some(inv) => {
                    assert_eq!(gcd(a.rem_euclid(m), m), 1, "inv for non-coprime {a} {m}");
                    assert!((0..m).contains(&inv));
                    assert_eq!((a * inv).rem_euclid(m), 1);
                }
                None => assert!(gcd(a.rem_euclid(m), m) > 1),
            }
            let e = rng.below(64) as i64;
            let b = rng.range(-100, 101) as i64;
            // mod_pow matches naive repeated multiplication.
            let mut want = 1i64;
            for _ in 0..e {
                want = (want * b).rem_euclid(m);
            }
            assert_eq!(mod_pow(b, e, m), Some(want));
        }
        assert_eq!(mod_inv(1, 1), None);
        assert_eq!(mod_pow(2, -1, 7), None);
        assert_eq!(mod_pow(5, 3, 1), Some(0));
    }

    #[test]
    fn crt_solutions_satisfy_congruences() {
        let mut rng = SplitMix64::new(0xC87);
        let mut solved = 0;
        for _ in 0..2_000 {
            let m1 = 2 + rng.below(50) as i64;
            let m2 = 2 + rng.below(50) as i64;
            let r1 = rng.below(m1 as u32) as i64;
            let r2 = rng.below(m2 as u32) as i64;
            match crt2(r1, m1, r2, m2) {
                Some((x, mm)) => {
                    solved += 1;
                    assert_eq!(x % m1, r1, "{x} mod {m1} != {r1}");
                    assert_eq!(x % m2, r2);
                    assert_eq!(mm, m1 * m2 / gcd(m1, m2));
                    assert!((0..mm).contains(&x));
                    // Uniqueness inside [0, mm).
                    for y in 0..mm {
                        if y % m1 == r1 && y % m2 == r2 {
                            assert_eq!(y, x);
                        }
                    }
                }
                None => assert_ne!((r2 - r1) % gcd(m1, m2), 0),
            }
        }
        assert!(solved > 500, "CRT solvable too rarely ({solved})");
        // General list CRT over a known system.
        assert_eq!(crt(&[(2, 3), (3, 5), (2, 7)]), Some((23, 105)));
        assert_eq!(crt(&[]), Some((0, 1)));
        assert_eq!(crt2(2, 4, 6, 4), Some((2, 4))); // 6 ≡ 2 (mod 4)
        assert_eq!(crt2(1, 4, 2, 6), None); // inconsistent parity
    }
}
