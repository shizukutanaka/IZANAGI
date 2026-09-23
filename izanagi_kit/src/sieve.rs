//! Prime sieves: the linear-time smallest-prime-factor sieve and
//! the segmented sieve for sparse windows — all integer, all
//! deterministic.
//!
//! [`spf_sieve`] builds the `spf[i]` table in `O(n)`: every
//! composite `i` is marked exactly once, by its *smallest* prime
//! factor — which is what makes the table enough to factor any
//! `i ≤ n` in `O(log i)` by peeling one prime at a time. For
//! large windows far from zero, [`primes_between`] sieves only
//! the `[lo, hi]` segment using base primes up to `√hi` — no
//! table the size of `hi` is ever allocated.
//!
//! ```
//! use izanagi_kit::sieve::{spf_sieve, factor, primes_between, Primes};
//! let spf = spf_sieve(30);
//! assert_eq!(factor(&spf, 12), vec![2, 2, 3]);
//! assert_eq!(primes_between(90, 100), vec![97]);
//! let p = Primes::new(20);
//! assert_eq!(p.list(), &[2, 3, 5, 7, 11, 13, 17, 19]);
//! ```
use std::collections::BTreeMap;

/// Smallest-prime-factor table for `0..=n` in `O(n)`.
///
/// `spf[i]` is the least prime dividing `i`; `spf[0] = spf[1] = 0`
/// and `spf[p] = p` on primes. The classic linear sieve: for each
/// `i` it marks `i·p` for primes `p` *up to `spf[i]`* — every
/// composite is marked exactly once, by its least prime factor.
pub fn spf_sieve(n: usize) -> Vec<u64> {
    let mut spf = vec![0u64; n + 1];
    let mut primes: Vec<u64> = Vec::new();
    for i in 2..=n {
        if spf[i] == 0 {
            spf[i] = i as u64;
            primes.push(i as u64);
        }
        for &p in &primes {
            let ip = i as u64 * p;
            if ip > n as u64 || p > spf[i] {
                break;
            }
            spf[ip as usize] = p;
        }
    }
    spf
}

/// Sorted prime factors of `v` with multiplicity — `v` must be
/// `≤ spf.len() − 1`. Each peel takes one `spf` hit, so this is
/// `O(log v)`.
pub fn factor(spf: &[u64], mut v: u64) -> Vec<u64> {
    let mut out = Vec::new();
    while v > 1 {
        let p = spf[v as usize];
        // v > 1 inside the table is never unmarked, but an
        // out-of-range input yields p == 0 — refuse silently
        // by reporting the residue itself as the last factor.
        if p == 0 {
            out.push(v);
            break;
        }
        out.push(p);
        v /= p;
    }
    out
}

/// `true` iff `v ≤ spf.len() − 1` is prime.
pub fn is_prime_table(spf: &[u64], v: u64) -> bool {
    v >= 2 && (v as usize) < spf.len() && spf[v as usize] == v
}

/// Multiplicity map `prime → exponent` of `v`'s factorization.
pub fn factor_map(spf: &[u64], v: u64) -> BTreeMap<u64, u32> {
    let mut m = BTreeMap::new();
    for p in factor(spf, v) {
        *m.entry(p).or_insert(0) += 1;
    }
    m
}

/// All primes `≤ n` — Eratosthenes over a `bool` scratch.
pub fn primes_up_to(n: usize) -> Vec<u64> {
    if n < 2 {
        return Vec::new();
    }
    let mut composite = vec![false; n + 1];
    let mut out = Vec::new();
    for i in 2..=n {
        if !composite[i] {
            out.push(i as u64);
            let mut m = i * i;
            while m <= n {
                composite[m] = true;
                m += i;
            }
        }
    }
    out
}

/// A reusable prime table for queries: the sorted list plus its
/// `spf` table, so `factor`/`is_prime`/`phi`/`tau`/`sigma` are
/// all `O(log v)`.
#[derive(Clone, Debug)]
pub struct Primes {
    list: Vec<u64>,
    spf: Vec<u64>,
}

impl Primes {
    /// Build tables covering `0..=n`.
    pub fn new(n: usize) -> Primes {
        let spf = spf_sieve(n);
        let list = primes_up_to(n);
        Primes { list, spf }
    }

    /// The sorted prime list.
    pub fn list(&self) -> &[u64] {
        &self.list
    }

    /// Upper bound the tables cover.
    pub fn bound(&self) -> usize {
        self.spf.len() - 1
    }

    /// `v` prime? (within the table)
    pub fn is_prime(&self, v: u64) -> bool {
        is_prime_table(&self.spf, v)
    }

    /// Sorted factors of `v` with multiplicity.
    pub fn factor(&self, v: u64) -> Vec<u64> {
        factor(&self.spf, v)
    }

    /// Euler's totient `φ(v)`: `v·∏(1 − 1/p)` over distinct `p|v`.
    /// Computed multiplicatively from the `spf` run of equal
    /// factors — no float, no reciprocal.
    pub fn phi(&self, v: u64) -> u64 {
        if v == 0 {
            return 0;
        }
        let fs = factor(&self.spf, v);
        let mut out = v;
        let mut prev = 0;
        for p in fs {
            if p != prev {
                out = out / p * (p - 1);
                prev = p;
            }
        }
        out
    }

    /// Number of divisors `τ(v)` from exponents `∏(e+1)`.
    pub fn tau(&self, v: u64) -> u64 {
        if v == 0 {
            return 0;
        }
        let mut out = 1u64;
        for e in factor_map(&self.spf, v).values() {
            out *= *e as u64 + 1;
        }
        out
    }

