//! Mo's algorithm — offline `O((n + q)·√n·C)` range queries by sliding
//! a window and answering with incremental add/remove.
//!
//! [`mos_order`] returns the deterministic processing order for the
//! queries (block-by-`l`, then `r`, alternating `r` direction between
//! adjacent blocks so the window never walks back across the whole
//! array per block). [`range_distinct`] is the canonical instantiation
//! — distinct-element count per `[l, r)` — and serves as the reference
//! for how to wire custom aggregates.
//!
//! Deterministic: the sort key is a pure function of `(l, r, index)`,
//! and identical queries return identical results.
//!
//! ```
//! use izanagi_kit::mo::range_distinct;
//! let vals = [1, 2, 1, 3, 2, 1];
//! let ans = range_distinct(&vals, &[(0, 3), (0, 6), (2, 5), (4, 6)]);
//! assert_eq!(ans, vec![2, 3, 3, 2]);
//! ```

use std::collections::BTreeMap;

/// Integer square root — `floor(√n)` without floats.
fn isqrt(n: usize) -> usize {
    let mut r = 0usize;
    while r < n / (r + 1) {
        r += 1;
    }
    r
}

/// Deterministic Mo-order permutation: indices into `queries` sorted
/// so a sliding `[l, r)` window moves monotonically inside each
/// `√n`-wide block of `l`. Ties inside a block resolve to the smaller
/// `r` (even-numbered blocks) or larger `r` (odd), then query index —
/// the standard zigzag ordering that halves window travel.
pub fn mos_order(n: usize, queries: &[(usize, usize)]) -> Vec<u32> {
    let block = isqrt(n).max(1);
    let mut idx: Vec<u32> = (0..queries.len() as u32).collect();
    idx.sort_by_key(|&i| {
        let (l, r) = queries[i as usize];
        let b = l / block;
        // Zigzag: even blocks scan r ascending, odd descending.
        let key = if b % 2 == 0 { r } else { !r };
        (b, key, i)
    });
    idx
}

fn add(cnt: &mut BTreeMap<u32, u32>, distinct: &mut usize, v: u32) {
    let e = cnt.entry(v).or_insert(0);
    *e += 1;
    if *e == 1 {
        *distinct += 1;
    }
}

fn remove(cnt: &mut BTreeMap<u32, u32>, distinct: &mut usize, v: u32) {
    if let std::collections::btree_map::Entry::Occupied(mut e) = cnt.entry(v) {
        let c = e.get_mut();
        *c -= 1;
        if *c == 0 {
            e.remove();
            *distinct -= 1;
        }
    }
}

/// Number of distinct values in `vals[l..r]` for each query, computed
/// offline in `O((n + q)·√n)` window moves. Out-of-range ends clamp to
/// `n` (`l > n` is an empty range).
pub fn range_distinct(vals: &[u32], queries: &[(usize, usize)]) -> Vec<u32> {
    let n = vals.len();
    let mut cnt: BTreeMap<u32, u32> = BTreeMap::new();
    let mut distinct = 0usize;
    let mut cur_l = 0usize;
    let mut cur_r = 0usize; // window is [cur_l, cur_r)
    let mut ans = vec![0u32; queries.len()];

    for &qi in &mos_order(n, queries) {
        let (ql, qr) = queries[qi as usize];
        let ql = ql.min(n);
        let qr = qr.min(n).max(ql);
        while cur_l > ql {
            cur_l -= 1;
            add(&mut cnt, &mut distinct, vals[cur_l]);
        }
        while cur_r < qr {
            add(&mut cnt, &mut distinct, vals[cur_r]);
            cur_r += 1;
        }
        while cur_l < ql {
            remove(&mut cnt, &mut distinct, vals[cur_l]);
            cur_l += 1;
        }
        while cur_r > qr {
            cur_r -= 1;
            remove(&mut cnt, &mut distinct, vals[cur_r]);
        }
        ans[qi as usize] = distinct as u32;
    }
    ans
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    #[test]
    fn distinct_matches_per_query_oracle() {
        let mut rng = SplitMix64::new(0xA051);
        for _ in 0..150 {
            let n = (rng.below(40) + 1) as usize;
            let vals: Vec<u32> = (0..n).map(|_| rng.below(8)).collect();
            let q = (rng.below(30) + 1) as usize;
            let queries: Vec<(usize, usize)> = (0..q)
                .map(|_| {
                    let l = rng.below(n as u32) as usize;
                    let r = l + rng.below((n - l) as u32) as usize + 1;
                    (l, r)
                })
                .collect();
            let got = range_distinct(&vals, &queries);
            for (i, &(l, r)) in queries.iter().enumerate() {
                let want = vals[l..r].iter().copied().collect::<BTreeSet<_>>().len() as u32;
                assert_eq!(got[i], want, "vals={vals:?} q={queries:?} i={i}");
            }
        }
    }

    #[test]
    fn arbitrary_values_are_counted() {
        // Values far outside the index space still count once.
        assert_eq!(range_distinct(&[u32::MAX, u32::MAX, 5], &[(0, 3)]), vec![2]);
        let mut rng = SplitMix64::new(0x12A0);
        for _ in 0..100 {
            let n = (rng.below(20) + 1) as usize;
            let vals: Vec<u32> = (0..n).map(|_| rng.below(5) * 1_000_003).collect();
            let q = (rng.below(15) + 1) as usize;
            let queries: Vec<(usize, usize)> = (0..q)
                .map(|_| {
                    let l = rng.below(n as u32) as usize;
                    let r = l + rng.below((n - l) as u32) as usize + 1;
                    (l, r)
                })
                .collect();
            let got = range_distinct(&vals, &queries);
            for (i, &(l, r)) in queries.iter().enumerate() {
                let want = vals[l..r].iter().copied().collect::<BTreeSet<_>>().len() as u32;
                assert_eq!(got[i], want);
            }
        }
    }

    #[test]
    fn mos_order_is_a_deterministic_permutation() {
        let queries: Vec<(usize, usize)> =
            vec![(0, 3), (2, 5), (1, 2), (0, 1), (4, 9), (3, 4), (2, 8)];
        let a = mos_order(10, &queries);
        let b = mos_order(10, &queries);
        assert_eq!(a, b);
        assert_eq!(
            a.iter().copied().collect::<BTreeSet<_>>().len(),
            queries.len()
        );
        // l-block ids must be non-decreasing in processing order
        // (block = l / floor(sqrt(n)) = l / 3 for n = 10).
        for w in a.windows(2) {
            let (bl, br) = (queries[w[0] as usize].0 / 3, queries[w[1] as usize].0 / 3);
            assert!(bl <= br);
        }
    }

    #[test]
    fn edge_cases() {
        assert_eq!(range_distinct(&[], &[(0, 0)]), vec![0]);
        assert_eq!(range_distinct(&[7], &[(0, 1), (0, 1)]), vec![1, 1]);
        assert_eq!(range_distinct(&[1, 1, 1], &[(0, 3)]), vec![1]);
        // Clamping: out-of-range ends clamp to n instead of panicking.
        assert_eq!(range_distinct(&[5, 6], &[(0, 99), (5, 9)]), vec![2, 0]);
        // Empty query list / empty ranges.
        assert_eq!(range_distinct(&[1, 2], &[]), Vec::<u32>::new());
        assert_eq!(range_distinct(&[1, 2], &[(1, 1)]), vec![0]);
    }
}
