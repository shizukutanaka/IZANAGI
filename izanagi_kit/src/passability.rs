//! Grid-based passability / collision layer (K1).
//!
//! `PassabilityGrid` stores one `bool` per cell in a row-major flat array:
//! `true` = blocked (wall, obstacle), `false` = passable (floor, open).
//!
//! It is the standard bridge between map data and movement systems:
//!
//! - Build from any map by supplying a closure: `from_fn(w, h, |x, y| …)`.
//! - Build from a [`tilemap::TileMap`] with a tile predicate.
//! - Mutate individual cells at runtime (`set_blocked`).
//! - Produce a **closure** compatible with [`pathfinding::astar`] and
//!   [`pathfinding::weighted_astar`] via `blocker()`.
//! - `DetHash` participation for replay state checksums.

use crate::rng::SplitMix64;
use crate::world_hash::{DetHash, Fnv1a};

/// A flat boolean passability grid.
///
/// `true` = blocked; `false` = passable.
/// Out-of-bounds coordinates are always considered blocked.
#[derive(Clone, Debug)]
pub struct PassabilityGrid {
    width: i32,
    height: i32,
    cells: Vec<bool>,
}

impl PassabilityGrid {
    /// Create a grid with all cells passable (`blocked = false`).
    pub fn new(width: i32, height: i32) -> Self {
        let w = width.max(0);
        let h = height.max(0);
        Self {
            width: w,
            height: h,
            cells: vec![false; (w * h) as usize],
        }
    }

    /// Create a grid by calling `f(x, y)` for every cell.
    pub fn from_fn<F>(width: i32, height: i32, mut f: F) -> Self
    where
        F: FnMut(i32, i32) -> bool,
    {
        let w = width.max(0);
        let h = height.max(0);
        let mut cells = Vec::with_capacity((w * h) as usize);
        for y in 0..h {
            for x in 0..w {
                cells.push(f(x, y));
            }
        }
        Self {
            width: w,
            height: h,
            cells,
        }
    }

    /// Create from a [`tilemap::TileMap<T>`], treating cells where
    /// `is_blocked(tile)` returns `true` as walls.
    pub fn from_tilemap<T, F>(map: &crate::tilemap::TileMap<T>, is_blocked: F) -> Self
    where
        T: Clone,
        F: Fn(&T) -> bool,
    {
        Self::from_fn(map.width() as i32, map.height() as i32, |x, y| {
            map.get(x, y).map_or(true, &is_blocked)
        })
    }

    /// Create from a [`mapgen::Dungeon`], treating wall tiles as blocked.
    pub fn from_dungeon(dungeon: &crate::mapgen::Dungeon) -> Self {
        Self::from_fn(dungeon.width() as i32, dungeon.height() as i32, |x, y| {
            dungeon.is_wall(x, y)
        })
    }

    /// Width in cells.
    pub fn width(&self) -> i32 {
        self.width
    }

    /// Height in cells.
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Whether `(x, y)` is passable (not blocked). Out-of-bounds always returns
    /// `false`. Convenience inverse of `is_blocked` — avoids `!grid.is_blocked(…)`
    /// at call sites where the positive condition is cleaner.
    #[inline]
    pub fn is_passable(&self, x: i32, y: i32) -> bool {
        !self.is_blocked(x, y)
    }

