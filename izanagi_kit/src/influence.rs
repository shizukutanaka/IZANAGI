//! Grid-based influence map for game AI.
//!
//! An influence map is a scalar field over a 2-D grid where each cell holds
//! an `i32` value representing cumulative influence from one or more sources
//! (e.g. threat from enemies, attraction to items, territory control).
//! Multiple maps can be combined (added, subtracted, scaled) to compose
//! complex AI behaviours without steering geometry.
//!
//! `InfluenceMap` supports:
//! - `add_source(x, y, strength, radius)` — radiate `strength` falling off
//!   linearly to 0 at `radius` cells (integer Manhattan or Chebyshev
//!   decay selectable at call time). All arithmetic is integer.
//! - `add_raw(x, y, value)` — directly increment one cell.
//! - `decay(factor_num, factor_den)` — multiply all cells by `factor_num/factor_den`
//!   (integer, saturating) — used for fading influence over time.
//! - `clear()` / `reset()` — zero all cells.
//! - `get(x, y) -> Option<i32>` — cell value or `None` for OOB.
//! - `highest_neighbour(x, y)` — (dx, dy, value) of the highest adjacent cell
//!   (8-directional, for steering toward influence peaks).
//! - `lowest_neighbour(x, y)` — same but toward troughs (flee).
//! - `DetHash` folds width, height, and all cell values in row-major order.

use crate::world_hash::{DetHash, Fnv1a};

/// A 2-D grid of `i32` influence values.
#[derive(Clone, Debug)]
pub struct InfluenceMap {
    width: i32,
    height: i32,
    cells: Vec<i32>,
}

impl InfluenceMap {
    /// Create a zeroed map of the given size. Zero-area maps are valid.
    pub fn new(width: i32, height: i32) -> Self {
        let w = width.max(0);
        let h = height.max(0);
        InfluenceMap {
            width: w,
            height: h,
            cells: vec![0; (w as usize).saturating_mul(h as usize)],
        }
    }

    #[inline]
    pub fn width(&self) -> i32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> i32 {
        self.height
    }

    #[inline]
    fn idx(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        Some(y as usize * self.width as usize + x as usize)
    }

    /// Get the influence value at `(x, y)`, or `None` if out of bounds.
    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Option<i32> {
        Some(self.cells[self.idx(x, y)?])
    }

    /// Directly add `value` to cell `(x, y)`. Saturating. No-op if OOB.
    pub fn add_raw(&mut self, x: i32, y: i32, value: i32) {
        if let Some(idx) = self.idx(x, y) {
            self.cells[idx] = self.cells[idx].saturating_add(value);
        }
    }

    /// Set cell `(x, y)` to `value`. No-op if OOB.
    pub fn set(&mut self, x: i32, y: i32, value: i32) {
        if let Some(idx) = self.idx(x, y) {
            self.cells[idx] = value;
        }
    }

    /// Radiate `strength` from `(sx, sy)` decaying linearly to 0 at `radius`
    /// cells of Chebyshev distance (king-move circles). Cells beyond `radius`
    /// are unaffected. `radius == 0` affects only the source cell. Saturating.
    pub fn add_source(&mut self, sx: i32, sy: i32, strength: i32, radius: i32) {
        let r = radius.max(0);
        let x0 = (sx - r).max(0);
        let y0 = (sy - r).max(0);
        let x1 = (sx + r).min(self.width - 1);
        let y1 = (sy + r).min(self.height - 1);

        for y in y0..=y1 {
            for x in x0..=x1 {
                let dist = (x - sx).abs().max((y - sy).abs()); // Chebyshev
                if dist > r {
                    continue;
                }
                let contrib = if r == 0 {
                    strength
                } else {
                    // Linear falloff: strength * (r - dist) / r
                    strength.saturating_mul(r - dist) / r
                };
                let idx = y as usize * self.width as usize + x as usize;
                self.cells[idx] = self.cells[idx].saturating_add(contrib);
            }
        }
    }

    /// Multiply every cell by `num / den` (integer division, saturating).
    /// A `den` of 0 is treated as 1 (no change).
    pub fn decay(&mut self, num: i32, den: i32) {
        let d = if den == 0 { 1 } else { den };
        for v in &mut self.cells {
            *v = v.saturating_mul(num) / d;
        }
    }

    /// Zero all cells.
    pub fn clear(&mut self) {
        self.cells.fill(0);
    }

    /// Set every cell to `value`. The inverse of `clear()` — use to establish
    /// a non-zero baseline (e.g. a uniform "unknown territory" starting value).
    pub fn fill(&mut self, value: i32) {
        self.cells.fill(value);
    }

