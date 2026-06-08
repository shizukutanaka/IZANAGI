//! Integer camera and viewport — world-to-screen coordinate mapping.
//!
//! A roguelike terminal renders a rectangular window into a potentially larger
//! world. The `Camera` tracks a focus point (world coordinates) and the
//! viewport's size; `world_to_screen` / `screen_to_world` perform the
//! two-way mapping. All arithmetic is integer — no float — so the mapping is
//! bit-identical across targets and safe to fold into the world hash.
//!
//! The camera never scrolls beyond the world boundary: clamping ensures the
//! viewport is always fully within `[0, world_width) × [0, world_height)`.
//! Callers that want an unclamped camera (e.g. an infinite world) can simply
//! omit the `world_*` bounds and do their own overflow handling.

use crate::world_hash::{DetHash, Fnv1a};

/// An axis-aligned integer camera.
///
/// `top_left` is the world-space coordinate of screen cell (0, 0).
/// It is always clamped so the full viewport fits within the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Camera {
    /// World-space X of the screen's left column.
    pub top_left_x: i32,
    /// World-space Y of the screen's top row.
    pub top_left_y: i32,
    /// Width of the viewport in screen cells.
    pub screen_w: u32,
    /// Height of the viewport in screen cells.
    pub screen_h: u32,
}

impl Camera {
    /// Create a camera centred on `(cx, cy)` in world space, clamped so that
    /// the full `screen_w × screen_h` viewport stays within the world.
    ///
    /// `world_w` / `world_h` are the world dimensions. If the world is smaller
    /// than the viewport the camera is clamped to (0, 0).
    pub fn new(cx: i32, cy: i32, screen_w: u32, screen_h: u32, world_w: u32, world_h: u32) -> Self {
        let tl_x = Self::clamp_origin(cx, screen_w, world_w);
        let tl_y = Self::clamp_origin(cy, screen_h, world_h);
        Camera {
            top_left_x: tl_x,
            top_left_y: tl_y,
            screen_w,
            screen_h,
        }
    }

    /// Move the focus to `(cx, cy)`, re-clamping within the world.
    pub fn recenter(&mut self, cx: i32, cy: i32, world_w: u32, world_h: u32) {
        self.top_left_x = Self::clamp_origin(cx, self.screen_w, world_w);
        self.top_left_y = Self::clamp_origin(cy, self.screen_h, world_h);
    }

    /// Convert a world-space point `(wx, wy)` to a screen-space cell
    /// `(sx, sy)`. Returns `None` if the point falls outside the viewport.
    #[inline]
    pub fn world_to_screen(&self, wx: i32, wy: i32) -> Option<(u32, u32)> {
        let sx = wx - self.top_left_x;
        let sy = wy - self.top_left_y;
        if sx < 0 || sy < 0 {
            return None;
        }
        let sx = sx as u32;
        let sy = sy as u32;
        if sx < self.screen_w && sy < self.screen_h {
            Some((sx, sy))
        } else {
            None
        }
    }

    /// Convert a screen-space cell `(sx, sy)` to a world-space coordinate.
    /// Out-of-bounds screen coordinates are clamped to the viewport edge.
    #[inline]
    pub fn screen_to_world(&self, sx: u32, sy: u32) -> (i32, i32) {
        let sx = sx.min(self.screen_w.saturating_sub(1));
        let sy = sy.min(self.screen_h.saturating_sub(1));
        (self.top_left_x + sx as i32, self.top_left_y + sy as i32)
    }

    /// Whether a world-space point is visible in the current viewport.
    #[inline]
    pub fn is_visible(&self, wx: i32, wy: i32) -> bool {
        self.world_to_screen(wx, wy).is_some()
    }

    /// The world-space rect covered by this viewport:
    /// `(left, top, right_exclusive, bottom_exclusive)`.
    #[inline]
    pub fn world_rect(&self) -> (i32, i32, i32, i32) {
        (
            self.top_left_x,
            self.top_left_y,
            self.top_left_x + self.screen_w as i32,
            self.top_left_y + self.screen_h as i32,
        )
    }

    // Compute the top-left origin for one axis: centre on `focus`, clamp so
    // the `view` cells fit in `world`.
    fn clamp_origin(focus: i32, view: u32, world: u32) -> i32 {
        if world == 0 || view == 0 {
            return 0;
        }
        let view = view as i32;
        let world = world as i32;
        // Ideal top-left: centre the focus.
        let ideal = focus - view / 2;
        // Clamp: [0, world - view] (or 0 if world < view).
        let max = (world - view).max(0);
        ideal.clamp(0, max)
    }
}

