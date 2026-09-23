//! Stirling numbers over `GF(p)` (`p = 998244353`, shared with
//! [`crate::conv`]): first kind `s1` (signed), second kind `s2`
//! (set partitions into blocks), and Bell numbers `B(n)` — each row
//! built by the `O(n·k)` triangle recurrence, so every value is a pure
//! function and the whole table is replay-safe.
//!
//! The first-kind triangle here produces *signed* values
//! `s(n,k) = s(n−1,k−1) − (n−1)·s(n−1,k)`; unsigned cycle counts
//! follow via [`us1`]. `s2(n,0)=0` for `n>0`, and both kinds obey the
//! canonical boundary `s(0,0)=1`, `s(n,0)=s(0,k)=0` for `n,k>0`.
//!
//! ```
//! use izanagi_kit::stirling::{s2, bell};
//! // S(4,2) = 7 partitions of a 4-set into two blocks
//! assert_eq!(s2(4, 2), 7);
//! // B(4) = 15 total set partitions
//! assert_eq!(bell(4), 15);
//! ```
use crate::conv::MOD;

/// Row `n` of the signed first-kind triangle: `s1(n,0..=n)`.
pub fn s1_row(n: usize) -> Vec<u64> {
    let mut row = vec![1u64];
    for i in 1..=n {
        let mut next = vec![0u64; i + 1];
        for (k, v) in next.iter_mut().enumerate() {
            let mut acc = 0u64;
            if k > 0 {
                acc = row[k - 1];
            }
            if k < row.len() {
                // −(i−1)·s(i−1,k)
                let t = (row[k] as u128 * (i as u128 - 1) % MOD as u128) as u64;
                acc = (acc + MOD - t) % MOD;
            }
            *v = acc;
        }
        row = next;
    }
    row
}

/// Signed Stirling number of the first kind `s(n,k)` — `0` when
/// `k > n`.
pub fn s1(n: usize, k: usize) -> u64 {
    if k > n {
        return 0;
    }
    s1_row(n)[k]
}

/// Unsigned Stirling first kind `|s(n,k)|` = permutations of `n` with
/// exactly `k` cycles.
pub fn us1(n: usize, k: usize) -> u64 {
    if k > n {
        return 0;
    }
    let v = s1(n, k);
    if (n - k) & 1 == 1 {
        (MOD - v) % MOD
    } else {
        v
    }
}

/// Stirling number of the second kind `S(n,k)` — partitions of an
/// `n`-set into `k` nonempty blocks. `0` when `k > n`.
pub fn s2(n: usize, k: usize) -> u64 {
    if k > n {
        return 0;
    }
    s2_row(n)[k]
}

/// Row `n` of the second-kind triangle.
pub fn s2_row(n: usize) -> Vec<u64> {
    let mut row = vec![1u64];
    for i in 1..=n {
        let mut next = vec![0u64; i + 1];
        for (k, v) in next.iter_mut().enumerate() {
            let mut acc = 0u64;
            if k > 0 {
                acc = row[k - 1];
            }
            if k < row.len() {
                acc = (acc as u128 + row[k] as u128 * k as u128 % MOD as u128) as u64;
            }
            *v = acc;
        }
        row = next;
    }
    row
}

/// Bell number `B(n)` — row sum of the second-kind triangle.
pub fn bell(n: usize) -> u64 {
    s2_row(n).iter().fold(0u64, |a, &b| (a + b) % MOD)
}

