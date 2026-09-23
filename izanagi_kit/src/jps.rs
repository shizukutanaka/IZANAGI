//! Jump Point Search — uniform-cost grid pathfinding that skips
//! symmetries instead of expanding every cell.
//!
//! The walkable set is a packed bit grid (`grid[y] >> x & 1`). Movement
//! is octile; a diagonal step is legal whenever its target is open —
//! corner *touching* is allowed, matching the paper's original rules
//! (strict no-corner-cut changes reachability and breaks jump pruning).
//! Costs are integers —
//! orthogonal steps cost `STRAIGHT`, diagonal `DIAG` (`2`/`3`, the
//! classic `sqrt2`≈`1.5` rounding) — so the whole search is a pure
//! function of `(grid, start, goal)`.
//!
//! `jps` runs the Harabor–Grastien jump scheme: straight rays stop at
//! forced neighbours or the goal, diagonal rays first probe their two
//! component straight rays, and only jump points enter the priority
//! queue. The returned path is interpolated between jump points.
//!
//! ```
//! use izanagi_kit::jps::{self, DIAG, STRAIGHT};
//! // 5x5 room, pillar at (2,1).
//! let mut grid = vec![0b11111u64; 5];
//! grid[1] &= !(1 << 2);
//! let (path, cost) = jps::find(&grid, 5, 5, (0, 0), (4, 4)).unwrap();
//! assert_eq!(path.first(), Some(&(0, 0)));
//! assert_eq!(path.last(), Some(&(4, 4)));
//! assert_eq!(cost, 4 * DIAG); // detour around the pillar costs 4 diagonals
//! assert!(DIAG > STRAIGHT && DIAG < 2 * STRAIGHT);
//! ```
//!
//! Reference: Harabor & Grastien, "Online Graph Pruning for Pathfinding
//! on Grid Maps" (AAAI 2011).

/// Orthogonal step cost.
pub const STRAIGHT: u64 = 2;
/// Diagonal step cost.
pub const DIAG: u64 = 3;

#[inline]
fn open(grid: &[u64], w: usize, x: i64, y: i64) -> bool {
    x >= 0
        && y >= 0
        && (x as usize) < w
        && (y as usize) < grid.len()
        && (grid[y as usize] >> x & 1) == 1
}

/// A diagonal step `(x,y) -> (x+dx, y+dy)` under the paper's rules:
/// legal whenever the target is open (corner touching allowed).
#[inline]
fn diag_ok(grid: &[u64], w: usize, x: i64, y: i64, dx: i64, dy: i64) -> bool {
    open(grid, w, x + dx, y + dy)
}

/// Directions the jump point at `pos` may scan, given `dir` the step by
/// which it was reached ((0,0) for the start). Returns up to 8
/// `(dx, dy)` directions — the pruned neighbour set of the paper.
fn pruned_dirs(
    grid: &[u64],
    w: usize,
    x: i64,
    y: i64,
    dx: i64,
    dy: i64,
    out: &mut Vec<(i64, i64)>,
) {
    out.clear();
    if dx == 0 && dy == 0 {
        // Start cell: every legal direction.
        for d in DIRS {
            let (ddx, ddy) = d;
            if is_step_ok(grid, w, x, y, ddx, ddy) {
                out.push(d);
            }
        }
        return;
    }
    if dx != 0 && dy != 0 {
        // Diagonal arrival: naturals are the two straights + the
        // diagonal itself.
        for d in [(dx, 0), (0, dy), (dx, dy)] {
            if is_step_ok(grid, w, x, y, d.0, d.1) {
                out.push(d);
            }
        }
        // Forced: a blocked orthogonal neighbour routes the detour
        // through this cell.
        if !open(grid, w, x - dx, y) && diag_ok(grid, w, x, y, -dx, dy) {
            out.push((-dx, dy));
        }
        if !open(grid, w, x, y - dy) && diag_ok(grid, w, x, y, dx, -dy) {
            out.push((dx, -dy));
        }
    } else {
        // Straight arrival: natural is continuing straight.
        if is_step_ok(grid, w, x, y, dx, dy) {
            out.push((dx, dy));
        }
        // Forced diagonals around blocked side cells.
        if dx != 0 {
            if !open(grid, w, x, y - 1) && diag_ok(grid, w, x, y, dx, -1) {
                out.push((dx, -1));
            }
            if !open(grid, w, x, y + 1) && diag_ok(grid, w, x, y, dx, 1) {
                out.push((dx, 1));
            }
        } else {
            if !open(grid, w, x - 1, y) && diag_ok(grid, w, x, y, -1, dy) {
                out.push((-1, dy));
            }
            if !open(grid, w, x + 1, y) && diag_ok(grid, w, x, y, 1, dy) {
                out.push((1, dy));
            }
        }
    }
}

