//! Interval scheduling — the interval graph classics: maximum
//! independent set (earliest-finish greedy), minimum resources
//! (partitioning = chromatic number for interval graphs), and weighted
//! selection (`O(n log n)` DP over finish times). Intervals are
//! half-open `[start, end)` `i64`s; degenerate `start == end` items
//! occupy no span and conflict with nothing.
//!
//! Everything is deterministic: ties break on `(end, start, index)`,
//! the DP predecessor link is chosen by the same canonical order, and
//! results carry the picked indices so callers can audit the choice.
//!
//! ```
//! use izanagi_kit::intervalgraph::{max_independent_set, min_rooms};
//! // meetings [(0,10),(5,15),(10,20)] — pick two, need two rooms
//! let iv = [(0, 10), (5, 15), (10, 20)];
//! assert_eq!(max_independent_set(&iv), vec![0, 2]);
//! assert_eq!(min_rooms(&iv), 2);
//! ```

fn canonical(iv: &[(i64, i64)]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..iv.len()).collect();
    order.sort_by_key(|&i| (iv[i].1, iv[i].0, i));
    order
}

/// Maximum-size pairwise-disjoint subset — returns the chosen indices
/// in schedule order (earliest-finish greedy, canonical tie-break).
pub fn max_independent_set(iv: &[(i64, i64)]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut last_end = i64::MIN;
    for &i in &canonical(iv) {
        let (s, e) = iv[i];
        if s < e && s >= last_end {
            out.push(i);
            last_end = e;
        }
    }
    out
}

/// Minimum number of parallel tracks needed to host every interval —
/// equals the maximum overlap count (interval graphs are perfect).
pub fn min_rooms(iv: &[(i64, i64)]) -> usize {
    let mut ev: Vec<(i64, i64)> = Vec::with_capacity(iv.len() * 2);
    for &(s, e) in iv {
        if s < e {
            ev.push((s, 1));
            ev.push((e, -1));
        }
    }
    // ends before starts at the same instant — half-open semantics
    ev.sort_unstable();
    let (mut depth, mut best) = (0i64, 0i64);
    for &(_, d) in &ev {
        depth += d;
        best = best.max(depth);
    }
    best as usize
}

