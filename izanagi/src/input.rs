//! Input state.
//!
//! Keyboard keys, mouse buttons, mouse position. Queried, not subscribed.

use std::collections::HashSet;

/// A keyboard key. Subset covering what games actually use.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Space,
    Enter,
    Escape,
    Tab,
    W,
    A,
    S,
    D,
    Q,
    E,
    R,
    F,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
}

/// Input state for the current frame.
pub struct Input {
    down: HashSet<Key>,
    pressed: HashSet<Key>,
    released: HashSet<Key>,
    mouse_x: f32,
    mouse_y: f32,
    mouse_down: bool,
    mouse_clicked: bool,
}

impl Input {
    /// Create an empty input state.
    pub fn new() -> Self {
        Self {
            down: HashSet::new(),
            pressed: HashSet::new(),
            released: HashSet::new(),
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_down: false,
            mouse_clicked: false,
        }
    }

    /// Is this key currently held?
    pub fn down(&self, key: Key) -> bool {
        self.down.contains(&key)
    }

    /// Was this key pressed this frame (edge)?
    pub fn pressed(&self, key: Key) -> bool {
        self.pressed.contains(&key)
    }

    /// Was this key released this frame (edge)?
    pub fn released(&self, key: Key) -> bool {
        self.released.contains(&key)
    }

    /// Mouse position in window-local coordinates.
    pub fn mouse(&self) -> (f32, f32) {
        (self.mouse_x, self.mouse_y)
    }

    /// Is the primary mouse button held?
    pub fn mouse_down(&self) -> bool {
        self.mouse_down
    }

    /// Was the primary mouse button clicked this frame (edge)?
    pub fn mouse_clicked(&self) -> bool {
        self.mouse_clicked
    }

    /// Feed a key-press event. Called by a backend.
    pub fn on_key_down(&mut self, key: Key) {
        if self.down.insert(key) {
            self.pressed.insert(key);
        }
    }

    /// Feed a key-release event. Called by a backend.
    pub fn on_key_up(&mut self, key: Key) {
        if self.down.remove(&key) {
            self.released.insert(key);
        }
    }

    /// Feed a mouse move event.
    pub fn on_mouse_move(&mut self, x: f32, y: f32) {
        self.mouse_x = x;
        self.mouse_y = y;
    }

    /// Feed a mouse down event.
    pub fn on_mouse_down(&mut self) {
        if !self.mouse_down {
            self.mouse_clicked = true;
        }
        self.mouse_down = true;
    }

    /// Feed a mouse up event.
    pub fn on_mouse_up(&mut self) {
        self.mouse_down = false;
    }

    /// Clear per-frame edge events. Called by the engine between frames.
    pub fn end_frame(&mut self) {
        self.pressed.clear();
        self.released.clear();
        self.mouse_clicked = false;
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn press_and_release_edges() {
        let mut i = Input::new();
        i.on_key_down(Key::Space);
        assert!(i.down(Key::Space));
        assert!(i.pressed(Key::Space));
        i.end_frame();
        assert!(i.down(Key::Space));
        assert!(!i.pressed(Key::Space));
        i.on_key_up(Key::Space);
        assert!(!i.down(Key::Space));
        assert!(i.released(Key::Space));
    }

    #[test]
    fn mouse_click_edge() {
        let mut i = Input::new();
        i.on_mouse_down();
        assert!(i.mouse_clicked());
        i.end_frame();
        assert!(i.mouse_down());
        assert!(!i.mouse_clicked());
        i.on_mouse_up();
        assert!(!i.mouse_down());
    }

    #[test]
    fn repeated_down_does_not_fire_pressed_again() {
        let mut i = Input::new();
        i.on_key_down(Key::W);
        i.end_frame();
        i.on_key_down(Key::W);
        assert!(!i.pressed(Key::W));
    }
}
