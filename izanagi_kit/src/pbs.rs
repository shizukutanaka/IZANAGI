//! Parallel binary search (並行二分探索): answer `q` offline queries of the
//! form *"the smallest `t` such that a monotone predicate over `ops[..t]`
//! holds"* in `O((n + q) log n)` `apply`/`pred` calls — instead of one
//! `O(n)` scan per query. The standard technique for "first time an event
//! threshold is crossed" questions: given a deterministic op log (damage
//! ticks, resource deltas, threat additions) and a batch of per-entity
//! thresholds, find each threshold's crossing tick without rescanning.
//!
//! The state is rebuilt from scratch each pass (`init()` + `apply` in op
//! order), so `apply` needs no inverse — the only contract is that `pred`
//! is **monotone in `t`** for every query: once true it stays true as more
//! ops are applied. On a non-monotone predicate the result is whatever the
//! bisection happens to land on (it still returns, never loops).
//!
//! ```
//! use izanagi_kit::pbs::parallel_binary_search;
//!
//! // First index where the prefix sum reaches each threshold:
//! let ops = [3i64, 1, 4, 1, 5];           // prefix sums 3,4,8,9,14
//! let thresholds = [(0usize, 5usize); 3]; // search window per query
//! let need = [4i64, 8, 100];              // each query's predicate
//! let ans = parallel_binary_search(
//!     &ops,
//!     &thresholds,
//!     || 0i64,
//!     |sum, &op| *sum += op,
//!     |sum, q| *sum >= need[q],
//! );
//! assert_eq!(ans, vec![Some(2), Some(3), None]);
//! ```