const DIRS: [(i64, i64); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
];

#[inline]
fn is_step_ok(grid: &[u64], w: usize, x: i64, y: i64, dx: i64, dy: i64) -> bool {
    if dx != 0 && dy != 0 {
        diag_ok(grid, w, x, y, dx, dy)
    } else {
        open(grid, w, x + dx, y + dy)
    }
}

/// Jump in direction `(dx,dy)` from `(x,y)`. Returns the first cell
/// that must be treated as a node — the goal, a forced-neighbour
/// carrier, or a cell whose orthogonal probes hit a node — or `None`
/// when the ray leaves the walkable set.
fn jump(
    grid: &[u64],
    w: usize,
    goal: (i64, i64),
    x: i64,
    y: i64,
    dx: i64,
    dy: i64,
) -> Option<(i64, i64)> {
    let (nx, ny) = (x + dx, y + dy);
    if !is_step_ok(grid, w, x, y, dx, dy) {
        return None;
    }
    let (nx, ny) = (nx, ny);
    if (nx, ny) == goal {
        return Some((nx, ny));
    }
    if dx != 0 && dy != 0 {
        // Diagonal: any forced neighbour makes this a jump point.
        if (!open(grid, w, nx - dx, ny) && open(grid, w, nx - dx, ny + dy))
            || (!open(grid, w, nx, ny - dy) && open(grid, w, nx + dx, ny - dy))
        {
            return Some((nx, ny));
        }
        // Component probes: if either straight ray hits a node, stop.
        if jump(grid, w, goal, nx, ny, dx, 0).is_some()
            || jump(grid, w, goal, nx, ny, 0, dy).is_some()
        {
            return Some((nx, ny));
        }
    } else {
        // Straight: forced diagonal neighbours make this a jump point.
        let f1 = if dx != 0 {
            (!open(grid, w, nx, ny - 1) && open(grid, w, nx + dx, ny - 1))
                || (!open(grid, w, nx, ny + 1) && open(grid, w, nx + dx, ny + 1))
        } else {
            (!open(grid, w, nx - 1, ny) && open(grid, w, nx - 1, ny + dy))
                || (!open(grid, w, nx + 1, ny) && open(grid, w, nx + 1, ny + dy))
        };
        if f1 {
            return Some((nx, ny));
        }
    }
    jump(grid, w, goal, nx, ny, dx, dy)
}

/// A* heuristic under the (STRAIGHT, DIAG) cost model — admissible and
/// consistent: `S·(dx+dy) + (D−2S)·min(dx,dy)`.
#[inline]
fn heuristic(x: i64, y: i64, goal: (i64, i64)) -> u64 {
    let dx = (x - goal.0).unsigned_abs();
    let dy = (y - goal.1).unsigned_abs();
    let (mn, mx) = (dx.min(dy), dx.max(dy));
    STRAIGHT * (mx - mn) + DIAG * mn
}

