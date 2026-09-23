//! Shamir's (k, n) threshold secret sharing over GF(p).
//!
//! The secret is the constant term of a random degree
//! `k − 1` polynomial over a prime field:
//!
//! ```text
//! share(x_i) = s + a_1·x + a_2·x² + … + a_{k−1}·x^{k−1}  mod p
//! ```
//!
//! - `split(secret, k, n, prime, seed)` → `n` shares
//!   `(x, y)`, one per participant id `1..=n`. The
//!   coefficients come from `SplitMix64` seeded with
//!   `(seed, prime, k)` — the split is a *pure function*:
//!   same inputs always produce the same share set, so a
//!   test can re-derive coefficients.
//! - `reconstruct(&shares, prime)` → the secret, by
//!   Lagrange interpolation at `x = 0`. Any `k` distinct
//!   shares suffice; `k − 1` shares are *perfectly*
//!   information-hiding over a prime field.
//!
//! `None` results (rather than panics) signal invalid
//! parameters: `k < 1`, `k > n`, `n ≥ prime`, non-prime
//! `p`, duplicate x-coordinates, or secret ≥ p.
//!
//! ```
//! use izanagi_kit::shamir::{reconstruct, split};
//! let shares = split(1234, 3, 5, 7919, 42).unwrap();
//! assert_eq!(shares.len(), 5);
//! // any 3 reconstruct
//! assert_eq!(reconstruct(&shares[..3], 7919), Some(1234));
//! assert_eq!(
//!     reconstruct(&[shares[0], shares[2], shares[4]], 7919),
//!     Some(1234)
//! );
//! // 2 shares leak nothing useful — they can't even
//! // distinguish s=1234 from s=0
//! ```

/// Horner evaluation of the degree-`k−1` polynomial.
fn eval_poly(coeffs: &[u64], x: u64, p: u64) -> u64 {
    let mut acc: u128 = 0;
    for &c in coeffs.iter().rev() {
        acc = (acc * u128::from(x) + u128::from(c)) % u128::from(p);
    }
    acc as u64
}

/// `a·b mod p` via u128 — fits any `p ≤ u64::MAX`.
fn mul(a: u64, b: u64, p: u64) -> u64 {
    ((a as u128 * b as u128) % p as u128) as u64
}

/// `a^−1 mod p` by Fermat (p must be prime — the caller's
/// contract; `split` verifies with a deterministic
/// Miller–Rabin).
fn inv(a: u64, p: u64) -> Option<u64> {
    if a == 0 || a >= p {
        return None;
    }
    // a^(p−2) mod p
    let mut base = a;
    let mut e = p - 2;
    let mut acc = 1u64;
    while e > 0 {
        if e & 1 == 1 {
            acc = mul(acc, base, p);
        }
        base = mul(base, base, p);
        e >>= 1;
    }
    Some(acc)
}

/// Deterministic Miller–Rabin for the supplied prime —
/// 8 fixed bases suffice well below 2^64/3 for our use
/// (shares are toy-sized; the caller supplies the prime so
/// we must vet it).
fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    for &small in &[2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        if n == small {
            return true;
        }
        if n % small == 0 {
            return false;
        }
    }
    let d = n - 1;
    let s = d.trailing_zeros();
    let dd = d >> s;
    for &a in &[2u64, 3, 5, 7, 11, 13, 17, 23] {
        if a >= n {
            continue;
        }
        let mut x = {
            let mut base = a;
            let mut e = dd;
            let mut acc = 1u64;
            while e > 0 {
                if e & 1 == 1 {
                    acc = mul(acc, base, n);
                }
                base = mul(base, base, n);
                e >>= 1;
            }
            acc
        };
        if x == 1 || x == n - 1 {
            continue;
        }
        let mut witness = false;
        for _ in 1..s {
            x = mul(x, x, n);
            if x == n - 1 {
                witness = true;
                break;
            }
        }
        if !witness {
            return false;
        }
    }
    true
}

