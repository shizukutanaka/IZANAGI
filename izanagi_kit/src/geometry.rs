//! Integer line drawing and line-of-sight.
//!
//! Bresenham's algorithm traces the grid cells along a segment using only
//! integer add/compare — deterministic and cross-platform, like the rest of the
//! kit. Uses: drawing beams/projectile paths, simple ranged line-of-sight, and
//! "can A shoot B?" checks.
//!
//! Note on symmetry: a single Bresenham ray is *not* guaranteed symmetric
//! (`line(a, b)` may visit different cells than `line(b, a)`), so for fair
//! mutual visibility prefer [`crate::fov`]. [`line_of_sight`] here is the cheap
//! single-ray check, ideal for targeting where the shooter's viewpoint is
//! authoritative.

/// The grid cells on the Bresenham line from `a` to `b`, **inclusive** of both
/// endpoints, ordered from `a`. Consecutive cells are always king-move adjacent
/// (orthogonal or diagonal). `a == b` yields a single cell.
pub fn line(a: (i32, i32), b: (i32, i32)) -> Vec<(i32, i32)> {
    let (mut x, mut y) = a;
    let (x1, y1) = b;
    let dx = (x1 - x).abs();
    let dy = -(y1 - y).abs();
    let sx = if x < x1 { 1 } else { -1 };
    let sy = if y < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut cells = Vec::new();
    loop {
        cells.push((x, y));
        if x == x1 && y == y1 {
            break;
        }
        // `2*err` compared against dy/dx decides whether to step in x, y or both.
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
    cells
}

/// Is there a clear line of sight from `a` to `b`? Walks the Bresenham ray and
/// returns `false` if any cell **strictly between** the endpoints is opaque.
///
/// The endpoints themselves never block: standing on a wall still lets you look
/// out, and an opaque target (e.g. a wall or a monster you're aiming at) is
/// still considered visible. Adjacent or identical cells are always visible.
pub fn line_of_sight<F>(a: (i32, i32), b: (i32, i32), mut is_opaque: F) -> bool
where
    F: FnMut(i32, i32) -> bool,
{
    let cells = line(a, b);
    // Interior cells only: skip a (index 0) and b (last).
    if let Some(interior) = cells.get(1..cells.len().saturating_sub(1)) {
        for &(x, y) in interior {
            if is_opaque(x, y) {
                return false;
            }
        }
    }
    true
}

/// The grid cells on the perimeter of a circle centred at `(cx, cy)` with
/// the given `radius`, using the Bresenham midpoint circle algorithm. The
/// returned cells are in ascending `(y, x)` order, without duplicates. An
/// empty `Vec` is returned for negative `radius`; `radius == 0` yields just
/// the centre cell.
///
/// Use this for drawing AoE rings, targeting outlines, and splash-attack
/// indicators.
pub fn circle(cx: i32, cy: i32, radius: i32) -> Vec<(i32, i32)> {
    if radius < 0 {
        return Vec::new();
    }
    if radius == 0 {
        return vec![(cx, cy)];
    }
    let mut pts = std::collections::BTreeSet::new();
    let mut x = radius;
    let mut y = 0;
    let mut p: i32 = 1 - radius;
    while x >= y {
        let plots = [
            (cx + x, cy + y),
            (cx - x, cy + y),
            (cx + x, cy - y),
            (cx - x, cy - y),
            (cx + y, cy + x),
            (cx - y, cy + x),
            (cx + y, cy - x),
            (cx - y, cy - x),
        ];
        for pt in plots {
            pts.insert(pt);
        }
        y += 1;
        if p < 0 {
            p += 2 * y + 1;
        } else {
            x -= 1;
            p += 2 * (y - x) + 1;
        }
    }
    pts.into_iter().collect()
}

/// All grid cells strictly within (and on the boundary of) a circle centred
/// at `(cx, cy)` with the given `radius` — i.e. cells `(x, y)` where
/// `(x − cx)² + (y − cy)² ≤ radius²`. Results are in ascending `(y, x)`
/// order. An empty `Vec` for negative `radius`; just the centre for
/// `radius == 0`.
///
/// Use this for AoE fill effects, fog-of-war circles, and region queries.
pub fn filled_circle(cx: i32, cy: i32, radius: i32) -> Vec<(i32, i32)> {
    if radius < 0 {
        return Vec::new();
    }
    let r2 = (radius as i64) * (radius as i64);
    let mut pts = Vec::new();
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if (dx as i64) * (dx as i64) + (dy as i64) * (dy as i64) <= r2 {
                pts.push((cx + dx, cy + dy));
            }
        }
    }
    pts
}

