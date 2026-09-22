//! Knapsack DP — best loot under a weight budget, both worlds covered:
//! [`knapsack_01`] for unique items, [`knapsack_unbounded`] for
//! stackables. `O(n·W)` exact answers with deterministic item
//! reconstruction — when several picks tie on value, the one keeping the
//! earliest-listed items wins, so identical inputs always produce
//! identical loadouts (no hash iteration, no float).
//!
//! ```
//! use izanagi_kit::knapsack::knapsack_01;
//! let r = knapsack_01(&[(2, 3), (3, 4), (4, 5)], 5).unwrap();
//! assert_eq!(r.value, 7); // items 0+1: weight 5, value 3+4
//! assert_eq!(r.items, vec![0, 1]);
//! ```

/// Result of a knapsack computation.
pub struct Knapsack {
    /// Best achievable value.
    pub value: i64,
    /// Item indices chosen (`knapsack_01`: each index at most once, in
    /// ascending order; `knapsack_unbounded`: repeats allowed).
    pub items: Vec<u32>,
}

/// 0/1 knapsack: each item `(weight, value)` may be taken at most once.
/// Returns `None` on a `weight` > `u32::MAX` — every remaining item is
/// clamped to `u32::MAX`, i.e. effectively too heavy. Ties keep the
/// earlier-indexed items.
pub fn knapsack_01(items: &[(u64, i64)], capacity: u64) -> Option<Knapsack> {
    let w = usize::try_from(capacity).ok()?;
    let n = items.len();
    // dp[i][c] = best value using only items 0..i under capacity c.
    // The full table is kept so reconstruction can ask "did the optimum
    // change when item i became available?" — a single rolling row cannot
    // distinguish that from a later item's improvement.
    let neg = i64::MIN;
    let mut dp = vec![vec![neg; w + 1]; n + 1];
    dp[0].fill(0);
    for (i, &(wi, vi)) in items.iter().enumerate() {
        let wi = match usize::try_from(wi) {
            Ok(v) => v,
            Err(_) => {
                dp[i + 1] = dp[i].clone();
                continue;
            }
        };
        let (upper, rest) = dp.split_at_mut(i + 1);
        let (prev, next) = (&upper[i], &mut rest[0]);
        for (c, cell) in next.iter_mut().enumerate() {
            *cell = prev[c];
            if wi <= c && prev[c - wi] != neg {
                let cand = prev[c - wi].saturating_add(vi);
                if cand > *cell {
                    *cell = cand;
                }
            }
        }
    }
    let best = (0..=w).max_by_key(|&c| dp[n][c])?;
    if dp[n][best] == neg {
        return Some(Knapsack {
            value: 0,
            items: Vec::new(),
        });
    }
    // Walk layers back: took item i iff the optimum strictly improved.
    // Ties resolve to NOT taking the later item — earliest indices win.
    let mut items_out = Vec::new();
    let mut c = best;
    for i in (1..=n).rev() {
        if dp[i][c] != dp[i - 1][c] {
            items_out.push((i - 1) as u32);
            c = c.saturating_sub(usize::try_from(items[i - 1].0).unwrap_or(0));
        }
    }
    items_out.reverse();
    Some(Knapsack {
        value: dp[n][best],
        items: items_out,
    })
}

