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

/// Number of cells [`line`] returns from `a` to `b`, computed without
/// allocating. Equals the Chebyshev distance plus one — each Bresenham step
/// advances by exactly one king-move, so the cell count is `max(|dx|,|dy|)+1`.
/// `a == b` yields `1`. Useful for sizing buffers or range checks before
/// tracing the full ray.
#[inline]
pub fn line_len(a: (i32, i32), b: (i32, i32)) -> usize {
    let dx = (a.0 as i64 - b.0 as i64).abs();
    let dy = (a.1 as i64 - b.1 as i64).abs();
    dx.max(dy) as usize + 1
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

/// Trace a bolt/beam from `origin` toward `target` along the Bresenham [`line`],
/// returning the ordered cells it travels through up to **and including** the
/// first blocked cell (the impact point). If nothing blocks the path, the full
/// line through `target` is returned, so the last element is always *where the
/// bolt landed* — either `target` or the obstruction that stopped it.
///
/// The `origin` cell is the shooter's own square: it is always included and
/// **never tested** for blocking (a bolt is not absorbed by the tile you stand
/// on). Every cell after it is a candidate impact — the first one for which
/// `is_blocked` returns `true` ends the trace. `origin == target` yields just
/// `[origin]`.
///
/// This is the projectile/bolt primitive (arrows, lightning bolts, thrown
/// items): interior cells are the pass-through path (apply beam damage there),
/// and `.last()` is the cell that takes the hit. Deterministic and integer-only
/// like the rest of the module.
pub fn ray_cast<F>(origin: (i32, i32), target: (i32, i32), mut is_blocked: F) -> Vec<(i32, i32)>
where
    F: FnMut(i32, i32) -> bool,
{
    let cells = line(origin, target);
    let mut out = Vec::with_capacity(cells.len());
    for (i, &(x, y)) in cells.iter().enumerate() {
        out.push((x, y));
        // Index 0 is the origin — the shooter's own cell never stops the bolt.
        if i > 0 && is_blocked(x, y) {
            break;
        }
    }
    out
}

/// Where a bolt from `origin` toward `target` is stopped, if it does not reach
/// `target`. Returns `Some(impact)` when an obstruction halts the bolt **before**
/// `target` (a blocked shot), and `None` when the bolt reaches `target`
/// unobstructed — whether or not `target` itself is blocked (aiming at a wall is
/// still a clear shot *to* that wall).
///
/// Companion to [`ray_cast`] sharing its exact blocking convention (the origin
/// never blocks); use this for the common "is my shot clear, and if not, what's
/// in the way?" targeting query without inspecting the whole path.
pub fn ray_blocked_at<F>(origin: (i32, i32), target: (i32, i32), is_blocked: F) -> Option<(i32, i32)>
where
    F: FnMut(i32, i32) -> bool,
{
    let path = ray_cast(origin, target, is_blocked);
    match path.last() {
        Some(&last) if last != target => Some(last),
        _ => None,
    }
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

/// Test whether the point `(px, py)` lies within the axis-aligned rectangle
/// `[x, x+w) × [y, y+h)`. Returns `false` for non-positive `w` or `h`.
///
/// Equivalent to the point-in-AABB check but without constructing an `Aabb`
/// struct — use this for quick inline predicate lambdas:
/// `rect_contains(rx, ry, rw, rh, px, py)`.
#[inline]
pub fn rect_contains(x: i32, y: i32, w: i32, h: i32, px: i32, py: i32) -> bool {
    w > 0 && h > 0 && px >= x && px < x + w && py >= y && py < y + h
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

/// Cells on the perimeter (outer border) of the rectangle `[x, x+w) × [y, y+h)`,
/// in row-major order `(y then x)`. Returns an empty `Vec` for non-positive `w`
/// or `h`. For `w == 1` or `h == 1` every cell is a border cell, so this
/// degenerates to [`rect`].
///
/// Use this for dungeon-room outlines, bordered UI panels, and rectangular
/// blast-edge AoE indicators without filling the interior.
pub fn rect_perimeter(x: i32, y: i32, w: i32, h: i32) -> Vec<(i32, i32)> {
    if w <= 0 || h <= 0 {
        return Vec::new();
    }
    if w == 1 || h == 1 {
        return rect(x, y, w, h);
    }
    let mut pts = Vec::with_capacity(2 * (w + h - 2) as usize);
    for dy in 0..h {
        for dx in 0..w {
            if dy == 0 || dy == h - 1 || dx == 0 || dx == w - 1 {
                pts.push((x + dx, y + dy));
            }
        }
    }
    pts
}

/// Cells at exactly Manhattan distance `r` from `(cx, cy)` — the "diamond
/// ring" used in 4-directional movement range queries and targeted blast
/// outlines. Returns empty for `r < 0`; just `(cx, cy)` for `r == 0`.
/// The `4·r` unique cells are returned in ascending `(x, y)` order.
/// All cells at exactly **Chebyshev** ("king-move") distance `r` from `(cx, cy)`.
///
/// Returns empty for `r < 0`, just the centre for `r == 0`, and the four sides
/// of the square perimeter at radius `r` (i.e. `rect_perimeter(cx-r, cy-r,
/// 2r+1, 2r+1)`) otherwise. Cell count is `8r` for `r ≥ 1`.
///
/// Complements `diamond` (Manhattan ring) and `circle` (Euclidean outline).
/// Useful for "all cells an 8-way actor can reach in exactly r steps" and
/// scrolling collision rings.
pub fn chebyshev_ring(cx: i32, cy: i32, r: i32) -> Vec<(i32, i32)> {
    if r < 0 {
        return Vec::new();
    }
    if r == 0 {
        return vec![(cx, cy)];
    }
    rect_perimeter(cx - r, cy - r, 2 * r + 1, 2 * r + 1)
}

pub fn diamond(cx: i32, cy: i32, r: i32) -> Vec<(i32, i32)> {
    if r < 0 {
        return Vec::new();
    }
    if r == 0 {
        return vec![(cx, cy)];
    }
    use std::collections::BTreeSet;
    let mut pts: BTreeSet<(i32, i32)> = BTreeSet::new();
    for dy in -r..=r {
        let dx = r - dy.abs();
        pts.insert((cx - dx, cy + dy));
        pts.insert((cx + dx, cy + dy));
    }
    pts.into_iter().collect()
}

/// The cells of a 90° **cone** (a breath-weapon / cone-spell shape) emanating
/// from `origin` along `facing`, out to Euclidean `range`. `facing` is any
/// non-zero direction vector — typically one of the eight compass steps such as
/// `(1, 0)` (east) or `(1, 1)` (south-east), but any integer axis works.
///
/// A cell offset `o` from the origin is included when all hold:
/// - it is **in front** of the origin (`o · facing > 0`);
/// - it lies within **±45°** of the axis, tested exactly with integers as
///   `2·(o · facing)² ≥ |o|²·|facing|²` (no trigonometry, no float — so the
///   shape is bit-identical on every target, like the rest of the kit);
/// - it is within range (`|o|² ≤ range²`).
///
/// The `origin` itself is **excluded** (the breather's own tile is not part of
/// the blast). Returns an empty `Vec` for `range < 0` or a zero `facing`. Cells
/// come back in ascending `(y, x)` order, without duplicates.
///
/// This is a pure geometric shape, matching [`circle`]/[`diamond`]; for a cone
/// that respects walls, filter the result with
/// [`line_of_sight`]`(origin, cell, is_opaque)` (or [`ray_cast`]) at the call
/// site, the same way the other area shapes compose with obstruction checks.
pub fn cone(origin: (i32, i32), facing: (i32, i32), range: i32) -> Vec<(i32, i32)> {
    let (fx, fy) = facing;
    if range < 0 || (fx == 0 && fy == 0) {
        return Vec::new();
    }
    let (ox, oy) = origin;
    let f_mag_sq = (fx as i64 * fx as i64) + (fy as i64 * fy as i64);
    let range_sq = range as i64 * range as i64;
    let mut cells = Vec::new();
    for dy in -range..=range {
        for dx in -range..=range {
            if dx == 0 && dy == 0 {
                continue; // origin is excluded
            }
            let dist_sq = (dx as i64 * dx as i64) + (dy as i64 * dy as i64);
            if dist_sq > range_sq {
                continue; // outside the Euclidean range
            }
            let dot = (dx as i64 * fx as i64) + (dy as i64 * fy as i64);
            if dot <= 0 {
                continue; // behind or perpendicular to the facing
            }
            // angle(o, facing) ≤ 45°  ⟺  cosθ ≥ √2/2  ⟺  2·dot² ≥ |o|²·|f|²
            if 2 * dot * dot >= dist_sq * f_mag_sq {
                cells.push((ox + dx, oy + dy));
            }
        }
    }
    cells
}

/// A wall-aware [`cone`]: the cone cells from `origin` that have a clear
/// line of sight from `origin`, given an `is_opaque` predicate. This is the
/// breath-weapon / cone-spell footprint as it actually lands on a map — the
/// blast reaches and **includes** the wall cells it strikes (endpoints never
/// block, per [`line_of_sight`]), but cells shadowed *behind* a wall are
/// culled.
///
/// The result is always a subset of `cone(origin, facing, range)`, in the same
/// ascending `(y, x)` order. Returns empty for `range < 0` or a zero `facing`.
/// Uses the kit's single-ray [`line_of_sight`] model (cheap, the shooter's
/// viewpoint is authoritative); for a strict circular blast prefer the
/// shadowcasting [`crate::fov::fov_circle`], which models occlusion symmetrically.
pub fn cone_visible<F>(
    origin: (i32, i32),
    facing: (i32, i32),
    range: i32,
    mut is_opaque: F,
) -> Vec<(i32, i32)>
where
    F: FnMut(i32, i32) -> bool,
{
    cone(origin, facing, range)
        .into_iter()
        .filter(|&cell| line_of_sight(origin, cell, &mut is_opaque))
        .collect()
}

/// Centre cell of the rectangle `[x, x+w) × [y, y+h)` using floor division.
///
/// Returns `(x + w/2, y + h/2)` — identical truncation bias to
/// `Aabb::center()`. For zero-size dimensions the result equals the origin
/// corner. The standalone complement to `midpoint` for "where is the middle
/// of this room?" spawn placement and camera targeting.
#[inline]
pub fn rect_center(x: i32, y: i32, w: i32, h: i32) -> (i32, i32) {
    (x + w / 2, y + h / 2)
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
        // `dx`/`dy` reach 2^32 for extreme coords, so `dx*dx` (~1.8e19) overflows
        // i64. Saturating ops cap the sum of squares at i64::MAX; the final
        // `.min(i32::MAX)` then yields the documented saturated result.
        let sum_sq = dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy));
        let v = match self {
            Distance::Manhattan => dx.saturating_add(dy),
            Distance::Chebyshev => dx.max(dy),
            Distance::EuclideanSquared => sum_sq,
            Distance::Euclidean => isqrt(sum_sq),
        };
        v.min(i32::MAX as i64) as i32
    }
}

