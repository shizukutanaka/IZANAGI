//! Multi-layer tile map for grid-based game worlds.
//!
//! `TileMap<T>` is a 2-D grid of cells of type `T` stored in row-major order.
//! Multiple layers are represented by a `Vec<TileMap<T>>` — callers choose how
//! many layers they need (terrain, objects, effects, …) and index them by
//! their own enum or constant. Each layer has the same width and height; that
//! invariant is enforced by `LayeredMap`, which bundles layers together and
//! ensures they are always the same size.
//!
//! Out-of-bounds access returns `None` rather than panicking so code that
//! ranges over the edges (e.g. A*, FOV) stays panic-free.
//!
//! `DetHash` for `TileMap<T: DetHash>` folds (width, height) then every cell
//! in row-major order so the hash is layout-independent.

use crate::world_hash::{DetHash, Fnv1a};

/// A single-layer 2-D tile grid.
#[derive(Clone, Debug)]
pub struct TileMap<T> {
    width: u32,
    height: u32,
    cells: Vec<T>,
}

impl<T: Clone> TileMap<T> {
    /// Create a map filled with `default` tiles.
    pub fn new(width: u32, height: u32, default: T) -> Self {
        TileMap {
            width,
            height,
            cells: vec![default; (width as usize).saturating_mul(height as usize)],
        }
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Total number of cells.
    #[inline]
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    #[inline]
    fn idx(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            return None;
        }
        Some(y as usize * self.width as usize + x as usize)
    }

    /// Get a reference to the tile at `(x, y)`, or `None` if out of bounds.
    pub fn get(&self, x: i32, y: i32) -> Option<&T> {
        self.cells.get(self.idx(x, y)?)
    }

    /// Get a mutable reference to the tile at `(x, y)`, or `None` if OOB.
    pub fn get_mut(&mut self, x: i32, y: i32) -> Option<&mut T> {
        let idx = self.idx(x, y)?;
        self.cells.get_mut(idx)
    }

    /// Set the tile at `(x, y)`. No-op if out of bounds.
    pub fn set(&mut self, x: i32, y: i32, tile: T) {
        if let Some(idx) = self.idx(x, y) {
            self.cells[idx] = tile;
        }
    }

    /// Fill the entire map with `tile`.
    pub fn fill(&mut self, tile: T) {
        self.cells.fill(tile);
    }