    /// Whether `(x, y)` is blocked. Out-of-bounds always returns `true`.
    #[inline]
    pub fn is_blocked(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return true;
        }
        self.cells[(y * self.width + x) as usize]
    }

    /// Set the passability of cell `(x, y)`. Out-of-bounds is a no-op.
    pub fn set_blocked(&mut self, x: i32, y: i32, blocked: bool) {
        if x >= 0 && y >= 0 && x < self.width && y < self.height {
            self.cells[(y * self.width + x) as usize] = blocked;
        }
    }

    /// Return a closure `|x, y| self.is_blocked(x, y)` that borrows `self`.
    ///
    /// Pass the closure directly to [`pathfinding::astar`] or
    /// [`pathfinding::weighted_astar`].
    pub fn blocker(&self) -> impl Fn(i32, i32) -> bool + '_ {
        move |x, y| self.is_blocked(x, y)
    }

    /// Number of cells.
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Whether the grid has no cells.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Count of blocked cells.
    pub fn blocked_count(&self) -> usize {
        self.cells.iter().filter(|&&b| b).count()
    }

    /// Count of passable cells.
    pub fn passable_count(&self) -> usize {
        self.cells.iter().filter(|&&b| !b).count()
    }

    /// Iterate `(x, y)` coordinates of all **passable** cells in row-major order.
    ///
    /// Useful for placing random spawns: collect the iterator, then index it with
    /// a `rng.below(count)` draw.
    pub fn iter_passable(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        let w = self.width;
        let h = self.height;
        let cells = &self.cells;
        (0..h).flat_map(move |y| {
            (0..w).filter_map(move |x| {
                if !cells[(y * w + x) as usize] {
                    Some((x, y))
                } else {
                    None
                }
            })
        })
    }

    /// Bulk-set all cells in the axis-aligned rectangle `[x1, x2] × [y1, y2]`
    /// (both endpoints inclusive). Out-of-bounds cells are silently skipped.
    /// Coordinates are treated in any order (`x1 > x2` is the same as `x1 ≤ x2`).
    pub fn set_region(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, blocked: bool) {
        let xs = x1.min(x2).max(0);
        let xe = x1.max(x2).min(self.width - 1);
        let ys = y1.min(y2).max(0);
        let ye = y1.max(y2).min(self.height - 1);
        for y in ys..=ye {
            for x in xs..=xe {
                self.cells[(y * self.width + x) as usize] = blocked;
            }
        }
    }

    /// Pick a uniformly random passable cell, or `None` if every cell is blocked.
    /// Deterministic: draws exactly one value from `rng` when at least one
    /// passable cell exists; draws nothing when all cells are blocked.
    pub fn random_passable(&self, rng: &mut SplitMix64) -> Option<(i32, i32)> {
        let candidates: Vec<(i32, i32)> = self.iter_passable().collect();
        rng.pick(&candidates).copied()
    }

    /// Set every cell to `blocked`. Equivalent to reconstructing the grid with
    /// `from_fn(w, h, |_, _| blocked)` but without reallocation. Useful for
    /// "seal entire floor as solid" / "clear all walls" primitives before
    /// carving a new layout.
    pub fn fill(&mut self, blocked: bool) {
        for cell in &mut self.cells {
            *cell = blocked;
        }
    }

    /// Iterate `(x, y)` coordinates of all **blocked** cells in row-major order.
    pub fn iter_blocked(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        let w = self.width;
        let h = self.height;
        let cells = &self.cells;
        (0..h).flat_map(move |y| {
            (0..w).filter_map(move |x| {
                if cells[(y * w + x) as usize] {
                    Some((x, y))
                } else {
                    None
                }
            })
        })
    }

    /// Count orthogonal (4-direction) neighbours of `(x, y)` that are blocked.
    ///
    /// The result is in `0..=4`. Out-of-bounds neighbours count as blocked.
    /// Used in cellular-automaton cave generation (if ≥ 5 neighbours blocked →
    /// become wall) and dungeon connectivity checks without allocating a list.
    #[inline]
    pub fn count_neighbors_blocked(&self, x: i32, y: i32) -> usize {
        [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
            .iter()
            .filter(|&&(nx, ny)| self.is_blocked(nx, ny))
            .count()
    }

    /// Flip every cell in place: passable → blocked, blocked → passable.
    /// Useful for testing ("treat walls as floor") and negative-space queries.
    pub fn invert(&mut self) {
        for cell in &mut self.cells {
            *cell = !*cell;
        }
    }
}

impl DetHash for PassabilityGrid {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_i32(self.width);
        hasher.write_i32(self.height);
        for &b in &self.cells {
            hasher.write_u32(b as u32);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mapgen::{generate_dungeon, GenParams},
        rng::SplitMix64,
        tilemap::TileMap,
        world_hash::hash_state,
    };

    #[test]
    fn test_new_all_passable() {
        let grid = PassabilityGrid::new(4, 4);
        for y in 0..4 {
            for x in 0..4 {
                assert!(!grid.is_blocked(x, y));
            }
        }
    }

