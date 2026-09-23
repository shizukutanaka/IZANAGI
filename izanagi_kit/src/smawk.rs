//! SMAWK — `O(n + m)` row-argmin for totally monotone implicit
//! matrices (Aggarwal–Klawe–Moran–Shor–Wilber 1987). A matrix is
//! *totally monotone* when every `2x2` submatrix's minimum stays
//! on the same side: if `f(r, c) < f(r, c')` for `c < c'` then
//! every lower row also prefers `c`. Row minima of such a matrix
//! are non-decreasing, which SMAWK exploits to read only `O(n + m)`
//! entries — the workhorse behind Monge-DP speedups (`Knuth`,
//! `Aliens`, `divide & conquer` recurrences).
//!
//! The matrix is never materialized: `f(r, c)` is evaluated
//! lazily. Ties resolve to the *smallest* column index, so the
//! output is a pure function of `f` — no interior randomness.
//!
//! ```
//! use izanagi_kit::smawk::smawk;
//!
//! // Strictly convex quadratic — row minima march right.
//! let argmins = smawk(5, 5, |r, c| (r as i64 - c as i64) * (r as i64 - c as i64));
//! assert_eq!(argmins, vec![0, 1, 2, 3, 4]);
//! ```
//!
//! References: AKMSW '87; the "totally monotone" test used here is
//! the standard `f(r,c) < f(r,c')` reduction from the paper's
//! recursive Reduce+Interpolate structure.

/// Returns `argmin_c f(r, c)` for each row `r` of a totally
/// monotone implicit matrix — ties to the smallest `c`.
/// `rows`/`cols` are index ranges `0..rows`/`0..cols`.
///
/// Row-minima positions are guaranteed non-decreasing by the
/// monotonicity contract; when `f` violates it the answer is
/// still computed but no longer meaningful.
pub fn smawk<F>(rows: usize, cols: usize, f: F) -> Vec<usize>
where
    F: Fn(usize, usize) -> i64 + Copy,
{
    if cols == 0 {
        return vec![0usize; rows];
    }
    let row_idx: Vec<usize> = (0..rows).collect();
    let col_idx: Vec<usize> = (0..cols).collect();
    let mut out = vec![0usize; rows];
    solve(&row_idx, &col_idx, &f, &mut out);
    out
}

/// Reduce: drop columns that provably contain no row minimum.
/// Walks columns left-to-right maintaining a stack; when
/// `f(top_row, stack_top) > f(top_row, c)` the stacked column is
/// dominated for every remaining row and popped; when the stack
/// outgrows the remaining rows, extra columns can't win either.
fn reduce<F>(rows: &[usize], cols: &[usize], f: &F) -> Vec<usize>
where
    F: Fn(usize, usize) -> i64 + Copy,
{
    let mut stack: Vec<usize> = Vec::new();
    for &c in cols {
        // Pop while the stack top loses to c on the row aligned
        // with the stack's depth.
        while let Some(&top) = stack.last() {
            let r = rows[stack.len() - 1];
            // Strict `<`: on a tie the *lower* column index (stack
            // top, inserted earlier) keeps the minimum.
            if f(r, top) <= f(r, c) {
                break;
            }
            stack.pop();
        }
        if stack.len() < rows.len() {
            stack.push(c);
        }
    }
    stack
}