/// All cells in the axis-aligned rectangle `[x, x+w) × [y, y+h)`, in
/// row-major order `(y then x)`. Returns an empty `Vec` for non-positive `w`
/// or `h`.
///
/// Use this for room fill operations, AoE rectangles, and region seeds.
pub fn rect(x: i32, y: i32, w: i32, h: i32) -> Vec<(i32, i32)> {
    if w <= 0 || h <= 0 {
        return Vec::new();
    }
    let mut pts = Vec::with_capacity((w * h) as usize);
    for dy in 0..h {
        for dx in 0..w {
            pts.push((x + dx, y + dy));
        }
    }
    pts
}

/// Cells in the annular ring between `inner_r` and `outer_r` (inclusive on
/// both boundaries): `(x,y)` where `inner_r² ≤ (x−cx)²+(y−cy)² ≤ outer_r²`.
/// Returns an empty `Vec` for non-positive `outer_r` or `inner_r ≥ outer_r`.
/// `inner_r ≤ 0` is treated as 0, producing a filled circle.
///
/// Use this for doughnut-shaped AoE effects, targeting rings that skip the
/// immediate vicinity, and ambient light falloff halos.
pub fn ring_annulus(cx: i32, cy: i32, inner_r: i32, outer_r: i32) -> Vec<(i32, i32)> {
    if outer_r <= 0 || inner_r >= outer_r {
        return Vec::new();
    }
    let inner_r2 = (inner_r.max(0) as i64) * (inner_r.max(0) as i64);
    let outer_r2 = (outer_r as i64) * (outer_r as i64);
    let mut pts = Vec::new();
    for dy in -outer_r..=outer_r {
        for dx in -outer_r..=outer_r {
            let d2 = (dx as i64) * (dx as i64) + (dy as i64) * (dy as i64);
            if d2 >= inner_r2 && d2 <= outer_r2 {
                pts.push((cx + dx, cy + dy));
            }
        }
    }
    pts
}

/// Grid distance metrics between two cells.
///
/// Mirrors the distance algorithms a roguelike toolkit needs (cf. `bracket-lib`
/// `DistanceAlg`): pick the metric that matches your movement rules — `Manhattan`
/// for 4-way movement, `Chebyshev` for 8-way (king moves), and the Euclidean
/// pair for true radial range checks.
///
/// All results are integers. `Euclidean` returns the **floor** of the true
/// distance via an integer square root, so it stays float-free and identical
/// across platforms; use `EuclideanSquared` when you only need to compare
/// ranges (it avoids the square root entirely).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Distance {
    /// `|dx| + |dy|` — taxicab distance, for 4-way (orthogonal) movement.
    Manhattan,
    /// `max(|dx|, |dy|)` — chessboard distance, for 8-way (king) movement.
    Chebyshev,
    /// `dx² + dy²` — squared Euclidean; cheap and exact for range comparisons.
    EuclideanSquared,
    /// `floor(sqrt(dx² + dy²))` — Euclidean distance, integer (floored).
    Euclidean,
}

impl Distance {
    /// Distance between `a` and `b` under this metric. Saturates to `i32::MAX`
    /// rather than overflowing for extreme coordinates.
    pub fn between(self, a: (i32, i32), b: (i32, i32)) -> i32 {
        let dx = (a.0 as i64 - b.0 as i64).abs();
        let dy = (a.1 as i64 - b.1 as i64).abs();
        let v = match self {
            Distance::Manhattan => dx + dy,
            Distance::Chebyshev => dx.max(dy),
            Distance::EuclideanSquared => dx * dx + dy * dy,
            Distance::Euclidean => isqrt(dx * dx + dy * dy),
        };
        v.min(i32::MAX as i64) as i32
    }
}

