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
