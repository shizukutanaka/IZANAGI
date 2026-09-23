//! Zeckendorf representation — every positive integer is
//! a unique sum of non-consecutive Fibonacci numbers
//! (`1, 2, 3, 5, 8, …` — the `F₂` indexing with no
//! duplicated `1`). The greedy "take the largest fib
//! ≤ n" loop produces it, and [`is_zeckendorf`] verifies
//! canonicality (descending fibs, no two consecutive).
//!
//! ```
//! use izanagi_kit::zeckendorf::{zeckendorf, decode, is_zeckendorf};
//! // 10 = 8 + 2
//! assert_eq!(zeckendorf(10), vec![8, 2]);
//! assert_eq!(decode(&[8, 2]), 10);
//! assert!(is_zeckendorf(&[8, 2]));
//! assert!(!is_zeckendorf(&[5, 3])); // consecutive fibs
//! ```
//!
//! References: Zeckendorf's theorem (1972) — existence
//! and uniqueness; the greedy construction is the
//! standard proof (Lekkerkerker 1952). Related structures:
//! Fibonacci coding and the Fibonacci word fractal.

/// Fibonacci numbers `1, 2, 3, 5, …` up to `u64` — the
/// `F₂=1, F₃=2` sequence (single `1`, as Zeckendorf
/// requires).
fn fibs_up_to(n: u64) -> Vec<u64> {
    let mut fs = vec![1u64, 2];
    while let Some(&last) = fs.last() {
        let Some(next) = last.checked_add(fs[fs.len() - 2]) else {
            break;
        };
        if next > n {
            break;
        }
        fs.push(next);
    }
    fs
}

/// The Zeckendorf representation of `n` — descending
/// non-consecutive Fibonacci values; `n = 0` gives `[]`.
/// Greedy: the largest fib `≤ n` is always in the sum
/// (Zeckendorf's theorem), so repeat on the remainder.
pub fn zeckendorf(n: u64) -> Vec<u64> {
    let fs = fibs_up_to(n);
    let mut rem = n;
    let mut out = Vec::new();
    while rem > 0 {
        // largest fib ≤ rem
        let i = fs.partition_point(|&f| f <= rem) - 1;
        out.push(fs[i]);
        rem -= fs[i];
    }
    out
}

/// Sum a Zeckendorf-style summand list — does *not*
/// check canonicality (use [`is_zeckendorf`]).
pub fn decode(ds: &[u64]) -> u64 {
    ds.iter().copied().sum()
}

/// Is `ds` a valid Zeckendorf representation? Strictly
/// descending, every entry a Fibonacci number (`1,2,3,…`
/// sequence), and no two consecutive Fibs — empty list
/// is valid (`n = 0`).
pub fn is_zeckendorf(ds: &[u64]) -> bool {
    let fs = fibs_up_to(u64::MAX);
    let idx = |v: u64| -> Option<usize> {
        let i = fs.partition_point(|&f| f <= v);
        if i > 0 && fs[i - 1] == v {
            Some(i - 1)
        } else {
            None
        }
    };
    let mut prev_idx: Option<usize> = None;
    for &d in ds {
        let Some(i) = idx(d) else { return false };
        if let Some(p) = prev_idx {
            if !(i < p && i + 1 < p) {
                return false;
            }
        }
        prev_idx = Some(i);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn basics() {
        assert_eq!(zeckendorf(10), vec![8, 2]);
        assert_eq!(zeckendorf(1), vec![1]);
        assert_eq!(zeckendorf(2), vec![2]);
        assert_eq!(zeckendorf(100), vec![89, 8, 3]);
        assert_eq!(zeckendorf(0), Vec::<u64>::new());
        assert_eq!(decode(&[8, 2]), 10);
        assert!(is_zeckendorf(&[8, 2]));
        assert!(is_zeckendorf(&[]));
        assert!(!is_zeckendorf(&[5, 3]));
        assert!(!is_zeckendorf(&[4])); // not a fib
        assert!(!is_zeckendorf(&[2, 8])); // not descending
        assert!(!is_zeckendorf(&[8, 8])); // repeated
    }

    /// Round-trip + canonicality for a wide range — the
    /// greedy must produce *the* valid representation.
    #[test]
    fn roundtrip() {
        for n in 1..20000u64 {
            let ds = zeckendorf(n);
            assert_eq!(decode(&ds), n, "n={n}");
            assert!(is_zeckendorf(&ds), "n={n} ds={ds:?}");
        }
    }

    /// Uniqueness oracle: for small n enumerate EVERY
    /// non-adjacent-index subset of fibs ≤ n — exactly
    /// one sums to n, and it is the greedy output
    /// (Zeckendorf's uniqueness theorem, not just the
    /// existence half).
    #[test]
    fn uniqueness_oracle() {
        let mut rng = SplitMix64::new(0x7EC1);
        for _ in 0..300 {
            let n = 1 + u64::from(rng.below(200));
            let fs = fibs_up_to(n);
            let m = fs.len(); // ≤ 16 for n ≤ 201
            let mut sols = 0u64;
            let mut sol = Vec::new();
            for mask in 0..(1u64 << m) {
                if mask & (mask << 1) != 0 {
                    continue; // adjacent indices
                }
                let sum: u64 = fs
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| mask >> i & 1 == 1)
                    .map(|(_, &f)| f)
                    .sum();
                if sum == n {
                    sols += 1;
                    sol = fs
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| mask >> i & 1 == 1)
                        .map(|(_, &f)| f)
                        .collect();
                }
            }
            assert_eq!(sols, 1, "n={n} must have a unique representation");
            sol.reverse(); // descending
            assert_eq!(zeckendorf(n), sol, "n={n}");
        }
    }

    /// The greedy head is the largest fib ≤ n, and
    /// `zeck(n − head)` is the representation of the
    /// remainder — the structural recursion that makes
    /// the loop correct, checked over a dense range.
    #[test]
    fn greedy_head() {
        for n in 1..2000u64 {
            let ds = zeckendorf(n);
            let fs = fibs_up_to(n);
            // fibs_up_to may hold one overshoot; the true
            // largest ≤ n sits at the partition point
            let want = fs[fs.partition_point(|&f| f <= n) - 1];
            assert_eq!(ds[0], want, "n={n}");
            assert_eq!(zeckendorf(n - ds[0]), ds[1..].to_vec(), "n={n}");
        }
    }
}
