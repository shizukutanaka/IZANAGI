//! Symmetric recursive shadowcasting field-of-view (FOV).
//!
//! Roguelikes need to know which cells an observer can see. Naive raycasting is
//! cheap but *asymmetric* — A can see B while B cannot see A — which produces
//! visual artifacts and unfair gameplay. This module implements Albert Ford's
//! *symmetric shadowcasting* (a refinement of Björn Bergström's recursive
//! shadowcasting; see RogueBasin and journal.stuffwithstuff "What the Hero
//! Sees"), the de-facto standard used by toolkits like `bracket-lib` and
//! `libtcod`. A sees B iff B sees A.
//!
//! Determinism: the algorithm is **integer-only** (slopes are rationals, not
//! `f32`), processes the four quadrants and their columns in a fixed order, and
//! allocates nothing beyond the recursion stack. Results are therefore
//! bit-identical across targets and safe to derive replay-visible state from.
//!
//! The cell at `origin` is always revealed. Cells are limited to a Euclidean
//! `radius`. `mark_visible` may be called more than once for the same cell
//! (axis/diagonal cells are shared between quadrants), so callers should treat
//! it idempotently (e.g. insert into a set or flip a bool).

/// A rational `num/den` with `den > 0`, used for octant slopes. Integer storage
/// keeps FOV free of floating-point so it stays cross-platform deterministic.
#[derive(Clone, Copy)]
struct Frac {
    num: i64,
    den: i64,
}

impl Frac {
    #[inline]
    fn new(num: i64, den: i64) -> Frac {
        debug_assert!(den > 0, "Frac denominator must be positive");
        Frac { num, den }
    }
}

/// Floor of `a / b` for `b > 0`.
#[inline]
fn floor_div(a: i64, b: i64) -> i64 {
    a.div_euclid(b)
}

/// Ceil of `a / b` for `b > 0`.
#[inline]
fn ceil_div(a: i64, b: i64) -> i64 {
    -((-a).div_euclid(b))
}

/// Round `f` to the nearest integer, ties going up: `floor(f + 1/2)`.
#[inline]
fn round_ties_up(f: Frac) -> i64 {
    floor_div(2 * f.num + f.den, 2 * f.den)
}

/// Round `f` to the nearest integer, ties going down: `ceil(f - 1/2)`.
#[inline]
fn round_ties_down(f: Frac) -> i64 {
    ceil_div(2 * f.num - f.den, 2 * f.den)
}

/// Slope of the cell `(depth, col)`: the line through the origin and the cell's
/// centre, as the rational `(2·col − 1) / (2·depth)`.
#[inline]
fn slope(depth: i64, col: i64) -> Frac {
    Frac::new(2 * col - 1, 2 * depth)
}

/// Is `col` within the row's `[start, end]` slope window at this depth? Used to
/// keep visibility symmetric (a cell is lit only if its centre lies inside the
/// scanned wedge).
#[inline]
fn is_symmetric(depth: i64, col: i64, start: Frac, end: Frac) -> bool {
    col * start.den >= start.num * depth && col * end.den <= end.num * depth
}

/// One scan row: a depth (distance from the origin along the quadrant axis) and
/// the slope window `[start, end]` still in shadow-free view.
#[derive(Clone, Copy)]
struct Row {
    depth: i64,
    start: Frac,
    end: Frac,
}

/// Per-quadrant scan context bundling the origin, radius and the caller's
/// callbacks. Held by reference so the recursion shares one borrow.
struct Quadrant<'a, O, V> {
    ox: i32,
    oy: i32,
    radius: i64,
    /// 0 = north, 1 = east, 2 = south, 3 = west.
    index: u8,
    is_opaque: &'a mut O,
    mark_visible: &'a mut V,
}

