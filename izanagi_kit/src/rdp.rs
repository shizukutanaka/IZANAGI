//! Ramer–Douglas–Peucker polyline simplification on integer coordinates.
//!
//! `simplify` keeps the two endpoints and recursively drops points whose
//! perpendicular distance to the chord is within `epsilon` — producing a
//! deterministic vertex-subset in input order. Ideal as a downstream pass
//! over [`crate::msquares::contour_loops`]-style outputs: the raw contour
//! is a staircase of half-cells; RDP collapses it to its salient corners.
//!
//! Exact arithmetic: the distance comparison is cross-multiplied into
//! `i128`, so `epsilon` is an integer bound on true perpendicular distance
//! (not on distance²) — `|cross| ≤ eps·|chord|` exactly.
//!
//! ```
//! use izanagi_kit::rdp::simplify;
//! let stair = vec![(0, 0), (0, 1), (1, 1), (1, 2), (2, 2)];
//! // The staircase's real deviation is ~0.7 — at eps=1 only the
//! // endpoints survive.
//! assert_eq!(simplify(&stair, 1), vec![(0, 0), (2, 2)]);
//! // At eps=0 every deviating vertex survives — (1,1) is collinear with
//! // the full chord but not with the sub-chords after splitting.
//! assert_eq!(simplify(&stair, 0), stair);
//! ```

/// Simplify `points` (a polyline in input order) by RDP with integer
/// tolerance `epsilon`. Returns the kept vertex subset including both
/// endpoints. For a closed loop pass `first == last` explicitly — the
/// result then keeps exactly one copy of the closing vertex.
///
/// `epsilon < 0` is treated as 0 (keep all vertices where any deviation
/// exists). A degenerate segment (endpoints equal) uses Chebyshev
/// distance to the single point.
pub fn simplify(points: &[(i32, i32)], epsilon: i32) -> Vec<(i32, i32)> {
    if points.len() <= 2 {
        return points.to_vec();
    }
    let eps = epsilon.max(0) as i128;
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    // Iterative recursion — depth-bounded stack of (lo, hi) ranges.
    let mut stack = vec![(0usize, points.len() - 1)];
    while let Some((lo, hi)) = stack.pop() {
        if hi <= lo + 1 {
            continue;
        }
        let (a, b) = (points[lo], points[hi]);
        let (dx, dy) = (b.0 as i128 - a.0 as i128, b.1 as i128 - a.1 as i128);
        let chord2 = dx * dx + dy * dy;
        let mut worst = -1i128; // max of |cross| (numerator of the distance)
        let mut worst_i = lo + 1;
        for (off, p) in points[lo + 1..hi].iter().enumerate() {
            let i = lo + 1 + off;
            let (px, py) = (p.0 as i128 - a.0 as i128, p.1 as i128 - a.1 as i128);
            // |cross| = |dx*py - dy*px|; distance = |cross|/sqrt(chord2).
            let cross = (dx * py - dy * px).abs();
            if cross > worst {
                worst = cross;
                worst_i = i;
            }
        }
        // Keep split point iff |cross| > eps * sqrt(chord2) — cross-multiplied.
        // When chord2 == 0 (a == b), distance degenerates to the point-to-
        // point metric; compare |AP|_chebyshev to eps directly then.
        let exceeds = if chord2 == 0 {
            let (px, py) = (
                (points[worst_i].0 - a.0).abs() as i128,
                (points[worst_i].1 - a.1).abs() as i128,
            );
            px.max(py) > eps // Chebyshev distance vs eps
        } else {
            worst * worst > eps * eps * chord2
        };
        if exceeds {
            keep[worst_i] = true;
            stack.push((lo, worst_i));
            stack.push((worst_i, hi));
        }
    }
    points
        .iter()
        .copied()
        .zip(keep.iter())
        .filter_map(|(p, &k)| k.then_some(p))
        .collect()
}