/// Integer midpoint of two grid cells. For cells separated by an odd number of
/// units the result rounds toward `a` (floor of the true average). Safe for any
/// `i32` inputs — avoids overflow by computing `a + (b - a) / 2`.
///
/// Useful for placing a marker at the middle of a segment: `midpoint(from, to)`
/// returns the cell that Bresenham `line(from, to)` visits closest to halfway.
#[inline]
pub fn midpoint(a: (i32, i32), b: (i32, i32)) -> (i32, i32) {
    (a.0 + (b.0 - a.0) / 2, a.1 + (b.1 - a.1) / 2)
}

/// Unit direction vector from `from` toward `to` — each component is `−1`, `0`,
/// or `+1` (the sign of the difference). Returns `(0, 0)` when `from == to`.
/// The cheapest "which way should I face?" primitive for enemy AI and melee
/// indicators. Does **not** check passability; use [`crate::pathfinding::step_toward`]
/// for passability-aware movement.
#[inline]
pub fn vec_toward(from: (i32, i32), to: (i32, i32)) -> (i32, i32) {
    ((to.0 - from.0).signum(), (to.1 - from.1).signum())
}

/// Slide an entity at `from` in direction `dir` by up to `distance` cells,
/// stopping **before** the first blocked cell. Returns the cell it ends on.
///
/// This is the forced-displacement primitive — knockback from an explosion,
/// a shield bash, telekinesis, a conveyor. `dir` is direction-only: it is
/// normalized to an 8-way step via `signum`, so its magnitude is ignored
/// (`(3, 0)` pushes east exactly like `(1, 0)`); pair it with
/// [`vec_toward`]`(source, target)` to knock a target directly away from a
/// blast source. The `from` cell is never tested (the entity already stands
/// there); each prospective cell is checked, and the entity halts on the last
/// open cell rather than entering a wall.
///
/// Returns `from` unchanged when `dir` is zero or `distance <= 0`. Integer-only
/// and deterministic. To recover the path travelled use [`line`]`(from, landing)`,
/// and the cells actually moved is `Distance::Chebyshev.between(from, landing)`.
pub fn knockback<F>(from: (i32, i32), dir: (i32, i32), distance: i32, mut is_blocked: F) -> (i32, i32)
where
    F: FnMut(i32, i32) -> bool,
{
    let step = (dir.0.signum(), dir.1.signum());
    if step == (0, 0) || distance <= 0 {
        return from;
    }
    let mut pos = from;
    for _ in 0..distance {
        let next = (pos.0 + step.0, pos.1 + step.1);
        if is_blocked(next.0, next.1) {
            break;
        }
        pos = next;
    }
    pos
}