impl<O, V> Quadrant<'_, O, V>
where
    O: FnMut(i32, i32) -> bool,
    V: FnMut(i32, i32),
{
    /// Map quadrant-local `(depth, col)` to absolute map coordinates.
    #[inline]
    fn transform(&self, depth: i64, col: i64) -> (i32, i32) {
        let d = depth as i32;
        let c = col as i32;
        match self.index {
            0 => (self.ox + c, self.oy - d),
            1 => (self.ox + d, self.oy + c),
            2 => (self.ox + c, self.oy + d),
            _ => (self.ox - d, self.oy + c),
        }
    }

    #[inline]
    fn is_wall(&mut self, depth: i64, col: i64) -> bool {
        let (x, y) = self.transform(depth, col);
        (self.is_opaque)(x, y)
    }

    #[inline]
    fn reveal(&mut self, depth: i64, col: i64) {
        let (x, y) = self.transform(depth, col);
        let dx = (x - self.ox) as i64;
        let dy = (y - self.oy) as i64;
        if dx * dx + dy * dy <= self.radius * self.radius {
            (self.mark_visible)(x, y);
        }
    }

    fn scan(&mut self, row: Row) {
        if row.depth > self.radius {
            return;
        }
        let min_col = round_ties_up(Frac::new(row.start.num * row.depth, row.start.den));
        let max_col = round_ties_down(Frac::new(row.end.num * row.depth, row.end.den));

        // `start` tightens as we cross wall→floor boundaries within the row.
        let mut start = row.start;
        let mut prev_wall: Option<bool> = None;
        let mut col = min_col;
        while col <= max_col {
            let wall = self.is_wall(row.depth, col);
            // A wall is always shown; a floor only if its centre is in the wedge.
            if wall || is_symmetric(row.depth, col, start, row.end) {
                self.reveal(row.depth, col);
            }
            if let Some(prev_was_wall) = prev_wall {
                if prev_was_wall && !wall {
                    // Leaving a wall: the next row's view starts at this slope.
                    start = slope(row.depth, col);
                } else if !prev_was_wall && wall {
                    // Hitting a wall: recurse the deeper row, capped at this slope.
                    self.scan(Row {
                        depth: row.depth + 1,
                        start,
                        end: slope(row.depth, col),
                    });
                }
            }
            prev_wall = Some(wall);
            col += 1;
        }
        // Trailing floor: the whole remaining wedge continues one row deeper.
        if prev_wall == Some(false) {
            self.scan(Row {
                depth: row.depth + 1,
                start,
                end: row.end,
            });
        }
    }
}

