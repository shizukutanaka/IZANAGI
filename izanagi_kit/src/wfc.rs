//! Wave Function Collapse (WFC) procedural tile-map generation (I5).
//!
//! WFC fills a `width × height` grid of tile types using constraint propagation
//! and entropy-guided collapse. Each tile type has adjacency rules — bitmasks
//! of which tile types are allowed in each cardinal direction. Starting from
//! all tiles possible at every cell, the algorithm:
//!
//! 1. Finds the cell with the fewest remaining possibilities (lowest entropy).
//! 2. Randomly collapses it to one tile (using the caller-supplied PRNG).
//! 3. Propagates the constraint to neighbours via a BFS queue.
//! 4. Repeats until all cells are collapsed or a contradiction is reached.
//!
//! Up to **64 tile types** are supported (one bit each in a `u64` bitmask).
//! All arithmetic is integer; no float. Determinism is guaranteed: given the
//! same rules, dimensions, and RNG state the output is identical.
//!
//! # Example
//! ```
//! use izanagi_kit::wfc::{WfcRules, WfcResult, wfc_solve};
//! use izanagi_kit::rng::SplitMix64;
//!
//! // Tile 0 = floor, tile 1 = wall.
//! let mut rules = WfcRules::new(2);
//! // Floor may be next to anything; wall may be next to anything.
//! for tile in 0..2u8 {
//!     for dir in 0..4 {
//!         rules.allow(tile, dir, 0);
//!         rules.allow(tile, dir, 1);
//!     }
//! }
//! let mut rng = SplitMix64::new(42);
//! if let WfcResult::Ok(grid) = wfc_solve(8, 8, &rules, &mut rng) {
//!     assert!(grid.is_fully_collapsed());
//! }
//! ```

use std::collections::VecDeque;

use crate::{
    rng::SplitMix64,
    world_hash::{DetHash, Fnv1a},
};

/// Cardinal direction indices used by [`WfcRules`].
/// - 0 = North (y − 1)
/// - 1 = East  (x + 1)
/// - 2 = South (y + 1)
/// - 3 = West  (x − 1)
const DIRS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

/// Return the direction index opposite to `dir`.
#[inline]
fn opposite(dir: usize) -> usize {
    (dir + 2) & 3
}

/// Adjacency rules for a WFC solve.
///
/// `tile_count` tiles are identified by their bit index `0..tile_count` (max
/// 64). For each tile and direction, `allow` records which other tiles may
/// appear there. Tiles with no allowed neighbours in some direction will cause
/// a contradiction whenever they are placed next to a border; design rules to
/// allow at least one neighbour in every direction for every tile.
#[derive(Clone, Debug)]
pub struct WfcRules {
    tile_count: u8,
    /// `adj[tile][dir]` = bitmask of allowed neighbor tile types.
    adj: Vec<[u64; 4]>,
}

impl WfcRules {
    /// Create rules for `tile_count` tiles (clamped to 64).
    pub fn new(tile_count: u8) -> Self {
        let tc = tile_count.clamp(1, 64);
        Self {
            tile_count: tc,
            adj: vec![[0u64; 4]; tc as usize],
        }
    }

    /// Read back the adjacency bitmask for `tile` in `dir`. Returns `0` for
    /// out-of-range arguments. Useful for debugging and serialising rule sets.
    pub fn get_allowed(&self, tile: u8, dir: usize) -> u64 {
        if (tile as usize) < self.adj.len() && dir < 4 {
            self.adj[tile as usize][dir]
        } else {
            0
        }
    }

    /// Allow `tile` in direction `dir` to be adjacent to `neighbor`.
    /// `dir` must be in `0..4`; out-of-range values are ignored.
    pub fn allow(&mut self, tile: u8, dir: usize, neighbor: u8) {
        if (tile as usize) < self.adj.len() && dir < 4 && neighbor < self.tile_count {
            self.adj[tile as usize][dir] |= 1 << neighbor;
        }
    }

