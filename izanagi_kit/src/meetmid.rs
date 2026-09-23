//! Meet-in-the-middle — split `n` choices into two halves,
//! enumerate both `2^(n/2)` subspaces, then join the halves by
//! sort + binary search. Turns exponential `O(2^n)` searches
//! (subset sum, cardinality-bounded best fit, exact count) into
//! `O(2^(n/2) · n)` — the standard technique from Wagner's
//! generalized-birthday line and competitive-programming kits.
//!
//! ```
//! use izanagi_kit::meetmid;
//! // subset sum over 20 items would take 2^20 brute force
//! let w: Vec<i64> = (1..=20).collect();
//! assert!(meetmid::subset_sum(&w, 55).is_some());
//! assert_eq!(meetmid::count_subsets(&w, 0), 1); // only the empty set
//! ```

/// Enumerate all `2^len` subset sums of `items`, sorted
/// ascending. Sums are `i128`; overflow on addition wraps in
/// the same way `i128` does — callers keep values inside the
/// accumulation range.
pub fn subset_sums(items: &[i64]) -> Vec<i128> {
    let mut sums = Vec::with_capacity(1usize << items.len());
    sums.push(0i128);
    for &x in items {
        let n = sums.len();
        for i in 0..n {
            sums.push(sums[i] + x as i128);
        }
    }
    sums.sort_unstable();
    sums
}

/// A subset (ascending indices) whose weights sum to `target`,
/// or `None`. Two-pass: enumerate the left half, binary-search
/// the right half for the complement.
pub fn subset_sum(weights: &[i64], target: i64) -> Option<Vec<u32>> {
    let n = weights.len();
    let (l, r) = weights.split_at(n / 2);
    let right = subset_sums(r);
    let mut sums = vec![0i128];
    for &x in l {
        let m = sums.len();
        for i in 0..m {
            sums.push(sums[i] + x as i128);
        }
    }
    let t = target as i128;
    let mut bits = Vec::with_capacity(l.len());
    for mask in 0..1u64 << l.len() {
        bits.clear();
        let mut s = 0i128;
        for (i, &x) in l.iter().enumerate() {
            if mask >> i & 1 == 1 {
                s += x as i128;
                bits.push(i as u32);
            }
        }
        let need = t - s;
        if right.binary_search(&need).is_ok() {
            // reconstruct the right half by its subset-sum index
            let mut out = bits.clone();
            // find a right-half mask producing `need`
            for rmask in 0..1u64 << r.len() {
                let mut rs = 0i128;
                let mut take = Vec::new();
                for (j, &x) in r.iter().enumerate() {
                    if rmask >> j & 1 == 1 {
                        rs += x as i128;
                        take.push((l.len() + j) as u32);
                    }
                }
                if rs == need {
                    out.extend(take);
                    return Some(out);
                }
            }
        }
    }
    None
}

/// Count of subsets whose weights sum to `target`. Each half's
/// multiset is counted with duplicates — subsets are distinct
/// by index even when weights repeat.
pub fn count_subsets(weights: &[i64], target: i64) -> u64 {
    let n = weights.len();
    let (l, r) = weights.split_at(n / 2);
    let right = subset_sums(r);
    let left = subset_sums(l);
    let t = target as i128;
    let mut count = 0u64;
    for &s in &left {
        let need = t - s;
        let lo = right.partition_point(|&x| x < need);
        let hi = right.partition_point(|&x| x <= need);
        count += (hi - lo) as u64;
    }
    count
}

/// Largest subset sum `≤ cap` (subset-sum knapsack bound).
/// Returns `i128::MIN` when `cap < 0` and no subset qualifies;
/// the empty sum 0 is a candidate whenever `cap >= 0`.
pub fn best_fit(weights: &[i64], cap: i64) -> i128 {
    let n = weights.len();
    let (l, r) = weights.split_at(n / 2);
    let left = subset_sums(l);
    let mut right = subset_sums(r);
    right.dedup();
    let cap = cap as i128;
    let mut best = i128::MIN;
    for &s in &left {
        if s > cap {
            continue;
        }
        let room = cap - s;
        let i = right.partition_point(|&x| x <= room);
        if i > 0 {
            best = best.max(s + right[i - 1]);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn brute_sums(w: &[i64]) -> Vec<i128> {
        let mut out = Vec::with_capacity(1usize << w.len());
        for mask in 0..1u64 << w.len() {
            let mut s = 0i128;
            for (i, &x) in w.iter().enumerate() {
                if mask >> i & 1 == 1 {
                    s += x as i128;
                }
            }
            out.push(s);
        }
        out
    }

    #[test]
    fn subset_sums_matches_brute() {
        let mut rng = SplitMix64::new(5);
        for _ in 0..60 {
            let n = rng.below(12) as usize;
            let w: Vec<i64> = (0..n).map(|_| rng.below(40) as i64 - 20).collect();
            let mut want = brute_sums(&w);
            want.sort_unstable();
            assert_eq!(subset_sums(&w), want);
        }
    }

    #[test]
    fn subset_sum_oracle() {
        let mut rng = SplitMix64::new(6);
        for _ in 0..400 {
            let n = rng.below(14) as usize;
            let w: Vec<i64> = (0..n).map(|_| rng.below(30) as i64 - 15).collect();
            let t = rng.below(60) as i64 - 30;
            let possible = brute_sums(&w).contains(&(t as i128));
            match subset_sum(&w, t) {
                Some(idx) => {
                    let s: i128 = idx.iter().map(|&i| w[i as usize] as i128).sum();
                    assert_eq!(s, t as i128);
                }
                None => assert!(!possible),
            }
        }
    }

    #[test]
    fn count_subsets_oracle() {
        let mut rng = SplitMix64::new(7);
        for _ in 0..200 {
            let n = rng.below(12) as usize;
            let w: Vec<i64> = (0..n).map(|_| rng.below(10) as i64 - 5).collect();
            let t = rng.below(20) as i64 - 10;
            let want = brute_sums(&w).iter().filter(|&&s| s == t as i128).count() as u64;
            assert_eq!(count_subsets(&w, t), want);
        }
    }

    #[test]
    fn best_fit_oracle() {
        let mut rng = SplitMix64::new(8);
        for _ in 0..200 {
            let n = rng.below(12) as usize;
            let w: Vec<i64> = (0..n).map(|_| rng.below(20) as i64).collect();
            let cap = rng.below(60) as i64;
            let want = brute_sums(&w)
                .into_iter()
                .filter(|&s| s <= cap as i128)
                .max()
                .unwrap_or(i128::MIN);
            assert_eq!(best_fit(&w, cap), want);
        }
    }
}
