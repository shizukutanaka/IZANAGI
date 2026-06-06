//! Terminal cell buffer — the presentation layer.
//!
//! IZANAGI is "terminal-first", but the kit had no way to actually draw. This
//! module is a headless [`Screen`] of [`Cell`]s (a glyph plus 24-bit foreground
//! and background colour) with drawing primitives, double-buffered change
//! tracking, and a deterministic 24-bit-ANSI serialiser.
//!
//! Headless & deterministic by design: drawing only mutates an in-memory grid,
//! so it runs unchanged in CI, snapshot-tests by inspecting cells, and folds
//! into the world hash via [`DetHash`]. Producing real terminal output is just
//! [`Screen::to_ansi`] (full frame) — actually writing it to a tty is the
//! caller's job, keeping this module free of OS I/O.

use crate::content::Color;
use crate::world_hash::{DetHash, Fnv1a};

/// Default foreground: white.
const DEFAULT_FG: Color = Color {
    r: 0xC0,
    g: 0xC0,
    b: 0xC0,
};
/// Default background: black.
const DEFAULT_BG: Color = Color { r: 0, g: 0, b: 0 };

/// One screen cell: a character and its colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub glyph: char,
    pub fg: Color,
    pub bg: Color,
}

impl Cell {
    /// A blank cell: space on the default background.
    pub const fn blank() -> Cell {
        Cell {
            glyph: ' ',
            fg: DEFAULT_FG,
            bg: DEFAULT_BG,
        }
    }
}

impl Default for Cell {
    fn default() -> Cell {
        Cell::blank()
    }
}

impl DetHash for Cell {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.glyph as u32);
        self.fg.det_hash(hasher);
        self.bg.det_hash(hasher);
    }
}

/// A `width × height` grid of [`Cell`]s with a back buffer for diffing.
#[derive(Clone, Debug)]
pub struct Screen {
    width: u32,
    height: u32,
    cells: Vec<Cell>,
    /// Snapshot taken at the last [`Screen::present`]; diffed against `cells`.
    prev: Vec<Cell>,
}

impl Screen {
    /// A blank screen of the given size.
    pub fn new(width: u32, height: u32) -> Screen {
        let len = (width as usize) * (height as usize);
        Screen {
            width,
            height,
            cells: vec![Cell::blank(); len],
            prev: vec![Cell::blank(); len],
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

    #[inline]
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            None
        } else {
            Some((y as u32 * self.width + x as u32) as usize)
        }
    }

    /// The cell at `(x, y)`, or `None` if out of bounds.
    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Option<&Cell> {
        self.index(x, y).map(|i| &self.cells[i])
    }

    /// Overwrite the cell at `(x, y)`. Out-of-bounds writes are silently
    /// clipped (no panic), so callers can draw without bounds-checking.
    #[inline]
    pub fn put(&mut self, x: i32, y: i32, cell: Cell) {
        if let Some(i) = self.index(x, y) {
            self.cells[i] = cell;
        }
    }

    /// Set a glyph and colours at `(x, y)` (clipped).
    #[inline]
    pub fn set(&mut self, x: i32, y: i32, glyph: char, fg: Color, bg: Color) {
        self.put(x, y, Cell { glyph, fg, bg });
    }

    /// Fill the whole screen with one cell.
    pub fn clear(&mut self, cell: Cell) {
        for c in &mut self.cells {
            *c = cell;
        }
    }

    /// Fill a rectangle `[x, x+w) × [y, y+h)` with `cell` (clipped to bounds).
    pub fn fill_rect(&mut self, x: i32, y: i32, w: u32, h: u32, cell: Cell) {
        for dy in 0..h as i32 {
            for dx in 0..w as i32 {
                self.put(x + dx, y + dy, cell);
            }
        }
    }

    /// Draw a string left-to-right starting at `(x, y)`, one column per `char`.
    /// Clipped at the screen edge; does not wrap.
    pub fn draw_str(&mut self, x: i32, y: i32, text: &str, fg: Color, bg: Color) {
        for (i, glyph) in text.chars().enumerate() {
            self.set(x + i as i32, y, glyph, fg, bg);
        }
    }

    /// Cells changed since the last [`Screen::present`], as `(x, y, cell)` in
    /// row-major order. This is the headless equivalent of "what would be
    /// redrawn" — ideal for snapshot tests and minimal-redraw output.
    pub fn diff(&self) -> Vec<(u32, u32, Cell)> {
        let mut changes = Vec::new();
        for (i, (cur, old)) in self.cells.iter().zip(self.prev.iter()).enumerate() {
            if cur != old {
                let i = i as u32;
                changes.push((i % self.width, i / self.width, *cur));
            }
        }
        changes
    }

    /// Commit the current frame: the back buffer becomes the current cells, so
    /// the next [`Screen::diff`] is relative to now.
    pub fn present(&mut self) {
        self.prev.copy_from_slice(&self.cells);
    }

