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

    /// Iterate `(x, y, &tile)` in row-major order.
    pub fn iter(&self) -> impl Iterator<Item = (i32, i32, &T)> {
        let w = self.width;
        self.cells
            .iter()
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
    fn test_layered_map_det_hash() {
        let a: LayeredMap<u8> = LayeredMap::new(4, 4, 2, 0);
        let b: LayeredMap<u8> = LayeredMap::new(4, 4, 2, 0);
        assert_eq!(hash_state(&a), hash_state(&b));
        let mut c = a.clone();
        c.set(1, 0, 0, 5);
        assert_ne!(hash_state(&b), hash_state(&c));
    }
}