/// Parallel binary search — see the module docs for the contract.
///
/// - `ops` is the ordered log; `pred` sees the state after `ops[..t]`.
/// - `ranges[q] = (lo, hi)` bounds the search for query `q`: the answer is
///   the smallest `t ∈ [lo, hi]` with `pred` true, or `None` when `pred`
///   never fires (including at `t = hi`). `hi` is clamped to `ops.len()`.
/// - `init` rebuilds the fold state per pass; `apply` folds one op in;
///   `pred(ctx, q)` reads the state for query `q` (the query index — close
///   over per-query parameters like `need` in the doctest).
///
/// Cost: `O((ops.len() + queries) · log n)` `apply`/`pred` calls — each of
/// the `⌈log₂ n⌉` passes replays `ops` at most once and touches every
/// still-undecided query once.
pub fn parallel_binary_search<Ctx, Op>(
    ops: &[Op],
    ranges: &[(usize, usize)],
    init: impl Fn() -> Ctx,
    mut apply: impl FnMut(&mut Ctx, &Op),
    mut pred: impl FnMut(&Ctx, usize) -> bool,
) -> Vec<Option<usize>> {
    let n = ops.len();
    let q = ranges.len();
    // lo[q] = first candidate that could be true; hi[q] = last allowed.
    // Invariant: pred is false below lo (as far as we know) and we only
    // ever ask about t ≤ hi.
    let mut lo: Vec<usize> = ranges.iter().map(|&(l, _)| l).collect();
    let mut hi: Vec<usize> = ranges.iter().map(|&(_, h)| h.min(n)).collect();
    let mut active: Vec<usize> = (0..q).filter(|&i| lo[i] <= hi[i]).collect();

    while !active.is_empty() {
        // Bucket active queries by mid, smallest mid first.
        active.sort_by_key(|&i| lo[i] + (hi[i] - lo[i]) / 2);
        let mut ctx = init();
        let mut ptr = 0usize;
        let mut still: Vec<usize> = Vec::with_capacity(active.len());
        for &i in &active {
            let mid = lo[i] + (hi[i] - lo[i]) / 2;
            while ptr < mid {
                apply(&mut ctx, &ops[ptr]);
                ptr += 1;
            }
            if pred(&ctx, i) {
                hi[i] = mid;
            } else {
                lo[i] = mid + 1;
            }
            if lo[i] < hi[i] {
                still.push(i);
            }
        }
        active = still;
    }
    // Final disambiguation at t = lo = hi: pred may still be false there.
    (0..q)
        .map(|i| {
            if lo[i] > hi[i] || lo[i] > n {
                return None;
            }
            let mut ctx = init();
            for op in &ops[..lo[i]] {
                apply(&mut ctx, op);
            }
            if pred(&ctx, i) {
                Some(lo[i])
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Brute-force oracle: apply ops 0..t in order, first t in [lo,hi] with
    /// pred true, None if it never fires.
    fn oracle<Ctx, Op>(
        ops: &[Op],
        range: (usize, usize),
        init: impl Fn() -> Ctx,
        mut apply: impl FnMut(&mut Ctx, &Op),
        mut pred: impl FnMut(&Ctx) -> bool,
    ) -> Option<usize> {
        let mut ctx = init();
        let (lo, hi) = (range.0, range.1.min(ops.len()));
        for op in &ops[..lo.min(ops.len())] {
            apply(&mut ctx, op);
        }
        for t in lo..=hi {
            if t > 0 && t <= ops.len() {
                // prefix ops[..t]: apply op t-1 when stepping past it
            }
            if t > ops.len() {
                break;
            }
            if pred(&ctx) {
                return Some(t);
            }
            if t < ops.len() {
                apply(&mut ctx, &ops[t]);
            }
        }
        None
    }

    #[test]
    fn prefix_sum_threshold_matches_brute_force() {
        let mut rng = crate::rng::SplitMix64::new(0x7EA);
        for _case in 0..300 {
            let n = 1 + rng.below(20) as usize;
            let ops: Vec<i64> = (0..n).map(|_| rng.below(10) as i64).collect();
            let q = 1 + rng.below(8) as usize;
            let need: Vec<i64> = (0..q).map(|_| rng.below(60) as i64).collect();
            let ranges: Vec<(usize, usize)> = (0..q).map(|_| (rng.below(2) as usize, n)).collect();
            let got = parallel_binary_search(
                &ops,
                &ranges,
                || 0i64,
                |s, &o| *s += o,
                |s, i| *s >= need[i],
            );
            for (i, &r) in ranges.iter().enumerate() {
                let want = oracle(&ops, r, || 0i64, |s, &o| *s += o, |s| *s >= need[i]);
                assert_eq!(got[i], want, "q={i} ops={ops:?} need={}", need[i]);
            }
        }
    }

    #[test]
    fn first_membership_tick_matches_brute_force() {
        // ops toggle a membership bit; query asks the first t where the bit
        // for a given key is set — monotone since nothing unsets.
        let ops: Vec<u32> = vec![5, 2, 5, 7, 2, 9];
        let keys = [2u32, 5, 9, 4];
        let ranges = [(0usize, ops.len()); 4];
        let ans = parallel_binary_search(
            &ops,
            &ranges,
            || [false; 10],
            |set, &k| {
                if (k as usize) < 10 {
                    set[k as usize] = true;
                }
            },
            |set, i| set[keys[i] as usize],
        );
        assert_eq!(ans, vec![Some(2), Some(1), Some(6), None]);
        for (i, &r) in ranges.iter().enumerate() {
            let want = oracle(
                &ops,
                r,
                || [false; 10],
                |set, &k| {
                    if (k as usize) < 10 {
                        set[k as usize] = true;
                    }
                },
                |set| set[keys[i] as usize],
            );
            assert_eq!(ans[i], want);
        }
    }

    #[test]
    fn edge_cases_stay_total() {
        let ops = [1i64, 2, 3];
        // Empty query set.
        assert!(
            parallel_binary_search(&ops, &[], || 0i64, |s, &o| *s += o, |_, _| true).is_empty()
        );
        // Empty op set: only t=0 is in range.
        let empty: [i64; 0] = [];
        let ans = parallel_binary_search(
            &empty,
            &[(0, 0), (0, 5)],
            || 0i64,
            |s, &o| *s += o,
            |s, _| *s >= 0,
        );
        assert_eq!(ans, vec![Some(0), Some(0)]);
        // lo > hi is immediately None.
        let ans = parallel_binary_search(&ops, &[(3, 1)], || 0i64, |s, &o| *s += o, |_, _| true);
        assert_eq!(ans, vec![None]);
        // hi beyond ops len is clamped.
        let ans =
            parallel_binary_search(&ops, &[(0, 99)], || 0i64, |s, &o| *s += o, |s, _| *s >= 6);
        assert_eq!(ans, vec![Some(3)]);
    }

    #[test]
    fn same_inputs_give_identical_answers() {
        let ops = [3i64, 1, 4, 1, 5, 9];
        let ranges = [(0usize, 6); 4];
        let run = |need: &[i64]| {
            parallel_binary_search(
                &ops,
                &ranges,
                || 0i64,
                |s, &o| *s += o,
                |s, i| *s >= need[i],
            )
        };
        assert_eq!(run(&[1, 6, 20, 0]), run(&[1, 6, 20, 0]));
    }
}
