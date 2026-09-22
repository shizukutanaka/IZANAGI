//! Perfect-maze generation via **Wilson's algorithm** — a loop-erased random
//! walk that samples *uniformly* from all spanning trees of the cell graph.
//!
//! D. B. Wilson, "Generating random spanning trees more quickly than the
//! cover time", *Proc. 28th ACM STOC*, 1996 — uniformly random spanning
//! trees via loop-erased random walks. (Also Buckblog, "Maze Generation:
//! Wilson's algorithm", 2011 — the readable procedural description.)
//!
//! Contrast with [`crate::mapgen`]'s drunkard's-walk carver: Wilson mazes are
//! *perfect* — exactly one path between any two cells — and unbiased, where
//! drunken carving leaves irregular cave blobs.
//!
//! Grid: a `w × h` maze renders into a `(2w+1) × (2h+1)` [`Dungeon`] — odd
//! tiles are cells, even tiles are walls, and the tile between two connected
//! cells is carved floor. Fully deterministic under `rng`: start cells are
//! picked in row-major order (any sequence yields a uniform spanning tree)
//! and each walk direction draws `rng.below(4)` — the same draws replay
//! identically.
//!
//! ```
//! use izanagi_kit::maze::wilson_maze;
//! use izanagi_kit::rng::SplitMix64;
//!
//! let mut rng = SplitMix64::new(7);
//! let dungeon = wilson_maze(10, 6, &mut rng);
//! assert_eq!(dungeon.width(), 21);
//! ```

use crate::mapgen::Dungeon;
use crate::rng::SplitMix64;

/// Four cardinal walk directions: N, E, S, W.
const CARDINAL: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

