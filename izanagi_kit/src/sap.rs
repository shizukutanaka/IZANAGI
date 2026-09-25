//! Sweep-and-prune broadphase — every pair of axis-aligned boxes that
//! overlap, in `O(n log n + k)` instead of `O(n²)` (sort on min-x, keep
//! an active list pruned by `max.x ≤ min.x`, test full overlap only on
//! survivors). The Box2D/bullet-style first pass before narrowphase.
//!
//! Output pairs are `(i, j)` input indices with `i < j`, sorted —
//! replay-stable regardless of sweep internals.
//!
//! ```
//! use izanagi_kit::sap::pairs;
//!
//! // (x, y, w, h) boxes; 0 and 1 overlap, 2 is clear of both.
//! let boxes = [(0, 0, 4, 4), (2, 2, 4, 4), (10, 10, 2, 2)];
//! assert_eq!(pairs(&boxes), vec![(0, 1)]);
//! ```

/// Positive-area overlap: the shared region must be non-empty on both
/// axes — touching edges and zero-area boxes count as nothing.
fn inter(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
    a.0.max(b.0) < a.0.saturating_add(a.2).min(b.0.saturating_add(b.2))
        && a.1.max(b.1) < a.1.saturating_add(a.3).min(b.1.saturating_add(b.3))
}

/// All overlapping pairs among `boxes` given as `(x, y, w, h)` with
/// `w, h ≥ 0`. Indices pair as `(i, j)`, `i < j`, in sorted order.
pub fn pairs(boxes: &[(i32, i32, i32, i32)]) -> Vec<(u32, u32)> {
    // Sort indices by (min_x, index) — the sweep key is total, so ties
    // can't reorder the output.
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_by_key(|&i| (boxes[i].0, i));
    let mut out = Vec::new();
    // Active list: indices whose max.x > the current box's min.x
    // (strict — touching on x can't overlap), kept in sorted order.
    let mut active: Vec<usize> = Vec::with_capacity(order.len());
    for &i in &order {
        active.retain(|&j| boxes[j].0.saturating_add(boxes[j].2) > boxes[i].0);
        for &j in &active {
            if inter(boxes[j], boxes[i]) {
                out.push((i.min(j) as u32, i.max(j) as u32));
            }
        }
        active.push(i);
    }
    out.sort_unstable();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Brute O(n²) oracle with identical overlap semantics.
    fn oracle(boxes: &[(i32, i32, i32, i32)]) -> Vec<(u32, u32)> {
        let mut out = Vec::new();
        for i in 0..boxes.len() {
            for j in i + 1..boxes.len() {
                if inter(boxes[i], boxes[j]) {
                    out.push((i as u32, j as u32));
                }
            }
        }
        out
    }

    #[test]
    fn matches_brute_oracle_on_random_layouts() {
        let mut g = SplitMix64::new(0x5A9);
        for _case in 0..60 {
            let n = g.range(0, 40) as usize;
            let boxes: Vec<(i32, i32, i32, i32)> = (0..n)
                .map(|_| {
                    (
                        g.range(-50, 50),
                        g.range(-50, 50),
                        g.range(0, 30),
                        g.range(0, 30),
                    )
                })
                .collect();
            assert_eq!(pairs(&boxes), oracle(&boxes), "boxes {boxes:?}");
        }
    }

    #[test]
    fn touching_edges_do_not_overlap() {
        // Share only the edge x=4: no area, no pair.
        assert_eq!(pairs(&[(0, 0, 4, 4), (4, 0, 4, 4)]), vec![]);
        // Share a 1-unit strip: overlaps.
        assert_eq!(pairs(&[(0, 0, 5, 4), (4, 0, 4, 4)]), vec![(0, 1)]);
        // Containment counts.
        assert_eq!(pairs(&[(0, 0, 10, 10), (3, 3, 2, 2)]), vec![(0, 1)]);
        // Zero-area boxes overlap nothing, even strictly inside.
        assert_eq!(pairs(&[(0, 0, 0, 4), (0, 0, 4, 4)]), vec![]);
        assert_eq!(pairs(&[(4, 0, 0, 4), (0, 0, 8, 8)]), vec![]);
    }

    #[test]
    fn sorted_deterministic_output() {
        let boxes = [(5, 0, 4, 4), (0, 0, 8, 8), (7, 1, 2, 2), (20, 20, 1, 1)];
        let got = pairs(&boxes);
        assert_eq!(got, oracle(&boxes));
        for w in got.windows(2) {
            assert!(w[0] < w[1]);
        }
        assert!(got.iter().all(|(i, j)| i < j));
        assert_eq!(pairs(&[]), vec![]);
    }
}
