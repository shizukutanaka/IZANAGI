//! Backends — where pixels actually end up.
//!
//! A backend owns the main loop. It polls OS events, ticks the game, then
//! presents the draw list. The same game code runs on every backend.
//!
//! Two backends ship in-crate:
//! - [`NullBackend`]: headless, ticks N frames then stops. Default.
//!   Used by tests, CI, and any non-interactive run.
//! - [`TerminalBackend`]: writes ANSI-colored blocks to stdout. Lets you
//!   actually play the game in a terminal with no windowing system.
//!
//! For native windows, plug in a backend that wraps winit or SDL2 via the
//! same trait. The engine does not care which.

use crate::input::Input;
use crate::render::{Color, Draw};
use crate::Result;
use std::io::{self, Write};
use std::time::{Duration, Instant};

/// A platform backend.
pub trait Backend {
    /// Called once before the first frame.
    fn init(&mut self) -> Result<()>;

    /// Called at the top of every frame. Return `false` to request quit.
    fn poll(&mut self, input: &mut Input) -> bool;

    /// Called at the end of every frame with the draw list.
    fn present(&mut self, clear: Color, draws: &[Draw], texts: &[String]);

    /// Seconds between this frame's start and the previous one.
    /// Used by [`crate::Engine`] to drive [`crate::Time`].
    fn dt(&self) -> f32;

    /// Called after the main loop exits.
    fn shutdown(&mut self) {}
}

// ─────────────────────────────────────────────────────────────────
// NullBackend
// ─────────────────────────────────────────────────────────────────

/// Headless backend. Runs `max_frames` frames then requests quit.
pub struct NullBackend {
    /// How many frames to simulate.
    pub max_frames: u64,
    /// Fixed delta-time per frame in seconds.
    pub fixed_dt: f32,
    frame: u64,
}

impl NullBackend {
    /// 600 frames at 1/60s each — ten seconds of simulated play.
    pub fn new() -> Self {
        Self {
            max_frames: 600,
            fixed_dt: 1.0 / 60.0,
            frame: 0,
        }
    }

    /// Configure frame count (for tests that want a specific length).
    pub fn with_frames(mut self, n: u64) -> Self {
        self.max_frames = n;
        self
    }
}

impl Default for NullBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for NullBackend {
    fn init(&mut self) -> Result<()> {
        Ok(())
    }
    fn poll(&mut self, _input: &mut Input) -> bool {
        self.frame += 1;
        self.frame <= self.max_frames
    }
    fn present(&mut self, _clear: Color, _draws: &[Draw], _texts: &[String]) {}
    fn dt(&self) -> f32 {
        self.fixed_dt
    }
}

// ─────────────────────────────────────────────────────────────────
// TerminalBackend
// ─────────────────────────────────────────────────────────────────

/// Backend that draws to a terminal using ANSI escape codes.
///
/// Rasterizes rectangles onto a character grid and prints each frame.
/// Does not read keyboard input (use `NullBackend` with a test harness,
/// or plug in a crossterm-based backend if you need real input).
pub struct TerminalBackend {
    /// Grid width in characters.
    pub cols: u32,
    /// Grid height in rows (each row is two pixels tall, using half-blocks).
    pub rows: u32,
    /// Virtual-canvas width. Draws are scaled from this range to `cols`.
    pub canvas_w: f32,
    /// Virtual-canvas height.
    pub canvas_h: f32,
    /// Target frame duration.
    pub frame_time: Duration,
    /// Maximum number of frames to run. 0 = forever.
    pub max_frames: u64,
    last: Instant,
    start: Instant,
    frame: u64,
    quit_after: Option<f32>,
    /// Previous frame's emitted colors, one `(top_rgb, bottom_rgb)` per
    /// character cell (row-major, `rows * cols`). Empty means "no prior frame"
    /// (first frame or a resize) → next present does a full redraw. Cell-level
    /// diffing against this only re-emits changed cells, the technique behind
    /// sub-millisecond TUI redraws (Ratatui's back-buffer diff model).
    prev: Vec<(RgbCell, RgbCell)>,
}

/// A quantized 24-bit color as actually emitted to the terminal. Diffing on
/// these (not on the f32 `Color`) matches the bytes on the wire exactly and
/// sidesteps float-equality hazards.
type RgbCell = (u8, u8, u8);

impl TerminalBackend {
    /// Default 80x24 terminal, 800x600 virtual canvas, ~30 fps, 5 second demo.
    pub fn new() -> Self {
        Self {
            cols: 80,
            rows: 24,
            canvas_w: 800.0,
            canvas_h: 600.0,
            frame_time: Duration::from_millis(33),
            max_frames: 0,
            last: Instant::now(),
            start: Instant::now(),
            frame: 0,
            quit_after: Some(5.0),
            prev: Vec::new(),
        }
    }

