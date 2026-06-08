//! Multi-floor dungeon demo: `MultiMap` + `Connector`.
//!
//! Generates a three-floor dungeon stack with deterministic `SplitMix64` seeds,
//! wires stair connectors between consecutive floors (down-stairs on floor N
//! lead to up-stairs on floor N+1), and renders all three floors side by side.
//!
//! Modules exercised:
//! - `generate_dungeon` — one `Dungeon` per floor (deterministic per seed).
//! - `MultiMap::new` / `floor` / `current` / `set_floor` — the floor stack.
//! - `Connector` + `add_connector` / `exits_from` / `connector_at` — inter-floor
//!   stairs. The `>` (descend) and `<` (ascend) glyphs are placed from the
//!   connector table, not hard-coded into the map.
//!
//! The active floor (index 1) is highlighted; stair cells are coloured.
//!
//! Run with `cargo run --example multimap_demo`.

use izanagi_kit::content::Color;
use izanagi_kit::mapgen::{generate_dungeon, Dungeon, GenParams};
use izanagi_kit::multimap::Connector;
use izanagi_kit::rng::SplitMix64;
use izanagi_kit::{Cell, MultiMap, Screen};
use std::io::{self, Write};

// ── layout ────────────────────────────────────────────────────────────────────

const SCREEN_W: u32 = 80;
const SCREEN_H: u32 = 24;

const FLOOR_W: u32 = 24;
const FLOOR_H: u32 = 16;
const N_FLOORS: usize = 3;
const PANEL_GAP: i32 = 2;
const MAP_Y: i32 = 3;
const ACTIVE_FLOOR: u32 = 1;

// ── palette ───────────────────────────────────────────────────────────────────

