//! Flow field — one-source-to-all-agents pathfinding for
//! tower-defense swarms (the *Planetary Annihilation* /
//! *Supreme Commander* style). Two layers:
//!
//! 1. **Integration field**: Dijkstra distances from the goal set to
//!    every reachable cell, computed once per goal change. Terrain
//!    cost is per-cell: moving into a cell costs `cost[v]` (walls =
//!    `u16::MAX`).
//! 2. **Vector field**: each cell stores the direction of its
//!    lowest-distance neighbor — agents anywhere on the map follow
//!    a single lookup per step with zero per-agent search.
//!
//! Moves are 8-connected with **no corner cutting** (a diagonal is
//! legal only when both orthogonal cells it squeezes between are
//! passable). Orthogonal moves cost `2·cost`, diagonals `3·cost` —
//! an integer √2 ≈ 1.5 that keeps every distance exactly
//! representable; divide reported distances by 2 for per-cell-cost
//! units. Distances are a pure function of `(map, goals)` and
//! direction ties break to the lowest direction index, so all peers
//! build identical fields from identical inputs.
//!
//! ```
//! use izanagi_kit::flowfield::FlowField;
//! // 3x3 open grid, goal at bottom-right (8).
//! let cost = vec![1u16; 9];
//! let ff = FlowField::new(3, 3, &cost, &[8]).unwrap();
//! assert_eq!(ff.dist(0, 0), Some(6)); // two diagonals: 3 + 3
//! let path = ff.path(0, 0).unwrap();
//! assert_eq!(*path.last().unwrap(), (2, 2));
//! ```

use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Cell cost meaning "solid" — never entered, never a goal.
pub const WALL: u16 = u16::MAX;