    #[test]
    fn test_out_of_bounds_is_blocked() {
        let grid = PassabilityGrid::new(3, 3);
        assert!(grid.is_blocked(-1, 0));
        assert!(grid.is_blocked(0, -1));
        assert!(grid.is_blocked(3, 0));
        assert!(grid.is_blocked(0, 3));
    }

    #[test]
    fn test_set_blocked() {
        let mut grid = PassabilityGrid::new(5, 5);
        grid.set_blocked(2, 3, true);
        assert!(grid.is_blocked(2, 3));
        grid.set_blocked(2, 3, false);
        assert!(!grid.is_blocked(2, 3));
    }

    #[test]
    fn test_set_blocked_out_of_bounds_is_noop() {
        let mut grid = PassabilityGrid::new(3, 3);
        grid.set_blocked(-1, 0, true); // no panic
        grid.set_blocked(99, 99, false); // no panic
        assert_eq!(grid.blocked_count(), 0);
    }

    #[test]
    fn test_from_fn() {
        // Checkerboard: blocked when (x+y) is even.
        let grid = PassabilityGrid::from_fn(4, 4, |x, y| (x + y) % 2 == 0);
        assert!(grid.is_blocked(0, 0));
        assert!(!grid.is_blocked(1, 0));
        assert!(!grid.is_blocked(0, 1));
        assert!(grid.is_blocked(1, 1));
    }

    #[test]
    fn test_from_tilemap() {
        let mut map: TileMap<u8> = TileMap::new(4, 4, 0);
        map.set(1, 1, 1); // mark (1,1) as a wall tile
        map.set(2, 2, 1);
        let grid = PassabilityGrid::from_tilemap(&map, |&tile| tile == 1);
        assert!(grid.is_blocked(1, 1));
        assert!(grid.is_blocked(2, 2));
        assert!(!grid.is_blocked(0, 0));
    }

    #[test]
    fn test_from_dungeon() {
        let mut rng = SplitMix64::new(42);
        let dungeon = generate_dungeon(20, 15, &mut rng, GenParams::default());
        let grid = PassabilityGrid::from_dungeon(&dungeon);
        assert_eq!(grid.width(), dungeon.width() as i32);
        assert_eq!(grid.height(), dungeon.height() as i32);
        // Dungeon borders are always walls.
        assert!(grid.is_blocked(0, 0));
    }

    #[test]
    fn test_blocker_closure_matches_is_blocked() {
        let mut grid = PassabilityGrid::new(6, 6);
        grid.set_blocked(3, 3, true);
        let b = grid.blocker();
        assert!(b(3, 3));
        assert!(!b(0, 0));
        assert!(b(-1, 0)); // out of bounds
    }

    #[test]
    fn test_blocker_with_astar() {
        use crate::pathfinding::astar;
        let mut grid = PassabilityGrid::new(10, 10);
        // Vertical wall at x=5, gap at y=9.
        for y in 0..9 {
            grid.set_blocked(5, y, true);
        }
        let path = astar((1, 4), (8, 4), grid.blocker()).unwrap();
        assert_eq!(path.first(), Some(&(1, 4)));
        assert_eq!(path.last(), Some(&(8, 4)));
        assert!(path.contains(&(5, 9)), "must pass through gap");
    }