    /// Sum of divisors `σ(v)` — per-prime geometric series.
    pub fn sigma(&self, v: u64) -> u64 {
        if v == 0 {
            return 0;
        }
        let mut out = 1u64;
        for (p, e) in factor_map(&self.spf, v) {
            // (p^{e+1} − 1)/(p − 1) — summed directly
            let mut term = 1u64;
            let mut pow = 1u64;
            for _ in 0..e {
                pow *= p;
                term += pow;
            }
            out *= term;
        }
        out
    }
}

/// Primes in `[lo, hi]` by segmented sieving — memory is
/// `O(√hi + hi − lo)`, never `O(hi)`. Both ends inclusive;
/// `lo > hi` yields empty.
///
/// For each base prime `p ≤ √hi` the first multiple inside the
/// segment is `⌈lo/p⌉·p` (or `p²` when that's larger — multiples
/// below `p²` were already crossed by a smaller prime).
pub fn primes_between(lo: u64, hi: u64) -> Vec<u64> {
    if hi < 2 || lo > hi {
        return Vec::new();
    }
    let lo = lo.max(2);
    // base primes up to floor(√hi) — the loop below uses i² ≤ hi
    let mut base: Vec<u64> = Vec::new();
    let mut i = 2u64;
    while i * i <= hi {
        if crate::miller::is_prime(i) {
            base.push(i);
        }
        i += 1;
    }
    let span = (hi - lo + 1) as usize;
    let mut composite = vec![false; span];
    for &p in &base {
        // first multiple of p ≥ max(lo, p·p)
        let mut m = lo.div_ceil(p) * p;
        if m < p * p {
            m = p * p;
        }
        while m <= hi {
            composite[(m - lo) as usize] = true;
            m += p;
        }
    }
    (0..span)
        .filter(|&k| !composite[k])
        .map(|k| lo + k as u64)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn trial_factor(v: u64) -> Vec<u64> {
        let mut out = Vec::new();
        let mut n = v;
        let mut p = 2u64;
        while p * p <= n {
            while n % p == 0 {
                out.push(p);
                n /= p;
            }
            p += 1;
        }
        if n > 1 {
            out.push(n);
        }
        out
    }

    fn trial_phi(v: u64) -> u64 {
        if v == 0 {
            return 0;
        }
        (1..=v).filter(|&k| gcd(k, v) == 1).count() as u64
    }

    fn gcd(a: u64, b: u64) -> u64 {
        if b == 0 {
            a
        } else {
            gcd(b, a % b)
        }
    }

    #[test]
    fn basics() {
        let spf = spf_sieve(30);
        assert_eq!(factor(&spf, 12), vec![2, 2, 3]);
        assert_eq!(factor(&spf, 1), Vec::<u64>::new());
        assert!(is_prime_table(&spf, 29) && !is_prime_table(&spf, 28));
        assert_eq!(primes_up_to(10), vec![2, 3, 5, 7]);
        assert_eq!(primes_up_to(1), Vec::<u64>::new());
        let p = Primes::new(20);
        assert_eq!(p.list(), &[2, 3, 5, 7, 11, 13, 17, 19]);
        assert_eq!(p.bound(), 20);
        assert!(p.is_prime(13) && !p.is_prime(15));
        assert_eq!(p.phi(9), 6);
        assert_eq!(p.tau(12), 6);
        assert_eq!(p.sigma(6), 12);
        assert_eq!(factor_map(&spf, 12), BTreeMap::from([(2, 2), (3, 1)]));
    }

    #[test]
    fn oracle_factor_and_arith_fns() {
        let n = 4096;
        let p = Primes::new(n);
        for v in 0..=n as u64 {
            assert_eq!(p.factor(v), trial_factor(v), "factor {v}");
            // product of factors reconstructs v
            let prod: u64 = p.factor(v).iter().product();
            assert_eq!(if v == 0 { 1 } else { prod }, if v == 0 { 1 } else { v });
            assert_eq!(p.phi(v), trial_phi(v), "phi {v}");
            let tau_want = (1..=v).filter(|&d| d != 0 && v % d == 0).count() as u64;
            assert_eq!(p.tau(v), tau_want, "tau {v}");
            let sigma_want: u64 = (1..=v).filter(|&d| d != 0 && v % d == 0).sum();
            assert_eq!(p.sigma(v), sigma_want, "sigma {v}");
            assert_eq!(p.is_prime(v), trial_factor(v).len() == 1 && v > 1);
        }
    }

    #[test]
    fn oracle_segmented() {
        let mut rng = SplitMix64::new(11);
        for _ in 0..60 {
            let lo = rng.below(20000) as u64;
            let hi = lo + rng.below(3000) as u64;
            let want: Vec<u64> = (lo..=hi).filter(|&v| crate::miller::is_prime(v)).collect();
            assert_eq!(primes_between(lo, hi), want, "lo={lo} hi={hi}");
        }
        // edge: hi < 2, lo > hi, segment spanning base-prime region
        assert_eq!(primes_between(0, 1), Vec::<u64>::new());
        assert_eq!(primes_between(5, 3), Vec::<u64>::new());
        assert_eq!(primes_between(2, 3), vec![2, 3]);
        // large window far from zero — miller is the oracle
        assert_eq!(
            primes_between(1_000_000, 1_000_100),
            (1_000_000u64..=1_000_100)
                .filter(|&v| crate::miller::is_prime(v))
                .collect::<Vec<_>>()
        );
    }
}