/// The eight move directions, index-order canonical:
/// `(+1,0),(+1,+1),(0,+1),(-1,+1),(-1,0),(-1,-1),(0,-1),(+1,-1)` —
/// E, SE, S, SW, W, NW, N, NE. Diagonals need both adjacent
/// orthogonal cells free.
pub const DIRS: [(i8, i8); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

/// Integration + vector field for one goal set on one map.
#[derive(Clone, Debug)]
pub struct FlowField {
    w: usize,
    h: usize,
    /// 2·per-cell-cost units; `u32::MAX` = unreachable or wall.
    dist: Vec<u32>,
    /// Best move direction per cell — `u8::MAX` = none (goal cells,
    /// walls, unreachable).
    dir: Vec<u8>,
}

impl FlowField {
    /// Build the field for `cost` (row-major `w*h`, `WALL` = solid)
    /// toward `goals` (row-major cell indices). `None` when `cost` is
    /// the wrong length or a goal is out of bounds — a walled-in or
    /// wall goal still builds (it just stays unreachable).
    pub fn new(w: usize, h: usize, cost: &[u16], goals: &[u32]) -> Option<Self> {
        if cost.len() != w * h || w == 0 || h == 0 {
            return None;
        }
        let n = w * h;
        let mut dist = vec![u32::MAX; n];
        let mut heap = BinaryHeap::with_capacity(n);
        for &g in goals {
            if g as usize >= n {
                return None;
            }
            if cost[g as usize] == WALL {
                continue;
            }
            dist[g as usize] = 0;
            heap.push((Reverse(0u32), g as usize));
        }
        while let Some((Reverse(d), u)) = heap.pop() {
            if dist[u] != d {
                continue; // stale
            }
            let ux = (u % w) as i64;
            let uy = (u / w) as i64;
            for &(dx, dy) in &DIRS {
                let (vx, vy) = (ux + dx as i64, uy + dy as i64);
                if vx < 0 || vy < 0 || vx >= w as i64 || vy >= h as i64 {
                    continue;
                }
                let v = (vy as usize) * w + vx as usize;
                let vc = cost[v];
                if vc == WALL {
                    continue;
                }
                let step = if dx != 0 && dy != 0 {
                    // No corner cutting: both orthogonals must be free.
                    let a = cost[(uy as usize) * w + vx as usize];
                    let b = cost[(vy as usize) * w + ux as usize];
                    if a == WALL || b == WALL {
                        continue;
                    }
                    3u64 * vc as u64
                } else {
                    2u64 * vc as u64
                };
                let nd = d as u64 + step;
                if nd < dist[v] as u64 {
                    dist[v] = nd.min(u32::MAX as u64) as u32;
                    heap.push((Reverse(dist[v]), v));
                }
            }
        }
        // Vector field: lowest-dist neighbor, lowest index on ties.
        let mut dir = vec![u8::MAX; n];
        for u in 0..n {
            if dist[u] == u32::MAX || dist[u] == 0 {
                continue; // unreachable or on a goal
            }
            let ux = (u % w) as i64;
            let uy = (u / w) as i64;
            let mut best = u32::MAX;
            let mut best_d = u8::MAX;
            for (di, &(dx, dy)) in DIRS.iter().enumerate() {
                let (vx, vy) = (ux + dx as i64, uy + dy as i64);
                if vx < 0 || vy < 0 || vx >= w as i64 || vy >= h as i64 {
                    continue;
                }
                if dx != 0 && dy != 0 {
                    let a = cost[(uy as usize) * w + vx as usize];
                    let b = cost[(vy as usize) * w + ux as usize];
                    if a == WALL || b == WALL {
                        continue;
                    }
                }
                let v = (vy as usize) * w + vx as usize;
                if dist[v] < best {
                    best = dist[v];
                    best_d = di as u8;
                }
            }
            if best < dist[u] {
                dir[u] = best_d;
            }
        }
        Some(FlowField { w, h, dist, dir })
    }

    /// Field dimensions — `(width, height)`.
    pub fn dims(&self) -> (usize, usize) {
        (self.w, self.h)
    }

    /// Integration distance of `(x, y)` in 2·cost units — `None`
    /// for walls, unreachable cells, and out-of-bounds queries.
    pub fn dist(&self, x: usize, y: usize) -> Option<u32> {
        if x >= self.w || y >= self.h {
            return None;
        }
        let d = self.dist[y * self.w + x];
        (d != u32::MAX).then_some(d)
    }

    /// Direction `(dx, dy)` toward the goal at `(x, y)` — `None` on
    /// walls, unreachable cells, goals themselves, and out of bounds.
    pub fn direction(&self, x: usize, y: usize) -> Option<(i8, i8)> {
        if x >= self.w || y >= self.h {
            return None;
        }
        let d = self.dir[y * self.w + x];
        if d == u8::MAX {
            return None;
        }
        Some(DIRS[d as usize])
    }

    /// Greedy path from `(x, y)` to the goal following the vector
    /// field — `None` when the start is unreachable; returns the
    /// cell sequence including both ends. Terminates because every
    /// step strictly decreases `dist`.
    pub fn path(&self, x: usize, y: usize) -> Option<Vec<(u32, u32)>> {
        if x >= self.w || y >= self.h {
            return None;
        }
        let mut cur = self.dist[y * self.w + x];
        if cur == u32::MAX {
            return None;
        }
        let mut out = vec![(x as u32, y as u32)];
        let mut pos = (x, y);
        while cur > 0 {
            let d = self.direction(pos.0, pos.1)?;
            pos = (
                (pos.0 as i64 + d.0 as i64) as usize,
                (pos.1 as i64 + d.1 as i64) as usize,
            );
            cur = self.dist[pos.1 * self.w + pos.0];
            out.push((pos.0 as u32, pos.1 as u32));
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    /// Independent oracle: Dijkstra over the same move rules.
    fn oracle_dist(w: usize, h: usize, cost: &[u16], goals: &[u32]) -> Vec<u32> {
        let n = w * h;
        let mut dist = vec![u32::MAX; n];
        let mut heap = BinaryHeap::new();
        for &g in goals {
            if cost[g as usize] != WALL {
                dist[g as usize] = 0;
                heap.push((Reverse(0u32), g as usize));
            }
        }
        while let Some((Reverse(d), u)) = heap.pop() {
            if d > dist[u] {
                continue;
            }
            let ux = (u % w) as i64;
            let uy = (u / w) as i64;
            for &(dx, dy) in &DIRS {
                let (vx, vy) = (ux + dx as i64, uy + dy as i64);
                if vx < 0 || vy < 0 || vx >= w as i64 || vy >= h as i64 {
                    continue;
                }
                let v = (vy as usize) * w + vx as usize;
                if cost[v] == WALL {
                    continue;
                }
                let step = if dx != 0 && dy != 0 {
                    if cost[(uy as usize) * w + vx as usize] == WALL
                        || cost[(vy as usize) * w + ux as usize] == WALL
                    {
                        continue;
                    }
                    3 * cost[v] as u64
                } else {
                    2 * cost[v] as u64
                };
                if d as u64 + step < dist[v] as u64 {
                    dist[v] = (d as u64 + step) as u32;
                    heap.push((Reverse(dist[v]), v));
                }
            }
        }
        dist
    }

    fn random_map(rng: &mut SplitMix64, w: usize, h: usize) -> Vec<u16> {
        (0..w * h)
            .map(|_| match rng.below(10) {
                0..=1 => WALL,
                2..=5 => (rng.below(3) + 1) as u16,
                _ => 1,
            })
            .collect()
    }

    #[test]
    fn distances_match_oracle_and_dirs_descend() {
        let mut rng = SplitMix64::new(0xF10F);
        for _ in 0..300 {
            let w = 2 + rng.below(7) as usize;
            let h = 2 + rng.below(7) as usize;
            let cost = random_map(&mut rng, w, h);
            let goals: Vec<u32> = (0..(w * h) as u32)
                .filter(|&g| cost[g as usize] != WALL && rng.below(8) == 0)
                .collect();
            if goals.is_empty() {
                continue;
            }
            let ff = FlowField::new(w, h, &cost, &goals).unwrap();
            let oracle = oracle_dist(w, h, &cost, &goals);
            for (i, &od) in oracle.iter().enumerate() {
                let expect = (od != u32::MAX).then_some(od);
                assert_eq!(ff.dist(i % w, i / w), expect, "cell {i}");
            }
            // Every reachable non-goal cell has a dir that strictly
            // descends toward a goal; unreachable cells have none.
            for u in 0..w * h {
                match ff.direction(u % w, u / w) {
                    None => assert!(oracle[u] == u32::MAX || oracle[u] == 0),
                    Some((dx, dy)) => {
                        let v = (u as i64 + dy as i64 * w as i64 + dx as i64) as usize;
                        assert!(oracle[v] < oracle[u], "dir at {u} does not descend");
                    }
                }
            }
            // Paths walk off the field monotonically into a goal.
            for u in 0..w * h {
                if oracle[u] != u32::MAX {
                    let path = ff.path(u % w, u / w).unwrap();
                    assert_eq!(
                        *path.first().unwrap(),
                        (u as u32 % w as u32, u as u32 / w as u32)
                    );
                    let end = *path.last().unwrap();
                    assert_eq!(ff.dist(end.0 as usize, end.1 as usize), Some(0));
                    for pair in path.windows(2) {
                        let d0 = oracle[(pair[0].1 as usize) * w + pair[0].0 as usize];
                        let d1 = oracle[(pair[1].1 as usize) * w + pair[1].0 as usize];
                        assert!(d1 < d0);
                    }
                } else {
                    assert!(ff.path(u % w, u / w).is_none());
                }
            }
        }
    }

    #[test]
    fn no_corner_cutting() {
        // 2x2: goal at 3 (bottom-right). Cell 0→3 diagonal must be
        // blocked when cells 1 and 2 are walls, forcing unreachable.
        let cost = vec![1u16, WALL, WALL, 1];
        let ff = FlowField::new(2, 2, &cost, &[3]).unwrap();
        assert_eq!(ff.dist(0, 0), None);
        // Opening one orthogonal lets the diagonal through? No —
        // both must be free; with only cell 1 open the route is
        // 0→1→3, distance 2+2 = 4.
        let cost = vec![1u16, 1u16, WALL, 1];
        let ff = FlowField::new(2, 2, &cost, &[3]).unwrap();
        assert_eq!(ff.dist(0, 0), Some(4));
        assert_eq!(ff.direction(0, 0), Some((1, 0))); // E toward cell 1
    }

    #[test]
    fn weighted_terrain_prefers_detours() {
        // 3x3; goal at 8. Make the direct middle column expensive
        // (cost 9) so the field routes around it.
        let mut cost = vec![1u16; 9];
        cost[4] = 9;
        let ff = FlowField::new(3, 3, &cost, &[8]).unwrap();
        // Best is a rim path with one cheap diagonal: 0→1→5→8 or
        // 0→3→7→8 = 2+3+2 = 7. The tempting center diagonal 0→4
        // costs 3·9 = 27 alone.
        assert_eq!(ff.dist(0, 0).unwrap(), 7);
    }

    #[test]
    fn validation_and_boundaries() {
        assert!(FlowField::new(0, 0, &[], &[]).is_none());
        assert!(FlowField::new(2, 2, &[1u16; 3], &[0]).is_none()); // wrong len
        assert!(FlowField::new(2, 2, &[1u16; 4], &[5]).is_none()); // bad goal
                                                                   // All-wall map, wall goal: builds, nothing reachable.
        let cost = vec![WALL; 4];
        let ff = FlowField::new(2, 2, &cost, &[0]).unwrap();
        assert_eq!(ff.dist(0, 0), None);
        assert_eq!(ff.dims(), (2, 2));
        // Single goal at itself has no direction.
        let ff = FlowField::new(1, 1, &[1u16], &[0]).unwrap();
        assert_eq!(ff.direction(0, 0), None);
        assert_eq!(ff.path(0, 0).unwrap(), vec![(0, 0)]);
    }
}
