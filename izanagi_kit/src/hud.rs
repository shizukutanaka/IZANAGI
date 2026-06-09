//! Heads-up display (HUD) primitives for terminal roguelike UI.
//!
//! Provides data-only HUD elements. The caller is responsible for rendering
//! them to a `terminal::Screen`; these types only compute what to draw.
//!
//! ## Provided types
//!
//! - `BarWidget` — a fill-bar (HP, mana, XP) rendered as `[====    ]`.
//!   Takes a `(current, max)` pair and a width; outputs a `String` of exactly
//!   `width + 2` chars (brackets included).
//! - `StatLine` — a single `key: value` line formatter with optional units.
//! - `HudPanel` — a simple bounding-box position (`x, y, w, h`) for placing
//!   HUD regions on a screen without hard-coding coordinates everywhere.
//!   Provides `inner_*` helpers for content regions after margin.
//!
//! All types implement `DetHash` so they can participate in snapshot tests.

use crate::world_hash::{DetHash, Fnv1a};

// ---------------------------------------------------------------------------
// BarWidget
// ---------------------------------------------------------------------------

/// A fill-bar widget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BarWidget {
    pub current: i32,
    pub max: i32,
    /// Inner width of the bar (not counting brackets). Minimum 1.
    pub width: u32,
    /// Character used for the filled portion.
    pub fill_char: char,
    /// Character used for the empty portion.
    pub empty_char: char,
}

impl BarWidget {
    pub fn new(current: i32, max: i32, width: u32) -> Self {
        BarWidget {
            current,
            max,
            width: width.max(1),
            fill_char: '=',
            empty_char: ' ',
        }
    }

    /// Fill percentage as an integer in `[0, 100]`. Useful for showing "85% HP"
    /// in a status line without floats. Result is clamped: negative `current`
    /// returns `0`; `current > max` returns `100`; `max <= 0` returns `0`.
    pub fn percentage(&self) -> i32 {
        if self.max <= 0 {
            return 0;
        }
        (self.current.max(0) as i64 * 100 / self.max as i64).clamp(0, 100) as i32
    }

    /// Compute the number of filled cells (integer, `current/max * width`).
    /// Clamped to `[0, width]`.
    pub fn filled_cells(&self) -> u32 {
        if self.max <= 0 {
            return 0;
        }
        let ratio_num = self.current.max(0) as u64 * self.width as u64;
        let cells = ratio_num / self.max.max(1) as u64;
        (cells as u32).min(self.width)
    }

    /// Render to a `String` of the form `[====    ]` (width + 2 chars).
    pub fn render(&self) -> String {
        let filled = self.filled_cells() as usize;
        let empty = self.width as usize - filled;
        let mut s = String::with_capacity(self.width as usize + 2);
        s.push('[');
        for _ in 0..filled {
            s.push(self.fill_char);
        }
        for _ in 0..empty {
            s.push(self.empty_char);
        }
        s.push(']');
        s
    }
}

impl DetHash for BarWidget {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_i32(self.current);
        hasher.write_i32(self.max);
        hasher.write_u32(self.width);
    }
}

// ---------------------------------------------------------------------------
// StatLine
// ---------------------------------------------------------------------------

/// A single "key: value [unit]" line for HUD readouts.
#[derive(Clone, Debug)]
pub struct StatLine {
    pub label: &'static str,
    pub value: i32,
    pub unit: Option<&'static str>,
}

impl StatLine {
    pub fn new(label: &'static str, value: i32) -> Self {
        StatLine {
            label,
            value,
            unit: None,
        }
    }

    pub fn with_unit(label: &'static str, value: i32, unit: &'static str) -> Self {
        StatLine {
            label,
            value,
            unit: Some(unit),
        }
    }

    /// Format as `"label: value"` or `"label: value unit"`.
    pub fn render(&self) -> String {
        match self.unit {
            None => format!("{}: {}", self.label, self.value),
            Some(u) => format!("{}: {} {}", self.label, self.value, u),
        }
    }
}

impl DetHash for StatLine {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        for b in self.label.as_bytes() {
            hasher.write_u32(*b as u32);
        }
        hasher.write_i32(self.value);
    }
}

// ---------------------------------------------------------------------------
// HudPanel
// ---------------------------------------------------------------------------

