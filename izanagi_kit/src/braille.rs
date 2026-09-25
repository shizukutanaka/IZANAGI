//! Unicode braille canvas — a 2×4-dot-per-cell raster giving terminals 2×
//! horizontal and 4× vertical sub-cell resolution for plots, maps, and game
//! boards (the technique behind `drawille`, used by UnicodePlots and many TUIs).
//!
//! Each cell packs eight dots as a `U+2800 + bits` codepoint: the left column
//! holds dots 1-3 and 7 (bits 0,1,2,6 top→bottom), the right column dots 4-6
//! and 8 (bits 3,4,5,7). [`Braille::set`]/[`Braille::get`]/[`Braille::clear`] address virtual
//! pixels `(x, y)` in a `2w × 4h` space; [`render`](Braille::render) emits the
//! text lines.
//!
//! ```
//! use izanagi_kit::braille::Braille;
//! let mut b = Braille::new(2, 1); // 4×4 virtual pixels
//! b.set(0, 0);
//! b.set(3, 3);
//! let lines = b.render();
//! assert_eq!(lines.len(), 1);
//! assert_eq!(lines[0].chars().count(), 2);
//! ```

/// A `w × h`-cell braille canvas (virtual pixels `2w × 4h`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Braille {
    w: usize,
    h: usize,
    cells: Vec<u8>,
}

/// Bit index of the braille dot at `(dx, dy)` inside a cell (0..2 × 0..4).
const fn dot(dx: usize, dy: usize) -> u8 {
    match (dx, dy) {
        (0, 0) => 0,
        (0, 1) => 1,
        (0, 2) => 2,
        (0, 3) => 6,
        (1, 0) => 3,
        (1, 1) => 4,
        (1, 2) => 5,
        _ => 7, // (1, 3)
    }
}

impl Braille {
    /// A blank canvas of `w` columns and `h` rows of cells.
    pub fn new(w: usize, h: usize) -> Self {
        Self {
            w,
            h,
            cells: vec![0u8; w.saturating_mul(h)],
        }
    }

    /// Virtual pixel width (`2 * columns`).
    pub fn width(&self) -> usize {
        self.w * 2
    }

    /// Virtual pixel height (`4 * rows`).
    pub fn height(&self) -> usize {
        self.h * 4
    }

    /// Set virtual pixel `(x, y)` on. Out of bounds is a no-op.
    pub fn set(&mut self, x: i32, y: i32) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.width() || y >= self.height() {
            return;
        }
        let i = (y / 4) * self.w + x / 2;
        self.cells[i] |= 1 << dot(x % 2, y % 4);
    }

    /// Clear virtual pixel `(x, y)`. Out of bounds is a no-op.
    pub fn clear_pixel(&mut self, x: i32, y: i32) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.width() || y >= self.height() {
            return;
        }
        let i = (y / 4) * self.w + x / 2;
        self.cells[i] &= !(1 << dot(x % 2, y % 4));
    }

    /// Is virtual pixel `(x, y)` on? Out of bounds is `false`.
    pub fn get(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 {
            return false;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.width() || y >= self.height() {
            return false;
        }
        self.cells[(y / 4) * self.w + x / 2] & (1 << dot(x % 2, y % 4)) != 0
    }

    /// Clear the whole canvas.
    pub fn clear(&mut self) {
        self.cells.fill(0);
    }

    /// Render one text line per cell row (blank cells are `U+2800`).
    pub fn render(&self) -> Vec<String> {
        let mut rows = Vec::with_capacity(self.h);
        for r in 0..self.h {
            let mut s = String::with_capacity(self.w * 3);
            for c in 0..self.w {
                let bits = self.cells[r * self.w + c];
                s.push(char::from_u32(0x2800 + bits as u32).unwrap_or('\u{2800}'));
            }
            rows.push(s);
        }
        rows
    }

    /// Render the whole canvas as one `'\n'`-joined string.
    pub fn render_string(&self) -> String {
        self.render().join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_and_bounds() {
        let mut b = Braille::new(2, 1);
        assert_eq!(b.width(), 4);
        assert_eq!(b.height(), 4);
        b.set(0, 0);
        b.set(1, 1);
        b.set(3, 3);
        assert!(b.get(0, 0));
        assert!(b.get(1, 1));
        assert!(b.get(3, 3));
        assert!(!b.get(0, 1));
        // Out of bounds: no panic, no-op / false.
        b.set(-1, 0);
        b.set(4, 0);
        b.set(0, 4);
        b.clear_pixel(-5, -5);
        assert!(!b.get(9, 9));
    }

    #[test]
    fn dot_mapping() {
        // Each of the 8 dots maps to its own bit; cell 0 shows U+2800+bit.
        let mut b = Braille::new(1, 1);
        for dy in 0..4 {
            for dx in 0..2 {
                b.clear();
                b.set(dx, dy);
                let s = b.render();
                let ch = s[0].chars().next().unwrap();
                assert_eq!(
                    ch as u32,
                    0x2800 + (1 << dot(dx as usize, dy as usize)) as u32,
                    "({dx},{dy})"
                );
            }
        }
    }

    #[test]
    fn full_cell_is_u28ff() {
        let mut b = Braille::new(1, 1);
        for y in 0..4 {
            for x in 0..2 {
                b.set(x, y);
            }
        }
        assert_eq!(b.render()[0], "\u{28ff}");
    }

    #[test]
    fn render_shape() {
        let mut b = Braille::new(3, 2);
        b.set(0, 0);
        b.set(5, 7);
        let lines = b.render();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].chars().count(), 3);
        assert_eq!(lines[1].chars().count(), 3);
        // Determinism.
        assert_eq!(b.render_string(), b.render_string());
    }

    #[test]
    fn clear_pixel_and_clear() {
        let mut b = Braille::new(1, 1);
        b.set(1, 2);
        b.clear_pixel(1, 2);
        assert!(!b.get(1, 2));
        b.set(0, 0);
        b.clear();
        assert!(!b.get(0, 0));
    }
}
