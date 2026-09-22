//! Deterministic selection — the `k`-th smallest element in worst-case
//! `O(n)` via median-of-medians (BFPRT, Blum–Floyd–Pratt–Rivest–Tarjan
//! 1972). Where sort-then-index is `O(n log n)` and quickselect is
//! expected-`O(n)` with a quadratic worst case, this is guaranteed
//! linear *and* input-order-independent: no randomness anywhere, so a
//! replay-pinned median or percentile never shifts.
//!
//! ```
//! use izanagi_kit::bfprt::{median, select};
//! assert_eq!(select(&[9, 1, 5, 3, 7], 2), Some(5));
//! assert_eq!(median(&[9, 1, 5, 3, 7]), Some(5));
//! ```

const GROUP: usize = 5;

/// The `k`-th smallest (0-indexed) element, `None` if `k >= len`.
/// Does not modify the input; internally copies then partition-walks.
pub fn select(data: &[i64], k: usize) -> Option<i64> {
    if k >= data.len() {
        return None;
    }
    let mut a = data.to_vec();
    Some(select_inplace(&mut a, k))
}

/// The upper median: `select(data, n/2)` — for even `n` the larger of
/// the two middle values. `None` on empty input.
pub fn median(data: &[i64]) -> Option<i64> {
    if data.is_empty() {
        None
    } else {
        select(data, data.len() / 2)
    }
}

/// In-place selection: `a[k]` ends as the `k`-th smallest.
/// `k < a.len()` is required by the callers.
fn select_inplace(mut a: &mut [i64], k: usize) -> i64 {
    loop {
        if a.len() <= GROUP {
            a.sort_unstable();
            return a[k];
        }
        // Median of the group-of-5 medians.
        let mut medians = Vec::with_capacity(a.len() / GROUP + 1);
        for g in a.chunks(GROUP) {
            let mut g = g.to_vec();
            g.sort_unstable();
            medians.push(g[g.len() / 2]);
        }
        let mid = medians.len() / 2;
        let pivot = select_inplace(&mut medians, mid);
        let (lt, gt) = partition(a, pivot);
        // a[..lt] < pivot; a[lt..gt] == pivot; a[gt..] > pivot.
        if k < lt {
            a = &mut a[..lt];
        } else if k < gt {
            return pivot;
        } else {
            return select_inplace(&mut a[gt..], k - gt);
        }
    }
}

/// Three-way partition of `a` around `pivot`. Returns `(lt, gt)` —
/// counts of elements `<` and `<=` pivot (Dutch national flag).
fn partition(a: &mut [i64], pivot: i64) -> (usize, usize) {
    let (mut lt, mut i, mut gt) = (0usize, 0usize, a.len());
    while i < gt {
        match a[i].cmp(&pivot) {
            std::cmp::Ordering::Less => {
                a.swap(lt, i);
                lt += 1;
                i += 1;
            }
            std::cmp::Ordering::Equal => i += 1,
            std::cmp::Ordering::Greater => {
                gt -= 1;
                a.swap(i, gt);
            }
        }
    }
    (lt, gt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn select_matches_sorted_oracle() {
        let mut rng = SplitMix64::new(0xBF98_77AA);
        for _ in 0..400 {
            let n = rng.below(80) as usize;
            let data: Vec<i64> = (0..n).map(|_| (rng.below(200) as i64) - 100).collect();
            let mut sorted = data.clone();
            sorted.sort_unstable();
            for k in [0usize, n / 4, n / 2, (3 * n) / 4, n.saturating_sub(1)] {
                if k < n {
                    assert_eq!(select(&data, k), Some(sorted[k]), "k={k} {data:?}");
                }
            }
            assert_eq!(select(&data, n), None);
            if !data.is_empty() {
                assert_eq!(median(&data), Some(sorted[n / 2]));
            }
        }
        assert_eq!(select(&[], 0), None);
        assert_eq!(median(&[]), None);
        assert_eq!(select(&[42], 0), Some(42));
        // Heavy duplicates: many equal elements stress 3-way partition.
        let dup = vec![5i64; 100];
        assert_eq!(select(&dup, 50), Some(5));
        // Adversarial: already sorted and reverse-sorted inputs.
        let sorted: Vec<i64> = (0..200).collect();
        assert_eq!(select(&sorted, 199), Some(199));
        let mut rev = sorted.clone();
        rev.reverse();
        assert_eq!(select(&rev, 0), Some(0));
    }
}
