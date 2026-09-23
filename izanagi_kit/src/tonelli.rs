//! Tonelli–Shanks — square roots modulo an odd prime.
//!
//! `sqrt_mod(a, p)` returns the two roots of `x² ≡ a (mod p)`
//! sorted `(lo, hi)` where `hi = p − lo`, or `None` when `a` is
//! a quadratic non-residue. The search for a non-residue `z`
//! scans `2, 3, …` sequentially, so the result depends only
//! on `(a, p)` — no randomness.
//!
//! The procedure itself is the classic one: write
//! `p − 1 = q·2ˢ` with `q` odd, seed
//! `r = a^((q+1)/2)`, `t = a^q`, `c = z^q`, then shrink the
//! 2-adic order of `t` by squaring `c` in. Each round halves
//! `m`, so the loop is `O(s²)` group multiplications.
//!
//! ```
//! use izanagi_kit::tonelli::sqrt_mod;
//! let (lo, hi) = sqrt_mod(10, 13).unwrap();
//! assert_eq!((lo, hi), (6, 7)); // 6² = 36 ≡ 10 (mod 13)
//! assert!(sqrt_mod(5, 13).is_none()); // no root exists
//! ```
//!
//! References: Tonelli (1891) / Shanks (1973), cp-algorithms
//! "Tonelli–Shanks algorithm". `p` is required to be an odd
//! prime — feed composites at your own risk (the algorithm's
//! group-order invariants only hold in `𝔽ₚ`).

/// Modular multiply — `a·b mod p` in `u128` (`p < 2^63`).
fn mul_mod(a: u64, b: u64, p: u64) -> u64 {
    ((a as u128 * b as u128) % p as u128) as u64
}

fn pow_mod(mut a: u64, mut e: u64, p: u64) -> u64 {
    let mut r = 1u64 % p;
    a %= p;
    while e > 0 {
        if e & 1 == 1 {
            r = mul_mod(r, a, p);
        }
        a = mul_mod(a, a, p);
        e >>= 1;
    }
    r
}

/// Euler's criterion — `a` is a quadratic residue mod odd
/// prime `p` iff `a^((p−1)/2) ≡ 1`. Also true for `a ≡ 0`,
/// which returns `true` here (0 has the root 0).
pub fn is_quadratic_residue(a: u64, p: u64) -> bool {
    if p == 2 {
        return true;
    }
    pow_mod(a % p, (p - 1) / 2, p) <= 1
}

/// The two square roots of `a` modulo odd prime `p`, sorted
/// `(lo, hi)` with `lo + hi = p`. `None` if `a` is a
/// non-residue. `sqrt_mod(0, p)` is `Some((0, 0))`.
pub fn sqrt_mod(a: u64, p: u64) -> Option<(u64, u64)> {
    if p == 2 {
        let r = a % 2;
        return Some((r, r));
    }
    if p < 3 {
        return None;
    }
    let a = a % p;
    if a == 0 {
        return Some((0, 0));
    }
    if !is_quadratic_residue(a, p) {
        return None;
    }
    // p − 1 = q · 2^s, q odd
    let mut q = p - 1;
    let mut s = 0u32;
    while q & 1 == 0 {
        q >>= 1;
        s += 1;
    }
    if s == 1 {
        // p ≡ 3 (mod 4) — the direct formula
        let r = pow_mod(a, (p + 1) / 4, p);
        return Some(order(r, p));
    }
    // least non-residue — deterministic scan
    let mut z = 2u64;
    while pow_mod(z, (p - 1) / 2, p) != p - 1 {
        z += 1;
    }
    let mut m = s;
    let mut c = pow_mod(z, q, p);
    let mut t = pow_mod(a, q, p);
    let mut r = pow_mod(a, q.div_ceil(2), p);
    while t != 1 {
        // least i in [1, m) with t^(2^i) = 1 — always exists
        // while t is a 2-power-order residue
        let mut i = 1u32;
        let mut u = mul_mod(t, t, p);
        while u != 1 {
            u = mul_mod(u, u, p);
            i += 1;
        }
        // b = c^(2^(m − i − 1))
        let mut b = c;
        for _ in 0..(m - i - 1) {
            b = mul_mod(b, b, p);
        }
        r = mul_mod(r, b, p);
        c = mul_mod(b, b, p);
        t = mul_mod(t, c, p);
        m = i;
    }
    Some(order(r, p))
}

