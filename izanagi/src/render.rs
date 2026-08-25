//! Renderer.
//!
//! Immediate-mode draw list. You submit rectangles, sprites, and text every
//! frame. A backend (wgpu, OpenGL, software) consumes the list. The null
//! backend included here records the list for testing and profiling.

/// A color in linear-ish RGBA.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Color {
    /// Red (0.0 to 1.0).
    pub r: f32,
    /// Green (0.0 to 1.0).
    pub g: f32,
    /// Blue (0.0 to 1.0).
    pub b: f32,
    /// Alpha (0.0 to 1.0).
    pub a: f32,
}

impl Color {
    /// Opaque white.
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    /// Opaque black.
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    /// Fully transparent.
    pub const CLEAR: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    /// Construct from RGBA components.
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Construct from 8-bit RGB, opaque.
    pub fn rgb8(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }

    /// Construct from 8-bit RGBA.
    pub fn rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    /// Same colour with a new alpha.
    pub fn with_alpha(self, a: f32) -> Self {
        Self {
            a: a.clamp(0.0, 1.0),
            ..self
        }
    }

    /// Linear interpolation between two colours (component-wise).
    pub fn lerp(self, to: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            r: self.r + (to.r - self.r) * t,
            g: self.g + (to.g - self.g) * t,
            b: self.b + (to.b - self.b) * t,
            a: self.a + (to.a - self.a) * t,
        }
    }

    /// Clamp every channel to `[0, 1]`.
    pub fn saturate(self) -> Self {
        Self {
            r: self.r.clamp(0.0, 1.0),
            g: self.g.clamp(0.0, 1.0),
            b: self.b.clamp(0.0, 1.0),
            a: self.a.clamp(0.0, 1.0),
        }
    }
}

impl std::ops::Mul<f32> for Color {
    type Output = Self;
    /// Scale every channel (including alpha) by a scalar.
    fn mul(self, s: f32) -> Self {
        Self {
            r: self.r * s,
            g: self.g * s,
            b: self.b * s,
            a: self.a * s,
        }
    }
}

impl std::ops::Mul<Color> for Color {
    type Output = Self;
    /// Component-wise multiply (modulate).
    fn mul(self, o: Self) -> Self {
        Self {
            r: self.r * o.r,
            g: self.g * o.g,
            b: self.b * o.b,
            a: self.a * o.a,
        }
    }
}

impl std::ops::Add for Color {
    type Output = Self;
    /// Component-wise add. Useful for additive blending in user code.
    fn add(self, o: Self) -> Self {
        Self {
            r: self.r + o.r,
            g: self.g + o.g,
            b: self.b + o.b,
            a: self.a + o.a,
        }
    }
}

/// A single draw command.
#[derive(Copy, Clone, Debug)]
#[allow(missing_docs)]
pub enum Draw {
    Clear(Color),
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: Color,
    },
    Text {
        x: f32,
        y: f32,
        size: f32,
        color: Color,
        text_id: usize,
    },
    /// A line segment from (x1, y1) to (x2, y2).
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        thickness: f32,
        color: Color,
    },
    /// An outlined or filled circle.
    Circle {
        cx: f32,
        cy: f32,
        radius: f32,
        filled: bool,
        color: Color,
    },
    /// Push a clip rectangle. Subsequent draws are clipped until [`Draw::ScissorPop`].
    ScissorPush {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    },
    /// Pop the most recent clip rect.
    ScissorPop,
    Sprite {
        /// Destination top-left in screen / world space.
        x: f32,
        y: f32,
        /// Destination size on screen.
        w: f32,
        h: f32,
        /// Atlas asset handle (opaque to the engine).
        atlas: u32,
        /// Source rect in atlas pixels.
        src_x: u32,
        src_y: u32,
        src_w: u32,
        src_h: u32,
        /// Tint colour. Multiplied with the sprite.
        tint: Color,
    },
}

/// The renderer.
pub struct Render {
    list: Vec<Draw>,
    texts: Vec<String>,
    width: u32,
    height: u32,
    clear: Color,
}

impl Render {
    /// New renderer with a 1920x1080 default surface.
    pub fn new() -> Self {
        Self {
            list: Vec::new(),
            texts: Vec::new(),
            width: 1920,
            height: 1080,
            clear: Color::BLACK,
        }
    }

    /// Resize the target surface.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    /// Current surface size.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Color to clear with at the start of each frame.
    pub fn set_clear(&mut self, color: Color) {
        self.clear = color;
    }

