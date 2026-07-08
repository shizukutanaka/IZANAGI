//! Additive integer illumination map for torchlit dungeons.
//!
//! [`compute_fov`](crate::fov::compute_fov) answers *"which cells are
//! visible?"* (boolean reachability). A [`LightMap`] answers *"how bright is
//! each cell?"* (0–255 integer illumination). The two concepts are orthogonal
//! and compose naturally: render bright cells for visible tiles, dim cells for
//! remembered tiles, and dark cells for the unknown.
//!
//! Each call to [`add_light`](LightMap::add_light) places a circular light
//! source: at distance `d` from the centre, the contribution is
//! `intensity × (radius − d) / radius` (linear falloff, integer division), or
//! `intensity` if `radius == 0` (point light, centre only). Contributions from
//! multiple overlapping sources are accumulated with **saturating add** — the
//! order of calls does not matter (commutativity) and no cell ever exceeds 255.
//!
//! ```
//! use izanagi_kit::lightmap::LightMap;
//!
//! let mut lm = LightMap::new(16, 16);
//! lm.add_light(8, 8, 4, 200); // torch near centre
//! lm.add_light(4, 4, 3, 100); // a dimmer source in the corner
//!
//! assert_eq!(lm.get(8, 8), 200); // centre of first torch
//! assert!(lm.get(12, 8) < lm.get(8, 8), "falloff with distance");
//! assert!(lm.get(4, 4) >= 100, "second source illuminates its centre");
//! assert_eq!(lm.get(0, 15), 0, "far corner stays dark");
//! ```
//!
//! Determinism: every value is a pure function of the source parameters and
//! map dimensions, computed entirely with integer arithmetic and saturating
//! addition. [`LightMap`] implements [`DetHash`]
//! over its full cell array, folding the current illumination state into the
//! replay checksum.

use crate::world_hash::{DetHash, Fnv1a};

/// The maximum illumination value a cell can hold.
pub const MAX_LIGHT: u16 = 255;

/// A grid of integer illumination levels, accumulated from multiple light sources.
#[derive(Clone, Debug)]
pub struct LightMap {
    width: u32,
    height: u32,
    /// Row-major: `levels[y * width + x]`
    levels: Vec<u16>,
}

impl LightMap {
    /// Create a dark (all-zero) light map.
    pub fn new(width: u32, height: u32) -> Self {
        let n = (width as usize).saturating_mul(height as usize);
        LightMap {
            width,
            height,
            levels: vec![0; n],
        }
    }

    /// Map dimensions.
    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }
    /// Map height in cells.
    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Total number of cells.
    #[inline]
    pub fn len(&self) -> usize {
        self.levels.len()
    }

    /// `true` if the map has no cells (`width == 0 || height == 0`).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.levels.is_empty()
    }

    /// The illumination of cell `(x, y)`. Returns `0` for out-of-bounds
    /// coordinates.
    #[inline]
    pub fn get(&self, x: i32, y: i32) -> u16 {
        match self.index(x, y) {
            Some(i) => self.levels[i],
            None => 0,
        }
    }

    /// `true` if cell `(x, y)` has any illumination (`> 0`).
    #[inline]
    pub fn is_lit(&self, x: i32, y: i32) -> bool {
        self.get(x, y) > 0
    }

    /// Set all cells to zero — ready for a fresh frame.
    pub fn clear(&mut self) {
        self.levels.iter_mut().for_each(|v| *v = 0);
    }

    /// Fill every cell with `level` (useful for ambient outdoor light).
    pub fn ambient_fill(&mut self, level: u16) {
        self.levels.iter_mut().for_each(|v| *v = level);
    }

    /// Place a circular light source at `(cx, cy)` with Chebyshev `radius` and
    /// peak `intensity`. Cells farther than `radius` receive no contribution.
    ///
    /// **Falloff** (linear by Chebyshev distance `d`):
    /// - `radius == 0`: only the centre cell is lit, at `intensity`.
    /// - `radius > 0`: contribution = `intensity × (radius − d) / radius` where
    ///   division is truncating integer division.
    ///
    /// Contributions are **saturating-added** to the current level, so overlapping
    /// sources accumulate and no cell exceeds [`MAX_LIGHT`].
    pub fn add_light(&mut self, cx: i32, cy: i32, radius: i32, intensity: u16) {
        if intensity == 0 {
            return;
        }
        let r = radius.max(0);
        // Bounding box clipped to the map.
        let x0 = (cx - r).max(0) as u32;
        let y0 = (cy - r).max(0) as u32;
        let x1 = (cx + r).min(self.width as i32 - 1).max(-1);
        let y1 = (cy + r).min(self.height as i32 - 1).max(-1);
        if x1 < 0 || y1 < 0 {
            return;
        }
        let x1 = x1 as u32;
        let y1 = y1 as u32;

        for y in y0..=y1 {
            for x in x0..=x1 {
                let d = chebyshev(cx, cy, x as i32, y as i32);
                let contribution = if r == 0 {
                    intensity
                } else {
                    (intensity as u32 * (r - d) as u32 / r as u32) as u16
                };
                let idx = y as usize * self.width as usize + x as usize;
                self.levels[idx] = self.levels[idx].saturating_add(contribution).min(MAX_LIGHT);
            }
        }
    }

    /// The maximum illumination level currently on the map.
    pub fn max_level(&self) -> u16 {
        self.levels.iter().copied().max().unwrap_or(0)
    }

    /// The number of cells with illumination `> 0`.
    pub fn lit_count(&self) -> usize {
        self.levels.iter().filter(|&&v| v > 0).count()
    }

    /// Iterate over `(x, y, level)` for **all** cells in row-major order.
    pub fn iter(&self) -> impl Iterator<Item = (i32, i32, u16)> + '_ {
        self.levels.iter().enumerate().map(move |(i, &v)| {
            let x = (i % self.width as usize) as i32;
            let y = (i / self.width as usize) as i32;
            (x, y, v)
        })
    }

    #[inline]
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        Some(y as usize * self.width as usize + x as usize)
    }
}

