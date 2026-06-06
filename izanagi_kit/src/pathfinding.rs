//! Deterministic A* pathfinding on a grid.
//!
//! Roguelikes need shortest paths for monster AI, auto-explore and travel. This
//! is 8-directional A* with **integer** octile costs (10 orthogonal, 14 diagonal
//! — a fixed-point stand-in for the 1 : √2 ratio) and the octile heuristic, so
//! it is admissible, consistent (optimal paths) and free of floating point.
//!
//! Determinism: the open set is ordered by the *total* key `(f, h, x, y)` — the
//! `(x, y)` tail makes every key unique, so there are no ties for the heap to
//! break arbitrarily and the popped order is identical on every run and target.
//! Neighbours are expanded in a fixed compass order. (See `bracket-lib`'s
//! `bracket-pathfinding` and libtcod for the same algorithm family.)
//!
//! Diagonal moves never cut a wall corner: moving diagonally requires both
//! shared orthogonal cells to be clear.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

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
}
