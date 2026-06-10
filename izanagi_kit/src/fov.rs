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

/// Per-quadrant scan context. The callback receives `(x, y, dist_sq)` so that
/// both [`compute_fov`] (which ignores `dist_sq`) and [`compute_fov_dist`]
/// (which passes it through) can share the same scan logic.
struct Quadrant<'a, O, V> {
    ox: i32,
    oy: i32,
    radius: i64,
    /// 0 = north, 1 = east, 2 = south, 3 = west.
    index: u8,
    is_opaque: &'a mut O,
    /// Called for each visible cell as `callback(x, y, dist_sq)`.
    callback: &'a mut V,
}

impl<O, V> Quadrant<'_, O, V>
where
    O: FnMut(i32, i32) -> bool,
    V: FnMut(i32, i32, i64),
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
        let dist_sq = dx * dx + dy * dy;
        if dist_sq <= self.radius * self.radius {
            (self.callback)(x, y, dist_sq);
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
    let mut cb = |x: i32, y: i32, _dsq: i64| mark_visible(x, y);
    for index in 0..4u8 {
        let mut quadrant = Quadrant {
            ox: origin.0,
            oy: origin.1,
            radius: radius as i64,
            index,
            is_opaque: &mut is_opaque,
            callback: &mut cb,
        };
        quadrant.scan(Row {
            depth: 1,
            start: Frac::new(-1, 1),
            end: Frac::new(1, 1),
        });
    }
}

/// Compute the field of view, reporting the squared Euclidean distance from
/// the origin to each visible cell alongside its coordinates.
///
/// Like [`compute_fov`] but `mark_visible(x, y, dist_sq)` receives an extra
/// `dist_sq: i32` — the integer squared distance from `origin` to `(x, y)`.
/// The origin itself is reported with `dist_sq == 0`.
///
/// Use `dist_sq` to implement light-falloff, range checks, or
/// distance-graded fog-of-war without a second sqrt pass:
///
/// ```
/// # use izanagi_kit::fov::compute_fov_dist;
/// let mut lit: Vec<(i32, i32, i32)> = Vec::new();
/// compute_fov_dist(
///     (10, 10), 8,
///     |_x, _y| false,                        // open field
///     |x, y, d| lit.push((x, y, d)),
/// );
/// // Cells closer to origin have smaller d (light falloff).
/// assert!(lit.iter().any(|&(_, _, d)| d == 0)); // origin
/// ```
///
/// Symmetry, determinism, and radius semantics are identical to [`compute_fov`].
pub fn compute_fov_dist<O, V>(
    origin: (i32, i32),
    radius: i32,
    mut is_opaque: O,
    mut mark_visible: V,
) where
    O: FnMut(i32, i32) -> bool,
    V: FnMut(i32, i32, i32),
{
    mark_visible(origin.0, origin.1, 0);
    if radius <= 0 {
        return;
    }
    let mut cb = |x: i32, y: i32, dist_sq: i64| {
        mark_visible(x, y, dist_sq.min(i32::MAX as i64) as i32);
    };
    for index in 0..4u8 {
        let mut quadrant = Quadrant {
            ox: origin.0,
            oy: origin.1,
            radius: radius as i64,
            index,
            is_opaque: &mut is_opaque,
            callback: &mut cb,
        };
        quadrant.scan(Row {
            depth: 1,
            start: Frac::new(-1, 1),
            end: Frac::new(1, 1),
        });
    }
}

/// Collect all visible cells from `origin` within `radius` into a `Vec`.
///
/// Convenience wrapper around [`compute_fov`] for callers that need a concrete
/// collection rather than a callback. Equivalent to bracket-lib's
/// `field_of_view_set` but returning a `Vec` (the caller can deduplicate
/// into a `HashSet` if needed).
///
/// `is_opaque(x, y)` follows the same contract as [`compute_fov`].
pub fn fov_to_vec<O>(origin: (i32, i32), radius: i32, is_opaque: O) -> Vec<(i32, i32)>
where
    O: FnMut(i32, i32) -> bool,
{
    let mut visible = Vec::new();
    compute_fov(origin, radius, is_opaque, |x, y| visible.push((x, y)));
    visible
}

