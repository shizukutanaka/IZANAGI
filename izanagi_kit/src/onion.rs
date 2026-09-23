//! Onion decomposition — nested convex layers of a point set: the
//! outer hull is layer 0, remove its vertices, and the hull of what
//! remains is layer 1, until nothing is left. `O(n² log n)` worst case
//! (one hull per layer), entirely `i32`/`i128` via [`crate::poly`].
//!
//! Layers come back outermost-first, each ring in the same canonical
//! vertex order [`crate::poly::convex_hull`] emits (CCW from the leftmost
//! lowest point). Duplicate input points are deduplicated first —
//! a repeated vertex would silently re-surface as its own layer.
//! A final runt of 1–2 points becomes a degenerate last layer rather
//! than vanishing.
//!
//! ```
//! use izanagi_kit::onion::onion_layers;
//! // square corners + center: two layers
//! let layers = onion_layers(&[(0, 0), (4, 0), (4, 4), (0, 4), (2, 2)]);
//! assert_eq!(layers.len(), 2);
//! assert_eq!(layers[1], vec![(2, 2)]);
//! ```
use crate::poly::convex_hull;

/// The nested convex layers of `pts`, outermost first.
/// Each layer is `convex_hull`'s canonical CCW output — collinear
/// boundary points are dropped, matching the hull's contract; fewer
/// than 3 remaining points form a trivial final layer.
///
/// Note: a point sitting *on* a hull edge is not a hull vertex under
/// [`convex_hull`]'s strict-corner output, so it survives the peel and
/// can surface as its own degenerate deeper layer — that's the honest
/// behavior for "onion depth" semantics, not a bug.
pub fn onion_layers(pts: &[(i32, i32)]) -> Vec<Vec<(i32, i32)>> {
    let mut rest: Vec<(i32, i32)> = pts.to_vec();
    rest.sort_unstable();
    rest.dedup();
    let mut layers = Vec::new();
    while !rest.is_empty() {
        if rest.len() <= 2 {
            layers.push(rest.clone());
            break;
        }
        let hull = convex_hull(&rest);
        let set: std::collections::BTreeSet<(i32, i32)> = hull.iter().copied().collect();
        rest.retain(|p| !set.contains(p));
        layers.push(hull);
    }
    layers
}

/// Depth of each layer — `onion_layers.len()`. `0` on empty input.
pub fn onion_depth(pts: &[(i32, i32)]) -> usize {
    onion_layers(pts).len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poly::{convex_hull, point_in_polygon, PointLocation};
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    /// Independent oracle: peel hulls with a *different* loop — keep
    /// recomputing the hull of the survivors, check each layer is
    /// exactly one hull, and the union of all layers is the input.
    fn oracle(pts: &[(i32, i32)]) -> Vec<Vec<(i32, i32)>> {
        let mut rest: BTreeSet<(i32, i32)> = pts.iter().copied().collect();
        let mut out = Vec::new();
        while !rest.is_empty() {
            let v: Vec<_> = rest.iter().copied().collect();
            if v.len() <= 2 {
                out.push(v);
                break;
            }
            let h = convex_hull(&v);
            for &p in &h {
                rest.remove(&p);
            }
            out.push(h);
        }
        out
    }

    #[test]
    fn nested_squares() {
        let mut pts = vec![
            (0, 0),
            (8, 0),
            (8, 8),
            (0, 8),
            (2, 2),
            (6, 2),
            (6, 6),
            (2, 6),
        ];
        pts.extend_from_slice(&[(4, 4)]);
        let layers = onion_layers(&pts);
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0].len(), 4);
        assert_eq!(layers[1].len(), 4);
        assert_eq!(layers[2], vec![(4, 4)]);
    }

    #[test]
    fn depth_tracks_layers() {
        let sq = |x0: i32, y0: i32, x1: i32, y1: i32| vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
        let mut pts = sq(0, 0, 8, 8);
        pts.extend(sq(2, 2, 6, 6));
        assert_eq!(onion_depth(&pts), onion_layers(&pts).len());
        assert_eq!(onion_depth(&[]), 0);
    }

    #[test]
    fn duplicates_and_collinear() {
        // duplicates collapse; collinear edge points are not hull
        // vertices under convex_hull's contract, so mid-edge (2,0)
        // survives the first peel and becomes a degenerate layer 1
        let layers = onion_layers(&[(0, 0), (0, 0), (4, 0), (4, 4), (0, 4), (2, 0)]);
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].len(), 4);
        assert_eq!(layers[1], vec![(2, 0)]);
        // all-collinear input: each peel keeps only the 2 extreme
        // endpoints, so 5 points descend in ceil(n/2)=3 layers
        let cl = onion_layers(&[(0, 0), (2, 2), (4, 4), (6, 6), (1, 1)]);
        assert_eq!(cl.len(), 3);
        assert!(cl.iter().all(|l| l.len() <= 2));
    }

    #[test]
    fn oracle_random() {
        let mut rng = SplitMix64::new(0x0a10_0001_5eed);
        for _ in 0..300 {
            let n = rng.below(30) as usize;
            let pts: Vec<(i32, i32)> = (0..n)
                .map(|_| (rng.below(20) as i32, rng.below(20) as i32))
                .collect();
            let got = onion_layers(&pts);
            let want = oracle(&pts);
            // same layer count, same per-layer vertex sets (order may
            // legitimately differ — compare as sets)
            assert_eq!(got.len(), want.len());
            for (g, w) in got.iter().zip(&want) {
                let gs: BTreeSet<_> = g.iter().copied().collect();
                let ws: BTreeSet<_> = w.iter().copied().collect();
                assert_eq!(gs, ws);
            }
            // union of layers = deduped input
            let all: BTreeSet<_> = got.iter().flatten().copied().collect();
            let inp: BTreeSet<_> = pts.iter().copied().collect();
            assert_eq!(all, inp);
            // strictly nested: each deeper hull inside-or-on the previous
            for w in 1..got.len() {
                let outer = &got[w - 1];
                if outer.len() >= 3 {
                    for &p in &got[w] {
                        assert_ne!(
                            point_in_polygon(p, outer),
                            PointLocation::Outside,
                            "layer {w} point {p:?} escapes"
                        );
                    }
                }
            }
        }
    }
}