/// Compute the field of view from `origin` out to a Euclidean `radius`.
///
/// - `is_opaque(x, y)` — does the cell at `(x, y)` block sight? Out-of-bounds
///   cells are the caller's responsibility; returning `true` for them keeps FOV
///   from leaking past the map edge.
/// - `mark_visible(x, y)` — invoked for each visible cell, including `origin`.
///   May fire more than once per cell; treat it idempotently.
///
/// A `radius` of 0 (or less) reveals only the origin.
pub fn compute_fov<O, V>(origin: (i32, i32), radius: i32, mut is_opaque: O, mut mark_visible: V)
where
    O: FnMut(i32, i32) -> bool,
    V: FnMut(i32, i32),
{
    mark_visible(origin.0, origin.1);
    if radius <= 0 {
        return;
    }
    for index in 0..4u8 {
        let mut quadrant = Quadrant {
            ox: origin.0,
            oy: origin.1,
            radius: radius as i64,
            index,
            is_opaque: &mut is_opaque,
            mark_visible: &mut mark_visible,
        };
        quadrant.scan(Row {
            depth: 1,
            start: Frac::new(-1, 1),
            end: Frac::new(1, 1),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// A small opaque-cell grid; out-of-bounds counts as opaque (a wall).
    struct Grid {
        w: i32,
        h: i32,
        opaque: HashSet<(i32, i32)>,
    }

    impl Grid {
        fn new(w: i32, h: i32) -> Grid {
            Grid {
                w,
                h,
                opaque: HashSet::new(),
            }
        }

        fn wall(&mut self, x: i32, y: i32) {
            self.opaque.insert((x, y));
        }

        /// Visible cells from `origin` within `radius`.
        fn fov(&self, origin: (i32, i32), radius: i32) -> HashSet<(i32, i32)> {
            let mut seen = HashSet::new();
            compute_fov(
                origin,
                radius,
                |x, y| {
                    x < 0 || y < 0 || x >= self.w || y >= self.h || self.opaque.contains(&(x, y))
                },
                |x, y| {
                    if x >= 0 && y >= 0 && x < self.w && y < self.h {
                        seen.insert((x, y));
                    }
                },
            );
            seen
        }
    }

    #[test]
    fn test_radius_zero_sees_only_origin() {
        let grid = Grid::new(11, 11);
        let seen = grid.fov((5, 5), 0);
        assert_eq!(seen, HashSet::from([(5, 5)]));
    }

    #[test]
    fn test_origin_always_visible() {
        let mut grid = Grid::new(11, 11);
        // Even boxed in by walls, you can see yourself (and your own walls).
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            grid.wall(5 + dx, 5 + dy);
        }
        let seen = grid.fov((5, 5), 6);
        assert!(seen.contains(&(5, 5)));
    }

    #[test]
    fn test_open_field_reveals_every_cell_in_radius() {
        let grid = Grid::new(21, 21);
        let origin = (10, 10);
        let radius = 6;
        let seen = grid.fov(origin, radius);
        for y in 0..21 {
            for x in 0..21 {
                let dx = (x - origin.0) as i64;
                let dy = (y - origin.1) as i64;
                let inside = dx * dx + dy * dy <= (radius as i64) * (radius as i64);
                assert_eq!(
                    seen.contains(&(x, y)),
                    inside,
                    "cell ({x},{y}) visibility wrong (inside={inside})"
                );
            }
        }
    }

    #[test]
    fn test_wall_casts_a_shadow_behind_it() {
        let mut grid = Grid::new(21, 21);
        let origin = (10, 10);
        // A wall directly east of the origin.
        grid.wall(11, 10);
        let seen = grid.fov(origin, 8);
        // The wall itself is visible...
        assert!(seen.contains(&(11, 10)));
        // ...but the cells in its shadow (further east on the same row) are not.
        assert!(
            !seen.contains(&(12, 10)),
            "cell directly behind wall must be hidden"
        );
        assert!(
            !seen.contains(&(15, 10)),
            "far cell behind wall must be hidden"
        );
        // A cell off the shadow axis is still visible.
        assert!(seen.contains(&(11, 12)));
    }

    #[test]
    fn test_visibility_is_symmetric() {
        // The defining property: A sees B  <=>  B sees A. Verify it across a map
        // with a couple of pillars by re-running FOV from every cell A sees.
        let mut grid = Grid::new(15, 15);
        grid.wall(7, 6);
        grid.wall(8, 8);
        grid.wall(5, 9);
        let origin = (7, 7);
        let radius = 6;
        let from_origin = grid.fov(origin, radius);
        for &b in &from_origin {
            if b == origin {
                continue;
            }
            // B must not be a wall to "see" from it, but symmetry of visibility
            // is asserted for transparent B (the standard guarantee).
            if grid.opaque.contains(&b) {
                continue;
            }
            let from_b = grid.fov(b, radius);
            assert!(
                from_b.contains(&origin),
                "asymmetry: origin sees {b:?} but {b:?} does not see origin"
            );
        }
    }

    #[test]
    fn test_fov_is_deterministic() {
        let mut grid = Grid::new(15, 15);
        grid.wall(8, 7);
        grid.wall(6, 9);
        let a = grid.fov((7, 7), 7);
        let b = grid.fov((7, 7), 7);
        assert_eq!(a, b);
    }
}
