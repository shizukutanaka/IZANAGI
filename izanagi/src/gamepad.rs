//! Gamepad / controller input.
//!
//! Polled state — same model as [`crate::Input`] for keyboard. The null
//! backend feeds no events; a platform backend calls `on_*` methods.
//!
//! Up to 4 gamepads (indices 0–3) are tracked simultaneously.

/// Standard gamepad buttons (Xbox layout naming).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub enum Button {
    South,
    North,
    East,
    West,
    L1,
    R1,
    L2,
    R2,
    L3,
    R3,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
    Start,
    Select,
}

/// Normalized analog stick value (−1.0 to 1.0 per axis).
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Stick {
    /// Horizontal axis. −1 = full left, +1 = full right.
    pub x: f32,
    /// Vertical axis. −1 = full up, +1 = full down.
    pub y: f32,
}

impl Stick {
    /// Apply a circular deadzone. Values inside radius `dz` are zeroed.
    pub fn with_deadzone(self, dz: f32) -> Self {
        let mag = (self.x * self.x + self.y * self.y).sqrt();
        if mag < dz {
            Self::default()
        } else {
            let scale = (mag - dz) / (1.0 - dz).max(f32::EPSILON);
            Self {
                x: self.x / mag * scale,
                y: self.y / mag * scale,
            }
        }
    }
}

use std::collections::HashSet;

struct Pad {
    down: HashSet<Button>,
    pressed: HashSet<Button>,
    released: HashSet<Button>,
    left: Stick,
    right: Stick,
    lt: f32,
    rt: f32,
    connected: bool,
}

impl Pad {
    fn new() -> Self {
        Self {
            down: HashSet::new(),
            pressed: HashSet::new(),
            released: HashSet::new(),
            left: Stick::default(),
            right: Stick::default(),
            lt: 0.0,
            rt: 0.0,
            connected: false,
        }
    }
    fn end_frame(&mut self) {
        self.pressed.clear();
        self.released.clear();
    }
}

/// Tracks up to 4 gamepads.
pub struct Gamepads {
    pads: [Pad; 4],
}

impl Gamepads {
    /// Create with all pads disconnected.
    pub fn new() -> Self {
        Self {
            pads: [Pad::new(), Pad::new(), Pad::new(), Pad::new()],
        }
    }

    // Option::is_some_and (clippy's preferred replacement for map_or(false, ..))
    // needs Rust 1.70; this crate's MSRV is 1.65 (CLAUDE.md), so map_or stays
    // and the lint is silenced at each call site instead of crate-wide.

    /// Is pad `id` connected?
    #[allow(clippy::unnecessary_map_or)]
    pub fn connected(&self, id: usize) -> bool {
        self.pads.get(id).map_or(false, |p| p.connected)
    }

    /// Is button held on pad `id`?
    #[allow(clippy::unnecessary_map_or)]
    pub fn down(&self, id: usize, btn: Button) -> bool {
        self.pads.get(id).map_or(false, |p| p.down.contains(&btn))
    }

    /// Was button pressed this frame on pad `id`?
    #[allow(clippy::unnecessary_map_or)]
    pub fn pressed(&self, id: usize, btn: Button) -> bool {
        self.pads
            .get(id)
            .map_or(false, |p| p.pressed.contains(&btn))
    }

    /// Was button released this frame on pad `id`?
    #[allow(clippy::unnecessary_map_or)]
    pub fn released(&self, id: usize, btn: Button) -> bool {
        self.pads
            .get(id)
            .map_or(false, |p| p.released.contains(&btn))
    }

    /// Left analog stick for pad `id`.
    pub fn left_stick(&self, id: usize) -> Stick {
        self.pads.get(id).map_or(Stick::default(), |p| p.left)
    }

    /// Right analog stick for pad `id`.
    pub fn right_stick(&self, id: usize) -> Stick {
        self.pads.get(id).map_or(Stick::default(), |p| p.right)
    }