    /// Configure grid size.
    pub fn size(mut self, cols: u32, rows: u32) -> Self {
        self.cols = cols;
        self.rows = rows;
        self
    }

    /// Configure virtual canvas.
    pub fn canvas(mut self, w: f32, h: f32) -> Self {
        self.canvas_w = w;
        self.canvas_h = h;
        self
    }

    /// Quit after N seconds (helpful for demos in non-interactive shells).
    pub fn quit_after(mut self, secs: f32) -> Self {
        self.quit_after = Some(secs);
        self
    }

    /// Run forever. Caller must press Ctrl-C.
    pub fn no_quit(mut self) -> Self {
        self.quit_after = None;
        self
    }
}

impl Default for TerminalBackend {
    fn default() -> Self {
        Self::new()
    }
}

fn ansi_color(c: Color) -> (u8, u8, u8) {
    (
        (c.r.clamp(0.0, 1.0) * 255.0) as u8,
        (c.g.clamp(0.0, 1.0) * 255.0) as u8,
        (c.b.clamp(0.0, 1.0) * 255.0) as u8,
    )
}

/// Append one character cell's escape sequence: set foreground (top pixel),
/// background (bottom pixel), draw the upper-half-block `▀`.
fn push_cell(out: &mut String, cell: &(RgbCell, RgbCell)) {
    let ((tr, tg, tb), (br, bg, bb)) = *cell;
    out.push_str(&format!("\x1b[38;2;{tr};{tg};{tb}m\x1b[48;2;{br};{bg};{bb}m▀"));
}

/// Build the escape-sequence string that transforms the terminal grid from
/// `prev` to `cur` (`rows` character rows of `w` cells each, row-major).
///
/// Pure — no I/O — so it is unit-testable. Two modes:
/// - **Full repaint** when `prev.len() != cur.len()` (first frame, or a resize
///   changed the cell count): homes the cursor and emits every cell.
/// - **Cell diff** otherwise: emits only cells that changed, batching
///   consecutive changed cells in a row into a single run so each run costs one
///   cursor-move, not one per cell. Unchanged cells produce no output at all —
///   the whole point (Ratatui's back-buffer diff model). An unchanged frame
///   yields the empty string.
fn terminal_grid_update(
    prev: &[(RgbCell, RgbCell)],
    cur: &[(RgbCell, RgbCell)],
    w: usize,
    rows: usize,
) -> String {
    let mut out = String::with_capacity(w * rows * 4);
    if prev.len() != cur.len() {
        out.push_str("\x1b[H"); // cursor home, no full clear (less flicker)
        for row in 0..rows {
            for col in 0..w {
                push_cell(&mut out, &cur[row * w + col]);
            }
            out.push_str("\x1b[0m\r\n");
        }
        return out;
    }
    for row in 0..rows {
        let mut col = 0;
        while col < w {
            if cur[row * w + col] != prev[row * w + col] {
                out.push_str(&format!("\x1b[{};{}H", row + 1, col + 1));
                while col < w && cur[row * w + col] != prev[row * w + col] {
                    push_cell(&mut out, &cur[row * w + col]);
                    col += 1;
                }
                out.push_str("\x1b[0m");
            } else {
                col += 1;
            }
        }
    }
    out
}

impl Backend for TerminalBackend {
    fn init(&mut self) -> Result<()> {
        // Hide cursor, clear screen.
        let mut out = io::stdout().lock();
        write!(out, "\x1b[?25l\x1b[2J").ok();
        out.flush().ok();
        self.start = Instant::now();
        self.last = Instant::now();
        Ok(())
    }

    fn poll(&mut self, _input: &mut Input) -> bool {
        self.frame += 1;
        if self.max_frames > 0 && self.frame > self.max_frames {
            return false;
        }
        if let Some(limit) = self.quit_after {
            if self.start.elapsed().as_secs_f32() > limit {
                return false;
            }
        }
        true
    }