    /// Submit a filled rectangle.
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        self.list.push(Draw::Rect { x, y, w, h, color });
    }

    /// Submit text at a position.
    pub fn text(&mut self, x: f32, y: f32, size: f32, color: Color, text: impl Into<String>) {
        let text_id = self.texts.len();
        self.texts.push(text.into());
        self.list.push(Draw::Text {
            x,
            y,
            size,
            color,
            text_id,
        });
    }

    /// Submit a sprite from an atlas, with a destination rect, source rect,
    /// and a multiplicative tint.
    #[allow(clippy::too_many_arguments)]
    pub fn sprite(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        atlas: u32,
        src_x: u32,
        src_y: u32,
        src_w: u32,
        src_h: u32,
        tint: Color,
    ) {
        self.list.push(Draw::Sprite {
            x,
            y,
            w,
            h,
            atlas,
            src_x,
            src_y,
            src_w,
            src_h,
            tint,
        });
    }

    /// Submit a line from (x1, y1) to (x2, y2). `thickness` is in pixels.
    pub fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, color: Color) {
        self.list.push(Draw::Line {
            x1,
            y1,
            x2,
            y2,
            thickness,
            color,
        });
    }

    /// Submit a filled circle.
    pub fn circle_fill(&mut self, cx: f32, cy: f32, radius: f32, color: Color) {
        self.list.push(Draw::Circle {
            cx,
            cy,
            radius,
            filled: true,
            color,
        });
    }

    /// Submit a circle outline.
    pub fn circle(&mut self, cx: f32, cy: f32, radius: f32, color: Color) {
        self.list.push(Draw::Circle {
            cx,
            cy,
            radius,
            filled: false,
            color,
        });
    }

    /// Push a scissor (clip) rectangle. All subsequent draws are clipped until
    /// [`Render::scissor_pop`]. Backends that don't implement clipping ignore it.
    pub fn scissor_push(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.list.push(Draw::ScissorPush { x, y, w, h });
    }

    /// Pop the most recent scissor rect.
    pub fn scissor_pop(&mut self) {
        self.list.push(Draw::ScissorPop);
    }

    /// Take the draw list for this frame and clear for the next.
    pub fn drain(&mut self) -> (Color, Vec<Draw>, Vec<String>) {
        let list = std::mem::take(&mut self.list);
        let texts = std::mem::take(&mut self.texts);
        (self.clear, list, texts)
    }

    /// Current draw list length. For diagnostics and tests.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// True if nothing is submitted.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

impl Default for Render {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_appears_in_draw_list() {
        let mut r = Render::new();
        r.line(0.0, 0.0, 100.0, 100.0, 2.0, Color::WHITE);
        let (_, list, _) = r.drain();
        assert!(matches!(list[0], Draw::Line { .. }));
    }

    #[test]
    fn circle_fill_and_outline() {
        let mut r = Render::new();
        r.circle_fill(50.0, 50.0, 10.0, Color::WHITE);
        r.circle(50.0, 50.0, 10.0, Color::BLACK);
        let (_, list, _) = r.drain();
        match list[0] {
            Draw::Circle { filled, .. } => assert!(filled),
            _ => panic!(),
        }
        match list[1] {
            Draw::Circle { filled, .. } => assert!(!filled),
            _ => panic!(),
        }
    }

    #[test]
    fn color_lerp_endpoints() {
        let a = Color::rgba(0.0, 0.0, 0.0, 1.0);
        let b = Color::rgba(1.0, 1.0, 1.0, 1.0);
        let mid = a.lerp(b, 0.5);
        assert!((mid.r - 0.5).abs() < 1e-5);
        assert_eq!(a.lerp(b, 0.0), a);
        assert_eq!(a.lerp(b, 1.0), b);
    }

    #[test]
    fn color_with_alpha_clamps() {
        let c = Color::WHITE.with_alpha(2.0);
        assert_eq!(c.a, 1.0);
        let c = Color::WHITE.with_alpha(-1.0);
        assert_eq!(c.a, 0.0);
    }

    #[test]
    fn color_mul_scalar_scales_all_channels() {
        let c = Color::rgba(1.0, 0.5, 0.25, 1.0) * 0.5;
        assert!((c.r - 0.5).abs() < 1e-5);
        assert!((c.a - 0.5).abs() < 1e-5);
    }

    #[test]
    fn color_mul_color_modulates() {
        let a = Color::rgba(1.0, 0.5, 0.0, 1.0);
        let b = Color::rgba(0.5, 1.0, 1.0, 0.8);
        let c = a * b;
        assert!((c.r - 0.5).abs() < 1e-5);
        assert!((c.g - 0.5).abs() < 1e-5);
        assert!((c.a - 0.8).abs() < 1e-5);
    }

    #[test]
    fn scissor_push_pop_in_draw_list() {
        let mut r = Render::new();
        r.scissor_push(10.0, 10.0, 100.0, 80.0);
        r.rect(0.0, 0.0, 200.0, 200.0, Color::WHITE);
        r.scissor_pop();
        let (_, list, _) = r.drain();
        assert_eq!(list.len(), 3);
        assert!(matches!(list[0], Draw::ScissorPush { .. }));
        assert!(matches!(list[2], Draw::ScissorPop));
    }

    #[test]
    fn sprite_appears_in_draw_list() {
        let mut r = Render::new();
        r.sprite(10.0, 20.0, 16.0, 16.0, 1, 0, 0, 16, 16, Color::WHITE);
        assert_eq!(r.len(), 1);
        let (_, list, _) = r.drain();
        match list[0] {
            Draw::Sprite {
                x, y, w, h, atlas, ..
            } => {
                assert_eq!((x, y, w, h), (10.0, 20.0, 16.0, 16.0));
                assert_eq!(atlas, 1);
            }
            _ => panic!("expected Sprite variant"),
        }
    }

    #[test]
    fn submit_and_drain() {
        let mut r = Render::new();
        r.rect(0.0, 0.0, 10.0, 10.0, Color::WHITE);
        r.text(0.0, 0.0, 12.0, Color::BLACK, "hi");
        assert_eq!(r.len(), 2);
        let (clear, list, texts) = r.drain();
        assert_eq!(clear, Color::BLACK);
        assert_eq!(list.len(), 2);
        assert_eq!(texts, vec!["hi".to_string()]);
        assert!(r.is_empty());
    }

    #[test]
    fn rgb8_matches_rgba() {
        let c = Color::rgb8(255, 128, 0);
        assert!((c.r - 1.0).abs() < 1e-5);
        assert!((c.g - 128.0 / 255.0).abs() < 1e-5);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn resize_sets_size() {
        let mut r = Render::new();
        r.resize(640, 480);
        assert_eq!(r.size(), (640, 480));
    }
}