/// First-kind expansion of the falling factorial
/// `x(x−1)…(x−n+1) = Σ s(n,k) xᵏ` — coefficient list of the monic
/// degree-`n` polynomial, signed mod `p`.
pub fn falling_coeffs(n: usize) -> Vec<u64> {
    s1_row(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn modpow(mut b: u64, mut e: u64) -> u64 {
        let mut r = 1u64;
        b %= MOD;
        while e > 0 {
            if e & 1 == 1 {
                r = (r as u128 * b as u128 % MOD as u128) as u64;
            }
            b = (b as u128 * b as u128 % MOD as u128) as u64;
            e >>= 1;
        }
        r
    }
    fn modinv(a: u64) -> u64 {
        modpow(a % MOD, MOD - 2)
    }
    fn choose(n: u64, k: u64) -> u64 {
        if k > n {
            return 0;
        }
        let (mut num, mut den) = (1u64, 1u64);
        for i in 0..k {
            num = (num as u128 * (n - i) as u128 % MOD as u128) as u64;
            den = (den as u128 * (i + 1) as u128 % MOD as u128) as u64;
        }
        (num as u128 * modinv(den) as u128 % MOD as u128) as u64
    }

    /// Independent formula: `S(n,k) = (1/k!) Σ_j (−1)^j C(k,j)(k−j)^n`.
    fn s2_closed(n: usize, k: usize) -> u64 {
        if k == 0 {
            return (n == 0) as u64;
        }
        let mut sum = 0u64;
        for j in 0..=k as u64 {
            let t = (choose(k as u64, j) as u128 * modpow(k as u64 - j, n as u64) as u128
                % MOD as u128) as u64;
            if j & 1 == 1 {
                sum = (sum + MOD - t) % MOD;
            } else {
                sum = (sum + t) % MOD;
            }
        }
        (sum as u128
            * modinv({
                let mut f = 1u64;
                for i in 1..=k as u64 {
                    f = (f as u128 * i as u128 % MOD as u128) as u64;
                }
                f
            }) as u128
            % MOD as u128) as u64
    }

    #[test]
    fn second_kind_oracle() {
        for n in 0..=12usize {
            for k in 0..=n {
                assert_eq!(s2(n, k), s2_closed(n, k), "S({n},{k})");
            }
            assert_eq!(bell(n), s2_row(n).iter().fold(0u64, |a, &b| (a + b) % MOD));
        }
        // known Bell sequence prefix
        let want: [u64; 10] = [1, 1, 2, 5, 15, 52, 203, 877, 4140, 21147];
        for (n, &w) in want.iter().enumerate() {
            assert_eq!(bell(n), w);
        }
    }

    #[test]
    fn s1_row_matches_table() {
        // Row view agrees with the 2-D table and stays normalized.
        for n in 0..10 {
            let row = s1_row(n);
            for k in 0..=n + 1 {
                assert_eq!(row.get(k).copied().unwrap_or(0), s1(n, k));
            }
        }
    }

    #[test]
    fn first_kind_counts_cycles() {
        // oracle: enumerate every permutation of n, count those with
        // exactly k cycles — the unsigned Stirling number by definition
        use crate::perm::{cycles, unrank};
        for n in 0..=7usize {
            let mut fact = 1u64;
            for i in 2..=n as u64 {
                fact *= i;
            }
            let mut tally = vec![0u64; n + 1];
            for r in 0..fact {
                let p = unrank(r, n);
                tally[cycles(&p).len()] += 1;
            }
            for (k, &tk) in tally.iter().enumerate() {
                assert_eq!(us1(n, k) as u128, tk as u128, "|s({n},{k})|");
            }
        }
    }

    #[test]
    fn signed_pattern_and_boundaries() {
        assert_eq!(s1(0, 0), 1);
        assert_eq!(s1(4, 0), 0);
        assert_eq!(s2(0, 0), 1);
        assert_eq!(s2(4, 0), 0);
        assert_eq!(s2(4, 5), 0);
        // sign alternates (−1)^(n−k)
        for n in 0..8 {
            for k in 1..=n {
                let u = us1(n, k);
                let s = s1(n, k);
                let expect = if (n - k) & 1 == 1 { MOD - u } else { u } % MOD;
                assert_eq!(s, expect);
            }
        }
    }

    #[test]
    fn falling_factorial_identity() {
        let mut rng = SplitMix64::new(0x5712_11a9_b315);
        for _ in 0..200 {
            let n = rng.below(9) as usize;
            let x = rng.below(MOD as u32) as u64;
            let cf = falling_coeffs(n);
            // Σ s(n,k) xᵏ vs direct product Π_{i<n}(x−i)
            let mut sum = 0u64;
            let mut xp = 1u64;
            for &c in &cf {
                sum = (sum as u128 + c as u128 * xp as u128 % MOD as u128) as u64;
                xp = (xp as u128 * x as u128 % MOD as u128) as u64;
            }
            let mut prod = 1u64;
            for i in 0..n as u64 {
                prod = (prod as u128 * ((x + MOD - i % MOD) % MOD) as u128 % MOD as u128) as u64;
            }
            assert_eq!(sum % MOD, prod, "n={n} x={x}");
        }
    }
}