/// Floor of the square root of a non-negative `i64`, computed with integer
/// arithmetic only (Newton's method). `isqrt(n)² <= n < (isqrt(n)+1)²`.
fn isqrt(n: i64) -> i64 {
    if n < 2 {
        return n.max(0);
    }
    // Newton's method; converges from above to the floor of the real root.
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_rect_basic() {
        let r = rect(1, 2, 3, 2);
        assert_eq!(r, vec![(1, 2), (2, 2), (3, 2), (1, 3), (2, 3), (3, 3)]);
    }

    #[test]
    fn test_rect_zero_dimension_is_empty() {
        assert!(rect(0, 0, 0, 4).is_empty());
        assert!(rect(0, 0, 4, 0).is_empty());
        assert!(rect(0, 0, -1, 3).is_empty());
    }

    #[test]
    fn test_rect_area_matches_dimensions() {
        let r = rect(5, 5, 4, 3);
        assert_eq!(r.len(), 12);
    }

    #[test]
    fn test_ring_annulus_excludes_inner() {
        let ring = ring_annulus(0, 0, 2, 4);
        // Centre (0,0): d²=0 < inner²=4 → excluded
        assert!(!ring.contains(&(0, 0)));
        // (2,0): d²=4 = inner² → included
        assert!(ring.contains(&(2, 0)));
        // (4,0): d²=16 = outer² → included
        assert!(ring.contains(&(4, 0)));
        // (5,0): d²=25 > outer²=16 → excluded
        assert!(!ring.contains(&(5, 0)));
    }

    #[test]
    fn test_ring_annulus_zero_outer_is_empty() {
        assert!(ring_annulus(0, 0, 0, 0).is_empty());
        assert!(ring_annulus(0, 0, 1, -1).is_empty());
    }

    #[test]
    fn test_ring_annulus_inner_zero_is_filled_circle() {
        let ring = ring_annulus(0, 0, 0, 3);
        let filled = filled_circle(0, 0, 3);
        // Both should contain the same cells (order may differ).
        let mut r2 = ring.clone();
        let mut f2 = filled.clone();
        r2.sort_unstable();
        f2.sort_unstable();
        assert_eq!(r2, f2);
    }

    #[test]
    fn test_single_cell_when_endpoints_equal() {
        assert_eq!(line((4, 4), (4, 4)), vec![(4, 4)]);
    }

    #[test]
    fn test_horizontal_and_vertical_lines() {
        assert_eq!(line((0, 0), (3, 0)), vec![(0, 0), (1, 0), (2, 0), (3, 0)]);
        assert_eq!(
            line((0, 0), (0, -3)),
            vec![(0, 0), (0, -1), (0, -2), (0, -3)]
        );
    }

    #[test]
    fn test_pure_diagonal_line() {
        assert_eq!(line((0, 0), (3, 3)), vec![(0, 0), (1, 1), (2, 2), (3, 3)]);
    }

    #[test]
    fn test_endpoints_and_adjacency() {
        let l = line((1, 2), (9, 5));
        assert_eq!(l.first(), Some(&(1, 2)));
        assert_eq!(l.last(), Some(&(9, 5)));
        // Every step is king-move adjacent (no gaps).
        for w in l.windows(2) {
            let (dx, dy) = ((w[1].0 - w[0].0).abs(), (w[1].1 - w[0].1).abs());
            assert!(
                dx <= 1 && dy <= 1 && (dx + dy) > 0,
                "non-adjacent step {w:?}"
            );
        }
    }

    #[test]
    fn test_line_of_sight_open_is_visible() {
        assert!(line_of_sight((0, 0), (5, 2), |_, _| false));
    }

    #[test]
    fn test_line_of_sight_blocked_by_interior_wall() {
        // Wall on the straight path between (0,0) and (4,0).
        let walls: HashSet<(i32, i32)> = [(2, 0)].into_iter().collect();
        assert!(!line_of_sight((0, 0), (4, 0), |x, y| walls.contains(&(x, y))));
    }

    #[test]
    fn test_line_of_sight_ignores_opaque_endpoints() {
        // Opaque target (and origin) must not block — you still see the wall.
        let walls: HashSet<(i32, i32)> = [(0, 0), (4, 0)].into_iter().collect();
        assert!(line_of_sight((0, 0), (4, 0), |x, y| walls.contains(&(x, y))));
        // Adjacent cells are always visible.
        assert!(line_of_sight((3, 3), (4, 3), |_, _| true));
    }

    #[test]
    fn test_distance_manhattan_and_chebyshev() {
        let a = (1, 2);
        let b = (4, 6); // dx=3, dy=4
        assert_eq!(Distance::Manhattan.between(a, b), 7);
        assert_eq!(Distance::Chebyshev.between(a, b), 4);
    }

    #[test]
    fn test_distance_euclidean_pair() {
        let a = (0, 0);
        let b = (3, 4); // 3-4-5 triangle
        assert_eq!(Distance::EuclideanSquared.between(a, b), 25);
        assert_eq!(Distance::Euclidean.between(a, b), 5);
    }

    #[test]
    fn test_distance_euclidean_floors() {
        // sqrt(2) ≈ 1.41 → floors to 1; sqrt(8) ≈ 2.83 → floors to 2.
        assert_eq!(Distance::Euclidean.between((0, 0), (1, 1)), 1);
        assert_eq!(Distance::Euclidean.between((0, 0), (2, 2)), 2);
        assert_eq!(Distance::EuclideanSquared.between((0, 0), (1, 1)), 2);
    }

    #[test]
    fn test_distance_symmetric_and_zero() {
        let a = (-3, 5);
        let b = (7, -2);
        for m in [
            Distance::Manhattan,
            Distance::Chebyshev,
            Distance::EuclideanSquared,
            Distance::Euclidean,
        ] {
            assert_eq!(m.between(a, b), m.between(b, a), "{m:?} not symmetric");
            assert_eq!(m.between(a, a), 0, "{m:?} self-distance not zero");
        }
    }

    // ── circle / filled_circle ───────────────────────────────────────────────

    #[test]
    fn test_circle_zero_radius_is_centre() {
        assert_eq!(circle(3, 4, 0), vec![(3, 4)]);
    }

    #[test]
    fn test_circle_negative_radius_is_empty() {
        assert!(circle(0, 0, -1).is_empty());
    }

    #[test]
    fn test_circle_radius_1_has_4_cells() {
        // A radius-1 circle in king-move terms visits the 4 cardinal neighbours.
        let c = circle(0, 0, 1);
        // Should contain exactly the cardinal directions (Bresenham r=1).
        assert!(c.contains(&(-1, 0)));
        assert!(c.contains(&(1, 0)));
        assert!(c.contains(&(0, -1)));
        assert!(c.contains(&(0, 1)));
    }

    #[test]
    fn test_circle_no_duplicates() {
        for r in 0..=10 {
            let c = circle(5, 5, r);
            let mut s = c.clone();
            s.sort_unstable();
            s.dedup();
            assert_eq!(c.len(), s.len(), "radius {r} has duplicates");
        }
    }

    #[test]
    fn test_circle_sorted_ascending() {
        let c = circle(0, 0, 5);
        for w in c.windows(2) {
            assert!(w[0] <= w[1], "not sorted: {:?} > {:?}", w[0], w[1]);
        }
    }

    #[test]
    fn test_filled_circle_contains_centre() {
        let f = filled_circle(2, 3, 2);
        assert!(f.contains(&(2, 3)));
    }

    #[test]
    fn test_filled_circle_zero_radius_is_centre_only() {
        assert_eq!(filled_circle(1, 1, 0), vec![(1, 1)]);
    }

    #[test]
    fn test_filled_circle_euclidean_criterion() {
        // Every cell in filled_circle must satisfy dx²+dy² ≤ r².
        let r = 5;
        for (x, y) in filled_circle(0, 0, r) {
            let d2 = (x as i64) * (x as i64) + (y as i64) * (y as i64);
            assert!(d2 <= (r as i64) * (r as i64), "({x},{y}) outside circle");
        }
    }

    #[test]
    fn test_filled_circle_area_approx_pi_r_sq() {
        // Area should be roughly π·r² ± a few cells.
        let r = 10;
        let area = filled_circle(0, 0, r).len() as f64;
        let expected = std::f64::consts::PI * (r as f64) * (r as f64);
        assert!(
            (area - expected).abs() < expected * 0.05,
            "area={area} expected≈{expected:.1}"
        );
    }

    #[test]
    fn test_isqrt_floor_property() {
        for n in 0..1000i64 {
            let r = isqrt(n);
            assert!(r * r <= n && (r + 1) * (r + 1) > n, "isqrt({n})={r}");
        }
        // Large value stays correct.
        assert_eq!(isqrt(1_000_000), 1000);
    }
}