/// A positioned bounding box for a HUD region.
///
/// `(x, y)` is the top-left corner; `(w, h)` is the outer size.
/// Use `inner_*` to get content coordinates after a 1-cell border margin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HudPanel {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl HudPanel {
    pub fn new(x: i32, y: i32, w: u32, h: u32) -> Self {
        HudPanel { x, y, w, h }
    }

    /// Left content edge (x + 1).
    #[inline]
    pub fn inner_x(&self) -> i32 {
        self.x + 1
    }

    /// Top content edge (y + 1).
    #[inline]
    pub fn inner_y(&self) -> i32 {
        self.y + 1
    }

    /// Content width (outer w − 2, clamped to 0).
    #[inline]
    pub fn inner_w(&self) -> u32 {
        self.w.saturating_sub(2)
    }

    /// Content height (outer h − 2, clamped to 0).
    #[inline]
    pub fn inner_h(&self) -> u32 {
        self.h.saturating_sub(2)
    }

    /// True if world point `(px, py)` lies within this panel.
    #[inline]
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.w as i32 && py >= self.y && py < self.y + self.h as i32
    }

    /// Translate by `(dx, dy)`, returning a new panel.
    #[inline]
    pub fn translate(&self, dx: i32, dy: i32) -> HudPanel {
        HudPanel {
            x: self.x.saturating_add(dx),
            y: self.y.saturating_add(dy),
            w: self.w,
            h: self.h,
        }
    }

    /// Divide this panel into `n` horizontal strips (top to bottom), each with
    /// the same width. Returns an empty `Vec` when `n == 0`. Heights are
    /// distributed by integer division; any remainder rows go to the first strip.
    pub fn split_h(&self, n: u32) -> Vec<HudPanel> {
        if n == 0 {
            return Vec::new();
        }
        let base_h = self.h / n;
        let extra = self.h % n;
        let mut panels = Vec::with_capacity(n as usize);
        let mut y = self.y;
        for i in 0..n {
            let h = base_h + if i == 0 { extra } else { 0 };
            panels.push(HudPanel {
                x: self.x,
                y,
                w: self.w,
                h,
            });
            y = y.saturating_add(h as i32);
        }
        panels
    }

    /// Divide this panel into `n` vertical strips (left to right), each with
    /// the same height. Returns an empty `Vec` when `n == 0`. Widths are
    /// distributed by integer division; any remainder columns go to the first strip.
    pub fn split_v(&self, n: u32) -> Vec<HudPanel> {
        if n == 0 {
            return Vec::new();
        }
        let base_w = self.w / n;
        let extra = self.w % n;
        let mut panels = Vec::with_capacity(n as usize);
        let mut x = self.x;
        for i in 0..n {
            let w = base_w + if i == 0 { extra } else { 0 };
            panels.push(HudPanel {
                x,
                y: self.y,
                w,
                h: self.h,
            });
            x = x.saturating_add(w as i32);
        }
        panels
    }
}