const BG: Color = Color { r: 8, g: 7, b: 16 };
const TITLE_BG: Color = Color {
    r: 20,
    g: 14,
    b: 52,
};
const TITLE_FG: Color = Color {
    r: 200,
    g: 200,
    b: 255,
};
const HDR_FG: Color = Color {
    r: 120,
    g: 112,
    b: 172,
};
const HDR_ACTIVE: Color = Color {
    r: 255,
    g: 220,
    b: 110,
};
const WALL_FG: Color = Color {
    r: 70,
    g: 64,
    b: 96,
};
const WALL_DIM: Color = Color {
    r: 38,
    g: 34,
    b: 54,
};
const FLOOR_FG: Color = Color {
    r: 110,
    g: 120,
    b: 150,
};
const FLOOR_DIM: Color = Color {
    r: 48,
    g: 50,
    b: 70,
};
const DOWN_FG: Color = Color {
    r: 255,
    g: 150,
    b: 70,
};
const UP_FG: Color = Color {
    r: 110,
    g: 210,
    b: 255,
};
const STAT_FG: Color = Color {
    r: 140,
    g: 195,
    b: 255,
};
const STAT_HI: Color = Color {
    r: 255,
    g: 218,
    b: 100,
};

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    // Generate three deterministic floors.
    let params = GenParams {
        max_rooms: 6,
        min_room: 4,
        max_room: 7,
    };
    let floors: Vec<Dungeon> = (0..N_FLOORS)
        .map(|i| {
            let mut rng = SplitMix64::new(0xACE1 + i as u64 * 0x9E37);
            generate_dungeon(FLOOR_W, FLOOR_H, &mut rng, params)
        })
        .collect();

    let mut mmap = MultiMap::new(floors, ACTIVE_FLOOR);

    // Wire stairs: down-stair on floor i (last room centre) → up-stair on
    // floor i+1 (first room centre). Each is a single Connector record.
    for i in 0..(N_FLOORS - 1) {
        let from = mmap.floor(i).unwrap();
        let to = mmap.floor(i + 1).unwrap();
        // Pick deterministic in-bounds room centres; fall back to mid-map.
        let (fx, fy) = from
            .rooms
            .last()
            .map(|r| r.center())
            .unwrap_or((FLOOR_W as i32 / 2, FLOOR_H as i32 / 2));
        let (tx, ty) = to
            .rooms
            .first()
            .map(|r| r.center())
            .unwrap_or((FLOOR_W as i32 / 2, FLOOR_H as i32 / 2));
        mmap.add_connector(Connector {
            from_floor: i as u32,
            from_x: fx,
            from_y: fy,
            to_floor: (i + 1) as u32,
            to_x: tx,
            to_y: ty,
        });
    }

    // Navigation demo: jump to the active floor (also clamps internally).
    mmap.set_floor(ACTIVE_FLOOR);

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
        " MULTI-FLOOR DUNGEON   MultiMap + Connector stairs",
        TITLE_FG,
        TITLE_BG,
    );

    let panel_w = FLOOR_W as i32 + PANEL_GAP;
    for fi in 0..N_FLOORS {
        let ox = 1 + fi as i32 * panel_w;
        let is_active = fi as u32 == mmap.current_floor();

        // Floor header.
        let hdr = if is_active {
            format!("▶ Floor {} ◀", fi)
        } else {
            format!("  Floor {}", fi)
        };
        let hfg = if is_active { HDR_ACTIVE } else { HDR_FG };
        screen.draw_str(ox, MAP_Y - 1, &hdr, hfg, BG);

        let dungeon = mmap.floor(fi).unwrap();
        let (wall_c, floor_c) = if is_active {
            (WALL_FG, FLOOR_FG)
        } else {
            (WALL_DIM, FLOOR_DIM)
        };

        for y in 0..FLOOR_H as i32 {
            for x in 0..FLOOR_W as i32 {
                let (g, fg) = if dungeon.is_wall(x, y) {
                    ('#', wall_c)
                } else {
                    ('·', floor_c)
                };
                screen.set(ox + x, MAP_Y + y, g, fg, BG);
            }
        }

        // Overlay stairs from the connector table.
        // Down-stairs originate on this floor:
        for c in mmap.exits_from(fi as u32) {
            screen.set(ox + c.from_x, MAP_Y + c.from_y, '>', DOWN_FG, BG);
        }
        // Up-stairs: any connector that lands on this floor.
        for src in 0..N_FLOORS as u32 {
            for c in mmap.exits_from(src) {
                if c.to_floor == fi as u32 {
                    screen.set(ox + c.to_x, MAP_Y + c.to_y, '<', UP_FG, BG);
                }
            }
        }
    }

    // ── stats ─────────────────────────────────────────────────────────────────
    let total_exits: usize = (0..N_FLOORS as u32).map(|f| mmap.exits_from(f).len()).sum();

    // Probe connector_at on the floor-0 down-stair.
    let probe = mmap
        .exits_from(0)
        .first()
        .map(|c| (c.from_x, c.from_y))
        .unwrap_or((0, 0));
    let probe_hit = mmap.connector_at(0, probe.0, probe.1).is_some();

    let legend_y = MAP_Y + FLOOR_H as i32 + 1;
    screen.draw_str(1, legend_y, "Legend:", HDR_FG, BG);
    screen.set(9, legend_y, '>', DOWN_FG, BG);
    screen.draw_str(10, legend_y, " descend", STAT_FG, BG);
    screen.set(20, legend_y, '<', UP_FG, BG);
    screen.draw_str(21, legend_y, " ascend", STAT_FG, BG);
    screen.draw_str(31, legend_y, "  active floor brightened", HDR_FG, BG);

    let stat = format!(
        " floors={}  active={}  connectors={}  probe connector_at(0,{},{})={}",
        mmap.floor_count(),
        mmap.current_floor(),
        total_exits,
        probe.0,
        probe.1,
        probe_hit,
    );
    screen.draw_str(0, SCREEN_H as i32 - 1, &stat, STAT_HI, BG);

    // ── output ────────────────────────────────────────────────────────────────
    screen.present();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(b"\x1b[2J");
    let _ = out.write_all(screen.to_ansi().as_bytes());
    let _ = out.write_all(b"\r\n");
    let _ = out.flush();

    eprintln!(
        "\nMulti-floor dungeon demo.\n\
         floors={}  active_floor={}  connectors={}\n\
         Each floor is a deterministic generate_dungeon; stairs are Connector\n\
         records (down-stair on floor N → up-stair on floor N+1).\n\
         connector_at(0,{},{}) resolved a stair: {}",
        mmap.floor_count(),
        mmap.current_floor(),
        total_exits,
        probe.0,
        probe.1,
        probe_hit,
    );
}