    /// Fill a rectangular region `[x, x+w) × [y, y+h)` with `tile`.
    /// Clips silently to the map boundary.
    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, tile: T) {
        let x0 = x.max(0) as u32;
        let y0 = y.max(0) as u32;
        let x1 = ((x + w.max(0)) as u32).min(self.width);
        let y1 = ((y + h.max(0)) as u32).min(self.height);
        for row in y0..y1 {
            for col in x0..x1 {
                let idx = row as usize * self.width as usize + col as usize;
                self.cells[idx] = tile.clone();
            }
        }
    }

    /// Copy a rectangular region `[x, x+w) × [y, y+h)` from this map, row by
    /// row. Cells outside the map boundary are replaced with `default`. The
    /// returned `Vec` has length `w * h` (or 0 for zero-area regions); index
    /// `row * w + col` gives the tile at offset `(col, row)` from `(x, y)`.
    pub fn copy_region(&self, x: i32, y: i32, w: i32, h: i32, default: T) -> Vec<T>
    where
        T: Clone,
    {
        if w <= 0 || h <= 0 {
            return Vec::new();
        }
        let mut out = Vec::with_capacity((w * h) as usize);
        for row in 0..h {
            for col in 0..w {
                let cell = self
                    .get(x + col, y + row)
                    .cloned()
                    .unwrap_or(default.clone());
                out.push(cell);
            }
        }
        out
    }

    /// Paste a `w × h` rectangular region from `data` into this map at `(x, y)`.
    /// `data` must have at least `w * h` elements (any excess is ignored); use
    /// index `row * w + col` for the tile at offset `(col, row)`. Clips silently
    /// to the map boundary.
    pub fn paste_region(&mut self, x: i32, y: i32, w: i32, h: i32, data: &[T])
    where
        T: Clone,
    {
        if w <= 0 || h <= 0 {
            return;
        }
        for row in 0..h {
            for col in 0..w {
                let src = row * w + col;
                if let Some(tile) = data.get(src as usize) {
                    self.set(x + col, y + row, tile.clone());
                }
            }
        }
    }

    /// Returns `true` if `(x, y)` is within the map boundaries.
    #[inline]
    pub fn contains(&self, x: i32, y: i32) -> bool {
        self.idx(x, y).is_some()
    }

    /// Swap the tiles at `(x1, y1)` and `(x2, y2)`. No-op if either coordinate
    /// is out of bounds or both coordinates are equal.
    pub fn swap(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        let Some(a) = self.idx(x1, y1) else { return };
        let Some(b) = self.idx(x2, y2) else { return };
        if a != b {
            self.cells.swap(a, b);
        }
    }

    /// Count cells for which `pred` returns `true`.
    pub fn count_where<P: Fn(&T) -> bool>(&self, pred: P) -> usize {
        self.cells.iter().filter(|t| pred(t)).count()
    }

    /// Collect `(x, y)` of all cells for which `pred` returns `true`, in
    /// row-major order. The equivalent of `iter().filter_map(|(x,y,t)| …)`.
    pub fn find_all<P: Fn(&T) -> bool>(&self, pred: P) -> Vec<(i32, i32)> {
        self.iter()
            .filter(|(_, _, t)| pred(t))
            .map(|(x, y, _)| (x, y))
            .collect()
    }

    /// Return the `(x, y)` position of the first cell (row-major order) for
    /// which `pred` returns `true`, or `None` if no cell matches. The
    /// single-result complement of [`find_all`](Self::find_all) — avoids
    /// allocating a full `Vec` when only the first match is needed.
    pub fn find_first<P: Fn(&T) -> bool>(&self, pred: P) -> Option<(i32, i32)> {
        self.iter()
            .find(|(_, _, t)| pred(t))
            .map(|(x, y, _)| (x, y))
    }

    /// The minimal [`Aabb`](crate::aabb::Aabb) enclosing every cell for which
    /// `pred` returns `true`, or `None` if no cell matches. The returned box
    /// uses the half-open convention (`right = max_x + 1`), so a single matching
    /// cell yields a `1×1` box. Useful for "bounds of this room / spell area /
    /// painted region" without manual min/max bookkeeping.
    pub fn bounds_of<P: Fn(&T) -> bool>(&self, pred: P) -> Option<crate::aabb::Aabb> {
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        for (x, y, tile) in self.iter() {
            if pred(tile) {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
        if min_x > max_x {
            return None;
        }
        Some(crate::aabb::Aabb::from_corners(
            min_x,
            min_y,
            max_x + 1,
            max_y + 1,
        ))
    }

    /// Apply `transform` to every cell for which `pred` returns `true`,
    /// replacing the cell value in place. `transform` receives the current
    /// value by move and returns the new one (allows non-`Copy` items).
    ///
    /// The typical "apply poison decay to all poisoned tiles" call:
    /// `map.mutate_where(|t| t.poisoned, |mut t| { t.hp -= 1; t })`.
    pub fn mutate_where<P, F>(&mut self, pred: P, mut transform: F)
    where
        P: Fn(&T) -> bool,
        F: FnMut(T) -> T,
    {
        for cell in &mut self.cells {
            if pred(cell) {
                // Temporarily replace with a clone to satisfy the borrow
                // checker; the original value is moved into `transform`.
                let old = cell.clone();
                *cell = transform(old);
            }
        }
    }

    /// Iterate `(x, y, &tile)` in row-major order.
    pub fn iter(&self) -> impl Iterator<Item = (i32, i32, &T)> {
        let w = self.width;
        self.cells
            .iter()
            .enumerate()
            .map(move |(i, t)| ((i % w as usize) as i32, (i / w as usize) as i32, t))
    }

    /// Iterate `(x, y, &mut tile)` in row-major order, yielding mutable access
    /// to each cell. Useful for in-place map updates such as ageing fog-of-war,
    /// applying environmental effects, or re-colouring tiles after an event.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (i32, i32, &mut T)> {
        let w = self.width;
        self.cells
            .iter_mut()
            .enumerate()
            .map(move |(i, t)| ((i % w as usize) as i32, (i / w as usize) as i32, t))
    }
}