/// Generate a `w × h`-cell perfect maze with Wilson's algorithm, rendered
/// into a `(2w+1) × (2h+1)` walled [`Dungeon`].
///
/// `w` and `h` are the *logical cell* dimensions; each cell becomes a floor
/// tile at `(2·cx+1, 2·cy+1)`, and the wall tile between two tree-connected
/// cells is carved too. Returns `Dungeon::new_walled(1, 1)`-style all-wall
/// output for `w == 0 || h == 0` (a `1×1` solid dungeon).
///
/// Determinism: identical `w`, `h`, and rng state give byte-identical
/// dungeons.
pub fn wilson_maze(w: u32, h: u32, rng: &mut SplitMix64) -> Dungeon {
    let mut d = Dungeon::new_walled(2 * w + 1, 2 * h + 1);
    let cells = (w * h) as usize;
    if cells == 0 {
        return d;
    }
    // Map a cell coordinate to its linear index.
    let idx = |x: i32, y: i32| (y * w as i32 + x) as usize;

    let mut in_tree = vec![false; cells];
    let mut remaining = cells;

    // Cell (0,0) seeds the tree — any seed yields a uniform UST.
    in_tree[0] = true;
    d.carve_floor(1, 1);
    remaining -= 1;

    // Scratch state for the current walk (allocated once).
    let mut step_of = vec![None; cells]; // step index in `walk`, or None
    let mut walk: Vec<(usize, i32, i32)> = Vec::new(); // (cell idx, x, y)

    while remaining > 0 {
        // Start at the first unvisited cell in row-major order.
        let (sx, sy) = {
            let i = in_tree.iter().position(|&v| !v).unwrap_or(0);
            (i as i32 % w as i32, i as i32 / w as i32)
        };
        walk.clear();
        walk.push((idx(sx, sy), sx, sy));
        step_of[idx(sx, sy)] = Some(0);

        // Loop-erased random walk until it hits the tree.
        let (mut x, mut y) = (sx, sy);
        loop {
            let (dx, dy) = CARDINAL[rng.below(4) as usize];
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                continue; // off-grid: redraw
            }
            let ni = idx(nx, ny);
            if in_tree[ni] {
                // Walk connected: add each recorded step to the tree and
                // carve its cell plus the corridor tile linking forward.
                // Corridor tile between cell (px,py) and adjacent cell
                // (cx,cy) sits at (px + cx + 1, py + cy + 1) — one tile past
                // the previous cell's (2px+1, 2py+1) position.
                let mut px = sx;
                let mut py = sy;
                for &(ci, cx, cy) in &walk {
                    d.carve_floor(cx + px + 1, cy + py + 1);
                    d.carve_floor(2 * cx + 1, 2 * cy + 1);
                    in_tree[ci] = true;
                    step_of[ci] = None;
                    px = cx;
                    py = cy;
                }
                // Corridor into the tree cell the walk hit.
                d.carve_floor(px + nx + 1, py + ny + 1);
                remaining -= walk.len();
                break;
            }
            // Loop erasure: rewind to the earlier occurrence of this cell.
            if let Some(pos) = step_of[ni] {
                for (ci, _, _) in walk.drain(pos + 1..) {
                    step_of[ci] = None;
                }
                x = nx;
                y = ny;
                continue;
            }
            walk.push((ni, nx, ny));
            step_of[ni] = Some(walk.len() - 1);
            x = nx;
            y = ny;
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    /// BFS over floor tiles; returns (reachable count, internal edge count).
    fn floor_stats(d: &Dungeon) -> (usize, usize) {
        let w = d.width() as i32;
        let h = d.height() as i32;
        let mut seen = vec![false; (w * h) as usize];
        let mut frontier: Vec<(i32, i32)> = Vec::new();
        // Seed BFS at the first floor tile.
        'outer: for y in 0..h {
            for x in 0..w {
                if !d.is_wall(x, y) {
                    frontier.push((x, y));
                    seen[(y * w + x) as usize] = true;
                    break 'outer;
                }
            }
        }
        let mut reached = 0usize;
        let mut edges = 0usize;
        while let Some((x, y)) = frontier.pop() {
            reached += 1;
            for &(dx, dy) in &[(1, 0), (0, 1)] {
                // Count each edge once (right/down only).
                let (nx, ny) = (x + dx, y + dy);
                if nx < w && ny < h && !d.is_wall(nx, ny) {
                    edges += 1;
                }
            }
            for &(dx, dy) in &CARDINAL {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && nx < w && ny < h {
                    let i = (ny * w + nx) as usize;
                    if !d.is_wall(nx, ny) && !seen[i] {
                        seen[i] = true;
                        frontier.push((nx, ny));
                    }
                }
            }
        }
        (reached, edges)
    }

    /// Oracle: a `w×h` perfect maze rendered on the wall grid is a *tree* —
    /// connected, with exactly `2·w·h − 1` floor tiles and
    /// `floor_tiles − 1` adjacency edges (connected + E = V−1 ⟹ acyclic,
    /// exactly one path between any two cells).
    fn assert_perfect_maze(d: &Dungeon, w: u32, h: u32) {
        let floor_tiles = (2 * w * h) as usize - 1;
        let (reached, edges) = floor_stats(d);
        assert_eq!(reached, floor_tiles, "all floor tiles connected");
        assert_eq!(edges, floor_tiles - 1, "connected + E=V-1 makes it a tree");
    }

    #[test]
    fn maze_is_perfect() {
        for &(w, h) in &[(1, 1), (3, 3), (10, 6), (7, 11), (25, 15)] {
            let mut rng = SplitMix64::new(42);
            let d = wilson_maze(w, h, &mut rng);
            assert_eq!(d.width(), 2 * w + 1);
            assert_eq!(d.height(), 2 * h + 1);
            assert_perfect_maze(&d, w, h);
        }
    }

    #[test]
    fn deterministic_under_same_seed() {
        let mut a = SplitMix64::new(0xC0DE);
        let mut b = SplitMix64::new(0xC0DE);
        let da = wilson_maze(12, 8, &mut a);
        let db = wilson_maze(12, 8, &mut b);
        for y in 0..da.height() as i32 {
            for x in 0..da.width() as i32 {
                assert_eq!(da.is_wall(x, y), db.is_wall(x, y));
            }
        }
        // A different seed produces a different maze (overwhelmingly likely).
        let mut c = SplitMix64::new(0xC0DF);
        let dc = wilson_maze(12, 8, &mut c);
        let mut differs = false;
        for y in 0..dc.height() as i32 {
            for x in 0..dc.width() as i32 {
                if da.is_wall(x, y) != dc.is_wall(x, y) {
                    differs = true;
                }
            }
        }
        assert!(differs);
    }

    #[test]
    fn cell_corners_are_floor() {
        let mut rng = SplitMix64::new(1);
        let d = wilson_maze(4, 4, &mut rng);
        for cy in 0..4 {
            for cx in 0..4 {
                assert!(!d.is_wall(2 * cx + 1, 2 * cy + 1));
            }
        }
        // Outer border stays wall.
        for x in 0..9 {
            assert!(d.is_wall(x, 0));
            assert!(d.is_wall(x, 8));
        }
        for y in 0..9 {
            assert!(d.is_wall(0, y));
            assert!(d.is_wall(8, y));
        }
    }

    #[test]
    fn degenerate_sizes() {
        let mut rng = SplitMix64::new(3);
        let d = wilson_maze(0, 5, &mut rng);
        assert!(d.is_wall(0, 0));
        let d = wilson_maze(5, 0, &mut rng);
        assert!(d.is_wall(0, 0));
        // 1x1: a single carved cell inside walls.
        let d = wilson_maze(1, 1, &mut rng);
        assert!(!d.is_wall(1, 1));
        assert_eq!(floor_stats(&d), (1, 0));
    }

    #[test]
    fn maze_repeats_rng_draws_independently_of_calls() {
        // Two calls on the same rng interleave deterministically.
        let mut rng = SplitMix64::new(9);
        let d1 = wilson_maze(5, 5, &mut rng);
        let d2 = wilson_maze(5, 5, &mut rng);
        assert_ne!(d1, d2);
    }
}