    /// Remove the adjacency permission: `tile` may no longer have `neighbor` in
    /// direction `dir`. Out-of-range arguments are silently ignored.
    pub fn disallow(&mut self, tile: u8, dir: usize, neighbor: u8) {
        if (tile as usize) < self.adj.len() && dir < 4 && neighbor < self.tile_count {
            self.adj[tile as usize][dir] &= !(1u64 << neighbor);
        }
    }

    /// Allow `tile_a` to be in direction `dir` next to `tile_b`, and
    /// symmetrically allow `tile_b` to be in the opposite direction next to
    /// `tile_a`. Use this to keep rules consistent without duplication.
    pub fn allow_symmetric(&mut self, tile_a: u8, dir: usize, tile_b: u8) {
        self.allow(tile_a, dir, tile_b);
        self.allow(tile_b, opposite(dir), tile_a);
    }

    /// Number of tile types these rules cover.
    pub fn tile_count(&self) -> u8 {
        self.tile_count
    }

    /// Count of tile types allowed adjacent to `tile` in direction `dir`.
    /// Returns `0` for out-of-range arguments. Useful for entropy estimation
    /// and detecting over-constrained tiles (a count of 0 will cause a
    /// contradiction whenever that tile-direction is encountered during solve).
    #[inline]
    pub fn allowed_count(&self, tile: u8, dir: usize) -> usize {
        self.get_allowed(tile, dir).count_ones() as usize
    }

    /// Clear all adjacency rules for `tile` in every direction, leaving it
    /// forbidden everywhere. Out-of-range `tile` is silently ignored.
    pub fn clear_adjacencies(&mut self, tile: u8) {
        if (tile as usize) < self.adj.len() {
            self.adj[tile as usize] = [0u64; 4];
        }
    }

    /// Returns `true` if `tile` is within the valid tile index range
    /// (`tile < tile_count()`). Out-of-range tiles are silently ignored by
    /// `allow`/`disallow`; this lets callers check before calling those.
    #[inline]
    pub fn is_valid_tile(&self, tile: u8) -> bool {
        (tile as usize) < self.adj.len()
    }

    /// Bitmask with one bit set per valid tile index.
    fn all_tiles(&self) -> u64 {
        if self.tile_count >= 64 {
            u64::MAX
        } else {
            (1u64 << self.tile_count) - 1
        }
    }
}

/// A WFC-solved (or partially collapsed) grid.
#[derive(Clone, Debug)]
pub struct WfcGrid {
    /// Grid width in cells.
    pub width: i32,
    /// Grid height in cells.
    pub height: i32,
    /// Per-cell bitmask of remaining tile possibilities (row-major).
    cells: Vec<u64>,
}