    fn present(&mut self, clear: Color, draws: &[Draw], texts: &[String]) {
        let w = self.cols as usize;
        let h = self.rows as usize * 2; // half-blocks double vertical resolution
        let mut buf = vec![clear; w * h];

        let sx = self.cols as f32 / self.canvas_w;
        let sy = (self.rows as f32 * 2.0) / self.canvas_h;

        for d in draws {
            match *d {
                Draw::Clear(c) => {
                    for cell in buf.iter_mut() {
                        *cell = c;
                    }
                }
                Draw::Rect {
                    x,
                    y,
                    w: rw,
                    h: rh,
                    color,
                } => {
                    let x0 = (x * sx) as i32;
                    let y0 = (y * sy) as i32;
                    let x1 = ((x + rw) * sx) as i32;
                    let y1 = ((y + rh) * sy) as i32;
                    for py in y0.max(0)..y1.min(h as i32) {
                        for px in x0.max(0)..x1.min(w as i32) {
                            buf[py as usize * w + px as usize] = color;
                        }
                    }
                }
                Draw::Text {
                    x,
                    y,
                    color,
                    text_id,
                    ..
                } => {
                    let Some(txt) = texts.get(text_id) else {
                        continue;
                    };
                    let tx = (x * sx) as i32;
                    let ty = (y * sy) as i32;
                    for (i, ch) in txt.chars().enumerate() {
                        let px = tx + i as i32;
                        if px < 0 || px >= w as i32 || ty < 0 || ty >= h as i32 {
                            continue;
                        }
                        // Stash text into buf by encoding into a scratch grid later.
                        // Simpler: overlay after rendering. We'll draw text after blit.
                        let _ = ch;
                        buf[ty as usize * w + px as usize] = color;
                    }
                }
                Draw::ScissorPush { .. } | Draw::ScissorPop => {
                    // TerminalBackend ignores scissor — too coarse-grained to matter.
                }
                Draw::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    color,
                    ..
                } => {
                    // Bresenham line in scaled grid space.
                    let mut p0 = ((x1 * sx) as i32, (y1 * sy) as i32);
                    let p1 = ((x2 * sx) as i32, (y2 * sy) as i32);
                    let dx = (p1.0 - p0.0).abs();
                    let dy = -(p1.1 - p0.1).abs();
                    let sxi = if p0.0 < p1.0 { 1 } else { -1 };
                    let syi = if p0.1 < p1.1 { 1 } else { -1 };
                    let mut err = dx + dy;
                    loop {
                        if p0.0 >= 0 && p0.0 < w as i32 && p0.1 >= 0 && p0.1 < h as i32 {
                            buf[p0.1 as usize * w + p0.0 as usize] = color;
                        }
                        if p0 == p1 {
                            break;
                        }
                        let e2 = 2 * err;
                        if e2 >= dy {
                            err += dy;
                            p0.0 += sxi;
                        }
                        if e2 <= dx {
                            err += dx;
                            p0.1 += syi;
                        }
                    }
                }
                Draw::Circle {
                    cx,
                    cy,
                    radius,
                    filled,
                    color,
                } => {
                    let cxp = (cx * sx) as i32;
                    let cyp = (cy * sy) as i32;
                    let rp = (radius * sx.min(sy)) as i32;
                    for py in (cyp - rp).max(0)..=(cyp + rp).min(h as i32 - 1) {
                        for px in (cxp - rp).max(0)..=(cxp + rp).min(w as i32 - 1) {
                            let dx2 = (px - cxp).pow(2);
                            let dy2 = (py - cyp).pow(2);
                            let d2 = dx2 + dy2;
                            let inside = d2 <= rp * rp;
                            let on_edge = (d2 - rp * rp).abs() < (2 * rp).max(1);
                            if (filled && inside) || (!filled && on_edge) {
                                buf[py as usize * w + px as usize] = color;
                            }
                        }
                    }
                }
                Draw::Sprite {
                    x,
                    y,
                    w: sw,
                    h: sh,
                    tint,
                    ..
                } => {
                    // Terminal backend has no atlas; render the bbox as a tinted block.
                    let x0 = (x * sx) as i32;
                    let y0 = (y * sy) as i32;
                    let x1 = ((x + sw) * sx) as i32;
                    let y1 = ((y + sh) * sy) as i32;
                    for py in y0.max(0)..y1.min(h as i32) {
                        for px in x0.max(0)..x1.min(w as i32) {
                            buf[py as usize * w + px as usize] = tint;
                        }
                    }
                }
            }
        }

        // Collapse the two half-block rows per character row into one
        // `(top_rgb, bottom_rgb)` cell each, quantized to what actually goes on
        // the wire — this is the buffer we diff against last frame.
        let rows = self.rows as usize;
        let mut cur: Vec<(RgbCell, RgbCell)> = Vec::with_capacity(rows * w);
        for row in 0..rows {
            for col in 0..w {
                let top = ansi_color(buf[(row * 2) * w + col]);
                let bot = ansi_color(buf[(row * 2 + 1) * w + col]);
                cur.push((top, bot));
            }
        }

        // Build the grid update (full repaint or cell diff) via a pure helper,
        // then append the text overlay.
        let mut out = terminal_grid_update(&self.prev, &cur, w, rows);

