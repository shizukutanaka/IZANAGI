//! Deterministic A* pathfinding on a grid.
//!
//! Roguelikes need shortest paths for monster AI, auto-explore and travel. This
//! is 8-directional A* with **integer** octile costs (10 orthogonal, 14 diagonal
//! — a fixed-point stand-in for the 1 : √2 ratio) and the octile heuristic, so
//! it is admissible, consistent (optimal paths) and free of floating point.
//!
//! [`weighted_astar`] extends the algorithm with an integer heuristic inflation
//! factor `weight ≥ 1`. `f = g + weight × h` trades path optimality for speed:
//! the returned path costs at most `weight × optimal`. At `weight = 1` the result
//! is identical to [`astar`]; at `weight = 2` roughly half the nodes are expanded
//! in open maps while the path is still within 2× optimal.
//!
//! [`jps`] is Jump Point Search — an *exact* speed-up of [`astar`] for uniform
//! cost grids that "jumps" over open, symmetric regions instead of expanding
//! every cell. It returns the same kind of full path at the same optimal cost,
//! obeying the same no-corner-cutting rule, and is validated against [`astar`]
//! over thousands of random grids (cost-equality is the correctness oracle).
//! [`jps4`] is its 4-connected sibling for rulesets that forbid diagonal
//! movement, validated the same way against a plain-BFS oracle.
//!
//! [`dijkstra_map`] builds multi-source distance fields ("Dijkstra maps");
//! [`flee_map`] derives intelligent flee behaviour from one, and
//! [`combine_maps`] blends several by per-map coefficient (Brogue's technique)
//! so one [`descend`] call expresses "approach X while avoiding Y".
//!
//! Determinism: the open set is ordered by the *total* key `(f, wh, x, y)` — the
//! `(x, y)` tail makes every key unique, so there are no ties for the heap to
//! break arbitrarily and the popped order is identical on every run and target.
//! Neighbours are expanded in a fixed compass order. (See `bracket-lib`'s
//! `bracket-pathfinding` and libtcod for the same algorithm family.)
//!
//! Diagonal moves never cut a wall corner: moving diagonally requires both
//! shared orthogonal cells to be clear.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use crate::geometry::line as bresenham_line;

/// A multi-source integer distance field as produced by [`dijkstra_map`]:
/// each reachable cell mapped to its path cost from the nearest source.
/// Consumed by [`descend`], [`flee_map`] and [`combine_maps`].
pub type DijkstraMap = HashMap<(i32, i32), i32>;

/// Orthogonal step cost (≈ 1.0, scaled by 10).
const COST_ORTHO: i32 = 10;
/// Diagonal step cost (≈ √2, scaled to 14).
const COST_DIAG: i32 = 14;

