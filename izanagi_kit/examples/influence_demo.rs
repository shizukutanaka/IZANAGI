//! Influence-map + HUD demo.
//!
//! Generates a dungeon, seeds an influence map with sources at room centres
//! (monsters = positive influence, traps = negative), and renders the scalar
//! field as a heat-map using 24-bit ANSI colour: blue (safe) → yellow →
//! red (dangerous). HUD widgets (`BarWidget`, `StatLine`) are drawn in a
//! right-hand panel showing the strongest source and map statistics.
//!
//! Arrow indicators show the steepest-ascent direction at the player's
//! starting position (the result of `InfluenceMap::highest_neighbour`),
//! demonstrating how AI can chase or flee peaks without explicit pathfinding.
//!
//! Run with `cargo run --example influence_demo`.

use izanagi_kit::content::Color;
use izanagi_kit::hud::{BarWidget, HudPanel, StatLine};
use izanagi_kit::influence::InfluenceMap;
use izanagi_kit::{generate_dungeon, Camera, Cell, GenParams, PassabilityGrid, Screen, SplitMix64};
use std::io::{self, Write};

// ── palette ───────────────────────────────────────────────────────────────────

/// Map an influence value in `[-max, max]` to a 24-bit heat-map colour.
/// Negative = blue (safe), zero = grey, positive = yellow/red (danger).
fn heat_colour(value: i32, max: i32) -> Color {
    if max == 0 {
        return Color {
            r: 60,
            g: 60,
            b: 60,
        };
    }
    if value >= 0 {
        // 0 → grey (60,60,60), max → red (220,40,40)
        let t = (value.min(max) * 255 / max.max(1)) as u8;
        Color {
            r: 60 + (160 * t as u32 / 255) as u8,
            g: 60u8.saturating_sub((20 * t as u32 / 255) as u8),
            b: 60u8.saturating_sub((20 * t as u32 / 255) as u8),
        }
    } else {
        // 0 → grey (60,60,60), -max → blue (40,40,220)
        let t = ((-value).min(max) * 255 / max.max(1)) as u8;
        Color {
            r: 60u8.saturating_sub((20 * t as u32 / 255) as u8),
            g: 60u8.saturating_sub((20 * t as u32 / 255) as u8),
            b: 60 + (160 * t as u32 / 255) as u8,
        }
    }
}

const WALL_FG: Color = Color {
    r: 90,
    g: 90,
    b: 90,
};
const WALL_BG: Color = Color {
    r: 20,
    g: 20,
    b: 20,
};
const PLAYER_FG: Color = Color {
    r: 240,
    g: 230,
    b: 60,
};
const PANEL_BG: Color = Color {
    r: 12,
    g: 12,
    b: 22,
};
const PANEL_FG: Color = Color {
    r: 180,
    g: 180,
    b: 200,
};
const PANEL_HI: Color = Color {
    r: 120,
    g: 200,
    b: 255,
};

// ── layout ────────────────────────────────────────────────────────────────────

