//! Integer partitions — `p(n)` counted two ways and the
//! canonical enumeration of partitions themselves.
//!
//! [`count`] uses Euler's pentagonal recurrence
//! `p(n) = Σ_{k≠0} (−1)^{k+1}·p(n − g_k)` over generalized
//! pentagonals `g_k = k(3k−1)/2`, which is `O(n^{3/2})`;
//! [`count_bounded`] is the `p(n, m)` triangle
//! `p(n,m) = p(n−1, m−1) + p(n−m, m)` counting partitions
//! whose largest part is `m` — the shadow oracle.
//! [`enumerate`] produces every non-increasing partition of
//! `n` in lexicographic order, and `d_part` restricts to
//! partitions into `d` distinct parts. Counts use [`BigInt`]:
//! `p(200) ≈ 4×10¹²` overflows `u64`.
//!
//! ```
//! use izanagi_kit::partitions::{count, enumerate};
//! use izanagi_kit::bigint::BigInt;
//! assert_eq!(count(6), BigInt::from_i64(11));
//! let parts = enumerate(5).unwrap();
//! assert_eq!(parts.len(), 7);
//! assert_eq!(parts[0], vec![5]); // descending lex order
//! assert_eq!(parts[6], vec![1, 1, 1, 1, 1]);
//! ```
//!
//! References: Euler's recurrence (Andrews, *The Theory of
//! Partitions* §1.3); `p(6)=11`, `p(10)=42` standard values.

use crate::bigint::BigInt;

/// `p(n)` — the partition number via Euler's pentagonal
/// recurrence. Exact at any `n` (arbitrary precision).
pub fn count(n: u32) -> BigInt {
    let n = n as usize;
    // memo[k] = p(k)
    let mut memo: Vec<BigInt> = vec![BigInt::from_i64(1)];
    for t in 1..=n {
        let mut acc = BigInt::from_i64(0);
        let mut sign = true; // (−1)^{k+1}: k=1,2 positive
        let mut k = 1i64;
        loop {
            // generalized pentagonals for ±k: k(3k−1)/2 and k(3k+1)/2
            for g in [k * (3 * k - 1) / 2, k * (3 * k + 1) / 2] {
                if g as usize > t {
                    break;
                }
                let term = &memo[t - g as usize];
                acc = if sign { acc.add(term) } else { acc.sub(term) };
            }
            if (k * (3 * k + 1) / 2) as usize > t {
                break;
            }
            k += 1;
            sign = !sign;
        }
        memo.push(acc);
    }
    memo[n].clone()
}

/// `p(n, m)` — partitions of `n` whose largest part is
/// exactly `m` (`m ≥ 1`, `n ≥ m`). Partitions into *at
/// most* `m` parts equal partitions with largest part `m` by
/// conjugation; `Σ_m p(n,m) = p(n)`.
///
/// `p(n, m) = p(n−1, m−1) + p(n−m, m)` — either the
/// partition has a part `1` (remove it: `p(n−1,m−1)` under
/// conjugation bookkeeping) or every part ≥ 2 (subtract 1
/// from each: `p(n−m, m)`).
pub fn count_bounded(n: u32, m: u32) -> BigInt {
    if m == 0 || n < m {
        return BigInt::from_i64(0);
    }
    let n = n as usize;
    let m = m as usize;
    // tab[i][j] = p(i, j)
    let mut tab: Vec<Vec<BigInt>> = vec![vec![BigInt::from_i64(0); m + 1]; n + 1];
    tab[0][0] = BigInt::from_i64(1); // empty partition of 0
    for i in 1..=n {
        for j in 1..=m.min(i) {
            let a = tab[i - 1][j - 1].clone();
            let b = if i >= j {
                tab[i - j][j].clone()
            } else {
                BigInt::from_i64(0)
            };
            tab[i][j] = a.add(&b);
        }
    }
    tab[n][m].clone()
}

/// Partitions of `n` with all parts distinct — `q(n)`.
/// `q(n) = p(n with distinct parts)` via the same
/// recurrence with the "largest part" argument tracking
/// strictness: `q(n, m) = q(n−m, m−1) + q(n, m−1)` (use `m`
/// or skip it).
pub fn count_distinct(n: u32) -> BigInt {
    let n = n as usize;
    // qn[i][j] = partitions of i into distinct parts ≤ j
    let mut qn: Vec<Vec<BigInt>> = vec![vec![BigInt::from_i64(0); n + 1]; n + 1];
    for v in qn[0].iter_mut() {
        *v = BigInt::from_i64(1);
    }
    for j in 1..=n {
        for i in 1..=n {
            // skip part j, or use it (remaining parts < j)
            let mut acc = qn[i][j - 1].clone();
            if i >= j {
                acc = acc.add(&qn[i - j][j - 1]);
            }
            qn[i][j] = acc;
        }
    }
    qn[n][n].clone()
}