        // Text overlay: reposition below the grid, clear from there to end of
        // screen (so shrinking text leaves no stale lines), then write. Kept as
        // a full rewrite each frame — cheap, and its content/position varies.
        out.push_str(&format!("\x1b[{};1H\x1b[J", rows + 1));
        for txt in texts {
            out.push_str("\x1b[0m");
            out.push_str(txt);
            out.push_str("\r\n");
        }

        let mut stdout = io::stdout().lock();
        let _ = stdout.write_all(out.as_bytes());
        let _ = stdout.flush();
        self.prev = cur;

        // Pace.
        let elapsed = self.last.elapsed();
        if elapsed < self.frame_time {
            std::thread::sleep(self.frame_time - elapsed);
        }
        self.last = Instant::now();
    }

    fn dt(&self) -> f32 {
        self.frame_time.as_secs_f32()
    }

    fn shutdown(&mut self) {
        let mut out = io::stdout().lock();
        // Show cursor, reset colors.
        writeln!(out, "\x1b[0m\x1b[?25h").ok();
        out.flush().ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_backend_stops_after_frame_budget() {
        let mut b = NullBackend::new().with_frames(3);
        let mut input = Input::new();
        let mut count = 0;
        while b.poll(&mut input) {
            count += 1;
            if count > 100 {
                panic!("runaway");
            }
        }
        assert_eq!(count, 3);
    }

    #[test]
    fn terminal_backend_accepts_draws_without_panic() {
        // Don't actually init (that writes to stdout).
        let mut b = TerminalBackend::new().quit_after(0.0);
        let draws = vec![Draw::Rect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
            color: Color::WHITE,
        }];
        let texts = vec![];
        // Skip init to avoid ANSI in test output; present is what we're checking.
        b.present(Color::BLACK, &draws, &texts);
    }

    #[test]
    fn ansi_color_clamps() {
        let (r, g, b) = ansi_color(Color::rgba(2.0, -1.0, 0.5, 1.0));
        assert_eq!(r, 255);
        assert_eq!(g, 0);
        assert_eq!(b, 127);
    }

    // --- terminal_grid_update (cell diffing) ---

    const BLACK: (u8, u8, u8) = (0, 0, 0);
    const WHITE: (u8, u8, u8) = (255, 255, 255);
    const RED: (u8, u8, u8) = (255, 0, 0);

    #[test]
    fn grid_update_first_frame_is_full_repaint() {
        // prev empty (len 0) != cur (len 4) → full repaint: homes and emits
        // every one of the 4 cells (each ▀), plus a row reset per row.
        let cur = vec![(BLACK, BLACK); 4]; // 2x2
        let out = terminal_grid_update(&[], &cur, 2, 2);
        assert!(out.starts_with("\x1b[H"), "full repaint homes the cursor");
        assert_eq!(out.matches('▀').count(), 4, "all 4 cells drawn");
    }

    #[test]
    fn grid_update_unchanged_frame_emits_nothing() {
        // Same prev and cur, same length → diff mode, zero changed cells →
        // empty output. This is the entire point of the optimization.
        let frame = vec![(BLACK, WHITE), (RED, BLACK), (WHITE, RED), (BLACK, BLACK)];
        let out = terminal_grid_update(&frame, &frame, 2, 2);
        assert_eq!(out, "", "an unchanged frame produces no escape sequences");
    }

    #[test]
    fn grid_update_emits_only_changed_cell() {
        // Change exactly one cell (row 1, col 0 of a 2x2). Expect one
        // cursor-move to (2,1) 1-based, one ▀, and no other cells touched.
        let prev = vec![(BLACK, BLACK); 4];
        let mut cur = prev.clone();
        cur[2] = (RED, RED); // index 2 = row 1, col 0
        let out = terminal_grid_update(&prev, &cur, 2, 2);
        assert_eq!(out.matches('▀').count(), 1, "only the one changed cell is drawn");
        assert!(out.contains("\x1b[2;1H"), "cursor moves to the changed cell (row 2, col 1)");
        assert!(!out.starts_with("\x1b[H"), "not a full repaint");
    }

    #[test]
    fn grid_update_batches_consecutive_changes_into_one_move() {
        // Two adjacent cells in the same row change → a single cursor-move
        // followed by both cells, not two separate moves.
        let prev = vec![(BLACK, BLACK); 4];
        let mut cur = prev.clone();
        cur[0] = (RED, RED); // row 0, col 0
        cur[1] = (WHITE, WHITE); // row 0, col 1 (adjacent)
        let out = terminal_grid_update(&prev, &cur, 2, 2);
        assert_eq!(out.matches('▀').count(), 2, "both changed cells drawn");
        // Exactly one cursor-position sequence (the run start), not two.
        assert_eq!(out.matches('H').count(), 1, "one cursor-move for the whole run");
        assert!(out.contains("\x1b[1;1H"), "run starts at row 1, col 1");
    }
}
