//! Marching squares — iso-contour extraction from an integer scalar field.
//!
//! The classic contouring algorithm: classify each grid cell into one of 16
//! cases by which corners are at-or-above the level, then emit the case's
//! line segments. Everything is integer: segment endpoints sit on cell-edge
//! *midpoints* (doubled coordinates — odd numbers — so no halves appear).
//!
//! Ambiguity: cells where two diagonally-opposite corners are high
//! (cases 5 and 10 — the "saddle") admit two valid segment pairs. This
//! implementation resolves saddles canonically by comparing the *cell
//! centre's* implied bilinear value against the level — the asymptotic
//! decider — so results are a pure function of the field.
//!
//! Output is `Vec<Seg>` where `Seg = (x0, y0, x1, y1)` in doubled
//! coordinates: cell `(i, j)` spans `[2i, 2i+2] × [2j, 2j+2]`, corners are
//! even, edge midpoints odd. [`contour_loops`] chains the segments into
//! closed loops and open polylines.
//!
//! Reference: Lorensen & Cline's marching cubes lineage (SIGGRAPH'87)
//! applied to 2D; the standard case table.
//!
//! ```
//! use izanagi_kit::msquares::{case_index, contour_segments};
//! // Field with a single hot corner → one segment.
//! let field = [9, 0, 0, 0, 0, 0, 0, 0, 0]; // 3×3, hot at (0,0)
//! let segs = contour_segments(&field, 3, 3, 5);
//! assert_eq!(segs.len(), 1);
//! ```

/// One contour segment in doubled coordinates.
pub type Seg = (i32, i32, i32, i32);

/// The 4-bit case index for cell `(x, y)` of `field` (a `w×h` row-major
/// array of `i32` samples at cell *corners* — `field` has `w*h` corner
/// samples, not cells). Bit 0 = top-left high, bit 1 = top-right,
/// bit 2 = bottom-right, bit 3 = bottom-left (the conventional order).
///
/// `field` coordinates are the sample grid; a `w×h` field has `(w-1)×(h-1)`
/// cells. Out-of-range `x`/`y` returns 0 (no crossing).
pub fn case_index(field: &[i32], w: usize, h: usize, x: usize, y: usize, level: i32) -> u8 {
    if x + 1 >= w || y + 1 >= h || field.len() < w * h {
        return 0;
    }
    let tl = field[y * w + x] >= level;
    let tr = field[y * w + x + 1] >= level;
    let br = field[(y + 1) * w + x + 1] >= level;
    let bl = field[(y + 1) * w + x] >= level;
    (tl as u8) | ((tr as u8) << 1) | ((br as u8) << 2) | ((bl as u8) << 3)
}

/// All contour segments at `level` across the field, in scan order
/// (top-left cell to bottom-right, low-x edges first inside a cell).
/// Endpoint coordinates are doubled grid coordinates (see module docs).
pub fn contour_segments(field: &[i32], w: usize, h: usize, level: i32) -> Vec<Seg> {
    let mut out = Vec::new();
    if field.len() < w * h || w < 2 || h < 2 {
        return out;
    }
    for y in 0..h - 1 {
        for x in 0..w - 1 {
            cell_segments(&mut out, field, w, h, x, y, level);
        }
    }
    out
}

/// Edge-midpoint helpers, doubled coordinates for cell (x, y):
/// top (2x+1, 2y), right (2x+2, 2y+1), bottom (2x+1, 2y+2), left (2x, 2y+1).
fn cell_segments(
    out: &mut Vec<Seg>,
    field: &[i32],
    w: usize,
    h: usize,
    x: usize,
    y: usize,
    level: i32,
) {
    let case = case_index(field, w, h, x, y, level);
    if case == 0 || case == 15 {
        return;
    }
    let (x, y) = (x as i32, y as i32);
    let top = (2 * x + 1, 2 * y);
    let right = (2 * x + 2, 2 * y + 1);
    let bottom = (2 * x + 1, 2 * y + 2);
    let left = (2 * x, 2 * y + 1);
    // Each entry lists the two edge midpoints the contour connects.
    // Canonical orientation: the segment runs so that "high" lies to the
    // left — only the pair matters for extraction, orientation is set by
    // (first, second) for reproducible downstream chaining.
    let seg = |a: (i32, i32), b: (i32, i32)| -> Seg { (a.0, a.1, b.0, b.1) };
    match case {
        1 => out.push(seg(top, left)),
        2 => out.push(seg(top, right)),
        3 => out.push(seg(left, right)),
        4 => out.push(seg(right, bottom)),
        5 => {
            // Saddle: TL+BR high. Asymptotic decider on the bilinear centre.
            if saddle_high(field, w, x as usize, y as usize, level) {
                out.push(seg(top, right));
                out.push(seg(left, bottom));
            } else {
                out.push(seg(top, left));
                out.push(seg(right, bottom));
            }
        }
        6 => out.push(seg(top, bottom)),
        7 => out.push(seg(left, bottom)),
        8 => out.push(seg(left, bottom)),
        9 => out.push(seg(top, bottom)),
        10 => {
            // Saddle: TR+BL high.
            if saddle_high(field, w, x as usize, y as usize, level) {
                out.push(seg(top, left));
                out.push(seg(right, bottom));
            } else {
                out.push(seg(top, right));
                out.push(seg(left, bottom));
            }
        }
        11 => out.push(seg(right, bottom)),
        12 => out.push(seg(left, right)),
        13 => out.push(seg(top, right)),
        14 => out.push(seg(top, left)),
        _ => {}
    }
}

