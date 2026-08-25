//! Content pipeline demo: parse → validate → load → render.
//!
//! The IZANAGI content pipeline converts a text DSL (`.game` files) into a
//! live ECS world in four steps:
//!
//!   text → `parse` → `validate` → `load_level` → ECS (SparseSet world)
//!
//! This demo embeds two content bundles inline (no file I/O needed):
//!
//!   1. `VALID_CONTENT` — the same dungeon as `examples/dungeon.game`,
//!      parsed, validated, and loaded. Entities are rendered to a terminal
//!      Screen at their grid positions.
//!
//!   2. `BROKEN_CONTENT` — intentionally erroneous DSL matching
//!      `examples/broken.game`. Parse errors are rendered in a diagnostic
//!      panel (line, col, message) — identical output to `gamec` in human
//!      mode.
//!
//! Run with `cargo run --example content_pipeline_demo`.

use izanagi_kit::content::Color;
use izanagi_kit::{is_loadable, load_level, parse, validate, Cell, Screen, Severity};
use std::io::{self, Write};

// ── content DSL strings ───────────────────────────────────────────────────────

const VALID_CONTENT: &str = r#"
// A tiny dungeon room
prefab hero
  glyph @
  color #00C4CC
  stat hp 20
  stat atk 5
prefab goblin
  glyph g
  color #f85149
  stat hp 8
  flag hostile
prefab potion
  glyph !
  color #3fb950
  flag item

tile floor . #3A3A3A
tile wall # #6E7681

level room 8x5
  row ########
  row #@.....#
  row #..g..!#
  row #....g.#
  row ########
  spawn hero 1 1
  spawn goblin 3 2
  spawn goblin 5 3
  spawn potion 6 2
"#;

const BROKEN_CONTENT: &str = r#"
prefab bad_prefab
  glyph @@
  color notacolor
  stat
  flag 123

level empty 0x0
"#;

// ── colours ───────────────────────────────────────────────────────────────────

const SCREEN_W: u32 = 80;
const SCREEN_H: u32 = 24;

const BG: Color = Color {
    r: 10,
    g: 10,
    b: 18,
};
const WALL_FG: Color = Color {
    r: 110,
    g: 110,
    b: 110,
};
const FLOOR_FG: Color = Color {
    r: 60,
    g: 60,
    b: 60,
};
const TITLE_FG: Color = Color {
    r: 120,
    g: 200,
    b: 255,
};
const OK_FG: Color = Color {
    r: 80,
    g: 220,
    b: 80,
};
const ERR_FG: Color = Color {
    r: 220,
    g: 60,
    b: 60,
};
const WARN_FG: Color = Color {
    r: 220,
    g: 200,
    b: 60,
};
const DIM_FG: Color = Color {
    r: 120,
    g: 120,
    b: 120,
};
const BRIGHT_FG: Color = Color {
    r: 210,
    g: 210,
    b: 210,
};

fn blank() -> Cell {
    Cell {
        glyph: ' ',
        fg: DIM_FG,
        bg: BG,
    }
}

// ── entry point ───────────────────────────────────────────────────────────────

