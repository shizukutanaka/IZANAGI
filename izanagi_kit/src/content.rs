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
    pub fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
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
}
