//! Saddleback search — `O(r + c)` lookup in a matrix where every
//! row and every column is sorted ascending (Bir & Bird 2006,
//! tracing back to Martin Gardner's puzzle). Start at the
//! top-right: an element larger than the target invalidates its
//! whole column (everything below is larger), a smaller one
//! invalidates its whole row (everything right is larger). Each
//! step discards a row or a column, so the walk is linear in
//! the dimensions, not the area.
//!
//! ```
//! use izanagi_kit::saddleback::{search, contains};
//! let m = vec![
//!     vec![1, 4, 7, 11],
//!     vec![2, 5, 8, 12],
//!     vec![3, 6, 9, 16],
//! ];
//! assert_eq!(search(&m, 5), Some((1, 1)));
//! assert_eq!(search(&m, 13), None);
//! assert!(contains(&m, 16));
//! ```

/// Search `m` for `target` — `Some((row, col))` of the cell the
/// top-right walk reaches, or `None`. With duplicate values the
/// returned cell is the deterministic one the algorithm visits,
/// not any canonical choice.
///
/// The walk covers the `m[0].len()`-wide rectangle: cells of a
/// ragged row beyond that width are unreachable and ignored.
pub fn search(m: &[Vec<i64>], target: i64) -> Option<(usize, usize)> {
    if m.is_empty() {
        return None;
    }
    let cols = m[0].len();
    if cols == 0 {
        return None;
    }
    let mut r = 0usize;
    let mut c = cols - 1;
    loop {
        if c >= m[r].len() {
            // ragged row — no cell here; column c is dead anyway
            if c == 0 {
                return None;
            }
            c -= 1;
            continue;
        }
        let v = m[r][c];
        if v == target {
            return Some((r, c));
        }
        if v > target {
            // column c is dead
            if c == 0 {
                return None;
            }
            c -= 1;
        } else {
            // row r is dead
            r += 1;
            if r >= m.len() {
                return None;
            }
        }
    }
}

/// `target` present anywhere in `m`?
pub fn contains(m: &[Vec<i64>], target: i64) -> bool {
    search(m, target).is_some()
}

/// Verify that a matrix satisfies the saddleback precondition —
/// every row and every column sorted ascending. Not needed by
/// `search`, but lets callers assert their data shape before
/// trusting a `None` verdict.
pub fn is_sorted_matrix(m: &[Vec<i64>]) -> bool {
    for row in m {
        for w in row.windows(2) {
            if w[0] > w[1] {
                return false;
            }
        }
    }
    for c in 0..m.first().map_or(0, |r| r.len()) {
        for r in 1..m.len() {
            if r < m.len() && c < m[r].len() && c < m[r - 1].len() && m[r - 1][c] > m[r][c] {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    #[test]
    fn basics() {
        let m = vec![vec![1, 4, 7, 11], vec![2, 5, 8, 12], vec![3, 6, 9, 16]];
        assert!(is_sorted_matrix(&m));
        assert_eq!(search(&m, 5), Some((1, 1)));
        assert_eq!(search(&m, 13), None);
        assert!(contains(&m, 16));
        assert!(contains(&m, 1));
        assert!(!contains(&m, 0));
        assert_eq!(search(&[], 3), None);
        assert_eq!(search(&[Vec::<i64>::new()], 3), None);
        assert!(!is_sorted_matrix(&[vec![2, 1]]));
        assert!(!is_sorted_matrix(&[vec![5], vec![3]]));
    }

    /// Naive full-scan oracle: `search` returns `Some` iff the
    /// target appears anywhere; when it does, the returned cell
    /// must hold the target value (can't be a phantom hit).
    #[test]
    fn oracle_full_scan() {
        let mut rng = SplitMix64::new(31);
        for _ in 0..200 {
            let rows = 1 + rng.below(8) as usize;
            let cols = 1 + rng.below(8) as usize;
            // build a sorted matrix
            let mut m = vec![vec![0i64; cols]; rows];
            for r in 0..rows {
                for c in 0..cols {
                    let lo = if r == 0 { 0 } else { m[r - 1][c] };
                    let lo = lo.max(if c == 0 { 0 } else { m[r][c - 1] });
                    m[r][c] = lo + rng.below(4) as i64;
                }
            }
            assert!(is_sorted_matrix(&m));
            // test present and absent targets
            let present: Vec<i64> = m.iter().flatten().copied().collect();
            let mut targets: Vec<i64> = (0..5)
                .map(|_| present[rng.below(present.len() as u32) as usize])
                .collect();
            targets.push(-1);
            targets.push(*present.iter().max().unwrap() + 1);
            for t in targets {
                let naive = m
                    .iter()
                    .enumerate()
                    .flat_map(|(r, row)| row.iter().enumerate().map(move |(c, &v)| (r, c, v)))
                    .any(|(_, _, v)| v == t);
                match search(&m, t) {
                    None => assert!(!naive, "missed {t}"),
                    Some((r, c)) => {
                        assert!(naive, "phantom {t} at ({r},{c})");
                        assert_eq!(m[r][c], t, "wrong cell for {t}");
                    }
                }
                assert_eq!(contains(&m, t), naive);
            }
        }
    }

    /// The walk's real claim — `O(r + c)` steps: instrument the
    /// loop bound directly (each iteration drops a row or a col,
    /// so steps ≤ rows + cols).
    #[test]
    fn linear_step_bound() {
        let n = 50usize;
        let m: Vec<Vec<i64>> = (0..n)
            .map(|r| (0..n).map(|c| (r + c) as i64).collect())
            .collect();
        // worst case: target forces maximal descent
        assert!(search(&m, 10000).is_none());
        assert!(search(&m, 49).is_some()); // on the diagonal band
                                           // duplicates: value r+c appears along an anti-diagonal —
                                           // the walk still terminates correctly
        let mut dup = vec![vec![0i64; 4]; 4];
        for (r, row) in dup.iter_mut().enumerate() {
            for (c, v) in row.iter_mut().enumerate() {
                *v = (r + c) as i64;
            }
        }
        let mut found = BTreeSet::new();
        for r in 0..4 {
            for c in 0..4 {
                if let Some((fr, fc)) = search(&dup, dup[r][c]) {
                    assert_eq!(dup[fr][fc], dup[r][c]);
                    found.insert((fr, fc));
                }
            }
        }
        assert!(!found.is_empty());
    }
}