    /// Serialise the whole frame to a 24-bit-ANSI string: cursor home, then each
    /// row with truecolor SGR sequences, ending with a reset. Deterministic
    /// (SGR is re-emitted only when a cell's colours differ from the previous
    /// one in the row, a fixed rule). Writing it to a terminal is the caller's
    /// responsibility.
    pub fn to_ansi(&self) -> String {
        let mut out = String::from("\x1b[H");
        for y in 0..self.height {
            let mut last: Option<(Color, Color)> = None;
            for x in 0..self.width {
                let cell = &self.cells[(y * self.width + x) as usize];
                if last != Some((cell.fg, cell.bg)) {
                    out.push_str(&format!(
                        "\x1b[38;2;{};{};{};48;2;{};{};{}m",
                        cell.fg.r, cell.fg.g, cell.fg.b, cell.bg.r, cell.bg.g, cell.bg.b
                    ));
                    last = Some((cell.fg, cell.bg));
                }
                out.push(cell.glyph);
            }
            out.push_str("\x1b[0m");
            if y + 1 < self.height {
                out.push_str("\r\n");
            }
        }
        out
    }
}

impl DetHash for Screen {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.width);
        hasher.write_u32(self.height);
        for cell in &self.cells {
            cell.det_hash(hasher);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    const RED: Color = Color { r: 255, g: 0, b: 0 };

    #[test]
    fn test_new_screen_is_blank() {
        let s = Screen::new(4, 3);
        assert_eq!(s.get(0, 0), Some(&Cell::blank()));
        assert_eq!(s.get(3, 2), Some(&Cell::blank()));
    }

    #[test]
    fn test_set_get_and_out_of_bounds() {
        let mut s = Screen::new(4, 3);
        s.set(1, 1, '@', RED, DEFAULT_BG);
        assert_eq!(s.get(1, 1).unwrap().glyph, '@');
        // Out of bounds: get is None, set is a no-op (no panic).
        assert_eq!(s.get(-1, 0), None);
        assert_eq!(s.get(4, 0), None);
        s.set(99, 99, 'X', RED, DEFAULT_BG); // must not panic
    }

    #[test]
    fn test_draw_str_clips_at_edge() {
        let mut s = Screen::new(5, 1);
        s.draw_str(3, 0, "hello", RED, DEFAULT_BG);
        assert_eq!(s.get(3, 0).unwrap().glyph, 'h');
        assert_eq!(s.get(4, 0).unwrap().glyph, 'e');
        // 'l','l','o' fall off the right edge and are dropped (no wrap/panic).
        assert_eq!(s.get(0, 0).unwrap().glyph, ' ');
    }

    #[test]
    fn test_fill_rect() {
        let mut s = Screen::new(6, 6);
        let wall = Cell {
            glyph: '#',
            fg: DEFAULT_FG,
            bg: DEFAULT_BG,
        };
        s.fill_rect(1, 1, 2, 2, wall);
        assert_eq!(s.get(1, 1).unwrap().glyph, '#');
        assert_eq!(s.get(2, 2).unwrap().glyph, '#');
        assert_eq!(s.get(3, 3).unwrap().glyph, ' '); // outside the rect
    }

    #[test]
    fn test_diff_tracks_changes_until_present() {
        let mut s = Screen::new(3, 3);
        s.present(); // baseline: blank
        assert!(s.diff().is_empty(), "no changes right after present");

        s.set(1, 1, '@', RED, DEFAULT_BG);
        let d = s.diff();
        assert_eq!(d.len(), 1);
        assert_eq!(d[0], (1, 1, *s.get(1, 1).unwrap()));

        s.present();
        assert!(s.diff().is_empty(), "present clears the diff");
    }

    #[test]
    fn test_to_ansi_is_deterministic_and_contains_glyph_and_truecolor() {
        let mut a = Screen::new(3, 1);
        a.set(0, 0, '@', RED, DEFAULT_BG);
        let mut b = Screen::new(3, 1);
        b.set(0, 0, '@', RED, DEFAULT_BG);
        let sa = a.to_ansi();
        assert_eq!(sa, b.to_ansi(), "identical screens render identically");
        assert!(sa.contains('@'));
        assert!(sa.contains("38;2;255;0;0"), "truecolor fg SGR for red");
        assert!(sa.starts_with("\x1b[H"));
    }

    #[test]
    fn test_det_hash_reflects_content() {
        let mut a = Screen::new(4, 4);
        let b = Screen::new(4, 4);
        assert_eq!(hash_state(&a), hash_state(&b), "blank screens hash equal");
        a.set(2, 2, '@', RED, DEFAULT_BG);
        assert_ne!(
            hash_state(&a),
            hash_state(&b),
            "a change must alter the hash"
        );
    }
}
