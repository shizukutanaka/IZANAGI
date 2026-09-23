//! The Möbius function `μ` and Dirichlet convolution —
//! inclusion–exclusion over the divisor lattice, computed
//! by a linear sieve and exact `i128` convolution loops.
//!
//! `μ(n) = 0` when `n` has a squared prime factor, else
//! `(−1)^k` for `k` distinct primes. Dirichlet convolution
//! `(f ∗ g)(n) = Σ_{d|n} f(d)·g(n/d)` makes the divisor
//! sum an algebra: `f ∗ μ` is the exact inverse of the
//! divisor summation `g(n) = Σ_{d|n} f(d)` — Möbius
//! inversion over the integers.
//!
//! ```
//! use izanagi_kit::mobius::{mu_sieve, convolve, invert, sigma_sum};
//! let mu = mu_sieve(10);
//! assert_eq!(&mu[1..6], &[1, -1, -1, 0, -1]); // 1,2,3,4,5
//! // g(n) = Σ_{d|n} d is σ(n); convolve with μ returns d
//! let sig = sigma_sum(10);
//! let f = convolve(&sig, &mu.iter().map(|&x| i128::from(x)).collect::<Vec<_>>());
//! assert_eq!(&f[1..6], &[1, 2, 3, 4, 5]);
//! let _ = invert;
//! ```
//!
//! References: the linear sieve is the standard
//! `lp[i]·i ≤ n` construction (Library-Checker / ACL
//! style); Dirichlet inversion is Hardy & Wright §16.

/// `μ(0), μ(1), …, μ(n)` via the linear sieve — every
/// `n > 0` is touched exactly once by its least prime
/// factor. `mu[0] = 0` by convention.
pub fn mu_sieve(n: usize) -> Vec<i64> {
    let mut mu = vec![0i64; n + 1];
    if n == 0 {
        return mu;
    }
    mu[1] = 1;
    let mut lp = vec![0usize; n + 1]; // least prime factor
    let mut primes: Vec<usize> = Vec::new();
    for i in 2..=n {
        if lp[i] == 0 {
            lp[i] = i;
            mu[i] = -1;
            primes.push(i);
        }
        for &p in &primes {
            let v = i * p;
            if v > n {
                break;
            }
            lp[v] = p;
            if p == lp[i] {
                // i already carries p — v is squareful
                mu[v] = 0;
                break;
            }
            mu[v] = -mu[i];
        }
    }
    mu
}

/// `μ(v)` for a single value — factors via the SPF table
/// (`spf` must cover `v`, e.g. from
/// [`sieve::spf_sieve`](crate::sieve::spf_sieve)).
pub fn mu(v: u64, spf: &[u64]) -> i64 {
    let f = crate::sieve::factor_map(spf, v);
    if f.values().any(|&e| e > 1) {
        0
    } else if f.len() % 2 == 0 {
        1
    } else {
        -1
    }
}

/// Dirichlet convolution `(f ∗ g)(n)` over indices
/// `0..=n` — `O(n log n)` divisor-enumeration loops.
/// Index 0 of both inputs is ignored (0 has infinitely
/// many divisors); `f` and `g` must have equal length.
pub fn convolve(f: &[i128], g: &[i128]) -> Vec<i128> {
    let n = f.len().min(g.len()).saturating_sub(1);
    let mut out = vec![0i128; n + 1];
    for d in 1..=n {
        let mut m = d;
        while m <= n {
            out[m] += f[d] * g[m / d];
            m += d;
        }
    }
    out
}

/// Möbius inversion: given `g(n) = Σ_{d|n} f(d)` (its
/// length `n+1`), recovers `f = g ∗ μ`. This is just
/// [`convolve`] against `mu_sieve` — the identity
/// `f ∗ (1 ∗ μ) = f ∗ ε = f` makes it exact.
pub fn invert(g: &[i128]) -> Vec<i128> {
    if g.is_empty() {
        return Vec::new();
    }
    let mu = mu_sieve(g.len() - 1);
    let mu128: Vec<i128> = mu.iter().map(|&x| i128::from(x)).collect();
    convolve(g, &mu128)
}

/// `σ(n) = Σ_{d|n} d` for `1..=n` — a convenient
/// convolution partner and oracle reference.
pub fn sigma_sum(n: usize) -> Vec<i128> {
    let mut s = vec![0i128; n + 1];
    for d in 1..=n {
        let mut m = d;
        while m <= n {
            s[m] += d as i128;
            m += d;
        }
    }
    s
}

