//! Algorithm X — Knuth's exact-cover solver (the logical core of
//! Dancing Links).
//!
//! Given a universe of `n_cols` items and candidate `rows` each covering
//! a subset, find a selection of rows that covers every item exactly
//! once. Classic applications: Sudoku, polyomino packing, tile/exact-
//! placement puzzles, and (with secondary columns skipped here)
//! constraint layouts.
//!
//! Determinism: the branching column is the one with the fewest
//! available rows (lowest index on ties), and candidate rows are tried
//! in input order — the returned solution is therefore the first one a
//! depth-first reader would reach, and the row ids are returned sorted.
//!
//! ```
//! use izanagi_kit::dlx::exact_cover;
//! // Universe {0,1,2,3}; rows: A={0,2} B={1,3} C={0,1} D={2,3}.
//! let rows = [[0usize, 2].as_slice(), [1, 3].as_slice(),
//!             [0, 1].as_slice(), [2, 3].as_slice()];
//! let sol = exact_cover(4, &rows).unwrap();
//! assert_eq!(sol, vec![0, 1]); // rows 0 and 1 tile the universe
//! ```

/// Finds the lexicographically-first (by search order) exact cover of
/// `0..n_cols` by `rows`. Each `rows[i]` is a set of column indices —
/// duplicates inside a row make it unusable and are treated as absent.
///
/// Returns the sorted ids of the chosen rows, or `None` when no exact
/// cover exists. `n_cols == 0` yields the empty solution `Some(vec![])`
/// immediately. Columns outside `0..n_cols` inside a row are ignored;
/// an empty row can never be part of a solution.
pub fn exact_cover(n_cols: usize, rows: &[&[usize]]) -> Option<Vec<u32>> {
    // col_rows[c] = row indices (ascending) covering column c.
    let mut col_rows: Vec<Vec<u32>> = vec![Vec::new(); n_cols];
    // row_cols[r] = sorted unique columns of row r.
    let mut row_cols: Vec<Vec<u32>> = vec![Vec::new(); rows.len()];
    for (r, row) in rows.iter().enumerate() {
        let mut cs: Vec<u32> = row
            .iter()
            .filter(|&&c| c < n_cols)
            .map(|&c| c as u32)
            .collect();
        cs.sort_unstable();
        cs.dedup();
        for &c in &cs {
            col_rows[c as usize].push(r as u32);
        }
        row_cols[r] = cs;
    }
    let mut state = Search {
        n_cols,
        col_rows: &col_rows,
        row_cols: &row_cols,
        // No column is satisfied yet.
        covered: vec![false; n_cols],
        // Row is usable (its columns not yet covered by a chosen row).
        row_free: vec![true; rows.len()],
        chosen: Vec::new(),
        journal: Vec::new(),
    };
    if state.solve() {
        let mut out: Vec<u32> = state.chosen;
        out.sort_unstable();
        return Some(out);
    }
    None
}

struct Search<'a> {
    n_cols: usize,
    col_rows: &'a [Vec<u32>],
    row_cols: &'a [Vec<u32>],
    /// Column satisfied by a chosen row.
    covered: Vec<bool>,
    row_free: Vec<bool>,
    chosen: Vec<u32>,
    /// Rows disabled by each `cover`, for exact `uncover` inversion.
    journal: Vec<Vec<u32>>,
}

