//! 2D camera — world-to-screen and screen-to-world transforms.
//!
//! The camera defines what portion of the world is visible. Everything
//! submitted to [`crate::Render`] is in **world space**; the camera's
//! transform converts it to screen space before the backend draws.
//!
//! ```
//! use izanagi::{camera::Camera, Vec2};
//!
//! let mut cam = Camera::new(800.0, 600.0);
//! cam.pos = Vec2::new(100.0, 50.0); // look at world point (100, 50)
//! let screen = cam.world_to_screen(Vec2::new(100.0, 50.0));
//! // Center of the screen.
//! assert!((screen.x - 400.0).abs() < 1e-3);
//! assert!((screen.y - 300.0).abs() < 1e-3);
//! ```

use crate::math::{Rect, Vec2};

/// A 2D camera.
pub struct Camera {
    /// World position the camera looks at (screen center maps here).
    pub pos: Vec2,
    /// Zoom factor. 1.0 = 1 world unit per pixel. 2.0 = zoomed in 2×.
    pub zoom: f32,
    /// Rotation in radians (CCW positive). Most 2D games leave this 0.
    pub rotation: f32,
    /// Viewport width in pixels.
    pub viewport_w: f32,
    /// Viewport height in pixels.
    pub viewport_h: f32,
}

impl Camera {
    /// New camera centred at the origin with no rotation and 1× zoom.
    pub fn new(viewport_w: f32, viewport_h: f32) -> Self {
        Self {
            pos: Vec2::ZERO,
            zoom: 1.0,
            rotation: 0.0,
            viewport_w,
            viewport_h,
        }
    }

    /// Convert a world-space point to screen-space pixels.
    ///
    /// Screen origin is the top-left corner of the viewport.
    pub fn world_to_screen(&self, world: Vec2) -> Vec2 {
        let dx = world.x - self.pos.x;
        let dy = world.y - self.pos.y;
        // Apply rotation.
        let (s, c) = self.rotation.sin_cos();
        let rx = dx * c - dy * s;
        let ry = dx * s + dy * c;
        Vec2::new(rx * self.zoom + self.viewport_w * 0.5, ry * self.zoom + self.viewport_h * 0.5)
    }

    /// Convert a screen-space pixel to world-space.
    pub fn screen_to_world(&self, screen: Vec2) -> Vec2 {
        let dx = (screen.x - self.viewport_w * 0.5) / self.zoom;
        let dy = (screen.y - self.viewport_h * 0.5) / self.zoom;
        // Inverse rotation.
        let (s, c) = (-self.rotation).sin_cos();
        Vec2::new(dx * c - dy * s + self.pos.x, dx * s + dy * c + self.pos.y)
    }

    /// The visible rectangle in world space.
    pub fn visible_rect(&self) -> Rect {
        let half_w = self.viewport_w * 0.5 / self.zoom;
        let half_h = self.viewport_h * 0.5 / self.zoom;
        Rect::new(self.pos.x - half_w, self.pos.y - half_h, half_w * 2.0, half_h * 2.0)
    }

    /// Smoothly follow a target position (exponential decay).
    ///
    /// `speed` of 5.0 is a good default — tighter follow is higher.
    pub fn follow(&mut self, target: Vec2, speed: f32, dt: f32) {
        let t = (1.0 - (-speed * dt).exp()).clamp(0.0, 1.0);
        self.pos = self.pos.lerp(target, t);
    }

    /// Clamp the camera so it never shows outside `world_bounds`.
    pub fn clamp_to(&mut self, world_bounds: &Rect) {
        let half_w = self.viewport_w * 0.5 / self.zoom;
        let half_h = self.viewport_h * 0.5 / self.zoom;
        self.pos.x = self.pos.x.clamp(
            world_bounds.x + half_w,
            (world_bounds.x + world_bounds.w - half_w).max(world_bounds.x + half_w),
        );
        self.pos.y = self.pos.y.clamp(
            world_bounds.y + half_h,
            (world_bounds.y + world_bounds.h - half_h).max(world_bounds.y + half_h),
        );
    }

    /// Resize the viewport (call when the window is resized).
    pub fn resize(&mut self, w: f32, h: f32) {
        self.viewport_w = w;
        self.viewport_h = h;
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new(800.0, 600.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_maps_to_half_viewport() {
        let cam = Camera::new(800.0, 600.0);
        let s = cam.world_to_screen(Vec2::ZERO);
        assert!((s.x - 400.0).abs() < 1e-3);
        assert!((s.y - 300.0).abs() < 1e-3);
    }

    #[test]
    fn round_trip_world_screen() {
        let mut cam = Camera::new(800.0, 600.0);
        cam.pos = Vec2::new(50.0, -30.0);
        cam.zoom = 2.0;
        let world = Vec2::new(123.0, -45.6);
        let screen = cam.world_to_screen(world);
        let back = cam.screen_to_world(screen);
        assert!((back.x - world.x).abs() < 1e-3, "{} vs {}", back.x, world.x);
        assert!((back.y - world.y).abs() < 1e-3);
    }

    #[test]
    fn zoom_shrinks_visible_rect() {
        let mut cam = Camera::new(800.0, 600.0);
        let r1 = cam.visible_rect();
        cam.zoom = 2.0;
        let r2 = cam.visible_rect();
        assert!(r2.w < r1.w);
        assert!(r2.h < r1.h);
    }

    #[test]
    fn follow_converges_toward_target() {
        let mut cam = Camera::new(800.0, 600.0);
        let target = Vec2::new(100.0, 0.0);
        for _ in 0..60 {
            cam.follow(target, 5.0, 1.0 / 60.0);
        }
        assert!((cam.pos.x - target.x).abs() < 1.0);
    }

    #[test]
    fn clamp_keeps_camera_inside() {
        let mut cam = Camera::new(800.0, 600.0);
        cam.pos = Vec2::new(-9999.0, -9999.0);
        let bounds = Rect::new(0.0, 0.0, 2000.0, 1500.0);
        cam.clamp_to(&bounds);
        let vr = cam.visible_rect();
        assert!(vr.x >= bounds.x - 1e-3);
        assert!(vr.y >= bounds.y - 1e-3);
    }

    #[test]
    fn round_trip_with_rotation() {
        let mut cam = Camera::new(800.0, 600.0);
        cam.rotation = std::f32::consts::FRAC_PI_4;
        let world = Vec2::new(20.0, 30.0);
        let screen = cam.world_to_screen(world);
        let back = cam.screen_to_world(screen);
        assert!((back.x - world.x).abs() < 1e-2);
        assert!((back.y - world.y).abs() < 1e-2);
    }
}