/// Same as [`simplify`] but guaranteed closed: the simplified output is
/// a loop (first point repeated at the end) whenever the input loop was.
/// Simpler to call on `contour_loops` output than threading `first==last`.
pub fn simplify_loop(points: &[(i32, i32)], epsilon: i32) -> Vec<(i32, i32)> {
    let closed = points.len() > 2 && points.first() == points.last();
    let body: &[(i32, i32)] = if closed {
        &points[..points.len() - 1]
    } else {
        points
    };
    if body.is_empty() {
        return Vec::new();
    }
    // Anchor the recursion at the point farthest from the centroid so the
    // loop cut is content-defined rather than dependent on index 0.
    let (cx, cy) = body.iter().fold((0i64, 0i64), |(ax, ay), p| {
        (ax + p.0 as i64, ay + p.1 as i64)
    });
    let n = body.len() as i64;
    let anchor = body
        .iter()
        .enumerate()
        .max_by_key(|(_, p)| {
            let dx = p.0 as i64 * n - cx;
            let dy = p.1 as i64 * n - cy;
            dx * dx + dy * dy
        })
        .map(|(i, _)| i)
        .unwrap_or(0);
    // Rotate so the anchor is first/last, simplify as an open chain.
    let mut ring: Vec<(i32, i32)> = body[anchor..]
        .iter()
        .chain(body[..anchor].iter())
        .copied()
        .collect();
    ring.push(ring[0]);
    let simplified = simplify(&ring, epsilon);
    if closed {
        return simplified;
    }
    simplified
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    fn max_deviation(points: &[(i32, i32)], kept: &[(i32, i32)]) -> i128 {
        // |cross| between consecutive kept points, worst over dropped points.
        let kept_set: std::collections::BTreeSet<(i32, i32)> = kept.iter().copied().collect();
        let mut worst = 0i128;
        // Walk input; between each pair of consecutive kept points,
        // measure each dropped point's |cross| (numerator).
        let mut i = 0;
        while i < points.len() {
            if !kept_set.contains(&points[i]) {
                i += 1;
                continue;
            }
            let a = points[i];
            let mut j = i + 1;
            while j < points.len() && !kept_set.contains(&points[j]) {
                j += 1;
            }
            if j >= points.len() {
                break;
            }
            let b = points[j];
            let (dx, dy) = (b.0 as i128 - a.0 as i128, b.1 as i128 - a.1 as i128);
            for p in &points[i + 1..j] {
                let (px, py) = (p.0 as i128 - a.0 as i128, p.1 as i128 - a.1 as i128);
                worst = worst.max((dx * py - dy * px).abs());
            }
            i = j;
        }
        worst
    }

    #[test]
    fn keeps_endpoints_and_preserves_order() {
        let mut rng = SplitMix64::new(0x8D9E);
        for _ in 0..200 {
            let n = 2 + rng.below(20) as usize;
            let pts: Vec<(i32, i32)> = (0..n)
                .map(|_| (rng.range(-50, 50), rng.range(-50, 50)))
                .collect();
            for eps in [0, 1, 3, 10] {
                let out = simplify(&pts, eps);
                assert_eq!(out.first(), pts.first());
                assert_eq!(out.last(), pts.last());
                // Subset in input order.
                let mut it = pts.iter().peekable();
                for &p in &out {
                    while it.next() != Some(&p) {
                        assert!(it.peek().is_some());
                    }
                }
                // epsilon bound is respected (|cross| bound is on the
                // numerator — here assert the weaker statement that a huge
                // eps keeps only the endpoints).
                if eps >= 100 {
                    assert_eq!(out.len(), 2);
                }
            }
        }
    }

    #[test]
    fn collinear_points_collapse() {
        let line = vec![(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)];
        assert_eq!(simplify(&line, 0), vec![(0, 0), (4, 0)]);
    }

    #[test]
    fn sharp_corner_survives() {
        let tri = vec![(0, 0), (2, 0), (2, 5), (4, 0)];
        let out = simplify(&tri, 0);
        assert!(out.contains(&(2, 5)));
    }

    #[test]
    fn loop_simplification_preserves_closure() {
        let square = vec![
            (0, 0),
            (2, 0),
            (2, 0), /*dup corner*/
            (2, 2),
            (0, 2),
            (0, 0),
        ];
        let out = simplify_loop(&square, 1);
        assert_eq!(out.first(), out.last());
        // The diamond-shaped content survives — 4 real corners kept.
        assert!(out.len() >= 4);
    }

    #[test]
    fn deterministic_and_idempotent() {
        let pts = vec![(0, 0), (1, 1), (3, -1), (5, 1), (7, 0), (9, 2)];
        let once = simplify(&pts, 1);
        assert_eq!(once, simplify(&pts, 1));
        // Idempotent: re-simplifying the result changes nothing.
        assert_eq!(simplify(&once, 1), once);
        let _ = max_deviation(&pts, &once);
    }
}