    /// Return the `(x, y)` coordinates of all cells whose value is ≥ `threshold`,
    /// in row-major order. Useful for AI "gather potential targets" queries.
    pub fn find_peaks(&self, threshold: i32) -> Vec<(i32, i32)> {
        self.cells
            .iter()
            .enumerate()
            .filter_map(|(i, &v)| {
                if v >= threshold {
                    let x = (i % self.width as usize) as i32;
                    let y = (i / self.width as usize) as i32;
                    Some((x, y))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Add `other * (num / den)` cell-wise into `self` (saturating). A no-op
    /// if the maps differ in size. `den == 0` is treated as `1`.
    ///
    /// Useful for composing multiple influence layers with different weights,
    /// e.g. `pref.combine(&threat, -1, 1); pref.combine(&food, 2, 3);`
    pub fn combine(&mut self, other: &InfluenceMap, num: i32, den: i32) {
        if other.width != self.width || other.height != self.height {
            return;
        }
        let d = if den == 0 { 1 } else { den };
        for (dst, &src) in self.cells.iter_mut().zip(other.cells.iter()) {
            let scaled = src.saturating_mul(num) / d;
            *dst = dst.saturating_add(scaled);
        }
    }

    /// The highest-valued immediate neighbour (8-directional) of `(x, y)`.
    /// Returns `(dx, dy, value)` where `(dx, dy)` is the step direction.
    /// Returns `None` if there are no in-bounds neighbours or the map is 1×1.
    pub fn highest_neighbour(&self, x: i32, y: i32) -> Option<(i32, i32, i32)> {
        self.best_neighbour(x, y, true)
    }

    /// The lowest-valued immediate neighbour (8-directional) of `(x, y)`.
    /// Returns `(dx, dy, value)`.
    pub fn lowest_neighbour(&self, x: i32, y: i32) -> Option<(i32, i32, i32)> {
        self.best_neighbour(x, y, false)
    }

    fn best_neighbour(&self, x: i32, y: i32, highest: bool) -> Option<(i32, i32, i32)> {
        let mut best: Option<(i32, i32, i32)> = None;
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                if let Some(v) = self.get(x + dx, y + dy) {
                    let take = match &best {
                        None => true,
                        Some((_, _, bv)) => {
                            if highest {
                                v > *bv
                            } else {
                                v < *bv
                            }
                        }
                    };
                    if take {
                        best = Some((dx, dy, v));
                    }
                }
            }
        }
        best
    }

    /// Iterate `(x, y, value)` for all cells in row-major order.
    pub fn iter(&self) -> impl Iterator<Item = (i32, i32, i32)> + '_ {
        let w = self.width;
        self.cells
            .iter()
            .enumerate()
            .map(move |(i, &v)| ((i % w as usize) as i32, (i / w as usize) as i32, v))
    }
}

impl DetHash for InfluenceMap {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_i32(self.width);
        hasher.write_i32(self.height);
        for v in &self.cells {
            hasher.write_i32(*v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[test]
    fn test_new_zeroed() {
        let m = InfluenceMap::new(5, 5);
        for (_, _, v) in m.iter() {
            assert_eq!(v, 0);
        }
    }

    #[test]
    fn test_add_raw_and_get() {
        let mut m = InfluenceMap::new(5, 5);
        m.add_raw(2, 3, 10);
        assert_eq!(m.get(2, 3), Some(10));
        assert_eq!(m.get(0, 0), Some(0));
    }

    #[test]
    fn test_get_oob_returns_none() {
        let m = InfluenceMap::new(4, 4);
        assert_eq!(m.get(-1, 0), None);
        assert_eq!(m.get(4, 0), None);
    }

    #[test]
    fn test_add_raw_saturating() {
        let mut m = InfluenceMap::new(3, 3);
        m.set(1, 1, i32::MAX);
        m.add_raw(1, 1, 1);
        assert_eq!(m.get(1, 1), Some(i32::MAX));
    }

    #[test]
    fn test_add_source_center() {
        let mut m = InfluenceMap::new(5, 5);
        m.add_source(2, 2, 100, 0);
        assert_eq!(m.get(2, 2), Some(100));
        assert_eq!(m.get(2, 3), Some(0)); // outside radius 0
    }

    #[test]
    fn test_add_source_radius_1() {
        let mut m = InfluenceMap::new(5, 5);
        m.add_source(2, 2, 100, 1);
        assert_eq!(m.get(2, 2), Some(100)); // dist=0, full strength
        assert_eq!(m.get(2, 3), Some(0)); // dist=1, 100*(1-1)/1=0
        assert_eq!(m.get(1, 1), Some(0)); // dist=1 → 0
    }

    #[test]
    fn test_add_source_radius_2() {
        let mut m = InfluenceMap::new(7, 7);
        m.add_source(3, 3, 100, 2);
        assert_eq!(m.get(3, 3), Some(100)); // dist=0
                                            // dist=1: 100*(2-1)/2 = 50
        assert_eq!(m.get(3, 4), Some(50));
        // dist=2: 100*(2-2)/2 = 0
        assert_eq!(m.get(3, 5), Some(0));
    }

    #[test]
    fn test_add_source_clips_to_boundary() {
        let mut m = InfluenceMap::new(4, 4);
        m.add_source(0, 0, 50, 5); // source at corner with large radius
        assert!(m.get(0, 0).unwrap() > 0);
        assert_eq!(m.get(-1, 0), None);
    }

    #[test]
    fn test_decay() {
        let mut m = InfluenceMap::new(3, 3);
        m.set(1, 1, 100);
        m.decay(1, 2); // halve
        assert_eq!(m.get(1, 1), Some(50));
    }

    #[test]
    fn test_decay_zero_den_is_noop() {
        let mut m = InfluenceMap::new(3, 3);
        m.set(1, 1, 100);
        m.decay(1, 0); // den=0 → treated as 1
        assert_eq!(m.get(1, 1), Some(100));
    }

    #[test]
    fn test_clear() {
        let mut m = InfluenceMap::new(3, 3);
        m.set(1, 1, 99);
        m.clear();
        assert_eq!(m.get(1, 1), Some(0));
    }

    #[test]
    fn test_highest_neighbour() {
        let mut m = InfluenceMap::new(3, 3);
        m.set(2, 2, 100); // southeast of (1,1)
        let (dx, dy, v) = m.highest_neighbour(1, 1).unwrap();
        assert_eq!((dx, dy), (1, 1));
        assert_eq!(v, 100);
    }

    #[test]
    fn test_lowest_neighbour() {
        let mut m = InfluenceMap::new(3, 3);
        m.set(0, 0, -50); // northwest of (1,1)
        let (dx, dy, v) = m.lowest_neighbour(1, 1).unwrap();
        assert_eq!((dx, dy), (-1, -1));
        assert_eq!(v, -50);
    }

    #[test]
    fn test_combine_half_weight() {
        let mut base = InfluenceMap::new(3, 3);
        let mut other = InfluenceMap::new(3, 3);
        other.set(1, 1, 100);
        base.combine(&other, 1, 2); // add 50
        assert_eq!(base.get(1, 1), Some(50));
        assert_eq!(base.get(0, 0), Some(0));
    }

    #[test]
    fn test_combine_full_weight_is_additive() {
        let mut base = InfluenceMap::new(3, 3);
        base.set(1, 1, 40);
        let mut other = InfluenceMap::new(3, 3);
        other.set(1, 1, 60);
        base.combine(&other, 1, 1);
        assert_eq!(base.get(1, 1), Some(100));
    }

    #[test]
    fn test_combine_negative_weight_subtracts() {
        let mut base = InfluenceMap::new(3, 3);
        base.set(1, 1, 100);
        let mut threat = InfluenceMap::new(3, 3);
        threat.set(1, 1, 30);
        base.combine(&threat, -1, 1);
        assert_eq!(base.get(1, 1), Some(70));
    }

    #[test]
    fn test_combine_mismatched_size_is_noop() {
        let mut base = InfluenceMap::new(3, 3);
        base.set(0, 0, 50);
        let other = InfluenceMap::new(4, 4);
        base.combine(&other, 1, 1);
        assert_eq!(base.get(0, 0), Some(50)); // unchanged
    }

    #[test]
    fn test_combine_zero_den_treated_as_one() {
        let mut base = InfluenceMap::new(2, 2);
        let mut other = InfluenceMap::new(2, 2);
        other.set(0, 0, 10);
        base.combine(&other, 3, 0); // den=0 → den=1, scaled=30
        assert_eq!(base.get(0, 0), Some(30));
    }

    #[test]
    fn test_det_hash_same_map_same_hash() {
        let mut a = InfluenceMap::new(4, 4);
        let mut b = InfluenceMap::new(4, 4);
        a.set(1, 1, 42);
        b.set(1, 1, 42);
        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_det_hash_changed_differs() {
        let mut a = InfluenceMap::new(4, 4);
        let mut b = InfluenceMap::new(4, 4);
        a.set(1, 1, 10);
        b.set(1, 1, 20);
        assert_ne!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_fill_sets_all_cells() {
        let mut m = InfluenceMap::new(3, 3);
        m.fill(42);
        for (_, _, v) in m.iter() {
            assert_eq!(v, 42);
        }
    }

    #[test]
    fn test_fill_then_clear_zeros_all() {
        let mut m = InfluenceMap::new(3, 3);
        m.fill(100);
        m.clear();
        for (_, _, v) in m.iter() {
            assert_eq!(v, 0);
        }
    }

    #[test]
    fn test_find_peaks_empty_map() {
        let m = InfluenceMap::new(3, 3);
        assert!(m.find_peaks(1).is_empty());
    }

    #[test]
    fn test_find_peaks_above_threshold() {
        let mut m = InfluenceMap::new(3, 3);
        m.set(0, 0, 10);
        m.set(1, 1, 50);
        m.set(2, 2, 30);
        let peaks = m.find_peaks(30);
        assert!(peaks.contains(&(1, 1)));
        assert!(peaks.contains(&(2, 2)));
        assert!(!peaks.contains(&(0, 0)));
    }

    #[test]
    fn test_find_peaks_threshold_inclusive() {
        let mut m = InfluenceMap::new(2, 2);
        m.set(0, 0, 5);
        let peaks = m.find_peaks(5);
        assert_eq!(peaks, vec![(0, 0)]);
    }
}
