//! Rectangle bin packing — deterministic skyline (bottom-left) packer.
//!
//! Algorithm: the **skyline / level-pack** heuristic surveyed in Jylänki,
//! *"A Thousand Ways to Pack the Bin"* (2010): keep a monotone skyline of
//! occupied height across the bin's width; each rectangle drops to the
//! lowest skyline segment that fits its width, then that segment's height
//! is raised. Fully integer, fully deterministic — placement order is
//! input order, so `pack_skyline` is a pure function of the input slice.
//!
//! Use it for texture-atlas layout, inventory grids, dialog-box tiling —
//! anywhere "fit these boxes in a bin" must replay identically.
//!
//! ```
//! use izanagi_kit::pack::pack_skyline;
//! let rects = [(4, 4u32), (2, 3), (2, 3)];
//! let placed = pack_skyline(&rects, 6, 6);
//! // All three fit: no overlap, all inside the 6×6 bin.
//! assert!(placed.iter().all(|p| p.is_some()));
//! ```

/// One skyline segment: `x..x+w` currently at height `y`.
#[derive(Clone, Copy, Debug)]
struct Node {
    x: u32,
    y: u32,
    w: u32,
}

/// Pack `rects` (as `(w, h)` pairs) into a `bin_w × bin_h` bin with the
/// skyline bottom-left heuristic. Returns a placement `(x, y)` per input
/// rectangle in input order, or `None` for a rectangle that doesn't fit.
///
/// Runs in `O(n · nodes)`.
pub fn pack_skyline(rects: &[(u32, u32)], bin_w: u32, bin_h: u32) -> Vec<Option<(u32, u32)>> {
    // Skyline nodes, sorted by x, covering [0, bin_w). The last node always
    // has y = lowest occupied height; heights only shrink leftward? No —
    // standard skyline: nodes record the current ceiling per x-interval.
    let mut nodes: Vec<Node> = vec![Node {
        x: 0,
        y: 0,
        w: bin_w,
    }];
    rects
        .iter()
        .map(|&(w, h)| place(&mut nodes, w, h, bin_w, bin_h))
        .collect()
}

