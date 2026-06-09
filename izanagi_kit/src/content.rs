//! Authored game-content data model.
//!
//! Decomposition of "what a game is made of", data layer (designer-authored):
//!   - Prefab  : an entity template (glyph, color, stats, flags)
//!   - Tile    : a named map cell appearance
//!   - Spawn   : a prefab placed at a grid coordinate
//!   - Level   : a grid of tile glyphs + spawns
//!   - Content : the whole authored bundle
//!
//! Behavior (systems), runtime state (ECS world), presentation (TerminalBackend)
//! and control (loop/input/state machine) are separate concerns and are NOT
//! authored here. This module is the input to the content pipeline:
//!
//! ```text
//!   text -> parser -> Content -> validator -> loader -> ECS world
//! ```
//!
//! Maps use `BTreeMap`/ordered `Vec` so iteration is deterministic, keeping the
//! pipeline consistent with the engine's bit-exact-replay goal.

use std::collections::BTreeMap;

/// 24-bit RGB color (matches the `#RRGGBB` authoring syntax).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    /// Construct a color from its channels. `const` so it can seed palettes.
    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b }
    }

    pub fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// Build a color from HSV: `hue` in degrees (wrapped mod 360), `sat` and
    /// `val` in `0..=255`. Pure integer arithmetic (no float), so it is
    /// deterministic across targets — handy for procedural palettes and
    /// rainbow/heat gradients. `sat == 0` yields a gray of brightness `val`.
    pub fn from_hsv(hue: u32, sat: u8, val: u8) -> Color {
        let v = val as u32;
        if sat == 0 {
            return Color {
                r: val,
                g: val,
                b: val,
            };
        }
        let s = sat as u32;
        let h = hue % 360;
        let region = h / 60; // 0..=5
        let rem = (h % 60) * 255 / 60; // fractional position within the sextant, 0..=255
        let p = v * (255 - s) / 255;
        let q = v * (255 - s * rem / 255) / 255;
        let t = v * (255 - s * (255 - rem) / 255) / 255;
        let (r, g, b) = match region {
            0 => (v, t, p),
            1 => (q, v, p),
            2 => (p, v, t),
            3 => (p, q, v),
            4 => (t, p, v),
            _ => (v, p, q),
        };
        Color {
            r: r as u8,
            g: g as u8,
            b: b as u8,
        }
    }

    /// Linearly interpolate each channel from `a` to `b` by the ratio
    /// `num/den` (integer, no float). `num == 0` yields `a`, `num == den`
    /// yields `b`; values outside `[0, den]` extrapolate and clamp per channel
    /// to `0..=255`. A zero denominator returns `a` unchanged. Use this for
    /// heat-map gradients and fades instead of hand-rolling channel math.
    pub fn lerp(a: Color, b: Color, num: i32, den: i32) -> Color {
        if den == 0 {
            return a;
        }
        let ch = |ca: u8, cb: u8| -> u8 {
            let v = ca as i32 + (cb as i32 - ca as i32) * num / den;
            v.clamp(0, 255) as u8
        };
        Color {
            r: ch(a.r, b.r),
            g: ch(a.g, b.g),
            b: ch(a.b, b.b),
        }
    }

    /// Desaturate to a gray of equal perceived luma, using integer Rec. 601
    /// weights (`0.299, 0.587, 0.114` scaled to sum 256). No float.
    #[inline]
    pub fn grayscale(self) -> Color {
        let y =
            ((self.r as u32 * 77 + self.g as u32 * 150 + self.b as u32 * 29) >> 8).min(255) as u8;
        Color { r: y, g: y, b: y }
    }

    /// Channel-wise complement: `rgb(255 − r, 255 − g, 255 − b)`.
    /// Useful for contrast highlights and selection indicators.
    #[inline]
    pub const fn invert(self) -> Color {
        Color {
            r: 255 - self.r,
            g: 255 - self.g,
            b: 255 - self.b,
        }
    }

    /// Perceived luma as a single `u8`, using integer Rec. 601 weights
    /// (`0.299 r + 0.587 g + 0.114 b`, scaled to sum 256). Returns the same
    /// value as all three channels of [`grayscale`](Self::grayscale).
    #[inline]
    pub fn luminance(self) -> u8 {
        ((self.r as u32 * 77 + self.g as u32 * 150 + self.b as u32 * 29) >> 8).min(255) as u8
    }

    /// Scale every channel by the ratio `num/den` (integer), clamping to
    /// `0..=255`. `num < den` dims, `num > den` brightens; a zero denominator
    /// returns the color unchanged. Handy for shading by distance or light.
    pub fn scale(self, num: i32, den: i32) -> Color {
        if den == 0 {
            return self;
        }
        let ch = |c: u8| -> u8 { (c as i32 * num / den).clamp(0, 255) as u8 };
        Color {
            r: ch(self.r),
            g: ch(self.g),
            b: ch(self.b),
        }
    }

    /// Composite `fg` over `bg` with integer alpha: `alpha = 0` yields `fg`,
    /// `alpha = 255` yields `bg`. Formula: `(fg * (255 − alpha) + bg * alpha) / 255`
    /// per channel — no float, deterministic across targets.
    pub fn alpha_blend(fg: Color, bg: Color, alpha: u8) -> Color {
        let a = alpha as u32;
        let ia = 255 - a;
        Color {
            r: ((fg.r as u32 * ia + bg.r as u32 * a) / 255) as u8,
            g: ((fg.g as u32 * ia + bg.g as u32 * a) / 255) as u8,
            b: ((fg.b as u32 * ia + bg.b as u32 * a) / 255) as u8,
        }
    }
}

