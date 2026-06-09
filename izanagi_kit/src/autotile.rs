//! Bitmask auto-tiling for grid-based terrain rendering.
//!
//! Auto-tiling computes which visual tile variant to display at each cell based
//! on which of its 8 neighbours share the same terrain type. The result is an
//! 8-bit mask (one bit per neighbour direction) that indexes into a 256-entry
//! tile variant table — the standard "blob" / "RPG Maker" style.
//!
//! Bit layout (bit 0 = least significant):
//! ```text
//!   7 | 0 | 1
//!   -----+-----
//!   6 | X | 2
//!   -----+-----
//!   5 | 4 | 3
//! ```
//! - Bit 0 = North, 1 = NE, 2 = East, 3 = SE, 4 = South, 5 = SW, 6 = West, 7 = NW
//!
//! Corner bits (1,3,5,7) are automatically cleared if either of their adjacent
//! cardinal neighbours is absent — the standard rule that prevents interior
//! corners from showing diagonal tiles incorrectly.
//!
//! `compute_mask(x, y, is_same)` computes the mask for a single cell.
//! `compute_all(w, h, is_same)` computes the full map in row-major order.
//! `SimpleTileTable` maps `u8` masks to tile IDs (u32) for the common case
//! where callers supply a flat 256-entry lookup.

/// Compute the 8-bit auto-tile mask for cell `(x, y)`.
///
/// `is_same(nx, ny)` returns `true` when the neighbour at `(nx, ny)` belongs to
/// the same terrain type as `(x, y)` (the source cell's type is caller-defined).
/// Out-of-bounds neighbours should return `false` so borders are treated as
/// different terrain.
///
/// The diagonal corner bits are automatically cleared when either of their
/// flanking cardinal bits is 0 (prevents artifact tiles on outer corners).
pub fn compute_mask<F>(x: i32, y: i32, is_same: F) -> u8
where
    F: Fn(i32, i32) -> bool,
{
    let n = is_same(x, y - 1);
    let ne = is_same(x + 1, y - 1);
    let e = is_same(x + 1, y);
    let se = is_same(x + 1, y + 1);
    let s = is_same(x, y + 1);
    let sw = is_same(x - 1, y + 1);
    let w = is_same(x - 1, y);
    let nw = is_same(x - 1, y - 1);

    let mut mask: u8 = 0;
    if n {
        mask |= 1 << 0;
    }
    if ne && n && e {
        mask |= 1 << 1;
    }
    if e {
        mask |= 1 << 2;
    }
    if se && s && e {
        mask |= 1 << 3;
    }
    if s {
        mask |= 1 << 4;
    }
    if sw && s && w {
        mask |= 1 << 5;
    }
    if w {
        mask |= 1 << 6;
    }
    if nw && n && w {
        mask |= 1 << 7;
    }

    mask
}

/// Compute auto-tile masks for every cell of a `w × h` grid.
///
/// Returns a `Vec<u8>` of length `w * h` in row-major order (row y starts at
/// index `y * w`). `is_same(x, y)` is called for each cell and its neighbours.
pub fn compute_all<F>(w: i32, h: i32, is_same: F) -> Vec<u8>
where
    F: Fn(i32, i32) -> bool,
{
    if w <= 0 || h <= 0 {
        return Vec::new();
    }
    let size = (w as usize) * (h as usize);
    let mut out = Vec::with_capacity(size);
    for y in 0..h {
        for x in 0..w {
            out.push(compute_mask(x, y, &is_same));
        }
    }
    out
}