/// Try to place `w×h`; on success mutates the skyline and returns `Some`.
fn place(nodes: &mut Vec<Node>, w: u32, h: u32, bin_w: u32, bin_h: u32) -> Option<(u32, u32)> {
    if w == 0 || h == 0 || w > bin_w || h > bin_h {
        return None;
    }
    // Best position = lowest y where a contiguous run of nodes spanning
    // width w fits under h. Bottom-left tie: smallest x among equal y.
    let mut best: Option<(u32, u32, usize)> = None; // (y, x, node index)
    for i in 0..nodes.len() {
        let (nx, mut y) = (nodes[i].x, nodes[i].y);
        if u64::from(y) + u64::from(h) > u64::from(bin_h) {
            continue;
        }
        // Extend right until width w is covered.
        let mut span = 0u32;
        let mut j = i;
        let mut fits = true;
        while span < w {
            if j >= nodes.len() {
                fits = false;
                break;
            }
            span = span.saturating_add(nodes[j].w);
            if nodes[j].y > y {
                y = nodes[j].y;
                if u64::from(y) + u64::from(h) > u64::from(bin_h) {
                    fits = false;
                }
                // Either way the original x would clip this taller
                // segment — retry when the outer loop reaches it.
                break;
            }
            j += 1;
        }
        if !fits || span < w {
            continue;
        }
        if u64::from(y) + u64::from(h) > u64::from(bin_h) {
            continue;
        }
        match best {
            Some((by, bx, _)) if by < y || (by == y && bx <= nx) => {}
            _ => best = Some((y, nx, i)),
        }
    }
    let (y, x, i) = best?;
    // Insert the placement: split the node that contains x if needed, then
    // raise all nodes overlapped by [x, x+w) to y+h and merge equal-height
    // neighbours.
    // Find node containing x.
    let mut k = i;
    while nodes[k].x + nodes[k].w <= x {
        k += 1;
    }
    // Split the containing node if x is interior to it.
    if nodes[k].x < x {
        let left = Node {
            x: nodes[k].x,
            y: nodes[k].y,
            w: x - nodes[k].x,
        };
        nodes[k].w -= left.w;
        nodes[k].x = x;
        nodes.insert(k, left);
        k += 1;
    }
    // Raise nodes covered by [x, x+w) to y + h, splitting the last if the
    // rect ends interior to it.
    let end = x + w;
    while k < nodes.len() && nodes[k].x < end {
        if nodes[k].x + nodes[k].w > end {
            let right = Node {
                x: end,
                y: nodes[k].y,
                w: nodes[k].x + nodes[k].w - end,
            };
            nodes[k].w = end - nodes[k].x;
            nodes.insert(k + 1, right);
        }
        nodes[k].y = y + h;
        k += 1;
    }
    // Merge adjacent nodes at equal height.
    let mut m = 0;
    while m + 1 < nodes.len() {
        if nodes[m].y == nodes[m + 1].y && nodes[m].x + nodes[m].w == nodes[m + 1].x {
            nodes[m].w += nodes[m + 1].w;
            nodes.remove(m + 1);
        } else {
            m += 1;
        }
    }
    Some((x, y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Invariant oracle: placements never overlap each other and never
    /// leave the bin.
    fn check(rects: &[(u32, u32)], placed: &[Option<(u32, u32)>], bin_w: u32, bin_h: u32) {
        let boxes: Vec<(u32, u32, u32, u32)> = rects
            .iter()
            .zip(placed)
            .filter_map(|(&(w, h), p)| p.map(|(x, y)| (x, y, x + w, y + h)))
            .collect();
        for &(x0, y0, x1, y1) in &boxes {
            assert!(x1 <= bin_w && y1 <= bin_h && x0 < x1 && y0 < y1);
        }
        for i in 0..boxes.len() {
            for j in i + 1..boxes.len() {
                let (a, b) = (boxes[i], boxes[j]);
                let overlap = a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3;
                assert!(!overlap, "overlap between {a:?} and {b:?}");
            }
        }
    }

    #[test]
    fn known_layout() {
        // Two 3×2 side by side, a 2×2 on top of the left one.
        let placed = pack_skyline(&[(3, 2), (3, 2), (2, 2)], 6, 6);
        assert_eq!(placed[0], Some((0, 0)));
        assert_eq!(placed[1], Some((3, 0)));
        assert_eq!(placed[2], Some((0, 2)));
        check(&[(3, 2), (3, 2), (2, 2)], &placed, 6, 6);
    }

    #[test]
    fn unplaceable_returns_none() {
        let placed = pack_skyline(&[(7, 1), (1, 7), (0, 5)], 6, 6);
        assert_eq!(placed, vec![None, None, None]);
    }

    #[test]
    fn fills_bin_exactly() {
        let placed = pack_skyline(&[(2, 2); 9], 6, 6);
        assert!(placed.iter().all(|p| p.is_some()));
        check(&[(2, 2); 9], &placed, 6, 6);
    }

    #[test]
    fn random_rects_keep_invariants() {
        let mut rng = SplitMix64::new(0x5A51);
        for _ in 0..200 {
            let (bw, bh) = (rng.below(12) + 4, rng.below(12) + 4);
            let n = rng.below(14) as usize + 1;
            let rects: Vec<(u32, u32)> = (0..n)
                .map(|_| (rng.below(6) + 1, rng.below(6) + 1))
                .collect();
            let placed = pack_skyline(&rects, bw, bh);
            assert_eq!(placed.len(), n);
            check(&rects, &placed, bw, bh);
        }
    }

    #[test]
    fn deterministic() {
        let rects = [(4, 2), (1, 3), (2, 2), (3, 3), (5, 1)];
        assert_eq!(pack_skyline(&rects, 8, 8), pack_skyline(&rects, 8, 8));
    }
}