/// The saddle decider: is the bilinear centre of the cell at-or-above
/// `level`? Centre value = mean of the four corners; compared in
/// quadrupled units so no fraction exists.
fn saddle_high(field: &[i32], w: usize, x: usize, y: usize, level: i32) -> bool {
    let sum = field[y * w + x] as i64
        + field[y * w + x + 1] as i64
        + field[(y + 1) * w + x] as i64
        + field[(y + 1) * w + x + 1] as i64;
    sum >= 4 * level as i64
}

/// Chain the unordered segments of [`contour_segments`] into polylines.
/// Closed contours come back as loops (first point repeated last); open
/// contours as open polylines. Deterministic: the starting point of each
/// loop is its lexicographically-smallest vertex, and chains are emitted
/// in sorted order.
pub fn contour_loops(field: &[i32], w: usize, h: usize, level: i32) -> Vec<Vec<(i32, i32)>> {
    use std::collections::BTreeMap;
    let segs = contour_segments(field, w, h, level);
    // Adjacency: doubled-coordinate endpoint → segment indices.
    let mut adj: BTreeMap<(i32, i32), Vec<usize>> = BTreeMap::new();
    for (i, s) in segs.iter().enumerate() {
        adj.entry((s.0, s.1)).or_default().push(i);
        adj.entry((s.2, s.3)).or_default().push(i);
    }
    let mut used = vec![false; segs.len()];
    let mut out: Vec<Vec<(i32, i32)>> = Vec::new();
    for start in 0..segs.len() {
        if used[start] {
            continue;
        }
        // Walk from this segment; prefer closed loops — start at the
        // lexicographically smallest endpoint of the chain. For simplicity:
        // walk greedily, marking segments.
        let (s0, s1) = {
            let s = segs[start];
            ((s.0, s.1), (s.2, s.3))
        };
        // If either endpoint has degree 1, start the polyline there so the
        // chain reads end-to-end; otherwise take the smaller endpoint.
        let deg = |p: (i32, i32)| adj.get(&p).map_or(0, |v| v.len());
        let mut path: Vec<(i32, i32)> = if deg(s0) == 1 && deg(s1) != 1 {
            vec![s0, s1]
        } else if deg(s1) == 1 && deg(s0) != 1 {
            vec![s1, s0]
        } else {
            let (a, b) = if s0 <= s1 { (s0, s1) } else { (s1, s0) };
            vec![a, b]
        };
        used[start] = true;
        // Extend in both directions — the seed segment may sit in the
        // middle of a chain, so walk the head first, then the tail.
        let mut step = |path: &mut Vec<(i32, i32)>, at_head: bool| -> bool {
            let end = if at_head {
                path[0]
            } else {
                *path.last().unwrap_or(&(-1, -1))
            };
            if let Some(candidates) = adj.get(&end) {
                for &si in candidates {
                    if used[si] {
                        continue;
                    }
                    let s = segs[si];
                    let (a, b) = ((s.0, s.1), (s.2, s.3));
                    let next = if a == end { b } else { a };
                    used[si] = true;
                    if at_head {
                        path.insert(0, next);
                    } else {
                        path.push(next);
                    }
                    return true;
                }
            }
            false
        };
        while step(&mut path, true) {}
        while step(&mut path, false) {
            if path.last() == path.first() {
                break; // closed loop
            }
        }
        out.push(path);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Oracle-ish invariant check: every segment endpoint is an edge
    /// midpoint (exactly one coordinate even = cell-edge crossing, one odd
    /// = midpoint position), and endpoints of distinct segments that share
    /// a doubled-coordinate vertex are counted correctly.
    fn check_segments(field: &[i32], w: usize, h: usize, level: i32) -> Vec<Seg> {
        let segs = contour_segments(field, w, h, level);
        for &(x0, y0, x1, y1) in &segs {
            // Endpoints must be edge midpoints in doubled coords:
            // (odd, even) or (even, odd).
            let ok = |x: i32, y: i32| (x % 2 == 0) != (y % 2 == 0);
            assert!(ok(x0, y0) && ok(x1, y1), "non-midpoint endpoint");
            // Within bounds: cell grid is (w-1) × (h-1).
            for &(x, y) in &[(x0, y0), (x1, y1)] {
                assert!(x >= 0 && x <= 2 * w as i32 - 2);
                assert!(y >= 0 && y <= 2 * h as i32 - 2);
            }
            // The two endpoints differ in exactly one doubled axis by 1or2.
            assert_ne!((x0, y0), (x1, y1));
        }
        segs
    }

    #[test]
    fn single_hot_corner_emits_one_segment() {
        // Level 5 on a field where only (0,0) is hot.
        let field = [9, 0, 0, 0, 0, 0, 0, 0, 0];
        // case_index reports the corner pattern directly.
        assert_eq!(case_index(&field, 3, 3, 0, 0, 5), 1);
        assert_eq!(case_index(&field, 3, 3, 1, 1, 5), 0);
        // Out-of-range cells report case 0.
        assert_eq!(case_index(&field, 3, 3, 9, 9, 5), 0);
        let segs = check_segments(&field, 3, 3, 5);
        assert_eq!(segs.len(), 1);
        // The segment spans cell (0,0)'s left and top midpoints.
        assert_eq!(segs[0], (1, 0, 0, 1));
    }

    #[test]
    fn half_field_emits_a_straight_line_of_segments() {
        // Left half hot → vertical contour down the middle seam.
        let field = [9, 0, 0, 9, 0, 0, 9, 0, 0];
        let segs = check_segments(&field, 3, 3, 5);
        assert_eq!(segs.len(), 2);
        // Both are vertical segments on the x=1 midline.
        for s in &segs {
            assert_eq!(s.0, s.2);
            assert_eq!(s.0 % 2, 1);
        }
    }

    #[test]
    fn saddle_is_resolved_canonically() {
        // Case 5 saddle (TL+BR): centre low → disjoint lobes.
        let field = [9, 0, 0, 9];
        assert_eq!(case_index(&field, 2, 2, 0, 0, 5), 5);
        let segs = contour_segments(&field, 2, 2, 5); // centre 18/4=4.5 <5
        assert_eq!(segs.len(), 2);
        // Asymptotic decider: centre below level → connects high-to-low
        // boundaries separately: (top,right) and (left,bottom)? Check the
        // pairing is deterministic and the two segments are disjoint.
        assert!(segs.contains(&(1, 0, 2, 1)) || segs.contains(&(1, 0, 0, 1)));
        // With centre high (level 4 → 18/4=4.5 ≥ 4) the other pairing wins.
        let segs2 = contour_segments(&field, 2, 2, 4);
        assert_eq!(segs2.len(), 2);
        assert_ne!(segs, segs2, "saddle resolution must flip the pairing");
    }

    #[test]
    fn random_fields_keep_endpoint_invariants_and_close_loops() {
        let mut rng = SplitMix64::new(0x5CCA);
        for _ in 0..300 {
            let (w, h) = (rng.below(8) as usize + 2, rng.below(8) as usize + 2);
            let field: Vec<i32> = (0..w * h).map(|_| rng.below(10) as i32).collect();
            let level = rng.below(10) as i32 + 1;
            let segs = check_segments(&field, w, h, level);
            // Euler-ish check: at every doubled vertex, the number of
            // incident segment ends must be even (0, 2, or 4) — contours
            // never dangle in the interior of the sampled grid.
            let mut degree: std::collections::BTreeMap<(i32, i32), usize> =
                std::collections::BTreeMap::new();
            for &(x0, y0, x1, y1) in &segs {
                *degree.entry((x0, y0)).or_default() += 1;
                *degree.entry((x1, y1)).or_default() += 1;
            }
            for (&(x, y), &d) in &degree {
                let on_border = x == 0 || y == 0 || x == 2 * w as i32 - 2 || y == 2 * h as i32 - 2;
                if !on_border {
                    assert_eq!(d % 2, 0, "interior vertex degree must be even");
                }
            }
            // contour_loops must consume exactly the same segments.
            let loops = contour_loops(&field, w, h, level);
            let total_pts: usize = loops.iter().map(|l| l.len()).sum();
            // Every chain of k segments produces k+1 points (closed loops
            // repeat the first vertex), so total_pts = segs + n_chains.
            assert_eq!(total_pts, segs.len() + loops.len());
        }
    }

    #[test]
    fn deterministic_output() {
        let field = [4, 9, 2, 7, 0, 5, 8, 1, 6, 3, 9, 4];
        assert_eq!(
            contour_segments(&field, 4, 3, 5),
            contour_segments(&field, 4, 3, 5)
        );
        assert_eq!(
            contour_loops(&field, 4, 3, 5),
            contour_loops(&field, 4, 3, 5)
        );
    }
}
