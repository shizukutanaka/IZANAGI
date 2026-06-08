//! Fixed-timestep demo: `FixedTimestep` accumulator + death-spiral guard.
//!
//! A variable real-frame trace is fed into a 60 Hz `FixedTimestep` (max 5
//! catch-up steps). The demo shows, per frame, how many fixed simulation
//! steps are emitted, the leftover accumulator, and the interpolation `alpha`
//! — and how a 500 ms stall is **clamped** to `max_steps` instead of spiralling.
//!
//! Modules exercised:
//! - `FixedTimestep::new` / `advance` / `step_ns` / `total_steps` / `alpha_ratio`.
//! - Death-spiral guard: a huge frame yields only `max_steps`, with no
//!   catch-up debt carried into the next frame.
//! - Frame-pacing independence: the same total time delivered as one big frame
//!   vs many small frames produces an identical `total_steps`.
//!
//! Run with `cargo run --example timestep_demo`.

use izanagi_kit::content::Color;
use izanagi_kit::{Cell, FixedTimestep, Screen};
use std::io::{self, Write};

// ── layout ────────────────────────────────────────────────────────────────────

const SCREEN_W: u32 = 80;
const SCREEN_H: u32 = 24;
const DIV_X: i32 = 46;

// ── palette ───────────────────────────────────────────────────────────────────

const BG: Color = Color { r: 8, g: 8, b: 16 };
const TITLE_BG: Color = Color {
    r: 16,
    g: 16,
    b: 52,
};
const TITLE_FG: Color = Color {
    r: 200,
    g: 205,
    b: 255,
};
const HDR_FG: Color = Color {
    r: 118,
    g: 114,
    b: 174,
};
const DIV_FG: Color = Color {
    r: 58,
    g: 52,
    b: 92,
};
const COL_FG: Color = Color {
    r: 150,
    g: 150,
    b: 195,
};
const VAL_FG: Color = Color {
    r: 200,
    g: 205,
    b: 230,
};
const STEP_FG: Color = Color {
    r: 110,
    g: 215,
    b: 130,
};
const ACC_FG: Color = Color {
    r: 90,
    g: 160,
    b: 245,
};
const STALL_FG: Color = Color {
    r: 245,
    g: 100,
    b: 90,
};
const OK_FG: Color = Color {
    r: 95,
    g: 210,
    b: 115,
};
const INFO_FG: Color = Color {
    r: 145,
    g: 152,
    b: 200,
};
const STAT_HI: Color = Color {
    r: 255,
    g: 218,
    b: 100,
};

