//! Digit DP — counting integers `x ∈ [0, n]` by decimal
//! digit properties without enumerating them: the
//! classic tight-bound DP walking the digit string
//! MSD→LSD with a `started` flag so leading zeros are
//! *not* digits of the number.
//!
//! ```
//! use izanagi_kit::digit::{count_digit_sum, count_avoid_digit};
//! // x ∈ [0,100] with digit-sum = 1: {1, 10, 100} → 3
//! assert_eq!(count_digit_sum(100, 1), 3);
//! // [0,100] avoiding digit '5': 100 − 19 = 81? brute says 81
//! assert_eq!(count_avoid_digit(100, 5), 82);
//! ```
//!
//! References: digit DP is the standard "count ≤ N with
//! digit property" technique (Qiita/Zenn digit-DP
//! write-ups, cf. EDPC-adjacent material); the `started`
//! state handling — leading zeros contribute no digit —
//! is the subtlety that makes the counts exact rather
//! than off-by-one.

/// Number of `x ∈ [0, n]` whose decimal digits avoid
/// `d`. `0`'s representation counts as the digit `0`, so
/// `d = 0` excludes `x = 0` too.
pub fn count_avoid_digit(n: u64, d: u8) -> u64 {
    if d > 9 {
        return n + 1; // nothing can contain digit ≥ 10
    }
    let digits: Vec<u64> = {
        let mut v = Vec::new();
        let mut m = n;
        while m > 0 {
            v.push(m % 10);
            m /= 10;
        }
        v.reverse();
        v
    };
    if digits.is_empty() {
        // n = 0: the only candidate is 0, which contains
        // digit 0 — avoid iff d != 0
        return u64::from(d != 0);
    }
    // dp[tight][started] — tight == 1 means still on the
    // bound; started == 1 means a real digit has begun
    let mut dp = [[0u64; 2]; 2];
    dp[1][0] = 1;
    for &bound in &digits {
        let mut next = [[0u64; 2]; 2];
        for (tight, row) in dp.iter().enumerate() {
            for (started, &cur) in row.iter().enumerate() {
                if cur == 0 {
                    continue;
                }
                let lim = if tight == 1 { bound } else { 9 };
                for v in 0..=lim {
                    let ns = started == 1 || v != 0;
                    if ns && v == u64::from(d) {
                        continue; // real occurrence of d
                    }
                    let nt = usize::from(tight == 1 && v == lim);
                    next[nt][usize::from(ns)] += cur;
                }
            }
        }
        dp = next;
    }
    let total = dp[0][0] + dp[0][1] + dp[1][0] + dp[1][1];
    // the all-unstarted path is the number 0: include it
    // unless d == 0 (its digit IS a real 0)
    total - u64::from(d == 0)
}

/// Number of `x ∈ [0, n]` whose decimal digit-sum equals
/// `s`. `x = 0` has digit-sum `0`.
pub fn count_digit_sum(n: u64, s: u64) -> u64 {
    if s > 9 * 20 {
        return 0;
    }
    let digits: Vec<u64> = {
        let mut v = Vec::new();
        let mut m = n;
        while m > 0 {
            v.push(m % 10);
            m /= 10;
        }
        v.reverse();
        v
    };
    if digits.is_empty() {
        return u64::from(s == 0);
    }
    // dp[sum][tight] — sum capped at s
    let mut dp = vec![[0u64; 2]; (s + 1) as usize];
    dp[0][1] = 1;
    for &bound in &digits {
        let mut next = vec![[0u64; 2]; (s + 1) as usize];
        for (sum, row) in dp.iter().enumerate() {
            for (tight, &cur) in row.iter().enumerate() {
                if cur == 0 {
                    continue;
                }
                let lim = if tight == 1 { bound } else { 9 };
                for v in 0..=lim {
                    let ns = sum + v as usize;
                    if ns > s as usize {
                        break;
                    }
                    next[ns][usize::from(tight == 1 && v == lim)] += cur;
                }
            }
        }
        dp = next;
    }
    dp[s as usize][0] + dp[s as usize][1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn digit_sum(mut x: u64) -> u64 {
        let mut s = 0;
        while x > 0 {
            s += x % 10;
            x /= 10;
        }
        s
    }

    fn has_digit(mut x: u64, d: u8) -> bool {
        if x == 0 {
            return d == 0;
        }
        while x > 0 {
            if x % 10 == u64::from(d) {
                return true;
            }
            x /= 10;
        }
        false
    }

    #[test]
    fn basics() {
        assert_eq!(count_digit_sum(100, 1), 3); // 1, 10, 100
        assert_eq!(count_digit_sum(20, 2), 3); // 2, 11, 20
        assert_eq!(count_digit_sum(9, 9), 1); // just 9
        assert_eq!(count_avoid_digit(100, 5), 82);
        assert_eq!(count_avoid_digit(10, 0), 9); // 1..9 (0 and 10 excluded)
        assert_eq!(count_avoid_digit(0, 5), 1); // just 0
        assert_eq!(count_avoid_digit(0, 0), 0);
        assert_eq!(count_avoid_digit(99, 1), 81);
    }

    /// Brute-force oracle over a dense prefix — both
    /// counters against direct enumeration.
    #[test]
    fn brute_oracle() {
        let mut rng = SplitMix64::new(0xD161);
        for _ in 0..600 {
            let n = u64::from(rng.below(20_000));
            let s = u64::from(rng.below(30));
            let d = rng.below(10) as u8;
            let mut es = 0u64;
            let mut ea = 0u64;
            for x in 0..=n {
                if digit_sum(x) == s {
                    es += 1;
                }
                if !has_digit(x, d) {
                    ea += 1;
                }
            }
            assert_eq!(count_digit_sum(n, s), es, "sum n={n} s={s}");
            assert_eq!(count_avoid_digit(n, d), ea, "avoid n={n} d={d}");
        }
    }

    /// Monotonicity + complement invariants: counts are
    /// nondecreasing in n; `avoid(d) + contains(d) = n+1`.
    #[test]
    fn invariants() {
        let mut rng = SplitMix64::new(0x1A2B);
        let mut prev_avoid = 0u64;
        let mut prev_sum = 0u64;
        for n in 0..3000u64 {
            let a = count_avoid_digit(n, 7);
            let s = count_digit_sum(n, 5);
            assert!(a >= prev_avoid);
            assert!(s >= prev_sum);
            prev_avoid = a;
            prev_sum = s;
            let _ = &mut rng;
        }
        // complement: contains = total − avoid
        for _ in 0..500 {
            let n = u64::from(rng.below(50_000));
            let d = rng.below(10) as u8;
            let mut contains = 0u64;
            for x in 0..=n.min(10_000) {
                if has_digit(x, d) {
                    contains += 1;
                }
            }
            let nn = n.min(10_000);
            assert_eq!(count_avoid_digit(nn, d) + contains, nn + 1);
        }
    }
}
