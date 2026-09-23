//! Dynamic time warping — the classic elastic distance between two
//! `i64` sequences. `dp[i][j]` is the cheapest warping cost aligning
//! `a[..i]` with `b[..j]`, where a step costs `|a[i−1] − b[j−1]|`
//! and the path may reuse either index (insert/delete/match).
//!
//! Everything is integer: no normalized costs, no squared norms —
//! the returned value is the exact sum of absolute differences
//! along the optimal path.
//!
//! ```
//! use izanagi_kit::dtw;
//! assert_eq!(dtw::distance(&[1, 2, 3], &[1, 2, 3]), 0);
//! // Shifted by one: |1−2| + |3−3|.
//! assert_eq!(dtw::distance(&[1, 3], &[2, 3]), 1);
//! ```

const INF: i64 = i64::MAX / 4;

/// Exact DTW distance — `i64::MAX` when either input is empty and
/// the other is not (no alignment exists); `0` for two empties.
pub fn distance(a: &[i64], b: &[i64]) -> i64 {
    let t = table(a, b, !0);
    let v = t[a.len()][b.len()];
    if v >= INF {
        i64::MAX
    } else {
        v
    }
}

/// Sakoe–Chiba banded variant — paths may wander at most `w` cells
/// off the diagonal; cells outside the band are unreachable.
/// Returns `i64::MAX` when no in-band alignment exists.
pub fn distance_windowed(a: &[i64], b: &[i64], w: usize) -> i64 {
    let t = table(a, b, w);
    let v = t[a.len()][b.len()];
    if v >= INF {
        i64::MAX
    } else {
        v
    }
}

/// Recover one optimal warping path as `(i, j)` index pairs from
/// `(0, 0)` to `(a.len(), b.len())`. Empty when inputs are empty.
pub fn path(a: &[i64], b: &[i64]) -> Vec<(u32, u32)> {
    let t = table(a, b, !0);
    let (mut i, mut j) = (a.len(), b.len());
    if t[i][j] >= INF {
        return Vec::new();
    }
    let mut out = vec![(i as u32, j as u32)];
    while i > 0 || j > 0 {
        let cur = t[i][j];
        let up = if i > 0 { t[i - 1][j] } else { INF };
        let left = if j > 0 { t[i][j - 1] } else { INF };
        let diag = if i > 0 && j > 0 { t[i - 1][j - 1] } else { INF };
        // Prefer diagonal (a real match) on ties — canonical path.
        if diag <= up && diag <= left && diag <= cur {
            i -= 1;
            j -= 1;
        } else if left <= up && left <= cur {
            j -= 1;
        } else {
            i -= 1;
        }
        out.push((i as u32, j as u32));
    }
    out.reverse();
    out
}

