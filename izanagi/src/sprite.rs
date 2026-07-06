//! Sprite and frame animation.
//!
//! A [`Sprite`] references a region of a texture atlas (tileset). An
//! [`Animation`] sequences frames over time and loops or holds.
//!
//! ```
//! use izanagi::sprite::{Animation, Frame, Sprite};
//!
//! let run = Animation::new(vec![
//!     Frame { sprite: Sprite::new(0, 0, 16, 16), duration: 0.1 },
//!     Frame { sprite: Sprite::new(16, 0, 16, 16), duration: 0.1 },
//!     Frame { sprite: Sprite::new(32, 0, 16, 16), duration: 0.1 },
//! ], true);
//! ```

/// A rectangular region in a texture atlas, in pixels.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Sprite {
    /// Left edge in the atlas (px).
    pub x: u32,
    /// Top edge in the atlas (px).
    pub y: u32,
    /// Width (px).
    pub w: u32,
    /// Height (px).
    pub h: u32,
}

impl Sprite {
    /// Construct from atlas coordinates.
    pub const fn new(x: u32, y: u32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    /// Sprite at column `col`, row `row` in a uniformly-tiled atlas.
    ///
    /// `tile_w` / `tile_h` are the pixel dimensions of each cell.
    pub const fn from_grid(col: u32, row: u32, tile_w: u32, tile_h: u32) -> Self {
        Self {
            x: col * tile_w,
            y: row * tile_h,
            w: tile_w,
            h: tile_h,
        }
    }
}

/// A single animation frame.
#[derive(Clone, Debug)]
pub struct Frame {
    /// Sprite (atlas region) to display.
    pub sprite: Sprite,
    /// How long to display this frame, in seconds.
    pub duration: f32,
}

/// A sequence of frames that plays over time.
#[derive(Clone, Debug)]
pub struct Animation {
    frames: Vec<Frame>,
    looping: bool,
    current: usize,
    elapsed: f32,
    done: bool,
}

impl Animation {
    /// New animation from a list of frames.
    ///
    /// `looping = true` wraps back to frame 0 on completion.
    pub fn new(frames: Vec<Frame>, looping: bool) -> Self {
        assert!(!frames.is_empty(), "animation must have at least one frame");
        Self {
            frames,
            looping,
            current: 0,
            elapsed: 0.0,
            done: false,
        }
    }

    /// Advance by `dt` seconds. Returns the current [`Sprite`] to draw.
    pub fn tick(&mut self, dt: f32) -> Sprite {
        if !self.done {
            self.elapsed += dt;
            while self.elapsed >= self.frames[self.current].duration {
                self.elapsed -= self.frames[self.current].duration;
                if self.current + 1 < self.frames.len() {
                    self.current += 1;
                } else if self.looping {
                    self.current = 0;
                } else {
                    self.done = true;
                    break;
                }
            }
        }
        self.frames[self.current].sprite
    }

    /// Current frame index.
    pub fn frame_index(&self) -> usize {
        self.current
    }

    /// True if a non-looping animation has finished.
    pub fn done(&self) -> bool {
        self.done
    }

    /// Restart from frame 0.
    pub fn restart(&mut self) {
        self.current = 0;
        self.elapsed = 0.0;
        self.done = false;
    }

    /// Jump to a specific frame index.
    pub fn goto(&mut self, index: usize) {
        self.current = index.min(self.frames.len() - 1);
        self.elapsed = 0.0;
    }

    /// Number of frames.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Always false (an animation always has at least one frame).
    pub fn is_empty(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn three_frame_anim(looping: bool) -> Animation {
        Animation::new(
            (0..3)
                .map(|i| Frame {
                    sprite: Sprite::new(i * 16, 0, 16, 16),
                    duration: 0.1,
                })
                .collect(),
            looping,
        )
    }

    #[test]
    fn first_frame_at_zero() {
        let mut a = three_frame_anim(false);
        let s = a.tick(0.0);
        assert_eq!(s.x, 0);
    }

    #[test]
    fn advances_to_second_frame() {
        let mut a = three_frame_anim(false);
        a.tick(0.15);
        assert_eq!(a.frame_index(), 1);
    }

    #[test]
    fn non_looping_stops_at_last() {
        let mut a = three_frame_anim(false);
        a.tick(10.0); // far past the end
        assert!(a.done());
        assert_eq!(a.frame_index(), 2);
    }

    #[test]
    fn looping_wraps_back() {
        let mut a = three_frame_anim(true);
        // 0.3s = exactly 3 frames → should be back at 0.
        a.tick(0.3);
        assert_eq!(a.frame_index(), 0);
        assert!(!a.done());
    }

    #[test]
    fn restart_resets_state() {
        let mut a = three_frame_anim(false);
        a.tick(10.0);
        assert!(a.done());
        a.restart();
        assert!(!a.done());
        assert_eq!(a.frame_index(), 0);
    }

    #[test]
    fn from_grid_coordinates() {
        let s = Sprite::from_grid(2, 1, 16, 16);
        assert_eq!(s.x, 32);
        assert_eq!(s.y, 16);
    }
}