/// JPS search over a packed row-major bit grid. Returns the canonical
/// optimal path (cell list including both endpoints) and its cost, or
/// `None` when `goal` is closed or unreachable. Pure function of the
/// inputs — expansion order is fixed.
pub fn find(
    grid: &[u64],
    w: usize,
    h: usize,
    start: (u32, u32),
    goal: (u32, u32),
) -> Option<(Vec<(u32, u32)>, u64)> {
    let h = h.min(grid.len());
    let (sx, sy) = (start.0 as i64, start.1 as i64);
    let (gx, gy) = (goal.0 as i64, goal.1 as i64);
    if !open(&grid[..h], w, sx, sy) || !open(&grid[..h], w, gx, gy) {
        return None;
    }
    if (sx, sy) == (gx, gy) {
        return Some((vec![start], 0));
    }
    use std::collections::{BTreeMap, BTreeSet};
    // Uniform-cost search over jump points; expansion order is
    // (f, g, x, y) — fully canonical.
    let mut g: BTreeMap<(i64, i64), u64> = BTreeMap::new();
    let mut parent: BTreeMap<(i64, i64), (i64, i64)> = BTreeMap::new();
    // Open set as ordered (f, g, x, y) tuples — no float keys.
    let mut open: BTreeSet<(u64, u64, i64, i64)> = BTreeSet::new();
    g.insert((sx, sy), 0);
    open.insert((heuristic(sx, sy, (gx, gy)), 0, sx, sy));
    // We need the incoming direction per node to prune successors;
    // store it alongside parents.
    let mut dirs: BTreeMap<(i64, i64), (i64, i64)> = BTreeMap::new();
    let mut scratch: Vec<(i64, i64)> = Vec::with_capacity(8);
    while let Some(&(_, gc, x, y)) = open.iter().next() {
        open.remove(&(heuristic(x, y, (gx, gy)) + gc, gc, x, y));
        // Remove all stale entries for (x,y)? Canonical set keeps
        // exactly one entry per settled/queued node — see insert below.
        if (x, y) == (gx, gy) {
            // Reconstruct + interpolate.
            let mut jp = vec![(x, y)];
            let mut cur = (x, y);
            while let Some(&p) = parent.get(&cur) {
                jp.push(p);
                cur = p;
            }
            jp.reverse();
            let mut path: Vec<(u32, u32)> = Vec::new();
            for i in 0..jp.len() {
                if i > 0 {
                    interpolate(jp[i - 1], jp[i], &mut path);
                } else {
                    path.push((jp[0].0 as u32, jp[0].1 as u32));
                }
            }
            return Some((path, gc));
        }
        let (pdx, pdy) = dirs.get(&(x, y)).copied().unwrap_or((0, 0));
        pruned_dirs(&grid[..h], w, x, y, pdx, pdy, &mut scratch);
        for &(ddx, ddy) in scratch.iter() {
            // Walk the ray to its jump point.
            if let Some((jx, jy)) = jump(&grid[..h], w, (gx, gy), x, y, ddx, ddy) {
                let (adx, ady) = (jx - x, jy - y);
                let steps = adx.abs().max(ady.abs()) as u64;
                let diag_steps = adx.abs().min(ady.abs()) as u64;
                let cost = DIAG * diag_steps + STRAIGHT * (steps - diag_steps);
                let ng = gc + cost;
                if ng < *g.get(&(jx, jy)).unwrap_or(&u64::MAX) {
                    g.insert((jx, jy), ng);
                    parent.insert((jx, jy), (x, y));
                    dirs.insert((jx, jy), (ddx, ddy));
                    open.insert((ng + heuristic(jx, jy, (gx, gy)), ng, jx, jy));
                }
            }
        }
    }
    None
}