/// Compute auto-tile masks for a rectangular subregion of a larger grid.
///
/// Returns a `Vec<u8>` of length `w * h` in row-major order covering the
/// rectangle `[x, x+w) × [y, y+h)`. `is_same` is called with absolute
/// coordinates (including neighbours outside the region boundary). An empty
/// `Vec` is returned for non-positive `w` or `h`.
///
/// Use this to recompute only the cells that changed (e.g. after a wall is
/// placed or removed) rather than recalculating the entire map with
/// `compute_all`.
pub fn compute_region<F>(x: i32, y: i32, w: i32, h: i32, is_same: F) -> Vec<u8>
where
    F: Fn(i32, i32) -> bool,
{
    if w <= 0 || h <= 0 {
        return Vec::new();
    }
    let size = (w as usize) * (h as usize);
    let mut out = Vec::with_capacity(size);
    for ry in y..y + h {
        for rx in x..x + w {
            out.push(compute_mask(rx, ry, &is_same));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// SimpleTileTable
// ---------------------------------------------------------------------------

/// A 256-entry lookup table mapping `u8` masks to tile variant IDs (`u32`).
///
/// Construct with `from_array` or use the builder API to fill only specific
/// entries. All unset entries default to 0.
#[derive(Clone, Debug)]
pub struct SimpleTileTable {
    table: [u32; 256],
}

impl Default for SimpleTileTable {
    fn default() -> Self {
        SimpleTileTable { table: [0u32; 256] }
    }
}

impl SimpleTileTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct from a full 256-entry array.
    pub fn from_array(table: [u32; 256]) -> Self {
        SimpleTileTable { table }
    }

    /// Set the tile variant for `mask` to `tile_id`.
    pub fn set(&mut self, mask: u8, tile_id: u32) {
        self.table[mask as usize] = tile_id;
    }

    /// Lookup the tile variant for `mask`.
    #[inline]
    pub fn get(&self, mask: u8) -> u32 {
        self.table[mask as usize]
    }

    /// Set all entries in the inclusive range `[start, end]` to `tile_id`.
    /// If `start > end` the call is a no-op.
    pub fn fill_range(&mut self, start: u8, end: u8, tile_id: u32) {
        if start > end {
            return;
        }
        for mask in start..=end {
            self.table[mask as usize] = tile_id;
        }
    }
}

impl crate::world_hash::DetHash for SimpleTileTable {
    fn det_hash(&self, hasher: &mut crate::world_hash::Fnv1a) {
        for &v in &self.table {
            hasher.write_u32(v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    // --- compute_mask ---

    fn all_same(_x: i32, _y: i32) -> bool {
        true
    }

    fn none_same(_x: i32, _y: i32) -> bool {
        false
    }

    #[test]
    fn test_mask_isolated_cell() {
        // No neighbours → all bits 0.
        let mask = compute_mask(5, 5, none_same);
        assert_eq!(mask, 0b0000_0000);
    }

    #[test]
    fn test_mask_surrounded_cell() {
        // All 8 neighbours present → all bits set.
        let mask = compute_mask(5, 5, all_same);
        assert_eq!(mask, 0b1111_1111);
    }

    #[test]
    fn test_mask_only_north_neighbour() {
        let mask = compute_mask(0, 0, |_x, y| y == -1);
        assert_eq!(mask & (1 << 0), 1 << 0); // North bit set
        assert_eq!(mask & (1 << 4), 0); // South bit clear
    }

    #[test]
    fn test_diagonal_cleared_without_cardinal() {
        // NE neighbour present but North is absent → NE bit should be 0.
        let mask = compute_mask(0, 0, |x, y| x == 1 && y == -1);
        // North bit 0 is clear (no N), so NE bit 1 must also be clear.
        assert_eq!(mask & (1 << 1), 0);
    }

    #[test]
    fn test_diagonal_set_when_cardinals_present() {
        // N, E, NE all present → NE bit set.
        let mask = compute_mask(5, 5, |x, y| {
            // N = (5, 4), E = (6, 5), NE = (6, 4)
            (x == 5 && y == 4) || (x == 6 && y == 5) || (x == 6 && y == 4)
        });
        assert_eq!(mask & (1 << 1), 1 << 1); // NE bit set
    }

    #[test]
    fn test_mask_north_and_south_only() {
        let mask = compute_mask(0, 0, |_x, y| y == -1 || y == 1);
        assert_eq!(mask & (1 << 0), 1 << 0); // N set
        assert_eq!(mask & (1 << 4), 1 << 4); // S set
        assert_eq!(mask & (1 << 2), 0); // E clear
        assert_eq!(mask & (1 << 6), 0); // W clear
    }

    // --- compute_all ---

    #[test]
    fn test_compute_all_size() {
        let masks = compute_all(4, 3, none_same);
        assert_eq!(masks.len(), 12);
    }

    #[test]
    fn test_compute_all_zero_size_returns_empty() {
        assert!(compute_all(0, 4, none_same).is_empty());
        assert!(compute_all(4, 0, none_same).is_empty());
    }

    #[test]
    fn test_compute_all_interior_surrounded() {
        // 3×3 grid all-same: the center cell (1,1) has 8 same-type neighbours.
        let masks = compute_all(3, 3, |x, y| (0..3).contains(&x) && (0..3).contains(&y));
        let center = masks[3 + 1]; // y=1, x=1
        assert_eq!(center, 0b1111_1111);
    }

    #[test]
    fn test_compute_all_border_cells_partial() {
        // Top-left corner (0,0) in a 3×3 all-same grid: N and W are absent.
        let masks = compute_all(3, 3, |x, y| (0..3).contains(&x) && (0..3).contains(&y));
        let tl = masks[0];
        // N(0), NE(1), NW(7) clear; W(6), NW(7) clear
        assert_eq!(tl & (1 << 0), 0); // N absent
        assert_eq!(tl & (1 << 6), 0); // W absent
    }

    // --- compute_region ---

    #[test]
    fn test_compute_region_size() {
        let masks = compute_region(2, 2, 3, 2, none_same);
        assert_eq!(masks.len(), 6); // 3 * 2
    }

    #[test]
    fn test_compute_region_matches_compute_all_slice() {
        // 5×5 all-same grid; region (1,1,3,3) should match the center 3×3 of compute_all.
        let is_same = |x: i32, y: i32| (0..5).contains(&x) && (0..5).contains(&y);
        let all = compute_all(5, 5, is_same);
        let region = compute_region(1, 1, 3, 3, is_same);
        // Extract same rows/cols from compute_all result.
        let mut expected = Vec::new();
        for ry in 1..4i32 {
            for rx in 1..4i32 {
                expected.push(all[(ry * 5 + rx) as usize]);
            }
        }
        assert_eq!(region, expected);
    }

    #[test]
    fn test_compute_region_zero_size_returns_empty() {
        assert!(compute_region(0, 0, 0, 3, none_same).is_empty());
        assert!(compute_region(0, 0, 3, 0, none_same).is_empty());
    }

    // --- SimpleTileTable ---

    #[test]
    fn test_table_default_zero() {
        let t = SimpleTileTable::new();
        assert_eq!(t.get(0), 0);
        assert_eq!(t.get(255), 0);
    }

    #[test]
    fn test_table_set_and_get() {
        let mut t = SimpleTileTable::new();
        t.set(0b0001_0101, 42);
        assert_eq!(t.get(0b0001_0101), 42);
    }

    #[test]
    fn test_table_det_hash_same() {
        let a = SimpleTileTable::new();
        let b = SimpleTileTable::new();
        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_table_det_hash_differs_after_set() {
        let a = SimpleTileTable::new();
        let mut b = SimpleTileTable::new();
        b.set(5, 99);
        assert_ne!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_fill_range_sets_all_entries() {
        let mut t = SimpleTileTable::new();
        t.fill_range(10, 14, 42);
        for mask in 10u8..=14 {
            assert_eq!(t.get(mask), 42, "mask={mask}");
        }
        // Adjacent entries outside the range remain 0.
        assert_eq!(t.get(9), 0);
        assert_eq!(t.get(15), 0);
    }

    #[test]
    fn test_fill_range_single_entry() {
        let mut t = SimpleTileTable::new();
        t.fill_range(7, 7, 99);
        assert_eq!(t.get(7), 99);
        assert_eq!(t.get(6), 0);
        assert_eq!(t.get(8), 0);
    }

    #[test]
    fn test_fill_range_inverted_is_noop() {
        let mut t = SimpleTileTable::new();
        t.fill_range(20, 10, 5); // start > end → no-op
        for mask in 10u8..=20 {
            assert_eq!(t.get(mask), 0);
        }
    }
}