    /// Left trigger (0.0–1.0) for pad `id`.
    pub fn left_trigger(&self, id: usize) -> f32 {
        self.pads.get(id).map_or(0.0, |p| p.lt)
    }

    /// Right trigger (0.0–1.0) for pad `id`.
    pub fn right_trigger(&self, id: usize) -> f32 {
        self.pads.get(id).map_or(0.0, |p| p.rt)
    }

    // ── Backend feed methods ─────────────────────────────────────────────

    /// Mark pad `id` connected or disconnected.
    pub fn on_connect(&mut self, id: usize, connected: bool) {
        if let Some(p) = self.pads.get_mut(id) {
            p.connected = connected;
        }
    }

    /// Feed a button press event.
    pub fn on_button_down(&mut self, id: usize, btn: Button) {
        if let Some(p) = self.pads.get_mut(id) {
            if p.down.insert(btn) {
                p.pressed.insert(btn);
            }
        }
    }

    /// Feed a button release event.
    pub fn on_button_up(&mut self, id: usize, btn: Button) {
        if let Some(p) = self.pads.get_mut(id) {
            if p.down.remove(&btn) {
                p.released.insert(btn);
            }
        }
    }

    /// Feed analog stick values.
    pub fn on_left_stick(&mut self, id: usize, x: f32, y: f32) {
        if let Some(p) = self.pads.get_mut(id) {
            p.left = Stick {
                x: x.clamp(-1.0, 1.0),
                y: y.clamp(-1.0, 1.0),
            };
        }
    }

    /// Feed right stick values.
    pub fn on_right_stick(&mut self, id: usize, x: f32, y: f32) {
        if let Some(p) = self.pads.get_mut(id) {
            p.right = Stick {
                x: x.clamp(-1.0, 1.0),
                y: y.clamp(-1.0, 1.0),
            };
        }
    }

    /// Feed trigger values.
    pub fn on_triggers(&mut self, id: usize, lt: f32, rt: f32) {
        if let Some(p) = self.pads.get_mut(id) {
            p.lt = lt.clamp(0.0, 1.0);
            p.rt = rt.clamp(0.0, 1.0);
        }
    }

    /// Clear per-frame edge events. Called by the engine between frames.
    pub fn end_frame(&mut self) {
        for p in self.pads.iter_mut() {
            p.end_frame();
        }
    }
}

impl Default for Gamepads {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disconnected_by_default() {
        let g = Gamepads::new();
        assert!(!g.connected(0));
    }

    #[test]
    fn button_press_edge() {
        let mut g = Gamepads::new();
        g.on_connect(0, true);
        g.on_button_down(0, Button::South);
        assert!(g.down(0, Button::South));
        assert!(g.pressed(0, Button::South));
        g.end_frame();
        assert!(g.down(0, Button::South));
        assert!(!g.pressed(0, Button::South));
        g.on_button_up(0, Button::South);
        assert!(!g.down(0, Button::South));
        assert!(g.released(0, Button::South));
    }

    #[test]
    fn stick_deadzone() {
        let s = Stick { x: 0.05, y: 0.05 };
        let d = s.with_deadzone(0.1);
        assert_eq!(d.x, 0.0);
        assert_eq!(d.y, 0.0);
    }

    #[test]
    fn stick_outside_deadzone_is_nonzero() {
        let s = Stick { x: 0.8, y: 0.0 };
        let d = s.with_deadzone(0.1);
        assert!(d.x > 0.0);
    }

    #[test]
    fn trigger_clamps() {
        let mut g = Gamepads::new();
        g.on_connect(0, true);
        g.on_triggers(0, 2.0, -1.0);
        assert_eq!(g.left_trigger(0), 1.0);
        assert_eq!(g.right_trigger(0), 0.0);
    }

    #[test]
    fn oob_pad_returns_defaults() {
        let g = Gamepads::new();
        assert!(!g.down(99, Button::South));
        assert_eq!(g.left_trigger(99), 0.0);
    }
}
