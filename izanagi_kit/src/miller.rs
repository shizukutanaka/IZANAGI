//! Deterministic primality and factorization for `u64`.
//!
//! `is_prime` runs the Miller–Rabin strong-probable-prime test with
//! the seven fixed bases proved deterministic for all `n < 2^64`
//! (Jaeschke 1993 / Sinclair's base set `{2, 325, 9375, 28178, 450775,
//! 9780504, 1795265022}`) — no randomness, no floating point, so the
//! answer is bit-exact everywhere. `factor` decomposes via trial
//! division plus Brent's rho variant with a fixed polynomial schedule,
//! returning sorted prime factors.
//!
//! ```
//! use izanagi_kit::miller::{factor, is_prime};
//! assert!(is_prime(1_000_000_007));
//! assert!(is_prime(1_000_000_009)); // prime too — the pair around 10^9
//! assert!(!is_prime(1_000_000_011)); // 3 * 333333337
//! assert_eq!(factor(360), vec![2, 2, 2, 3, 3, 5]);
//! ```

/// The deterministic base set for `u64` Miller–Rabin.
const BASES: [u64; 7] = [2, 325, 9375, 28178, 450775, 9780504, 1795265022];

fn mul_mod(a: u64, b: u64, m: u64) -> u64 {
    ((a as u128 * b as u128) % m as u128) as u64
}

fn pow_mod(mut b: u64, mut e: u64, m: u64) -> u64 {
    let mut acc = 1u64;
    while e > 0 {
        if e & 1 == 1 {
            acc = mul_mod(acc, b, m);
        }
        b = mul_mod(b, b, m);
        e >>= 1;
    }
    acc
}

/// Exact primality for every `u64`. Handles even numbers, small
/// primes, and Carmichael-pseudoprime traps via the full base set.
pub fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    // Trial division by the first few primes short-circuits most
    // composites and covers every base < n boundary cleanly.
    for p in [2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        if n == p {
            return true;
        }
        if n % p == 0 {
            return false;
        }
    }
    // n-1 = d * 2^s
    let mut d = n - 1;
    let mut s = 0u32;
    while d & 1 == 0 {
        d >>= 1;
        s += 1;
    }
    'outer: for &a in &BASES {
        let a = a % n;
        if a == 0 {
            continue;
        }
        let mut x = pow_mod(a, d, n);
        if x == 1 || x == n - 1 {
            continue 'outer;
        }
        for _ in 1..s {
            x = mul_mod(x, x, n);
            if x == n - 1 {
                continue 'outer;
            }
        }
        return false;
    }
    true
}

