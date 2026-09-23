//! Union area of axis-aligned rectangles via an x-sweep: collect
//! `(x, [y0,y1], ±1)` edge events, sort by `x`, and between consecutive
//! event abscissae multiply the covered-y length by the width. All
//! `i64`/`i128` — the classic sweep-line problem, kept `O(n²)` (per
//! slab we recompute the union of active y-intervals) because the
//! simplicity is worth more than a coordinate-compressed segment tree
//! here, and it stays exact for `n` up to ~10⁴.
//!
//! Returns `i128` — overlapping `u64`-sized boards tile far past
//! `u64::MAX` total area.
//!
//! ```
//! use izanagi_kit::rectunion::union_area;
//! // two 4x4 squares overlapping in a 2x2 corner
//! let a = union_area(&[(0, 0, 4, 4), (2, 2, 6, 6)]);
//! assert_eq!(a, 16 + 16 - 4);
//! ```

/// Rectangles are `(x0, y0, x1, y1)` in any order — endpoints are
/// sorted internally, so inverted input still measures the same cell.
/// Degenerate (zero-area) rectangles contribute nothing.
pub fn union_area(rects: &[(i64, i64, i64, i64)]) -> i128 {
    // (x, y0, y1, delta)
    let mut ev: Vec<(i64, i64, i64, i64)> = Vec::with_capacity(rects.len() * 2);
    for &(x0, y0, x1, y1) in rects {
        let (xa, xb) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        let (ya, yb) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
        if xa == xb || ya == yb {
            continue;
        }
        ev.push((xa, ya, yb, 1));
        ev.push((xb, ya, yb, -1));
    }
    ev.sort_unstable();
    let mut active: Vec<(i64, i64)> = Vec::with_capacity(rects.len());
    let mut area = 0i128;
    let mut i = 0;
    let mut prev_x = 0i64;
    while i < ev.len() {
        let x = ev[i].0;
        if i > 0 && x > prev_x {
            // union of active y-intervals over the slab [prev_x, x)
            let mut seg = active.clone();
            seg.sort_unstable();
            let mut cover = 0i128;
            let (mut cy0, mut cy1) = (0i64, 0i64);
            let mut open = false;
            for &(a, b) in &seg {
                if !open {
                    cy0 = a;
                    cy1 = b;
                    open = true;
                } else if a <= cy1 {
                    cy1 = cy1.max(b);
                } else {
                    cover += (cy1 - cy0) as i128;
                    cy0 = a;
                    cy1 = b;
                }
            }
            if open {
                cover += (cy1 - cy0) as i128;
            }
            area += cover * (x - prev_x) as i128;
        }
        // apply every event at this x (order irrelevant — deltas commute)
        while i < ev.len() && ev[i].0 == x {
            let (_, ya, yb, d) = ev[i];
            if d == 1 {
                active.push((ya, yb));
            } else {
                // remove one matching interval — multiset semantics
                if let Some(pos) = active.iter().position(|&v| v == (ya, yb)) {
                    active.swap_remove(pos);
                }
            }
            i += 1;
        }
        prev_x = x;
    }
    area
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Independent oracle: mark each unit cell's coverage on a fine
    /// grid — exact whenever every edge lands on integer coordinates
    /// (always, by construction here).
    fn grid_area(rects: &[(i64, i64, i64, i64)]) -> i128 {
        let mut cells = std::collections::BTreeSet::new();
        for &(x0, y0, x1, y1) in rects {
            let (xa, xb) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
            let (ya, yb) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
            for x in xa..xb {
                for y in ya..yb {
                    cells.insert((x, y));
                }
            }
        }
        cells.len() as i128
    }

    #[test]
    fn disjoint_and_nested() {
        assert_eq!(union_area(&[(0, 0, 2, 2), (4, 4, 6, 6)]), 8);
        assert_eq!(union_area(&[(0, 0, 10, 10), (2, 2, 4, 4)]), 100);
        assert_eq!(union_area(&[]), 0);
        // degenerate slivers count for nothing
        assert_eq!(union_area(&[(0, 0, 0, 5), (1, 1, 3, 1)]), 0);
        // inverted endpoints normalize
        assert_eq!(union_area(&[(4, 4, 0, 0)]), 16);
    }

    #[test]
    fn grid_oracle() {
        let mut rng = SplitMix64::new(0x2ec7_0a1e_0001);
        for _ in 0..400 {
            let n = rng.below(8) as usize + 1;
            let mut rs = Vec::with_capacity(n);
            for _ in 0..n {
                // small positive coords → the cell oracle stays cheap
                let x0 = rng.below(14) as i64;
                let y0 = rng.below(14) as i64;
                let x1 = x0 + rng.below(8) as i64;
                let y1 = y0 + rng.below(8) as i64;
                rs.push((x0, y0, x1, y1));
            }
            assert_eq!(union_area(&rs), grid_area(&rs), "{rs:?}");
        }
    }

    #[test]
    fn negative_and_huge_coords() {
        let mut rng = SplitMix64::new(0x2ec7_0a1e_0002);
        for _ in 0..200 {
            let n = rng.below(6) as usize + 1;
            let mut rs = Vec::with_capacity(n);
            for _ in 0..n {
                let x0 = rng.below(20) as i64 - 10;
                let y0 = rng.below(20) as i64 - 10;
                let x1 = x0 + rng.below(9) as i64;
                let y1 = y0 + rng.below(9) as i64;
                rs.push((x0, y0, x1, y1));
            }
            assert_eq!(union_area(&rs), grid_area(&rs), "{rs:?}");
        }
        // identical rectangles stack once, not n times
        assert_eq!(union_area(&[(0, 0, 5, 5); 4]), 25);
        // i64-scale extents stay exact in i128
        let big = union_area(&[
            (0, 0, 1 << 40, 1 << 40),
            (1 << 40, 0, (1 << 40) + 1, 1 << 40),
        ]);
        assert_eq!(big, (1i128 << 40) * (1i128 << 40) + (1i128 << 40));
    }
}
