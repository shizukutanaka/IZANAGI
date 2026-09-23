//! The Kronecker symbol `(a | n)` — the full generalization
//! of the Legendre symbol to any integers, computed by the
//! binary quadratic-reciprocity algorithm in `O(log)` steps
//! with `i128` intermediates.
//!
//! For odd prime `p` it is the Legendre symbol: `+1` when
//! `a` is a nonzero quadratic residue mod `p`, `−1` for a
//! non-residue, `0` when `p | a`. Extended to all `n` by
//! multiplicativity, and to `n ≤ 1` by the conventions
//! `(a|0) = [a = ±1]`, `(a|−1) = sign(a)`.
//!
//! ```
//! use izanagi_kit::jacobi::kronecker;
//! // 3 is a QR mod 11 (5² = 3): (3|11) = 1
//! assert_eq!(kronecker(3, 11), 1);
//! // 2 is not: (2|11) = -1
//! assert_eq!(kronecker(2, 11), -1);
//! // Jacobi over composite: (2|15) = (2|3)(2|5) = (-1)(-1) = 1
//! assert_eq!(kronecker(2, 15), 1);
//! ```
//!
//! References: the binary algorithm is the classic
//! Shallit/Cohen `kronecker` (Cohen, *A Course in
//! Computational Algebraic Number Theory*, Alg. 1.4.10);
//! the extension table for `n ∈ {−1, 0}` is standard.

/// `(a | n)` — Kronecker symbol, `-1`, `0`, or `1`.
/// Total over all `i64` inputs.
pub fn kronecker(mut a: i64, mut n: i64) -> i64 {
    // trivial extensions
    if n == 0 {
        return i64::from(a.abs() == 1);
    }
    if n == -1 {
        return if a < 0 { -1 } else { 1 };
    }
    if n == 1 {
        return 1;
    }
    // (a|2): 0 if a even, +1 if a ≡ ±1 (mod 8), −1 if a ≡ ±3 (mod 8)
    let mut t = 1i64;
    if n % 2 == 0 {
        // peel the factor 2 of n, handling sign via (a|−1)
        if n < 0 {
            if a < 0 {
                t = -t;
            }
            n = -n;
        }
        let mut k = 0u32;
        while n % 2 == 0 {
            k += 1;
            n /= 2;
        }
        // (a|2)^k
        if k > 0 {
            let r = a.rem_euclid(8);
            let two = match r {
                0 | 2 | 4 | 6 => 0,
                1 | 7 => 1,
                _ => -1, // 3, 5
            };
            if two == 0 {
                return 0;
            }
            if k % 2 == 1 {
                t *= two;
            }
        }
        if n == 1 {
            return t;
        }
        if n < 0 {
            n = -n;
        }
    } else if n < 0 {
        if a < 0 {
            t = -t;
        }
        n = -n;
    }
    // now n is odd positive; Jacobi via reciprocity
    a = a.rem_euclid(n);
    let mut n = n;
    let mut a = a;
    while a != 0 {
        // strip 2s from a: (2|n) = +1 iff n ≡ ±1 mod 8
        while a % 2 == 0 {
            a /= 2;
            let r = n.rem_euclid(8);
            if r == 3 || r == 5 {
                t = -t;
            }
        }
        // reciprocity: (a|n) = (n|a)·(−1)^((a−1)(n−1)/4)
        std::mem::swap(&mut a, &mut n);
        if a % 4 == 3 && n % 4 == 3 {
            t = -t;
        }
        a %= n;
    }
    if n == 1 {
        t
    } else {
        0
    }
}

/// Is `a` a nonzero quadratic residue mod odd prime `p`?
/// `None` when `p` is even or `≤ 2`. Convenience over
/// [`kronecker`].
pub fn is_qr(a: i64, p: i64) -> Option<bool> {
    if p <= 2 || p % 2 == 0 {
        return None;
    }
    Some(kronecker(a, p) == 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basics() {
        assert_eq!(kronecker(3, 11), 1);
        assert_eq!(kronecker(2, 11), -1);
        assert_eq!(kronecker(2, 15), 1); // (−1)·(−1)
        assert_eq!(kronecker(0, 5), 0); // p | a
        assert_eq!(kronecker(1, 0), 1);
        assert_eq!(kronecker(-1, 0), 1);
        assert_eq!(kronecker(2, 0), 0);
        assert_eq!(kronecker(-3, -1), -1);
        assert_eq!(kronecker(7, -1), 1);
        assert_eq!(kronecker(4, 9), 1); // 4 ≡ 2² mod 9
        assert_eq!(kronecker(2, 7), 1); // 3² = 2 mod 7
        assert_eq!(kronecker(3, 7), -1);
        assert_eq!(is_qr(4, 7), Some(true));
        assert_eq!(is_qr(3, 7), Some(false));
        assert_eq!(is_qr(1, 4), None);
    }

    /// Prime-modulus oracle: (a|p) must equal the Euler
    /// criterion `a^((p−1)/2) mod p ∈ {0, 1, p−1}`.
    #[test]
    fn euler_criterion() {
        let primes = crate::sieve::Primes::new(2000);
        let mut rng = SplitMix64::new(0xAC0B);
        for _ in 0..4000 {
            let p = *primes.list().get(2 + rng.below(400) as usize).unwrap_or(&7) as i64;
            if p <= 2 {
                continue;
            }
            let a = rng.below(200) as i64 - 100;
            let sym = kronecker(a, p);
            let Some(e) = crate::ntheory::mod_pow(a.rem_euclid(p), (p - 1) / 2, p) else {
                continue;
            };
            let want = if e == 0 {
                0
            } else if e == 1 {
                1
            } else {
                -1 // e == p−1
            };
            assert_eq!(sym, want, "a = {a}, p = {p}");
        }
    }

    /// Multiplicativity oracle: (a|m·n) = (a|m)·(a|n) for
    /// positive odd m, n — and (a|n) must vanish when
    /// gcd(a, n) > 1.
    #[test]
    fn multiplicative_and_vanishing() {
        let mut rng = SplitMix64::new(0xA71E);
        for _ in 0..3000 {
            let m = (rng.below(60) as i64) * 2 + 1;
            let n = (rng.below(60) as i64) * 2 + 1;
            let a = rng.below(200) as i64 - 100;
            assert_eq!(
                kronecker(a, m * n),
                kronecker(a, m) * kronecker(a, n),
                "a = {a}, m = {m}, n = {n}"
            );
            if crate::ntheory::gcd(a, n) > 1 {
                assert_eq!(kronecker(a, n), 0);
            }
        }
    }
}