/// All partitions of `n` as non-increasing vectors in
/// descending-lexicographic order — `[n]` first, `[1,…,1]` last.
/// `None` when `n` is too large to enumerate honestly
/// (`p(100) ≈ 1.9×10⁸` — capped at `n ≤ 80`, still heavy;
/// callers needing counts use [`count`]).
pub fn enumerate(n: u32) -> Option<Vec<Vec<u32>>> {
    if n > 80 {
        return None;
    }
    if n == 0 {
        return Some(vec![Vec::new()]);
    }
    let mut out = Vec::new();
    // start at [n], the first partition in descending-lex
    // order; the walk ends at [1,…,1]
    let mut cur = vec![n];
    loop {
        out.push(cur.clone());
        // standard "last non-1 part, split tail" successor
        // find rightmost part > 1
        let mut i = cur.len();
        let mut tail = 0u32;
        while i > 0 {
            if cur[i - 1] > 1 {
                break;
            }
            tail += cur[i - 1];
            i -= 1;
        }
        if i == 0 {
            return Some(out);
        }
        // cur[i-1] is the rightmost part >1: reduce it and
        // redistribute (its 1 + collected tail)
        cur[i - 1] -= 1;
        let unit = cur[i - 1];
        let mut pool = tail + 1;
        cur.truncate(i);
        while pool > 0 {
            let take = pool.min(unit);
            cur.push(take);
            pool -= take;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basics() {
        // OEIS A000041 head
        let vals = [1u64, 1, 2, 3, 5, 7, 11, 15, 22, 30, 42];
        for (i, &v) in vals.iter().enumerate() {
            assert_eq!(count(i as u32), BigInt::from_i128(i128::from(v)));
        }
        assert_eq!(count(10), BigInt::from_i64(42));
        assert_eq!(count(50), BigInt::from_i64(204_226));
        // p(100) = 190,569,292 — beyond u32, checks BigInt path
        assert_eq!(count(100), BigInt::from_i64(190_569_292));
    }

    /// `Σ_m p(n,m) = p(n)` and the DP agrees with pentagonal
    /// counts — the two recurrences shadow each other.
    #[test]
    fn pentagonal_vs_bounded() {
        for n in 0..40u32 {
            let mut s = BigInt::from_i64(0);
            for m in 1..=n {
                s = s.add(&count_bounded(n, m));
            }
            if n == 0 {
                assert_eq!(count(0), BigInt::from_i64(1));
            } else {
                assert_eq!(s, count(n));
            }
            // p(n,1) = 1 always; p(n,n) = 1
            if n > 0 {
                assert_eq!(count_bounded(n, 1), BigInt::from_i64(1));
                assert_eq!(count_bounded(n, n), BigInt::from_i64(1));
            }
        }
        // Euler's theorem: distinct parts = odd parts
        for n in 0..20u32 {
            let mut odd = BigInt::from_i64(0);
            // count partitions with all parts odd via the
            // bounded triangle restricted to odd m… simpler:
            // enumerate for small n and check directly.
            if n <= 12 {
                let parts = enumerate(n).unwrap();
                let d = parts
                    .iter()
                    .filter(|p| p.iter().all(|&x| x % 2 == 1))
                    .count();
                odd = BigInt::from_i64(d as i64);
                let dist = parts
                    .iter()
                    .filter(|p| {
                        let mut seen = std::collections::BTreeSet::new();
                        p.iter().all(|&x| seen.insert(x))
                    })
                    .count();
                assert_eq!(BigInt::from_i64(dist as i64), count_distinct(n));
                assert_eq!(BigInt::from_i64(dist as i64), odd);
            }
            let _ = odd;
        }
    }

    /// The enumeration is exhaustive, lex-ordered, and each
    /// member is a genuine partition.
    #[test]
    fn enumerate_is_canonical() {
        let mut rng = SplitMix64::new(0xA817);
        for _ in 0..30 {
            let n = rng.below(15);
            let parts = enumerate(n).unwrap();
            let cnt = count(n);
            assert_eq!(BigInt::from_i64(parts.len() as i64), cnt);
            let mut sorted_ok = true;
            for w in parts.windows(2) {
                if w[0] < w[1] {
                    sorted_ok = false;
                }
            }
            assert!(sorted_ok);
            for p in &parts {
                assert_eq!(p.iter().sum::<u32>(), n);
                for w in p.windows(2) {
                    assert!(w[0] >= w[1]);
                }
            }
        }
        // n=0 → the single empty partition
        assert_eq!(enumerate(0), Some(vec![Vec::new()]));
        assert!(enumerate(81).is_none());
    }
}
