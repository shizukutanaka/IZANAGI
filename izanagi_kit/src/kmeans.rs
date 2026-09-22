//! Deterministic k-means — Lloyd's algorithm with two integer-only
//! adjustments that remove all randomness: centroids initialize by
//! farthest-point traversal (Gonzalez-style — deterministic, no
//! seed), and cluster means are rounded to integer lattice points.
//! Iteration runs until the assignment is stable — a fixpoint —
//! which the bounded-inertia decrease guarantees; a hard iteration
//! cap keeps adversarial configurations finite.
//!
//! The result is a pure function of `(points, k)`. Every returned
//! label is the argmin of squared distances to the final centers by
//! construction (the loop only stops on stability), and ties break
//! to the lower center index for a canonical answer.
//!
//! ```
//! use izanagi_kit::kmeans::kmeans;
//! let pts = [(0, 0), (1, 0), (0, 1), (50, 50), (51, 50), (50, 51)];
//! let r = kmeans(&pts, 2);
//! assert_eq!(r.labels[0], r.labels[1]);
//! assert_eq!(r.labels[3], r.labels[4]);
//! assert_ne!(r.labels[0], r.labels[3]);
//! ```

/// K-means result: integer `centers` plus per-point `labels` in
/// `0..k`, assigned by nearest-center argmin.
#[derive(Clone, Debug)]
pub struct KMeans {
    /// Final integer centroid positions.
    pub centers: Vec<(i32, i32)>,
    /// `labels[i]` = index of `points[i]`'s assigned center.
    pub labels: Vec<u32>,
    /// Lloyd iterations actually performed.
    pub iterations: u32,
    /// Total squared distance from points to their centers.
    pub inertia: i128,
}

const MAX_ITERS: u32 = 256;

fn dist2(a: (i32, i32), b: (i32, i32)) -> i128 {
    let dx = a.0 as i128 - b.0 as i128;
    let dy = a.1 as i128 - b.1 as i128;
    dx * dx + dy * dy
}

/// `labels[i]` = argmin_c dist²(points[i], centers[c]); ties → lower c.
fn assign(points: &[(i32, i32)], centers: &[(i32, i32)]) -> Vec<u32> {
    points
        .iter()
        .map(|&p| {
            let mut best = 0usize;
            let mut bd = dist2(p, centers[0]);
            for (c, &ctr) in centers.iter().enumerate().skip(1) {
                let d = dist2(p, ctr);
                if d < bd {
                    bd = d;
                    best = c;
                }
            }
            best as u32
        })
        .collect()
}

/// Deterministic farthest-point seeding: start at the first point
/// (index 0 wins the initial argmax of an all-equal competition via
/// the strict `>`), then repeatedly take the point farthest from the
/// nearest chosen center.
fn farthest_init(points: &[(i32, i32)], k: usize) -> Vec<(i32, i32)> {
    let mut centers = vec![points[0]];
    while centers.len() < k {
        let mut best = 0usize;
        let mut bd = -1i128;
        for (i, &p) in points.iter().enumerate() {
            let d = centers.iter().map(|&c| dist2(p, c)).min().unwrap_or(0);
            if d > bd || (d == bd && i < best) {
                bd = d;
                best = i;
            }
        }
        centers.push(points[best]);
    }
    centers
}

/// Deterministic integer k-means. `k = 0` or empty points yields an
/// empty `KMeans`; `k > points.len()` clamps to `points.len()`.
pub fn kmeans(points: &[(i32, i32)], k: usize) -> KMeans {
    if points.is_empty() || k == 0 {
        return KMeans {
            centers: Vec::new(),
            labels: Vec::new(),
            iterations: 0,
            inertia: 0,
        };
    }
    let k = k.min(points.len());
    let mut centers = farthest_init(points, k);
    let mut labels = assign(points, &centers);
    let mut iterations = 0;
    while iterations < MAX_ITERS {
        iterations += 1;
        // Update step — integer mean per cluster; empty clusters keep
        // their old center (they re-seed nowhere: deterministic).
        let mut sum = vec![(0i128, 0i128, 0u64); k];
        for (i, &p) in points.iter().enumerate() {
            let c = labels[i] as usize;
            sum[c].0 += p.0 as i128;
            sum[c].1 += p.1 as i128;
            sum[c].2 += 1;
        }
        for (c, ctr) in centers.iter_mut().enumerate() {
            if sum[c].2 > 0 {
                *ctr = (
                    (sum[c].0 / sum[c].2 as i128) as i32,
                    (sum[c].1 / sum[c].2 as i128) as i32,
                );
            }
        }
        let next = assign(points, &centers);
        if next == labels {
            break;
        }
        labels = next;
    }
    let inertia = points
        .iter()
        .enumerate()
        .map(|(i, &p)| dist2(p, centers[labels[i] as usize]))
        .sum();
    KMeans {
        centers,
        labels,
        iterations,
        inertia,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn known_two_blobs() {
        let pts = [(0, 0), (1, 0), (0, 1), (50, 50), (51, 50), (50, 51)];
        let r = kmeans(&pts, 2);
        assert_eq!(r.labels[0], r.labels[1]);
        assert_eq!(r.labels[3], r.labels[4]);
        assert_ne!(r.labels[0], r.labels[3]);
        assert!(r.iterations <= MAX_ITERS);
    }

    #[test]
    fn every_label_is_argmin_consistent() {
        let mut rng = SplitMix64::new(41);
        for _ in 0..20 {
            let n = rng.below(40) as usize + 4;
            let k = rng.below(6) as usize + 1;
            let pts: Vec<(i32, i32)> = (0..n)
                .map(|_| (rng.below(50) as i32, rng.below(50) as i32))
                .collect();
            let r = kmeans(&pts, k);
            for (i, &p) in pts.iter().enumerate() {
                let d = dist2(p, r.centers[r.labels[i] as usize]);
                for &c in &r.centers {
                    assert!(d <= dist2(p, c), "label not argmin at {i}");
                }
            }
            // Determinism: identical calls → identical result.
            let r2 = kmeans(&pts, k);
            assert_eq!(r.centers, r2.centers);
            assert_eq!(r.labels, r2.labels);
        }
    }

    #[test]
    fn degenerate_inputs() {
        assert!(kmeans(&[], 3).centers.is_empty());
        assert!(kmeans(&[(1, 1)], 0).centers.is_empty());
        let r = kmeans(&[(1, 1), (2, 2)], 5);
        assert_eq!(r.centers.len(), 2); // clamped to n
        assert_eq!(r.inertia, 0); // singleton clusters
    }

    #[test]
    fn inertia_matches_reported_centers() {
        let pts = [(0, 0), (2, 0), (10, 10), (12, 10), (10, 12)];
        let r = kmeans(&pts, 2);
        let check: i128 = pts
            .iter()
            .enumerate()
            .map(|(i, &p)| dist2(p, r.centers[r.labels[i] as usize]))
            .sum();
        assert_eq!(r.inertia, check);
    }
}
