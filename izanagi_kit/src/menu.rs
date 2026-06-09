//! Keyboard-navigable menu widget for roguelike UI.
//!
//! `Menu<T>` holds an ordered list of labelled items (each carrying a payload
//! `T`) and tracks a cursor position. The caller drives navigation with
//! `move_up`/`move_down`; `select` returns the payload of the current item.
//! A `disabled` flag per item lets the menu skip over greyed-out entries
//! automatically so callers never land on a non-selectable item.
//!
//! The widget is purely data — no I/O, no rendering. Pair it with
//! `terminal::Screen` to draw the visible portion, and `KeyMap` / `CmdQueue`
//! to translate key events into navigation commands.
//!
//! `DetHash` (gated on `T: DetHash`) folds the item labels + disabled flags +
//! cursor position in order, making the menu state part of the world hash.

use crate::world_hash::{DetHash, Fnv1a};

/// A single entry in a `Menu`.
#[derive(Clone, Debug)]
pub struct MenuItem<T> {
    /// Display text shown to the player.
    pub label: String,
    /// Payload returned when this item is selected.
    pub value: T,
    /// When `true` the item is visible but cannot be selected; navigation
    /// skips over it automatically.
    pub disabled: bool,
}

/// A keyboard-navigable list menu.
///
/// `cursor` is always a valid index into `items`, unless `items` is empty
/// (in which case `cursor == 0` as a sentinel).
#[derive(Clone, Debug)]
pub struct Menu<T> {
    items: Vec<MenuItem<T>>,
    cursor: usize,
}

impl<T: Clone> Menu<T> {
    /// Create an empty menu. `add_item` populates it.
    pub fn new() -> Self {
        Menu {
            items: Vec::new(),
            cursor: 0,
        }
    }

    /// Append an enabled item.
    pub fn add_item(&mut self, label: impl Into<String>, value: T) {
        self.items.push(MenuItem {
            label: label.into(),
            value,
            disabled: false,
        });
    }

    /// Append a disabled (non-selectable) item.
    pub fn add_disabled(&mut self, label: impl Into<String>, value: T) {
        self.items.push(MenuItem {
            label: label.into(),
            value,
            disabled: true,
        });
    }

    /// Number of items (including disabled ones).
    #[inline]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Current cursor index.
    #[inline]
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// The item currently under the cursor, or `None` if the menu is empty.
    pub fn current(&self) -> Option<&MenuItem<T>> {
        self.items.get(self.cursor)
    }

    /// Move cursor up (toward index 0), skipping disabled items.
    /// Wraps around from the first item to the last.
    /// Has no effect if there are no enabled items.
    pub fn move_up(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let start = self.cursor;
        let mut next = if self.cursor == 0 {
            self.items.len() - 1
        } else {
            self.cursor - 1
        };
        loop {
            if !self.items[next].disabled {
                self.cursor = next;
                return;
            }
            if next == start {
                return; // all items disabled
            }
            next = if next == 0 {
                self.items.len() - 1
            } else {
                next - 1
            };
        }
    }

    /// Move cursor down (toward the last index), skipping disabled items.
    /// Wraps around from the last item to the first.
    /// Has no effect if there are no enabled items.
    pub fn move_down(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let start = self.cursor;
        let mut next = (self.cursor + 1) % self.items.len();
        loop {
            if !self.items[next].disabled {
                self.cursor = next;
                return;
            }
            if next == start {
                return; // all items disabled
            }
            next = (next + 1) % self.items.len();
        }
    }

    /// Jump directly to index `idx` (clamped to valid range).
    /// Disabled items can be targeted explicitly via `set_cursor` — callers
    /// that want to avoid disabled items should check `current().disabled`.
    pub fn set_cursor(&mut self, idx: usize) {
        if !self.items.is_empty() {
            self.cursor = idx.min(self.items.len() - 1);
        }
    }

    /// Return a clone of the current item's payload if the item is enabled.
    /// Returns `None` if the menu is empty or the current item is disabled.
    pub fn select(&self) -> Option<T> {
        let item = self.items.get(self.cursor)?;
        if item.disabled {
            None
        } else {
            Some(item.value.clone())
        }
    }

    /// Iterate `(index, &MenuItem)` for all items in order.
    pub fn iter(&self) -> impl Iterator<Item = (usize, &MenuItem<T>)> {
        self.items.iter().enumerate()
    }

    /// Enable or disable item at `idx`. Silently ignores out-of-range indices.
    ///
    /// When the cursor lands on a now-disabled item (because the game just
    /// greyed it out), the caller should drive the cursor away with `move_down`
    /// or `move_up` to keep the invariant that the cursor is on an enabled item.
    pub fn set_enabled(&mut self, idx: usize, enabled: bool) {
        if let Some(item) = self.items.get_mut(idx) {
            item.disabled = !enabled;
        }
    }