/// Neighbour offsets in a fixed compass order (N, NE, E, SE, S, SW, W, NW). A
/// fixed order keeps expansion — and thus the result — deterministic.
const DIRS: [(i32, i32); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

/// Octile distance between `a` and `b`, scaled to match [`COST_ORTHO`] /
/// [`COST_DIAG`]: `10·max + 4·min`. Admissible and consistent for 8-way grids.
#[inline]
fn octile(a: (i32, i32), b: (i32, i32)) -> i32 {
    // Widen to i64: coordinate spans up to i32::MAX - i32::MIN (~2^32) and the
    // ×10 scaling both overflow i32 for extreme inputs. The heuristic stays
    // admissible after a saturating clamp (it only ever under-estimates true
    // cost once the true cost itself would exceed i32::MAX).
    let dx = (a.0 as i64 - b.0 as i64).abs();
    let dy = (a.1 as i64 - b.1 as i64).abs();
    let d = COST_ORTHO as i64 * (dx + dy) - (2 * COST_ORTHO - COST_DIAG) as i64 * dx.min(dy);
    d.clamp(0, i32::MAX as i64) as i32
}

/// Octile heuristic cost between `a` and `b` on this module's integer scale
/// (`10` orthogonal, `14` diagonal): `10·(dx+dy) − 6·min(dx,dy)`. This is the
/// exact cost [`astar`] pays to cross open ground, so callers can estimate a
/// path's length or test "is the goal within N cost?" without running a search.
/// Admissible and consistent for 8-way movement.
#[inline]
pub fn octile_distance(a: (i32, i32), b: (i32, i32)) -> i32 {
    octile(a, b)
}

fn reconstruct(came_from: &HashMap<(i32, i32), (i32, i32)>, goal: (i32, i32)) -> Vec<(i32, i32)> {
    let mut path = vec![goal];
    let mut cur = goal;
    while let Some(&prev) = came_from.get(&cur) {
        cur = prev;
        path.push(cur);
    }
    path.reverse();
    path
}

/// Find a shortest 8-directional path from `start` to `goal`, inclusive of both
/// endpoints, or `None` if `goal` is unreachable.
///
/// `is_blocked(x, y)` must report walls. It **must** also return `true` for
/// out-of-bounds cells: that is what bounds the search to a finite area (and
/// lets the function terminate with `None` when the goal is walled off).
///
/// Path cost uses internal `COST_ORTHO`/`COST_DIAG` constants; the returned path is one of the
/// optimal paths, chosen deterministically by the `(f, h, x, y)` ordering.
pub fn astar<B>(start: (i32, i32), goal: (i32, i32), mut is_blocked: B) -> Option<Vec<(i32, i32)>>
where
    B: FnMut(i32, i32) -> bool,
{
    if is_blocked(start.0, start.1) || is_blocked(goal.0, goal.1) {
        return None;
    }
    if start == goal {
        return Some(vec![start]);
    }

    // Open set keyed by (f, h, x, y); `Reverse` turns the max-heap into a
    // min-heap so the lowest f (then h, then coords) pops first.
    let mut open: BinaryHeap<Reverse<(i32, i32, i32, i32)>> = BinaryHeap::new();
    let mut g_score: HashMap<(i32, i32), i32> = HashMap::new();
    let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();

    let h0 = octile(start, goal);
    g_score.insert(start, 0);
    open.push(Reverse((h0, h0, start.0, start.1)));

    while let Some(Reverse((f, _h, cx, cy))) = open.pop() {
        let cur = (cx, cy);
        let cur_g = g_score[&cur];
        // Lazy deletion: skip stale heap entries left over from a cheaper relax.
        if f != cur_g + octile(cur, goal) {
            continue;
        }
        if cur == goal {
            return Some(reconstruct(&came_from, goal));
        }
        for (dx, dy) in DIRS {
            let (nx, ny) = (cx + dx, cy + dy);
            if is_blocked(nx, ny) {
                continue;
            }
            let diagonal = dx != 0 && dy != 0;
            // Never squeeze through a wall corner.
            if diagonal && (is_blocked(cx + dx, cy) || is_blocked(cx, cy + dy)) {
                continue;
            }
            let step = if diagonal { COST_DIAG } else { COST_ORTHO };
            let tentative = cur_g + step;
            let neighbour = (nx, ny);
            if tentative < *g_score.get(&neighbour).unwrap_or(&i32::MAX) {
                g_score.insert(neighbour, tentative);
                came_from.insert(neighbour, cur);
                let h = octile(neighbour, goal);
                open.push(Reverse((tentative + h, h, nx, ny)));
            }
        }
    }
    None
}

/// Find a path from `start` to `goal` using a weighted (ε-admissible) heuristic.
///
/// `weight ≥ 1` inflates the octile heuristic: `f = g + weight × h`.
/// At `weight = 1` the result is identical to [`astar`] (optimal). Larger
/// weights expand fewer nodes and return a path whose cost is at most
/// `weight × optimal_cost`. Values in `1..=3` cover typical roguelike needs.
///
/// All determinism invariants of [`astar`] hold: the open-set key is
/// `(f, weight*h, x, y)` — unique and total. `is_blocked` must return `true`
/// for out-of-bounds coordinates to bound the search.
pub fn weighted_astar<B>(
    start: (i32, i32),
    goal: (i32, i32),
    mut is_blocked: B,
    weight: u32,
) -> Option<Vec<(i32, i32)>>
where
    B: FnMut(i32, i32) -> bool,
{
    let w = weight.max(1) as i32;
    if is_blocked(start.0, start.1) || is_blocked(goal.0, goal.1) {
        return None;
    }
    if start == goal {
        return Some(vec![start]);
    }

    let mut open: BinaryHeap<Reverse<(i32, i32, i32, i32)>> = BinaryHeap::new();
    let mut g_score: HashMap<(i32, i32), i32> = HashMap::new();
    let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();

    let h0 = octile(start, goal);
    g_score.insert(start, 0);
    open.push(Reverse((w * h0, w * h0, start.0, start.1)));

    while let Some(Reverse((f, _wh, cx, cy))) = open.pop() {
        let cur = (cx, cy);
        let cur_g = g_score[&cur];
        // Lazy deletion: skip stale heap entries.
        if f != cur_g + w * octile(cur, goal) {
            continue;
        }
        if cur == goal {
            return Some(reconstruct(&came_from, goal));
        }
        for (dx, dy) in DIRS {
            let (nx, ny) = (cx + dx, cy + dy);
            if is_blocked(nx, ny) {
                continue;
            }
            let diagonal = dx != 0 && dy != 0;
            if diagonal && (is_blocked(cx + dx, cy) || is_blocked(cx, cy + dy)) {
                continue;
            }
            let step = if diagonal { COST_DIAG } else { COST_ORTHO };
            let tentative = cur_g + step;
            let neighbour = (nx, ny);
            if tentative < *g_score.get(&neighbour).unwrap_or(&i32::MAX) {
                g_score.insert(neighbour, tentative);
                came_from.insert(neighbour, cur);
                let h = octile(neighbour, goal);
                open.push(Reverse((tentative + w * h, w * h, nx, ny)));
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Jump Point Search (JPS)
// ---------------------------------------------------------------------------

/// Jump in direction `(dx, dy)` from `(x0, y0)` until reaching a *jump point*
/// (the goal, or a cell with a forced neighbour), a wall, or out of bounds.
///
/// `walk(x, y)` is the passability predicate (the negation of `is_blocked`).
/// Movement obeys the same no-corner-cutting rule as [`astar`]: a diagonal step
/// is only taken when both shared orthogonal cells are clear. Returns the jump
/// point cell, or `None` if the ray dead-ends without one.
///
/// Recursion depth is at most 2 (a diagonal frame probes two straight rays, and
/// straight rays never recurse), so this cannot blow the stack on large maps.
fn jps_jump<W: Fn(i32, i32) -> bool>(
    mut x: i32,
    mut y: i32,
    dx: i32,
    dy: i32,
    goal: (i32, i32),
    walk: &W,
) -> Option<(i32, i32)> {
    loop {
        if !walk(x, y) {
            return None;
        }
        if (x, y) == goal {
            return Some((x, y));
        }
        if dx != 0 && dy != 0 {
            // Diagonal: this cell is a jump point if a straight probe in either
            // component direction finds one (a forced neighbour lies that way).
            if jps_jump(x + dx, y, dx, 0, goal, walk).is_some()
                || jps_jump(x, y + dy, 0, dy, goal, walk).is_some()
            {
                return Some((x, y));
            }
            // Continue diagonally only without cutting a corner.
            if walk(x + dx, y) && walk(x, y + dy) {
                x += dx;
                y += dy;
                continue;
            }
            return None;
        } else if dx != 0 {
            // Horizontal: no-corner-cutting forced neighbour — a perpendicular
            // cell is open but the cell diagonally *behind* it is blocked, so the
            // only way to reach it is to turn here.
            if (walk(x, y + 1) && !walk(x - dx, y + 1)) || (walk(x, y - 1) && !walk(x - dx, y - 1))
            {
                return Some((x, y));
            }
            if walk(x + dx, y) {
                x += dx;
                continue;
            }
            return None;
        } else {
            // Vertical (dy != 0) — symmetric no-corner-cutting forced neighbour.
            if (walk(x + 1, y) && !walk(x + 1, y - dy)) || (walk(x - 1, y) && !walk(x - 1, y - dy))
            {
                return Some((x, y));
            }
            if walk(x, y + dy) {
                y += dy;
                continue;
            }
            return None;
        }
    }
}

/// Every legal jump direction from `(cx, cy)` in fixed compass order: an
/// orthogonal step needs only its target clear; a diagonal step is corner-safe
/// (target plus both shared orthogonal cells clear).
///
/// JPS only requires the expanded direction set to be a *superset* of the
/// natural and forced neighbours, so emitting all legal directions is correct —
/// the jumping speedup comes from [`jps_jump`], which collapses each direction
/// down to its next jump point. Keeping the set direction-agnostic also makes
/// the no-corner-cutting forced-neighbour bookkeeping unnecessary: a forced
/// neighbour is, by construction, one of these legal directions.
fn jps_successors<W: Fn(i32, i32) -> bool>(cx: i32, cy: i32, walk: &W) -> Vec<(i32, i32)> {
    let mut v = Vec::new();
    for (dx, dy) in DIRS {
        let legal = if dx != 0 && dy != 0 {
            walk(cx + dx, cy) && walk(cx, cy + dy) && walk(cx + dx, cy + dy)
        } else {
            walk(cx + dx, cy + dy)
        };
        if legal {
            v.push((dx, dy));
        }
    }
    v
}

/// Expand a chain of jump points (start → goal) into the full cell-by-cell path
/// by walking the straight/diagonal segment between each consecutive pair.
fn jps_reconstruct(
    came_from: &HashMap<(i32, i32), (i32, i32)>,
    start: (i32, i32),
    goal: (i32, i32),
) -> Vec<(i32, i32)> {
    let mut points = vec![goal];
    let mut cur = goal;
    while let Some(&prev) = came_from.get(&cur) {
        points.push(prev);
        cur = prev;
    }
    debug_assert_eq!(cur, start, "jump-point chain must terminate at start");
    points.reverse();

    let mut full = vec![points[0]];
    for w in points.windows(2) {
        let (ax, ay) = w[0];
        let (bx, by) = w[1];
        let sx = (bx - ax).signum();
        let sy = (by - ay).signum();
        let (mut x, mut y) = (ax, ay);
        while (x, y) != (bx, by) {
            x += sx;
            y += sy;
            full.push((x, y));
        }
    }
    full
}

/// Find a shortest 8-directional path from `start` to `goal` using **Jump Point
/// Search** — an optimisation of [`astar`] for uniform-cost grids that "jumps"
/// over symmetric, obstacle-free regions instead of expanding every cell.
///
/// Returns the same kind of full, cell-by-cell path as [`astar`] (so the two
/// are drop-in interchangeable) and a path of **identical cost** — JPS is exact,
/// not approximate. On open maps it expands far fewer nodes; on cramped maps it
/// degrades gracefully toward A*. Movement obeys the same no-corner-cutting rule.
///
/// `is_blocked(x, y)` must return `true` for walls **and** out-of-bounds cells
/// (this bounds the search). Determinism matches [`astar`]: the open set is keyed
/// by the total order `(f, h, x, y)` and directions are pruned in a fixed compass
/// order, so the result is identical on every run and platform.
///
/// The predicate is `Fn` (not `FnMut`) because the jump recursion queries cells
/// reentrantly; pass a closure that reads an immutable grid.
pub fn jps<B>(start: (i32, i32), goal: (i32, i32), is_blocked: B) -> Option<Vec<(i32, i32)>>
where
    B: Fn(i32, i32) -> bool,
{
    let walk = |x: i32, y: i32| !is_blocked(x, y);
    if !walk(start.0, start.1) || !walk(goal.0, goal.1) {
        return None;
    }
    if start == goal {
        return Some(vec![start]);
    }

    let mut open: BinaryHeap<Reverse<(i32, i32, i32, i32)>> = BinaryHeap::new();
    let mut g_score: HashMap<(i32, i32), i32> = HashMap::new();
    let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();

    let h0 = octile(start, goal);
    g_score.insert(start, 0);
    open.push(Reverse((h0, h0, start.0, start.1)));

    while let Some(Reverse((f, _h, cx, cy))) = open.pop() {
        let cur = (cx, cy);
        let cur_g = g_score[&cur];
        // Lazy deletion: skip stale heap entries left over from a cheaper relax.
        if f != cur_g + octile(cur, goal) {
            continue;
        }
        if cur == goal {
            return Some(jps_reconstruct(&came_from, start, goal));
        }
        for (dx, dy) in jps_successors(cx, cy, &walk) {
            if let Some(jp) = jps_jump(cx + dx, cy + dy, dx, dy, goal, &walk) {
                // A jump ray is purely straight or purely diagonal, so the octile
                // distance equals the exact segment cost.
                let tentative = cur_g + octile(cur, jp);
                if tentative < *g_score.get(&jp).unwrap_or(&i32::MAX) {
                    g_score.insert(jp, tentative);
                    came_from.insert(jp, cur);
                    let h = octile(jp, goal);
                    open.push(Reverse((tentative + h, h, jp.0, jp.1)));
                }
            }
        }
    }
    None
}

/// Manhattan distance — the exact path cost between two cells when movement is
/// restricted to the four cardinal directions at unit cost. Computed in `i64`
/// and clamped like [`octile`], so extreme coordinates saturate instead of
/// overflowing.
#[inline]
fn manhattan(a: (i32, i32), b: (i32, i32)) -> i32 {
    let dx = (a.0 as i64 - b.0 as i64).abs();
    let dy = (a.1 as i64 - b.1 as i64).abs();
    (dx + dy).min(i32::MAX as i64) as i32
}

/// Slide along a cardinal ray for [`jps4`], returning the first *jump point*
/// (a cell the parent search must expand) or `None` if the ray dies in a wall.
///
/// The design mirrors [`jps_jump`]'s diagonal/straight split, with **vertical
/// as the dominant axis** (the canonical shortest path moves vertically first;
/// Baum 2025, arXiv:2501.14816):
///
/// - A **horizontal** ray is terminal: it stops at (1) the goal, (2) the
///   goal's column — a canonical path may turn toward the goal there; without
///   this stop the ray would overshoot — or (3) a *forced neighbour*: a
///   walkable cell perpendicular to the ray whose predecessor (one step back
///   along the ray) is blocked, meaning any shortest path reaching that
///   perpendicular cell must pass through here.
/// - A **vertical** ray additionally *probes* both horizontal directions at
///   every step (just as [`jps_jump`]'s diagonal case probes its two straight
///   components): if either probe finds a jump point, a canonical shortest
///   path may turn horizontally here, so this cell is itself a jump point.
///   Without the probes a vertical ray in open space would run to the map
///   boundary and die, and the search could never turn — losing completeness.
///
/// Horizontal rays never recurse, so the probe recursion is exactly one level
/// deep.
fn jps4_jump<W: Fn(i32, i32) -> bool>(
    mut x: i32,
    mut y: i32,
    dx: i32,
    dy: i32,
    goal: (i32, i32),
    walk: &W,
) -> Option<(i32, i32)> {
    loop {
        if !walk(x, y) {
            return None;
        }
        if (x, y) == goal {
            return Some((x, y));
        }
        if dx != 0 {
            if x == goal.0 {
                return Some((x, y));
            }
            if (walk(x, y + 1) && !walk(x - dx, y + 1)) || (walk(x, y - 1) && !walk(x - dx, y - 1))
            {
                return Some((x, y));
            }
            x += dx;
        } else {
            if y == goal.1 {
                return Some((x, y));
            }
            if (walk(x + 1, y) && !walk(x + 1, y - dy)) || (walk(x - 1, y) && !walk(x - 1, y - dy))
            {
                return Some((x, y));
            }
            // Dominant-axis probes: scan east and west from this cell; a hit
            // means a canonical path may turn here.
            if jps4_jump(x + 1, y, 1, 0, goal, walk).is_some()
                || jps4_jump(x - 1, y, -1, 0, goal, walk).is_some()
            {
                return Some((x, y));
            }
            y += dy;
        }
    }
}

/// Find a shortest **4-directional** (cardinal-only, unit-cost) path from
/// `start` to `goal` using Jump Point Search specialised for 4-connected grids
/// (Baum 2025, arXiv:2501.14816).
///
/// This is the pathfinder to use when game rules forbid diagonal movement —
/// classic roguelike movement, sokoban-likes, pipe/wire routing. [`astar`] and
/// [`jps`] are 8-connected and will happily cut diagonals; `jps4` never does.
/// The returned path is a full cell-by-cell walk (consecutive cells differ by
/// exactly one cardinal step), its length minus one is the exact minimal move
/// count, and like [`jps`] it is exact, not approximate.
///
/// Why the jump conditions preserve optimality (exchange argument): on a
/// 4-connected unit grid any shortest path can be rewritten, without changing
/// its length, into a *canonical* form that moves vertically before
/// horizontally wherever possible, so that every horizontal→vertical turn is
/// *forced* by an obstacle (moving the turn earlier is blocked — exactly the
/// forced-neighbour pattern tested in [`jps4_jump`]) or happens in the goal's
/// column, and every vertical→horizontal turn starts a horizontal segment that
/// itself ends at a jump point — which is exactly what the vertical ray's
/// horizontal probes detect. Every straight run between such cells can
/// therefore be skipped wholesale, which is what jumping does. The expansion
/// generates all four cardinal directions at each jump point (a conservative
/// superset of the canonical pruned set — same philosophy as
/// [`jps_successors`]), so no successor needed by that canonical form is ever
/// missing.
///
/// `is_blocked(x, y)` must return `true` for walls **and** out-of-bounds cells
/// (this bounds the search). The predicate is `Fn` (not `FnMut`) because the
/// jump scan re-queries cells; pass a closure reading an immutable grid.
/// Deterministic: the open set is keyed by the total order `(f, h, x, y)` and
/// directions are tried in fixed N/E/S/W order, so the result is identical on
/// every run and platform.
pub fn jps4<B>(start: (i32, i32), goal: (i32, i32), is_blocked: B) -> Option<Vec<(i32, i32)>>
where
    B: Fn(i32, i32) -> bool,
{
    let walk = |x: i32, y: i32| !is_blocked(x, y);
    if !walk(start.0, start.1) || !walk(goal.0, goal.1) {
        return None;
    }
    if start == goal {
        return Some(vec![start]);
    }

    const CARDINALS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    let mut open: BinaryHeap<Reverse<(i32, i32, i32, i32)>> = BinaryHeap::new();
    let mut g_score: HashMap<(i32, i32), i32> = HashMap::new();
    let mut came_from: HashMap<(i32, i32), (i32, i32)> = HashMap::new();

    let h0 = manhattan(start, goal);
    g_score.insert(start, 0);
    open.push(Reverse((h0, h0, start.0, start.1)));

    while let Some(Reverse((f, _h, cx, cy))) = open.pop() {
        let cur = (cx, cy);
        let cur_g = g_score[&cur];
        // Lazy deletion: skip stale heap entries left over from a cheaper relax.
        if f != cur_g + manhattan(cur, goal) {
            continue;
        }
        if cur == goal {
            return Some(jps_reconstruct(&came_from, start, goal));
        }
        for (dx, dy) in CARDINALS {
            if !walk(cx + dx, cy + dy) {
                continue;
            }
            if let Some(jp) = jps4_jump(cx + dx, cy + dy, dx, dy, goal, &walk) {
                // A jump ray is a straight cardinal segment, so the Manhattan
                // distance equals the exact segment cost.
                let tentative = cur_g + manhattan(cur, jp);
                if tentative < *g_score.get(&jp).unwrap_or(&i32::MAX) {
                    g_score.insert(jp, tentative);
                    came_from.insert(jp, cur);
                    let h = manhattan(jp, goal);
                    open.push(Reverse((tentative + h, h, jp.0, jp.1)));
                }
            }
        }
    }
    None
}

/// Multi-source Dijkstra distance field — a "flow field" / Dijkstra map (the
/// `bracket-lib` term). Returns the minimum path cost from the *nearest* source
/// to every reachable cell whose cost is `<= max_cost`. Sources map to 0.
///
/// Same 8-way moves, integer octile costs and no-corner-cutting rule as
/// [`astar`]. Deterministic: the frontier is ordered by `(cost, x, y)`, so the
/// computed cost of each cell is identical across runs and targets (the
/// returned map's *iteration* order is not meaningful — look cells up by key).
///
/// `is_blocked` must report walls and out-of-bounds (this bounds the search).
/// Blocked source cells are skipped; duplicate sources are harmless.
pub fn dijkstra_map<B>(
    sources: &[(i32, i32)],
    max_cost: i32,
    mut is_blocked: B,
) -> HashMap<(i32, i32), i32>
where
    B: FnMut(i32, i32) -> bool,
{
    let mut dist: HashMap<(i32, i32), i32> = HashMap::new();
    let mut frontier: BinaryHeap<Reverse<(i32, i32, i32)>> = BinaryHeap::new();
    for &(sx, sy) in sources {
        if is_blocked(sx, sy) || dist.contains_key(&(sx, sy)) {
            continue;
        }
        dist.insert((sx, sy), 0);
        frontier.push(Reverse((0, sx, sy)));
    }
    while let Some(Reverse((cost, cx, cy))) = frontier.pop() {
        // Lazy deletion: skip stale entries superseded by a cheaper relax.
        if cost > dist[&(cx, cy)] {
            continue;
        }
        for (dx, dy) in DIRS {
            let (nx, ny) = (cx + dx, cy + dy);
            if is_blocked(nx, ny) {
                continue;
            }
            let diagonal = dx != 0 && dy != 0;
            if diagonal && (is_blocked(cx + dx, cy) || is_blocked(cx, cy + dy)) {
                continue;
            }
            let next = cost + if diagonal { COST_DIAG } else { COST_ORTHO };
            if next > max_cost {
                continue;
            }
            if next < *dist.get(&(nx, ny)).unwrap_or(&i32::MAX) {
                dist.insert((nx, ny), next);
                frontier.push(Reverse((next, nx, ny)));
            }
        }
    }
    dist
}

/// The reachable cell **farthest** (greatest path cost) from `sources`, with
/// that cost — the canonical "place the down-stairs as far from the entrance
/// as possible" primitive (Brian Walker's Dijkstra-map technique, Roguelike
/// Celebration 2018; the stair-placement step of a Wolverson-style map builder).
///
/// Runs one [`dijkstra_map`] from `sources` bounded by `max_cost`, then takes
/// the argmax. Ties (several cells equidistant at the maximum) break by
/// row-major order — smallest `(y, x)` — so the choice is **deterministic**
/// regardless of the underlying map's iteration order. Returns `None` only
/// when no cell is reachable (every source blocked, or `sources` empty);
/// otherwise at least the nearest source itself is present at cost 0.
///
/// `is_blocked` must report walls and out-of-bounds. Drops straight onto a
/// [`crate::mapgen::Dungeon`] via `|x, y| dungeon.is_wall(x, y)`: pass the
/// entrance/up-stairs as the sole source and the result is where the
/// down-stairs (or the boss, or the best loot) should go.
pub fn farthest_cell<B>(
    sources: &[(i32, i32)],
    max_cost: i32,
    is_blocked: B,
) -> Option<((i32, i32), i32)>
where
    B: FnMut(i32, i32) -> bool,
{
    let dist = dijkstra_map(sources, max_cost, is_blocked);
    let mut best: Option<((i32, i32), i32)> = None;
    for (&(x, y), &cost) in &dist {
        let better = match best {
            None => true,
            // Strictly farther wins; on a tie the row-major-earliest cell wins,
            // a total order independent of HashMap iteration order.
            Some((bcell, bcost)) => cost > bcost || (cost == bcost && (y, x) < (bcell.1, bcell.0)),
        };
        if better {
            best = Some(((x, y), cost));
        }
    }
    best
}

/// One step of steepest descent down a [`dijkstra_map`] — the passable
/// neighbour with the lowest cost strictly below `from`'s. Returns `None` at a
/// source, a local minimum, or a cell absent from the map. Useful for chase /
/// flee (descend a map; flee by descending its negation). Ties break by fixed
/// compass order, so the choice is deterministic.
pub fn descend<B>(
    map: &HashMap<(i32, i32), i32>,
    from: (i32, i32),
    mut is_blocked: B,
) -> Option<(i32, i32)>
where
    B: FnMut(i32, i32) -> bool,
{
    let current = *map.get(&from)?;
    let mut best: Option<((i32, i32), i32)> = None;
    for (dx, dy) in DIRS {
        let (nx, ny) = (from.0 + dx, from.1 + dy);
        if is_blocked(nx, ny) {
            continue;
        }
        let diagonal = dx != 0 && dy != 0;
        if diagonal && (is_blocked(from.0 + dx, from.1) || is_blocked(from.0, from.1 + dy)) {
            continue;
        }
        if let Some(&cost) = map.get(&(nx, ny)) {
            // Replace only on a strict improvement, so the earliest (fixed-order)
            // neighbour wins ties — deterministic.
            if cost < current && best.is_none_or_lower(cost) {
                best = Some(((nx, ny), cost));
            }
        }
    }
    best.map(|(cell, _)| cell)
}

/// Small helper to keep [`descend`]'s tie-break explicit and MSRV-friendly.
trait BestCost {
    fn is_none_or_lower(&self, candidate: i32) -> bool;
}

impl BestCost for Option<((i32, i32), i32)> {
    #[inline]
    fn is_none_or_lower(&self, candidate: i32) -> bool {
        match self {
            None => true,
            Some((_, best)) => candidate < *best,
        }
    }
}

/// Build a **flee map** ("safety map") from a [`dijkstra_map`] desire field, so
/// that descending the result makes an entity flee *intelligently* — away from
/// the sources but routing around obstacles instead of into dead ends.
///
/// Naively negating a Dijkstra map and descending it produces cowardly-but-dumb
/// behaviour: an entity backed into a dead-end corner sees no lower-valued
/// neighbour and stops, even though the corner is a death trap. The fix, from
/// the RogueBasin technique "The Incredible Power of Dijkstra Maps", is to
/// negate the map by a coefficient slightly above 1 (`coeff_num/coeff_den`,
/// e.g. `12/10` = 1.2) and then **rescan**: relax every cell against its
/// neighbours until the field is a consistent distance map again. The rescan
/// re-introduces the gradient that pulls fleers down corridors toward open
/// space rather than letting them freeze in local minima.
///
/// Uses the same 8-way moves, octile costs and no-corner-cutting rule as
/// [`dijkstra_map`]. `coeff_den` of `0` is treated as `1`. Deterministic: cells
/// are relaxed in sorted `(x, y)` order to a fixpoint, so the result is
/// identical across runs. Only cells present in `desire` appear in the output.
pub fn flee_map<B>(
    desire: &HashMap<(i32, i32), i32>,
    coeff_num: i32,
    coeff_den: i32,
    mut is_blocked: B,
) -> HashMap<(i32, i32), i32>
where
    B: FnMut(i32, i32) -> bool,
{
    let den = if coeff_den == 0 { 1 } else { coeff_den };
    // Step 1: negate by the coefficient. i64 intermediate avoids overflow.
    let mut flee: HashMap<(i32, i32), i32> = HashMap::with_capacity(desire.len());
    for (&cell, &v) in desire {
        let scaled = -((v as i64 * coeff_num as i64) / den as i64);
        flee.insert(cell, scaled.clamp(i32::MIN as i64, i32::MAX as i64) as i32);
    }

    // Deterministic cell order for the relaxation passes.
    let mut cells: Vec<(i32, i32)> = flee.keys().copied().collect();
    cells.sort_unstable();

    // Step 2: rescan to a fixpoint. Values only ever decrease and are bounded
    // below by the most-negative seed, so this terminates in <= |cells| passes.
    loop {
        let mut changed = false;
        for &(cx, cy) in &cells {
            let current = flee[&(cx, cy)];
            let mut best = current;
            for (dx, dy) in DIRS {
                let (nx, ny) = (cx + dx, cy + dy);
                if is_blocked(nx, ny) {
                    continue;
                }
                let diagonal = dx != 0 && dy != 0;
                if diagonal && (is_blocked(cx + dx, cy) || is_blocked(cx, cy + dy)) {
                    continue;
                }
                if let Some(&nv) = flee.get(&(nx, ny)) {
                    let step = if diagonal { COST_DIAG } else { COST_ORTHO };
                    let candidate = nv.saturating_add(step);
                    if candidate < best {
                        best = candidate;
                    }
                }
            }
            if best < current {
                flee.insert((cx, cy), best);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    flee
}

/// Blend several [`dijkstra_map`] fields into one desire field by weighted
/// sum — the coefficient-composition technique from Brogue (Brian Walker,
/// Roguelike Celebration 2018): build one map per motivation (distance to
/// player, to food, to the exit…), then combine them with a coefficient per
/// map — positive attracts (lower is nearer, so it pulls the descent toward
/// that map's sources), negative repels — and drive the entity by [`descend`]
/// on the sum. Rich behaviour ("approach the player but keep away from fire")
/// falls out of two or three coefficients instead of bespoke AI code.
///
/// Only cells present in **every** input map appear in the output: a cell
/// missing from one field has no defined desirability there, so blending it
/// would compare incomplete sums against complete ones. Passing an empty
/// slice yields an empty map. Sums use `i64` intermediates and saturate to
/// `i32`, so extreme values and coefficients cannot overflow.
///
/// Deterministic: the output is pure per-cell arithmetic on the inputs; as
/// with [`dijkstra_map`], look cells up by key — the map's iteration order is
/// not meaningful.
pub fn combine_maps(maps: &[(&DijkstraMap, i32)]) -> DijkstraMap {
    let (first, rest) = match maps.split_first() {
        Some(split) => split,
        None => return HashMap::new(),
    };
    let mut out = HashMap::new();
    'cells: for (&cell, &v0) in first.0 {
        let mut sum = v0 as i64 * first.1 as i64;
        for &(map, coeff) in rest {
            match map.get(&cell) {
                Some(&v) => sum += v as i64 * coeff as i64,
                None => continue 'cells,
            }
        }
        out.insert(cell, sum.clamp(i32::MIN as i64, i32::MAX as i64) as i32);
    }
    out
}

/// Remove redundant waypoints from a grid path using Bresenham LOS pruning
/// ("greedy string-pull").
///
/// Starting from each waypoint, skips to the farthest successor reachable via
/// a straight Bresenham line with no blocked interior cells. The result always
/// includes the original start and goal and is traversable under the same
/// blocking rules. A path of 0–2 cells is returned unchanged.
///
/// This is a **post-processor** — call it on a path from [`astar`] or
/// [`weighted_astar`]. The smoothed path has fewer direction changes (reduces
/// the stair-step visual) and is suitable for AI waypoint navigation: the actor
/// still moves step-by-step between each consecutive pair of smoothed waypoints,
/// Take one step from `from` toward `goal` on the shortest unblocked path,
/// returning the next cell to move to. Returns `None` when no path exists or
/// `from == goal`. The canonical "move AI one cell per turn" primitive:
/// avoids storing a full path when only the next step matters.
///
/// Internally runs `astar`; if repeated single-step calls are expensive,
/// cache the full path with `astar` and consume it a step at a time.
pub fn step_toward<B>(from: (i32, i32), goal: (i32, i32), is_blocked: B) -> Option<(i32, i32)>
where
    B: FnMut(i32, i32) -> bool,
{
    if from == goal {
        return None;
    }
    let path = astar(from, goal, is_blocked)?;
    path.get(1).copied()
}

/// so the no-corner-cutting rule is enforced per-step at runtime.
///
/// Determinism: purely functional; the same inputs always yield the same output.
pub fn smooth_path<B>(path: &[(i32, i32)], mut is_blocked: B) -> Vec<(i32, i32)>
where
    B: FnMut(i32, i32) -> bool,
{
    if path.len() <= 2 {
        return path.to_vec();
    }
    let mut result = vec![path[0]];
    let mut anchor = 0usize;
    loop {
        if anchor >= path.len() - 1 {
            break;
        }
        // Find the farthest j > anchor reachable via a clear Bresenham segment.
        let mut j = path.len() - 1;
        while j > anchor + 1 {
            if los_segment_clear(path[anchor], path[j], &mut is_blocked) {
                break;
            }
            j -= 1;
        }
        result.push(path[j]);
        anchor = j;
    }
    result
}

/// Total octile cost of a path — the sum of [`octile_distance`] for each
/// consecutive step pair. An empty or single-cell path has cost `0`. Matches
/// the cost scale A* uses (`10` per orthogonal step, `14` per diagonal step),
/// so the value is directly comparable to A* `g`-scores and heuristic estimates.
pub fn path_cost(path: &[(i32, i32)]) -> i32 {
    path.windows(2).map(|w| octile_distance(w[0], w[1])).sum()
}

/// Returns `true` if `goal` is reachable from `start` under the `is_blocked`
/// predicate, without retaining the path.
///
/// This is a thin wrapper around [`astar`] that discards the path Vec and
/// returns only the reachability boolean. Avoids allocating the full path when
/// connectivity alone is needed (e.g. "can the player reach the exit?").
pub fn is_reachable<B>(start: (i32, i32), goal: (i32, i32), is_blocked: B) -> bool
where
    B: FnMut(i32, i32) -> bool,
{
    astar(start, goal, is_blocked).is_some()
}

/// A precomputed connected-components labeling of a rectangular grid: build it
/// once, then answer "can A reach B?" in **O(1)** instead of a fresh
/// [`is_reachable`] BFS per query.
///
/// Dwarf Fortress found that a maintained connectivity structure beats
/// repeatedly improving the pathfinder when the frequent question is merely
/// *whether* two cells connect (GDC talks, 2016 onward): a monster deciding
/// if the player is even reachable shouldn't pay for a full search every turn.
/// [`ConnectivityMap`] labels every passable cell with a component id via a
/// single flood fill, so `connected` is a label comparison.
///
/// Connectivity matches [`is_reachable`] exactly — the same 8-directional,
/// no-corner-cutting movement rule (a diagonal link needs both shared
/// orthogonal cells passable) — so `map.connected(a, b) == is_reachable(a, b,
/// same_blocker)` for every in-bounds pair. Labels are assigned in row-major
/// scan order, so the whole structure is **deterministic**: the same grid
/// always yields the same ids.
///
/// It is an **immutable snapshot**. When the map changes (a door opens, a wall
/// is dug), rebuild it — this deliberately sidesteps the incremental-cache
/// invalidation that is easy to get subtly wrong (and non-deterministic). A
/// rebuild is one linear flood fill.
pub struct ConnectivityMap {
    width: i32,
    height: i32,
    /// Row-major component id per cell; `-1` for blocked/impassable cells.
    labels: Vec<i32>,
    /// `sizes[id]` = number of cells in component `id`.
    sizes: Vec<u32>,
}

impl ConnectivityMap {
    /// Label the connected components of a `width × height` grid under
    /// `is_blocked` (which must also report out-of-bounds as blocked, matching
    /// the rest of this module). Runs one flood fill; components use the same
    /// 8-way no-corner-cutting rule as [`astar`]/[`is_reachable`].
    pub fn new<B>(width: u32, height: u32, mut is_blocked: B) -> Self
    where
        B: FnMut(i32, i32) -> bool,
    {
        let w = width as i32;
        let h = height as i32;
        let stride = width as usize;
        let mut labels = vec![-1i32; stride.saturating_mul(height as usize)];
        let mut sizes: Vec<u32> = Vec::new();
        let idx = |x: i32, y: i32| y as usize * stride + x as usize;

        for sy in 0..h {
            for sx in 0..w {
                if is_blocked(sx, sy) || labels[idx(sx, sy)] >= 0 {
                    continue;
                }
                // A fresh component: flood it, all cells taking this id. The
                // scan order fixes ids deterministically.
                let id = sizes.len() as i32;
                let mut count = 0u32;
                let mut queue = std::collections::VecDeque::new();
                labels[idx(sx, sy)] = id;
                queue.push_back((sx, sy));
                while let Some((cx, cy)) = queue.pop_front() {
                    count += 1;
                    for (dx, dy) in DIRS {
                        let (nx, ny) = (cx + dx, cy + dy);
                        if nx < 0 || ny < 0 || nx >= w || ny >= h || is_blocked(nx, ny) {
                            continue;
                        }
                        let diagonal = dx != 0 && dy != 0;
                        if diagonal && (is_blocked(cx + dx, cy) || is_blocked(cx, cy + dy)) {
                            continue;
                        }
                        if labels[idx(nx, ny)] < 0 {
                            labels[idx(nx, ny)] = id;
                            queue.push_back((nx, ny));
                        }
                    }
                }
                sizes.push(count);
            }
        }
        ConnectivityMap {
            width: w,
            height: h,
            labels,
            sizes,
        }
    }

    /// The component id of `(x, y)`, or `None` if it is out of bounds or an
    /// impassable (blocked) cell.
    pub fn component(&self, x: i32, y: i32) -> Option<u32> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        let l = self.labels[y as usize * self.width as usize + x as usize];
        if l < 0 {
            None
        } else {
            Some(l as u32)
        }
    }

    /// Whether `a` and `b` are in the same component — i.e. mutually reachable.
    /// `false` if either is blocked or out of bounds. O(1).
    pub fn connected(&self, a: (i32, i32), b: (i32, i32)) -> bool {
        match (self.component(a.0, a.1), self.component(b.0, b.1)) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        }
    }

    /// Number of distinct components (0 for an all-blocked grid).
    pub fn component_count(&self) -> usize {
        self.sizes.len()
    }

    /// The number of passable cells in component `id`, or `None` if `id` is
    /// out of range.
    pub fn component_size(&self, id: u32) -> Option<u32> {
        self.sizes.get(id as usize).copied()
    }

    /// The id of the largest component (most cells), or `None` if the grid has
    /// no passable cells. Ties break toward the smallest id, so the choice is
    /// deterministic — handy for "which region is the main map" when culling
    /// disconnected pockets.
    pub fn largest_component(&self) -> Option<u32> {
        self.sizes
            .iter()
            .enumerate()
            .max_by_key(|&(i, &s)| (s, core::cmp::Reverse(i)))
            .map(|(i, _)| i as u32)
    }
}

/// Returns `true` if every cell in `path` is passable under `is_blocked`.
///
/// Use this to validate a cached path against the current map state before
/// following it — dynamic obstacles (doors, actors) may have blocked cells
/// that were open when the path was originally computed.
///
/// An empty path is considered clear. `is_blocked` is called once per cell
/// in path order and short-circuits on the first blocked cell.
pub fn is_path_clear<B: FnMut(i32, i32) -> bool>(path: &[(i32, i32)], mut is_blocked: B) -> bool {
    path.iter().all(|&(x, y)| !is_blocked(x, y))
}

/// Convert a sequence of waypoints into unit direction vectors between
/// consecutive cells. For each pair `(a, b)` of adjacent path entries the
/// result contains `((b.0−a.0).signum(), (b.1−a.1).signum())`. Returns an
/// empty `Vec` for paths of length `< 2`. Useful for driving animation ("face
/// this direction on each step") and smooth movement without storing full
/// coordinates.
pub fn path_to_direction_vec(path: &[(i32, i32)]) -> Vec<(i32, i32)> {
    path.windows(2)
        .map(|w| ((w[1].0 - w[0].0).signum(), (w[1].1 - w[0].1).signum()))
        .collect()
}

/// BFS from `start`, collecting all cells reachable within `max_dist` steps
/// (orthogonal or diagonal, non-corner-cutting). `start` itself is always
/// included unless `is_blocked(start)` is `true`. Returns cells in BFS
/// expansion order — deterministic because the 8-direction neighbour order is
/// fixed (same compass order as A*). Use for "reveal connected room",
/// "spread fire", and "count reachable floor cells" patterns.
pub fn flood_fill<B>(start: (i32, i32), max_dist: i32, mut is_blocked: B) -> Vec<(i32, i32)>
where
    B: FnMut(i32, i32) -> bool,
{
    if max_dist < 0 || is_blocked(start.0, start.1) {
        return Vec::new();
    }
    let mut visited: HashMap<(i32, i32), i32> = HashMap::new();
    let mut queue = std::collections::VecDeque::new();
    visited.insert(start, 0);
    queue.push_back((start.0, start.1, 0i32));
    let mut result = Vec::new();
    while let Some((cx, cy, dist)) = queue.pop_front() {
        result.push((cx, cy));
        if dist >= max_dist {
            continue;
        }
        for (dx, dy) in DIRS {
            let (nx, ny) = (cx + dx, cy + dy);
            if visited.contains_key(&(nx, ny)) || is_blocked(nx, ny) {
                continue;
            }
            let diagonal = dx != 0 && dy != 0;
            if diagonal && (is_blocked(cx + dx, cy) || is_blocked(cx, cy + dy)) {
                continue;
            }
            visited.insert((nx, ny), dist + 1);
            queue.push_back((nx, ny, dist + 1));
        }
    }
    result
}

/// BFS from `start`, returning the nearest passable non-start cell for which
/// `pred(x, y)` is `true`. "Nearest" is BFS depth (hop count), which is
/// deterministic because neighbours are expanded in the fixed `DIRS` compass
/// order. Uses the same corner-cutting rule as `flood_fill`: a diagonal move
/// is only taken when both adjacent orthogonal cells are passable.
///
/// `is_passable(x, y)` must return `false` for out-of-bounds cells (the
/// search terminates naturally when no passable neighbours can be found).
/// `pred` is tested only on passable cells; if the target cell itself may be
/// blocked, widen `is_passable` to include it.
///
/// Returns `None` when no reachable cell satisfies `pred` or when `start` is
/// itself impassable.
pub fn nearest_reachable<P, F>(
    start: (i32, i32),
    mut is_passable: P,
    mut pred: F,
) -> Option<(i32, i32)>
where
    P: FnMut(i32, i32) -> bool,
    F: FnMut(i32, i32) -> bool,
{
    if !is_passable(start.0, start.1) {
        return None;
    }
    let mut visited = std::collections::HashSet::new();
    let mut queue = std::collections::VecDeque::new();
    visited.insert(start);
    queue.push_back(start);
    while let Some((cx, cy)) = queue.pop_front() {
        for (dx, dy) in DIRS {
            let (nx, ny) = (cx + dx, cy + dy);
            if visited.contains(&(nx, ny)) || !is_passable(nx, ny) {
                continue;
            }
            let diagonal = dx != 0 && dy != 0;
            if diagonal && (!is_passable(cx + dx, cy) || !is_passable(cx, cy + dy)) {
                continue;
            }
            visited.insert((nx, ny));
            if pred(nx, ny) {
                return Some((nx, ny));
            }
            queue.push_back((nx, ny));
        }
    }
    None
}

/// **Auto-explore**: find the shortest route from `start` to the nearest cell
/// bordering unexplored territory, travelling only through cells the player has
/// already seen. This is the classic roguelike "explore" / travel command
/// (NetHack, Dungeon Crawl): repeatedly call it and walk the returned path to
/// sweep an entire level without manual input.
///
/// The search expands only over cells that are both **explored** (`is_explored`)
/// and **passable** (`!is_blocked`) — you cannot route through tiles you have
/// never seen. A *frontier* is a reached cell that has at least one 8-neighbour
/// which is still unexplored; the nearest such cell is the goal.
///
/// Returns:
/// - `None` if the whole reachable, explored area has been fully explored (no
///   frontier remains) — auto-explore is "done" — or if `start` is impassable.
/// - `Some(path)` from `start` to the nearest frontier, inclusive of both ends.
///   If `start` itself borders the unknown, the path is `vec![start]`.
///
/// Same 8-way moves, octile costs and no-corner-cutting rule as [`astar`].
/// Deterministic: the frontier is ordered by `(cost, x, y)` and parents are set
/// only on a strict cost improvement, so the chosen target and path are stable.
pub fn auto_explore<B, E>(
    start: (i32, i32),
    mut is_blocked: B,
    mut is_explored: E,
) -> Option<Vec<(i32, i32)>>
where
    B: FnMut(i32, i32) -> bool,
    E: FnMut(i32, i32) -> bool,
{
    if is_blocked(start.0, start.1) {
        return None;
    }
    // Has `c` any 8-neighbour that is unexplored *and passable* — i.e. genuinely
    // explorable unknown floor, not a wall or out-of-bounds edge (which would
    // make every border cell a false frontier)?
    let borders_unknown = |x: i32, y: i32, is_blocked: &mut B, is_explored: &mut E| {
        for (dx, dy) in DIRS {
            let (nx, ny) = (x + dx, y + dy);
            if !is_explored(nx, ny) && !is_blocked(nx, ny) {
                return true;
            }
        }
        false
    };

    let mut dist: HashMap<(i32, i32), i32> = HashMap::new();
    let mut parent: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
    let mut frontier: BinaryHeap<Reverse<(i32, i32, i32)>> = BinaryHeap::new();
    dist.insert(start, 0);
    frontier.push(Reverse((0, start.0, start.1)));

    while let Some(Reverse((cost, cx, cy))) = frontier.pop() {
        if cost > dist[&(cx, cy)] {
            continue; // stale heap entry
        }
        // The first cell popped (lowest cost, (x,y) tie-break) that borders the
        // unknown is the nearest frontier — reconstruct and return its path.
        if borders_unknown(cx, cy, &mut is_blocked, &mut is_explored) {
            let mut path = vec![(cx, cy)];
            let mut cur = (cx, cy);
            while let Some(&p) = parent.get(&cur) {
                path.push(p);
                cur = p;
            }
            path.reverse();
            return Some(path);
        }
        for (dx, dy) in DIRS {
            let (nx, ny) = (cx + dx, cy + dy);
            // Only travel through explored, passable cells.
            if is_blocked(nx, ny) || !is_explored(nx, ny) {
                continue;
            }
            let diagonal = dx != 0 && dy != 0;
            if diagonal
                && (is_blocked(cx + dx, cy)
                    || is_blocked(cx, cy + dy)
                    || !is_explored(cx + dx, cy)
                    || !is_explored(cx, cy + dy))
            {
                continue;
            }
            let next = cost + if diagonal { COST_DIAG } else { COST_ORTHO };
            if next < *dist.get(&(nx, ny)).unwrap_or(&i32::MAX) {
                dist.insert((nx, ny), next);
                parent.insert((nx, ny), (cx, cy));
                frontier.push(Reverse((next, nx, ny)));
            }
        }
    }
    None
}

/// Returns `true` when the Bresenham segment from `a` to `b` has no blocked
/// interior cells (endpoints are not checked — same semantics as `line_of_sight`
/// in `geometry`).
fn los_segment_clear<B: FnMut(i32, i32) -> bool>(
    a: (i32, i32),
    b: (i32, i32),
    is_blocked: &mut B,
) -> bool {
    let cells = bresenham_line(a, b);
    // Skip the first (a) and last (b) cells — only check the interior.
    if let Some(interior) = cells.get(1..cells.len().saturating_sub(1)) {
        for &(x, y) in interior {
            if is_blocked(x, y) {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Cost of a path under the same octile scale the search uses.
    fn path_cost(path: &[(i32, i32)]) -> i32 {
        path.windows(2)
            .map(|w| {
                let (dx, dy) = ((w[1].0 - w[0].0).abs(), (w[1].1 - w[0].1).abs());
                if dx != 0 && dy != 0 {
                    COST_DIAG
                } else {
                    COST_ORTHO
                }
            })
            .sum()
    }

    /// Walls = the given set; everything outside `[0,w)×[0,h)` is also blocked.
    fn blocker(w: i32, h: i32, walls: HashSet<(i32, i32)>) -> impl Fn(i32, i32) -> bool {
        move |x, y| x < 0 || y < 0 || x >= w || y >= h || walls.contains(&(x, y))
    }

    #[test]
    fn test_start_equals_goal() {
        let path = astar((3, 3), (3, 3), blocker(10, 10, HashSet::new())).unwrap();
        assert_eq!(path, vec![(3, 3)]);
    }

    #[test]
    fn test_straight_line_open_grid() {
        let path = astar((1, 5), (6, 5), blocker(12, 12, HashSet::new())).unwrap();
        assert_eq!(path.first(), Some(&(1, 5)));
        assert_eq!(path.last(), Some(&(6, 5)));
        // 5 orthogonal steps.
        assert_eq!(path.len(), 6);
        assert_eq!(path_cost(&path), 5 * COST_ORTHO);
    }

    #[test]
    fn test_diagonal_open_grid_uses_diagonals() {
        let path = astar((0, 0), (4, 4), blocker(10, 10, HashSet::new())).unwrap();
        // Pure diagonal: 4 steps, cost 4·14.
        assert_eq!(path.len(), 5);
        assert_eq!(path_cost(&path), 4 * COST_DIAG);
    }

    #[test]
    fn test_blocked_endpoints_return_none() {
        let walls = HashSet::from([(2, 2)]);
        assert!(astar((2, 2), (5, 5), blocker(10, 10, walls.clone())).is_none());
        assert!(astar((5, 5), (2, 2), blocker(10, 10, walls)).is_none());
    }

    #[test]
    fn test_unreachable_goal_returns_none() {
        // Seal (9,9) into its own pocket with a wall ring around it.
        let mut walls = HashSet::new();
        for x in 7..=9 {
            for y in 7..=9 {
                if (x, y) != (9, 9) {
                    walls.insert((x, y));
                }
            }
        }
        assert!(astar((0, 0), (9, 9), blocker(10, 10, walls)).is_none());
    }

    #[test]
    fn test_wall_forces_detour() {
        // A vertical wall with a single gap; the path must route through it.
        let mut walls = HashSet::new();
        for y in 0..9 {
            walls.insert((5, y)); // wall along x=5 for y in 0..9, gap at y=9
        }
        let path = astar((1, 4), (8, 4), blocker(12, 12, walls)).unwrap();
        assert_eq!(path.first(), Some(&(1, 4)));
        assert_eq!(path.last(), Some(&(8, 4)));
        // Must pass through the only gap in the wall column.
        assert!(path.contains(&(5, 9)), "path must use the wall's only gap");
    }

    #[test]
    fn test_no_diagonal_corner_cutting() {
        // Block E and S of the start; the SE diagonal would cut the corner.
        let walls = HashSet::from([(4, 3), (3, 4)]);
        let path = astar((3, 3), (4, 4), blocker(8, 8, walls)).unwrap();
        // The illegal direct step (3,3)->(4,4) must not appear; a detour is taken.
        let took_corner = path.windows(2).any(|w| w[0] == (3, 3) && w[1] == (4, 4));
        assert!(!took_corner, "must not cut the wall corner diagonally");
        assert!(
            path_cost(&path) > COST_DIAG,
            "detour must cost more than one diagonal"
        );
    }

    #[test]
    fn test_pathfinding_is_deterministic() {
        let walls = HashSet::from([(4, 2), (4, 3), (4, 4), (2, 6)]);
        let a = astar((1, 1), (8, 8), blocker(12, 12, walls.clone()));
        let b = astar((1, 1), (8, 8), blocker(12, 12, walls));
        assert_eq!(a, b);
        assert!(a.is_some());
    }

    #[test]
    fn test_dijkstra_map_costs_on_open_grid() {
        let map = dijkstra_map(&[(0, 0)], 1000, blocker(12, 12, HashSet::new()));
        assert_eq!(map[&(0, 0)], 0);
        assert_eq!(map[&(3, 0)], 3 * COST_ORTHO); // straight
        assert_eq!(map[&(3, 3)], 3 * COST_DIAG); // pure diagonal
        assert_eq!(map[&(1, 1)], COST_DIAG);
        assert_eq!(map[&(2, 1)], COST_DIAG + COST_ORTHO); // one diag + one ortho
    }

    #[test]
    fn test_dijkstra_map_respects_max_cost() {
        let map = dijkstra_map(&[(0, 0)], 2 * COST_ORTHO, blocker(20, 20, HashSet::new()));
        assert!(map.contains_key(&(2, 0)), "cost 20 is within budget");
        assert!(!map.contains_key(&(3, 0)), "cost 30 exceeds the budget");
        assert!(map.values().all(|&c| c <= 2 * COST_ORTHO));
    }

    #[test]
    fn test_dijkstra_map_multi_source_takes_minimum() {
        // Two sources at opposite ends; the midpoint takes the nearer one.
        let map = dijkstra_map(&[(0, 0), (10, 0)], 1000, blocker(11, 3, HashSet::new()));
        assert_eq!(map[&(0, 0)], 0);
        assert_eq!(map[&(10, 0)], 0);
        assert_eq!(map[&(2, 0)], 2 * COST_ORTHO); // nearer to (0,0)
        assert_eq!(map[&(8, 0)], 2 * COST_ORTHO); // nearer to (10,0)
    }

    #[test]
    fn test_farthest_cell_on_open_grid_is_a_corner() {
        // From the top-left corner of an open grid, the farthest reachable
        // cell is the opposite corner; with the row-major tie-break the
        // reported cell is deterministic.
        let (w, h) = (6, 4);
        let (cell, cost) =
            farthest_cell(&[(0, 0)], 100_000, blocker(w, h, HashSet::new())).unwrap();
        assert_eq!(cell, (5, 3), "opposite corner is farthest");
        // 3 diagonal steps then 2 orthogonal to reach (5,3).
        assert_eq!(cost, 3 * COST_DIAG + 2 * COST_ORTHO);
    }

    #[test]
    fn test_farthest_cell_none_when_unreachable() {
        // Source is walled in on all sides: only the source itself is
        // reachable, at cost 0.
        let walls = HashSet::from([(1, 0), (0, 1), (1, 1)]);
        let (cell, cost) = farthest_cell(&[(0, 0)], 100_000, blocker(4, 4, walls)).unwrap();
        assert_eq!(cell, (0, 0));
        assert_eq!(cost, 0);
        // A blocked source yields nothing reachable at all.
        let walled = HashSet::from([(0, 0)]);
        assert!(farthest_cell(&[(0, 0)], 100_000, blocker(4, 4, walled)).is_none());
    }

    #[test]
    fn test_farthest_cell_empty_sources_is_none() {
        assert!(farthest_cell(&[], 100_000, blocker(4, 4, HashSet::new())).is_none());
    }

    #[test]
    fn test_farthest_cell_matches_dijkstra_argmax_and_is_deterministic() {
        // Cross-check against a manual argmax over the full dijkstra_map, with
        // the same row-major tie-break, over a walled grid.
        let walls = HashSet::from([(3, 1), (3, 2), (3, 3), (3, 4)]);
        let (w, h) = (10, 8);
        let blocked = blocker(w, h, walls);
        let map = dijkstra_map(&[(1, 1)], 100_000, &blocked);
        let mut expect: Option<((i32, i32), i32)> = None;
        for (&(x, y), &c) in &map {
            let better = match expect {
                None => true,
                Some((bc, bcost)) => c > bcost || (c == bcost && (y, x) < (bc.1, bc.0)),
            };
            if better {
                expect = Some(((x, y), c));
            }
        }
        let got = farthest_cell(&[(1, 1)], 100_000, &blocked);
        assert_eq!(got, expect);
        // Deterministic across repeated calls.
        assert_eq!(got, farthest_cell(&[(1, 1)], 100_000, &blocked));
    }

    #[test]
    fn test_farthest_cell_places_stairs_far_from_entrance_in_a_dungeon() {
        // The intended use: farthest reachable floor cell from the entrance is
        // where the down-stairs go. Build a deterministic dungeon and confirm
        // the result is a floor cell strictly farther than the entrance.
        use crate::mapgen::{generate_dungeon, GenParams};
        use crate::rng::SplitMix64;
        let mut rng = SplitMix64::new(0xDEAD_BEEF);
        let d = generate_dungeon(48, 32, &mut rng, GenParams::default());
        let entrance = d.floor_cells()[0];
        let (stairs, cost) = farthest_cell(&[entrance], 1_000_000, |x, y| d.is_wall(x, y)).unwrap();
        assert!(
            d.is_floor(stairs.0, stairs.1),
            "stairs must be on a floor cell"
        );
        assert!(
            cost > 0,
            "farthest cell must be strictly beyond the entrance"
        );
    }

    // --- ConnectivityMap ---

    #[test]
    fn test_connectivity_open_grid_is_one_component() {
        let cm = ConnectivityMap::new(6, 4, blocker(6, 4, HashSet::new()));
        assert_eq!(cm.component_count(), 1);
        assert_eq!(cm.component_size(0), Some(24));
        assert!(cm.connected((0, 0), (5, 3)));
        assert_eq!(cm.largest_component(), Some(0));
    }

    #[test]
    fn test_connectivity_all_walls_has_no_components() {
        let walls: HashSet<(i32, i32)> = (0..3).flat_map(|y| (0..3).map(move |x| (x, y))).collect();
        let cm = ConnectivityMap::new(3, 3, blocker(3, 3, walls));
        assert_eq!(cm.component_count(), 0);
        assert_eq!(cm.component(1, 1), None);
        assert!(!cm.connected((0, 0), (2, 2)));
        assert_eq!(cm.largest_component(), None);
    }

    #[test]
    fn test_connectivity_wall_splits_into_two_components() {
        // A solid vertical wall at x=2 splits a 5x3 grid into left and right.
        let walls: HashSet<(i32, i32)> = (0..3).map(|y| (2, y)).collect();
        let cm = ConnectivityMap::new(5, 3, blocker(5, 3, walls));
        assert_eq!(cm.component_count(), 2);
        assert!(
            cm.connected((0, 0), (1, 2)),
            "left region internally connected"
        );
        assert!(
            cm.connected((3, 0), (4, 2)),
            "right region internally connected"
        );
        assert!(
            !cm.connected((0, 0), (4, 0)),
            "wall separates the two sides"
        );
        // Row-major scan labels the left region (first passable cell) as 0.
        assert_eq!(cm.component(0, 0), Some(0));
    }

    #[test]
    fn test_connectivity_respects_no_corner_cutting() {
        // (0,0) and (1,1) touch only diagonally, with both shared orthogonals
        // walled — no corner cutting, so they are NOT connected, matching
        // is_reachable.
        let walls = HashSet::from([(1, 0), (0, 1)]);
        let blocked = blocker(2, 2, walls.clone());
        let cm = ConnectivityMap::new(2, 2, blocker(2, 2, walls));
        assert!(!cm.connected((0, 0), (1, 1)));
        assert!(!is_reachable((0, 0), (1, 1), &blocked));
        assert_eq!(cm.component_count(), 2);
    }

    #[test]
    fn test_connectivity_matches_is_reachable_over_random_grids() {
        // Oracle: connected() must agree with a fresh is_reachable BFS for
        // every pair, over thousands of random grids.
        let mut state: u64 = 0xC0FF_EE00_1234_5678;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut checked = 0u32;
        for _ in 0..400 {
            let w = 4 + (next() % 8) as i32;
            let h = 4 + (next() % 8) as i32;
            let density = next() % 55;
            let mut walls = HashSet::new();
            for y in 0..h {
                for x in 0..w {
                    if next() % 100 < density {
                        walls.insert((x, y));
                    }
                }
            }
            let cm = ConnectivityMap::new(w as u32, h as u32, blocker(w, h, walls.clone()));
            // Compare several random pairs against the is_reachable oracle.
            for _ in 0..6 {
                let a = ((next() % w as u64) as i32, (next() % h as u64) as i32);
                let b = ((next() % w as u64) as i32, (next() % h as u64) as i32);
                let expect = is_reachable(a, b, blocker(w, h, walls.clone()));
                assert_eq!(
                    cm.connected(a, b),
                    expect,
                    "connected({a:?},{b:?}) disagreed with is_reachable on w={w} h={h}"
                );
                checked += 1;
            }
        }
        assert!(
            checked >= 2400,
            "expected >= 2400 pair checks, got {checked}"
        );
    }

    #[test]
    fn test_connectivity_is_deterministic() {
        let walls = HashSet::from([(2, 0), (2, 1), (2, 2), (5, 3)]);
        let a = ConnectivityMap::new(8, 6, blocker(8, 6, walls.clone()));
        let b = ConnectivityMap::new(8, 6, blocker(8, 6, walls));
        assert_eq!(a.component_count(), b.component_count());
        for y in 0..6 {
            for x in 0..8 {
                assert_eq!(a.component(x, y), b.component(x, y), "label at ({x},{y})");
            }
        }
    }

    #[test]
    fn test_connectivity_largest_component_ties_to_smallest_id() {
        // Two equal-size regions split by a wall column: largest_component
        // must deterministically return the lower id (0), not an arbitrary one.
        let walls: HashSet<(i32, i32)> = (0..3).map(|y| (2, y)).collect();
        let cm = ConnectivityMap::new(5, 3, blocker(5, 3, walls));
        assert_eq!(cm.component_size(0), cm.component_size(1));
        assert_eq!(cm.largest_component(), Some(0));
    }

    #[test]
    fn test_descend_walks_to_a_source() {
        let walls = HashSet::from([(5, 0), (5, 1), (5, 2), (5, 3)]); // partial wall
        let blocked = blocker(12, 12, walls);
        let map = dijkstra_map(&[(0, 0)], 10_000, &blocked);
        // Greedily descend from a far cell; must strictly decrease and reach 0.
        let mut cur = (9, 5);
        let mut last = map[&cur];
        let mut steps = 0;
        while let Some(next) = descend(&map, cur, &blocked) {
            assert!(map[&next] < last, "descent must strictly decrease cost");
            last = map[&next];
            cur = next;
            steps += 1;
            assert!(steps < 1000, "descent must terminate");
        }
        assert_eq!(map[&cur], 0, "descent ends at a source");
    }

    // --- flee_map ---

    #[test]
    fn test_flee_map_covers_same_cells_as_desire() {
        let blocked = blocker(8, 8, HashSet::new());
        let desire = dijkstra_map(&[(0, 0)], 10_000, &blocked);
        let flee = flee_map(&desire, 12, 10, &blocked);
        assert_eq!(
            flee.len(),
            desire.len(),
            "flee map covers exactly the desire cells"
        );
        for key in desire.keys() {
            assert!(flee.contains_key(key));
        }
    }

    #[test]
    fn test_flee_map_descends_away_from_source() {
        // On an open grid, descending the flee map should move AWAY from the
        // player at (0,0): each step's Chebyshev distance to the source grows.
        let blocked = blocker(10, 10, HashSet::new());
        let desire = dijkstra_map(&[(0, 0)], 10_000, &blocked);
        let flee = flee_map(&desire, 12, 10, &blocked);
        let mut cur = (4, 4);
        let mut last_chev = cur.0.max(cur.1);
        let mut steps = 0;
        while let Some(next) = descend(&flee, cur, &blocked) {
            let chev = next.0.max(next.1);
            assert!(
                chev >= last_chev,
                "fleeing should not move back toward the source"
            );
            last_chev = chev;
            cur = next;
            steps += 1;
            assert!(steps < 1000, "descent must terminate");
        }
        assert!(steps > 0, "a fleer in the open should be able to move");
    }

    #[test]
    fn test_flee_map_escapes_dead_end() {
        // A pocket at (1,1) with the only exit at (1,2) leading out to open
        // space; the player sits just outside at (1,0). A naive negated map
        // would trap a fleer in the pocket; the rescanned flee map must still
        // provide a descending step OUT of the pocket toward the exit.
        //   col:   0 1 2 3 4
        // row0:    # P # # #
        // row1:    # . # # #   <- fleer starts here, walls left/right
        // row2:    # . . . .   <- corridor opens to the right
        let mut walls = HashSet::new();
        for x in 0..5 {
            for y in 0..3 {
                walls.insert((x, y));
            }
        }
        // Carve the pocket + corridor.
        for cell in [(1, 0), (1, 1), (1, 2), (2, 2), (3, 2), (4, 2)] {
            walls.remove(&cell);
        }
        let blocked = blocker(5, 3, walls);
        let player = (1, 0);
        let desire = dijkstra_map(&[player], 10_000, &blocked);
        let flee = flee_map(&desire, 12, 10, &blocked);
        // From the pocket cell (1,1), descending must lead to (1,2) — deeper
        // into safety — not stall.
        let next = descend(&flee, (1, 1), &blocked);
        assert_eq!(
            next,
            Some((1, 2)),
            "fleer escapes the pocket toward the corridor"
        );
    }

    #[test]
    fn test_flee_map_is_deterministic() {
        let walls = HashSet::from([(3, 3), (3, 4), (4, 3)]);
        let blocked = blocker(9, 9, walls);
        let desire = dijkstra_map(&[(0, 0), (8, 8)], 10_000, &blocked);
        let a = flee_map(&desire, 12, 10, &blocked);
        let b = flee_map(&desire, 12, 10, &blocked);
        assert_eq!(a, b, "flee_map is deterministic for identical input");
    }

    #[test]
    fn test_flee_map_zero_denominator_treated_as_one() {
        let blocked = blocker(5, 5, HashSet::new());
        let desire = dijkstra_map(&[(0, 0)], 10_000, &blocked);
        let safe = flee_map(&desire, 1, 0, &blocked); // den 0 → 1
                                                      // Should not panic and should produce a same-sized map.
        assert_eq!(safe.len(), desire.len());
    }

    // --- combine_maps ---

    #[test]
    fn test_combine_maps_empty_slice_yields_empty_map() {
        assert!(combine_maps(&[]).is_empty());
    }

    #[test]
    fn test_combine_maps_single_map_coeff_one_is_identity() {
        let blocked = blocker(6, 6, HashSet::new());
        let m = dijkstra_map(&[(0, 0)], 10_000, &blocked);
        assert_eq!(combine_maps(&[(&m, 1)]), m);
    }

    #[test]
    fn test_combine_maps_intersection_only() {
        // Cells missing from either map must not appear in the output.
        let mut a = HashMap::new();
        a.insert((0, 0), 1);
        a.insert((1, 0), 2);
        let mut b = HashMap::new();
        b.insert((1, 0), 5);
        b.insert((2, 0), 7);
        let c = combine_maps(&[(&a, 1), (&b, 1)]);
        assert_eq!(c.len(), 1);
        assert_eq!(c[&(1, 0)], 7);
    }

    #[test]
    fn test_combine_maps_weighted_sum() {
        let mut a = HashMap::new();
        a.insert((0, 0), 10);
        let mut b = HashMap::new();
        b.insert((0, 0), 3);
        let c = combine_maps(&[(&a, 2), (&b, -4)]);
        assert_eq!(c[&(0, 0)], 2 * 10 - 4 * 3);
    }

    #[test]
    fn test_combine_maps_saturates_instead_of_overflowing() {
        let mut a = HashMap::new();
        a.insert((0, 0), i32::MAX);
        let c = combine_maps(&[(&a, i32::MAX)]);
        assert_eq!(c[&(0, 0)], i32::MAX);
        let d = combine_maps(&[(&a, i32::MIN)]);
        assert_eq!(d[&(0, 0)], i32::MIN);
    }

    #[test]
    fn test_combine_maps_approach_while_avoiding() {
        // Brogue's motivating example: approach the player while avoiding a
        // hazard. A corridor 0..=8 at y=0; player at (8,0), fire at (4,0).
        // Pure attraction walks into the fire; the blend detours the descent
        // *away* from the fire cell even while net motion is toward the player.
        let blocked = blocker(9, 1, HashSet::new());
        let to_player = dijkstra_map(&[(8, 0)], 10_000, &blocked);
        let to_fire = dijkstra_map(&[(4, 0)], 10_000, &blocked);
        // Strong fire repulsion: coefficient -3 vs. player attraction +1.
        let blend = combine_maps(&[(&to_player, 1), (&to_fire, -3)]);
        // blend[x] = (8-x)*ORTHO - 3*|x-4|*ORTHO, peaking on the fire cell.
        // Standing right next to the fire at (3,0): pure attraction would step
        // east onto (4,0); in the blend that step trades -1 step of player
        // distance for +3 steps of repulsion, so the descent retreats west.
        assert_eq!(
            descend(&blend, (3, 0), &blocked),
            Some((2, 0)),
            "blend must retreat from the fire, not descend into it"
        );
        // West of the fire every step east strengthens repulsion (3x) faster
        // than it improves attraction (1x), so the west end is a local minimum
        // and the entity correctly refuses to run the gauntlet.
        assert_eq!(descend(&blend, (0, 0), &blocked), None);
        // East of the fire both motivations agree (player nearer, fire
        // farther), so the descent resumes toward the player.
        assert_eq!(descend(&blend, (5, 0), &blocked), Some((6, 0)));
    }

    #[test]
    fn test_combine_maps_is_deterministic() {
        let walls = HashSet::from([(3, 3)]);
        let blocked = blocker(8, 8, walls);
        let m1 = dijkstra_map(&[(0, 0)], 10_000, &blocked);
        let m2 = dijkstra_map(&[(7, 7)], 10_000, &blocked);
        let a = combine_maps(&[(&m1, 2), (&m2, -1)]);
        let b = combine_maps(&[(&m1, 2), (&m2, -1)]);
        assert_eq!(a, b);
    }

    // --- weighted_astar ---

    #[test]
    fn test_weighted_astar_weight_one_matches_astar_cost() {
        // weight=1 must find a path of the same cost as astar.
        let path_a = astar((0, 0), (8, 5), blocker(12, 12, HashSet::new())).unwrap();
        let path_w = weighted_astar((0, 0), (8, 5), blocker(12, 12, HashSet::new()), 1).unwrap();
        assert_eq!(path_cost(&path_a), path_cost(&path_w));
    }

    #[test]
    fn test_weighted_astar_finds_goal_with_weight_two() {
        let path = weighted_astar((0, 0), (9, 9), blocker(12, 12, HashSet::new()), 2).unwrap();
        assert_eq!(path.first(), Some(&(0, 0)));
        assert_eq!(path.last(), Some(&(9, 9)));
    }

    #[test]
    fn test_weighted_astar_start_equals_goal() {
        let path = weighted_astar((3, 3), (3, 3), blocker(10, 10, HashSet::new()), 2).unwrap();
        assert_eq!(path, vec![(3, 3)]);
    }

    #[test]
    fn test_weighted_astar_blocked_start_returns_none() {
        let walls = HashSet::from([(2, 2)]);
        assert!(weighted_astar((2, 2), (5, 5), blocker(10, 10, walls), 2).is_none());
    }

    #[test]
    fn test_weighted_astar_unreachable_goal_returns_none() {
        let mut walls = HashSet::new();
        for x in 7..=9 {
            for y in 7..=9 {
                if (x, y) != (9, 9) {
                    walls.insert((x, y));
                }
            }
        }
        assert!(weighted_astar((0, 0), (9, 9), blocker(10, 10, walls), 2).is_none());
    }

    #[test]
    fn test_weighted_astar_path_cost_bounded() {
        // Path cost must be ≤ weight × optimal cost.
        let weight = 3u32;
        let optimal = astar((0, 0), (10, 7), blocker(15, 15, HashSet::new())).unwrap();
        let subopt =
            weighted_astar((0, 0), (10, 7), blocker(15, 15, HashSet::new()), weight).unwrap();
        assert!(
            path_cost(&subopt) <= weight as i32 * path_cost(&optimal),
            "weighted path cost {} must be ≤ {}×optimal={}",
            path_cost(&subopt),
            weight,
            path_cost(&optimal)
        );
    }

    #[test]
    fn test_weighted_astar_is_deterministic() {
        let walls = HashSet::from([(4, 2), (4, 3), (4, 4)]);
        let a = weighted_astar((1, 1), (9, 9), blocker(12, 12, walls.clone()), 2);
        let b = weighted_astar((1, 1), (9, 9), blocker(12, 12, walls), 2);
        assert_eq!(a, b);
        assert!(a.is_some());
    }

    #[test]
    fn test_weighted_astar_weight_zero_treated_as_one() {
        // weight=0 is clamped to 1; must still find a path.
        let path = weighted_astar((0, 0), (5, 5), blocker(10, 10, HashSet::new()), 0).unwrap();
        assert_eq!(path.last(), Some(&(5, 5)));
    }

    #[test]
    fn test_dijkstra_map_is_deterministic() {
        let walls = HashSet::from([(4, 2), (4, 3), (4, 4)]);
        let a = dijkstra_map(&[(1, 1), (9, 9)], 500, blocker(12, 12, walls.clone()));
        let b = dijkstra_map(&[(1, 1), (9, 9)], 500, blocker(12, 12, walls));
        // Same keys and same costs (values are the deterministic part).
        let mut ka: Vec<_> = a.iter().collect();
        let mut kb: Vec<_> = b.iter().collect();
        ka.sort();
        kb.sort();
        assert_eq!(ka, kb);
    }

    // --- smooth_path ---

    #[test]
    fn test_smooth_path_trivial_cases() {
        let b = blocker(10, 10, HashSet::new());
        assert_eq!(smooth_path(&[], b), vec![]);
        let b = blocker(10, 10, HashSet::new());
        assert_eq!(smooth_path(&[(1, 1)], b), vec![(1, 1)]);
        let b = blocker(10, 10, HashSet::new());
        assert_eq!(smooth_path(&[(1, 1), (3, 1)], b), vec![(1, 1), (3, 1)]);
    }

    #[test]
    fn test_smooth_path_straight_line_collapses() {
        // A* staircase on open grid: smoothing should reduce to just start and goal.
        let path = astar((0, 0), (6, 0), blocker(10, 10, HashSet::new())).unwrap();
        let smooth = smooth_path(&path, blocker(10, 10, HashSet::new()));
        assert_eq!(smooth.first(), Some(&(0, 0)));
        assert_eq!(smooth.last(), Some(&(6, 0)));
        // Open straight line: all interior hops are skippable.
        assert!(smooth.len() <= path.len(), "smooth must not add waypoints");
    }

    #[test]
    fn test_smooth_path_preserves_start_and_goal() {
        let walls = HashSet::from([(3, 0), (3, 1), (3, 2), (3, 3)]);
        let path = astar((0, 2), (7, 2), blocker(10, 10, walls.clone())).unwrap();
        let smooth = smooth_path(&path, blocker(10, 10, walls));
        assert_eq!(smooth.first(), path.first());
        assert_eq!(smooth.last(), path.last());
    }

    #[test]
    fn test_smooth_path_does_not_cross_walls() {
        // The wall separates start from goal; smooth path must not pass through it.
        let walls: HashSet<(i32, i32)> = (0..8).map(|y| (4, y)).collect();
        let path = astar((1, 4), (8, 4), blocker(12, 12, walls.clone())).unwrap();
        let smooth = smooth_path(&path, blocker(12, 12, walls.clone()));
        // Every consecutive pair of smoothed waypoints must have a clear LOS.
        for w in smooth.windows(2) {
            assert!(
                los_segment_clear(w[0], w[1], &mut |x, y| walls.contains(&(x, y))),
                "smoothed segment {:?}->{:?} crosses a wall",
                w[0],
                w[1]
            );
        }
    }

    #[test]
    fn test_smooth_path_is_deterministic() {
        let walls = HashSet::from([(4, 2), (4, 3)]);
        let path = astar((0, 0), (8, 6), blocker(12, 12, walls.clone())).unwrap();
        let a = smooth_path(&path, blocker(12, 12, walls.clone()));
        let b = smooth_path(&path, blocker(12, 12, walls));
        assert_eq!(a, b);
    }

    #[test]
    fn test_step_toward_adjacent_returns_goal() {
        let step = step_toward((0, 0), (1, 0), |_, _| false);
        assert_eq!(step, Some((1, 0)));
    }

    #[test]
    fn test_step_toward_same_position_returns_none() {
        let step = step_toward((3, 3), (3, 3), |_, _| false);
        assert_eq!(step, None);
    }

    #[test]
    fn test_step_toward_no_path_returns_none() {
        // Surround start with walls on all 8 neighbours — A* terminates immediately.
        let walls: std::collections::HashSet<(i32, i32)> = [
            (-1, 0),
            (1, 0),
            (0, -1),
            (0, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
            (1, 1),
        ]
        .into();
        let step = step_toward((0, 0), (5, 5), |x, y| walls.contains(&(x, y)));
        assert_eq!(step, None);
    }

    #[test]
    fn test_step_toward_moves_one_cell_closer() {
        // Open field: step should decrease Chebyshev distance by 1.
        let from = (0, 0);
        let goal = (5, 5);
        let step = step_toward(from, goal, |_, _| false).unwrap();
        let before = (from.0 - goal.0).abs().max((from.1 - goal.1).abs());
        let after = (step.0 - goal.0).abs().max((step.1 - goal.1).abs());
        assert!(after < before, "step {step:?} did not get closer");
    }

    #[test]
    fn test_step_toward_is_deterministic() {
        let a = step_toward((0, 0), (5, 3), |_, _| false);
        let b = step_toward((0, 0), (5, 3), |_, _| false);
        assert_eq!(a, b);
    }

    // --- octile_distance ---

    #[test]
    fn test_octile_distance_orthogonal() {
        // 5 cells right: 5 orthogonal steps.
        assert_eq!(octile_distance((0, 0), (5, 0)), 5 * COST_ORTHO);
    }

    #[test]
    fn test_octile_distance_diagonal() {
        // 4 cells diagonal: 4 diagonal steps.
        assert_eq!(octile_distance((0, 0), (4, 4)), 4 * COST_DIAG);
    }

    #[test]
    fn test_octile_distance_zero_and_matches_open_astar() {
        assert_eq!(octile_distance((3, 3), (3, 3)), 0);
        // On an open grid the heuristic equals the actual optimal path cost.
        let path = astar((0, 0), (8, 5), blocker(15, 15, HashSet::new())).unwrap();
        assert_eq!(octile_distance((0, 0), (8, 5)), path_cost(&path));
    }

    #[test]
    fn test_path_cost_empty_and_single_is_zero() {
        assert_eq!(path_cost(&[]), 0);
        assert_eq!(path_cost(&[(3, 3)]), 0);
    }

    #[test]
    fn test_path_cost_orthogonal_steps() {
        // Three cells right: 2 orthogonal steps = 2 * 10 = 20.
        let path = [(0i32, 0i32), (1, 0), (2, 0)];
        assert_eq!(path_cost(&path), 20);
    }

    #[test]
    fn test_path_cost_matches_astar_path() {
        // A* on an open grid: path cost equals octile_distance(start, goal).
        let start = (0i32, 0i32);
        let goal = (3i32, 4i32);
        let path = astar(start, goal, blocker(15, 15, HashSet::new())).unwrap();
        assert_eq!(path_cost(&path), octile_distance(start, goal));
    }

    #[test]
    fn test_is_reachable_open_grid() {
        let start = (0, 0);
        let goal = (5, 5);
        assert!(is_reachable(start, goal, blocker(20, 20, HashSet::new())));
    }

    #[test]
    fn test_is_reachable_blocked_path() {
        let start = (0, 0);
        let goal = (5, 0);
        // Full-height wall at x=2 — spans the entire grid so A* cannot go around.
        let wall: HashSet<(i32, i32)> = (0..20).map(|y| (2, y)).collect();
        assert!(!is_reachable(start, goal, blocker(20, 20, wall)));
    }

    #[test]
    fn test_is_reachable_same_point() {
        assert!(is_reachable(
            (3, 3),
            (3, 3),
            blocker(10, 10, HashSet::new())
        ));
    }

    #[test]
    fn test_flood_fill_open_area() {
        let cells = flood_fill((5, 5), 2, |_x, _y| false);
        // All cells within Chebyshev 2 of (5,5) = (2*2+1)^2 = 25 cells.
        assert_eq!(cells.len(), 25);
        for &(x, y) in &cells {
            let dx = (x - 5).abs();
            let dy = (y - 5).abs();
            assert!(dx.max(dy) <= 2);
        }
    }

    #[test]
    fn test_flood_fill_blocked_start_returns_empty() {
        let cells = flood_fill((0, 0), 5, |x, y| x == 0 && y == 0);
        assert!(cells.is_empty());
    }

    #[test]
    fn test_flood_fill_zero_max_dist_is_just_start() {
        let cells = flood_fill((3, 7), 0, |_x, _y| false);
        assert_eq!(cells, vec![(3, 7)]);
    }

    // --- is_path_clear ---

    #[test]
    fn test_is_path_clear_no_blocked_cells_returns_true() {
        let path = [(0i32, 0i32), (1, 0), (2, 0), (3, 0)];
        assert!(is_path_clear(&path, |_, _| false));
    }

    #[test]
    fn test_is_path_clear_one_blocked_cell_returns_false() {
        let path = [(0i32, 0i32), (1, 0), (2, 0), (3, 0)];
        assert!(!is_path_clear(&path, |x, y| x == 2 && y == 0));
    }

    #[test]
    fn test_is_path_clear_empty_path_returns_true() {
        assert!(is_path_clear(&[], |_, _| false));
    }

    // --- nearest_reachable --------------------------------------------------

    #[test]
    fn test_nearest_reachable_finds_first_matching_cell() {
        // Open 10×10 grid; start = (0,0); target = (3,3) and (7,7).
        let pass = blocker(10, 10, HashSet::new());
        // Find first cell with x==y==3; BFS should reach it before (7,7).
        let result = nearest_reachable((0, 0), |x, y| !pass(x, y), |x, y| x == 3 && y == 3);
        assert_eq!(result, Some((3, 3)));
    }

    #[test]
    fn test_nearest_reachable_impassable_start_returns_none() {
        // Start cell is itself blocked → None immediately.
        let result = nearest_reachable(
            (5, 5),
            |x, y| !(x == 5 && y == 5), // (5,5) is impassable
            |_x, _y| true,
        );
        assert_eq!(result, None);
    }

    #[test]
    fn test_nearest_reachable_no_match_returns_none() {
        // A 3×3 open grid; pred is never true → None.
        let result = nearest_reachable(
            (1, 1),
            |x, y| x >= 0 && y >= 0 && x < 3 && y < 3,
            |_x, _y| false,
        );
        assert_eq!(result, None);
    }

    // --- auto_explore ---

    #[test]
    fn test_auto_explore_paths_to_nearest_frontier() {
        // A 1×5 explored corridor at y=0, x in 0..=3 explored; x=4 unexplored.
        // From (0,0) the nearest frontier is (3,0) — it borders unexplored (4,0).
        let bounds = |x: i32, y: i32| !(0..5).contains(&x) || y != 0;
        let explored = |x: i32, _y: i32| (0..4).contains(&x); // 0,1,2,3 seen; 4 unknown
        let path = auto_explore((0, 0), bounds, explored).expect("a frontier exists");
        assert_eq!(path.first(), Some(&(0, 0)), "path starts at the player");
        assert_eq!(path.last(), Some(&(3, 0)), "path ends at the frontier cell");
    }

    #[test]
    fn test_auto_explore_done_when_fully_explored() {
        // Everything in bounds is explored → no frontier → None.
        let bounds = |x: i32, y: i32| !(0..3).contains(&x) || !(0..3).contains(&y);
        let explored = |x: i32, y: i32| (0..3).contains(&x) && (0..3).contains(&y);
        assert_eq!(auto_explore((1, 1), bounds, explored), None);
    }

    #[test]
    fn test_auto_explore_start_on_frontier_is_singleton() {
        // (0,0) itself borders the unexplored (1,0): path is just [start].
        let bounds = |_x: i32, _y: i32| false; // nothing blocked
        let explored = |x: i32, y: i32| x == 0 && y == 0; // only the start is seen
        let path = auto_explore((0, 0), bounds, explored).expect("start is a frontier");
        assert_eq!(path, vec![(0, 0)]);
    }

    #[test]
    fn test_auto_explore_impassable_start_returns_none() {
        let bounds = |_x: i32, _y: i32| true; // everything blocked
        let explored = |_x: i32, _y: i32| true;
        assert_eq!(auto_explore((0, 0), bounds, explored), None);
    }

    #[test]
    fn test_auto_explore_path_stays_in_explored_passable_cells() {
        // 5×5 grid, all explored except a frontier; a wall column at x=2 except
        // y=4 forces the route around it. Verify every path cell is legal.
        let walls = HashSet::from([(2, 0), (2, 1), (2, 2), (2, 3)]);
        let bounds = blocker(5, 5, walls.clone());
        // Explored: all in-bounds cells except (4,4) which is the unknown.
        let explored =
            |x: i32, y: i32| (0..5).contains(&x) && (0..5).contains(&y) && (x, y) != (4, 4);
        let path = auto_explore((0, 0), &bounds, explored).expect("frontier near (4,4)");
        let check_blocked = blocker(5, 5, walls);
        for &(x, y) in &path {
            assert!(!check_blocked(x, y), "path cell ({x},{y}) must be passable");
        }
        // The frontier cell borders (4,4): it should be one of (3,3),(4,3),(3,4).
        let last = *path.last().unwrap();
        assert!(
            [(3, 3), (4, 3), (3, 4)].contains(&last),
            "frontier {last:?} should border the unexplored (4,4)"
        );
    }

    // --- path_to_direction_vec ---

    #[test]
    fn test_path_to_direction_vec_right_then_down() {
        let path = vec![(0, 0), (1, 0), (2, 0), (2, 1)];
        let dirs = path_to_direction_vec(&path);
        assert_eq!(dirs, vec![(1, 0), (1, 0), (0, 1)]);
    }

    #[test]
    fn test_path_to_direction_vec_single_point_returns_empty() {
        let dirs = path_to_direction_vec(&[(3, 5)]);
        assert!(dirs.is_empty());
    }

    #[test]
    fn test_path_to_direction_vec_length_is_path_len_minus_one() {
        let path = vec![(0, 0), (1, 1), (2, 0), (3, 1)];
        assert_eq!(path_to_direction_vec(&path).len(), path.len() - 1);
    }

    // --- jps (Jump Point Search) -------------------------------------------

    /// Validate that a JPS path is a legal, contiguous, corner-safe route from
    /// `start` to `goal` under `walls` on a `w×h` grid.
    fn assert_valid_path(
        path: &[(i32, i32)],
        start: (i32, i32),
        goal: (i32, i32),
        w: i32,
        h: i32,
        walls: &HashSet<(i32, i32)>,
    ) {
        assert_eq!(path.first(), Some(&start), "path must start at start");
        assert_eq!(path.last(), Some(&goal), "path must end at goal");
        let blk = |x: i32, y: i32| x < 0 || y < 0 || x >= w || y >= h || walls.contains(&(x, y));
        for &(x, y) in path {
            assert!(!blk(x, y), "path steps onto a blocked cell ({x},{y})");
        }
        for win in path.windows(2) {
            let (ax, ay) = win[0];
            let (bx, by) = win[1];
            let (dx, dy) = ((bx - ax).abs(), (by - ay).abs());
            assert!(dx <= 1 && dy <= 1 && (dx + dy) > 0, "non-adjacent step");
            if dx == 1 && dy == 1 {
                // No corner cutting: both shared orthogonal cells must be clear.
                assert!(
                    !blk(bx, ay) && !blk(ax, by),
                    "diagonal step cut a wall corner ({ax},{ay})->({bx},{by})"
                );
            }
        }
    }

    #[test]
    fn test_jps_open_grid_matches_astar_cost() {
        let b = blocker(15, 15, HashSet::new());
        let a = astar((0, 0), (10, 7), b).unwrap();
        let j = jps((0, 0), (10, 7), blocker(15, 15, HashSet::new())).unwrap();
        assert_eq!(path_cost(&a), path_cost(&j));
        assert_eq!(j.first(), Some(&(0, 0)));
        assert_eq!(j.last(), Some(&(10, 7)));
    }

    #[test]
    fn test_jps_start_equals_goal() {
        let j = jps((3, 3), (3, 3), blocker(10, 10, HashSet::new())).unwrap();
        assert_eq!(j, vec![(3, 3)]);
    }

    #[test]
    fn test_jps_blocked_endpoints_return_none() {
        let walls = HashSet::from([(2, 2)]);
        assert!(jps((2, 2), (5, 5), blocker(10, 10, walls.clone())).is_none());
        assert!(jps((5, 5), (2, 2), blocker(10, 10, walls)).is_none());
    }

    #[test]
    fn test_jps_unreachable_goal_returns_none() {
        let mut walls = HashSet::new();
        for x in 7..=9 {
            for y in 7..=9 {
                if (x, y) != (9, 9) {
                    walls.insert((x, y));
                }
            }
        }
        assert!(jps((0, 0), (9, 9), blocker(10, 10, walls)).is_none());
    }

    #[test]
    fn test_jps_respects_no_corner_cutting() {
        // Block E and S of the start; the SE diagonal would cut the corner.
        let walls = HashSet::from([(4, 3), (3, 4)]);
        let path = jps((3, 3), (4, 4), blocker(8, 8, walls.clone())).unwrap();
        assert_valid_path(&path, (3, 3), (4, 4), 8, 8, &walls);
        let took_corner = path.windows(2).any(|w| w[0] == (3, 3) && w[1] == (4, 4));
        assert!(!took_corner, "JPS must not cut the wall corner");
    }

    #[test]
    fn test_jps_is_deterministic() {
        let walls = HashSet::from([(4, 2), (4, 3), (4, 4), (2, 6)]);
        let a = jps((1, 1), (8, 8), blocker(12, 12, walls.clone()));
        let b = jps((1, 1), (8, 8), blocker(12, 12, walls));
        assert_eq!(a, b);
        assert!(a.is_some());
    }

    /// **Metamorphic correctness oracle**: over thousands of random grids, JPS
    /// must agree with A* on reachability, return the same optimal cost, and
    /// produce a legal corner-safe path. A* is the trusted reference; any
    /// forced-neighbour or pruning bug surfaces here as a cost/None mismatch.
    #[test]
    fn test_jps_matches_astar_over_random_grids() {
        // Tiny deterministic PRNG (no external deps; independent of the kit RNG
        // so this test stands alone).
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut compared = 0u32;
        for _ in 0..6000 {
            let w = 4 + (next() % 9) as i32; // 4..=12
            let h = 4 + (next() % 9) as i32;
            // Wall density 0..=45%.
            let density = next() % 46;
            let mut walls = HashSet::new();
            for y in 0..h {
                for x in 0..w {
                    if next() % 100 < density {
                        walls.insert((x, y));
                    }
                }
            }
            let start = ((next() % w as u64) as i32, (next() % h as u64) as i32);
            let goal = ((next() % w as u64) as i32, (next() % h as u64) as i32);
            walls.remove(&start);
            walls.remove(&goal);

            let a = astar(start, goal, blocker(w, h, walls.clone()));
            let j = jps(start, goal, blocker(w, h, walls.clone()));

            assert_eq!(
                a.is_some(),
                j.is_some(),
                "reachability mismatch: start={start:?} goal={goal:?} w={w} h={h}"
            );
            if let (Some(ap), Some(jp)) = (&a, &j) {
                assert_eq!(
                    path_cost(ap),
                    path_cost(jp),
                    "cost mismatch start={start:?} goal={goal:?}: astar={} jps={}",
                    path_cost(ap),
                    path_cost(jp)
                );
                assert_valid_path(jp, start, goal, w, h, &walls);
            }
            compared += 1;
        }
        assert!(
            compared >= 6000,
            "expected 6000 comparisons, got {compared}"
        );
    }

    // --- jps4 (4-connected Jump Point Search) --------------------------------

    /// Validate that a jps4 path is a legal, contiguous, *cardinal-only* route
    /// from `start` to `goal` under `walls` on a `w×h` grid.
    fn assert_valid_path4(
        path: &[(i32, i32)],
        start: (i32, i32),
        goal: (i32, i32),
        w: i32,
        h: i32,
        walls: &HashSet<(i32, i32)>,
    ) {
        assert_eq!(path.first(), Some(&start), "path must start at start");
        assert_eq!(path.last(), Some(&goal), "path must end at goal");
        let blk = |x: i32, y: i32| x < 0 || y < 0 || x >= w || y >= h || walls.contains(&(x, y));
        for &(x, y) in path {
            assert!(!blk(x, y), "path steps onto a blocked cell ({x},{y})");
        }
        for win in path.windows(2) {
            let (ax, ay) = win[0];
            let (bx, by) = win[1];
            let (dx, dy) = ((bx - ax).abs(), (by - ay).abs());
            assert!(
                dx + dy == 1,
                "step must be exactly one cardinal move: ({ax},{ay})->({bx},{by})"
            );
        }
    }

    /// Reference oracle: plain BFS. On a 4-connected unit-cost grid BFS is
    /// trivially optimal, so its step count is ground truth for [`jps4`].
    fn bfs4_steps(
        start: (i32, i32),
        goal: (i32, i32),
        w: i32,
        h: i32,
        walls: &HashSet<(i32, i32)>,
    ) -> Option<usize> {
        let blk = |x: i32, y: i32| x < 0 || y < 0 || x >= w || y >= h || walls.contains(&(x, y));
        if blk(start.0, start.1) || blk(goal.0, goal.1) {
            return None;
        }
        let mut dist: HashMap<(i32, i32), usize> = HashMap::new();
        let mut queue = std::collections::VecDeque::new();
        dist.insert(start, 0);
        queue.push_back(start);
        while let Some((x, y)) = queue.pop_front() {
            let d = dist[&(x, y)];
            if (x, y) == goal {
                return Some(d);
            }
            for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                let n = (x + dx, y + dy);
                if !blk(n.0, n.1) && !dist.contains_key(&n) {
                    dist.insert(n, d + 1);
                    queue.push_back(n);
                }
            }
        }
        None
    }

    #[test]
    fn test_jps4_open_grid_l_path() {
        let path = jps4((0, 0), (7, 4), blocker(10, 10, HashSet::new())).unwrap();
        // Manhattan distance 11 → 12 cells, all cardinal steps.
        assert_eq!(path.len(), 12);
        assert_valid_path4(&path, (0, 0), (7, 4), 10, 10, &HashSet::new());
    }

    #[test]
    fn test_jps4_start_equals_goal() {
        let path = jps4((3, 3), (3, 3), blocker(10, 10, HashSet::new())).unwrap();
        assert_eq!(path, vec![(3, 3)]);
    }

    #[test]
    fn test_jps4_blocked_endpoints_return_none() {
        let walls = HashSet::from([(2, 2)]);
        assert!(jps4((2, 2), (5, 5), blocker(10, 10, walls.clone())).is_none());
        assert!(jps4((5, 5), (2, 2), blocker(10, 10, walls)).is_none());
    }

    #[test]
    fn test_jps4_unreachable_goal_returns_none() {
        // A diagonal-only gap: 8-connected search could squeeze through, a
        // 4-connected one cannot — this is exactly what distinguishes jps4.
        let walls: HashSet<(i32, i32)> = (0..10).map(|i| (i, 5)).collect(); // solid row
        assert!(jps4((0, 0), (0, 9), blocker(10, 10, walls.clone())).is_none());
        // The 8-connected jps agrees here (solid row blocks diagonals too),
        // but with a checkerboard gap they differ:
        let mut diag_walls = HashSet::new();
        for i in 0..10 {
            if i != 4 {
                diag_walls.insert((i, 5));
            }
        }
        diag_walls.insert((4, 4)); // block the straight approach above the gap
                                   // 4-connected: must enter (4,5) from (4,4) or (4,6) or sideways — (4,4)
                                   // blocked, (3,5)/(5,5) blocked → only from below; start is above, so
                                   // the only way down is through (4,5) itself, reachable solely via
                                   // (4,4) which is walled → unreachable.
        assert!(jps4((0, 0), (0, 9), blocker(10, 10, diag_walls.clone())).is_none());
        // Sanity-check against the BFS oracle.
        assert_eq!(bfs4_steps((0, 0), (0, 9), 10, 10, &diag_walls), None);
    }

    #[test]
    fn test_jps4_forced_neighbour_around_wall() {
        // A wall stub forces the path to detour; the detour corner is a forced
        // neighbour and must be discovered as a jump point.
        let walls: HashSet<(i32, i32)> = (1..=4).map(|y| (3, y)).collect();
        let path = jps4((0, 2), (6, 2), blocker(8, 8, walls.clone())).unwrap();
        assert_valid_path4(&path, (0, 2), (6, 2), 8, 8, &walls);
        let oracle = bfs4_steps((0, 2), (6, 2), 8, 8, &walls).unwrap();
        assert_eq!(path.len() - 1, oracle);
    }

    #[test]
    fn test_jps4_is_deterministic() {
        let walls = HashSet::from([(4, 2), (4, 3), (4, 4), (2, 6)]);
        let a = jps4((1, 1), (8, 8), blocker(12, 12, walls.clone()));
        let b = jps4((1, 1), (8, 8), blocker(12, 12, walls));
        assert_eq!(a, b);
        assert!(a.is_some());
    }

    /// **BFS oracle**: over thousands of random grids, jps4 must agree with
    /// plain BFS on reachability and return exactly the BFS-optimal step
    /// count, with every step a legal cardinal move. On 4-connected unit-cost
    /// grids BFS optimality is a theorem, so this is the strongest available
    /// machine check of jps4's jump conditions.
    #[test]
    fn test_jps4_matches_bfs_over_random_grids() {
        let mut state: u64 = 0x243F_6A88_85A3_08D3;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut compared = 0u32;
        for _ in 0..6000 {
            let w = 4 + (next() % 9) as i32; // 4..=12
            let h = 4 + (next() % 9) as i32;
            let density = next() % 46; // 0..=45% walls
            let mut walls = HashSet::new();
            for y in 0..h {
                for x in 0..w {
                    if next() % 100 < density {
                        walls.insert((x, y));
                    }
                }
            }
            let start = ((next() % w as u64) as i32, (next() % h as u64) as i32);
            let goal = ((next() % w as u64) as i32, (next() % h as u64) as i32);
            walls.remove(&start);
            walls.remove(&goal);

            let oracle = bfs4_steps(start, goal, w, h, &walls);
            let j = jps4(start, goal, blocker(w, h, walls.clone()));

            assert_eq!(
                oracle.is_some(),
                j.is_some(),
                "reachability mismatch: start={start:?} goal={goal:?} w={w} h={h} walls={walls:?}"
            );
            if let (Some(steps), Some(jp)) = (oracle, &j) {
                assert_eq!(
                    jp.len() - 1,
                    steps,
                    "cost mismatch start={start:?} goal={goal:?} w={w} h={h}: bfs={} jps4={} walls={walls:?}",
                    steps,
                    jp.len() - 1
                );
                assert_valid_path4(jp, start, goal, w, h, &walls);
            }
            compared += 1;
        }
        assert!(
            compared >= 6000,
            "expected 6000 comparisons, got {compared}"
        );
    }
}