/// Fill `path` with the cells strictly after `a` up to and including
/// `b` — `a`→`b` is always a straight or diagonal ray.
fn interpolate(a: (i64, i64), b: (i64, i64), path: &mut Vec<(u32, u32)>) {
    let dx = (b.0 - a.0).signum();
    let dy = (b.1 - a.1).signum();
    let (mut x, mut y) = a;
    while (x, y) != b {
        x += dx;
        y += dy;
        path.push((x as u32, y as u32));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Reference optimum: Dijkstra over the plain 8-dir grid with the
    /// same corner rule and costs — the oracle.
    fn oracle(grid: &[u64], w: usize, h: usize, s: (u32, u32), t: (u32, u32)) -> Option<u64> {
        use std::collections::{BTreeMap, BTreeSet};
        let mut dist: BTreeMap<(i64, i64), u64> = BTreeMap::new();
        let mut open: BTreeSet<(u64, i64, i64)> = BTreeSet::new();
        let (sx, sy) = (s.0 as i64, s.1 as i64);
        if !open_cell(grid, w, h, sx, sy) || !open_cell(grid, w, h, t.0 as i64, t.1 as i64) {
            return None;
        }
        dist.insert((sx, sy), 0);
        open.insert((0, sx, sy));
        while let Some(&(d, x, y)) = open.iter().next() {
            open.remove(&(d, x, y));
            if (x, y) == (t.0 as i64, t.1 as i64) {
                return Some(d);
            }
            for &(dx, dy) in DIRS.iter() {
                if !is_step_ok(&grid[..h], w, x, y, dx, dy) {
                    continue;
                }
                let c = if dx != 0 && dy != 0 { DIAG } else { STRAIGHT };
                let nd = d + c;
                let (nx, ny) = (x + dx, y + dy);
                if nd < *dist.get(&(nx, ny)).unwrap_or(&u64::MAX) {
                    dist.insert((nx, ny), nd);
                    open.insert((nd, nx, ny));
                }
            }
        }
        None
    }

    fn open_cell(grid: &[u64], w: usize, h: usize, x: i64, y: i64) -> bool {
        x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h && (grid[y as usize] >> x & 1) == 1
    }

    fn validate(grid: &[u64], w: usize, path: &[(u32, u32)], cost: u64) {
        // Every step is a legal octile move under the corner rule;
        // summed cost matches.
        let mut total = 0;
        for k in 1..path.len() {
            let (ax, ay) = (path[k - 1].0 as i64, path[k - 1].1 as i64);
            let (bx, by) = (path[k].0 as i64, path[k].1 as i64);
            let (dx, dy) = (bx - ax, by - ay);
            assert!(dx.abs() <= 1 && dy.abs() <= 1 && (dx != 0 || dy != 0));
            if dx != 0 && dy != 0 {
                total += DIAG;
            } else {
                total += STRAIGHT;
            }
            assert!(open(grid, w, bx, by));
        }
        assert_eq!(total, cost);
    }

    #[test]
    fn basics() {
        // Open 4x4: straight line.
        let grid = vec![0b1111u64; 4];
        let (path, cost) = find(&grid, 4, 4, (0, 0), (3, 0)).unwrap();
        assert_eq!(path, vec![(0, 0), (1, 0), (2, 0), (3, 0)]);
        assert_eq!(cost, 3 * STRAIGHT);
        // Diagonal across open space.
        let (path, cost) = find(&grid, 4, 4, (0, 0), (3, 3)).unwrap();
        assert_eq!(cost, 3 * DIAG);
        assert_eq!(path.len(), 4);
        // Blocked goal.
        let mut g2 = grid.clone();
        g2[3] = 0;
        assert!(find(&g2, 4, 4, (0, 0), (3, 3)).is_none());
        // Start == goal.
        let (p, c) = find(&grid, 4, 4, (1, 1), (1, 1)).unwrap();
        assert_eq!((p, c), (vec![(1, 1)], 0));
    }

    #[test]
    fn forced_detour() {
        // Corridor: wall row forces a forced-neighbour jump.
        // ....
        // .##.
        // ....
        let mut grid = vec![0b1111u64; 3];
        grid[1] = 0b1001;
        let (path, cost) = find(&grid, 4, 3, (0, 1), (3, 1)).unwrap();
        validate(&grid, 4, &path, cost);
        // Optimal is (0,1)->(1,0)->(2,0)->(3,1): D,S,D = 8.
        assert_eq!(cost, STRAIGHT + 2 * DIAG);
    }

    #[test]
    fn oracle_random() {
        let mut rng = SplitMix64::new(0x3a3c_e55a_11d1_9a3f);
        for _case in 0..60 {
            let w = 1 + rng.below(12) as usize;
            let h = 1 + rng.below(10) as usize;
            let mut grid = vec![0u64; h];
            for row in grid.iter_mut() {
                for x in 0..w {
                    if rng.below(3) > 0 {
                        *row |= 1 << x;
                    }
                }
            }
            let s = (rng.below(w as u32), rng.below(h as u32));
            let t = (rng.below(w as u32), rng.below(h as u32));
            let expect = oracle(&grid, w, h, s, t);
            match find(&grid, w, h, s, t) {
                None => assert!(expect.is_none(), "JPS missed a path oracle found"),
                Some((path, cost)) => {
                    assert_eq!(Some(cost), expect);
                    assert_eq!(path.first().copied(), Some(s));
                    assert_eq!(path.last().copied(), Some(t));
                    validate(&grid, w, &path, cost);
                }
            }
        }
    }
}