/// Maximum total weight of a pairwise-disjoint subset: `w`-weighted
/// intervals `(start, end, weight)`. Returns `(weight, indices)`.
/// `weight` may be negative — an empty schedule scores `0`.
pub fn weighted_select(iv: &[(i64, i64, i64)]) -> (i64, Vec<usize>) {
    let n = iv.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| (iv[i].1, iv[i].0, i));
    // p[j] = #order elements finishing ≤ start of order[j]
    let mut p = vec![0usize; n];
    for (j, &i) in order.iter().enumerate() {
        let s = iv[i].0;
        p[j] = order.partition_point(|&k| iv[k].1 <= s);
    }
    // dp[j] = best weight using first j order-items
    let mut dp = vec![0i64; n + 1];
    let mut take = vec![false; n];
    for j in 0..n {
        let i = order[j];
        // degenerate spans are unselectable — same contract as
        // max_independent_set (a zero-length item picks nothing)
        let inc = if iv[i].0 < iv[i].1 {
            iv[i].2.saturating_add(dp[p[j]])
        } else {
            i64::MIN
        };
        if inc > dp[j] {
            dp[j + 1] = inc;
            take[j] = true;
        } else {
            dp[j + 1] = dp[j];
        }
    }
    let mut out = Vec::new();
    let mut j = n;
    while j > 0 {
        if take[j - 1] {
            out.push(order[j - 1]);
            j = p[j - 1];
        } else {
            j -= 1;
        }
    }
    out.sort_unstable();
    (dp[n], out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn brute_mis(iv: &[(i64, i64)]) -> usize {
        let n = iv.len();
        let mut best = 0usize;
        for mask in 0..(1usize << n) {
            let mut ok = true;
            let mut cnt = 0;
            for (i, &(s, e)) in iv.iter().enumerate() {
                if mask >> i & 1 == 0 || s >= e {
                    continue; // degenerate intervals never block
                }
                cnt += 1;
                for (j, &(s2, e2)) in iv.iter().enumerate().take(i) {
                    if mask >> j & 1 == 1 && s.max(s2) < e.min(e2) {
                        ok = false;
                    }
                }
            }
            if ok {
                best = best.max(cnt);
            }
        }
        best
    }
    fn brute_weighted(iv: &[(i64, i64, i64)]) -> i64 {
        let n = iv.len();
        let mut best = 0i64;
        for mask in 0..(1usize << n) {
            let (mut w, mut ok) = (0i64, true);
            for (i, &(s, e, wi)) in iv.iter().enumerate() {
                if mask >> i & 1 == 0 || s >= e {
                    continue;
                }
                w = w.saturating_add(wi);
                for (j, &(s2, e2, _)) in iv.iter().enumerate().take(i) {
                    if mask >> j & 1 == 1 && s.max(s2) < e.min(e2) {
                        ok = false;
                    }
                }
            }
            if ok {
                best = best.max(w);
            }
        }
        best
    }
    fn disjoint(iv: &[(i64, i64)], pick: &[usize]) -> bool {
        for (a, &i) in pick.iter().enumerate() {
            if iv[i].0 >= iv[i].1 {
                return false;
            }
            for &j in &pick[..a] {
                if iv[i].0.max(iv[j].0) < iv[i].1.min(iv[j].1) {
                    return false;
                }
            }
        }
        true
    }

    #[test]
    fn textbook() {
        assert_eq!(
            max_independent_set(&[(0, 6), (1, 4), (3, 5), (5, 7), (8, 9)]),
            vec![1, 3, 4]
        );
        assert_eq!(min_rooms(&[(0, 30), (5, 10), (15, 20)]), 2);
        let (w, pick) = weighted_select(&[(1, 3, 5), (2, 5, 6), (4, 6, 5), (6, 7, 4)]);
        assert_eq!(w, 14);
        assert!(disjoint(&[(1, 3), (2, 5), (4, 6), (6, 7)], &pick));
    }

    #[test]
    fn mis_oracle() {
        let mut rng = SplitMix64::new(0x01a7_ea11_5ced);
        for _ in 0..500 {
            let n = rng.below(9) as usize;
            let iv: Vec<(i64, i64)> = (0..n)
                .map(|_| {
                    let s = rng.below(14) as i64;
                    (s, s + rng.below(9) as i64)
                })
                .collect();
            let pick = max_independent_set(&iv);
            assert!(disjoint(&iv, &pick), "{iv:?} {pick:?}");
            assert_eq!(pick.len(), brute_mis(&iv), "{iv:?}");
        }
    }

    #[test]
    fn rooms_oracle() {
        let mut rng = SplitMix64::new(0x01a7_ea11_5002);
        for _ in 0..400 {
            let n = rng.below(9) as usize;
            let iv: Vec<(i64, i64)> = (0..n)
                .map(|_| {
                    let s = rng.below(14) as i64;
                    (s, s + rng.below(9) as i64)
                })
                .collect();
            // min rooms = max depth = brute: check assignability greedily
            // via direct max-overlap recompute (independent of sweep)
            let mut best = 0;
            for x in -1..40 {
                let d = iv.iter().filter(|&&(s, e)| s <= x && x < e).count();
                best = best.max(d);
            }
            assert_eq!(min_rooms(&iv), best, "{iv:?}");
        }
    }

    #[test]
    fn weighted_oracle() {
        let mut rng = SplitMix64::new(0x01a7_ea11_5003);
        for _ in 0..400 {
            let n = rng.below(9) as usize;
            let iv: Vec<(i64, i64, i64)> = (0..n)
                .map(|_| {
                    let s = rng.below(14) as i64;
                    (s, s + rng.below(9) as i64, rng.below(40) as i64)
                })
                .collect();
            let (w, pick) = weighted_select(&iv);
            let flat: Vec<(i64, i64)> = iv.iter().map(|&(s, e, _)| (s, e)).collect();
            assert!(disjoint(&flat, &pick), "{iv:?} {pick:?}");
            assert_eq!(w, brute_weighted(&iv), "{iv:?}");
            assert_eq!(w, pick.iter().map(|&i| iv[i].2).sum());
        }
    }

    #[test]
    fn negatives_and_empty() {
        // negative weights → empty schedule wins
        assert_eq!(weighted_select(&[(0, 5, -3)]).0, 0);
        assert!(weighted_select(&[(0, 5, -3)]).1.is_empty());
        assert_eq!(min_rooms(&[]), 0);
        assert!(max_independent_set(&[]).is_empty());
        // zero-length items occupy nothing
        assert_eq!(max_independent_set(&[(3, 3), (0, 4)]), vec![1]);
    }
}
