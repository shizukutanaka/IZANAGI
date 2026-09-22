//! Largest-Triangle-Three-Buckets time-series downsampling — keeps
//! visually-significant points when rendering dense series onto small
//! charts (HUD graphs, profiler timelines, economy dashboards).
//!
//! Sveinn Steinarsson's LTTB (University of Iceland, 2013) is the
//! standard "doesn't destroy the shape" downsampler: each bucket keeps
//! the point that forms the largest triangle with the previous
//! selection and the next bucket's centroid. Implemented on `i64`
//! coordinates with `i128` twice-area math — exact, no floats, fully
//! deterministic.
//!
//! ```
//! use izanagi_kit::lttb::lttb;
//! let series: Vec<(i64, i64)> = (0..100).map(|x| (x, (x * x) % 50)).collect();
//! let small = lttb(&series, 10).unwrap();
//! assert_eq!(small.len(), 10);
//! assert_eq!(small.first(), series.first());
//! assert_eq!(small.last(), series.last());
//! ```

/// Downsample `pts` to `threshold` points via LTTB.
///
/// Contract:
/// - `threshold >= pts.len()` → returns the input unchanged (clone).
/// - `threshold < 3` and input longer than 3 → `None` (the algorithm
///   needs first + last + at least one bucket point).
/// - Output is a subsequence of the input, endpoints preserved, order
///   preserved — a pure function of the data.
pub fn lttb(pts: &[(i64, i64)], threshold: usize) -> Option<Vec<(i64, i64)>> {
    let n = pts.len();
    if threshold >= n {
        return Some(pts.to_vec());
    }
    if threshold < 3 || n < 3 {
        return None;
    }

    let bucket_count = threshold - 2;
    // Bucket boundaries: bucket i covers [i*(n-2)/bucket_count + 1,
    // (i+1)*(n-2)/bucket_count + 1) — floor division keeps every
    // interior point in exactly one bucket.
    let span = n - 2;
    let mut out = Vec::with_capacity(threshold);
    out.push(pts[0]);

    let mut a = pts[0]; // selected point of the previous bucket
    for i in 0..bucket_count {
        // Bucket ranges by floor division — deterministic, stable.
        let lo = 1 + i * span / bucket_count;
        let hi = 1 + (i + 1) * span / bucket_count;
        // Average point of the *next* bucket.
        let nlo = hi;
        let nhi = 1 + (i + 2) * span / bucket_count;
        let nhi = nhi.min(n);
        let mut ax = 0i128;
        let mut ay = 0i128;
        for p in &pts[nlo.min(nhi)..nhi] {
            ax += p.0 as i128;
            ay += p.1 as i128;
        }
        let cnt = (nhi - nlo.min(nhi)) as i128;
        let (ax, ay) = if cnt > 0 {
            (ax / cnt, ay / cnt)
        } else {
            (0, 0)
        };

        // Pick the bucket point maximizing 2×triangle area with `a`
        // and the next-bucket centroid — signed areas need abs.
        let mut best_area = -1i128;
        let mut best = pts[lo.min(hi - 1)];
        for p in &pts[lo..hi] {
            let area2 = (a.0 as i128 - ax) * (p.1 as i128 - a.1 as i128)
                - (a.0 as i128 - p.0 as i128) * (ay - a.1 as i128);
            let area2 = area2.abs();
            if area2 > best_area {
                best_area = area2;
                best = *p;
            }
        }
        out.push(best);
        a = best;
    }
    out.push(pts[n - 1]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn structural_contracts_hold() {
        let mut rng = SplitMix64::new(0xD00D);
        for _ in 0..300 {
            let n = 4 + rng.below(300) as usize;
            let pts: Vec<(i64, i64)> = (0..n)
                .map(|i| (i as i64, rng.range(-1000, 1001) as i64))
                .collect();
            // threshold must stay strictly below n — at t >= n the
            // function returns the input unchanged by contract.
            let t = 3 + rng.below(n as u32 - 3) as usize;
            let out = lttb(&pts, t).unwrap();
            assert_eq!(out.len(), t);
            assert_eq!(out.first(), pts.first());
            assert_eq!(out.last(), pts.last());
            // Output is an order-preserving subsequence of input.
            let mut cursor = 0usize;
            for &p in &out {
                let found = pts[cursor..].iter().position(|&q| q == p);
                assert!(found.is_some(), "{p:?} not a later input point");
                cursor += found.unwrap();
            }
        }
    }

    #[test]
    fn trivial_and_boundary_cases() {
        // threshold ≥ n → input clone, no loss.
        let pts = vec![(0i64, 1i64), (1, 5), (2, 2), (3, 9)];
        assert_eq!(lttb(&pts, 4), Some(pts.clone()));
        assert_eq!(lttb(&pts, 10), Some(pts.clone()));
        // threshold < 3 with n > 3 → None.
        assert_eq!(lttb(&pts, 2), None);
        // n < 3 with n > threshold → short-circuit rules first.
        let two = vec![(0i64, 0i64), (1, 1)];
        assert_eq!(lttb(&two, 3), Some(two.clone()));
        // Empty input.
        assert_eq!(lttb(&[], 5), Some(vec![]));
        // Deterministic: identical call twice.
        assert_eq!(lttb(&pts, 3), lttb(&pts, 3));
    }

    #[test]
    fn preserves_the_spike_shape() {
        // A lone spike in flat data must survive — the classic case
        // every naive "keep every k-th" sampler destroys.
        let mut pts: Vec<(i64, i64)> = (0..100).map(|x| (x, 0)).collect();
        pts[50] = (50, 1000);
        let out = lttb(&pts, 3).unwrap();
        assert_eq!(out, vec![(0, 0), (50, 1000), (99, 0)]);
        // Monotone ramp + threshold n-1 → all kept.
        let ramp: Vec<(i64, i64)> = (0..20).map(|x| (x, x)).collect();
        assert_eq!(lttb(&ramp, 20), Some(ramp));
    }
}
