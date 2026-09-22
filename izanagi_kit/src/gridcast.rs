//! Grid ray traversal (DDA) — walks every cell a segment enters, in order.
//!
//! Algorithm: the voxel/grid traversal of Amanatides & Woo, *"A Fast Voxel
//! Traversal Algorithm for Ray Tracing"*, Eurographics'87 — the standard
//! laser/line-of-sight walk: at each step, step along whichever axis's
//! boundary comes first in ray parameter `t`. Unlike `geometry`'s Bresenham
//! (which picks one cell per column along a rasterized line), this visits
//! *every* cell the continuous segment passes through — a ray grazing a
//! cell corner enters the cells on both sides of it.
//!
//! Integer-exactness:
//!
//! * Endpoints are cell **centres** — `grid_ray(a, b)` is the ray from the
//!   centre of cell `a` to the centre of cell `b`. All arithmetic runs on
//!   doubled coordinates (`2c + 1`), so boundaries sit at even integers and
//!   no halves appear anywhere.
//! * The next-boundary comparison `tMaxX < tMaxY` is a cross-multiplied
//!   `i128` comparison — no division, no floats.
//! * When a ray exits a cell exactly through a corner (`tMaxX == tMaxY`),
//!   it enters only the diagonally-adjacent cell — the two side cells get
//!   zero-length intersection and are skipped. This keeps every consecutive
//!   pair of cells edge- or corner-adjacent and makes the walk's length
//!   minimal.
//!
//! ```
//! use izanagi_kit::gridcast::grid_ray;
//! // (0,0) → (2,2) passes through corners: only the diagonal cells count.
//! assert_eq!(grid_ray(0, 0, 2, 2), vec![(0, 0), (1, 1), (2, 2)]);
//! ```

/// All cells a segment from `a` to `b` passes through, in entry order —
/// including both endpoint cells. `grid_ray(a, b)` == `grid_ray(b, a)`
/// reversed: the traversal is symmetric.
///
/// Cost is `O(|Δx| + |Δy|)` cells — exactly the cells entered.
pub fn grid_ray(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
    // Doubled coordinates: cell (x,y) covers [2x, 2x+2) × [2y, 2y+2);
    // endpoints are the odd-valued centres.
    let (px0, py0) = (2 * x0 as i64 + 1, 2 * y0 as i64 + 1);
    let (px1, py1) = (2 * x1 as i64 + 1, 2 * y1 as i64 + 1);
    let dx = px1 - px0;
    let dy = py1 - py0;
    let step_x: i64 = if dx > 0 { 1 } else { -1 };
    let step_y: i64 = if dy > 0 { 1 } else { -1 };
    let adx = dx.unsigned_abs();
    let ady = dy.unsigned_abs();
    // Next x/y boundary coordinate (even). Stepping +1 enters the cell above
    // the boundary at px rounded up to even; stepping -1 enters the cell
    // below the boundary at px rounded down to even.
    let mut bx = if dx > 0 {
        px0 + 1 // next even above the odd centre
    } else {
        px0 - 1
    };
    let mut by = if dy > 0 { py0 + 1 } else { py0 - 1 };

    let mut out = Vec::with_capacity((adx + ady) as usize / 2 + 2);
    let (mut cx, mut cy) = (x0, y0);
    out.push((cx, cy));
    // Each iteration crosses at least one boundary, so at most |Δx|+|Δy|
    // steps; the bound also guards against any degenerate-capture bug.
    let max_steps = (x1 as i64 - x0 as i64).unsigned_abs() + (y1 as i64 - y0 as i64).unsigned_abs();
    for _ in 0..max_steps {
        if cx == x1 && cy == y1 {
            break;
        }
        // Compare t at next x-boundary vs next y-boundary:
        //   tX = (bx - px0)/dx   vs   tY = (by - py0)/dy
        // with i128 cross-multiplication (denominators are adx/ady; the
        // zero-denominator axes can never win).
        let txw = dx != 0;
        let tyw = dy != 0;
        let (take_x, take_y) = match (txw, tyw) {
            (true, true) => {
                // Distances to the next boundary are always positive —
                // take magnitudes so the same code serves ± directions.
                let lhs = (bx - px0).unsigned_abs() as i128 * ady as i128;
                let rhs = (by - py0).unsigned_abs() as i128 * adx as i128;
                // Equal: corner exit → diagonal step (enters only the cell
                // across the corner).
                (lhs <= rhs, rhs <= lhs)
            }
            (true, false) => (true, false),
            (false, true) => (false, true),
            (false, false) => break,
        };
        if take_x {
            cx += step_x as i32;
            bx += 2 * step_x;
        }
        if take_y {
            cy += step_y as i32;
            by += 2 * step_y;
        }
        out.push((cx, cy));
    }
    out
}