impl DetHash for Camera {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_i32(self.top_left_x);
        hasher.write_i32(self.top_left_y);
        hasher.write_u32(self.screen_w);
        hasher.write_u32(self.screen_h);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    fn cam(cx: i32, cy: i32) -> Camera {
        Camera::new(cx, cy, 10, 8, 40, 30)
    }

    #[test]
    fn test_centre_maps_to_middle_of_screen() {
        // Focus (20, 15) on a 40×30 world, 10×8 viewport.
        // top_left = (20 - 5, 15 - 4) = (15, 11).
        let c = cam(20, 15);
        assert_eq!(c.top_left_x, 15);
        assert_eq!(c.top_left_y, 11);
        // World (20, 15) → screen (5, 4).
        assert_eq!(c.world_to_screen(20, 15), Some((5, 4)));
    }

    #[test]
    fn test_world_to_screen_top_left_corner() {
        let c = cam(20, 15);
        assert_eq!(c.world_to_screen(c.top_left_x, c.top_left_y), Some((0, 0)));
    }

    #[test]
    fn test_world_to_screen_bottom_right_corner() {
        let c = cam(20, 15);
        let (_l, _t, r, b) = c.world_rect();
        // The exclusive boundary is out of bounds.
        assert_eq!(c.world_to_screen(r - 1, b - 1), Some((9, 7)));
        assert_eq!(c.world_to_screen(r, b), None);
    }

    #[test]
    fn test_world_to_screen_out_of_viewport_returns_none() {
        let c = cam(20, 15);
        assert_eq!(c.world_to_screen(0, 0), None); // far off-screen
        assert_eq!(c.world_to_screen(39, 29), None); // far off-screen right
    }

    #[test]
    fn test_screen_to_world_roundtrip() {
        let c = cam(20, 15);
        for sx in 0..10u32 {
            for sy in 0..8u32 {
                let (wx, wy) = c.screen_to_world(sx, sy);
                assert_eq!(c.world_to_screen(wx, wy), Some((sx, sy)));
            }
        }
    }

    #[test]
    fn test_clamping_near_left_edge() {
        // Focus very close to the left edge: top_left must not go negative.
        let c = Camera::new(1, 15, 10, 8, 40, 30);
        assert_eq!(c.top_left_x, 0);
    }

    #[test]
    fn test_clamping_near_right_edge() {
        // Focus near the right edge: top_left must not push viewport off.
        let c = Camera::new(38, 15, 10, 8, 40, 30);
        assert_eq!(c.top_left_x, 30); // 40 - 10
    }

    #[test]
    fn test_clamping_near_top_edge() {
        let c = Camera::new(20, 1, 10, 8, 40, 30);
        assert_eq!(c.top_left_y, 0);
    }

    #[test]
    fn test_clamping_near_bottom_edge() {
        let c = Camera::new(20, 28, 10, 8, 40, 30);
        assert_eq!(c.top_left_y, 22); // 30 - 8
    }

    #[test]
    fn test_viewport_larger_than_world_clamps_to_origin() {
        // A 100×100 viewport in a 10×10 world.
        let c = Camera::new(5, 5, 100, 100, 10, 10);
        assert_eq!(c.top_left_x, 0);
        assert_eq!(c.top_left_y, 0);
    }

    #[test]
    fn test_is_visible() {
        let c = cam(20, 15);
        assert!(c.is_visible(20, 15));
        assert!(!c.is_visible(0, 0));
    }

    #[test]
    fn test_recenter_updates_top_left() {
        let mut c = cam(20, 15);
        // focus=(5,5), screen=(10,8), world=(40,30)
        // tl_x = clamp(5 - 5, 0, 30) = 0
        // tl_y = clamp(5 - 4, 0, 22) = 1
        c.recenter(5, 5, 40, 30);
        assert_eq!(c.top_left_x, 0);
        assert_eq!(c.top_left_y, 1);
    }

    #[test]
    fn test_world_rect_dimensions() {
        let c = cam(20, 15);
        let (l, t, r, b) = c.world_rect();
        assert_eq!(r - l, 10);
        assert_eq!(b - t, 8);
    }

    #[test]
    fn test_det_hash_changes_on_recenter() {
        let c1 = cam(20, 15);
        let c2 = cam(5, 5);
        assert_ne!(hash_state(&c1), hash_state(&c2));
    }

    #[test]
    fn test_det_hash_same_position_same_hash() {
        let c1 = cam(20, 15);
        let c2 = cam(20, 15);
        assert_eq!(hash_state(&c1), hash_state(&c2));
    }
}
