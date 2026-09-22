//! Largest-rectangle queries — the classic monotonic-stack histogram
//! problem and its lifted form over binary matrices.
//!
//! [`largest_rectangle`] answers the max-area bar rectangle in
//! `O(n)`; [`maximal_rectangle`] stacks the same routine per row to
//! find the largest all-`true` submatrix of a boolean grid in
//! `O(rows·cols)`.
//!
//! Both are integer-exact and order-deterministic: ties in area do
//! not affect the returned value (only the area is reported).
//!
//! ```
//! use izanagi_kit::histrect::{largest_rectangle, maximal_rectangle};
//! assert_eq!(largest_rectangle(&[2, 1, 5, 6, 2, 3]), 10); // 5x2 @ 2..4
//! let grid = vec![
//!     vec![true, true, false],
//!     vec![true, true, true],
//!     vec![true, true, true],
//! ];
//! assert_eq!(maximal_rectangle(&grid), 6); // rows 1..3 x cols 0..2
//! ```

/// Maximum rectangle area inscribed in the histogram `heights`.
/// Monotonic stack of strictly-increasing heights; each pop resolves
/// `height[i] × (right_boundary − left_boundary)` in `O(1)` amortized.
pub fn largest_rectangle(heights: &[u32]) -> u64 {
    // Sentinel-free two-pass structure: the stack holds indices of
    // increasing heights; when heights[j] < heights[stack_top], bar
    // stack_top's rectangle ends at j-1. A virtual bar of height 0 at
    // index n flushes the stack — implemented by iterating 0..=n.
    let n = heights.len();
    let mut stack: Vec<usize> = Vec::new();
    let mut best = 0u64;
    for j in 0..=n {
        let h = if j == n { 0 } else { heights[j] };
        while let Some(&top) = stack.last() {
            if heights[top] < h {
                break;
            }
            stack.pop();
            let height = heights[top] as u64;
            // Left boundary: first index after the new stack top
            // (that bar is strictly lower), else 0.
            let left = match stack.last() {
                Some(&l) => l + 1,
                None => 0,
            };
            best = best.max(height * (j - left) as u64);
        }
        stack.push(j);
    }
    best
}

/// Largest all-`true` axis-aligned rectangle inside `grid`, computed
/// as the running-height histogram over each row fed to
/// [`largest_rectangle`]. Ragged rows treat missing cells as `false`.
pub fn maximal_rectangle(grid: &[Vec<bool>]) -> u64 {
    let cols = grid.iter().map(Vec::len).max().unwrap_or(0);
    let mut heights = vec![0u32; cols];
    let mut best = 0u64;
    for row in grid {
        for (c, h) in heights.iter_mut().enumerate() {
            if row.get(c).copied().unwrap_or(false) {
                *h += 1;
            } else {
                *h = 0;
            }
        }
        best = best.max(largest_rectangle(&heights));
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Oracle: every pair of sides, take min height inside.
    fn oracle_hist(hs: &[u32]) -> u64 {
        let n = hs.len();
        let mut best = 0u64;
        for l in 0..n {
            let mut mn = u32::MAX;
            for (r, &h) in hs.iter().enumerate().take(n).skip(l) {
                mn = mn.min(h);
                best = best.max(mn as u64 * (r - l + 1) as u64);
            }
        }
        best
    }

    /// Oracle: every rectangle, check all-true.
    fn oracle_matrix(g: &[Vec<bool>]) -> u64 {
        let rows = g.len();
        let cols = g.iter().map(Vec::len).max().unwrap_or(0);
        let at = |r: usize, c: usize| {
            g.get(r)
                .and_then(|row| row.get(c))
                .copied()
                .unwrap_or(false)
        };
        let mut best = 0u64;
        for r1 in 0..rows {
            for r2 in r1..rows {
                let mut run = 0u64;
                for c in 0..cols {
                    if (r1..=r2).all(|r| at(r, c)) {
                        run += 1;
                        best = best.max(run * (r2 - r1 + 1) as u64);
                    } else {
                        run = 0;
                    }
                }
            }
        }
        best
    }

    #[test]
    fn histogram_matches_oracle() {
        let mut rng = SplitMix64::new(0xA1E4);
        for _ in 0..200 {
            let n = (rng.below(25) + 1) as usize;
            let hs: Vec<u32> = (0..n).map(|_| rng.below(15)).collect();
            assert_eq!(largest_rectangle(&hs), oracle_hist(&hs), "hs={hs:?}");
        }
    }

    #[test]
    fn matrix_matches_oracle() {
        let mut rng = SplitMix64::new(0xB1E4);
        for _ in 0..150 {
            let rows = (rng.below(8) + 1) as usize;
            let cols = (rng.below(9) + 1) as usize;
            let g: Vec<Vec<bool>> = (0..rows)
                .map(|_| (0..cols).map(|_| rng.below(3) != 0).collect())
                .collect();
            assert_eq!(maximal_rectangle(&g), oracle_matrix(&g), "g={g:?}");
        }
    }

    #[test]
    fn known_and_edge_cases() {
        assert_eq!(largest_rectangle(&[]), 0);
        assert_eq!(largest_rectangle(&[7]), 7);
        assert_eq!(largest_rectangle(&[3, 3, 3]), 9);
        assert_eq!(largest_rectangle(&[5, 1, 5]), 5);
        assert_eq!(largest_rectangle(&[1, 2, 3, 4, 5]), 9);
        assert_eq!(maximal_rectangle(&[]), 0);
        assert_eq!(maximal_rectangle(&[vec![false]]), 0);
        assert_eq!(maximal_rectangle(&[vec![true, true], vec![true, false]]), 2);
        // Ragged rows: missing cells count as false.
        assert_eq!(maximal_rectangle(&[vec![true, true], vec![true]]), 2);
    }
}