/// Reflect `point` through `center`: `(2·cx − px, 2·cy − py)`. Useful for
/// symmetric dungeon layouts, mirror-image room templates, and paired-entity
/// positioning (e.g. place a second torch on the opposite side of a door).
/// Uses saturating arithmetic so extreme coordinates never wrap.
#[inline]
pub fn reflect_point(point: (i32, i32), center: (i32, i32)) -> (i32, i32) {
    (
        (2 * center.0 as i64 - point.0 as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32,
        (2 * center.1 as i64 - point.1 as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32,
    )
}

/// Manhattan distance between `a` and `b`: `|dx| + |dy|`. Shorthand for
/// `Distance::Manhattan.between(a, b)` — avoids the enum at call sites where
/// only the taxi-cab metric is needed.
#[inline]
pub fn manhattan_distance(a: (i32, i32), b: (i32, i32)) -> i32 {
    (b.0 - a.0).abs() + (b.1 - a.1).abs()
}

/// Chebyshev distance between `a` and `b`: `max(|dx|, |dy|)`. Shorthand for
/// `Distance::Chebyshev.between(a, b)` — the "king moves" metric used for
/// 8-directional range checks and adjacency tests.
#[inline]
pub fn chebyshev_distance(a: (i32, i32), b: (i32, i32)) -> i32 {
    (b.0 - a.0).abs().max((b.1 - a.1).abs())
}

/// Rotate point `(x, y)` 90° clockwise around the origin in **screen
/// coordinates** (y increases downward). The sequence Right→Down→Left→Up
/// cycles as `(1,0)→(0,1)→(-1,0)→(0,-1)`.
///
/// Useful for rotating room templates, projectile direction tables, and
/// symmetry generation without floating-point math.
#[inline]
pub fn rotate_90_cw(x: i32, y: i32) -> (i32, i32) {
    (-y, x)
}

/// Rotate point `(x, y)` 90° counter-clockwise around the origin in **screen
/// coordinates** (y increases downward). Inverse of [`rotate_90_cw`].
#[inline]
pub fn rotate_90_ccw(x: i32, y: i32) -> (i32, i32) {
    (y, -x)
}

/// Floor of the square root of a non-negative `i64`, computed with integer
/// arithmetic only (Newton's method). `isqrt(n)² <= n < (isqrt(n)+1)²`.
fn isqrt(n: i64) -> i64 {
    if n < 2 {
        return n.max(0);
    }
    // Bit-by-bit integer square root using only add/shift/compare. Overflow-safe
    // for any non-negative `i64`, including `i64::MAX` — Newton's `x + n/x` would
    // overflow there (initial `x == n`). Returns the floor of the real root,
    // identical to the previous implementation for all in-range inputs.
    let n = n as u64;
    let mut rem = n;
    let mut root: u64 = 0;
    let mut bit: u64 = 1 << 62;
    while bit > rem {
        bit >>= 2;
    }
    while bit != 0 {
        if rem >= root + bit {
            rem -= root + bit;
            root = (root >> 1) + bit;
        } else {
            root >>= 1;
        }
        bit >>= 2;
    }
    root as i64
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

    // ── rect_perimeter ────────────────────────────────────────────────────────

    #[test]
    fn test_rect_perimeter_empty_for_zero_dimension() {
        assert!(rect_perimeter(0, 0, 0, 5).is_empty());
        assert!(rect_perimeter(0, 0, 5, 0).is_empty());
        assert!(rect_perimeter(0, 0, -1, 3).is_empty());
    }

    #[test]
    fn test_rect_perimeter_1x1_is_single_cell() {
        assert_eq!(rect_perimeter(3, 4, 1, 1), vec![(3, 4)]);
    }

    #[test]
    fn test_rect_perimeter_1xh_is_full_column() {
        let p = rect_perimeter(0, 0, 1, 3);
        assert_eq!(p, vec![(0, 0), (0, 1), (0, 2)]);
    }

    #[test]
    fn test_rect_perimeter_count_for_3x3() {
        // 3×3 border: 4 corners + 4 edges of length 1 = 8 cells
        let p = rect_perimeter(0, 0, 3, 3);
        assert_eq!(p.len(), 8);
        assert!(p.contains(&(0, 0)));
        assert!(p.contains(&(2, 2)));
        assert!(!p.contains(&(1, 1))); // interior excluded
    }

    #[test]
    fn test_rect_perimeter_all_cells_on_border() {
        // Every returned point must be on the border.
        let (x, y, w, h) = (5, 3, 6, 4);
        for (px, py) in rect_perimeter(x, y, w, h) {
            let on_border = px == x || px == x + w - 1 || py == y || py == y + h - 1;
            assert!(on_border, "({px},{py}) is not on the border");
        }
    }

    #[test]
    fn test_rect_perimeter_count_formula() {
        // Perimeter cell count = 2*(w+h-2) for w,h >= 2.
        for w in 2..=10i32 {
            for h in 2..=10i32 {
                let expected = 2 * (w + h - 2) as usize;
                assert_eq!(rect_perimeter(0, 0, w, h).len(), expected, "w={w} h={h}");
            }
        }
    }

    // ── diamond ───────────────────────────────────────────────────────────────

    #[test]
    fn test_diamond_negative_r_is_empty() {
        assert!(diamond(0, 0, -1).is_empty());
    }

    #[test]
    fn test_diamond_r0_is_centre() {
        assert_eq!(diamond(5, 5, 0), vec![(5, 5)]);
    }

    #[test]
    fn test_diamond_r1_is_four_cardinals() {
        let d = diamond(0, 0, 1);
        assert_eq!(d.len(), 4);
        assert!(d.contains(&(-1, 0)));
        assert!(d.contains(&(1, 0)));
        assert!(d.contains(&(0, -1)));
        assert!(d.contains(&(0, 1)));
        assert!(!d.contains(&(0, 0))); // centre excluded
    }

    #[test]
    fn test_diamond_count_is_4r() {
        for r in 1..=8i32 {
            assert_eq!(diamond(0, 0, r).len(), (4 * r) as usize, "r={r}");
        }
    }

    #[test]
    fn test_diamond_all_cells_at_exact_manhattan_distance() {
        let r = 5;
        for (x, y) in diamond(3, 7, r) {
            assert_eq!((x - 3).abs() + (y - 7).abs(), r, "({x},{y}) wrong distance");
        }
    }

    // --- rect_contains ---

    #[test]
    fn test_rect_contains_point_inside() {
        assert!(rect_contains(0, 0, 5, 5, 2, 3));
    }

    #[test]
    fn test_rect_contains_point_on_left_top_boundary() {
        assert!(rect_contains(0, 0, 5, 5, 0, 0));
    }

    #[test]
    fn test_rect_contains_point_on_exclusive_right_bottom() {
        assert!(!rect_contains(0, 0, 5, 5, 5, 5)); // right/bottom are exclusive
        assert!(!rect_contains(0, 0, 5, 5, 5, 0));
        assert!(!rect_contains(0, 0, 5, 5, 0, 5));
    }

    #[test]
    fn test_rect_contains_degenerate_zero_size() {
        assert!(!rect_contains(1, 1, 0, 5, 1, 1));
        assert!(!rect_contains(1, 1, 5, 0, 1, 1));
    }

    // --- line_len ---

    #[test]
    fn test_line_len_same_point_is_one() {
        assert_eq!(line_len((4, 4), (4, 4)), 1);
    }

    #[test]
    fn test_line_len_matches_line_cell_count() {
        for &(a, b) in &[
            ((0, 0), (5, 0)),
            ((0, 0), (0, -3)),
            ((1, 2), (9, 5)),
            ((3, 3), (-4, 8)),
        ] {
            assert_eq!(
                line_len(a, b),
                line(a, b).len(),
                "mismatch for {a:?}->{b:?}"
            );
        }
    }

    #[test]
    fn test_line_len_diagonal_is_chebyshev_plus_one() {
        // (0,0)->(3,3): 4 cells (chebyshev 3 + 1).
        assert_eq!(line_len((0, 0), (3, 3)), 4);
    }

    #[test]
    fn test_chebyshev_ring_negative_r_is_empty() {
        assert!(chebyshev_ring(0, 0, -1).is_empty());
    }

    #[test]
    fn test_chebyshev_ring_r0_is_centre() {
        assert_eq!(chebyshev_ring(3, 5, 0), vec![(3, 5)]);
    }

    #[test]
    fn test_chebyshev_ring_r1_has_eight_cells() {
        let ring = chebyshev_ring(0, 0, 1);
        assert_eq!(ring.len(), 8);
        // All cells must be at Chebyshev distance exactly 1.
        for (x, y) in &ring {
            assert_eq!(x.abs().max(y.abs()), 1);
        }
    }

    #[test]
    fn test_midpoint_horizontal() {
        assert_eq!(midpoint((0, 0), (4, 0)), (2, 0));
    }

    #[test]
    fn test_midpoint_diagonal() {
        assert_eq!(midpoint((0, 0), (6, 6)), (3, 3));
    }

    #[test]
    fn test_midpoint_same_point_is_itself() {
        assert_eq!(midpoint((5, 7), (5, 7)), (5, 7));
    }

    #[test]
    fn test_vec_toward_cardinal_directions() {
        assert_eq!(vec_toward((0, 0), (5, 0)), (1, 0));
        assert_eq!(vec_toward((0, 0), (-3, 0)), (-1, 0));
        assert_eq!(vec_toward((0, 0), (0, 10)), (0, 1));
        assert_eq!(vec_toward((0, 0), (0, -7)), (0, -1));
    }

    #[test]
    fn test_vec_toward_diagonal_and_same() {
        assert_eq!(vec_toward((3, 3), (7, 1)), (1, -1));
        assert_eq!(vec_toward((3, 3), (1, 9)), (-1, 1));
        assert_eq!(vec_toward((5, 5), (5, 5)), (0, 0));
    }

    #[test]
    fn test_vec_toward_components_are_unit() {
        for dx in -2i32..=2 {
            for dy in -2i32..=2 {
                let (vx, vy) = vec_toward((0, 0), (dx * 100, dy * 100));
                assert!(vx.abs() <= 1 && vy.abs() <= 1);
            }
        }
    }

    #[test]
    fn test_rect_center_even_dimensions() {
        assert_eq!(rect_center(0, 0, 4, 6), (2, 3));
    }

    #[test]
    fn test_rect_center_odd_floors_toward_origin() {
        // 5/2 == 2, 3/2 == 1 (integer floor)
        assert_eq!(rect_center(10, 20, 5, 3), (12, 21));
    }

    #[test]
    fn test_rect_center_zero_size_is_origin() {
        assert_eq!(rect_center(7, 9, 0, 0), (7, 9));
    }

    #[test]
    fn test_manhattan_distance_axis_aligned() {
        assert_eq!(manhattan_distance((0, 0), (3, 0)), 3);
        assert_eq!(manhattan_distance((0, 0), (0, 4)), 4);
    }

    #[test]
    fn test_manhattan_distance_diagonal() {
        assert_eq!(manhattan_distance((1, 1), (4, 5)), 7);
    }

    #[test]
    fn test_manhattan_distance_same_point_zero() {
        assert_eq!(manhattan_distance((5, 5), (5, 5)), 0);
    }

    #[test]
    fn test_chebyshev_distance_diagonal_equals_max_component() {
        assert_eq!(chebyshev_distance((0, 0), (3, 5)), 5);
    }

    #[test]
    fn test_chebyshev_distance_horizontal() {
        assert_eq!(chebyshev_distance((0, 0), (7, 0)), 7);
    }

    #[test]
    fn test_chebyshev_distance_same_point_zero() {
        assert_eq!(chebyshev_distance((2, 3), (2, 3)), 0);
    }

    // --- reflect_point ---

    #[test]
    fn test_reflect_point_across_center() {
        assert_eq!(reflect_point((1, 1), (5, 5)), (9, 9));
    }

    #[test]
    fn test_reflect_point_identity_at_center() {
        assert_eq!(reflect_point((3, 4), (3, 4)), (3, 4));
    }

    #[test]
    fn test_reflect_point_symmetric() {
        let p = (2, 8);
        let center = (5, 5);
        let r = reflect_point(p, center);
        assert_eq!(
            reflect_point(r, center),
            p,
            "double reflection returns original"
        );
    }

    // --- rotate_90_cw / rotate_90_ccw ---

    #[test]
    fn test_rotate_90_cw_cardinal_directions() {
        // Screen coords (y-down): Right→Down→Left→Up
        assert_eq!(rotate_90_cw(1, 0), (0, 1), "Right → Down");
        assert_eq!(rotate_90_cw(0, 1), (-1, 0), "Down → Left");
        assert_eq!(rotate_90_cw(-1, 0), (0, -1), "Left → Up");
        assert_eq!(rotate_90_cw(0, -1), (1, 0), "Up → Right");
    }

    #[test]
    fn test_rotate_90_ccw_cardinal_directions() {
        // CCW is the inverse of CW
        assert_eq!(rotate_90_ccw(1, 0), (0, -1), "Right → Up");
        assert_eq!(rotate_90_ccw(0, -1), (-1, 0), "Up → Left");
        assert_eq!(rotate_90_ccw(-1, 0), (0, 1), "Left → Down");
        assert_eq!(rotate_90_ccw(0, 1), (1, 0), "Down → Right");
    }

    #[test]
    fn test_rotate_90_cw_then_ccw_is_identity() {
        let cases = [(1, 2), (-3, 4), (0, 0), (7, -5)];
        for (x, y) in cases {
            let (rx, ry) = rotate_90_cw(x, y);
            assert_eq!(rotate_90_ccw(rx, ry), (x, y));
        }
    }

    #[test]
    fn test_rotate_90_cw_four_times_is_identity() {
        let (mut x, mut y) = (3, -7);
        for _ in 0..4 {
            let (nx, ny) = rotate_90_cw(x, y);
            x = nx;
            y = ny;
        }
        assert_eq!((x, y), (3, -7));
    }

    // --- ray_cast / ray_blocked_at -----------------------------------------

    #[test]
    fn test_ray_cast_clear_path_returns_full_line() {
        // No obstruction: the bolt reaches the target and the path matches `line`.
        let path = ray_cast((0, 0), (4, 0), |_, _| false);
        assert_eq!(path, line((0, 0), (4, 0)));
        assert_eq!(path.last(), Some(&(4, 0)), "landed on the target");
    }

    #[test]
    fn test_ray_cast_stops_at_first_blocker_inclusive() {
        // Wall at (3,0): the bolt stops there, and the wall is the last cell.
        let path = ray_cast((0, 0), (6, 0), |x, y| (x, y) == (3, 0));
        assert_eq!(path, vec![(0, 0), (1, 0), (2, 0), (3, 0)]);
        assert_eq!(path.last(), Some(&(3, 0)), "impact cell is included");
    }

    #[test]
    fn test_ray_cast_origin_never_blocks() {
        // Even if the origin cell tests as blocked, the bolt launches from it.
        let path = ray_cast((2, 2), (5, 2), |x, y| (x, y) == (2, 2));
        assert_eq!(path.first(), Some(&(2, 2)));
        assert_eq!(path.last(), Some(&(5, 2)), "reaches target despite blocked origin");
        assert_eq!(path.len(), 4);
    }

    #[test]
    fn test_ray_cast_origin_equals_target_is_single_cell() {
        let path = ray_cast((7, 7), (7, 7), |_, _| true);
        assert_eq!(path, vec![(7, 7)]);
    }

    #[test]
    fn test_ray_cast_blocked_at_target_returns_full_path() {
        // The target itself is a wall: the bolt reaches and stops on it.
        let path = ray_cast((0, 0), (3, 3), |x, y| (x, y) == (3, 3));
        assert_eq!(path.last(), Some(&(3, 3)));
        assert_eq!(path.len(), line_len((0, 0), (3, 3)));
    }

    #[test]
    fn test_ray_cast_is_deterministic() {
        let a = ray_cast((1, 1), (9, 4), |x, y| (x, y) == (5, 3));
        let b = ray_cast((1, 1), (9, 4), |x, y| (x, y) == (5, 3));
        assert_eq!(a, b, "pure integer trace must be reproducible");
    }

    #[test]
    fn test_ray_blocked_at_clear_shot_is_none() {
        assert_eq!(ray_blocked_at((0, 0), (5, 0), |_, _| false), None);
    }

    #[test]
    fn test_ray_blocked_at_reports_obstruction() {
        let hit = ray_blocked_at((0, 0), (6, 0), |x, y| (x, y) == (3, 0));
        assert_eq!(hit, Some((3, 0)), "first blocker short of target");
    }

    #[test]
    fn test_ray_blocked_at_wall_on_target_is_clear() {
        // Aiming at a wall is a clear shot *to* that wall, not a blocked shot.
        assert_eq!(ray_blocked_at((0, 0), (4, 4), |x, y| (x, y) == (4, 4)), None);
    }

    #[test]
    fn test_ray_blocked_at_origin_equals_target_is_none() {
        assert_eq!(ray_blocked_at((2, 2), (2, 2), |_, _| true), None);
    }

    #[test]
    fn test_ray_cast_interior_matches_line_of_sight() {
        // A clear ray_cast (reaching target) iff line_of_sight is true, for the
        // same opacity predicate — the two line-family checks must agree.
        let opaque = |x: i32, y: i32| (x, y) == (2, 1);
        let reaches = ray_cast((0, 0), (5, 2), opaque).last() == Some(&(5, 2));
        let los = line_of_sight((0, 0), (5, 2), opaque);
        assert_eq!(reaches, los, "ray_cast reach and line_of_sight must agree");
    }

    // --- cone --------------------------------------------------------------

    fn cone_set(origin: (i32, i32), facing: (i32, i32), range: i32) -> HashSet<(i32, i32)> {
        cone(origin, facing, range).into_iter().collect()
    }

    #[test]
    fn test_cone_excludes_origin() {
        assert!(!cone((0, 0), (1, 0), 3).contains(&(0, 0)));
    }

    #[test]
    fn test_cone_negative_range_or_zero_facing_is_empty() {
        assert!(cone((0, 0), (1, 0), -1).is_empty());
        assert!(cone((0, 0), (0, 0), 5).is_empty());
    }

    #[test]
    fn test_cone_east_includes_axis_and_45deg_edges() {
        // range 3 so the (2,±2) diagonal edges (dist √8 ≈ 2.83) are in range.
        let c = cone_set((0, 0), (1, 0), 3);
        // On-axis cells are in the cone.
        assert!(c.contains(&(1, 0)));
        assert!(c.contains(&(3, 0)));
        // The ±45° boundary cells are included (2·dot² == |o|²·|f|²).
        assert!(c.contains(&(1, 1)));
        assert!(c.contains(&(1, -1)));
        assert!(c.contains(&(2, 2)));
        assert!(c.contains(&(2, -2)));
    }

    #[test]
    fn test_cone_east_excludes_behind_and_steep_and_out_of_range() {
        let c = cone_set((0, 0), (1, 0), 2);
        assert!(!c.contains(&(-1, 0)), "behind the facing");
        assert!(!c.contains(&(0, 1)), "perpendicular (dot == 0)");
        assert!(!c.contains(&(1, 2)), "steeper than 45 degrees");
        assert!(!c.contains(&(3, 0)), "beyond range");
    }

    #[test]
    fn test_cone_is_symmetric_about_its_axis() {
        // East cone must be mirror-symmetric across the x-axis (y -> -y).
        let c = cone_set((0, 0), (1, 0), 4);
        for &(x, y) in &c {
            assert!(
                c.contains(&(x, -y)),
                "({x},{y}) in cone but its mirror ({x},{}) is not",
                -y
            );
        }
    }

    #[test]
    fn test_cone_diagonal_facing_spans_adjacent_cardinals() {
        // A south-east (1,1) cone is the ±45° wedge around the diagonal, so it
        // reaches toward both east (1,0) and south (0,1) but not the opposite.
        let c = cone_set((0, 0), (1, 1), 3);
        assert!(c.contains(&(1, 1)), "on the diagonal axis");
        assert!(c.contains(&(1, 0)), "east edge of the SE cone");
        assert!(c.contains(&(0, 1)), "south edge of the SE cone");
        assert!(!c.contains(&(-1, 0)), "west is outside a SE cone");
        assert!(!c.contains(&(0, -1)), "north is outside a SE cone");
    }

    #[test]
    fn test_cone_rotations_are_congruent() {
        // Rotating the facing 90° must rotate the cone cell-set the same way:
        // east cone rotated 90° CW equals the south cone (facing (0,1)).
        let east = cone((0, 0), (1, 0), 3);
        let rotated: HashSet<(i32, i32)> =
            east.iter().map(|&(x, y)| rotate_90_cw(x, y)).collect();
        let south = cone_set((0, 0), rotate_90_cw(1, 0), 3);
        assert_eq!(rotated, south, "cone must be rotation-congruent");
    }

    #[test]
    fn test_cone_is_deterministic_and_ordered() {
        let a = cone((2, 3), (1, 0), 4);
        let b = cone((2, 3), (1, 0), 4);
        assert_eq!(a, b, "pure integer shape must be reproducible");
        let mut sorted = a.clone();
        sorted.sort_by_key(|&(x, y)| (y, x));
        assert_eq!(a, sorted, "cells are returned in ascending (y, x) order");
    }

    #[test]
    fn test_cone_within_range() {
        // Every returned cell must satisfy the documented Euclidean range bound.
        let r = 5;
        for &(x, y) in &cone((0, 0), (2, 1), r) {
            assert!(x * x + y * y <= r * r, "({x},{y}) exceeds range {r}");
        }
    }

    // --- cone_visible ------------------------------------------------------

    #[test]
    fn test_cone_visible_no_walls_equals_cone() {
        let plain = cone((0, 0), (1, 0), 4);
        let visible = cone_visible((0, 0), (1, 0), 4, |_, _| false);
        assert_eq!(visible, plain, "with no opacity the footprint is the full cone");
    }

    #[test]
    fn test_cone_visible_is_always_a_subset_of_cone() {
        let plain: HashSet<(i32, i32)> = cone((0, 0), (1, 1), 5).into_iter().collect();
        let visible = cone_visible((0, 0), (1, 1), 5, |x, y| (x, y) == (2, 2));
        assert!(visible.iter().all(|c| plain.contains(c)), "visible ⊆ cone");
        assert!(visible.len() <= plain.len());
    }

    #[test]
    fn test_cone_visible_includes_struck_wall_but_culls_behind_it() {
        // East cone from origin; a wall at (3,0) on the central axis. The breath
        // strikes the wall (included) but cannot reach (4,0)/(5,0) behind it.
        let origin = (0, 0);
        let wall = |x: i32, y: i32| (x, y) == (3, 0);
        let visible: HashSet<(i32, i32)> =
            cone_visible(origin, (1, 0), 5, wall).into_iter().collect();
        assert!(visible.contains(&(3, 0)), "the struck wall cell is included");
        assert!(!visible.contains(&(4, 0)), "cell directly behind the wall is culled");
        assert!(!visible.contains(&(5, 0)), "cell further behind the wall is culled");
        assert!(visible.contains(&(2, 0)), "cell in front of the wall is reached");
    }

    #[test]
    fn test_cone_visible_empty_for_bad_args() {
        assert!(cone_visible((0, 0), (1, 0), -1, |_, _| false).is_empty());
        assert!(cone_visible((0, 0), (0, 0), 5, |_, _| false).is_empty());
    }

    #[test]
    fn test_cone_visible_is_deterministic() {
        let wall = |x: i32, y: i32| (x, y) == (2, 1);
        let a = cone_visible((0, 0), (1, 0), 4, wall);
        let b = cone_visible((0, 0), (1, 0), 4, wall);
        assert_eq!(a, b);
    }

    // --- knockback ---------------------------------------------------------

    #[test]
    fn test_knockback_unobstructed_moves_full_distance() {
        assert_eq!(knockback((0, 0), (1, 0), 3, |_, _| false), (3, 0));
    }

    #[test]
    fn test_knockback_stops_before_wall() {
        // Wall at (2,0): the entity halts on the last open cell (1,0).
        assert_eq!(knockback((0, 0), (1, 0), 5, |x, y| (x, y) == (2, 0)), (1, 0));
    }

    #[test]
    fn test_knockback_adjacent_wall_does_not_move() {
        assert_eq!(knockback((0, 0), (1, 0), 3, |x, y| (x, y) == (1, 0)), (0, 0));
    }

    #[test]
    fn test_knockback_zero_dir_and_nonpositive_distance_are_noops() {
        assert_eq!(knockback((4, 4), (0, 0), 5, |_, _| false), (4, 4));
        assert_eq!(knockback((4, 4), (1, 1), 0, |_, _| false), (4, 4));
        assert_eq!(knockback((4, 4), (1, 1), -2, |_, _| false), (4, 4));
    }

    #[test]
    fn test_knockback_direction_magnitude_is_ignored() {
        // (3, 0) is normalized to the east step, identical to (1, 0).
        let big = knockback((0, 0), (3, 0), 4, |_, _| false);
        let unit = knockback((0, 0), (1, 0), 4, |_, _| false);
        assert_eq!(big, unit);
        assert_eq!(big, (4, 0));
    }

    #[test]
    fn test_knockback_diagonal_push() {
        assert_eq!(knockback((1, 1), (1, 1), 2, |_, _| false), (3, 3));
    }

    #[test]
    fn test_knockback_away_from_source_via_vec_toward() {
        // Source at (5,5), target at (5,2): push the target north, away from source.
        let target = (5, 2);
        let dir = vec_toward((5, 5), target);
        assert_eq!(dir, (0, -1));
        assert_eq!(knockback(target, dir, 2, |_, _| false), (5, 0));
    }

    #[test]
    fn test_knockback_is_deterministic() {
        let wall = |x: i32, y: i32| (x, y) == (4, 0);
        assert_eq!(
            knockback((0, 0), (1, 0), 10, wall),
            knockback((0, 0), (1, 0), 10, wall)
        );
    }
}