impl crate::world_hash::DetHash for Color {
    #[inline]
    fn det_hash(&self, hasher: &mut crate::world_hash::Fnv1a) {
        hasher.write_bytes(&[self.r, self.g, self.b]);
    }
}

/// An entity template. Instantiated once per [`Spawn`] that names it.
#[derive(Clone, Debug)]
pub struct Prefab {
    pub name: String,
    pub glyph: char,
    pub color: Color,
    pub stats: BTreeMap<String, i32>,
    pub flags: Vec<String>,
}

impl Prefab {
    pub fn new(name: String) -> Self {
        Self {
            name,
            glyph: '?',
            color: Color {
                r: 0xFF,
                g: 0xFF,
                b: 0xFF,
            },
            stats: BTreeMap::new(),
            flags: Vec::new(),
        }
    }
}

/// A named map-cell appearance.
#[derive(Clone, Debug)]
pub struct Tile {
    pub name: String,
    pub glyph: char,
    pub color: Color,
}

/// A prefab placed at a level coordinate.
#[derive(Clone, Debug)]
pub struct Spawn {
    pub prefab: String,
    pub x: u32,
    pub y: u32,
}

/// A level: a `width`x`height` grid of glyph rows plus spawns.
#[derive(Clone, Debug)]
pub struct Level {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub rows: Vec<String>,
    pub spawns: Vec<Spawn>,
}

/// The complete authored bundle produced by the parser.
#[derive(Clone, Debug, Default)]
pub struct Content {
    pub prefabs: Vec<Prefab>,
    pub tiles: Vec<Tile>,
    pub levels: Vec<Level>,
}

impl Content {
    pub fn prefab(&self, name: &str) -> Option<&Prefab> {
        self.prefabs.iter().find(|p| p.name == name)
    }

    pub fn level(&self, name: &str) -> Option<&Level> {
        self.levels.iter().find(|l| l.name == name)
    }
}

/// Severity of a pipeline diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// A single parser/validator finding, tied to a 1-based source line and an
/// optional 1-based column (0 == column unknown, e.g. semantic checks).
#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub line: usize,
    pub col: usize,
    pub severity: Severity,
    pub message: String,
}

impl Diagnostic {
    pub fn error(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            col: 0,
            severity: Severity::Error,
            message: message.into(),
        }
    }

    pub fn warning(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            col: 0,
            severity: Severity::Warning,
            message: message.into(),
        }
    }

    /// Error with a known column, enabling a caret in the rendered output.
    pub fn error_at(line: usize, col: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            col,
            severity: Severity::Error,
            message: message.into(),
        }
    }

    pub fn warning_at(line: usize, col: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            col,
            severity: Severity::Warning,
            message: message.into(),
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// Renders a clang/rustc-style report with the offending source line and a
    /// caret under the column. `source` is the full input; `file` labels the
    /// location. Falls back to a one-line form when no column/line is known.
    pub fn render(&self, file: &str, source: &str) -> String {
        let sev = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        if self.line == 0 {
            return format!("{file}: {sev}: {}", self.message);
        }
        let mut out = format!("{file}:{}:{}: {sev}: {}", self.line, self.col, self.message);
        if let Some(text) = source.lines().nth(self.line - 1) {
            out.push('\n');
            out.push_str(text);
            if self.col > 0 {
                out.push('\n');
                // Caret aligned to the column (1-based). Tabs in source would
                // misalign; content uses spaces, so a simple repeat suffices.
                out.push_str(&" ".repeat(self.col.saturating_sub(1)));
                out.push('^');
            }
        }
        out
    }
}