impl DetHash for HudPanel {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_i32(self.x);
        hasher.write_i32(self.y);
        hasher.write_u32(self.w);
        hasher.write_u32(self.h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    // --- BarWidget ---

    #[test]
    fn test_percentage_full() {
        assert_eq!(BarWidget::new(10, 10, 5).percentage(), 100);
    }

    #[test]
    fn test_percentage_empty() {
        assert_eq!(BarWidget::new(0, 10, 5).percentage(), 0);
    }

    #[test]
    fn test_percentage_half() {
        assert_eq!(BarWidget::new(5, 10, 5).percentage(), 50);
    }

    #[test]
    fn test_percentage_clamped_above() {
        assert_eq!(BarWidget::new(20, 10, 5).percentage(), 100);
    }

    #[test]
    fn test_percentage_negative_current() {
        assert_eq!(BarWidget::new(-5, 10, 5).percentage(), 0);
    }

    #[test]
    fn test_percentage_zero_max() {
        assert_eq!(BarWidget::new(5, 0, 5).percentage(), 0);
    }

    #[test]
    fn test_bar_full() {
        let b = BarWidget::new(10, 10, 5);
        assert_eq!(b.render(), "[=====]");
    }

    #[test]
    fn test_bar_empty() {
        let b = BarWidget::new(0, 10, 5);
        assert_eq!(b.render(), "[     ]");
    }

    #[test]
    fn test_bar_half() {
        let b = BarWidget::new(5, 10, 10);
        assert_eq!(b.render(), "[=====     ]");
        assert_eq!(b.render().chars().count(), 12); // 10 + 2 brackets
    }

    #[test]
    fn test_bar_clamps_above_max() {
        let b = BarWidget::new(20, 10, 5);
        assert_eq!(b.filled_cells(), 5); // capped at width
    }

    #[test]
    fn test_bar_negative_current() {
        let b = BarWidget::new(-5, 10, 5);
        assert_eq!(b.filled_cells(), 0);
    }

    #[test]
    fn test_bar_zero_max() {
        let b = BarWidget::new(5, 0, 5);
        assert_eq!(b.filled_cells(), 0);
    }

    #[test]
    fn test_bar_det_hash_same() {
        let a = BarWidget::new(5, 10, 8);
        let b = BarWidget::new(5, 10, 8);
        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_bar_det_hash_differs_on_change() {
        let a = BarWidget::new(5, 10, 8);
        let b = BarWidget::new(6, 10, 8);
        assert_ne!(hash_state(&a), hash_state(&b));
    }

    // --- StatLine ---

    #[test]
    fn test_stat_line_no_unit() {
        let s = StatLine::new("HP", 42);
        assert_eq!(s.render(), "HP: 42");
    }

    #[test]
    fn test_stat_line_with_unit() {
        let s = StatLine::with_unit("Speed", 7, "m/s");
        assert_eq!(s.render(), "Speed: 7 m/s");
    }

    #[test]
    fn test_stat_line_det_hash() {
        let a = StatLine::new("ATK", 15);
        let b = StatLine::new("ATK", 15);
        assert_eq!(hash_state(&a), hash_state(&b));
    }

    // --- HudPanel ---

    #[test]
    fn test_inner_dimensions() {
        let p = HudPanel::new(0, 0, 10, 6);
        assert_eq!(p.inner_x(), 1);
        assert_eq!(p.inner_y(), 1);
        assert_eq!(p.inner_w(), 8);
        assert_eq!(p.inner_h(), 4);
    }

    #[test]
    fn test_inner_dimensions_clamp_to_zero() {
        let p = HudPanel::new(0, 0, 1, 1);
        assert_eq!(p.inner_w(), 0);
        assert_eq!(p.inner_h(), 0);
    }

    #[test]
    fn test_contains() {
        let p = HudPanel::new(5, 5, 4, 3);
        assert!(p.contains(5, 5));
        assert!(p.contains(8, 7));
        assert!(!p.contains(9, 5)); // x >= x+w
        assert!(!p.contains(4, 5)); // x < x
    }

    #[test]
    fn test_translate() {
        let p = HudPanel::new(0, 0, 4, 3);
        let t = p.translate(5, 2);
        assert_eq!(t.x, 5);
        assert_eq!(t.y, 2);
        assert_eq!(t.w, 4);
        assert_eq!(t.h, 3);
    }

    #[test]
    fn test_split_h_produces_n_strips() {
        let p = HudPanel::new(0, 0, 10, 9);
        let strips = p.split_h(3);
        assert_eq!(strips.len(), 3);
        // Heights must sum to parent height.
        let total_h: u32 = strips.iter().map(|s| s.h).sum();
        assert_eq!(total_h, p.h);
        // All strips share the parent's width and x.
        assert!(strips.iter().all(|s| s.w == p.w && s.x == p.x));
    }

    #[test]
    fn test_split_h_y_positions_tile() {
        let p = HudPanel::new(2, 3, 5, 6);
        let strips = p.split_h(2);
        assert_eq!(strips[0].y, 3);
        assert_eq!(strips[1].y, 3 + strips[0].h as i32);
    }

    #[test]
    fn test_split_h_zero_returns_empty() {
        assert!(HudPanel::new(0, 0, 10, 10).split_h(0).is_empty());
    }

    #[test]
    fn test_split_v_produces_n_strips() {
        let p = HudPanel::new(0, 0, 10, 4);
        let strips = p.split_v(5);
        assert_eq!(strips.len(), 5);
        let total_w: u32 = strips.iter().map(|s| s.w).sum();
        assert_eq!(total_w, p.w);
        assert!(strips.iter().all(|s| s.h == p.h && s.y == p.y));
    }

    #[test]
    fn test_split_v_zero_returns_empty() {
        assert!(HudPanel::new(0, 0, 10, 10).split_v(0).is_empty());
    }

    #[test]
    fn test_hud_panel_det_hash() {
        let a = HudPanel::new(0, 0, 10, 5);
        let b = HudPanel::new(0, 0, 10, 5);
        assert_eq!(hash_state(&a), hash_state(&b));
        let c = HudPanel::new(1, 0, 10, 5);
        assert_ne!(hash_state(&a), hash_state(&c));
    }
}