/// Number of integers in `1..=n` coprime to each `k` —
/// the classic Möbius inclusion–exclusion count
/// `Σ_{d|k} μ(d)·⌊n/d⌋`. `None` when `k = 0` or
/// `spf` doesn't cover `k`.
pub fn coprime_count(n: u64, k: u64, spf: &[u64]) -> Option<u64> {
    if k == 0 || spf.len() <= k as usize {
        return None;
    }
    let f = crate::sieve::factor_map(spf, k);
    let primes: Vec<u64> = f.keys().copied().collect();
    let mut total = 0i128;
    for mask in 0u32..(1u32 << primes.len()) {
        let mut d = 1u64;
        let mut bits = 0u32;
        for (i, &p) in primes.iter().enumerate() {
            if mask >> i & 1 == 1 {
                d *= p;
                bits += 1;
            }
        }
        let term = (n / d) as i128;
        total += if bits % 2 == 0 { term } else { -term };
    }
    u64::try_from(total).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basics() {
        let tbl = mu_sieve(20);
        // 1..10: 1,−1,−1,0,−1,1,−1,0,0,1
        assert_eq!(&tbl[1..=10], &[1, -1, -1, 0, -1, 1, -1, 0, 0, 1]);
        assert_eq!(tbl.len(), 21);
        assert_eq!(mu_sieve(0), vec![0]);
        let spf = crate::sieve::spf_sieve(64);
        assert_eq!(mu(30, &spf), -1); // 2·3·5 → k=3 → −1
        assert_eq!(mu(12, &spf), 0); // 2² factor → 0
        assert_eq!(mu(1, &spf), 1);
        let s = sigma_sum(12);
        assert_eq!(s[6], 12); // 1+2+3+6
        assert_eq!(s[12], 28); // 1+2+3+4+6+12
        assert_eq!(coprime_count(10, 6, &spf), Some(3)); // 1,5,7
        assert_eq!(coprime_count(10, 0, &spf), None);
    }

    /// Möbius inversion oracle: for random `f`, build
    /// `g(n) = Σ_{d|n} f(d)` then recover `f` exactly.
    #[test]
    fn inversion_roundtrip() {
        let mut rng = SplitMix64::new(0xBE11);
        for _ in 0..200 {
            let n = 1 + rng.below(40) as usize;
            let f: Vec<i128> = (0..=n).map(|_| i128::from(rng.below(100)) - 50).collect();
            // g = f ∗ 1
            let one = vec![1i128; n + 1];
            let g = convolve(&f, &one);
            // direct divisor-sum reference for g
            for (v, gv) in g.iter().enumerate().take(n + 1).skip(1) {
                let want: i128 = (1..=v).filter(|d| v % d == 0).map(|d| f[d]).sum();
                assert_eq!(*gv, want);
            }
            let back = invert(&g);
            assert_eq!(&back[1..], &f[1..], "inversion must recover f");
        }
    }

    /// μ sieve vs single-value `mu` + the identity
    /// `Σ_{d|n} μ(d) = [n == 1]` — the fundamental lemma
    /// that makes inversion work.
    #[test]
    fn sieve_matches_single() {
        let n = 256usize;
        let spf = crate::sieve::spf_sieve(n);
        let sieve = mu_sieve(n);
        for v in 1..=n as u64 {
            assert_eq!(mu(v, &spf), sieve[v as usize], "v = {v}");
        }
        // Σ_{d|n} μ(d) = ε(n) = [n == 1]
        for v in 1..=n {
            let s: i64 = (1..=v).filter(|d| v % d == 0).map(|d| sieve[d]).sum();
            assert_eq!(s, i64::from(v == 1), "v = {v}");
        }
        // coprime_count oracle: direct gcd enumeration
        let mut rng = SplitMix64::new(0xC0FF);
        for _ in 0..500 {
            let n = 1 + u64::from(rng.below(60));
            let k = 1 + u64::from(rng.below(50));
            if k as usize >= spf.len() {
                continue;
            }
            let want = (1..=n)
                .filter(|&x| crate::ntheory::gcd(x as i64, k as i64) == 1)
                .count() as u64;
            assert_eq!(coprime_count(n, k, &spf), Some(want));
        }
    }
}