/// Collect visible cells filtered by a squared-distance limit.
///
/// Like [`fov_to_vec`] but only includes cells where `dist_sq <= max_dist_sq`.
/// Use this for light-falloff queries ("torch radius 4 → max_dist_sq 16")
/// without a separate filtering pass. `radius` is still the outer shadow-cast
/// limit; `max_dist_sq` is a tighter inner clamp.
///
/// The result may contain duplicates (axis/diagonal cells can fire twice from
/// different quadrants); deduplicate with a `HashSet` if uniqueness matters.
pub fn fov_to_vec_dist<O>(
    origin: (i32, i32),
    radius: i32,
    max_dist_sq: i32,
    is_opaque: O,
) -> Vec<(i32, i32)>
where
    O: FnMut(i32, i32) -> bool,
{
    let mut visible = Vec::new();
    compute_fov_dist(origin, radius, is_opaque, |x, y, d| {
        if d <= max_dist_sq {
            visible.push((x, y));
        }
    });
    visible
}

/// Count visible cells within `radius` without allocating a `Vec`.
/// Equivalent to `fov_to_vec(origin, radius, is_opaque).len()` but skips
/// the intermediate allocation — use for broad-phase budget checks and
/// lighting-budget queries where only the count matters.
pub fn fov_count<O: FnMut(i32, i32) -> bool>(
    origin: (i32, i32),
    radius: i32,
    is_opaque: O,
) -> usize {
    let mut count = 0usize;
    compute_fov(origin, radius, is_opaque, |_, _| count += 1);
    count
}

/// All visible cells at **exactly** `radius` Chebyshev distance from `origin`.
///
/// Equivalent to `fov_to_vec(origin, radius, is_opaque)` filtered to cells
/// where `max(|dx|, |dy|) == radius`. Useful for AoE ring effects that should
/// hit only the outer edge of a blast without touching the interior cells.
pub fn fov_ring<O: FnMut(i32, i32) -> bool>(
    origin: (i32, i32),
    radius: i32,
    is_opaque: O,
) -> Vec<(i32, i32)> {
    fov_to_vec(origin, radius, is_opaque)
        .into_iter()
        .filter(|&(x, y)| {
            let dx = (x - origin.0).abs();
            let dy = (y - origin.1).abs();
            dx.max(dy) == radius
        })
        .collect()
}

/// Single-cell visibility query: can `origin` see `target` within `radius`?
///
/// Equivalent to `fov_to_vec(origin, radius, is_opaque).contains(&target)` but
/// avoids allocating the full visible set. Internally runs the same symmetric
/// shadowcasting as [`compute_fov`], so the result is identical to checking
/// membership in the FOV set — including the symmetry guarantee (A sees B ⟺
/// B sees A).
///
/// Returns `true` when `target` is within `radius` and in line-of-sight of
/// `origin` (or when `origin == target`). Returns `false` when `target` is
/// outside the radius or blocked.
///
/// `is_opaque(x, y)` follows the same contract as [`compute_fov`].
pub fn can_see<O: FnMut(i32, i32) -> bool>(
    origin: (i32, i32),
    target: (i32, i32),
    radius: i32,
    is_opaque: O,
) -> bool {
    let mut found = false;
    let tx = target.0;
    let ty = target.1;
    compute_fov(origin, radius, is_opaque, |x, y| {
        if x == tx && y == ty {
            found = true;
        }
    });
    found
}

