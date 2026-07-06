//! Tilemap — grid-based world storage with camera-culled rendering.
//!
//! ```
//! use izanagi::tilemap::Tilemap;
//!
//! let mut map = Tilemap::new(20, 15, 16.0); // 20×15 tiles, 16px each
//! map.set(5, 3, 1); // place tile id 1 at grid (5, 3)
//! assert_eq!(map.get(5, 3), 1);
//! ```

use crate::math::{Rect, Vec2};

/// A flat grid of tile IDs. 0 = empty/air.
pub struct Tilemap {
    tiles: Vec<u16>,
    /// Grid width in tiles.
    pub cols: u32,
    /// Grid height in tiles.
    pub rows: u32,
    /// Pixel size of each tile (square).
    pub tile_size: f32,
}

impl Tilemap {
    /// New tilemap, all tiles 0 (empty).
    pub fn new(cols: u32, rows: u32, tile_size: f32) -> Self {
        Self {
            tiles: vec![0; (cols * rows) as usize],
            cols,
            rows,
            tile_size: tile_size.max(1.0),
        }
    }

    /// Get tile ID at grid coordinate. Returns 0 for out-of-bounds.
    pub fn get(&self, col: i32, row: i32) -> u16 {
        if col < 0 || row < 0 || col >= self.cols as i32 || row >= self.rows as i32 {
            return 0;
        }
        self.tiles[row as usize * self.cols as usize + col as usize]
    }

    /// Set tile ID. No-op for out-of-bounds.
    pub fn set(&mut self, col: i32, row: i32, id: u16) {
        if col < 0 || row < 0 || col >= self.cols as i32 || row >= self.rows as i32 {
            return;
        }
        self.tiles[row as usize * self.cols as usize + col as usize] = id;
    }

    /// Fill a rectangular region with `id`.
    pub fn fill(&mut self, col: i32, row: i32, w: u32, h: u32, id: u16) {
        for r in row..row + h as i32 {
            for c in col..col + w as i32 {
                self.set(c, r, id);
            }
        }
    }

    /// World-space bounding box of the entire map.
    pub fn world_rect(&self) -> Rect {
        Rect::new(0.0, 0.0, self.cols as f32 * self.tile_size, self.rows as f32 * self.tile_size)
    }

    /// World position of the top-left corner of tile (col, row).
    pub fn tile_world_pos(&self, col: i32, row: i32) -> Vec2 {
        Vec2::new(col as f32 * self.tile_size, row as f32 * self.tile_size)
    }

    /// Grid coordinate of the tile containing world point `p`.
    pub fn world_to_tile(&self, p: Vec2) -> (i32, i32) {
        ((p.x / self.tile_size).floor() as i32, (p.y / self.tile_size).floor() as i32)
    }

    /// Iterate tiles visible in `view` (world-space rect), yielding (col, row, id).
    /// Skips empty (id == 0) tiles.
    pub fn visible_tiles<'a>(&'a self, view: &Rect) -> impl Iterator<Item = (i32, i32, u16)> + 'a {
        let ts = self.tile_size;
        let c0 = ((view.x / ts).floor() as i32).max(0);
        let r0 = ((view.y / ts).floor() as i32).max(0);
        let c1 = (((view.x + view.w) / ts).ceil() as i32).min(self.cols as i32);
        let r1 = (((view.y + view.h) / ts).ceil() as i32).min(self.rows as i32);

        (r0..r1).flat_map(move |r| {
            (c0..c1).filter_map(move |c| {
                let id = self.get(c, r);
                if id != 0 {
                    Some((c, r, id))
                } else {
                    None
                }
            })
        })
    }

    /// Is the tile at (col, row) solid (id != 0)?
    pub fn is_solid(&self, col: i32, row: i32) -> bool {
        self.get(col, row) != 0
    }

    /// Is the world-space point inside any solid tile?
    pub fn is_solid_at(&self, p: Vec2) -> bool {
        let (c, r) = self.world_to_tile(p);
        self.is_solid(c, r)
    }

    /// Total non-empty tile count.
    pub fn tile_count(&self) -> usize {
        self.tiles.iter().filter(|&&t| t != 0).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_roundtrip() {
        let mut m = Tilemap::new(10, 10, 16.0);
        m.set(3, 4, 42);
        assert_eq!(m.get(3, 4), 42);
        assert_eq!(m.get(0, 0), 0);
    }

    #[test]
    fn oob_returns_zero_and_is_noop() {
        let mut m = Tilemap::new(5, 5, 16.0);
        assert_eq!(m.get(-1, 0), 0);
        m.set(-1, 0, 99); // no panic
        m.set(100, 100, 99); // no panic
    }

    #[test]
    fn fill_region() {
        let mut m = Tilemap::new(10, 10, 16.0);
        m.fill(1, 1, 3, 2, 7);
        assert_eq!(m.get(1, 1), 7);
        assert_eq!(m.get(3, 2), 7);
        assert_eq!(m.get(0, 0), 0);
        assert_eq!(m.tile_count(), 6);
    }

    #[test]
    fn world_to_tile_roundtrip() {
        let m = Tilemap::new(20, 20, 16.0);
        let (c, r) = m.world_to_tile(Vec2::new(48.5, 32.1));
        assert_eq!(c, 3);
        assert_eq!(r, 2);
    }

    #[test]
    fn visible_tiles_skips_empty() {
        let mut m = Tilemap::new(10, 10, 16.0);
        m.set(2, 2, 1);
        m.set(3, 3, 2);
        let view = Rect::new(0.0, 0.0, 160.0, 160.0);
        let tiles: Vec<_> = m.visible_tiles(&view).collect();
        assert_eq!(tiles.len(), 2);
    }

    #[test]
    fn solid_at() {
        let mut m = Tilemap::new(10, 10, 16.0);
        m.set(1, 1, 1);
        assert!(m.is_solid_at(Vec2::new(20.0, 20.0))); // inside tile (1,1)
        assert!(!m.is_solid_at(Vec2::new(0.5, 0.5))); // tile (0,0) is empty
    }
}