/// Recursive solve over explicit index lists.
fn solve<F>(rows: &[usize], cols: &[usize], f: &F, out: &mut [usize])
where
    F: Fn(usize, usize) -> i64 + Copy,
{
    if rows.is_empty() {
        return;
    }
    let kept = reduce(rows, cols, f);
    if rows.len() == 1 {
        // Single row: linear argmin over surviving columns.
        let r = rows[0];
        let mut bj = kept[0];
        for &c in &kept[1..] {
            if f(r, c) < f(r, bj) {
                bj = c;
            }
        }
        out[r] = bj;
        return;
    }
    // Recurse on odd-indexed rows.
    let odd_rows: Vec<usize> = rows.iter().skip(1).step_by(2).copied().collect();
    solve(&odd_rows, &kept, f, out);
    // Even rows' minima lie between their odd neighbors' answers.
    let mut k = 0usize; // position of current left bound in kept
    for (ri, &r) in rows.iter().step_by(2).enumerate() {
        // Left bound: argmin of the previous odd row (or first col).
        let lo_idx = if ri == 0 {
            0
        } else {
            pos_of(&kept, out[rows[2 * ri - 1]])
        };
        if lo_idx > k {
            k = lo_idx;
        }
        // Right bound: argmin of the next odd row (or last col).
        let hi_idx = if 2 * ri + 1 < rows.len() {
            pos_of(&kept, out[rows[2 * ri + 1]])
        } else {
            kept.len() - 1
        };
        let mut bj = kept[k];
        while k <= hi_idx {
            let c = kept[k];
            if f(r, c) < f(r, bj) {
                bj = c;
            }
            k += 1;
        }
        out[r] = bj;
        k -= 1; // k overshot hi_idx by one; rewind for next row
    }
}

/// First index of `v` inside `xs` — `kept` is a sorted column
/// list so this is a binary search.
fn pos_of(xs: &[usize], v: usize) -> usize {
    xs.binary_search(&v).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// A Monge matrix generator: `a[i] + b[j] + w[i]*x[j]` with
    /// sorted `w`/`x` yields total monotonicity.
    fn monge_matrix(rows: usize, cols: usize, rng: &mut SplitMix64) -> Vec<Vec<i64>> {
        let mut w: Vec<i64> = (0..rows).map(|_| (rng.next_u64() % 100) as i64).collect();
        let mut x: Vec<i64> = (0..cols).map(|_| (rng.next_u64() % 100) as i64).collect();
        w.sort_unstable();
        x.sort_unstable();
        let a: Vec<i64> = (0..rows).map(|_| (rng.next_u64() % 50) as i64).collect();
        let b: Vec<i64> = (0..cols).map(|_| (rng.next_u64() % 50) as i64).collect();
        // a[i]+b[j]-w[i]*x[j] is Monge (quadrangle <= 0) when
        // both w and x are sorted ascending — the product term
        // must be NEGATED: +w*x is the anti-Monge sign.
        (0..rows)
            .map(|i| (0..cols).map(|j| a[i] + b[j] - w[i] * x[j]).collect())
            .collect()
    }

    #[test]
    fn diagonal_quadratic() {
        assert_eq!(
            smawk(4, 4, |r, c| (r as i64 - c as i64) * (r as i64 - c as i64)),
            vec![0, 1, 2, 3]
        );
    }

    #[test]
    fn constant_matrix_picks_first_column() {
        assert_eq!(smawk(3, 6, |_, _| 7), vec![0, 0, 0]);
    }

    #[test]
    fn wide_matrix() {
        // more columns than rows: reduce must prune hard
        let v = smawk(2, 10, |r, c| (c as i64 - 5 * r as i64).abs());
        assert_eq!(v, vec![0, 5]);
    }

    #[test]
    fn matches_bruteforce_on_monge() {
        let mut rng = SplitMix64::new(0x5AA7);
        for _ in 0..300 {
            let rows = 1 + (rng.next_u64() % 10) as usize;
            let cols = 1 + (rng.next_u64() % 10) as usize;
            let m = monge_matrix(rows, cols, &mut rng);
            let got = smawk(rows, cols, |r, c| m[r][c]);
            let want: Vec<usize> = (0..rows)
                .map(|r| (0..cols).min_by_key(|&c| (m[r][c], c)).unwrap_or(0))
                .collect();
            assert_eq!(got, want, "matrix {m:?}");
            // Monotone non-decreasing argmins.
            for w in got.windows(2) {
                assert!(w[0] <= w[1]);
            }
        }
    }
}