/// Visible cells within an **Euclidean** circle of `radius` — a strict disc
/// rather than the Chebyshev square that `fov_to_vec` returns. Filters
/// `fov_to_vec` to cells where `dx² + dy² ≤ radius²`. Returns empty for
/// `radius < 0`. Useful for torch-light and ranged-attack indicators where
/// a round boundary is expected.
pub fn fov_circle<O: FnMut(i32, i32) -> bool>(
    origin: (i32, i32),
    radius: i32,
    is_opaque: O,
) -> Vec<(i32, i32)> {
    if radius < 0 {
        return Vec::new();
    }
    let r2 = (radius as i64) * (radius as i64);
    fov_to_vec(origin, radius, is_opaque)
        .into_iter()
        .filter(|&(x, y)| {
            let dx = (x - origin.0) as i64;
            let dy = (y - origin.1) as i64;
            dx * dx + dy * dy <= r2
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

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

        /// Visible cells with their squared distances.
        fn fov_dist(&self, origin: (i32, i32), radius: i32) -> HashMap<(i32, i32), i32> {
            let mut seen: HashMap<(i32, i32), i32> = HashMap::new();
            compute_fov_dist(
                origin,
                radius,
                |x, y| {
                    x < 0 || y < 0 || x >= self.w || y >= self.h || self.opaque.contains(&(x, y))
                },
                |x, y, d| {
                    if x >= 0 && y >= 0 && x < self.w && y < self.h {
                        seen.insert((x, y), d);
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

    // --- compute_fov_dist tests ---

    #[test]
    fn test_fov_dist_origin_reports_zero() {
        let grid = Grid::new(11, 11);
        let seen = grid.fov_dist((5, 5), 4);
        assert_eq!(seen[&(5, 5)], 0, "origin must have dist_sq == 0");
    }

    #[test]
    fn test_fov_dist_matches_fov_visible_set() {
        let mut grid = Grid::new(15, 15);
        grid.wall(8, 7);
        grid.wall(6, 9);
        let origin = (7, 7);
        let radius = 5;
        let plain: HashSet<(i32, i32)> = grid.fov(origin, radius);
        let dist: HashSet<(i32, i32)> = grid.fov_dist(origin, radius).into_keys().collect();
        assert_eq!(
            plain, dist,
            "compute_fov_dist must report the same cells as compute_fov"
        );
    }

    #[test]
    fn test_fov_dist_values_are_correct() {
        let grid = Grid::new(21, 21);
        let origin = (10, 10);
        let seen = grid.fov_dist(origin, 6);
        // Orthogonal neighbour: dist_sq = 1.
        assert_eq!(seen[&(11, 10)], 1);
        // Diagonal neighbour: dist_sq = 2.
        assert_eq!(seen[&(11, 11)], 2);
        // Cell 3 east: dist_sq = 9.
        assert_eq!(seen[&(13, 10)], 9);
    }

    // --- fov_to_vec tests ---

    #[test]
    fn test_fov_to_vec_includes_origin() {
        let visible = fov_to_vec((5, 5), 3, |_, _| false);
        assert!(visible.contains(&(5, 5)));
    }

    #[test]
    fn test_fov_to_vec_matches_compute_fov_set() {
        let mut grid = Grid::new(15, 15);
        grid.wall(7, 6);
        grid.wall(8, 8);
        let origin = (7, 7);
        let radius = 5;
        let expected: HashSet<(i32, i32)> = grid.fov(origin, radius);
        let from_vec: HashSet<(i32, i32)> = fov_to_vec(origin, radius, |x, y| {
            x < 0 || y < 0 || x >= 15 || y >= 15 || grid.opaque.contains(&(x, y))
        })
        .into_iter()
        .collect();
        assert_eq!(expected, from_vec);
    }

    #[test]
    fn test_fov_to_vec_radius_zero_only_origin() {
        let visible = fov_to_vec((3, 3), 0, |_, _| false);
        assert_eq!(visible, vec![(3, 3)]);
    }

    // --- fov_to_vec_dist tests ---

    #[test]
    fn test_fov_to_vec_dist_zero_max_includes_only_origin() {
        let visible = fov_to_vec_dist((5, 5), 5, 0, |_, _| false);
        assert_eq!(visible, vec![(5, 5)], "only origin has dist_sq == 0");
    }

    #[test]
    fn test_fov_to_vec_dist_large_max_matches_fov_to_vec() {
        let origin = (10, 10);
        let radius = 4;
        let full: HashSet<(i32, i32)> = fov_to_vec(origin, radius, |_, _| false)
            .into_iter()
            .collect();
        let filtered: HashSet<(i32, i32)> =
            fov_to_vec_dist(origin, radius, radius * radius, |_, _| false)
                .into_iter()
                .collect();
        assert_eq!(full, filtered);
    }

    #[test]
    fn test_fov_to_vec_dist_one_excludes_diagonals() {
        let origin = (0, 0);
        let visible: HashSet<(i32, i32)> = fov_to_vec_dist(origin, 10, 1, |_, _| false)
            .into_iter()
            .collect();
        assert!(visible.contains(&(0, 0)), "origin");
        assert!(visible.contains(&(1, 0)), "east");
        assert!(visible.contains(&(-1, 0)), "west");
        assert!(visible.contains(&(0, 1)), "south");
        assert!(visible.contains(&(0, -1)), "north");
        assert!(!visible.contains(&(1, 1)), "diagonal excluded (dist_sq=2)");
    }

    #[test]
    fn test_fov_count_matches_fov_to_vec_len() {
        let origin = (5, 5);
        let radius = 4;
        let expected = fov_to_vec(origin, radius, |_, _| false).len();
        assert_eq!(fov_count(origin, radius, |_, _| false), expected);
    }

    #[test]
    fn test_fov_count_zero_radius_is_one() {
        // Radius 0 means only the origin cell.
        assert_eq!(fov_count((0, 0), 0, |_, _| false), 1);
    }

    #[test]
    fn test_fov_count_fully_blocked_matches_vec() {
        // When every cell is opaque fov_count must still equal fov_to_vec.len()
        // (opaque cells adjacent to the origin are visible as blocking walls).
        let origin = (5, 5);
        let radius = 5;
        let expected = fov_to_vec(origin, radius, |_, _| true).len();
        let count = fov_count(origin, radius, |_, _| true);
        assert_eq!(count, expected);
    }

    #[test]
    fn test_fov_ring_all_at_chebyshev_radius() {
        let origin = (10, 10);
        let radius = 3;
        let ring = fov_ring(origin, radius, |_, _| false);
        for (x, y) in &ring {
            let dx = (x - origin.0).abs();
            let dy = (y - origin.1).abs();
            assert_eq!(dx.max(dy), radius, "({},{}) not on ring", x, y);
        }
    }

    #[test]
    fn test_fov_ring_zero_radius_is_origin() {
        let ring = fov_ring((5, 5), 0, |_, _| false);
        assert_eq!(ring, vec![(5, 5)]);
    }

    #[test]
    fn test_fov_ring_subset_of_fov_to_vec() {
        let origin = (8, 8);
        let radius = 4;
        let all: std::collections::HashSet<_> = fov_to_vec(origin, radius, |_, _| false)
            .into_iter()
            .collect();
        for pt in fov_ring(origin, radius, |_, _| false) {
            assert!(all.contains(&pt), "{:?} not in fov_to_vec", pt);
        }
    }

    #[test]
    fn test_fov_circle_includes_origin() {
        let cells = fov_circle((0, 0), 5, |_, _| false);
        assert!(cells.contains(&(0, 0)), "origin must be visible");
    }

    #[test]
    fn test_fov_circle_subset_of_square_fov() {
        let origin = (10, 10);
        let radius = 4;
        let square: std::collections::HashSet<_> = fov_to_vec(origin, radius, |_, _| false)
            .into_iter()
            .collect();
        for &(x, y) in &fov_circle(origin, radius, |_, _| false) {
            assert!(square.contains(&(x, y)));
            let dx = (x - origin.0) as i64;
            let dy = (y - origin.1) as i64;
            assert!(dx * dx + dy * dy <= (radius as i64) * (radius as i64));
        }
    }

    #[test]
    fn test_fov_circle_negative_radius_empty() {
        assert!(fov_circle((5, 5), -1, |_, _| false).is_empty());
    }

    // --- can_see tests ---

    #[test]
    fn test_can_see_open_field_returns_true() {
        let origin = (5, 5);
        let target = (8, 5);
        assert!(can_see(origin, target, 10, |_, _| false));
    }

    #[test]
    fn test_can_see_blocked_by_wall_returns_false() {
        // A wall column directly between origin and target on the same row.
        let origin = (5, 5);
        let target = (8, 5);
        // Wall at (7, 5) blocks line of sight.
        assert!(!can_see(origin, target, 10, |x, y| x == 7 && y == 5));
    }

    #[test]
    fn test_can_see_out_of_radius_returns_false() {
        let origin = (0, 0);
        let target = (10, 0);
        // Radius 5 — target is 10 cells away, beyond radius.
        assert!(!can_see(origin, target, 5, |_, _| false));
    }
}