const SCREEN_W: u32 = 80;
const SCREEN_H: u32 = 24;
const MAP_W: u32 = 58; // columns 0..58 = map viewport
const PANEL_X: i32 = 59; // column 59..80 = HUD panel (1-char separator at 58)

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    const SEED: u64 = 0xB00B_5EED;
    const DUNGEON_W: u32 = 58;
    const DUNGEON_H: u32 = 24;

    // ── 1. generate dungeon ───────────────────────────────────────────────────
    let mut rng = SplitMix64::new(SEED);
    let dungeon = generate_dungeon(DUNGEON_W, DUNGEON_H, &mut rng, GenParams::default());
    let grid = PassabilityGrid::from_dungeon(&dungeon);

    // ── 2. influence map ──────────────────────────────────────────────────────
    let mut inf = InfluenceMap::new(DUNGEON_W as i32, DUNGEON_H as i32);

    // First room: player start — skip.
    // Odd rooms: monsters (positive danger influence).
    // Even rooms (after first): traps (negative — repellent).
    for (i, room) in dungeon.rooms.iter().enumerate() {
        let (cx, cy) = room.center();
        if i == 0 {
            continue; // player room: no influence source
        }
        if i % 2 == 1 {
            // Monster: +800 influence, radius 10.
            inf.add_source(cx, cy, 800, 10);
        } else {
            // Trap: -400 influence, radius 6.
            inf.add_source(cx, cy, -400, 6);
        }
    }

    // ── 3. statistics ─────────────────────────────────────────────────────────
    let max_inf = inf
        .iter()
        .map(|(_, _, v)| v.abs())
        .max()
        .unwrap_or(1)
        .max(1);
    let positive_cells = inf.iter().filter(|(_, _, v)| *v > 0).count();
    let negative_cells = inf.iter().filter(|(_, _, v)| *v < 0).count();
    let monster_count =
        dungeon.rooms.len().saturating_sub(1) / 2 + dungeon.rooms.len().saturating_sub(1) % 2;
    let trap_count = dungeon.rooms.len().saturating_sub(1) / 2;

    // ── 4. player position + highest-neighbour steering ───────────────────────
    let player_pos = dungeon.rooms.first().map(|r| r.center()).unwrap_or((1, 1));
    let steer = inf.highest_neighbour(player_pos.0, player_pos.1);
    let flee = inf.lowest_neighbour(player_pos.0, player_pos.1);

    // ── 5. render ─────────────────────────────────────────────────────────────
    let mut screen = Screen::new(SCREEN_W, SCREEN_H);

    // Camera centered on the dungeon.
    let cam = Camera::new(
        (DUNGEON_W / 2) as i32,
        (DUNGEON_H / 2) as i32,
        MAP_W,
        SCREEN_H,
        DUNGEON_W,
        DUNGEON_H,
    );

    // Map viewport.
    for sy in 0..SCREEN_H as i32 {
        for sx in 0..MAP_W as i32 {
            let (wx, wy) = cam.screen_to_world(sx as u32, sy as u32);
            let blocked = grid.is_blocked(wx, wy);
            if blocked {
                screen.set(sx, sy, '#', WALL_FG, WALL_BG);
            } else {
                let v = inf.get(wx, wy).unwrap_or(0);
                let bg = heat_colour(v, max_inf);
                screen.set(sx, sy, '.', WALL_FG, bg);
            }
        }
    }

    // Draw sources.
    for (i, room) in dungeon.rooms.iter().enumerate() {
        let (cx, cy) = room.center();
        if let Some((sx, sy)) = cam.world_to_screen(cx, cy) {
            if i == 0 {
                screen.set(sx as i32, sy as i32, '@', PLAYER_FG, WALL_BG);
            } else if i % 2 == 1 {
                screen.set(
                    sx as i32,
                    sy as i32,
                    'M',
                    Color {
                        r: 240,
                        g: 80,
                        b: 80,
                    },
                    WALL_BG,
                );
            } else {
                screen.set(
                    sx as i32,
                    sy as i32,
                    'T',
                    Color {
                        r: 80,
                        g: 160,
                        b: 240,
                    },
                    WALL_BG,
                );
            }
        }
    }

    // Steering arrows at player position.
    if let Some((sx, sy)) = cam.world_to_screen(player_pos.0, player_pos.1) {
        if let Some((dx, dy, _)) = steer {
            let arrow = match (dx, dy) {
                (1, 0) => '→',
                (-1, 0) => '←',
                (0, 1) => '↓',
                (0, -1) => '↑',
                (1, 1) => '↘',
                (-1, 1) => '↙',
                (1, -1) => '↗',
                (-1, -1) => '↖',
                _ => '·',
            };
            let nx = sx as i32 + dx;
            let ny = sy as i32 + dy;
            if nx >= 0 && ny >= 0 {
                screen.set(
                    nx,
                    ny,
                    arrow,
                    Color {
                        r: 240,
                        g: 220,
                        b: 60,
                    },
                    WALL_BG,
                );
            }
        }
    }

    // Separator column.
    for y in 0..SCREEN_H as i32 {
        screen.set(MAP_W as i32, y, '│', PANEL_FG, PANEL_BG);
    }

    // HUD panel.
    let panel = HudPanel::new(PANEL_X, 0, SCREEN_W - PANEL_X as u32, SCREEN_H);
    for y in panel.inner_y()..panel.inner_y() + panel.inner_h() as i32 {
        screen.fill_rect(
            panel.inner_x(),
            y,
            panel.inner_w(),
            1,
            Cell {
                glyph: ' ',
                fg: PANEL_FG,
                bg: PANEL_BG,
            },
        );
    }

    let mut py = panel.inner_y();
    let px = panel.inner_x();
    let pw = panel.inner_w() as usize;

    let draw_line = |screen: &mut Screen, y: i32, text: &str, fg: Color| {
        screen.draw_str(px, y, &format!("{:width$}", text, width = pw), fg, PANEL_BG);
    };

    draw_line(&mut screen, py, "izanagi_kit", PANEL_HI);
    py += 1;
    draw_line(&mut screen, py, "influence demo", PANEL_FG);
    py += 2;

    draw_line(
        &mut screen,
        py,
        &format!("Rooms: {}", dungeon.rooms.len()),
        PANEL_FG,
    );
    py += 1;
    draw_line(
        &mut screen,
        py,
        &format!("@ Player pos: {:?}", player_pos),
        PANEL_FG,
    );
    py += 1;
    draw_line(
        &mut screen,
        py,
        &format!("M Monsters: {}", monster_count),
        Color {
            r: 240,
            g: 80,
            b: 80,
        },
    );
    py += 1;
    draw_line(
        &mut screen,
        py,
        &format!("T Traps: {}", trap_count),
        Color {
            r: 80,
            g: 160,
            b: 240,
        },
    );
    py += 2;

    // Influence bar: fraction of cells under positive influence.
    let total_cells = (DUNGEON_W * DUNGEON_H) as usize;
    let coverage = positive_cells * 100 / total_cells.max(1);
    let bar = BarWidget::new(coverage as i32, 100, pw as u32);
    draw_line(&mut screen, py, "Danger zone:", PANEL_FG);
    py += 1;
    draw_line(
        &mut screen,
        py,
        &bar.render(),
        Color {
            r: 220,
            g: 80,
            b: 80,
        },
    );
    py += 1;
    draw_line(
        &mut screen,
        py,
        &format!("  {:>3}% of floor", coverage),
        PANEL_FG,
    );
    py += 2;

    let neg_bar = BarWidget::new(negative_cells as i32, total_cells as i32 / 4, pw as u32);
    draw_line(&mut screen, py, "Trap zones:", PANEL_FG);
    py += 1;
    draw_line(
        &mut screen,
        py,
        &neg_bar.render(),
        Color {
            r: 80,
            g: 140,
            b: 220,
        },
    );
    py += 2;

    // Steering info.
    let steer_str = match &steer {
        Some((dx, dy, v)) => format!("→ ({:+},{:+}) v={}", dx, dy, v),
        None => "(none)".to_string(),
    };
    let flee_str = match &flee {
        Some((dx, dy, v)) => format!("← ({:+},{:+}) v={}", dx, dy, v),
        None => "(none)".to_string(),
    };
    draw_line(&mut screen, py, "Chase:", PANEL_FG);
    py += 1;
    draw_line(
        &mut screen,
        py,
        &steer_str,
        Color {
            r: 220,
            g: 160,
            b: 60,
        },
    );
    py += 1;
    draw_line(&mut screen, py, "Flee:", PANEL_FG);
    py += 1;
    draw_line(
        &mut screen,
        py,
        &flee_str,
        Color {
            r: 80,
            g: 200,
            b: 120,
        },
    );
    py += 2;

    let stat = StatLine::new("max_inf", max_inf);
    draw_line(&mut screen, py, &stat.render(), PANEL_FG);

    screen.present();

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(b"\x1b[2J");
    let _ = out.write_all(screen.to_ansi().as_bytes());
    let _ = out.write_all(b"\r\n");
    let _ = out.flush();

    eprintln!(
        "\nInfluence demo.  seed={:#018x}  rooms={}  max_inf={}",
        SEED,
        dungeon.rooms.len(),
        max_inf,
    );
}
