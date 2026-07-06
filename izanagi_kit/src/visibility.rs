//! Tri-state visibility / exploration memory ("fog of war") for roguelike maps.
//!
//! [`fov::compute_fov`](crate::fov::compute_fov) tells you which cells are
//! *currently* visible, but a roguelike screen needs three states, not two:
//!
//! - **Visible** — in the player's field of view right now (drawn brightly).
//! - **Remembered** — seen on a previous turn but not visible now (drawn dim).
//! - **Unseen** — never observed (drawn blank / black).
//!
//! [`VisibilityMap`] is the missing memory layer between FOV and the renderer.
//! It composes directly with [`fov::compute_fov`](crate::fov::compute_fov):
//!
//! ```
//! use izanagi_kit::visibility::{VisibilityMap, Visibility};
//! use izanagi_kit::fov::compute_fov;
//!
//! let mut vis = VisibilityMap::new(16, 16);
//! let walls = |x: i32, y: i32| !(0..16).contains(&x) || !(0..16).contains(&y);
//!
//! // Each turn: demote last frame's visible cells to "remembered", then
//! // recompute FOV, marking the freshly-visible cells.
//! vis.begin_frame();
//! compute_fov((8, 8), 5, walls, |x, y| vis.mark_visible(x, y));
//!
//! assert_eq!(vis.get(8, 8), Visibility::Visible); // the player's own cell
//! assert!(vis.is_explored(8, 8));                 // visible ⇒ explored
//! ```
//!
//! Everything is integer/enum-only and order-independent: [`VisibilityMap`]
//! implements [`DetHash`](crate::world_hash::DetHash) by folding `(width,
//! height)` then every cell in row-major order, so the fog state folds into the
//! replay checksum and is bit-identical across platforms.

use crate::world_hash::{DetHash, Fnv1a};

/// The observation state of a single map cell.
///
/// The discriminants are ordered `Unseen < Remembered < Visible` so that
/// "more observed" is simply "greater": exploration progress can be expressed
/// with [`Ord`]/`max`, and [`is_explored`](VisibilityMap::is_explored) is just
/// `state >= Remembered`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Visibility {
    /// Never observed.
    Unseen = 0,
    /// Seen before, but not in the current field of view.
    Remembered = 1,
    /// In the current field of view.
    Visible = 2,
}

impl Visibility {
    /// The discriminant as a `u8` (`Unseen = 0`, `Remembered = 1`, `Visible = 2`).
    #[inline]
    pub fn rank(self) -> u8 {
        self as u8
    }
}

impl DetHash for Visibility {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u8(*self as u8);
    }
}

/// A row-major grid of [`Visibility`] cells: the player's fog-of-war / explored
/// memory of a map.
///
/// Out-of-bounds reads return [`Visibility::Unseen`] and out-of-bounds writes
/// are silently ignored, so callers can probe neighbours past the edge without
/// bounds-checking (mirroring the panic-free convention of the rest of the kit).
#[derive(Clone, Debug)]
pub struct VisibilityMap {
    width: u32,
    height: u32,
    cells: Vec<Visibility>,
}

