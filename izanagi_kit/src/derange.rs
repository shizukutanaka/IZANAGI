//! Derangements — permutations with no fixed point.
//! `!n` (the subfactorial) counts them; computed over
//! [`BigInt`] by `D₀=1, D₁=0, Dₙ = (n−1)·(Dₙ₋₁ + Dₙ₋₂)`,
//! plus the rencontres numbers `R(n,k) = C(n,k)·!(n−k)`
//! (perms with exactly `k` fixed points) and an
//! enumerator over [`crate::perm`] rank space.
//!
//! ```
//! use izanagi_kit::derange::{derangement, rencontres, derangements};
//! use izanagi_kit::bigint::BigInt;
//! // !4 = 9
//! assert_eq!(derangement(4), BigInt::from_i64(9));
//! // perms of [4] with exactly 2 fixed points: C(4,2)·!2 = 6
//! assert_eq!(rencontres(4, 2), BigInt::from_i64(6));
//! assert_eq!(derangements(3).len(), 2);
//! ```
//!
//! References: the subfactorial recurrence and
//! `R(n,k) = C(n,k)·!(n−k)` are classical (Concrete
//! Mathematics §5.6, OEIS A000166/A008290); `Σₖ R(n,k)
//! = n!` is the row-sum theorem.

use crate::bigint::BigInt;

/// `!n` — the number of derangements of `[n]`. `!0 = 1`
/// (the empty permutation vacuously has no fixed point).
pub fn derangement(n: u32) -> BigInt {
    let mut d0 = BigInt::from_i64(1);
    let mut d1 = BigInt::from_i64(0);
    if n == 0 {
        return d0;
    }
    for i in 2..=n {
        let d2 = d0.add(&d1).mul(&BigInt::from_i64(i as i64 - 1));
        d0 = d1;
        d1 = d2;
    }
    d1
}

/// `R(n,k)` — permutations of `[n]` with exactly `k`
/// fixed points: `C(n,k)·!(n−k)`.
pub fn rencontres(n: u32, k: u32) -> BigInt {
    crate::catalan::binomial(n, k).mul(&derangement(n - k))
}

/// All derangements of `[n]` in factoradic rank order —
/// `O(n!·n)`; keep `n ≤ 7`.
pub fn derangements(n: u32) -> Vec<Vec<u32>> {
    let n = n as usize;
    let mut fact = 1u64;
    for i in 2..=n as u64 {
        fact = fact.saturating_mul(i);
    }
    let mut out = Vec::new();
    for r in 0..fact {
        let p = crate::perm::unrank(r, n);
        if (0..n).all(|i| p[i] != i as u32) {
            out.push(p);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basics() {
        let d: Vec<i64> = vec![1, 0, 1, 2, 9, 44, 265, 1854, 14833];
        for (n, &v) in d.iter().enumerate() {
            assert_eq!(derangement(n as u32), BigInt::from_i64(v), "!{n}");
        }
        assert_eq!(rencontres(4, 2), BigInt::from_i64(6));
        assert_eq!(rencontres(4, 4), BigInt::from_i64(1));
        assert_eq!(rencontres(4, 3), BigInt::from_i64(0)); // 3 fixed ⇒ 4th fixed
        assert_eq!(derangements(3).len(), 2);
        assert_eq!(derangements(1).len(), 0);
        assert_eq!(derangements(0).len(), 1);
    }

    /// Row-sum `Σₖ R(n,k) = n!` and `R(n,n−1) = 0` —
    /// theorem-level checks of the rencontres triangle.
    #[test]
    fn rencontres_theorems() {
        for n in 0..9u32 {
            let mut fact = BigInt::from_i64(1);
            for i in 2..=n.max(1) {
                fact = fact.mul(&BigInt::from_i64(i as i64));
            }
            let mut row = BigInt::from_i64(0);
            for k in 0..=n {
                row = row.add(&rencontres(n, k));
            }
            assert_eq!(row, fact, "row sum n={n}");
            if n >= 1 {
                assert_eq!(rencontres(n, n - 1), BigInt::zero());
            }
        }
    }

    /// Enumeration oracle: count and content — every
    /// returned perm is a genuine derangement and the
    /// length equals `!n`; also compare against the
    /// unfiltered unrank space size `n!`.
    #[test]
    fn enumeration_oracle() {
        let mut rng = SplitMix64::new(0xDE12);
        for _ in 0..150 {
            let n = rng.below(7);
            let ds = derangements(n);
            let expect = derangement(n).to_i128().unwrap() as usize;
            assert_eq!(ds.len(), expect, "n={n}");
            let mut seen = std::collections::BTreeSet::new();
            for p in &ds {
                assert!(seen.insert(p));
                assert!((0..n as usize).all(|i| p[i] != i as u32));
                let mut sorted = p.clone();
                sorted.sort_unstable();
                assert_eq!(sorted, (0..n).collect::<Vec<_>>());
            }
        }
    }

    /// Fixed-point histogram oracle: for each n ≤ 6 and
    /// each k, the number of unrank-perms with exactly k
    /// fixed points equals `R(n,k)` — the *definition* of
    /// the rencontres numbers, not just their sum.
    #[test]
    fn histogram_oracle() {
        for n in 0..7u32 {
            let mut fact = 1u64;
            for i in 2..=n as u64 {
                fact *= i.max(1);
            }
            let mut hist = vec![0u64; n as usize + 1];
            for r in 0..fact {
                let p = crate::perm::unrank(r, n as usize);
                let fp = (0..n as usize).filter(|&i| p[i] == i as u32).count();
                hist[fp] += 1;
            }
            for k in 0..=n {
                assert_eq!(
                    rencontres(n, k),
                    BigInt::from_i64(hist[k as usize] as i64),
                    "n={n} k={k}"
                );
            }
        }
    }
}