    /// Return the index of the first item whose label equals `label`, or
    /// `None` if no match is found. Case-sensitive.
    pub fn find_by_label(&self, label: &str) -> Option<usize> {
        self.items.iter().position(|it| it.label == label)
    }

    /// Remove all items and reset cursor to 0.
    pub fn clear(&mut self) {
        self.items.clear();
        self.cursor = 0;
    }

    /// Move the cursor to the first item with a matching label and return its
    /// value (if enabled). Returns `None` if no matching label exists, or if
    /// the matching item is disabled. Case-sensitive.
    pub fn select_by_label(&mut self, label: &str) -> Option<T> {
        let idx = self.find_by_label(label)?;
        self.set_cursor(idx);
        self.select()
    }

    /// Return the cursor index of the next enabled item after the current
    /// cursor, wrapping around. Returns `None` if there are no enabled items.
    /// Does *not* move the cursor — use `set_cursor` to act on the result.
    pub fn next_enabled(&self) -> Option<usize> {
        if self.items.is_empty() {
            return None;
        }
        let mut next = (self.cursor + 1) % self.items.len();
        for _ in 0..self.items.len() {
            if !self.items[next].disabled {
                return Some(next);
            }
            next = (next + 1) % self.items.len();
        }
        None
    }
}

impl<T: Clone> Default for Menu<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + DetHash> DetHash for Menu<T> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.items.len() as u32);
        for item in &self.items {
            for b in item.label.as_bytes() {
                hasher.write_u32(*b as u32);
            }
            item.value.det_hash(hasher);
            hasher.write_u32(item.disabled as u32);
        }
        hasher.write_u32(self.cursor as u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    fn sample() -> Menu<u32> {
        let mut m = Menu::new();
        m.add_item("Item A", 1);
        m.add_item("Item B", 2);
        m.add_item("Item C", 3);
        m
    }

    #[test]
    fn test_new_is_empty() {
        let m: Menu<u32> = Menu::new();
        assert!(m.is_empty());
        assert_eq!(m.len(), 0);
        assert_eq!(m.cursor(), 0);
        assert!(m.current().is_none());
    }

    #[test]
    fn test_add_item_len() {
        let m = sample();
        assert_eq!(m.len(), 3);
        assert!(!m.is_empty());
    }

    #[test]
    fn test_cursor_starts_at_zero() {
        let m = sample();
        assert_eq!(m.cursor(), 0);
        assert_eq!(m.current().unwrap().label, "Item A");
    }

    #[test]
    fn test_move_down_advances_cursor() {
        let mut m = sample();
        m.move_down();
        assert_eq!(m.cursor(), 1);
        m.move_down();
        assert_eq!(m.cursor(), 2);
    }

    #[test]
    fn test_move_down_wraps() {
        let mut m = sample();
        m.set_cursor(2);
        m.move_down();
        assert_eq!(m.cursor(), 0);
    }

    #[test]
    fn test_move_up_goes_backward() {
        let mut m = sample();
        m.set_cursor(2);
        m.move_up();
        assert_eq!(m.cursor(), 1);
    }

    #[test]
    fn test_move_up_wraps() {
        let mut m = sample();
        assert_eq!(m.cursor(), 0);
        m.move_up();
        assert_eq!(m.cursor(), 2);
    }

    #[test]
    fn test_select_returns_value() {
        let m = sample();
        assert_eq!(m.select(), Some(1u32));
    }

    #[test]
    fn test_set_cursor_direct() {
        let mut m = sample();
        m.set_cursor(2);
        assert_eq!(m.select(), Some(3u32));
    }

    #[test]
    fn test_set_cursor_clamps() {
        let mut m = sample();
        m.set_cursor(999);
        assert_eq!(m.cursor(), 2);
    }

    #[test]
    fn test_disabled_item_skipped_on_move_down() {
        let mut m: Menu<u32> = Menu::new();
        m.add_item("A", 1);
        m.add_disabled("B", 2); // index 1 disabled
        m.add_item("C", 3);
        m.move_down(); // should skip 1, land on 2
        assert_eq!(m.cursor(), 2);
    }

    #[test]
    fn test_disabled_item_skipped_on_move_up() {
        let mut m: Menu<u32> = Menu::new();
        m.add_item("A", 1);
        m.add_disabled("B", 2); // index 1 disabled
        m.add_item("C", 3);
        m.set_cursor(2);
        m.move_up(); // should skip 1, land on 0
        assert_eq!(m.cursor(), 0);
    }

    #[test]
    fn test_select_disabled_returns_none() {
        let mut m: Menu<u32> = Menu::new();
        m.add_disabled("X", 99);
        assert_eq!(m.select(), None);
    }

    #[test]
    fn test_all_disabled_move_is_noop() {
        let mut m: Menu<u32> = Menu::new();
        m.add_disabled("A", 1);
        m.add_disabled("B", 2);
        m.set_cursor(0);
        m.move_down();
        assert_eq!(m.cursor(), 0); // still at 0
    }

    #[test]
    fn test_clear_resets() {
        let mut m = sample();
        m.set_cursor(2);
        m.clear();
        assert!(m.is_empty());
        assert_eq!(m.cursor(), 0);
    }

    #[test]
    fn test_iter_yields_all_items_in_order() {
        let m = sample();
        let v: Vec<(usize, &str)> = m.iter().map(|(i, it)| (i, it.label.as_str())).collect();
        assert_eq!(v, [(0, "Item A"), (1, "Item B"), (2, "Item C")]);
    }

    #[test]
    fn test_set_enabled_toggles_disabled_state() {
        let mut m: Menu<u32> = Menu::new();
        m.add_item("A", 1);
        m.add_disabled("B", 2);
        // Enable the disabled item.
        m.set_enabled(1, true);
        assert!(!m.items[1].disabled);
        // Disable an enabled item.
        m.set_enabled(0, false);
        assert!(m.items[0].disabled);
    }

    #[test]
    fn test_set_enabled_out_of_range_is_noop() {
        let mut m: Menu<u32> = Menu::new();
        m.add_item("A", 1);
        m.set_enabled(99, false); // should not panic
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn test_set_enabled_gates_selection() {
        let mut m: Menu<u32> = Menu::new();
        m.add_item("Buy", 10);
        // Disable the only item — select should now return None.
        m.set_enabled(0, false);
        assert_eq!(m.select(), None);
        // Re-enable — select should return Some again.
        m.set_enabled(0, true);
        assert_eq!(m.select(), Some(10));
    }

    #[test]
    fn test_find_by_label_found() {
        let m = sample();
        assert_eq!(m.find_by_label("Item B"), Some(1));
    }

    #[test]
    fn test_find_by_label_not_found() {
        let m = sample();
        assert_eq!(m.find_by_label("Item Z"), None);
    }

    #[test]
    fn test_find_by_label_case_sensitive() {
        let m = sample();
        assert_eq!(m.find_by_label("item a"), None); // lowercase — no match
        assert_eq!(m.find_by_label("Item A"), Some(0));
    }

    #[test]
    fn test_det_hash_same_state_same_hash() {
        let m1 = sample();
        let m2 = sample();
        assert_eq!(hash_state(&m1), hash_state(&m2));
    }

    #[test]
    fn test_det_hash_differs_on_cursor() {
        let m1 = sample();
        let mut m2 = sample();
        m2.move_down();
        assert_ne!(hash_state(&m1), hash_state(&m2));
    }

    #[test]
    fn test_select_by_label_moves_cursor_and_returns_value() {
        let mut m = sample();
        assert_eq!(m.select_by_label("Item C"), Some(3u32));
        assert_eq!(m.cursor(), 2);
    }

    #[test]
    fn test_select_by_label_not_found_returns_none() {
        let mut m = sample();
        assert_eq!(m.select_by_label("No Such Item"), None);
        assert_eq!(m.cursor(), 0); // cursor unchanged
    }

    #[test]
    fn test_select_by_label_disabled_returns_none() {
        let mut m: Menu<u32> = Menu::new();
        m.add_disabled("X", 99);
        assert_eq!(m.select_by_label("X"), None);
    }

    #[test]
    fn test_next_enabled_skips_disabled() {
        let mut m: Menu<u32> = Menu::new();
        m.add_item("A", 1);
        m.add_disabled("B", 2);
        m.add_item("C", 3);
        // cursor at 0 → next enabled after 0 is 2 (index 1 is disabled)
        assert_eq!(m.next_enabled(), Some(2));
    }

    #[test]
    fn test_next_enabled_wraps_around() {
        let m = sample();
        // cursor at 0, items are A(0), B(1), C(2) — all enabled; next = 1
        assert_eq!(m.next_enabled(), Some(1));
    }

    #[test]
    fn test_next_enabled_all_disabled_returns_none() {
        let mut m: Menu<u32> = Menu::new();
        m.add_disabled("A", 1);
        m.add_disabled("B", 2);
        assert_eq!(m.next_enabled(), None);
    }

    #[test]
    fn test_next_enabled_empty_menu_returns_none() {
        let m: Menu<u32> = Menu::new();
        assert_eq!(m.next_enabled(), None);
    }
}
