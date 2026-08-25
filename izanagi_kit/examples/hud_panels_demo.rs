//! HUD widgets demo: `HudPanel` / `BarWidget` / `StatLine`.
//!
//! Lays out a four-panel RPG character-status screen entirely from the HUD
//! primitives — no ad-hoc geometry. Each `HudPanel` owns a bordered region;
//! `inner_x`/`inner_y`/`inner_w` give the content origin after the 1-cell
//! margin, `BarWidget::render` draws fill bars (HP/MP/Stamina/XP with distinct
//! fill glyphs), and `StatLine::render` formats labelled values with optional
//! units.
//!
//! Modules exercised:
//! - `HudPanel::new` / `inner_x` / `inner_y` / `inner_w` / `contains` / `translate`.
//! - `BarWidget::new` / `filled_cells` / `render` (custom `fill_char`).
//! - `StatLine::new` / `with_unit` / `render`.
//!
//! A cursor point is hit-tested against every panel with `contains`, and a
//! tooltip panel is positioned with `translate`.
//!
//! Run with `cargo run --example hud_panels_demo`.

use izanagi_kit::content::Color;
use izanagi_kit::{BarWidget, Cell, HudPanel, Screen, StatLine};
use std::io::{self, Write};

// ── layout ────────────────────────────────────────────────────────────────────

const SCREEN_W: u32 = 80;
const SCREEN_H: u32 = 24;

// ── palette ───────────────────────────────────────────────────────────────────

const BG: Color = Color { r: 8, g: 8, b: 18 };
const TITLE_BG: Color = Color {
    r: 18,
    g: 16,
    b: 54,
};
const TITLE_FG: Color = Color {
    r: 205,
    g: 205,
    b: 255,
};
const BORDER_FG: Color = Color {
    r: 80,
    g: 72,
    b: 120,
};
const PANEL_HDR: Color = Color {
    r: 180,
    g: 170,
    b: 230,
};
const LABEL_FG: Color = Color {
    r: 150,
    g: 150,
    b: 195,
};
const HP_FG: Color = Color {
    r: 235,
    g: 90,
    b: 90,
};
const MP_FG: Color = Color {
    r: 90,
    g: 150,
    b: 245,
};
const ST_FG: Color = Color {
    r: 235,
    g: 200,
    b: 80,
};
const XP_FG: Color = Color {
    r: 130,
    g: 220,
    b: 130,
};
const VAL_FG: Color = Color {
    r: 220,
    g: 220,
    b: 235,
};
const CURSOR_FG: Color = Color {
    r: 255,
    g: 240,
    b: 120,
};
const TIP_FG: Color = Color {
    r: 255,
    g: 225,
    b: 140,
};
const STAT_FG: Color = Color {
    r: 140,
    g: 195,
    b: 255,
};

// ── helpers ───────────────────────────────────────────────────────────────────

/// Draw a single-line box border for `panel` and write its `title` into the top edge.
fn draw_panel(screen: &mut Screen, panel: &HudPanel, title: &str) {
    let x0 = panel.x;
    let y0 = panel.y;
    let x1 = panel.x + panel.w as i32 - 1;
    let y1 = panel.y + panel.h as i32 - 1;
    for x in x0..=x1 {
        screen.set(x, y0, '─', BORDER_FG, BG);
        screen.set(x, y1, '─', BORDER_FG, BG);
    }
    for y in y0..=y1 {
        screen.set(x0, y, '│', BORDER_FG, BG);
        screen.set(x1, y, '│', BORDER_FG, BG);
    }
    screen.set(x0, y0, '┌', BORDER_FG, BG);
    screen.set(x1, y0, '┐', BORDER_FG, BG);
    screen.set(x0, y1, '└', BORDER_FG, BG);
    screen.set(x1, y1, '┘', BORDER_FG, BG);
    // Title inset into the top border.
    let t = format!(" {} ", title);
    screen.draw_str(x0 + 2, y0, &t, PANEL_HDR, BG);
}

