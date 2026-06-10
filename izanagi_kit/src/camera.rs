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

    /// Scroll the viewport by `(dx, dy)` world cells, clamped so it stays
    /// within the world. Useful for arrow-key map scrolling.
    pub fn pan(&mut self, dx: i32, dy: i32, world_w: u32, world_h: u32) {
        let max_x = (world_w as i32 - self.screen_w as i32).max(0);
        let max_y = (world_h as i32 - self.screen_h as i32).max(0);
        self.top_left_x = self.top_left_x.saturating_add(dx).clamp(0, max_x);
        self.top_left_y = self.top_left_y.saturating_add(dy).clamp(0, max_y);
    }

    /// World-space coordinate at the centre of the viewport.
    ///
    /// Uses integer division so the result is exact for even viewport sizes.
    #[inline]
    pub fn center(&self) -> (i32, i32) {
        (
            self.top_left_x + self.screen_w as i32 / 2,
            self.top_left_y + self.screen_h as i32 / 2,
        )
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

    /// Convert a world-space point `(wx, wy)` to a screen-space offset without
    /// bounds checking. May return negative values or coordinates outside the
    /// viewport for points that are off-screen. Use `world_to_screen` when you
    /// only need visible points.
    #[inline]
    pub fn world_to_screen_unclamped(&self, wx: i32, wy: i32) -> (i32, i32) {
        (wx - self.top_left_x, wy - self.top_left_y)
    }

    /// Whether a world-space point is visible in the current viewport.
    #[inline]
    pub fn is_visible(&self, wx: i32, wy: i32) -> bool {
        self.world_to_screen(wx, wy).is_some()
    }

    /// Resize the viewport to `screen_w × screen_h` and re-clamp so the full
    /// viewport stays within the world. The current world-space centre is
    /// preserved so the view "grows out" symmetrically on a terminal resize.
    pub fn set_screen_size(&mut self, screen_w: u32, screen_h: u32, world_w: u32, world_h: u32) {
        let (cx, cy) = self.center();
        self.screen_w = screen_w;
        self.screen_h = screen_h;
        self.top_left_x = Self::clamp_origin(cx, screen_w, world_w);
        self.top_left_y = Self::clamp_origin(cy, screen_h, world_h);
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

    /// Whether the world-space axis-aligned rectangle `[l, r) × [t, b)` (all
    /// exclusive-right/bottom) overlaps the camera's current viewport.
    /// An empty rect (`l >= r` or `t >= b`) never overlaps.
    #[inline]
    pub fn contains_rect(&self, l: i32, t: i32, r: i32, b: i32) -> bool {
        if l >= r || t >= b {
            return false;
        }
        let (vl, vt, vr, vb) = self.world_rect();
        l < vr && r > vl && t < vb && b > vt
    }

    /// Chebyshev distance from the viewport centre to world point `(wx, wy)`.
    /// Chebyshev distance is `max(|cx - wx|, |cy - wy|)` — the natural
    /// "number of king moves" metric for 8-directional tile grids. Returns 0
    /// when the point equals the centre.
    #[inline]
    pub fn chebyshev_to_center(&self, wx: i32, wy: i32) -> u32 {
        let (cx, cy) = self.center();
        let dx = (cx - wx).unsigned_abs();
        let dy = (cy - wy).unsigned_abs();
        dx.max(dy)
    }

    /// The centre of the screen viewport in screen-space, i.e.
    /// `(screen_w / 2, screen_h / 2)`. Useful for centering HUD elements or
    /// computing the screen midpoint for radial spawns.
    #[inline]
    pub fn screen_center(&self) -> (u32, u32) {
        (self.screen_w / 2, self.screen_h / 2)
    }

    /// Convert world coordinates to screen coordinates, clamping to the screen
    /// bounds. Off-screen points land on the nearest edge pixel rather than
    /// returning `None`. Useful for "draw an arrow toward an off-screen target."
    #[inline]
    pub fn clamp_world_to_screen(&self, wx: i32, wy: i32) -> (u32, u32) {
        let max_x = self.screen_w.saturating_sub(1) as i32;
        let max_y = self.screen_h.saturating_sub(1) as i32;
        let sx = (wx - self.top_left_x).clamp(0, max_x) as u32;
        let sy = (wy - self.top_left_y).clamp(0, max_y) as u32;
        (sx, sy)
    }

    /// Chebyshev distance between two screen-space cells `(sx1, sy1)` and
    /// `(sx2, sy2)`. Returns `max(|sx1−sx2|, |sy1−sy2|)`.
    /// Both inputs are in viewport coordinates (0-based from top-left).
    #[inline]
    pub fn screen_distance(sx1: u32, sy1: u32, sx2: u32, sy2: u32) -> u32 {
        let dx = sx1.abs_diff(sx2);
        let dy = sy1.abs_diff(sy2);
        dx.max(dy)
    }

    /// Lazy-follow `(wx, wy)`: pan the minimum amount to keep the point within
    /// `margin` cells of every viewport edge. If the point is already within the
    /// inner region no pan occurs. Useful for keeping the player visible without
    /// constantly re-centring the view on every step.
    pub fn follow(&mut self, wx: i32, wy: i32, margin: u32, world_w: u32, world_h: u32) {
        let m = margin as i32;
        let inner_l = self.top_left_x + m;
        let inner_r = self.top_left_x + self.screen_w as i32 - m - 1;
        let inner_t = self.top_left_y + m;
        let inner_b = self.top_left_y + self.screen_h as i32 - m - 1;
        let dx = if wx < inner_l {
            wx - inner_l
        } else if inner_r >= inner_l && wx > inner_r {
            wx - inner_r
        } else {
            0
        };
        let dy = if wy < inner_t {
            wy - inner_t
        } else if inner_b >= inner_t && wy > inner_b {
            wy - inner_b
        } else {
            0
        };
        if dx != 0 || dy != 0 {
            self.pan(dx, dy, world_w, world_h);
        }
    }

    /// Total number of cells in the viewport: `screen_w × screen_h`. Useful
    /// for per-frame draw-call budgets, renderer pre-allocation, and broad-phase
    /// "is the map larger than the screen?" checks without two separate reads.
    #[inline]
    pub fn viewport_area(&self) -> u32 {
        self.screen_w.saturating_mul(self.screen_h)
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
    fn test_pan_shifts_top_left() {
        let mut c = cam(20, 15); // top_left = (15, 11)
        c.pan(3, 2, 40, 30);
        assert_eq!(c.top_left_x, 18);
        assert_eq!(c.top_left_y, 13);
    }

    #[test]
    fn test_pan_clamps_at_world_boundaries() {
        let mut c = Camera::new(0, 0, 10, 8, 40, 30); // top_left = (0,0)
        c.pan(-10, -10, 40, 30); // can't go negative
        assert_eq!(c.top_left_x, 0);
        assert_eq!(c.top_left_y, 0);
        c.pan(100, 100, 40, 30); // can't push viewport off the right/bottom
        assert_eq!(c.top_left_x, 30); // 40 - 10
        assert_eq!(c.top_left_y, 22); // 30 - 8
    }

    #[test]
    fn test_center_returns_world_midpoint() {
        let c = cam(20, 15); // top_left = (15, 11), screen = 10×8
        let (cx, cy) = c.center();
        // 15 + 10/2 = 20, 11 + 8/2 = 15
        assert_eq!(cx, 20);
        assert_eq!(cy, 15);
    }

    #[test]
    fn test_det_hash_same_position_same_hash() {
        let c1 = cam(20, 15);
        let c2 = cam(20, 15);
        assert_eq!(hash_state(&c1), hash_state(&c2));
    }

    #[test]
    fn test_set_screen_size_updates_dimensions() {
        let mut c = cam(20, 15); // screen_w=10, screen_h=8
        c.set_screen_size(20, 16, 40, 30);
        assert_eq!(c.screen_w, 20);
        assert_eq!(c.screen_h, 16);
    }

    #[test]
    fn test_set_screen_size_reclamps_within_world() {
        // Start near the right edge so growing the viewport forces reclamping.
        let mut c = Camera::new(38, 28, 10, 8, 40, 30); // top_left = (30, 22)
                                                        // Grow to 20×16 — needs top_left ≤ (20, 14) to fit.
        c.set_screen_size(20, 16, 40, 30);
        assert!(c.top_left_x + c.screen_w as i32 <= 40);
        assert!(c.top_left_y + c.screen_h as i32 <= 30);
    }

    #[test]
    fn test_set_screen_size_preserves_centre() {
        let mut c = cam(20, 15); // centre = (20, 15)
                                 // Resize to the same viewport: centre should be unchanged.
        c.set_screen_size(10, 8, 40, 30);
        let (cx, cy) = c.center();
        assert_eq!(cx, 20);
        assert_eq!(cy, 15);
    }

    #[test]
    fn test_contains_rect_empty_rect_never_overlaps() {
        let c = cam(20, 15);
        assert!(!c.contains_rect(5, 5, 5, 10)); // l == r
        assert!(!c.contains_rect(5, 5, 10, 5)); // t == b
        assert!(!c.contains_rect(10, 5, 5, 10)); // l > r
    }

    #[test]
    fn test_contains_rect_overlapping() {
        let c = cam(20, 15); // viewport: x [15,25), y [11,19)
        assert!(c.contains_rect(10, 10, 20, 16)); // overlaps left side
        assert!(c.contains_rect(20, 15, 30, 25)); // overlaps right side
        assert!(c.contains_rect(0, 0, 40, 30)); // fully contains viewport
    }

    #[test]
    fn test_contains_rect_non_overlapping() {
        let c = cam(20, 15); // viewport: x [15,25), y [11,19)
        assert!(!c.contains_rect(0, 0, 15, 11)); // just outside top-left
        assert!(!c.contains_rect(25, 19, 35, 25)); // just outside bottom-right
    }

    #[test]
    fn test_chebyshev_to_center_at_center_is_zero() {
        let c = cam(20, 15); // centre = (20, 15)
        assert_eq!(c.chebyshev_to_center(20, 15), 0);
    }

    #[test]
    fn test_chebyshev_to_center_horizontal_offset() {
        let c = cam(20, 15); // centre = (20, 15)
        assert_eq!(c.chebyshev_to_center(23, 15), 3); // dx=3, dy=0 → 3
    }

    #[test]
    fn test_chebyshev_to_center_diagonal_offset() {
        let c = cam(20, 15); // centre = (20, 15)
        assert_eq!(c.chebyshev_to_center(17, 12), 3); // dx=3, dy=3 → 3
    }

    #[test]
    fn test_chebyshev_to_center_asymmetric_uses_max() {
        let c = cam(20, 15); // centre = (20, 15)
        assert_eq!(c.chebyshev_to_center(22, 19), 4); // dx=2, dy=4 → 4
    }

    #[test]
    fn test_screen_distance_same_cell_is_zero() {
        assert_eq!(Camera::screen_distance(3, 5, 3, 5), 0);
    }

    #[test]
    fn test_screen_distance_horizontal() {
        assert_eq!(Camera::screen_distance(0, 0, 4, 0), 4);
    }

    #[test]
    fn test_screen_distance_diagonal_uses_max() {
        assert_eq!(Camera::screen_distance(1, 1, 4, 3), 3); // dx=3, dy=2 → 3
    }

    #[test]
    fn test_follow_no_pan_when_within_margin() {
        let mut cam = Camera::new(10, 10, 20, 10, 100, 100);
        let before = (cam.top_left_x, cam.top_left_y);
        cam.follow(cam.top_left_x + 5, cam.top_left_y + 3, 2, 100, 100);
        assert_eq!(
            (cam.top_left_x, cam.top_left_y),
            before,
            "within margin — no pan"
        );
    }

    #[test]
    fn test_follow_pans_right_when_target_near_right_edge() {
        let mut cam = Camera::new(0, 0, 20, 10, 100, 100);
        let right_edge_x = cam.top_left_x + 20 - 1; // exactly on right edge
        cam.follow(right_edge_x, cam.top_left_y + 5, 2, 100, 100);
        assert!(cam.top_left_x > 0, "should have panned right");
    }

    #[test]
    fn test_follow_pans_left_when_target_exits_left_margin() {
        let mut cam = Camera::new(10, 10, 20, 10, 100, 100);
        cam.follow(cam.top_left_x - 1, cam.top_left_y + 5, 2, 100, 100);
        assert!(cam.top_left_x < 10, "should have panned left");
    }

    #[test]
    fn test_world_to_screen_unclamped_visible_point() {
        // Direct struct init so top_left is exactly known.
        let cam = Camera {
            top_left_x: 5,
            top_left_y: 3,
            screen_w: 20,
            screen_h: 10,
        };
        let (sx, sy) = cam.world_to_screen_unclamped(7, 5);
        assert_eq!(sx, 2);
        assert_eq!(sy, 2);
    }

    #[test]
    fn test_world_to_screen_unclamped_offscreen_negative() {
        let cam = Camera {
            top_left_x: 10,
            top_left_y: 10,
            screen_w: 20,
            screen_h: 10,
        };
        let (sx, sy) = cam.world_to_screen_unclamped(5, 7);
        assert!(sx < 0, "point left of viewport should give negative sx");
        assert!(sy < 0, "point above viewport should give negative sy");
    }

    #[test]
    fn test_world_to_screen_unclamped_matches_world_to_screen_for_visible() {
        let cam = Camera {
            top_left_x: 0,
            top_left_y: 0,
            screen_w: 20,
            screen_h: 10,
        };
        let wx = 3;
        let wy = 4;
        let (sx, sy) = cam.world_to_screen_unclamped(wx, wy);
        let visible = cam.world_to_screen(wx, wy).unwrap();
        assert_eq!(sx as u32, visible.0);
        assert_eq!(sy as u32, visible.1);
    }

    #[test]
    fn test_screen_center_is_half_dimensions() {
        let cam = Camera {
            top_left_x: 0,
            top_left_y: 0,
            screen_w: 40,
            screen_h: 24,
        };
        assert_eq!(cam.screen_center(), (20, 12));
    }

    #[test]
    fn test_screen_center_truncates_on_odd() {
        let cam = Camera {
            top_left_x: 0,
            top_left_y: 0,
            screen_w: 5,
            screen_h: 3,
        };
        assert_eq!(cam.screen_center(), (2, 1));
    }

    #[test]
    fn test_screen_center_zero_size() {
        let cam = Camera {
            top_left_x: 0,
            top_left_y: 0,
            screen_w: 0,
            screen_h: 0,
        };
        assert_eq!(cam.screen_center(), (0, 0));
    }

    #[test]
    fn test_clamp_world_to_screen_in_bounds_identity() {
        let cam = Camera {
            top_left_x: 0,
            top_left_y: 0,
            screen_w: 20,
            screen_h: 10,
        };
        assert_eq!(cam.clamp_world_to_screen(5, 3), (5, 3));
    }

    #[test]
    fn test_clamp_world_to_screen_off_screen_clamps() {
        let cam = Camera {
            top_left_x: 0,
            top_left_y: 0,
            screen_w: 20,
            screen_h: 10,
        };
        assert_eq!(cam.clamp_world_to_screen(-5, 50), (0, 9));
    }

    #[test]
    fn test_clamp_world_to_screen_offset_viewport() {
        let cam = Camera {
            top_left_x: 10,
            top_left_y: 5,
            screen_w: 20,
            screen_h: 10,
        };
        assert_eq!(cam.clamp_world_to_screen(10, 5), (0, 0));
        assert_eq!(cam.clamp_world_to_screen(29, 14), (19, 9));
    }

    #[test]
    fn test_viewport_area_is_product() {
        let cam = Camera::new(10, 10, 20, 15, 100, 100);
        assert_eq!(cam.viewport_area(), 20 * 15);
    }

    #[test]
    fn test_viewport_area_zero_screen() {
        let cam = Camera::new(0, 0, 0, 5, 100, 100);
        assert_eq!(cam.viewport_area(), 0);
    }

    #[test]
    fn test_viewport_area_unit_screen() {
        let cam = Camera::new(0, 0, 1, 1, 10, 10);
        assert_eq!(cam.viewport_area(), 1);
    }
}