#[inline]
fn chebyshev(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    (ax - bx).unsigned_abs().max((ay - by).unsigned_abs()) as i32
}

impl DetHash for LightMap {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.width);
        hasher.write_u32(self.height);
        for &v in &self.levels {
            hasher.write_u16(v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[test]
    fn test_new_is_dark() {
        let lm = LightMap::new(8, 8);
        assert_eq!(lm.max_level(), 0);
        assert_eq!(lm.lit_count(), 0);
        assert!(!lm.is_lit(4, 4));
    }

    #[test]
    fn test_oob_returns_zero() {
        let lm = LightMap::new(4, 4);
        assert_eq!(lm.get(-1, 0), 0);
        assert_eq!(lm.get(4, 0), 0);
        assert_eq!(lm.get(0, 4), 0);
    }

    #[test]
    fn test_point_light_radius_zero() {
        let mut lm = LightMap::new(8, 8);
        lm.add_light(3, 3, 0, 200);
        assert_eq!(lm.get(3, 3), 200, "centre at full intensity");
        assert_eq!(lm.get(4, 3), 0, "adjacent cell stays dark");
        assert_eq!(lm.get(2, 3), 0);
    }

    #[test]
    fn test_falloff_is_monotone() {
        let mut lm = LightMap::new(16, 8);
        lm.add_light(8, 4, 6, 240);
        // Horizontal slice from centre outward.
        let mut prev = lm.get(8, 4);
        for dx in 1..=6i32 {
            let cur = lm.get(8 + dx, 4);
            assert!(
                cur <= prev,
                "light must not increase with distance (dx={dx})"
            );
            prev = cur;
        }
        assert_eq!(lm.get(8 + 7, 4), 0, "beyond radius is dark");
    }

    #[test]
    fn test_additive_superposition() {
        // Two lights summed ≥ either alone (saturating add is non-negative).
        let mut a = LightMap::new(12, 12);
        let mut b = LightMap::new(12, 12);
        let mut both = LightMap::new(12, 12);
        a.add_light(3, 3, 4, 100);
        b.add_light(9, 9, 4, 80);
        both.add_light(3, 3, 4, 100);
        both.add_light(9, 9, 4, 80);
        for (x, y, v) in both.iter() {
            assert!(v >= a.get(x, y), "superposition >= source A at ({x},{y})");
            assert!(v >= b.get(x, y), "superposition >= source B at ({x},{y})");
        }
    }

    #[test]
    fn test_commutativity_of_sources() {
        let mut ab = LightMap::new(10, 10);
        let mut ba = LightMap::new(10, 10);
        ab.add_light(2, 2, 3, 150);
        ab.add_light(7, 7, 3, 90);
        ba.add_light(7, 7, 3, 90);
        ba.add_light(2, 2, 3, 150);
        for (x, y, v) in ab.iter() {
            assert_eq!(v, ba.get(x, y), "add order must not matter at ({x},{y})");
        }
    }

    #[test]
    fn test_clamped_at_max() {
        let mut lm = LightMap::new(4, 4);
        for _ in 0..10 {
            lm.add_light(2, 2, 0, 200);
        }
        assert_eq!(lm.get(2, 2), MAX_LIGHT, "must saturate at MAX_LIGHT");
    }

    #[test]
    fn test_clear_resets_to_dark() {
        let mut lm = LightMap::new(8, 8);
        lm.add_light(4, 4, 3, 200);
        assert!(lm.max_level() > 0);
        lm.clear();
        assert_eq!(lm.max_level(), 0);
        assert_eq!(lm.lit_count(), 0);
    }

    #[test]
    fn test_ambient_fill() {
        let mut lm = LightMap::new(5, 5);
        lm.ambient_fill(50);
        assert_eq!(lm.get(0, 0), 50);
        assert_eq!(lm.get(4, 4), 50);
        assert_eq!(lm.lit_count(), 25);
        lm.add_light(2, 2, 0, 100);
        assert_eq!(lm.get(2, 2), 150, "torch adds on top of ambient");
    }

    #[test]
    fn test_zero_intensity_is_noop() {
        let mut lm = LightMap::new(6, 6);
        lm.add_light(3, 3, 5, 0);
        assert_eq!(lm.max_level(), 0, "zero intensity must be a no-op");
    }

    #[test]
    fn test_iter_covers_all_cells() {
        let lm = LightMap::new(3, 4);
        let cells: Vec<_> = lm.iter().collect();
        assert_eq!(cells.len(), 12);
    }

    #[test]
    fn test_det_hash_canonical_and_sensitive() {
        let mut a = LightMap::new(8, 8);
        a.add_light(4, 4, 3, 200);
        let mut b = LightMap::new(8, 8);
        b.add_light(4, 4, 3, 200);
        assert_eq!(hash_state(&a), hash_state(&b), "same state, same hash");
        let mut c = a.clone();
        c.add_light(2, 2, 1, 50);
        assert_ne!(
            hash_state(&a),
            hash_state(&c),
            "extra light must change hash"
        );
    }
}