/// Draw a labelled bar: "LABEL [====    ] cur/max".
fn draw_bar(screen: &mut Screen, x: i32, y: i32, label: &str, bar: &BarWidget, bar_fg: Color) {
    screen.draw_str(x, y, label, LABEL_FG, BG);
    let bx = x + 4;
    let rendered = bar.render();
    screen.draw_str(bx, y, &rendered, bar_fg, BG);
    let suffix = format!(" {}/{}", bar.current, bar.max);
    screen.draw_str(bx + rendered.chars().count() as i32, y, &suffix, VAL_FG, BG);
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    // Four panels laid out as a 2×2 grid.
    let vitals = HudPanel::new(1, 2, 38, 8);
    let attrs = HudPanel::new(41, 2, 38, 8);
    let progress = HudPanel::new(1, 11, 38, 9);
    let status = HudPanel::new(41, 11, 38, 9);

    // ── render ────────────────────────────────────────────────────────────────
    let mut screen = Screen::new(SCREEN_W, SCREEN_H);
    screen.clear(Cell {
        glyph: ' ',
        fg: STAT_FG,
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
        " CHARACTER STATUS   HudPanel + BarWidget + StatLine",
        TITLE_FG,
        TITLE_BG,
    );

    // ── VITALS panel: three resource bars with distinct fill glyphs ──────────
    draw_panel(&mut screen, &vitals, "VITALS");
    let ix = vitals.inner_x() + 1;
    let bar_w = vitals.inner_w() - 16;
    let mut hp = BarWidget::new(72, 100, bar_w);
    hp.fill_char = '█';
    let mut mp = BarWidget::new(31, 60, bar_w);
    mp.fill_char = '▓';
    let mut st = BarWidget::new(48, 50, bar_w);
    st.fill_char = '▒';
    draw_bar(&mut screen, ix, vitals.inner_y() + 1, "HP", &hp, HP_FG);
    draw_bar(&mut screen, ix, vitals.inner_y() + 3, "MP", &mp, MP_FG);
    draw_bar(&mut screen, ix, vitals.inner_y() + 5, "ST", &st, ST_FG);

    // ── ATTRIBUTES panel: StatLine readouts ──────────────────────────────────
    draw_panel(&mut screen, &attrs, "ATTRIBUTES");
    let attr_lines = [
        StatLine::new("STR", 17),
        StatLine::new("DEX", 13),
        StatLine::new("INT", 9),
        StatLine::new("CON", 15),
        StatLine::new("WIS", 11),
    ];
    for (i, s) in attr_lines.iter().enumerate() {
        let y = attrs.inner_y() + i as i32 + 1;
        screen.draw_str(attrs.inner_x() + 1, y, &s.render(), VAL_FG, BG);
    }

    // ── PROGRESS panel: XP bar + StatLines with units ────────────────────────
    draw_panel(&mut screen, &progress, "PROGRESS");
    let px = progress.inner_x() + 1;
    let mut xp = BarWidget::new(340, 500, progress.inner_w() - 16);
    xp.fill_char = '·';
    draw_bar(&mut screen, px, progress.inner_y() + 1, "XP", &xp, XP_FG);
    let prog_lines = [
        StatLine::new("Level", 7),
        StatLine::with_unit("Gold", 1284, "gp"),
        StatLine::with_unit("Depth", 12, "floors"),
        StatLine::with_unit("Played", 47, "min"),
    ];
    for (i, s) in prog_lines.iter().enumerate() {
        let y = progress.inner_y() + 3 + i as i32;
        screen.draw_str(px, y, &s.render(), VAL_FG, BG);
    }

    // ── STATUS panel: timed-effect bars ──────────────────────────────────────
    draw_panel(&mut screen, &status, "STATUS EFFECTS");
    let sx = status.inner_x() + 1;
    let effects: [(&str, i32, i32, Color); 3] = [
        ("Rgn", 8, 10, XP_FG),
        ("Hst", 3, 12, MP_FG),
        ("Psn", 5, 6, HP_FG),
    ];
    for (i, &(label, cur, max, col)) in effects.iter().enumerate() {
        let y = status.inner_y() + 1 + i as i32 * 2;
        let mut bar = BarWidget::new(cur, max, status.inner_w() - 16);
        bar.fill_char = '■';
        bar.empty_char = '·';
        draw_bar(&mut screen, sx, y, label, &bar, col);
    }

    // ── cursor hit-test with contains() + tooltip via translate() ────────────
    let cursor = (52, 6); // inside the ATTRIBUTES panel
    screen.set(cursor.0, cursor.1, '╳', CURSOR_FG, BG);
    let panels: [(&str, &HudPanel); 4] = [
        ("VITALS", &vitals),
        ("ATTRIBUTES", &attrs),
        ("PROGRESS", &progress),
        ("STATUS", &status),
    ];
    let hit = panels
        .iter()
        .find(|(_, p)| p.contains(cursor.0, cursor.1))
        .map(|(n, _)| *n)
        .unwrap_or("none");

    // Tooltip panel positioned by translating a small panel near the cursor.
    let tip = HudPanel::new(cursor.0, cursor.1, 1, 1).translate(2, 1);
    let tip_text = format!("cursor in: {}", hit);
    screen.draw_str(tip.x, tip.y, &tip_text, TIP_FG, BG);

    // ── stats line ──────────────────────────────────────────────────────────
    let stat = format!(
        " HP {}/{} filled={}cells  cursor=({},{}) → {}  panels=4",
        hp.current,
        hp.max,
        hp.filled_cells(),
        cursor.0,
        cursor.1,
        hit,
    );
    screen.draw_str(0, SCREEN_H as i32 - 1, &stat, STAT_FG, BG);

    // ── output ────────────────────────────────────────────────────────────────
    screen.present();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(b"\x1b[2J");
    let _ = out.write_all(screen.to_ansi().as_bytes());
    let _ = out.write_all(b"\r\n");
    let _ = out.flush();

    eprintln!(
        "\nHUD widgets demo.\n\
         Four HudPanels (VITALS/ATTRIBUTES/PROGRESS/STATUS) laid out with\n\
         inner_* content origins. BarWidget bars: HP {}/{} ({} filled cells),\n\
         MP {}/{}, ST {}/{}. cursor=({},{}) hit-tested via contains() → {}.",
        hp.current,
        hp.max,
        hp.filled_cells(),
        mp.current,
        mp.max,
        st.current,
        st.max,
        cursor.0,
        cursor.1,
        hit,
    );
}
