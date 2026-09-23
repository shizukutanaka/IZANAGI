//! Eulerian numbers `⟨n k⟩` — the number of permutations
//! of `[n]` with exactly `k` descents (positions `i` with
//! `π(i) > π(i+1)`). Computed by the recurrence
//! `⟨n k⟩ = (n−k)·⟨n−1 k−1⟩ + (k+1)·⟨n−1 k⟩` over
//! [`BigInt`], plus `permutations(n,k)` which enumerates
//! the permutations themselves via [`crate::perm`] rank
//! space.
//!
//! ```
//! use izanagi_kit::eulerian::{eulerian, permutations};
//! use izanagi_kit::bigint::BigInt;
//! // ⟨4 1⟩ = 11
//! assert_eq!(eulerian(4, 1), BigInt::from_i64(11));
//! // the 11 perms of [4] with one descent
//! assert_eq!(permutations(4, 1).len(), 11);
//! ```
//!
//! References: the recurrence and the descent-count
//! interpretation are classical (Concrete Mathematics
//! §6.2, OEIS A008292); `Σₖ ⟨n k⟩ = n!` is the row-sum
//! theorem.

use crate::bigint::BigInt;

/// `⟨n k⟩` — Eulerian number, exact `BigInt`.
/// `⟨0 0⟩ = 1`; out-of-range `k` gives `0` for `n > 0`.
pub fn eulerian(n: u32, k: u32) -> BigInt {
    let n = n as usize;
    let mut prev = vec![BigInt::from_i64(0); n + 1];
    prev[0] = BigInt::from_i64(1);
    for i in 1..=n {
        let mut cur = vec![BigInt::from_i64(0); n + 1];
        for j in 0..=i.min(n - 1) {
            // ⟨i j⟩ = (i−j)·⟨i−1 j−1⟩ + (j+1)·⟨i−1 j⟩
            let mut v = prev[j].mul(&BigInt::from_i64((j + 1) as i64));
            if j > 0 {
                v = v.add(&prev[j - 1].mul(&BigInt::from_i64((i - j) as i64)));
            }
            cur[j] = v;
        }
        prev = cur;
    }
    if k as usize >= prev.len() {
        return BigInt::from_i64(0);
    }
    prev[k as usize].clone()
}

/// All permutations of `[n]` with exactly `k` descents,
/// in factoradic rank order — `O(n! · n)`; intended for
/// small `n` (enumeration is exponential in output size).
/// Descents counted as positions `i` with `π(i) > π(i+1)`.
pub fn permutations(n: u32, k: u32) -> Vec<Vec<u32>> {
    let n = n as usize;
    let mut fact = 1u64;
    for i in 2..=n as u64 {
        fact = fact.saturating_mul(i);
    }
    let mut out = Vec::new();
    for r in 0..fact {
        let p = crate::perm::unrank(r, n);
        let desc = (0..n.saturating_sub(1))
            .filter(|&i| p[i] > p[i + 1])
            .count() as u32;
        if desc == k {
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
        assert_eq!(eulerian(0, 0), BigInt::from_i64(1));
        assert_eq!(eulerian(4, 1), BigInt::from_i64(11));
        assert_eq!(eulerian(5, 2), BigInt::from_i64(66));
        assert_eq!(eulerian(7, 3), BigInt::from_i64(2416));
        assert_eq!(eulerian(1, 0), BigInt::from_i64(1));
        assert_eq!(eulerian(1, 1), BigInt::from_i64(0));
        assert_eq!(permutations(4, 1).len(), 11);
        assert_eq!(permutations(3, 0), vec![vec![0, 1, 2]]);
        assert_eq!(permutations(3, 2), vec![vec![2, 1, 0]]);
    }

    /// Row-sum theorem `Σₖ ⟨n k⟩ = n!` and the Worpitzky
    /// identity `xⁿ = Σₖ ⟨n k⟩·C(x+k, n)` at small x —
    /// two theorem-level oracles, not just tabulated rows.
    #[test]
    fn theorems() {
        for n in 0..10u32 {
            let mut fact = BigInt::from_i64(1);
            for i in 2..=n.max(1) {
                fact = fact.mul(&BigInt::from_i64(i as i64));
            }
            let mut row = BigInt::from_i64(0);
            for k in 0..n.max(1) {
                row = row.add(&eulerian(n, k));
            }
            assert_eq!(row, fact, "row sum n={n}");
        }
        // Worpitzky: x^n = Σ_k ⟨n k⟩·C(x+k, n)
        for n in 1..8u32 {
            for x in 1..5u64 {
                let mut sum = BigInt::from_i64(0);
                for k in 0..n {
                    let a = eulerian(n, k);
                    // C(x+k, n) as an exact u128 chain
                    let mut c = 1u128;
                    for j in 0..n as u128 {
                        // j > x+k ⇒ factor 0 ⇒ C = 0, no underflow
                        c = c * (x as u128 + k as u128).saturating_sub(j) / (j + 1);
                    }
                    sum = sum.add(&a.mul(&BigInt::from_i128(c as i128)));
                }
                let mut xn = BigInt::from_i64(1);
                for _ in 0..n {
                    xn = xn.mul(&BigInt::from_i64(x as i64));
                }
                assert_eq!(sum, xn, "worpitzky n={n} x={x}");
            }
        }
    }

    /// Enumeration oracle: `permutations(n,k)` length must
    /// equal `⟨n k⟩`, and every returned perm really has
    /// `k` descents — an end-to-end check that the
    /// counting triangle and the descent semantics agree.
    #[test]
    fn enumeration_oracle() {
        let mut rng = SplitMix64::new(0xE37A);
        for _ in 0..200 {
            let n = 1 + rng.below(7);
            let k = rng.below(n);
            let ps = permutations(n, k);
            let expect = eulerian(n, k).to_i128().unwrap() as usize;
            assert_eq!(ps.len(), expect, "n={n} k={k}");
            for p in &ps {
                let d = (0..n as usize - 1).filter(|&i| p[i] > p[i + 1]).count();
                assert_eq!(d as u32, k);
                let mut sorted = p.clone();
                sorted.sort_unstable();
                assert_eq!(sorted, (0..n).collect::<Vec<_>>());
            }
        }
    }
}