fn main() {
    let mut screen = Screen::new(SCREEN_W, SCREEN_H);
    screen.fill_rect(0, 0, SCREEN_W, SCREEN_H, blank());

    // ── LEFT PANEL: parse + load valid content ────────────────────────────────
    // Panel occupies columns 0..38, rows 0..23.

    let sep = "─".repeat(38);
    screen.draw_str(0, 0, "  VALID CONTENT  parse→validate→load", TITLE_FG, BG);
    screen.draw_str(0, 1, &sep, DIM_FG, BG);

    let (content, parse_diags) = parse(VALID_CONTENT);
    let validate_diags = validate(&content);
    let loadable = is_loadable(&parse_diags, &validate_diags);

    // Show pipeline step summary.
    let p_ok = parse_diags.iter().all(|d| d.severity != Severity::Error);
    let v_ok = validate_diags.is_empty();
    screen.draw_str(
        0,
        2,
        &fmt_step("parse", p_ok, parse_diags.len()),
        if p_ok { OK_FG } else { ERR_FG },
        BG,
    );
    screen.draw_str(
        0,
        3,
        &fmt_step("validate", v_ok, validate_diags.len()),
        if v_ok { OK_FG } else { ERR_FG },
        BG,
    );
    screen.draw_str(
        0,
        4,
        &format!("  loadable: {}", if loadable { "yes ✓" } else { "no" }),
        if loadable { OK_FG } else { ERR_FG },
        BG,
    );
    screen.draw_str(0, 5, &sep, DIM_FG, BG);

    // Load and render the level if loadable.
    if loadable {
        match load_level(&content, "room") {
            Ok(world) => {
                screen.draw_str(
                    0,
                    6,
                    &format!("  entities: {}", world.entity_count()),
                    OK_FG,
                    BG,
                );
                screen.draw_str(0, 7, &sep, DIM_FG, BG);

                // Render the tile grid from the level definition.
                let level_y_start = 8i32;
                if let Some(level) = content.level("room") {
                    for (row_idx, row) in level.rows.iter().enumerate() {
                        let sy = level_y_start + row_idx as i32;
                        for (col_idx, glyph) in row.chars().enumerate() {
                            let sx = 2 + col_idx as i32;
                            let (fg, bg) = match glyph {
                                '#' => (
                                    WALL_FG,
                                    Color {
                                        r: 25,
                                        g: 25,
                                        b: 25,
                                    },
                                ),
                                '.' => (
                                    FLOOR_FG,
                                    Color {
                                        r: 30,
                                        g: 28,
                                        b: 25,
                                    },
                                ),
                                _ => (BRIGHT_FG, BG),
                            };
                            screen.set(sx, sy, glyph, fg, bg);
                        }
                    }
                }

                // Overlay entities from the loaded world.
                let pos_iter = world.positions.iter_sorted();
                for (e, pos) in pos_iter.iter() {
                    if let Some(render) = world.renders.get(*e) {
                        let sx = 2 + pos.x as i32;
                        let sy = level_y_start + pos.y as i32;
                        screen.set(
                            sx,
                            sy,
                            render.glyph,
                            render.color,
                            Color {
                                r: 30,
                                g: 28,
                                b: 25,
                            },
                        );
                    }
                }

                // Entity list below the map.
                let list_y = level_y_start + 6;
                screen.draw_str(0, list_y, &sep, DIM_FG, BG);
                let mut ey = list_y + 1;
                for e in &world.entities {
                    if let (Some(pos), Some(render)) =
                        (world.positions.get(*e), world.renders.get(*e))
                    {
                        let line = format!(
                            "  e{:?}  '{}'  #{:02X}{:02X}{:02X}  @({},{})",
                            e.index(),
                            render.glyph,
                            render.color.r,
                            render.color.g,
                            render.color.b,
                            pos.x,
                            pos.y,
                        );
                        screen.draw_str(0, ey, &line, render.color, BG);
                        ey += 1;
                    }
                }
            }
            Err(e) => {
                screen.draw_str(0, 7, &format!("  load error: {}", e), ERR_FG, BG);
            }
        }
    }

    // ── SEPARATOR ─────────────────────────────────────────────────────────────
    for y in 0..SCREEN_H as i32 {
        screen.set(39, y, '│', DIM_FG, BG);
    }

    // ── RIGHT PANEL: parse broken content, show diagnostics ──────────────────
    // Panel occupies columns 40..80, rows 0..23.

    let sep2 = "─".repeat(39);
    screen.draw_str(40, 0, " BROKEN CONTENT  parse diagnostics", TITLE_FG, BG);
    screen.draw_str(40, 1, &sep2, DIM_FG, BG);

    let (_, broken_diags) = parse(BROKEN_CONTENT);
    let has_errors = broken_diags.iter().any(|d| d.severity == Severity::Error);
    screen.draw_str(
        40,
        2,
        &format!(
            "  {} diagnostics  ({} errors)",
            broken_diags.len(),
            broken_diags
                .iter()
                .filter(|d| d.severity == Severity::Error)
                .count()
        ),
        if has_errors { ERR_FG } else { OK_FG },
        BG,
    );
    screen.draw_str(40, 3, &sep2, DIM_FG, BG);

    let mut dy = 4i32;
    for diag in &broken_diags {
        if dy >= SCREEN_H as i32 - 1 {
            break;
        }
        let (marker, fg) = match diag.severity {
            Severity::Error => ("E", ERR_FG),
            Severity::Warning => ("W", WARN_FG),
        };
        let header = format!("  [{}] {}:{}", marker, diag.line, diag.col);
        screen.draw_str(40, dy, &header, fg, BG);
        dy += 1;

        // Word-wrap message into panel width (39 chars).
        let msg = format!("      {}", diag.message);
        let truncated: String = msg.chars().take(38).collect();
        screen.draw_str(40, dy, &truncated, DIM_FG, BG);
        dy += 2;
    }

    screen.present();

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(b"\x1b[2J");
    let _ = out.write_all(screen.to_ansi().as_bytes());
    let _ = out.write_all(b"\r\n");
    let _ = out.flush();

    eprintln!(
        "\nContent pipeline demo.  valid_entities={}  broken_diags={}",
        {
            let (c, pd) = parse(VALID_CONTENT);
            let vd = validate(&c);
            if is_loadable(&pd, &vd) {
                load_level(&c, "room")
                    .map(|w| w.entity_count())
                    .unwrap_or(0)
            } else {
                0
            }
        },
        broken_diags.len(),
    );
}

fn fmt_step(name: &str, ok: bool, diag_count: usize) -> String {
    if ok {
        format!("  {} ✓  (0 errors)", name)
    } else {
        format!("  {} ✗  ({} diagnostics)", name, diag_count)
    }
}