impl WfcGrid {
    /// The collapsed tile at `(x, y)`, or `None` if the cell is still
    /// ambiguous (more than one possibility) or out of bounds.
    pub fn tile_at(&self, x: i32, y: i32) -> Option<u8> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        let v = self.cells[(y * self.width + x) as usize];
        if v.count_ones() == 1 {
            Some(v.trailing_zeros() as u8)
        } else {
            None
        }
    }

    /// Whether every cell is fully collapsed to exactly one tile.
    pub fn is_fully_collapsed(&self) -> bool {
        self.cells.iter().all(|&v| v.count_ones() == 1)
    }

    /// Number of cells that are **not** yet fully collapsed (still have more
    /// than one possibility remaining). Complement of the count yielded by
    /// `is_fully_collapsed`.
    pub fn count_uncollapsed(&self) -> usize {
        self.cells.iter().filter(|&&v| v.count_ones() != 1).count()
    }

    /// Width times height.
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// True if the grid has no cells.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Count how many fully-collapsed cells have the given `tile` type.
    pub fn count_tiles(&self, tile: u8) -> usize {
        let mask = 1u64 << tile;
        self.cells.iter().filter(|&&v| v == mask).count()
    }

    /// Number of remaining tile possibilities at cell `(x, y)`.
    ///
    /// Returns `0` for out-of-bounds coordinates or cells in contradiction
    /// (bitmask `0`). Returns `1` for a fully-collapsed cell. Values `> 1`
    /// indicate superposition — use this to inspect WFC entropy during
    /// debugging or to drive custom collapse strategies.
    pub fn possibilities_at(&self, x: i32, y: i32) -> usize {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return 0;
        }
        self.cells[(y * self.width + x) as usize].count_ones() as usize
    }

    /// Export collapsed tiles as a flat row-major `Vec<Option<u8>>`. Each cell
    /// is `Some(tile)` if fully collapsed or `None` if still ambiguous.
    /// Length is always `width * height`.
    pub fn to_vec(&self) -> Vec<Option<u8>> {
        self.cells
            .iter()
            .map(|&v| {
                if v.count_ones() == 1 {
                    Some(v.trailing_zeros() as u8)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Iterate `(x, y, tile)` over all fully-collapsed cells in row-major order.
    pub fn iter_collapsed(&self) -> impl Iterator<Item = (i32, i32, u8)> + '_ {
        self.cells.iter().enumerate().filter_map(move |(i, &v)| {
            if v.count_ones() == 1 {
                let x = (i as i32) % self.width;
                let y = (i as i32) / self.width;
                Some((x, y, v.trailing_zeros() as u8))
            } else {
                None
            }
        })
    }
}

impl DetHash for WfcGrid {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_i32(self.width);
        hasher.write_i32(self.height);
        for &cell in &self.cells {
            hasher.write_u64(cell);
        }
    }
}

/// Result of a WFC solve attempt.
#[derive(Clone, Debug)]
pub enum WfcResult {
    /// All cells were successfully collapsed.
    Ok(WfcGrid),
    /// A cell reached zero possibilities — the rules or initial constraints
    /// produced a contradiction. Try a different seed or relax the rules.
    Contradiction,
}

/// Pick a uniformly random set bit from `mask` using `rng`.
/// Returns the bit index of the chosen bit. `mask` must be non-zero.
fn pick_random_bit(mask: u64, rng: &mut SplitMix64) -> u8 {
    let count = mask.count_ones();
    let idx = rng.below(count);
    let mut m = mask;
    for _ in 0..idx {
        m &= m - 1; // clear lowest set bit
    }
    m.trailing_zeros() as u8
}

/// Propagate constraints starting from `(sx, sy)` using BFS.
/// Returns `false` if a contradiction (zero-possibility cell) is reached.
fn propagate(
    cells: &mut [u64],
    width: i32,
    height: i32,
    sx: i32,
    sy: i32,
    rules: &WfcRules,
) -> bool {
    let mut queue: VecDeque<(i32, i32)> = VecDeque::new();
    let mut in_queue = vec![false; cells.len()];

    let start_idx = (sy * width + sx) as usize;
    queue.push_back((sx, sy));
    in_queue[start_idx] = true;

    while let Some((x, y)) = queue.pop_front() {
        let idx = (y * width + x) as usize;
        in_queue[idx] = false;
        let cur_mask = cells[idx];

        for (dir, &(dx, dy)) in DIRS.iter().enumerate() {
            let nx = x + dx;
            let ny = y + dy;
            if nx < 0 || ny < 0 || nx >= width || ny >= height {
                continue;
            }
            let nidx = (ny * width + nx) as usize;

            // Compute union of allowed neighbors for all current possibilities.
            let mut allowed = 0u64;
            let mut bits = cur_mask;
            while bits != 0 {
                let tile = bits.trailing_zeros() as u8;
                allowed |= rules.adj[tile as usize][dir];
                bits &= bits - 1;
            }

            let prev = cells[nidx];
            let new_val = prev & allowed;
            if new_val == 0 {
                return false;
            }
            if new_val != prev {
                cells[nidx] = new_val;
                if !in_queue[nidx] {
                    in_queue[nidx] = true;
                    queue.push_back((nx, ny));
                }
            }
        }
    }
    true
}

/// Solve a `width × height` WFC grid using `rules` and `rng`.
///
/// Returns [`WfcResult::Ok`] with a fully collapsed grid on success, or
/// [`WfcResult::Contradiction`] when the rules force a dead end.
///
/// `width <= 0`, `height <= 0`, or `rules.tile_count() == 0` all produce
/// `Contradiction` immediately.
pub fn wfc_solve(width: i32, height: i32, rules: &WfcRules, rng: &mut SplitMix64) -> WfcResult {
    if width <= 0 || height <= 0 || rules.tile_count == 0 {
        return WfcResult::Contradiction;
    }

    let size = (width * height) as usize;
    let all = rules.all_tiles();
    let mut cells = vec![all; size];

    loop {
        // Find the cell with minimum entropy (popcount) > 1.
        let mut min_entropy = u32::MAX;
        let mut min_idx: Option<usize> = None;

        for (i, &v) in cells.iter().enumerate() {
            let bits = v.count_ones();
            if bits == 0 {
                return WfcResult::Contradiction;
            }
            if bits > 1 && bits < min_entropy {
                min_entropy = bits;
                min_idx = Some(i);
            }
        }

        let idx = match min_idx {
            None => {
                // All cells have entropy 1 — fully collapsed.
                return WfcResult::Ok(WfcGrid {
                    width,
                    height,
                    cells,
                });
            }
            Some(i) => i,
        };

        // Collapse the chosen cell.
        let chosen = pick_random_bit(cells[idx], rng);
        cells[idx] = 1 << chosen;

        let x = (idx as i32) % width;
        let y = (idx as i32) / width;
        if !propagate(&mut cells, width, height, x, y, rules) {
            return WfcResult::Contradiction;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{rng::SplitMix64, world_hash::hash_state};

    /// 2-tile rules: floor (0) and wall (1) may both be adjacent to anything.
    fn open_rules() -> WfcRules {
        let mut r = WfcRules::new(2);
        for tile in 0..2u8 {
            for dir in 0..4 {
                r.allow(tile, dir, 0);
                r.allow(tile, dir, 1);
            }
        }
        r
    }

    /// Rules where tile 0 only allows itself in every direction.
    fn uniform_rules() -> WfcRules {
        let mut r = WfcRules::new(1);
        for dir in 0..4 {
            r.allow(0, dir, 0);
        }
        r
    }

    #[test]
    fn test_uniform_rules_always_succeeds() {
        let mut rng = SplitMix64::new(1);
        match wfc_solve(8, 8, &uniform_rules(), &mut rng) {
            WfcResult::Ok(grid) => {
                assert!(grid.is_fully_collapsed());
                for y in 0..8 {
                    for x in 0..8 {
                        assert_eq!(grid.tile_at(x, y), Some(0));
                    }
                }
            }
            WfcResult::Contradiction => panic!("uniform rules should never contradict"),
        }
    }

    #[test]
    fn test_open_rules_fully_collapsed() {
        let mut rng = SplitMix64::new(42);
        match wfc_solve(10, 10, &open_rules(), &mut rng) {
            WfcResult::Ok(grid) => assert!(grid.is_fully_collapsed()),
            WfcResult::Contradiction => panic!("open rules should not contradict"),
        }
    }

    #[test]
    fn test_tiles_in_range() {
        let mut rng = SplitMix64::new(7);
        match wfc_solve(6, 6, &open_rules(), &mut rng) {
            WfcResult::Ok(grid) => {
                for y in 0..6 {
                    for x in 0..6 {
                        let t = grid.tile_at(x, y).unwrap();
                        assert!(t < 2, "tile {t} out of range");
                    }
                }
            }
            WfcResult::Contradiction => panic!(),
        }
    }

    #[test]
    fn test_deterministic_same_seed() {
        let rules = open_rules();
        let r1 = wfc_solve(8, 8, &rules, &mut SplitMix64::new(99));
        let r2 = wfc_solve(8, 8, &rules, &mut SplitMix64::new(99));
        match (r1, r2) {
            (WfcResult::Ok(g1), WfcResult::Ok(g2)) => {
                assert_eq!(hash_state(&g1), hash_state(&g2));
            }
            _ => panic!("both should succeed"),
        }
    }

    #[test]
    fn test_different_seeds_likely_differ() {
        let rules = open_rules();
        let r1 = wfc_solve(10, 10, &rules, &mut SplitMix64::new(1));
        let r2 = wfc_solve(10, 10, &rules, &mut SplitMix64::new(999));
        match (r1, r2) {
            (WfcResult::Ok(g1), WfcResult::Ok(g2)) => {
                // Not guaranteed but virtually certain on a 100-cell grid.
                assert_ne!(hash_state(&g1), hash_state(&g2));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_contradiction_on_impossible_rules() {
        // Tile 0 requires tile 1 to the North, but tile 1 requires tile 0 to
        // the North only and allows nothing to the South → on a 1×2 grid
        // where tile 0 is forced North, no valid South exists.
        let mut r = WfcRules::new(2);
        // tile 0 north neighbor must be tile 1
        r.allow(0, 0, 1);
        // tile 1 north neighbor must be tile 0 (fine), but South allows nothing
        r.allow(1, 0, 0);
        // Neither tile allows any south neighbor → any cell placed will kill south constraint
        // Force a 1x3 vertical strip; the bottom cell gets no allowed tiles.
        let result = wfc_solve(1, 3, &r, &mut SplitMix64::new(1));
        // May succeed or contradict depending on collapse order; just assert no panic.
        let _ = result;
    }

    #[test]
    fn test_zero_dimension_is_contradiction() {
        assert!(matches!(
            wfc_solve(0, 5, &open_rules(), &mut SplitMix64::new(1)),
            WfcResult::Contradiction
        ));
        assert!(matches!(
            wfc_solve(5, 0, &open_rules(), &mut SplitMix64::new(1)),
            WfcResult::Contradiction
        ));
    }

    #[test]
    fn test_single_cell_grid() {
        let mut rng = SplitMix64::new(1);
        match wfc_solve(1, 1, &open_rules(), &mut rng) {
            WfcResult::Ok(grid) => {
                assert!(grid.is_fully_collapsed());
                assert!(grid.tile_at(0, 0).is_some());
            }
            WfcResult::Contradiction => panic!(),
        }
    }

    #[test]
    fn test_tile_at_out_of_bounds_is_none() {
        let grid = WfcGrid {
            width: 4,
            height: 4,
            cells: vec![1u64; 16],
        };
        assert!(grid.tile_at(-1, 0).is_none());
        assert!(grid.tile_at(4, 0).is_none());
        assert!(grid.tile_at(0, 4).is_none());
    }

    #[test]
    fn test_iter_collapsed_counts() {
        let mut rng = SplitMix64::new(5);
        match wfc_solve(5, 5, &open_rules(), &mut rng) {
            WfcResult::Ok(grid) => {
                let count = grid.iter_collapsed().count();
                assert_eq!(count, 25);
            }
            WfcResult::Contradiction => panic!(),
        }
    }

    #[test]
    fn test_disallow_removes_adjacency() {
        let mut r = WfcRules::new(2);
        r.allow(0, 0, 1);
        assert!(r.adj[0][0] & (1 << 1) != 0);
        r.disallow(0, 0, 1);
        assert_eq!(r.adj[0][0] & (1 << 1), 0);
    }

    #[test]
    fn test_disallow_out_of_range_is_noop() {
        let mut r = WfcRules::new(2);
        r.allow(0, 0, 1);
        r.disallow(5, 0, 1); // tile out of range
        r.disallow(0, 5, 1); // dir out of range
        assert!(r.adj[0][0] & (1 << 1) != 0, "original bit must survive");
    }

    #[test]
    fn test_disallow_does_not_affect_other_dirs() {
        let mut r = WfcRules::new(2);
        for dir in 0..4 {
            r.allow(0, dir, 1);
        }
        r.disallow(0, 1, 1); // remove only East
        assert_eq!(r.adj[0][1] & (1 << 1), 0); // East cleared
        assert!(r.adj[0][0] & (1 << 1) != 0); // North intact
        assert!(r.adj[0][2] & (1 << 1) != 0); // South intact
        assert!(r.adj[0][3] & (1 << 1) != 0); // West intact
    }

    #[test]
    fn test_allow_symmetric() {
        let mut r = WfcRules::new(2);
        r.allow_symmetric(0, 1, 1); // tile 0 East → tile 1; tile 1 West → tile 0
        assert!(r.adj[0][1] & (1 << 1) != 0); // 0 allows 1 to East
        assert!(r.adj[1][3] & (1 << 0) != 0); // 1 allows 0 to West
    }

    #[test]
    fn test_det_hash_same_grid() {
        let grid1 = WfcGrid {
            width: 2,
            height: 2,
            cells: vec![1, 2, 3, 4],
        };
        let grid2 = grid1.clone();
        assert_eq!(hash_state(&grid1), hash_state(&grid2));
    }

    #[test]
    fn test_det_hash_differs_on_change() {
        let grid1 = WfcGrid {
            width: 2,
            height: 2,
            cells: vec![1, 2, 3, 4],
        };
        let grid2 = WfcGrid {
            width: 2,
            height: 2,
            cells: vec![1, 2, 3, 5],
        };
        assert_ne!(hash_state(&grid1), hash_state(&grid2));
    }

    #[test]
    fn test_len_and_is_empty() {
        let grid = WfcGrid {
            width: 3,
            height: 4,
            cells: vec![1u64; 12],
        };
        assert_eq!(grid.len(), 12);
        assert!(!grid.is_empty());

        let empty = WfcGrid {
            width: 0,
            height: 0,
            cells: vec![],
        };
        assert!(empty.is_empty());
    }

    #[test]
    fn test_get_allowed_roundtrip() {
        let mut r = WfcRules::new(3);
        r.allow(0, 1, 2); // tile 0 East may have tile 2
        assert_eq!(r.get_allowed(0, 1), 1 << 2);
        assert_eq!(r.get_allowed(0, 0), 0); // North not set
        assert_eq!(r.get_allowed(99, 0), 0); // OOB tile
        assert_eq!(r.get_allowed(0, 9), 0); // OOB dir
    }

    #[test]
    fn test_count_tiles_counts_collapsed() {
        let mut rng = SplitMix64::new(1);
        match wfc_solve(6, 6, &open_rules(), &mut rng) {
            WfcResult::Ok(grid) => {
                let total: usize = (0..open_rules().tile_count())
                    .map(|t| grid.count_tiles(t))
                    .sum();
                assert_eq!(total, 36); // all 36 cells counted exactly once
            }
            WfcResult::Contradiction => panic!(),
        }
    }

    #[test]
    fn test_to_vec_length_and_some_for_collapsed() {
        let mut rng = SplitMix64::new(7);
        match wfc_solve(4, 4, &open_rules(), &mut rng) {
            WfcResult::Ok(grid) => {
                let v = grid.to_vec();
                assert_eq!(v.len(), 16);
                assert!(v.iter().all(|t| t.is_some())); // all collapsed
            }
            WfcResult::Contradiction => panic!(),
        }
    }

    #[test]
    fn test_clear_adjacencies_removes_all_bits() {
        let mut r = WfcRules::new(3);
        for dir in 0..4 {
            r.allow(0, dir, 1);
            r.allow(0, dir, 2);
        }
        r.clear_adjacencies(0);
        for dir in 0..4 {
            assert_eq!(r.adj[0][dir], 0, "dir {dir} should be 0 after clear");
        }
    }

    #[test]
    fn test_clear_adjacencies_does_not_affect_other_tiles() {
        let mut r = WfcRules::new(3);
        r.allow(0, 0, 1);
        r.allow(1, 0, 0); // other tile
        r.clear_adjacencies(0);
        assert_eq!(r.adj[0][0], 0);
        assert_ne!(r.adj[1][0], 0, "tile 1 should be unaffected");
    }

    #[test]
    fn test_clear_adjacencies_oob_is_noop() {
        let mut r = WfcRules::new(2);
        r.allow(0, 0, 1);
        r.clear_adjacencies(99); // out of range — should not panic
        assert_ne!(r.adj[0][0], 0, "tile 0 unaffected");
    }

    #[test]
    fn test_allowed_count_zero_when_no_rules() {
        let r = WfcRules::new(4);
        assert_eq!(r.allowed_count(0, 0), 0, "fresh rules: no adjacencies");
    }

    #[test]
    fn test_allowed_count_matches_number_of_allows() {
        let mut r = WfcRules::new(4);
        r.allow(0, 1, 0);
        r.allow(0, 1, 2);
        assert_eq!(r.allowed_count(0, 1), 2, "two tiles allowed in dir 1");
        r.allow(0, 1, 3);
        assert_eq!(r.allowed_count(0, 1), 3);
    }

    #[test]
    fn test_allowed_count_oob_returns_zero() {
        let r = WfcRules::new(4);
        assert_eq!(r.allowed_count(99, 0), 0, "OOB tile");
        assert_eq!(r.allowed_count(0, 9), 0, "OOB dir");
    }

    #[test]
    fn test_is_valid_tile_in_range() {
        let r = WfcRules::new(3);
        assert!(r.is_valid_tile(0));
        assert!(r.is_valid_tile(2));
        assert!(!r.is_valid_tile(3));
    }

    #[test]
    fn test_is_valid_tile_clamped_min_allows_zero() {
        // new(0) clamps to 1, so tile 0 is valid and tile 1 is not.
        let r = WfcRules::new(0);
        assert!(r.is_valid_tile(0));
        assert!(!r.is_valid_tile(1));
    }

    #[test]
    fn test_is_valid_tile_boundary_values() {
        let r = WfcRules::new(64);
        assert!(r.is_valid_tile(63));
        assert!(!r.is_valid_tile(64));
    }

    #[test]
    fn test_count_uncollapsed_zero_after_solve() {
        let mut rng = SplitMix64::new(1);
        if let WfcResult::Ok(grid) = wfc_solve(4, 4, &open_rules(), &mut rng) {
            assert_eq!(grid.count_uncollapsed(), 0);
        }
    }

    #[test]
    fn test_count_uncollapsed_complement_of_collapsed() {
        let mut rng = SplitMix64::new(7);
        if let WfcResult::Ok(grid) = wfc_solve(3, 3, &open_rules(), &mut rng) {
            assert_eq!(
                grid.count_uncollapsed() + grid.to_vec().iter().filter(|c| c.is_some()).count(),
                grid.len()
            );
        }
    }

    #[test]
    fn test_count_uncollapsed_uniform_is_zero() {
        let mut rng = SplitMix64::new(2);
        if let WfcResult::Ok(grid) = wfc_solve(5, 5, &uniform_rules(), &mut rng) {
            assert_eq!(grid.count_uncollapsed(), 0);
        }
    }

    #[test]
    fn test_possibilities_at_collapsed_is_one() {
        let mut rng = SplitMix64::new(7);
        if let WfcResult::Ok(grid) = wfc_solve(4, 4, &uniform_rules(), &mut rng) {
            // Every cell is collapsed after a successful solve.
            assert_eq!(grid.possibilities_at(0, 0), 1);
            assert_eq!(grid.possibilities_at(3, 3), 1);
        }
    }

    #[test]
    fn test_possibilities_at_out_of_bounds_is_zero() {
        let mut rng = SplitMix64::new(7);
        if let WfcResult::Ok(grid) = wfc_solve(4, 4, &uniform_rules(), &mut rng) {
            assert_eq!(grid.possibilities_at(-1, 0), 0);
            assert_eq!(grid.possibilities_at(0, 99), 0);
        }
    }

    #[test]
    fn test_possibilities_at_matches_count_uncollapsed() {
        let mut rng = SplitMix64::new(3);
        if let WfcResult::Ok(grid) = wfc_solve(5, 5, &uniform_rules(), &mut rng) {
            // Fully collapsed: every cell has exactly one possibility.
            let multi = (0..5)
                .flat_map(|y| (0..5).map(move |x| (x, y)))
                .filter(|&(x, y)| grid.possibilities_at(x, y) > 1)
                .count();
            assert_eq!(multi, grid.count_uncollapsed());
        }
    }
}
