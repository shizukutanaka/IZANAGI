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
    let dx = (a.0 - b.0).abs();
    let dy = (a.1 - b.1).abs();
    COST_ORTHO * (dx + dy) - (2 * COST_ORTHO - COST_DIAG) * dx.min(dy)
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
/// Path cost uses [`COST_ORTHO`]/[`COST_DIAG`]; the returned path is one of the
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
}