/// Parses `#RRGGBB` into a [`Color`] without floats or panics.
///
/// Operates on raw bytes (never slices the `&str`), so a 7-byte input that
/// contains a multi-byte UTF-8 char — e.g. `#aéABC` — is rejected with an error
/// rather than panicking on a non-char-boundary slice.
pub fn parse_color(s: &str) -> Result<Color, String> {
    let bytes = s.as_bytes();
    if bytes.len() != 7 || bytes[0] != b'#' {
        return Err(format!("color must be #RRGGBB, got {:?}", s));
    }
    // Parse a hex nibble straight from a byte; non-ASCII/non-hex bytes (incl.
    // UTF-8 continuation bytes) fail here instead of being sliced into.
    let nibble = |b: u8| (b as char).to_digit(16);
    let hex = |i: usize| -> Result<u8, String> {
        match (nibble(bytes[i]), nibble(bytes[i + 1])) {
            (Some(hi), Some(lo)) => Ok((hi * 16 + lo) as u8),
            _ => Err(format!("invalid hex in color {:?}", s)),
        }
    };
    Ok(Color {
        r: hex(1)?,
        g: hex(3)?,
        b: hex(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_color_valid() {
        assert_eq!(
            parse_color("#00C4CC").unwrap(),
            Color {
                r: 0,
                g: 0xC4,
                b: 0xCC
            }
        );
    }

    #[test]
    fn test_parse_color_roundtrip() {
        let c = Color {
            r: 0x12,
            g: 0xAB,
            b: 0xFF,
        };
        assert_eq!(parse_color(&c.to_hex()).unwrap(), c);
    }

    #[test]
    fn test_parse_color_rejects_bad_length() {
        assert!(parse_color("#FFF").is_err());
        assert!(parse_color("00C4CC").is_err());
    }

    #[test]
    fn test_parse_color_rejects_non_hex() {
        assert!(parse_color("#GG0000").is_err());
    }

    #[test]
    fn test_parse_color_rejects_multibyte_without_panic() {
        // 7 bytes but not 7 ASCII chars: `#` + `a` + `é`(2 bytes) + `ABC`. The
        // old fixed-offset slicing panicked on the non-char-boundary; now it is
        // a clean error. Reaching the assert at all proves no panic occurred.
        assert_eq!("#aéABC".len(), 7, "fixture must be exactly 7 bytes");
        assert!(parse_color("#aéABC").is_err());
        // A multibyte char straddling a different nibble boundary, also rejected.
        assert_eq!("#é0000".len(), 7);
        assert!(parse_color("#é0000").is_err());
    }

    #[test]
    fn test_content_lookup() {
        let mut c = Content::default();
        c.prefabs.push(Prefab::new("hero".into()));
        assert!(c.prefab("hero").is_some());
        assert!(c.prefab("villain").is_none());
    }

    #[test]
    fn test_render_with_caret_shows_source_and_marker() {
        let src = "prefab a\n  glyph @@\n";
        let d = Diagnostic::error_at(2, 9, "glyph must be one character");
        let r = d.render("x.game", src);
        assert!(r.contains("x.game:2:9: error"));
        assert!(r.contains("  glyph @@"));
        assert!(r.contains("^"));
    }

    #[test]
    fn test_render_semantic_diag_without_line() {
        let d = Diagnostic::error(0, "duplicate prefab 'g'");
        let r = d.render("x.game", "");
        assert_eq!(r, "x.game: error: duplicate prefab 'g'");
    }

    // --- Color operations ---

    #[test]
    fn test_color_rgb_const() {
        const C: Color = Color::rgb(10, 20, 30);
        assert_eq!(
            C,
            Color {
                r: 10,
                g: 20,
                b: 30
            }
        );
    }

    #[test]
    fn test_color_lerp_endpoints_and_midpoint() {
        let black = Color::rgb(0, 0, 0);
        let white = Color::rgb(255, 255, 255);
        assert_eq!(Color::lerp(black, white, 0, 1), black);
        assert_eq!(Color::lerp(black, white, 1, 1), white);
        assert_eq!(Color::lerp(black, white, 1, 2), Color::rgb(127, 127, 127));
    }

    #[test]
    fn test_color_lerp_per_channel() {
        let a = Color::rgb(0, 100, 200);
        let b = Color::rgb(100, 100, 0);
        // quarter of the way: r 0→25, g unchanged, b 200→150
        assert_eq!(Color::lerp(a, b, 1, 4), Color::rgb(25, 100, 150));
    }

    #[test]
    fn test_color_lerp_zero_den_returns_a() {
        let a = Color::rgb(1, 2, 3);
        assert_eq!(Color::lerp(a, Color::rgb(9, 9, 9), 1, 0), a);
    }

    #[test]
    fn test_color_lerp_extrapolation_clamps() {
        // num > den extrapolates past b but channels clamp to 0..=255.
        let a = Color::rgb(200, 0, 0);
        let b = Color::rgb(255, 0, 0);
        assert_eq!(Color::lerp(a, b, 4, 1).r, 255);
    }

    #[test]
    fn test_color_grayscale_extremes_and_luma() {
        assert_eq!(Color::rgb(0, 0, 0).grayscale(), Color::rgb(0, 0, 0));
        assert_eq!(
            Color::rgb(255, 255, 255).grayscale(),
            Color::rgb(255, 255, 255)
        );
        // Pure green carries the most luma weight (150/256 ≈ 0.586).
        let g = Color::rgb(0, 255, 0).grayscale();
        assert_eq!(g.r, g.g);
        assert_eq!(g.g, g.b);
        assert!((149..=150).contains(&g.r), "green luma ≈149, got {}", g.r);
    }

    #[test]
    fn test_color_scale_dim_brighten_clamp() {
        let c = Color::rgb(100, 200, 50);
        assert_eq!(c.scale(1, 2), Color::rgb(50, 100, 25)); // half
        assert_eq!(c.scale(4, 1).g, 255); // brighten clamps
        assert_eq!(c.scale(1, 0), c); // zero den is a no-op
    }

    #[test]
    fn test_color_from_hsv_primaries() {
        assert_eq!(Color::from_hsv(0, 255, 255), Color::rgb(255, 0, 0)); // red
        assert_eq!(Color::from_hsv(120, 255, 255), Color::rgb(0, 255, 0)); // green
        assert_eq!(Color::from_hsv(240, 255, 255), Color::rgb(0, 0, 255)); // blue
    }

    #[test]
    fn test_color_from_hsv_zero_sat_is_gray() {
        assert_eq!(Color::from_hsv(200, 0, 137), Color::rgb(137, 137, 137));
        assert_eq!(Color::from_hsv(0, 0, 0), Color::rgb(0, 0, 0));
    }

    #[test]
    fn test_color_from_hsv_hue_wraps() {
        assert_eq!(Color::from_hsv(360, 255, 255), Color::from_hsv(0, 255, 255));
        assert_eq!(
            Color::from_hsv(480, 255, 255),
            Color::from_hsv(120, 255, 255)
        );
    }

    #[test]
    fn test_color_invert_black_white() {
        assert_eq!(Color::rgb(0, 0, 0).invert(), Color::rgb(255, 255, 255));
        assert_eq!(Color::rgb(255, 255, 255).invert(), Color::rgb(0, 0, 0));
    }

    #[test]
    fn test_color_invert_double_is_identity() {
        let c = Color::rgb(100, 150, 200);
        assert_eq!(c.invert().invert(), c);
    }

    #[test]
    fn test_color_luminance_matches_grayscale() {
        let c = Color::rgb(77, 200, 30);
        assert_eq!(c.luminance(), c.grayscale().r);
    }

    #[test]
    fn test_color_luminance_extremes() {
        assert_eq!(Color::rgb(0, 0, 0).luminance(), 0);
        assert_eq!(Color::rgb(255, 255, 255).luminance(), 255);
    }

    #[test]
    fn test_color_from_hsv_value_scales_brightness() {
        // Half value on pure red → darker red, no other channel introduced.
        let dim = Color::from_hsv(0, 255, 128);
        assert_eq!(dim, Color::rgb(128, 0, 0));
    }

    #[test]
    fn test_alpha_blend_zero_alpha_yields_fg() {
        let fg = Color::rgb(200, 100, 50);
        let bg = Color::rgb(10, 20, 30);
        assert_eq!(Color::alpha_blend(fg, bg, 0), fg);
    }

    #[test]
    fn test_alpha_blend_full_alpha_yields_bg() {
        let fg = Color::rgb(200, 100, 50);
        let bg = Color::rgb(10, 20, 30);
        assert_eq!(Color::alpha_blend(fg, bg, 255), bg);
    }

    #[test]
    fn test_alpha_blend_midpoint_channels() {
        let fg = Color::rgb(0, 0, 0);
        let bg = Color::rgb(255, 200, 100);
        let mid = Color::alpha_blend(fg, bg, 128);
        // (0*127 + 255*128)/255 = 128, (0*127 + 200*128)/255 = 100, (0*127 + 100*128)/255 = 50
        assert_eq!(mid.r, 128);
        assert_eq!(mid.g, 100);
        assert_eq!(mid.b, 50);
    }
}
