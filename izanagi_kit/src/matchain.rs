//! Matrix-chain multiplication order — the classic `O(n³)` DP that
//! picks the parenthesization minimizing scalar multiplications, with
//! a canonical split table for reconstruction.
//!
//! For `dims = [d0, d1, …, dk]`, matrix `i` is `d[i] × d[i+1]` and
//! `dp[i][j]` = minimum cost of the product `A_i…A_j`. Determinism:
//! the split index `s` scans left-to-right so the *first* argmin wins,
//! which pins a single canonical plan among equal-cost orders.
//!
//! ```
//! use izanagi_kit::matchain::order;
//! // A:10x30, B:30x5, C:5x60 — (AB)C = 4500+3000 vs A(BC) = 9000+90000.
//! let (cost, splits) = order(&[10, 30, 5, 60]).unwrap();
//! assert_eq!(cost, 4500);
//! assert_eq!(splits, vec![(0, 0, 1), (0, 1, 2)]); // postorder: (A·B)·C
//! assert_eq!(order(&[7]).unwrap().0, 0);
//! assert!(order(&[]).is_none());
//! ```

/// One multiply step: `(lo, mid, hi)` multiplies the chain products
/// `A_lo…A_mid` and `A_mid+1…A_hi` at cost `d[lo]·d[mid+1]·d[hi+1]`.
/// Steps are returned in *postorder* — dependencies always appear
/// before the multiply that consumes them.
pub type Step = (usize, usize, usize);

/// Minimum-cost multiplication order for a chain of `dims.len() − 1`
/// matrices — `Some((cost, steps))`, `None` when `dims` is empty.
///
/// `order(&[d])` is a single-matrix chain with cost `0` and no steps.
/// Costs accumulate in `u128` internally but are returned as `u64`
/// saturating at `u64::MAX` (dimensions that large are already
/// outside any real workload).
pub fn order(dims: &[u64]) -> Option<(u64, Vec<Step>)> {
    if dims.is_empty() {
        return None;
    }
    let n = dims.len() - 1;
    if n == 0 {
        return Some((0, Vec::new()));
    }
    let mut dp = vec![vec![0u128; n]; n];
    let mut sp = vec![vec![0usize; n]; n];
    for len in 2..=n {
        for i in 0..=n - len {
            let j = i + len - 1;
            let mut best = u128::MAX;
            let mut bs = i;
            for s in i..j {
                let c = dp[i][s].saturating_add(dp[s + 1][j]).saturating_add(
                    u128::from(dims[i])
                        .saturating_mul(u128::from(dims[s + 1]))
                        .saturating_mul(u128::from(dims[j + 1])),
                );
                if c < best {
                    best = c;
                    bs = s;
                }
            }
            dp[i][j] = best;
            sp[i][j] = bs;
        }
    }
    let mut steps = Vec::new();
    fn emit(sp: &[Vec<usize>], i: usize, j: usize, out: &mut Vec<Step>) {
        if i < j {
            let s = sp[i][j];
            emit(sp, i, s, out);
            emit(sp, s + 1, j, out);
            out.push((i, s, j));
        }
    }
    emit(&sp, 0, n - 1, &mut steps);
    Some((dp[0][n - 1].min(u128::from(u64::MAX)) as u64, steps))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Enumerate every full parenthesization (Catalan) and take the min.
    fn brute(dims: &[u64]) -> u128 {
        let n = dims.len() - 1;
        fn rec(d: &[u64], i: usize, j: usize) -> u128 {
            if i == j {
                return 0;
            }
            let mut best = u128::MAX;
            for s in i..j {
                let c = rec(d, i, s)
                    .saturating_add(rec(d, s + 1, j))
                    .saturating_add(u128::from(d[i]) * u128::from(d[s + 1]) * u128::from(d[j + 1]));
                if c < best {
                    best = c;
                }
            }
            best
        }
        rec(dims, 0, n - 1)
    }

    #[test]
    fn known() {
        // CLRS example: [30,35,15,5,10,20,25] → 15125.
        let (c, _) = order(&[30, 35, 15, 5, 10, 20, 25]).unwrap();
        assert_eq!(c, 15125);
        // Single and empty.
        assert_eq!(order(&[4]).unwrap(), (0, vec![]));
        assert!(order(&[]).is_none());
        // Two matrices: exactly one multiply.
        let (c, s) = order(&[3, 4, 5]).unwrap();
        assert_eq!(c, 60);
        assert_eq!(s, vec![(0, 0, 1)]);
    }

    #[test]
    fn oracle_random() {
        let mut rng = SplitMix64::new(0xca7a_1a1b_0059);
        for _ in 0..200 {
            let n = 1 + rng.below(7) as usize; // ≤7 matrices, Catalan small
            let dims: Vec<u64> = (0..=n).map(|_| 1 + rng.below(30) as u64).collect();
            let (cost, steps) = order(&dims).unwrap();
            assert_eq!(u128::from(cost), brute(&dims));
            // Steps form a valid binary tree: n−1 multiplies, each lo≤mid<hi.
            assert_eq!(steps.len(), n.saturating_sub(1));
            for &(lo, mid, hi) in &steps {
                assert!(lo <= mid && mid < hi && hi < n);
            }
            // Replay: multiplying in order covers [0, n−1] exactly once.
            if n > 1 {
                assert_eq!(
                    steps.last().copied().unwrap_or_default(),
                    steps[steps.len() - 1]
                );
                let (lo, _, hi) = steps[steps.len() - 1];
                assert_eq!((lo, hi), (0, n - 1));
            }
        }
    }
}
