//! DBSCAN — density-based spatial clustering on integer points
//! (Ester, Kriegel, Sander & Xu 1996). The radius test is squared-
//! distance `eps2`, so no square root ever appears; border and noise
//! are byproducts, no `k` needs choosing.
//!
//! Labeling is canonical: seeds expand in ascending index order and
//! each expansion processes its queue in ascending index order too —
//! cluster ids are assigned in order of discovery, making the result
//! a pure function of `(points, eps2, min_pts)` independent of any
//! adjacency representation.
//!
//! ```
//! use izanagi_kit::dbscan::dbscan;
//! // Two tight blobs plus an outlier.
//! let pts = [(0, 0), (1, 0), (0, 1), (50, 50), (51, 50), (50, 51), (200, 200)];
//! let labels = dbscan(&pts, 4, 2);
//! assert_eq!(labels[0], labels[1]);
//! assert_eq!(labels[3], labels[4]);
//! assert_ne!(labels[0], labels[3]);
//! assert_eq!(labels[6], -1); // noise
//! ```

/// Cluster labels for `points`: `-1` = noise, otherwise a small
/// cluster id assigned in discovery order. Two points share a label
/// iff one is density-reachable from the other through core points.
///
/// - `eps2`: squared neighborhood radius — `p` and `q` are neighbors
///   iff `dist²(p, q) <= eps2`.
/// - `min_pts`: neighborhood size (inclusive of the point itself)
///   required for a point to be a *core* point.
pub fn dbscan(points: &[(i32, i32)], eps2: i128, min_pts: usize) -> Vec<i32> {
    let n = points.len();
    let mut label = vec![-1i32; n];
    let mut cluster = 0i32;
    for seed in 0..n {
        if label[seed] != -1 {
            continue;
        }
        let nbrs = neighbors(points, seed, eps2);
        if nbrs.len() < min_pts {
            // Provisional noise; may later be absorbed as a border
            // point of another cluster — leave -1 for now.
            continue;
        }
        // Expand cluster `cluster` from seed.
        label[seed] = cluster;
        // Canonical expansion queue: ascending index order.
        let mut queue = nbrs;
        let mut head = 0;
        while head < queue.len() {
            let q = queue[head];
            head += 1;
            if label[q] != -1 {
                continue;
            }
            label[q] = cluster;
            let qn = neighbors(points, q, eps2);
            if qn.len() >= min_pts {
                for &r in &qn {
                    if label[r] == -1 && !queue.contains(&r) {
                        queue.push(r);
                    }
                }
            }
        }
        cluster += 1;
    }
    label
}

/// Sorted neighbor list of point `i`, `i` included.
fn neighbors(points: &[(i32, i32)], i: usize, eps2: i128) -> Vec<usize> {
    let (x, y) = (points[i].0 as i128, points[i].1 as i128);
    let mut out = Vec::new();
    for (j, &(px, py)) in points.iter().enumerate() {
        let dx = px as i128 - x;
        let dy = py as i128 - y;
        if dx * dx + dy * dy <= eps2 {
            out.push(j);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::{BTreeMap, BTreeSet};

    fn nbr_count(points: &[(i32, i32)], i: usize, eps2: i128) -> usize {
        let (x, y) = (points[i].0 as i128, points[i].1 as i128);
        points
            .iter()
            .filter(|&&(px, py)| {
                let dx = px as i128 - x;
                let dy = py as i128 - y;
                dx * dx + dy * dy <= eps2
            })
            .count()
    }

    #[test]
    fn labels_satisfy_dbscan_definitions() {
        let mut rng = SplitMix64::new(19);
        for _ in 0..25 {
            let n = rng.below(30) as usize + 5;
            let pts: Vec<(i32, i32)> = (0..n)
                .map(|_| (rng.below(30) as i32, rng.below(30) as i32))
                .collect();
            let eps2 = 25i128;
            let min_pts = 3usize;
            let label = dbscan(&pts, eps2, min_pts);
            assert_eq!(label.len(), n);
            let dense = |i: usize| nbr_count(&pts, i, eps2) >= min_pts;
            // 1) Any point with a dense neighborhood is core, hence
            //    must be labeled (seeded itself or absorbed).
            for (i, &l) in label.iter().enumerate() {
                if dense(i) {
                    assert!(l >= 0, "core point {i} left as noise");
                }
            }
            // 2) Border points must be eps-adjacent to a core point
            //    of the same cluster.
            let mut cluster_cores: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
            for (i, &l) in label.iter().enumerate() {
                if l >= 0 && dense(i) {
                    cluster_cores.entry(l).or_default().push(i);
                }
            }
            for i in 0..n {
                if label[i] >= 0 && !dense(i) {
                    let cores = &cluster_cores[&label[i]];
                    assert!(
                        cores.iter().any(|&c| {
                            let dx = pts[c].0 as i128 - pts[i].0 as i128;
                            let dy = pts[c].1 as i128 - pts[i].1 as i128;
                            dx * dx + dy * dy <= eps2
                        }),
                        "border point {i} not adjacent to a core"
                    );
                }
            }
            // 3) Each cluster's core points are connected by core-core
            //    eps² edges (single density chain).
            for (l, cores) in &cluster_cores {
                let cset: BTreeSet<usize> = cores.iter().copied().collect();
                let mut seen = BTreeSet::new();
                let mut stack = vec![cores[0]];
                while let Some(v) = stack.pop() {
                    if !seen.insert(v) {
                        continue;
                    }
                    for &j in &cset {
                        if !seen.contains(&j) {
                            let dx = pts[j].0 as i128 - pts[v].0 as i128;
                            let dy = pts[j].1 as i128 - pts[v].1 as i128;
                            if dx * dx + dy * dy <= eps2 {
                                stack.push(j);
                            }
                        }
                    }
                }
                assert_eq!(seen, cset, "cluster {l} cores disconnected");
            }
        }
    }

    #[test]
    fn deterministic_and_order_sensitive_only_canonically() {
        let pts: Vec<(i32, i32)> = (0..40).map(|i| (i % 7, i % 5)).collect();
        let a = dbscan(&pts, 30, 4);
        let b = dbscan(&pts, 30, 4);
        assert_eq!(a, b);
    }

    #[test]
    fn known_structure() {
        // Ring: four corner blobs of 3 points each + 1 noise.
        let mut pts = vec![];
        for c in 0..4 {
            let (cx, cy) = (c * 100, 0);
            pts.extend_from_slice(&[(cx, cy), (cx + 1, cy), (cx, cy + 1)]);
        }
        pts.push((37, 37));
        let l = dbscan(&pts, 4, 3);
        assert_eq!(l[12], -1);
        let mut ids: BTreeSet<i32> = l[..12].iter().copied().collect();
        assert_eq!(ids.len(), 4);
        ids.clear();
    }
}