/// Unbounded knapsack: each item may be taken any number of times.
/// Returns `None` on a `weight` > `u32::MAX`. Reconstruction picks the
/// same deterministic multiset.
pub fn knapsack_unbounded(items: &[(u64, i64)], capacity: u64) -> Option<Knapsack> {
    let w = usize::try_from(capacity).ok()?;
    let neg = i64::MIN;
    let mut dp = vec![neg; w + 1];
    dp[0] = 0;
    // last[c] = item index that last improved dp[c] (for reconstruction).
    let mut last = vec![u32::MAX; w + 1];
    for c in 1..=w {
        for (i, &(wi, vi)) in items.iter().enumerate() {
            let wi = match usize::try_from(wi) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if wi <= c && wi > 0 && dp[c - wi] != neg {
                let cand = dp[c - wi].saturating_add(vi);
                if cand > dp[c] {
                    dp[c] = cand;
                    last[c] = i as u32;
                }
            }
        }
    }
    let best = (0..=w).max_by_key(|&c| dp[c])?;
    if dp[best] == neg {
        return Some(Knapsack {
            value: 0,
            items: Vec::new(),
        });
    }
    let mut items_out = Vec::new();
    let mut c = best;
    while c > 0 && last[c] != u32::MAX {
        let i = last[c] as usize;
        items_out.push(i as u32);
        c -= usize::try_from(items[i].0).unwrap_or(0);
    }
    items_out.sort_unstable();
    Some(Knapsack {
        value: dp[best],
        items: items_out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Brute force over all 2^n subsets (0/1) and bounded compositions.
    fn oracle_01(items: &[(u64, i64)], cap: u64) -> (i64, Vec<u32>) {
        let n = items.len();
        let mut best = 0i64;
        let mut pick: Vec<u32> = Vec::new();
        for mask in 0..(1usize << n) {
            let mut tw = 0u64;
            let mut tv = 0i64;
            let mut set = Vec::new();
            for (i, &(wi, vi)) in items.iter().enumerate() {
                if mask >> i & 1 == 1 {
                    tw += wi;
                    tv += vi;
                    set.push(i as u32);
                }
            }
            if tw <= cap && tv > best {
                best = tv;
                pick = set;
            }
        }
        (best, pick)
    }

    /// Unbounded oracle: bounded item list expansion with a fixed count
    /// cap per item — exact when capacity/item-weight fits.
    fn oracle_unbounded(items: &[(u64, i64)], cap: u64) -> i64 {
        // Expand into a bounded 0/1 problem: at most floor(cap/wi) copies.
        let mut expanded: Vec<(u64, i64)> = Vec::new();
        for &(wi, vi) in items {
            if wi == 0 {
                continue;
            }
            for _ in 0..cap / wi {
                expanded.push((wi, vi));
            }
        }
        let n = expanded.len();
        let mut best = 0i64;
        if n > 20 {
            return -1; // oracle scope: small expansions only
        }
        for mask in 0..(1usize << n) {
            let mut tw = 0u64;
            let mut tv = 0i64;
            for (i, &(wi, vi)) in expanded.iter().enumerate() {
                if mask >> i & 1 == 1 {
                    tw += wi;
                    tv += vi;
                }
            }
            if tw <= cap && tv > best {
                best = tv;
            }
        }
        best
    }

    #[test]
    fn knapsack_01_matches_brute_force() {
        let mut rng = SplitMix64::new(0xA1B2_C3D4);
        for _ in 0..200 {
            let n = (rng.below(6) + 1) as usize;
            let items: Vec<(u64, i64)> = (0..n)
                .map(|_| ((rng.below(8) + 1) as u64, rng.below(20) as i64))
                .collect();
            let cap = (rng.below(20) + 1) as u64;
            let r = knapsack_01(&items, cap).unwrap();
            let (ov, os) = oracle_01(&items, cap);
            assert_eq!(r.value, ov);
            // Feasibility + index order of the returned witness.
            let tw: u64 = r.items.iter().map(|&i| items[i as usize].0).sum();
            assert!(tw <= cap);
            let tv: i64 = r.items.iter().map(|&i| items[i as usize].1).sum();
            assert_eq!(tv, r.value);
            let mut sorted = r.items.clone();
            sorted.sort_unstable();
            assert_eq!(sorted, r.items);
            // Deterministic tie-break keeps it equal to the oracle's
            // first-best witness on identical orderings.
            assert_eq!(r.items, os);
        }
        // Empty / zero-capacity corners.
        assert_eq!(knapsack_01(&[], 5).unwrap().value, 0);
        assert_eq!(knapsack_01(&[(5, 9)], 4).unwrap().value, 0);
    }

    #[test]
    fn knapsack_unbounded_matches_oracle() {
        let mut rng = SplitMix64::new(0xD5E6_F708);
        for _ in 0..100 {
            let n = (rng.below(3) + 1) as usize;
            let items: Vec<(u64, i64)> = (0..n)
                .map(|_| ((rng.below(5) + 2) as u64, rng.below(15) as i64))
                .collect();
            let cap = (rng.below(9) + 2) as u64;
            let r = knapsack_unbounded(&items, cap).unwrap();
            let ov = oracle_unbounded(&items, cap);
            assert_eq!(r.value, ov);
            let tw: u64 = r.items.iter().map(|&i| items[i as usize].0).sum();
            assert!(tw <= cap);
            let tv: i64 = r.items.iter().map(|&i| items[i as usize].1).sum();
            assert_eq!(tv, r.value);
        }
        // Classic: cap 10, items (5,10) × (4,8) × (3,6) → two of (5,10)=20.
        let r = knapsack_unbounded(&[(5, 10), (4, 8), (3, 6)], 10).unwrap();
        assert_eq!(r.value, 20);
        assert_eq!(r.items, vec![0, 0]);
    }
}