fn order(r: u64, p: u64) -> (u64, u64) {
    let other = if r == 0 { 0 } else { p - r };
    if r <= other {
        (r, other)
    } else {
        (other, r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn brute_sqrt(a: u64, p: u64) -> Option<(u64, u64)> {
        let mut r0 = None;
        for x in 0..p {
            if mul_mod(x, x, p) == a % p {
                r0 = Some(x);
                break;
            }
        }
        r0.map(|lo| {
            let mut hi = lo;
            for x in (lo + 1)..p {
                if mul_mod(x, x, p) == a % p {
                    hi = x;
                }
            }
            (lo, hi)
        })
    }

    fn is_prime(p: u64) -> bool {
        if p < 2 {
            return false;
        }
        let mut d = 2;
        while d * d <= p {
            if p % d == 0 {
                return false;
            }
            d += 1;
        }
        true
    }

    #[test]
    fn known_vectors() {
        assert_eq!(sqrt_mod(10, 13), Some((6, 7)));
        assert_eq!(sqrt_mod(4, 13), Some((2, 11)));
        assert_eq!(sqrt_mod(1, 13), Some((1, 12)));
        assert_eq!(sqrt_mod(0, 13), Some((0, 0)));
        assert_eq!(sqrt_mod(5, 13), None);
        // p ≡ 3 (mod 4) shortcut — 7, 11, 19
        assert_eq!(sqrt_mod(4, 7), Some((2, 5)));
        assert_eq!(sqrt_mod(9, 11), Some((3, 8)));
        // p ≡ 1 (mod 4) with large 2-adic factor: 17 → q=1,s=4
        assert_eq!(sqrt_mod(4, 17), Some((2, 15)));
        assert_eq!(sqrt_mod(13, 17), Some((8, 9))); // 8²=64≡13
                                                    // p = 2 edge
        assert_eq!(sqrt_mod(0, 2), Some((0, 0)));
        assert_eq!(sqrt_mod(1, 2), Some((1, 1)));
        assert_eq!(sqrt_mod(15, 2), Some((1, 1)));
    }

    #[test]
    fn brute_oracle_small_primes() {
        // exhaustive: every residue of every odd prime < 200
        for p in 3..200u64 {
            if !is_prime(p) {
                continue;
            }
            for a in 0..p {
                assert_eq!(sqrt_mod(a, p), brute_sqrt(a, p), "a={a} p={p}");
            }
        }
    }

    #[test]
    fn roundtrip_larger_primes() {
        // Mersenne-ish and NTT-friendly primes
        let primes = [
            1009u64,
            10007,
            99991,
            65537,
            998244353,  // 119·2^23 + 1 — huge 2-adic factor
            4294967291, // largest prime < 2^32
            4611686018427387847,
        ];
        let mut rng = SplitMix64::new(29);
        for &p in &primes {
            assert!(crate::miller::is_prime(p), "{p}");
            assert!(is_quadratic_residue(1, p));
            for _ in 0..400 {
                let x = rng.next_u64() % p;
                let a = mul_mod(x, x, p);
                let (lo, hi) = sqrt_mod(a, p).expect("x² must be a residue");
                // roots are complements: lo = p − hi (0 is its own pair)
                assert!(lo <= hi && (lo + hi) % p == 0);
                assert!(x == lo || x == hi, "x={x} p={p} roots=({lo},{hi})");
                assert_eq!(mul_mod(lo, lo, p), a);
                assert_eq!(mul_mod(hi, hi, p), a);
            }
        }
    }

    #[test]
    fn nonresidues_agree() {
        // Legendre symbol vs brute classification on p = 997
        let p = 997u64;
        for a in 0..p {
            let want = brute_sqrt(a, p).is_some();
            assert_eq!(is_quadratic_residue(a, p), want, "a={a}");
            assert_eq!(sqrt_mod(a, p).is_some(), want, "a={a}");
        }
    }
}
