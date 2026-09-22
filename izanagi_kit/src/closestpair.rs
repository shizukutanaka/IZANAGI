//! Closest pair of points — `O(n log n)` divide-and-conquer on integer
//! coordinates, all math in `i128` squared distances.
//!
//! Density checks on generated layouts ("did `poisson_disc` actually
//! keep its spacing"), nearest-neighbour seeds for `voronoi`, collision
//! hotspots. [`crate::zorder::spatial_sort`] gives a locality order for
//! broad sweeps; this is the exact answer.
//!
//! Tie-break contract: among all pairs achieving the minimum distance,
//! the returned `(p, q)` is the lexicographically smallest with
//! `p < q` — a pure function of the point set.
//!
//! ```
//! use izanagi_kit::closestpair::closest_pair;
//! let pts = [(0,0), (5,5), (1,0), (9,9)];
//! let (d2, p, q) = closest_pair(&pts).unwrap();
//! assert_eq!(d2, 1);
//! assert_eq!((p, q), ((0,0), (1,0)));
//! ```

/// Result of [`closest_pair`]: `(dist², p, q)` with `p < q`
/// lexicographically, smallest among equal-distance pairs.
pub type ClosestPair = (i128, (i32, i32), (i32, i32));

/// Exact squared distance in `i128`.
fn dist2(a: (i32, i32), b: (i32, i32)) -> i128 {
    let dx = a.0 as i128 - b.0 as i128;
    let dy = a.1 as i128 - b.1 as i128;
    dx * dx + dy * dy
}

/// Canonical pair ordering: `p` is the lexicographically smaller point.
fn canon(a: (i32, i32), b: (i32, i32)) -> ((i32, i32), (i32, i32)) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// The closest pair in `pts` — `Some((dist², p, q))` with `p < q`,
/// lexicographically smallest among equal-distance pairs. `None` when
/// `pts.len() < 2`.
///
/// `O(n log n)` divide-and-conquer with a y-sorted strip scan; every
/// comparison that can tie is ordered by `(dist², p, q)` so the result
/// is content-defined.
pub fn closest_pair(pts: &[(i32, i32)]) -> Option<ClosestPair> {
    if pts.len() < 2 {
        return None;
    }
    let mut by_x = pts.to_vec();
    by_x.sort(); // (x, y) order — deterministic on duplicates too
                 // Remove exact duplicates early? No — a duplicated point gives
                 // dist² 0 and must be found; keep them.
    let mut buf = vec![(0i32, 0i32); by_x.len()];
    let mut best: ClosestPair = (i128::MAX, (i32::MAX, i32::MAX), (i32::MAX, i32::MAX));
    let len = by_x.len();
    rec(&mut by_x, &mut buf, 0, len, &mut best);
    Some(best)
}

/// `pts[lo..hi)` is x-sorted on entry and gets merge-sorted by y on
/// exit; `buf` is scratch of the same length.
fn rec(
    pts: &mut [(i32, i32)],
    buf: &mut [(i32, i32)],
    lo: usize,
    hi: usize,
    best: &mut ClosestPair,
) {
    let n = hi - lo;
    if n <= 3 {
        // Brute force the base case, then y-sort the range.
        for i in lo..hi {
            for j in i + 1..hi {
                update(pts[i], pts[j], best);
            }
        }
        pts[lo..hi].sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
        return;
    }
    let mid = lo + n / 2;
    let midx = pts[mid].0;
    rec(pts, buf, lo, mid, best);
    rec(pts, buf, mid, hi, best);

    // Merge the two y-sorted halves back into y order.
    let (mut i, mut j, mut k) = (lo, mid, lo);
    while i < mid && j < hi {
        if pts[i].1 <= pts[j].1 {
            buf[k] = pts[i];
            i += 1;
        } else {
            buf[k] = pts[j];
            j += 1;
        }
        k += 1;
    }
    while i < mid {
        buf[k] = pts[i];
        i += 1;
        k += 1;
    }
    while j < hi {
        buf[k] = pts[j];
        j += 1;
        k += 1;
    }
    pts[lo..hi].copy_from_slice(&buf[lo..hi]);

    // Strip scan: points within `best.0` of the split line (inclusive —
    // equal-distance pairs must still be examined for the tie-break).
    let strip: Vec<(i32, i32)> = pts[lo..hi]
        .iter()
        .copied()
        .filter(|p| {
            let dx = p.0 as i128 - midx as i128;
            dx * dx <= best.0
        })
        .collect();
    for i in 0..strip.len() {
        let mut j = i + 1;
        while j < strip.len() {
            let dy = strip[j].1 as i128 - strip[i].1 as i128;
            if dy * dy > best.0 {
                break;
            }
            update(strip[i], strip[j], best);
            j += 1;
        }
    }
}

/// `best` := min of `(dist², p, q)` with canonical pair ordering.
fn update(a: (i32, i32), b: (i32, i32), best: &mut ClosestPair) {
    let d = dist2(a, b);
    let (p, q) = canon(a, b);
    if d < best.0 || (d == best.0 && (p, q) < (best.1, best.2)) {
        *best = (d, p, q);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Oracle: O(n²) exhaustive argmin with the same tie-break.
    fn brute(pts: &[(i32, i32)]) -> Option<ClosestPair> {
        if pts.len() < 2 {
            return None;
        }
        let mut best: ClosestPair = (i128::MAX, (i32::MAX, i32::MAX), (i32::MAX, i32::MAX));
        for i in 0..pts.len() {
            for j in i + 1..pts.len() {
                let d = dist2(pts[i], pts[j]);
                let (p, q) = canon(pts[i], pts[j]);
                if d < best.0 || (d == best.0 && (p, q) < (best.1, best.2)) {
                    best = (d, p, q);
                }
            }
        }
        Some(best)
    }

    #[test]
    fn matches_brute_force_on_random_sets() {
        let mut rng = SplitMix64::new(0xC105E);
        for _ in 0..400 {
            let n = 2 + rng.below(40) as usize;
            let pts: Vec<(i32, i32)> = (0..n)
                .map(|_| (rng.range(-50, 51), rng.range(-50, 51)))
                .collect();
            assert_eq!(closest_pair(&pts), brute(&pts), "pts={pts:?}");
        }
    }

    #[test]
    fn duplicates_and_degenerate_inputs() {
        // Duplicated point ⇒ dist² 0.
        let pts = [(3, 3), (0, 0), (3, 3), (9, 9)];
        assert_eq!(closest_pair(&pts), Some((0, (3, 3), (3, 3))));
        assert_eq!(closest_pair(&[]), None);
        assert_eq!(closest_pair(&[(1, 1)]), None);
        // Two points only.
        assert_eq!(closest_pair(&[(0, 0), (3, 4)]), Some((25, (0, 0), (3, 4))));
    }

    #[test]
    fn answer_is_input_order_independent() {
        // Same set, shuffled input — identical answer.
        let mut rng = SplitMix64::new(0xCAFE);
        for _ in 0..100 {
            let n = 5 + rng.below(20) as usize;
            let mut pts: Vec<(i32, i32)> = (0..n)
                .map(|_| (rng.range(-30, 31), rng.range(-30, 31)))
                .collect();
            let want = closest_pair(&pts);
            // Reverse + rotate — different orders, same set.
            pts.reverse();
            pts.rotate_left(3);
            assert_eq!(closest_pair(&pts), want);
        }
    }
}