    #[test]
    fn test_len_and_is_empty() {
        let grid = PassabilityGrid::new(3, 4);
        assert_eq!(grid.len(), 12);
        assert!(!grid.is_empty());

        let empty = PassabilityGrid::new(0, 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_blocked_and_passable_counts() {
        let mut grid = PassabilityGrid::new(4, 4);
        grid.set_blocked(0, 0, true);
        grid.set_blocked(1, 1, true);
        assert_eq!(grid.blocked_count(), 2);
        assert_eq!(grid.passable_count(), 14);
        assert_eq!(grid.blocked_count() + grid.passable_count(), grid.len());
    }

    #[test]
    fn test_det_hash_same_state() {
        let g1 = PassabilityGrid::from_fn(4, 4, |x, y| x == y);
        let g2 = PassabilityGrid::from_fn(4, 4, |x, y| x == y);
        assert_eq!(hash_state(&g1), hash_state(&g2));
    }

    #[test]
    fn test_det_hash_differs_on_change() {
        let g1 = PassabilityGrid::from_fn(4, 4, |_, _| false);
        let mut g2 = PassabilityGrid::from_fn(4, 4, |_, _| false);
        g2.set_blocked(2, 2, true);
        assert_ne!(hash_state(&g1), hash_state(&g2));
    }

    #[test]
    fn test_width_height() {
        let grid = PassabilityGrid::new(7, 3);
        assert_eq!(grid.width(), 7);
        assert_eq!(grid.height(), 3);
    }

    #[test]
    fn test_negative_dimensions_are_empty() {
        let grid = PassabilityGrid::new(-5, 10);
        assert!(grid.is_empty());
        assert_eq!(grid.width(), 0);
    }

    #[test]
    fn test_iter_passable_counts_match() {
        let mut grid = PassabilityGrid::new(4, 4);
        grid.set_blocked(0, 0, true);
        grid.set_blocked(1, 1, true);
        let passable: Vec<(i32, i32)> = grid.iter_passable().collect();
        assert_eq!(passable.len(), grid.passable_count());
        assert!(!passable.contains(&(0, 0)));
        assert!(!passable.contains(&(1, 1)));
    }

    #[test]
    fn test_iter_blocked_counts_match() {
        let mut grid = PassabilityGrid::new(4, 4);
        grid.set_blocked(2, 3, true);
        let blocked: Vec<(i32, i32)> = grid.iter_blocked().collect();
        assert_eq!(blocked.len(), grid.blocked_count());
        assert!(blocked.contains(&(2, 3)));
    }

    #[test]
    fn test_iter_passable_row_major_order() {
        let grid = PassabilityGrid::new(3, 2);
        let cells: Vec<(i32, i32)> = grid.iter_passable().collect();
        // All passable → row-major: (0,0),(1,0),(2,0),(0,1),(1,1),(2,1)
        assert_eq!(cells, [(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)]);
    }

    #[test]
    fn test_is_passable_is_inverse_of_is_blocked() {
        let mut grid = PassabilityGrid::new(4, 4);
        grid.set_blocked(1, 2, true);
        assert!(!grid.is_passable(1, 2));
        assert!(grid.is_passable(0, 0));
    }

    #[test]
    fn test_is_passable_out_of_bounds_returns_false() {
        let grid = PassabilityGrid::new(3, 3);
        assert!(!grid.is_passable(-1, 0));
        assert!(!grid.is_passable(0, -1));
        assert!(!grid.is_passable(3, 0));
    }

    #[test]
    fn test_set_region_marks_all_cells() {
        let mut grid = PassabilityGrid::new(6, 6);
        grid.set_region(1, 1, 3, 3, true);
        for y in 1..=3 {
            for x in 1..=3 {
                assert!(grid.is_blocked(x, y), "({x},{y}) should be blocked");
            }
        }
        // Cells outside the region should be unaffected.
        assert!(!grid.is_blocked(0, 0));
        assert!(!grid.is_blocked(4, 4));
    }

    #[test]
    fn test_set_region_reversed_coords_same_result() {
        let mut a = PassabilityGrid::new(5, 5);
        let mut b = PassabilityGrid::new(5, 5);
        a.set_region(1, 1, 3, 3, true);
        b.set_region(3, 3, 1, 1, true);
        assert_eq!(a.blocked_count(), b.blocked_count());
    }

    #[test]
    fn test_set_region_out_of_bounds_clipped() {
        let mut grid = PassabilityGrid::new(4, 4);
        grid.set_region(-5, -5, 1, 1, true); // only (0,0)-(1,1) is valid
                                             // Should not panic and should mark the valid overlap.
        assert!(grid.is_blocked(0, 0));
        assert!(grid.is_blocked(1, 1));
        assert!(!grid.is_blocked(2, 2));
    }

    #[test]
    fn test_set_region_clear_cells() {
        let mut grid = PassabilityGrid::from_fn(5, 5, |_, _| true); // all blocked
        grid.set_region(1, 1, 3, 3, false);
        for y in 1..=3 {
            for x in 1..=3 {
                assert!(!grid.is_blocked(x, y));
            }
        }
        assert!(grid.is_blocked(0, 0)); // border still blocked
    }

    #[test]
    fn test_random_passable_returns_passable_cell() {
        let grid = PassabilityGrid::new(4, 4); // all passable
        let mut rng = SplitMix64::new(42);
        let cell = grid.random_passable(&mut rng).unwrap();
        assert!(!grid.is_blocked(cell.0, cell.1));
    }

    #[test]
    fn test_random_passable_all_blocked_returns_none() {
        let grid = PassabilityGrid::from_fn(3, 3, |_, _| true); // all blocked
        let mut rng = SplitMix64::new(1);
        assert!(grid.random_passable(&mut rng).is_none());
    }

    #[test]
    fn test_random_passable_deterministic() {
        let grid = PassabilityGrid::new(5, 5);
        let cell1 = grid.random_passable(&mut SplitMix64::new(77)).unwrap();
        let cell2 = grid.random_passable(&mut SplitMix64::new(77)).unwrap();
        assert_eq!(cell1, cell2);
    }

    #[test]
    fn test_fill_blocked_makes_all_blocked() {
        let mut g = PassabilityGrid::new(4, 4);
        g.fill(true);
        assert_eq!(g.blocked_count(), 16);
        assert_eq!(g.passable_count(), 0);
    }

    #[test]
    fn test_fill_passable_clears_all_blocks() {
        let mut g = PassabilityGrid::new(3, 3);
        g.fill(true); // block first
        g.fill(false); // then clear
        assert_eq!(g.passable_count(), 9);
        assert_eq!(g.blocked_count(), 0);
    }

    #[test]
    fn test_fill_empty_grid_is_noop() {
        let mut g = PassabilityGrid::new(0, 0);
        g.fill(true); // must not panic
        assert_eq!(g.blocked_count(), 0);
    }

    #[test]
    fn test_invert_flips_all_cells() {
        let mut g = PassabilityGrid::new(3, 3);
        g.set_blocked(1, 1, true); // one blocked cell
        g.invert();
        // was passable → now blocked (8 cells), was blocked → now passable (1)
        assert_eq!(g.blocked_count(), 8);
        assert!(!g.is_blocked(1, 1));
    }

    #[test]
    fn test_invert_twice_is_identity() {
        let mut g = PassabilityGrid::new(4, 4);
        g.set_blocked(0, 0, true);
        g.set_blocked(3, 3, true);
        let before_blocked = g.blocked_count();
        g.invert();
        g.invert();
        assert_eq!(g.blocked_count(), before_blocked);
        assert!(g.is_blocked(0, 0));
        assert!(g.is_blocked(3, 3));
    }

    #[test]
    fn test_invert_all_blocked_makes_all_passable() {
        let mut g = PassabilityGrid::new(2, 2);
        g.fill(true);
        g.invert();
        assert_eq!(g.passable_count(), 4);
    }

    // --- count_neighbors_blocked ---

    #[test]
    fn test_count_neighbors_blocked_all_walls_is_four() {
        let mut g = PassabilityGrid::new(3, 3);
        // Set all four orthogonal neighbors of center (1,1) to blocked.
        g.set_blocked(0, 1, true);
        g.set_blocked(2, 1, true);
        g.set_blocked(1, 0, true);
        g.set_blocked(1, 2, true);
        assert_eq!(g.count_neighbors_blocked(1, 1), 4);
    }

    #[test]
    fn test_count_neighbors_blocked_oob_counts_as_blocked() {
        let g = PassabilityGrid::new(1, 1);
        // (0,0) has no in-bounds neighbors; all 4 are OOB → all blocked.
        assert_eq!(g.count_neighbors_blocked(0, 0), 4);
    }

    #[test]
    fn test_count_neighbors_blocked_open_field_is_zero() {
        let g = PassabilityGrid::new(5, 5);
        // Interior cell — all 4 neighbors are passable.
        assert_eq!(g.count_neighbors_blocked(2, 2), 0);
    }
}
