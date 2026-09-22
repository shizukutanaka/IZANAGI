//! The Hungarian (Kuhn–Munkres) assignment solver — minimum-cost
//! worker→task matching on an `n × m` cost matrix (`n ≤ m`).
//!
//! `O(n³)` potential method in the classic e-maxx form, adapted to `i64`
//! costs (negatives welcome — a profit matrix is just a negated one) and
//! deterministic scan order: rows augment in order, columns scan
//! left-to-right, so among multiple optimal assignments the output is a
//! pure function of the matrix.
//!
//! ```
//! use izanagi_kit::hungarian::assign_min_cost;
//! // Two workers, three tasks — cheaper to give job 0→worker0, 2→worker1.
//! let cost = vec![vec![4, 9, 8], vec![7, 6, 3]];
//! assert_eq!(assign_min_cost(&cost), Some(vec![0, 2]));
//! ```

/// Minimum-cost assignment of every row to a distinct column. Returns the
/// column chosen for each row (length `n`), or `None` when `n > m`
/// (columns can't be reused) or the matrix is ragged/empty.
///
/// When several assignments share the optimum, the lexicographically
/// smallest column vector is *not* separately guaranteed — the result is
/// still fully deterministic (fixed scan order), just not tied to a
/// documented secondary criterion.
pub fn assign_min_cost(cost: &[Vec<i64>]) -> Option<Vec<usize>> {
    let n = cost.len();
    let m = cost.first().map_or(0, |r| r.len());
    if n == 0 || n > m || cost.iter().any(|r| r.len() != m) {
        return None;
    }
    // 1-indexed potentials scheme (e-maxx formulation).
    let mut u = vec![0i64; n + 1];
    let mut v = vec![0i64; m + 1];
    let mut p = vec![0usize; m + 1]; // column → matched row (0 = free)
    let mut way = vec![0usize; m + 1];
    for i in 1..=n {
        p[0] = i;
        let mut j0 = 0usize;
        let mut minv = vec![i64::MAX; m + 1];
        let mut used = vec![false; m + 1];
        loop {
            used[j0] = true;
            let i0 = p[j0];
            let mut delta = i64::MAX;
            let mut j1 = 0usize;
            for j in 1..=m {
                if used[j] {
                    continue;
                }
                let cur = cost[i0 - 1][j - 1] - u[i0] - v[j];
                if cur < minv[j] {
                    minv[j] = cur;
                    way[j] = j0;
                }
                if minv[j] < delta {
                    delta = minv[j];
                    j1 = j;
                }
            }
            for j in 0..=m {
                if used[j] {
                    u[p[j]] += delta;
                    v[j] -= delta;
                } else {
                    minv[j] -= delta;
                }
            }
            j0 = j1;
            if p[j0] == 0 {
                break;
            }
        }
        // Augment along the alternating path.
        while j0 != 0 {
            p[j0] = p[way[j0]];
            j0 = way[j0];
        }
    }
    // p[j] = row matched to column j; invert to row → column.
    let mut answer = vec![0usize; n];
    for j in 1..=m {
        if p[j] != 0 {
            answer[p[j] - 1] = j - 1;
        }
    }
    Some(answer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Brute-force oracle: the true minimum total cost over all injective
    /// row→column maps (n small).
    fn oracle_min(cost: &[Vec<i64>]) -> i64 {
        let m = cost[0].len();
        let mut best = i64::MAX;
        // Enumerate injections rows→columns recursively.
        fn rec(cost: &[Vec<i64>], row: usize, used: &mut [bool], acc: i64, best: &mut i64) {
            let n = cost.len();
            let m = cost[0].len();
            if row == n {
                *best = (*best).min(acc);
                return;
            }
            for c in 0..m {
                if !used[c] {
                    used[c] = true;
                    rec(cost, row + 1, used, acc + cost[row][c], best);
                    used[c] = false;
                }
            }
        }
        let mut used = vec![false; m];
        rec(cost, 0, &mut used, 0, &mut best);
        best
    }

    fn total(cost: &[Vec<i64>], a: &[usize]) -> i64 {
        a.iter().enumerate().map(|(r, &c)| cost[r][c]).sum()
    }

    #[test]
    fn known_matrices() {
        // Diagonal is optimal.
        let cost = vec![vec![1, 9, 9], vec![9, 1, 9]];
        assert_eq!(assign_min_cost(&cost), Some(vec![0, 1]));
        // Classic 3×3.
        let cost = vec![vec![4, 1, 3], vec![2, 0, 5], vec![3, 2, 2]];
        let a = assign_min_cost(&cost).unwrap_or_default();
        assert_eq!(total(&cost, &a), oracle_min(&cost));
        // Negative costs work (negated profit matrix).
        let cost = vec![vec![-5, -1, -3], vec![-2, -4, -6]];
        let a = assign_min_cost(&cost).unwrap_or_default();
        assert_eq!(total(&cost, &a), oracle_min(&cost));
    }

    #[test]
    fn rejects_bad_shapes() {
        assert_eq!(assign_min_cost(&[]), None);
        assert_eq!(assign_min_cost(&[vec![1, 2], vec![3, 4], vec![5, 6]]), None);
        assert_eq!(assign_min_cost(&[vec![1, 2], vec![3]]), None);
    }

    #[test]
    fn matches_brute_force_on_random_matrices() {
        let mut rng = SplitMix64::new(0xA551);
        for _ in 0..200 {
            let n = rng.below(4) as usize + 1;
            let m = n + rng.below(4) as usize;
            let cost: Vec<Vec<i64>> = (0..n)
                .map(|_| (0..m).map(|_| rng.range(-9, 10) as i64).collect())
                .collect();
            let a = assign_min_cost(&cost).expect("n ≤ m matrix");
            // Validity: distinct columns, in range.
            let mut cols = a.clone();
            cols.sort_unstable();
            cols.dedup();
            assert_eq!(cols.len(), n);
            assert_eq!(total(&cost, &a), oracle_min(&cost));
        }
    }

    #[test]
    fn deterministic() {
        let cost = vec![vec![2, 4, 1, 7], vec![3, 5, 9, 2], vec![8, 1, 4, 3]];
        assert_eq!(assign_min_cost(&cost), assign_min_cost(&cost));
    }
}