impl VisibilityMap {
    /// Create a `width × height` map with every cell [`Visibility::Unseen`].
    pub fn new(width: u32, height: u32) -> Self {
        let n = (width as usize).saturating_mul(height as usize);
        VisibilityMap {
            width,
            height,
            cells: vec![Visibility::Unseen; n],
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

    /// Total number of cells (`width × height`).
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

    /// The visibility of `(x, y)`. Out-of-bounds cells read as
    /// [`Visibility::Unseen`].
    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Visibility {
        match self.idx(x, y) {
            Some(i) => self.cells[i],
            None => Visibility::Unseen,
        }
    }

    /// `true` if `(x, y)` is in the current field of view.
    #[inline]
    pub fn is_visible(&self, x: i32, y: i32) -> bool {
        self.get(x, y) == Visibility::Visible
    }

    /// `true` if `(x, y)` was seen previously but is not currently visible.
    #[inline]
    pub fn is_remembered(&self, x: i32, y: i32) -> bool {
        self.get(x, y) == Visibility::Remembered
    }

    /// `true` if `(x, y)` has ever been observed (visible **or** remembered).
    #[inline]
    pub fn is_explored(&self, x: i32, y: i32) -> bool {
        self.get(x, y) >= Visibility::Remembered
    }

    /// `true` if `(x, y)` has never been observed.
    #[inline]
    pub fn is_unseen(&self, x: i32, y: i32) -> bool {
        self.get(x, y) == Visibility::Unseen
    }

    /// Mark `(x, y)` as currently [`Visible`](Visibility::Visible). No-op if out
    /// of bounds. Idempotent. This is the callback to hand to
    /// [`fov::compute_fov`](crate::fov::compute_fov).
    #[inline]
    pub fn mark_visible(&mut self, x: i32, y: i32) {
        if let Some(i) = self.idx(x, y) {
            self.cells[i] = Visibility::Visible;
        }
    }

    /// Start a new observation frame: demote every [`Visible`](Visibility::Visible)
    /// cell to [`Remembered`](Visibility::Remembered), leaving `Remembered` and
    /// `Unseen` cells unchanged. Call this **before** recomputing FOV each turn,
    /// then mark the freshly-visible cells with [`mark_visible`](Self::mark_visible).
    ///
    /// Exploration is monotone: this never turns an explored cell back to
    /// `Unseen`.
    pub fn begin_frame(&mut self) {
        for cell in &mut self.cells {
            if *cell == Visibility::Visible {
                *cell = Visibility::Remembered;
            }
        }
    }

    /// Explicitly set the state of `(x, y)`. No-op if out of bounds. Primarily
    /// for restoring saved fog state exactly.
    #[inline]
    pub fn set(&mut self, x: i32, y: i32, state: Visibility) {
        if let Some(i) = self.idx(x, y) {
            self.cells[i] = state;
        }
    }

    /// Reveal the entire map as [`Visible`](Visibility::Visible) (debug / "see
    /// all" cheat).
    pub fn reveal_all(&mut self) {
        self.cells.fill(Visibility::Visible);
    }

    /// "Magic mapping": promote every [`Unseen`](Visibility::Unseen) cell to
    /// [`Remembered`](Visibility::Remembered), revealing the layout as memory
    /// without granting current sight. Already-visible cells are left visible.
    pub fn remember_all(&mut self) {
        for cell in &mut self.cells {
            if *cell == Visibility::Unseen {
                *cell = Visibility::Remembered;
            }
        }
    }

    /// Reset every cell to [`Unseen`](Visibility::Unseen) (e.g. on entering a new
    /// floor that should start dark).
    pub fn reset(&mut self) {
        self.cells.fill(Visibility::Unseen);
    }

    /// Count of cells in a given visibility state.
    pub fn count(&self, state: Visibility) -> usize {
        self.cells.iter().filter(|&&c| c == state).count()
    }

    /// Count of cells currently in the field of view.
    #[inline]
    pub fn visible_count(&self) -> usize {
        self.count(Visibility::Visible)
    }

    /// Count of cells ever observed (visible or remembered).
    #[inline]
    pub fn explored_count(&self) -> usize {
        self.cells
            .iter()
            .filter(|&&c| c >= Visibility::Remembered)
            .count()
    }

    /// Fraction explored as an integer percentage in `[0, 100]`: `explored ·
    /// 100 / total`. Returns `0` for an empty map. Float-free — useful for a
    /// "map 73% explored" HUD readout.
    pub fn explored_percent(&self) -> u32 {
        if self.cells.is_empty() {
            return 0;
        }
        (self.explored_count() as u64 * 100 / self.cells.len() as u64) as u32
    }

    /// Iterate `(x, y, Visibility)` in row-major order.
    pub fn iter(&self) -> impl Iterator<Item = (i32, i32, Visibility)> + '_ {
        let w = self.width;
        self.cells
            .iter()
            .enumerate()
            .map(move |(i, &c)| ((i % w as usize) as i32, (i / w as usize) as i32, c))
    }
}

impl DetHash for VisibilityMap {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.width);
        hasher.write_u32(self.height);
        for c in &self.cells {
            c.det_hash(hasher);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fov::compute_fov;
    use crate::world_hash::hash_state;

    #[test]
    fn test_new_is_all_unseen() {
        let v = VisibilityMap::new(4, 3);
        assert_eq!(v.len(), 12);
        assert_eq!(v.count(Visibility::Unseen), 12);
        assert_eq!(v.visible_count(), 0);
        assert_eq!(v.explored_count(), 0);
        for (_, _, s) in v.iter() {
            assert_eq!(s, Visibility::Unseen);
        }
    }

    #[test]
    fn test_mark_visible_sets_visible_and_explored() {
        let mut v = VisibilityMap::new(5, 5);
        v.mark_visible(2, 3);
        assert!(v.is_visible(2, 3));
        assert!(v.is_explored(2, 3));
        assert_eq!(v.get(2, 3), Visibility::Visible);
        assert_eq!(v.visible_count(), 1);
    }

    #[test]
    fn test_mark_visible_is_idempotent() {
        let mut v = VisibilityMap::new(5, 5);
        v.mark_visible(1, 1);
        let h1 = hash_state(&v);
        v.mark_visible(1, 1);
        assert_eq!(hash_state(&v), h1, "marking twice must equal marking once");
    }

    #[test]
    fn test_oob_get_is_unseen_and_oob_writes_are_noops() {
        let mut v = VisibilityMap::new(3, 3);
        assert_eq!(v.get(-1, 0), Visibility::Unseen);
        assert_eq!(v.get(3, 0), Visibility::Unseen);
        assert_eq!(v.get(0, 3), Visibility::Unseen);
        let before = hash_state(&v);
        v.mark_visible(-1, 0);
        v.mark_visible(3, 3);
        v.set(99, 99, Visibility::Visible);
        assert_eq!(
            hash_state(&v),
            before,
            "OOB writes must not change anything"
        );
    }

    #[test]
    fn test_begin_frame_demotes_visible_to_remembered() {
        let mut v = VisibilityMap::new(4, 4);
        v.mark_visible(1, 1);
        v.mark_visible(2, 2);
        v.begin_frame();
        assert_eq!(v.get(1, 1), Visibility::Remembered);
        assert_eq!(v.get(2, 2), Visibility::Remembered);
        assert_eq!(v.visible_count(), 0);
        assert_eq!(v.explored_count(), 2, "still explored after demotion");
    }

    #[test]
    fn test_begin_frame_leaves_remembered_and_unseen_untouched() {
        let mut v = VisibilityMap::new(3, 3);
        v.mark_visible(0, 0);
        v.begin_frame(); // (0,0) -> Remembered
        let before = hash_state(&v);
        v.begin_frame(); // no Visible cells; must be a no-op
        assert_eq!(
            hash_state(&v),
            before,
            "begin_frame on no-visible is a no-op"
        );
        assert_eq!(v.get(0, 0), Visibility::Remembered);
        assert_eq!(v.get(2, 2), Visibility::Unseen);
    }

    #[test]
    fn test_explore_then_revisit_cycle() {
        let mut v = VisibilityMap::new(5, 1);
        // Turn 1: see cells 0,1,2.
        v.begin_frame();
        for x in 0..3 {
            v.mark_visible(x, 0);
        }
        assert_eq!(v.visible_count(), 3);

        // Turn 2: move so only cells 2,3,4 are visible.
        v.begin_frame();
        for x in 2..5 {
            v.mark_visible(x, 0);
        }
        assert_eq!(v.get(0, 0), Visibility::Remembered); // left behind
        assert_eq!(v.get(1, 0), Visibility::Remembered);
        assert_eq!(v.get(2, 0), Visibility::Visible);
        assert_eq!(v.get(4, 0), Visibility::Visible);
        assert_eq!(v.explored_count(), 5, "all five explored");
        assert_eq!(v.visible_count(), 3);
    }

    #[test]
    fn test_reveal_all_and_reset() {
        let mut v = VisibilityMap::new(3, 3);
        v.reveal_all();
        assert_eq!(v.visible_count(), 9);
        assert_eq!(v.explored_percent(), 100);
        v.reset();
        assert_eq!(v.count(Visibility::Unseen), 9);
        assert_eq!(v.explored_percent(), 0);
    }

    #[test]
    fn test_remember_all_is_magic_mapping() {
        let mut v = VisibilityMap::new(3, 3);
        v.mark_visible(1, 1); // currently visible
        v.remember_all();
        // Unseen cells become Remembered; the visible cell stays Visible.
        assert_eq!(v.get(0, 0), Visibility::Remembered);
        assert_eq!(v.get(1, 1), Visibility::Visible);
        assert_eq!(v.explored_count(), 9, "whole map now explored");
    }

    #[test]
    fn test_explored_percent_is_integer_ratio() {
        let mut v = VisibilityMap::new(4, 1); // 4 cells
        v.mark_visible(0, 0);
        v.mark_visible(1, 0);
        // 2 of 4 explored → 50%
        assert_eq!(v.explored_percent(), 50);
        assert_eq!(VisibilityMap::new(0, 0).explored_percent(), 0);
    }

    #[test]
    fn test_set_restores_exact_state() {
        let mut v = VisibilityMap::new(2, 2);
        v.set(0, 0, Visibility::Remembered);
        v.set(1, 1, Visibility::Visible);
        assert_eq!(v.get(0, 0), Visibility::Remembered);
        assert_eq!(v.get(1, 1), Visibility::Visible);
        assert_eq!(v.get(1, 0), Visibility::Unseen);
    }

    #[test]
    fn test_det_hash_canonical_and_sensitive() {
        let a = VisibilityMap::new(4, 4);
        let b = VisibilityMap::new(4, 4);
        assert_eq!(hash_state(&a), hash_state(&b), "same content same hash");
        let mut c = a.clone();
        c.mark_visible(2, 2);
        assert_ne!(
            hash_state(&a),
            hash_state(&c),
            "a change must change the hash"
        );
    }

    #[test]
    fn test_composes_with_compute_fov() {
        // An open room with a wall pillar; FOV results feed straight into the map.
        let mut v = VisibilityMap::new(11, 11);
        let opaque = |x: i32, y: i32| (x, y) == (7, 5);
        v.begin_frame();
        compute_fov((5, 5), 4, opaque, |x, y| v.mark_visible(x, y));
        assert!(v.is_visible(5, 5), "origin is always visible");
        // The cell directly behind the pillar (from the origin) is shadowed.
        assert!(!v.is_visible(8, 5), "cell behind the pillar must be hidden");
        assert!(v.visible_count() > 1, "FOV revealed a region");
    }
}