const NS_PER_MS: u64 = 1_000_000;

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let mut ts = FixedTimestep::new(60, 5);
    let step_ms = ts.step_ns() as f64 / NS_PER_MS as f64; // for display only

    // A scripted variable-frame trace (milliseconds). Frame 6 is a 500 ms stall.
    let frame_ms: [u64; 9] = [17, 17, 8, 9, 50, 500, 17, 16, 33];

    struct Row {
        idx: usize,
        dt_ms: u64,
        steps: u32,
        accum_ns: u64,
        alpha_pct: u64,
        stall: bool,
    }
    let mut rows: Vec<Row> = Vec::new();
    for (i, &ms) in frame_ms.iter().enumerate() {
        let steps = ts.advance(ms * NS_PER_MS);
        let (num, den) = ts.alpha_ratio();
        let alpha_pct = num * 100 / den.max(1);
        // A frame "stalled" if its raw time would demand more than max_steps.
        let demanded = (ms * NS_PER_MS) / ts.step_ns();
        rows.push(Row {
            idx: i,
            dt_ms: ms,
            steps,
            accum_ns: num,
            alpha_pct,
            stall: demanded > 5,
        });
    }
    let traced_total = ts.total_steps();

    // ── frame-pacing independence check ──────────────────────────────────────
    // Same total time, two different chunkings, high max_steps so neither clamps.
    let one_step = FixedTimestep::new(60, 100_000).step_ns();
    let total_time = one_step * 100;
    let mut big = FixedTimestep::new(60, 100_000);
    big.advance(total_time); // one giant frame
    let mut small = FixedTimestep::new(60, 100_000);
    let chunk = total_time / 400;
    let mut delivered = 0u64;
    for i in 0..400 {
        // Deliver the exact remainder on the last frame so the two runs receive
        // identical total time (integer division would otherwise drop a few ns).
        let f = if i == 399 {
            total_time - delivered
        } else {
            chunk
        };
        small.advance(f);
        delivered += f;
    }
    let pacing_match = big.total_steps() == small.total_steps();

    // ── render ────────────────────────────────────────────────────────────────
    let mut screen = Screen::new(SCREEN_W, SCREEN_H);
    screen.clear(Cell {
        glyph: ' ',
        fg: VAL_FG,
        bg: BG,
    });

    screen.fill_rect(
        0,
        0,
        SCREEN_W,
        1,
        Cell {
            glyph: ' ',
            fg: TITLE_FG,
            bg: TITLE_BG,
        },
    );
    screen.draw_str(
        0,
        0,
        " FIXED TIMESTEP   accumulator + death-spiral guard (60Hz, max 5)",
        TITLE_FG,
        TITLE_BG,
    );

    screen.draw_str(1, 2, "FRAME TRACE", HDR_FG, BG);
    screen.draw_str(DIV_X + 2, 2, "DETERMINISM NOTES", HDR_FG, BG);
    for x in 0..SCREEN_W as i32 {
        let g = if x == DIV_X { '┬' } else { '─' };
        screen.set(x, 3, g, DIV_FG, BG);
    }
    for y in 4..22 {
        screen.set(DIV_X, y, '│', DIV_FG, BG);
    }

    // Column headers.
    screen.draw_str(1, 4, "frame  dt(ms)  steps  accum  alpha", COL_FG, BG);

    for (r, row) in rows.iter().enumerate() {
        let y = 5 + r as i32;
        let label_fg = if row.stall { STALL_FG } else { VAL_FG };
        // frame index
        screen.draw_str(2, y, &format!("f{}", row.idx), label_fg, BG);
        // dt
        screen.draw_str(8, y, &format!("{:>4}", row.dt_ms), VAL_FG, BG);
        // steps (green; red+"CLAMP" if stalled)
        let steps_str = if row.stall {
            format!("{:>3} CLAMP", row.steps)
        } else {
            format!("{:>3}", row.steps)
        };
        let steps_fg = if row.stall { STALL_FG } else { STEP_FG };
        screen.draw_str(15, y, &steps_str, steps_fg, BG);
        // accumulator as a small fraction-of-step bar
        let accum_cells = (row.accum_ns * 6 / ts.step_ns().max(1)).min(6) as usize;
        let mut bar = String::from("[");
        for c in 0..6 {
            bar.push(if c < accum_cells { '▰' } else { '▱' });
        }
        bar.push(']');
        screen.draw_str(if row.stall { 26 } else { 24 }, y, &bar, ACC_FG, BG);
        // alpha %
        screen.draw_str(35, y, &format!("{:>3}%", row.alpha_pct), INFO_FG, BG);
    }

    // ── right panel notes ─────────────────────────────────────────────────────
    let nx = DIV_X + 2;
    let mut ny = 5;
    let notes: [(&str, Color); 9] = [
        ("step = 1s/60 ≈ 16.67ms", INFO_FG),
        ("", INFO_FG),
        ("f4 (50ms) → 3 steps: a slow", VAL_FG),
        ("  frame runs extra sub-steps.", INFO_FG),
        ("", INFO_FG),
        ("f5 (500ms stall) demands 30", STALL_FG),
        ("  steps → CLAMPED to 5; the", STALL_FG),
        ("  backlog is dropped so the", INFO_FG),
        ("  sim slows, never spirals.", INFO_FG),
    ];
    for (text, col) in notes {
        if !text.is_empty() {
            screen.draw_str(nx, ny, text, col, BG);
        }
        ny += 1;
    }

    // Recovery proof.
    let recovery = &rows[6];
    screen.draw_str(
        nx,
        15,
        &format!("f6 after stall → {} step", recovery.steps),
        OK_FG,
        BG,
    );
    screen.draw_str(nx, 16, "  (no catch-up debt left)", INFO_FG, BG);

    // Pacing independence.
    screen.draw_str(nx, 18, "Frame-pacing independence:", HDR_FG, BG);
    let pacing_line = format!(
        "1 big vs 400 tiny frames → {}",
        if pacing_match { "EQUAL" } else { "DIFF!" }
    );
    let pacing_fg = if pacing_match { OK_FG } else { STALL_FG };
    screen.draw_str(nx, 19, &pacing_line, pacing_fg, BG);
    screen.draw_str(
        nx,
        20,
        &format!("  both = {} steps", big.total_steps()),
        INFO_FG,
        BG,
    );

    // ── bottom separator + stats ─────────────────────────────────────────────
    for x in 0..SCREEN_W as i32 {
        let g = if x == DIV_X { '┴' } else { '─' };
        screen.set(x, 22, g, DIV_FG, BG);
    }
    let stat = format!(
        " step≈{:.2}ms  frames={}  total_steps={}  pacing_independent={}",
        step_ms,
        frame_ms.len(),
        traced_total,
        pacing_match,
    );
    screen.draw_str(0, 23, &stat, STAT_HI, BG);

    // ── output ────────────────────────────────────────────────────────────────
    screen.present();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(b"\x1b[2J");
    let _ = out.write_all(screen.to_ansi().as_bytes());
    let _ = out.write_all(b"\r\n");
    let _ = out.flush();

    eprintln!(
        "\nFixed-timestep demo.\n\
         60Hz sim (step≈{:.2}ms), max 5 catch-up steps.\n\
         9-frame trace → total_steps={}. The 500ms stall (f5) demanded 30\n\
         steps but was CLAMPED to 5; f6 then took 1 step (no catch-up debt).\n\
         Frame-pacing independence: 1 big frame vs 400 tiny frames → both {} steps ({}).",
        step_ms,
        traced_total,
        big.total_steps(),
        if pacing_match { "EQUAL" } else { "DIFFER" },
    );
}