/// Split `secret` into `n` shares, `k` needed to recover.
///
/// Shares carry x-coordinates `1..=n` — the index doubles
/// as the evaluation point, which is the standard trick to
/// make share ids unforgeable-free metadata.
pub fn split(secret: u64, k: usize, n: usize, prime: u64, seed: u64) -> Option<Vec<(u64, u64)>> {
    if k == 0 || k > n || n == 0 || secret >= prime {
        return None;
    }
    if !is_prime(prime) {
        return None;
    }
    if n as u64 >= prime {
        return None;
    }
    let mut rng = crate::rng::SplitMix64::new(seed ^ (prime << 1) ^ (k as u64));
    let mut coeffs = Vec::with_capacity(k);
    coeffs.push(secret);
    for _ in 1..k {
        coeffs.push(rng.next_u64() % prime);
    }
    let mut shares = Vec::with_capacity(n);
    for x in 1..=n as u64 {
        shares.push((x, eval_poly(&coeffs, x, prime)));
    }
    Some(shares)
}

/// Reconstruct the secret from `k` (or more) shares.
///
/// Lagrange at `x = 0`: `s = Σ y_i · Π_{j≠i} (−x_j) /
/// (x_i − x_j)`. Returns `None` on duplicate x's or an
/// x out of field.
pub fn reconstruct(shares: &[(u64, u64)], prime: u64) -> Option<u64> {
    if shares.is_empty() || !is_prime(prime) {
        return None;
    }
    let mut seen = std::collections::BTreeSet::new();
    for &(x, _) in shares {
        if x == 0 || x >= prime || !seen.insert(x) {
            return None;
        }
    }
    let mut acc = 0u64;
    for i in 0..shares.len() {
        let (xi, yi) = shares[i];
        // numerator Π_{j≠i} (−x_j) — evaluated at 0 the
        // product term is (0 − x_j) = p − x_j
        let mut num = 1u64;
        let mut den = 1u64;
        for (j, &(xj, _)) in shares.iter().enumerate() {
            if i == j {
                continue;
            }
            num = mul(num, prime - xj, prime);
            // (x_i − x_j) mod p
            let diff = if xi >= xj { xi - xj } else { prime - (xj - xi) };
            den = mul(den, diff, prime);
        }
        let term = mul(yi, mul(num, inv(den, prime)?, prime), prime);
        acc = ((acc as u128 + term as u128) % prime as u128) as u64;
    }
    Some(acc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    /// Hand-checked: the classic toy example s=1234,
    /// k=3, n=6, p=1613 from Shamir's own examples in many
    /// references.
    #[test]
    fn basics() {
        // p must be prime; 1613 is prime
        let shares = split(1234, 3, 6, 1613, 7).unwrap();
        assert_eq!(shares.len(), 6);
        assert_eq!(reconstruct(&shares[..3], 1613), Some(1234));
        // reject: k=0, k>n, secret>=p, non-prime, n>=p
        assert_eq!(split(1, 0, 5, 1613, 7), None);
        assert_eq!(split(1, 6, 5, 1613, 7), None);
        assert_eq!(split(1613, 2, 5, 1613, 7), None);
        assert_eq!(split(1, 2, 5, 1614, 7), None); // 1614 composite
        assert_eq!(split(1, 2, 2000, 1613, 7), None); // n >= p
                                                      // duplicate x rejected
        assert_eq!(reconstruct(&[(1, 5), (1, 9)], 1613), None);
        // x=0 rejected (x=0 reveals the secret directly)
        assert_eq!(reconstruct(&[(0, 42)], 1613), None);
    }

    /// k−1 shares must give NOTHING — with Shamir over a
    /// prime field each k−1 subset is consistent with every
    /// possible secret. Check this *statistically*: a
    /// k−1 share subset for two different secrets produces
    /// overlapping compatible-value sets (always, by theory),
    /// so instead verify the stronger statement that
    /// *reconstruction under-shares* is undefined — i.e. a
    /// k−1 "reconstruct" only recovers something different
    /// from the real secret in >90% of seeds, but never
    /// errors. (The real guarantee is information-theoretic;
    /// we test that the API doesn't accidentally leak via a
    /// degenerate polynomial.)
    #[test]
    fn fewer_than_k_never_recovers() {
        for seed in 0..50u64 {
            let s = 1000 + seed;
            let shares = split(s, 3, 5, 7919, seed).unwrap();
            // 2-of-3 subset: result must be a defined field
            // element but essentially never == s (p−1 / p odds)
            let r = reconstruct(&shares[..2], 7919);
            assert!(r.is_some());
            if r == Some(s) {
                // probability ~1/p per attempt; a hit here is
                // a signal the polynomial degenerated
                panic!("k−1 leaked the secret at seed {seed}");
            }
        }
    }

    /// Shadow oracle: pick a random distinct k-subset each
    /// round and verify it reconstructs. Also verify share
    /// determinism (same seed → identical shares) and that
    /// x_i are exactly 1..=n.
    #[test]
    fn oracle_random_subsets() {
        let mut rng = SplitMix64::new(0x5EED);
        for _ in 0..200 {
            let p = [7919u64, 104729, 1299709][rng.below(3) as usize];
            let k = 2 + rng.below(4) as usize;
            let n = k + rng.below(5) as usize;
            let secret = rng.next_u64() % p;
            let seed = rng.next_u64();
            let shares = split(secret, k, n, p, seed).unwrap();
            // deterministic
            assert_eq!(shares, split(secret, k, n, p, seed).unwrap());
            // x's are 1..=n
            let xs: BTreeSet<u64> = shares.iter().map(|s| s.0).collect();
            assert_eq!(xs.len(), n);
            // a random k-subset reconstructs
            let mut idx: Vec<usize> = (0..n).collect();
            for i in (1..n).rev() {
                let j = rng.below(i as u32 + 1) as usize;
                idx.swap(i, j);
            }
            let sub: Vec<(u64, u64)> = idx[..k].iter().map(|&i| shares[i]).collect();
            assert_eq!(reconstruct(&sub, p), Some(secret), "p {p} k {k}");
            // a swapped share body changes the answer —
            // the k points then lie on a different
            // interpolating polynomial
            if k >= 2 {
                let mut bad = sub.clone();
                bad[0].1 = (bad[0].1 + 1) % p;
                assert_ne!(reconstruct(&bad, p), Some(secret));
            }
        }
    }

    /// Any *superset* of k shares also reconstructs.
    #[test]
    fn more_than_k_works() {
        let shares = split(777, 2, 8, 7919, 99).unwrap();
        for take in 2..=8 {
            assert_eq!(reconstruct(&shares[..take], 7919), Some(777));
        }
    }

    /// A share from a *different* split injected into a
    /// reconstruction must not be silently accepted — the
    /// outcome is a wrong secret, which the caller detects
    /// by an outer checksum. Verify it does change the
    /// result (it must: shares are points on different
    /// polynomials).
    #[test]
    fn foreign_share_corrupts() {
        let a = split(111, 2, 3, 7919, 1).unwrap();
        let b = split(222, 2, 3, 7919, 2).unwrap();
        let mixed = [a[0], b[1]];
        let r = reconstruct(&mixed, 7919);
        assert!(r.is_some() && r != Some(111) && r != Some(222));
    }

    /// Field boundaries: prime just above the values.
    #[test]
    fn boundary_values() {
        // secret == p−1 (max)
        let shares = split(7918, 2, 4, 7919, 5).unwrap();
        assert_eq!(reconstruct(&shares[..2], 7919), Some(7918));
        // secret == 0
        let shares0 = split(0, 2, 4, 7919, 5).unwrap();
        assert_eq!(reconstruct(&shares0[..2], 7919), Some(0));
        // k = n (everyone needed)
        let all = split(42, 5, 5, 7919, 5).unwrap();
        assert_eq!(reconstruct(&all, 7919), Some(42));
    }
}