/// The first cell along `grid_ray(x0..x1, y0..y1)` where `blocked` reports
/// true, or `None` when the whole path is clear. `blocked` receives the
/// endpoint cells too — callers wanting pure LOS should whitelist endpoints
/// in the predicate.
///
/// ```
/// use izanagi_kit::gridcast::ray_blocked_at;
/// // Wall at (1,0) blocks a straight ray.
/// assert_eq!(ray_blocked_at(0, 0, 3, 0, |x, y| (x, y) == (1, 0)), Some((1, 0)));
/// assert_eq!(ray_blocked_at(0, 0, 3, 0, |_, _| false), None);
/// ```
pub fn ray_blocked_at<B>(x0: i32, y0: i32, x1: i32, y1: i32, mut blocked: B) -> Option<(i32, i32)>
where
    B: FnMut(i32, i32) -> bool,
{
    grid_ray(x0, y0, x1, y1)
        .into_iter()
        .find(|&(x, y)| blocked(x, y))
}

/// Symmetric visibility: `true` iff no strictly-interior cell of the ray is
/// blocked (endpoint cells are exempt — a ray may start or end inside a
/// wall without that wall blocking sight to itself).
///
/// ```
/// use izanagi_kit::gridcast::clear_los;
/// assert!(clear_los(0, 0, 3, 0, |_, _| false));
/// assert!(!clear_los(0, 0, 3, 0, |x, y| (x, y) == (1, 0)));
/// assert!(clear_los(0, 0, 3, 0, |x, y| (x, y) == (3, 0))); // endpoint wall
/// ```
pub fn clear_los<B>(x0: i32, y0: i32, x1: i32, y1: i32, mut blocked: B) -> bool
where
    B: FnMut(i32, i32) -> bool,
{
    let ray = grid_ray(x0, y0, x1, y1);
    ray[1..ray.len().saturating_sub(1)]
        .iter()
        .all(|&(x, y)| !blocked(x, y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    /// Independent oracle: every cell the segment *truly* passes through is
    /// found by a slab test on doubled coordinates — cell (i,j) covers the
    /// open box (2i, 2i+2) × (2j, 2j+2), and counts iff the segment's
    /// intersection with it has positive length. Corner touches produce a
    /// zero-length intersection and must NOT count. Fractions are stored
    /// sign-normalised (`norm` flips both parts of a negative denominator).
    fn oracle_cells(x0: i32, y0: i32, x1: i32, y1: i32) -> BTreeSet<(i32, i32)> {
        let (px0, py0) = (2 * x0 as i64 + 1, 2 * y0 as i64 + 1);
        let (px1, py1) = (2 * x1 as i64 + 1, 2 * y1 as i64 + 1);
        let dx = px1 - px0;
        let dy = py1 - py0;
        let mut cells = BTreeSet::new();
        for i in x0.min(x1) - 1..=x0.max(x1) + 1 {
            for j in y0.min(y1) - 1..=y0.max(y1) + 1 {
                // Per-axis entry/exit parameters as (num, den) fractions,
                // den normalised positive — cross-multiply for comparisons.
                let norm = |n: i64, d: i64| if d < 0 { (-n, -d) } else { (n, d) };
                let (lo_xn, lo_xd, hi_xn, hi_xd) = if dx == 0 {
                    // Segment runs parallel to x-slabs: inside this cell's
                    // x-range iff px0 is strictly between the boundaries.
                    if 2 * i as i64 >= px0 || px0 >= 2 * i as i64 + 2 {
                        continue;
                    }
                    (0i64, 1i64, 1i64, 1i64) // whole [0,1]
                } else {
                    let ta = norm(2 * i as i64 - px0, dx);
                    let tb = norm(2 * i as i64 + 2 - px0, dx);
                    if (ta.0 as i128) * (tb.1 as i128) < (tb.0 as i128) * (ta.1 as i128) {
                        (ta.0, ta.1, tb.0, tb.1)
                    } else {
                        (tb.0, tb.1, ta.0, ta.1)
                    }
                };
                let (lo_yn, lo_yd, hi_yn, hi_yd) = if dy == 0 {
                    if 2 * j as i64 >= py0 || py0 >= 2 * j as i64 + 2 {
                        continue;
                    }
                    (0i64, 1i64, 1i64, 1i64)
                } else {
                    let ta = norm(2 * j as i64 - py0, dy);
                    let tb = norm(2 * j as i64 + 2 - py0, dy);
                    if (ta.0 as i128) * (tb.1 as i128) < (tb.0 as i128) * (ta.1 as i128) {
                        (ta.0, ta.1, tb.0, tb.1)
                    } else {
                        (tb.0, tb.1, ta.0, ta.1)
                    }
                };
                // lo = max(lo_x, lo_y, 0); hi = min(hi_x, hi_y, 1); positive
                // length iff lo < hi (strict — equality is a point touch).
                let le = |an: i64, ad: i64, bn: i64, bd: i64| -> i32 {
                    let l = an as i128 * bd as i128;
                    let r = bn as i128 * ad as i128;
                    if l < r {
                        -1
                    } else if l > r {
                        1
                    } else {
                        0
                    }
                };
                let mut lo = (0i64, 1i64);
                if le(lo_xn, lo_xd, lo.0, lo.1) > 0 {
                    lo = (lo_xn, lo_xd);
                }
                if le(lo_yn, lo_yd, lo.0, lo.1) > 0 {
                    lo = (lo_yn, lo_yd);
                }
                let mut hi = (1i64, 1i64);
                if le(hi_xn, hi_xd, hi.0, hi.1) < 0 {
                    hi = (hi_xn, hi_xd);
                }
                if le(hi_yn, hi_yd, hi.0, hi.1) < 0 {
                    hi = (hi_yn, hi_yd);
                }
                if le(lo.0, lo.1, hi.0, hi.1) < 0 {
                    cells.insert((i, j));
                }
            }
        }
        cells
    }

    #[test]
    fn straight_and_diagonal_rays() {
        assert_eq!(grid_ray(0, 0, 3, 0), vec![(0, 0), (1, 0), (2, 0), (3, 0)]);
        assert_eq!(grid_ray(0, 0, 0, 2), vec![(0, 0), (0, 1), (0, 2)]);
        // Exact diagonal: corner exits enter only the diagonal cell.
        assert_eq!(grid_ray(0, 0, 2, 2), vec![(0, 0), (1, 1), (2, 2)]);
        assert_eq!(grid_ray(2, 0, 0, 2), vec![(2, 0), (1, 1), (0, 2)]);
        assert_eq!(grid_ray(5, 5, 5, 5), vec![(5, 5)]);
    }

    #[test]
    fn ray_is_symmetric() {
        let mut rng = SplitMix64::new(0xCA57);
        for _ in 0..200 {
            let (x0, y0) = (rng.range(-20, 20), rng.range(-20, 20));
            let (x1, y1) = (rng.range(-20, 20), rng.range(-20, 20));
            let fwd = grid_ray(x0, y0, x1, y1);
            let mut rev = grid_ray(x1, y1, x0, y0);
            rev.reverse();
            assert_eq!(fwd, rev);
        }
    }

    #[test]
    fn cells_match_slab_oracle_and_path_is_connected() {
        let mut rng = SplitMix64::new(0x9E377);
        for _ in 0..500 {
            let (x0, y0) = (rng.range(-15, 15), rng.range(-15, 15));
            let (x1, y1) = (rng.range(-15, 15), rng.range(-15, 15));
            let ray = grid_ray(x0, y0, x1, y1);
            // Oracle: same set of cells.
            let oracle = oracle_cells(x0, y0, x1, y1);
            let got: BTreeSet<(i32, i32)> = ray.iter().copied().collect();
            assert_eq!(got, oracle, "ray {x0},{y0} → {x1},{y1}");
            // No duplicates; endpoints correct.
            assert_eq!(got.len(), ray.len());
            assert_eq!(ray[0], (x0, y0));
            assert_eq!(*ray.last().unwrap_or(&(0, 0)), (x1, y1));
            // Consecutive cells share an edge or a corner.
            for w in ray.windows(2) {
                let dx = (w[1].0 - w[0].0).abs();
                let dy = (w[1].1 - w[0].1).abs();
                assert!(dx <= 1 && dy <= 1 && (dx + dy) > 0, "gap in ray");
            }
        }
    }

    #[test]
    fn blocked_and_los() {
        assert_eq!(
            ray_blocked_at(0, 0, 4, 0, |x, y| (x, y) == (2, 0)),
            Some((2, 0))
        );
        assert_eq!(ray_blocked_at(0, 0, 4, 0, |_, _| false), None);
        assert!(clear_los(0, 0, 4, 0, |_, _| false));
        assert!(!clear_los(0, 0, 4, 0, |x, _| x == 2));
        assert!(clear_los(0, 0, 4, 0, |x, _| x == 0 || x == 4));
    }

    #[test]
    fn ray_is_deterministic() {
        assert_eq!(grid_ray(-3, 7, 9, -4), grid_ray(-3, 7, 9, -4));
    }
}