fn gcd_u64(a: u64, b: u64) -> u64 {
    let (mut a, mut b) = (a, b);
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// Brent's rho cycle-finding factor search. The polynomial schedule
/// `f(x) = x*x + c` advances `c` deterministically on each retry, so
/// the sequence of divisors found is a pure function of `n`.
fn brent_rho(n: u64) -> u64 {
    let mut c: u64 = 1;
    loop {
        let mut y: u64 = 2;
        let mut r: u64 = 1;
        let mut q: u64 = 1;
        let mut g: u64 = 1;
        const M: u64 = 128;
        while g == 1 {
            let x = y;
            for _ in 0..r {
                y = (mul_mod(y, y, n) + c) % n;
            }
            let mut k = 0u64;
            while k < r && g == 1 {
                for _ in 0..M.min(r - k) {
                    y = (mul_mod(y, y, n) + c) % n;
                    q = mul_mod(q, x.abs_diff(y), n);
                }
                g = gcd_u64(q, n);
                k += M;
            }
            r <<= 1;
            if g == n {
                // Backtrack through the last block.
                loop {
                    y = (mul_mod(y, y, n) + c) % n;
                    g = gcd_u64(x.abs_diff(y), n);
                    if g > 1 {
                        break;
                    }
                }
            }
        }
        if g != n {
            return g;
        }
        c += 1;
    }
}

/// Sorted prime factorization of `n` (with multiplicity). `factor(0)`
/// and `factor(1)` are empty; `factor(p)` for prime `p` is `[p]`.
///
/// Trial division removes every factor ≤ 1000, then Brent's rho
/// splits the remaining cofactor; recursion bottoms out on primes
/// certified by [`is_prime`].
pub fn factor(mut n: u64) -> Vec<u64> {
    let mut out = Vec::new();
    if n < 2 {
        return out;
    }
    // Trial division up to 1000 covers most inputs outright and keeps
    // rho calls rare.
    let mut d = 2u64;
    while d <= 1000 && d * d <= n {
        while n % d == 0 {
            out.push(d);
            n /= d;
        }
        d += if d == 2 { 1 } else { 2 };
    }
    if n == 1 {
        return out;
    }
    let mut stack = vec![n];
    while let Some(m) = stack.pop() {
        if is_prime(m) {
            out.push(m);
            continue;
        }
        let g = brent_rho(m);
        stack.push(g);
        stack.push(m / g);
    }
    out.sort_unstable();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Eratosthenes sieve oracle up to LIMIT.
    fn sieve() -> Vec<bool> {
        const N: usize = 200_000;
        let mut p = vec![true; N];
        p[0] = false;
        p[1] = false;
        for i in 2..N {
            if p[i] {
                let mut j = i * i;
                while j < N {
                    p[j] = false;
                    j += i;
                }
            }
        }
        p
    }

    #[test]
    fn is_prime_matches_sieve_below_200k() {
        let s = sieve();
        for n in 0..s.len() as u64 {
            assert_eq!(is_prime(n), s[n as usize], "{n}");
        }
    }

    #[test]
    fn is_prime_handles_large_values() {
        assert!(is_prime(u64::MAX - 58)); // 2^64 - 59, largest u64 prime
        assert!(is_prime(1_000_000_007));
        assert!(!is_prime(9_999_999_999_999_999_989)); // 10^19 - 11
        assert!(!is_prime(u64::MAX)); // 3 * 5 * 17 * 257 * ... product
        assert!(!is_prime(1_000_000_000_000_000_000));
    }

    #[test]
    fn strong_pseudoprimes_are_rejected() {
        // Carmichael numbers and base-2/3 SPSPs must all fail.
        for n in [
            561u64,
            1105,
            1729,
            41041,
            2047,          // 2-SPSP
            1373653,       // 2,3-SPSP
            25326001,      // 2,3,5-SPSP
            3215031751,    // 2,3,5,7-SPSP
            2152302898747, // first 5-prime SPSP
            3474749660383, // 6-prime SPSP
        ] {
            assert!(!is_prime(n), "{n} must be composite");
        }
        // And primes just above them still pass.
        assert!(is_prime(3215031767));
    }

    #[test]
    fn factor_is_sorted_prime_and_products_match() {
        let mut rng = SplitMix64::new(0xFAC7);
        for _ in 0..300 {
            let n = rng.next_u64() % 10_000_000 + 2;
            let f = factor(n);
            assert_eq!(f.iter().product::<u64>(), n, "{n}");
            assert!(f.windows(2).all(|w| w[0] <= w[1]), "{n}");
            assert!(f.iter().all(|&p| is_prime(p)), "{n}");
        }
        assert!(factor(1).is_empty());
        assert!(factor(0).is_empty());
        assert_eq!(factor(2), vec![2]);
    }

    #[test]
    fn factor_splits_semiprimes() {
        // Product of two ~1e9 primes.
        let (p, q) = (1_000_000_007u64, 999_999_937u64);
        assert_eq!(factor(p * q), vec![999_999_937, 1_000_000_007]);
        // Square of a prime.
        assert_eq!(factor(97 * 97), vec![97, 97]);
        // Highly composite.
        assert_eq!(factor(720), vec![2, 2, 2, 2, 3, 3, 5]);
    }

    #[test]
    fn factor_is_deterministic_across_calls() {
        let a = factor(999_999_999_999_999);
        let b = factor(999_999_999_999_999);
        assert_eq!(a, b);
    }
}