/// `dp` with `dp[0][0] = 0`, border = INF, band `w` (!0 =
/// full rectangle). Cost cell `(i, j)` = `|a[i−1] − b[j−1]|` plus
/// the min of up/left/diag.
fn table(a: &[i64], b: &[i64], w: usize) -> Vec<Vec<i64>> {
    let (n, m) = (a.len(), b.len());
    let mut t = vec![vec![INF; m + 1]; n + 1];
    t[0][0] = 0;
    for i in 0..=n {
        let jlo = i.saturating_sub(w);
        let jhi = if w == !0 { m } else { (i + w).min(m) };
        for j in jlo..=jhi {
            if i == 0 && j == 0 {
                continue;
            }
            if i == 0 || j == 0 {
                // Border cells stay INF — aligning an exhausted
                // prefix against a nonempty one is impossible.
                continue;
            }
            let cost = (a[i - 1] - b[j - 1]).unsigned_abs() as i64;
            let best = t[i - 1][j].min(t[i][j - 1]).min(t[i - 1][j - 1]);
            t[i][j] = best.saturating_add(cost).min(INF);
        }
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Independent memoized recursion — the oracle.
    fn rec(a: &[i64], b: &[i64], i: usize, j: usize, memo: &mut Vec<Vec<i64>>) -> i64 {
        if i == 0 && j == 0 {
            return 0;
        }
        if i == 0 || j == 0 {
            return INF; // exhausted prefix vs nonempty is unalignable
        }
        if memo[i][j] != -1 {
            return memo[i][j];
        }
        let cost = (a[i - 1] - b[j - 1]).abs();
        let best = rec(a, b, i - 1, j, memo)
            .min(rec(a, b, i, j - 1, memo))
            .min(rec(a, b, i - 1, j - 1, memo));
        memo[i][j] = best.saturating_add(cost).min(INF);
        memo[i][j]
    }

    fn oracle(a: &[i64], b: &[i64]) -> i64 {
        if a.is_empty() || b.is_empty() {
            return if a.is_empty() && b.is_empty() {
                0
            } else {
                i64::MAX
            };
        }
        let mut memo = vec![vec![-1i64; b.len() + 1]; a.len() + 1];
        rec(a, b, a.len(), b.len(), &mut memo)
    }

    #[test]
    fn basics() {
        assert_eq!(distance(&[1, 2, 3], &[1, 2, 3]), 0);
        assert_eq!(distance(&[1, 3], &[2, 3]), 1);
        assert_eq!(distance(&[], &[]), 0);
        assert_eq!(distance(&[1], &[]), i64::MAX);
        assert_eq!(distance(&[1, 2, 3], &[1, 1, 2, 3]), 0);
        // Classic example: |2−1| … optimal aligns (2,1)(3,2)(5,3).
        assert_eq!(distance(&[2, 3, 5], &[1, 2, 3]), 3);
    }

    #[test]
    fn windowed() {
        // Same sequences inside a wide band equal the full result.
        assert_eq!(distance_windowed(&[1, 2, 3], &[1, 2, 3], 3), 0);
        // Tight band on length-mismatched inputs can block everything.
        assert_eq!(distance_windowed(&[1, 2, 3, 4], &[9], 1), i64::MAX);
        assert_eq!(distance_windowed(&[1, 2, 3], &[1, 2, 3], 0), 0);
    }

    #[test]
    fn path_recovers() {
        let p = path(&[1, 2, 3], &[1, 1, 2, 3]);
        assert_eq!(p.first(), Some(&(0, 0)));
        assert_eq!(p.last(), Some(&(3, 4)));
        // Every step moves at least one coordinate, never backwards.
        for w in p.windows(2) {
            let ((i0, j0), (i1, j1)) = (w[0], w[1]);
            assert!(i1 >= i0 && j1 >= j0 && (i1 - i0) + (j1 - j0) >= 1);
            assert!(i1 - i0 <= 1 && j1 - j0 <= 1);
        }
        let p2 = path(&[5], &[7]);
        assert_eq!(p2, vec![(0, 0), (1, 1)]);
    }

    #[test]
    fn oracle_small() {
        let mut rng = SplitMix64::new(0x0e57_a11c_e55e_5a1e);
        for _case in 0..300 {
            let n = rng.below(9) as usize;
            let m = rng.below(9) as usize;
            let a: Vec<i64> = (0..n).map(|_| (rng.next_u64() % 20) as i64 - 10).collect();
            let b: Vec<i64> = (0..m).map(|_| (rng.next_u64() % 20) as i64 - 10).collect();
            assert_eq!(distance(&a, &b), oracle(&a, &b));
            // Wide-enough band must match the unbounded answer.
            let w = n.max(m);
            assert_eq!(distance_windowed(&a, &b, w), oracle(&a, &b));
        }
    }

    #[test]
    fn metric_properties() {
        let mut rng = SplitMix64::new(0x5eed_c0de_1234_5678);
        for _case in 0..80 {
            let n = 1 + rng.below(8) as usize;
            let mut gen = |len: usize| -> Vec<i64> {
                (0..len).map(|_| (rng.next_u64() % 30) as i64).collect()
            };
            let a = gen(n);
            let b = gen(n);
            assert_eq!(distance(&a, &b), distance(&b, &a));
            assert!(distance(&a, &b) >= 0);
            assert_eq!(distance(&a, &a), 0);
            // Upper bound: the identity path costs Σ|a_i − b_i|.
            let pointwise: i64 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum();
            assert!(distance(&a, &b) <= pointwise);
        }
    }
}
