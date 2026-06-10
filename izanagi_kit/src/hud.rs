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

    /// Count of unfilled (empty) cells — the complement of `filled_cells()`.
    /// Always in `[0, width]`. Useful for "remaining capacity" queries such as
    /// "how much stamina can I restore?" without subtracting from `width`.
    #[inline]
    pub fn empty_cells(&self) -> u32 {
        self.width - self.filled_cells()
    }

    /// Returns `true` when the bar is at maximum capacity (`current >= max`).
    /// Also returns `true` when `max <= 0` (degenerate bar).
    #[inline]
    pub fn is_full(&self) -> bool {
        self.max <= 0 || self.current >= self.max
    }

    /// Whether the bar is empty (`current <= 0`). The complement of `is_full`;
    /// useful for "out of fuel / mana / HP" checks and for graying out UI elements
    /// when a resource is completely depleted.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.current <= 0
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

    /// Shrink this panel by explicit per-side margins (in cells), returning a
    /// new panel. Width and height are clamped to 0 if the margins exceed the
    /// panel's outer size.
    pub fn pad(&self, left: u32, top: u32, right: u32, bottom: u32) -> HudPanel {
        let pad_w = left.saturating_add(right);
        let pad_h = top.saturating_add(bottom);
        HudPanel {
            x: self.x.saturating_add(left as i32),
            y: self.y.saturating_add(top as i32),
            w: self.w.saturating_sub(pad_w),
            h: self.h.saturating_sub(pad_h),
        }
    }

    /// Compute the smallest `HudPanel` enclosing all panels in `panels`.
    /// Returns `None` for an empty slice. Useful for "group selection" bounding
    /// boxes and computing a containing region for composite HUD layouts.
    pub fn merge(panels: &[HudPanel]) -> Option<HudPanel> {
        let mut iter = panels.iter();
        let first = iter.next()?;
        let mut x0 = first.x;
        let mut y0 = first.y;
        let mut x1 = first.x.saturating_add(first.w as i32);
        let mut y1 = first.y.saturating_add(first.h as i32);
        for p in iter {
            x0 = x0.min(p.x);
            y0 = y0.min(p.y);
            x1 = x1.max(p.x.saturating_add(p.w as i32));
            y1 = y1.max(p.y.saturating_add(p.h as i32));
        }
        let w = (x1 - x0).max(0) as u32;
        let h = (y1 - y0).max(0) as u32;
        Some(HudPanel { x: x0, y: y0, w, h })
    }

    /// Returns `true` when the panel is wider than it is tall (`w >= h`).
    /// Useful for adaptive layouts: split wide panels horizontally, tall panels
    /// vertically, without hard-coding orientation at each call site.
    #[inline]
    pub fn is_landscape(&self) -> bool {
        self.w >= self.h
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

    // --- HudPanel::merge ---

    #[test]
    fn test_merge_empty_returns_none() {
        assert!(HudPanel::merge(&[]).is_none());
    }

    #[test]
    fn test_merge_single_is_identity() {
        let p = HudPanel::new(2, 3, 5, 4);
        assert_eq!(HudPanel::merge(&[p]), Some(p));
    }

    #[test]
    fn test_merge_two_adjacent_panels() {
        let a = HudPanel::new(0, 0, 5, 4);
        let b = HudPanel::new(5, 0, 5, 4);
        let m = HudPanel::merge(&[a, b]).unwrap();
        assert_eq!(m.x, 0);
        assert_eq!(m.y, 0);
        assert_eq!(m.w, 10);
        assert_eq!(m.h, 4);
    }

    #[test]
    fn test_merge_offset_panels() {
        // a: x=[1,4), y=[2,5); b: x=[5,7), y=[1,6)
        // bounding: x0=1, y0=1, x1=7, y1=6 → w=6, h=5
        let a = HudPanel::new(1, 2, 3, 3);
        let b = HudPanel::new(5, 1, 2, 5);
        let m = HudPanel::merge(&[a, b]).unwrap();
        assert_eq!(m.x, 1);
        assert_eq!(m.y, 1);
        assert_eq!(m.w, 6);
        assert_eq!(m.h, 5);
    }

    // --- HudPanel::pad ---

    #[test]
    fn test_pad_uniform_shrinks_correctly() {
        let p = HudPanel::new(0, 0, 10, 8);
        let inner = p.pad(1, 1, 1, 1);
        assert_eq!(inner.x, 1);
        assert_eq!(inner.y, 1);
        assert_eq!(inner.w, 8);
        assert_eq!(inner.h, 6);
    }

    #[test]
    fn test_pad_asymmetric_margins() {
        let p = HudPanel::new(2, 3, 20, 12);
        let inner = p.pad(2, 1, 3, 4);
        assert_eq!(inner.x, 4); // 2 + 2
        assert_eq!(inner.y, 4); // 3 + 1
        assert_eq!(inner.w, 15); // 20 - 2 - 3
        assert_eq!(inner.h, 7); // 12 - 1 - 4
    }

    #[test]
    fn test_pad_exceeds_size_clamps_to_zero() {
        let p = HudPanel::new(0, 0, 4, 3);
        let inner = p.pad(3, 2, 3, 2); // pad_w=6 > 4, pad_h=4 > 3
        assert_eq!(inner.w, 0);
        assert_eq!(inner.h, 0);
    }

    #[test]
    fn test_empty_cells_full_bar_is_zero() {
        let b = BarWidget::new(10, 10, 8);
        assert_eq!(b.empty_cells(), 0);
    }

    #[test]
    fn test_empty_cells_empty_bar_is_width() {
        let b = BarWidget::new(0, 10, 8);
        assert_eq!(b.empty_cells(), 8);
    }

    #[test]
    fn test_empty_cells_plus_filled_cells_equals_width() {
        let b = BarWidget::new(3, 10, 8);
        assert_eq!(b.filled_cells() + b.empty_cells(), b.width);
    }

    #[test]
    fn test_is_full_when_current_equals_max() {
        let b = BarWidget::new(10, 10, 8);
        assert!(b.is_full());
    }

    #[test]
    fn test_is_full_when_current_exceeds_max() {
        let b = BarWidget::new(15, 10, 8);
        assert!(b.is_full());
    }

    #[test]
    fn test_is_full_false_when_partial() {
        let b = BarWidget::new(5, 10, 8);
        assert!(!b.is_full());
    }

    #[test]
    fn test_is_landscape_wide_panel() {
        let p = HudPanel::new(0, 0, 10, 5);
        assert!(p.is_landscape());
    }

    #[test]
    fn test_is_landscape_square_panel() {
        let p = HudPanel::new(0, 0, 4, 4);
        assert!(p.is_landscape());
    }

    #[test]
    fn test_is_landscape_tall_panel() {
        let p = HudPanel::new(0, 0, 3, 8);
        assert!(!p.is_landscape());
    }

    #[test]
    fn test_bar_is_empty_when_current_zero() {
        let b = BarWidget::new(0, 10, 10);
        assert!(b.is_empty());
    }

    #[test]
    fn test_bar_is_empty_false_when_current_positive() {
        let b = BarWidget::new(5, 10, 10);
        assert!(!b.is_empty());
    }

    #[test]
    fn test_bar_is_empty_true_when_current_negative() {
        let b = BarWidget::new(-3, 10, 10);
        assert!(b.is_empty());
    }
}