impl<T: Clone + DetHash> DetHash for TileMap<T> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.width);
        hasher.write_u32(self.height);
        for t in &self.cells {
            t.det_hash(hasher);
        }
    }
}

// ---------------------------------------------------------------------------
// LayeredMap
// ---------------------------------------------------------------------------

/// A fixed set of `TileMap` layers all sharing the same dimensions.
///
/// Layers are indexed by `usize` (callers typically alias indices with
/// constants or an enum).  The layer count is fixed at construction; use
/// `layer(i)` / `layer_mut(i)` to access individual layers.
#[derive(Clone, Debug)]
pub struct LayeredMap<T> {
    layers: Vec<TileMap<T>>,
    width: u32,
    height: u32,
}

impl<T: Clone> LayeredMap<T> {
    /// Create a map with `layer_count` layers, each filled with `default`.
    pub fn new(width: u32, height: u32, layer_count: usize, default: T) -> Self {
        LayeredMap {
            layers: (0..layer_count)
                .map(|_| TileMap::new(width, height, default.clone()))
                .collect(),
            width,
            height,
        }
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[inline]
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Borrow layer `i`, or `None` if `i` is out of range.
    pub fn layer(&self, i: usize) -> Option<&TileMap<T>> {
        self.layers.get(i)
    }

    /// Mutably borrow layer `i`, or `None` if `i` is out of range.
    pub fn layer_mut(&mut self, i: usize) -> Option<&mut TileMap<T>> {
        self.layers.get_mut(i)
    }

    /// Convenience: get a tile from layer `layer_idx` at `(x, y)`.
    pub fn get(&self, layer_idx: usize, x: i32, y: i32) -> Option<&T> {
        self.layers.get(layer_idx)?.get(x, y)
    }

    /// Convenience: set a tile in layer `layer_idx` at `(x, y)`.
    pub fn set(&mut self, layer_idx: usize, x: i32, y: i32, tile: T) {
        if let Some(layer) = self.layers.get_mut(layer_idx) {
            layer.set(x, y, tile);
        }
    }

    /// Fill every cell of every layer with `tile`. The one-call equivalent of
    /// calling `layer_mut(i).fill(tile)` for all layers.
    pub fn fill_all(&mut self, tile: T) {
        for layer in &mut self.layers {
            layer.fill(tile.clone());
        }
    }
}

impl<T: Clone + DetHash> DetHash for LayeredMap<T> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.width);
        hasher.write_u32(self.height);
        hasher.write_u32(self.layers.len() as u32);
        for layer in &self.layers {
            layer.det_hash(hasher);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    // --- TileMap ---

    #[test]
    fn test_new_fills_default() {
        let m: TileMap<u8> = TileMap::new(4, 3, 0);
        assert_eq!(m.width(), 4);
        assert_eq!(m.height(), 3);
        assert_eq!(m.len(), 12);
        assert_eq!(m.get(0, 0), Some(&0u8));
    }

    #[test]
    fn test_set_and_get() {
        let mut m: TileMap<u8> = TileMap::new(4, 4, 0);
        m.set(2, 3, 99);
        assert_eq!(m.get(2, 3), Some(&99));
        assert_eq!(m.get(0, 0), Some(&0));
    }

    #[test]
    fn test_get_oob_returns_none() {
        let m: TileMap<u8> = TileMap::new(4, 4, 0);
        assert_eq!(m.get(-1, 0), None);
        assert_eq!(m.get(4, 0), None);
        assert_eq!(m.get(0, -1), None);
        assert_eq!(m.get(0, 4), None);
    }

    #[test]
    fn test_set_oob_is_noop() {
        let mut m: TileMap<u8> = TileMap::new(4, 4, 0);
        m.set(10, 10, 99); // should not panic
        assert_eq!(m.get(0, 0), Some(&0));
    }

    #[test]
    fn test_get_mut_modify() {
        let mut m: TileMap<u8> = TileMap::new(4, 4, 0);
        *m.get_mut(1, 2).unwrap() = 7;
        assert_eq!(m.get(1, 2), Some(&7));
    }

    #[test]
    fn test_fill() {
        let mut m: TileMap<u8> = TileMap::new(3, 3, 0);
        m.fill(5);
        for (_, _, &v) in m.iter() {
            assert_eq!(v, 5);
        }
    }

    #[test]
    fn test_fill_rect() {
        let mut m: TileMap<u8> = TileMap::new(5, 5, 0);
        m.fill_rect(1, 1, 3, 3, 1);
        assert_eq!(m.get(0, 0), Some(&0)); // outside
        assert_eq!(m.get(1, 1), Some(&1)); // inside
        assert_eq!(m.get(3, 3), Some(&1)); // inside (last col/row)
        assert_eq!(m.get(4, 4), Some(&0)); // outside (x+w = 4, exclusive)
    }

    #[test]
    fn test_iter_row_major_order() {
        let mut m: TileMap<u8> = TileMap::new(3, 2, 0);
        m.set(1, 0, 10);
        m.set(2, 1, 20);
        let items: Vec<(i32, i32, u8)> = m.iter().map(|(x, y, &v)| (x, y, v)).collect();
        assert_eq!(items[0], (0, 0, 0));
        assert_eq!(items[1], (1, 0, 10));
        assert_eq!(items[5], (2, 1, 20));
    }

    #[test]
    fn test_iter_mut_updates_in_place() {
        let mut m: TileMap<u8> = TileMap::new(3, 3, 0);
        m.set(1, 1, 5);
        for (_, _, v) in m.iter_mut() {
            *v = v.saturating_add(10);
        }
        assert_eq!(m.get(1, 1), Some(&15));
        assert_eq!(m.get(0, 0), Some(&10)); // 0 + 10
    }

    #[test]
    fn test_iter_mut_preserves_row_major_coords() {
        let mut m: TileMap<u8> = TileMap::new(4, 3, 0);
        for (x, y, v) in m.iter_mut() {
            *v = (y as u8) * 4 + x as u8;
        }
        assert_eq!(m.get(0, 0), Some(&0));
        assert_eq!(m.get(3, 0), Some(&3));
        assert_eq!(m.get(0, 1), Some(&4));
        assert_eq!(m.get(3, 2), Some(&11));
    }

    #[test]
    fn test_det_hash_equal_maps_equal_hash() {
        let a: TileMap<u8> = TileMap::new(4, 4, 0);
        let b: TileMap<u8> = TileMap::new(4, 4, 0);
        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_det_hash_differs_on_change() {
        let a: TileMap<u8> = TileMap::new(4, 4, 0);
        let mut b: TileMap<u8> = TileMap::new(4, 4, 0);
        b.set(2, 2, 1);
        assert_ne!(hash_state(&a), hash_state(&b));
    }

    // --- LayeredMap ---

    #[test]
    fn test_layered_map_layer_count() {
        let m: LayeredMap<u8> = LayeredMap::new(10, 10, 3, 0);
        assert_eq!(m.layer_count(), 3);
        assert_eq!(m.width(), 10);
        assert_eq!(m.height(), 10);
    }

    #[test]
    fn test_layered_map_set_and_get() {
        let mut m: LayeredMap<u8> = LayeredMap::new(5, 5, 2, 0);
        m.set(0, 2, 3, 42);
        m.set(1, 2, 3, 7);
        assert_eq!(m.get(0, 2, 3), Some(&42));
        assert_eq!(m.get(1, 2, 3), Some(&7));
    }

    #[test]
    fn test_layered_map_oob_layer_returns_none() {
        let m: LayeredMap<u8> = LayeredMap::new(5, 5, 2, 0);
        assert!(m.layer(99).is_none());
        assert!(m.get(99, 0, 0).is_none());
    }

    #[test]
    fn test_layered_map_layers_are_independent() {
        let mut m: LayeredMap<u8> = LayeredMap::new(5, 5, 2, 0);
        m.set(0, 1, 1, 99);
        // layer 1 at same position should still be 0
        assert_eq!(m.get(1, 1, 1), Some(&0));
    }

    #[test]
    fn test_copy_region_basic() {
        let mut m: TileMap<u8> = TileMap::new(4, 4, 0);
        m.set(1, 1, 10);
        m.set(2, 1, 20);
        m.set(1, 2, 30);
        m.set(2, 2, 40);
        let data = m.copy_region(1, 1, 2, 2, 0);
        assert_eq!(data, vec![10, 20, 30, 40]);
    }

    #[test]
    fn test_copy_region_oob_fills_default() {
        let m: TileMap<u8> = TileMap::new(2, 2, 7);
        // Region starting at (-1,-1) — half out of bounds.
        let data = m.copy_region(-1, -1, 3, 3, 99);
        assert_eq!(data.len(), 9); // 3*3
                                   // Top-left corner cells are out of bounds → 99
        assert_eq!(data[0], 99); // (-1,-1) → OOB
                                 // (0,0) is in bounds → 7
        assert_eq!(data[4], 7); // (0+1=0, 0+1=0) in row 1, col 1 of the copy
    }

    #[test]
    fn test_copy_region_zero_size_returns_empty() {
        let m: TileMap<u8> = TileMap::new(4, 4, 1);
        assert!(m.copy_region(0, 0, 0, 4, 0).is_empty());
        assert!(m.copy_region(0, 0, 4, 0, 0).is_empty());
    }

    #[test]
    fn test_paste_region_basic() {
        let mut m: TileMap<u8> = TileMap::new(4, 4, 0);
        let data = vec![1u8, 2, 3, 4];
        m.paste_region(1, 1, 2, 2, &data);
        assert_eq!(m.get(1, 1), Some(&1));
        assert_eq!(m.get(2, 1), Some(&2));
        assert_eq!(m.get(1, 2), Some(&3));
        assert_eq!(m.get(2, 2), Some(&4));
        // Other cells unchanged
        assert_eq!(m.get(0, 0), Some(&0));
    }

    #[test]
    fn test_copy_paste_roundtrip() {
        let mut src: TileMap<u8> = TileMap::new(4, 4, 0);
        src.set(0, 0, 1);
        src.set(1, 0, 2);
        src.set(0, 1, 3);
        src.set(1, 1, 4);
        let data = src.copy_region(0, 0, 2, 2, 0);
        let mut dst: TileMap<u8> = TileMap::new(4, 4, 0);
        dst.paste_region(2, 2, 2, 2, &data);
        assert_eq!(dst.get(2, 2), Some(&1));
        assert_eq!(dst.get(3, 2), Some(&2));
        assert_eq!(dst.get(2, 3), Some(&3));
        assert_eq!(dst.get(3, 3), Some(&4));
    }

    #[test]
    fn test_contains_in_bounds() {
        let m: TileMap<u8> = TileMap::new(4, 3, 0);
        assert!(m.contains(0, 0));
        assert!(m.contains(3, 2));
        assert!(!m.contains(-1, 0));
        assert!(!m.contains(4, 0));
        assert!(!m.contains(0, 3));
    }

    #[test]
    fn test_swap_two_cells() {
        let mut m: TileMap<u8> = TileMap::new(4, 4, 0);
        m.set(1, 0, 10);
        m.set(2, 3, 20);
        m.swap(1, 0, 2, 3);
        assert_eq!(m.get(1, 0), Some(&20));
        assert_eq!(m.get(2, 3), Some(&10));
    }

    #[test]
    fn test_swap_oob_is_noop() {
        let mut m: TileMap<u8> = TileMap::new(3, 3, 0);
        m.set(0, 0, 5);
        m.swap(0, 0, 99, 99); // OOB second coord
        assert_eq!(m.get(0, 0), Some(&5)); // unchanged
    }

    #[test]
    fn test_swap_same_cell_is_noop() {
        let mut m: TileMap<u8> = TileMap::new(3, 3, 7);
        m.swap(1, 1, 1, 1);
        assert_eq!(m.get(1, 1), Some(&7));
    }

    #[test]
    fn test_count_where() {
        let mut m: TileMap<u8> = TileMap::new(4, 4, 0);
        m.set(0, 0, 1);
        m.set(1, 1, 1);
        m.set(2, 2, 2);
        assert_eq!(m.count_where(|&v| v == 1), 2);
        assert_eq!(m.count_where(|&v| v == 0), 13);
        assert_eq!(m.count_where(|&v| v > 0), 3);
    }

    #[test]
    fn test_fill_all_fills_every_layer() {
        let mut m: LayeredMap<u8> = LayeredMap::new(3, 3, 3, 0);
        m.fill_all(7);
        for i in 0..3 {
            assert_eq!(m.get(i, 1, 1), Some(&7));
        }
    }

    #[test]
    fn test_layered_map_det_hash() {
        let a: LayeredMap<u8> = LayeredMap::new(4, 4, 2, 0);
        let b: LayeredMap<u8> = LayeredMap::new(4, 4, 2, 0);
        assert_eq!(hash_state(&a), hash_state(&b));
        let mut c = a.clone();
        c.set(1, 0, 0, 5);
        assert_ne!(hash_state(&b), hash_state(&c));
    }

    #[test]
    fn test_find_all_returns_matching_coords() {
        let mut m: TileMap<u8> = TileMap::new(3, 3, 0);
        m.set(0, 0, 1);
        m.set(2, 2, 1);
        let coords = m.find_all(|&t| t == 1);
        assert_eq!(coords.len(), 2);
        assert!(coords.contains(&(0, 0)));
        assert!(coords.contains(&(2, 2)));
    }

    #[test]
    fn test_find_all_empty_when_no_match() {
        let m: TileMap<u8> = TileMap::new(4, 4, 0);
        assert!(m.find_all(|&t| t == 99).is_empty());
    }

    #[test]
    fn test_find_all_row_major_order() {
        let mut m: TileMap<u8> = TileMap::new(3, 2, 0);
        m.set(1, 0, 1);
        m.set(0, 1, 1);
        let coords = m.find_all(|&t| t == 1);
        // Row-major: (1,0) comes before (0,1)
        assert_eq!(coords, [(1, 0), (0, 1)]);
    }

    #[test]
    fn test_mutate_where_transforms_matching_cells() {
        let mut m: TileMap<u8> = TileMap::new(3, 3, 0);
        m.set(1, 1, 5);
        m.set(2, 0, 5);
        m.mutate_where(|&t| t == 5, |t| t * 2);
        assert_eq!(m.get(1, 1), Some(&10));
        assert_eq!(m.get(2, 0), Some(&10));
        assert_eq!(m.get(0, 0), Some(&0)); // unmatched unchanged
    }

    #[test]
    fn test_mutate_where_no_match_unchanged() {
        let mut m: TileMap<u8> = TileMap::new(2, 2, 3);
        m.mutate_where(|&t| t == 99, |t| t + 1);
        assert!(m.iter().all(|(_, _, &t)| t == 3));
    }

    #[test]
    fn test_mutate_where_all_cells() {
        let mut m: TileMap<u8> = TileMap::new(2, 2, 1);
        m.mutate_where(|_| true, |t| t + 10);
        assert!(m.iter().all(|(_, _, &t)| t == 11));
    }

    #[test]
    fn test_bounds_of_single_cell_is_one_by_one() {
        let mut m: TileMap<u8> = TileMap::new(5, 5, 0);
        m.set(2, 3, 1);
        let b = m.bounds_of(|&t| t == 1).unwrap();
        assert_eq!((b.x, b.y, b.w, b.h), (2, 3, 1, 1));
    }

    #[test]
    fn test_bounds_of_scattered_cells_minimal_box() {
        let mut m: TileMap<u8> = TileMap::new(8, 8, 0);
        m.set(1, 1, 1);
        m.set(4, 2, 1);
        m.set(2, 5, 1);
        let b = m.bounds_of(|&t| t == 1).unwrap();
        // min (1,1), max (4,5) → half-open box [1,5) x [1,6)
        assert_eq!((b.x, b.y, b.w, b.h), (1, 1, 4, 5));
    }

    #[test]
    fn test_bounds_of_no_match_returns_none() {
        let m: TileMap<u8> = TileMap::new(4, 4, 0);
        assert!(m.bounds_of(|&t| t == 99).is_none());
    }

    #[test]
    fn test_find_first_returns_none_on_no_match() {
        let m: TileMap<u8> = TileMap::new(4, 4, 0);
        assert!(m.find_first(|&t| t == 99).is_none());
    }

    #[test]
    fn test_find_first_returns_first_match_row_major() {
        let mut m: TileMap<u8> = TileMap::new(4, 4, 0);
        m.set(3, 3, 1);
        m.set(0, 1, 1); // earlier in row-major order (y=1 < y=3)
        assert_eq!(m.find_first(|&t| t == 1), Some((0, 1)));
    }

    #[test]
    fn test_find_first_single_cell_map() {
        let m: TileMap<u8> = TileMap::new(1, 1, 7);
        assert_eq!(m.find_first(|&t| t == 7), Some((0, 0)));
    }
}