impl Search<'_> {
    /// Depth-first Algorithm X. `cover/uncover` mutate `covered` and
    /// `row_free` and are exact inverses, applied in LIFO order.
    fn solve(&mut self) -> bool {
        // All columns covered → success.
        if self.covered.iter().all(|&b| b) {
            return true;
        }
        // Pick the uncovered column with the fewest available rows —
        // lowest index on ties. Zero rows means dead end.
        let mut pick = None;
        let mut pick_n = u32::MAX;
        for c in 0..self.n_cols {
            if !self.covered[c] {
                let free = self.col_rows[c]
                    .iter()
                    .filter(|&&r| self.row_free[r as usize])
                    .count() as u32;
                if free == 0 {
                    return false; // uncovered column, no candidate row
                }
                if free < pick_n {
                    pick_n = free;
                    pick = Some(c);
                }
            }
        }
        let col = match pick {
            Some(c) => c,
            None => return true, // nothing left uncovered
        };
        for i in 0..self.col_rows[col].len() {
            let r = self.col_rows[col][i];
            if !self.row_free[r as usize] {
                continue;
            }
            self.cover(r);
            if self.solve() {
                return true;
            }
            self.uncover(r);
        }
        false
    }

    /// Select row `r`: every column it covers becomes satisfied and
    /// every still-free row sharing one of those columns is disabled
    /// (it could no longer be chosen without double-covering). The
    /// disabled rows are journaled for `uncover`.
    fn cover(&mut self, r: u32) {
        self.chosen.push(r);
        let mut disabled = Vec::new();
        for &c in &self.row_cols[r as usize].clone() {
            self.covered[c as usize] = true;
            for &other in &self.col_rows[c as usize].clone() {
                if self.row_free[other as usize] {
                    self.row_free[other as usize] = false;
                    disabled.push(other);
                }
            }
        }
        self.journal.push(disabled);
    }

    fn uncover(&mut self, r: u32) {
        for &c in self.row_cols[r as usize].clone().iter().rev() {
            self.covered[c as usize] = false;
        }
        let disabled = self.journal.pop();
        let disabled = match disabled {
            Some(d) => d,
            None => return,
        };
        for &other in disabled.iter().rev() {
            self.row_free[other as usize] = true;
        }
        self.chosen.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Oracle: brute-force over row subsets — find ANY exact cover.
    fn oracle_solvable(n_cols: usize, rows: &[&[usize]]) -> bool {
        let k = rows.len();
        for mask in 0..(1u64 << k) {
            let mut cover = vec![0u32; n_cols];
            for (r, row) in rows.iter().enumerate() {
                if mask & (1 << r) != 0 {
                    for &c in *row {
                        if c < n_cols {
                            cover[c] += 1;
                        }
                    }
                }
            }
            if cover.iter().all(|&x| x == 1) {
                return true;
            }
        }
        false
    }

    fn valid_cover(n_cols: usize, rows: &[&[usize]], sol: &[u32]) -> bool {
        let mut cover = vec![0u32; n_cols];
        for &r in sol {
            if (r as usize) >= rows.len() {
                return false;
            }
            for &c in rows[r as usize] {
                if c >= n_cols || cover[c] != 0 {
                    return false;
                }
                cover[c] += 1;
            }
        }
        cover.iter().all(|&x| x == 1)
    }

    #[test]
    fn solvable_iff_oracle_says_so() {
        let mut rng = SplitMix64::new(0xD1E7);
        for _ in 0..200 {
            let n_cols = (rng.below(6) + 1) as usize;
            let k = (rng.below(8) + 1) as usize;
            let mut rows: Vec<Vec<usize>> = Vec::new();
            for _ in 0..k {
                let mut row = Vec::new();
                for c in 0..n_cols {
                    if rng.below(3) == 0 {
                        row.push(c);
                    }
                }
                rows.push(row);
            }
            let rrefs: Vec<&[usize]> = rows.iter().map(|r| r.as_slice()).collect();
            let got = exact_cover(n_cols, &rrefs);
            assert_eq!(
                got.is_some(),
                oracle_solvable(n_cols, &rrefs),
                "n_cols={n_cols} rows={rows:?}"
            );
            if let Some(sol) = got {
                assert!(valid_cover(n_cols, &rrefs, &sol));
            }
        }
    }

    #[test]
    fn first_solution_is_deterministic() {
        // Two tiling solutions exist: {A,B} and {C,D}; {0,1} wins.
        let rows = [
            [0usize, 2].as_slice(),
            [1, 3].as_slice(),
            [0, 1].as_slice(),
            [2, 3].as_slice(),
        ];
        assert_eq!(exact_cover(4, &rows).unwrap(), vec![0, 1]);
        // Rows tried in input order: earlier ids preferred.
        let rows2 = [
            [2usize, 3].as_slice(), // D first
            [0, 1].as_slice(),      // C
            [1, 3].as_slice(),      // B
            [0, 2].as_slice(),      // A
        ];
        let sol = exact_cover(4, &rows2).unwrap();
        assert!(valid_cover(4, &rows2, &sol));
        // Search still finds the {C,D}={0,1} cover because column 1
        // has candidates {0(C),2(B)} — C is row 1 < B's row 2.
        assert_eq!(sol, vec![0, 1]);
    }

    #[test]
    fn edge_cases() {
        // Empty universe.
        assert_eq!(exact_cover(0, &[]).unwrap(), Vec::<u32>::new());
        // No rows for a nonempty universe.
        assert!(exact_cover(3, &[]).is_none());
        // Uncoverable column.
        let rows = [[0usize].as_slice()];
        assert!(exact_cover(2, &rows).is_none());
        // Out-of-range columns are ignored.
        let rows = [[0usize, 99].as_slice(), [1].as_slice()];
        assert_eq!(exact_cover(2, &rows).unwrap(), vec![0, 1]);
        // Duplicate cells inside a row are harmless.
        let rows = [[0usize, 0, 1].as_slice()];
        assert_eq!(exact_cover(2, &rows).unwrap(), vec![0]);
    }
}
