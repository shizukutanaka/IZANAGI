//! Auto-tiling demo: `compute_mask` / `compute_all` / `SimpleTileTable`.
//!
//! Bitmask auto-tiling picks a wall glyph for each cell from which of its 8
//! neighbours share its terrain. Here a small dungeon of `#` walls is turned
//! into connected box-drawing lines purely from each cell's 8-bit neighbour
//! mask — no hand-authored tile placement.
//!
//! Modules exercised:
//! - `compute_all(w, h, is_same)` — full-map mask pass in row-major order.
//! - `compute_mask(x, y, is_same)` — single-cell mask (used in the legend).
//! - `SimpleTileTable` — maps each 8-bit mask to a glyph-index (`u32`), with
//!   the diagonal-corner-clearing rule handled by `compute_mask`.
//!
//! The left panel shows the raw `#` map; the right panel shows the same map
//! auto-tiled into connected walls. A legend maps the four cardinal bits to
//! their box-drawing glyph.
//!
//! Run with `cargo run --example autotile_demo`.

use izanagi_kit::content::Color;
use izanagi_kit::{compute_all, compute_mask, Cell, Screen, SimpleTileTable};
use std::io::{self, Write};

// ── layout ────────────────────────────────────────────────────────────────────

const SCREEN_W: u32 = 80;
const SCREEN_H: u32 = 24;

const MAP_W: i32 = 26;
const MAP_H: i32 = 14;
const RAW_X: i32 = 2; // raw map top-left
const TILE_X: i32 = 38; // auto-tiled map top-left
const MAP_Y: i32 = 4;

// ── palette ───────────────────────────────────────────────────────────────────

const BG: Color = Color { r: 10, g: 8, b: 18 };
const TITLE_BG: Color = Color {
    r: 24,
    g: 14,
    b: 58,
};
const TITLE_FG: Color = Color {
    r: 200,
    g: 200,
    b: 255,
};
const HDR_FG: Color = Color {
    r: 120,
    g: 110,
    b: 175,
};
const RAW_FG: Color = Color {
    r: 110,
    g: 96,
    b: 140,
};
const FLOOR_FG: Color = Color {
    r: 40,
    g: 36,
    b: 60,
};
const WALL_FG: Color = Color {
    r: 130,
    g: 200,
    b: 255,
};
const LEGEND_FG: Color = Color {
    r: 150,
    g: 200,
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

// ── the dungeon (1 = wall, 0 = floor) ───────────────────────────────────────────

#[rustfmt::skip]
const MAP: [&str; MAP_H as usize] = [
    "##########################",
    "#........#...............#",
    "#........#....######.....#",
    "#........#....#....#.....#",
    "#####.####....#....#.....#",
    "#...........###....######.#",
    "#...........#............#",
    "#...........#....#####...#",
    "#####.#######....#...#...#",
    "#................#...#...#",
    "#......####......#####...#",
    "#......#..#..............#",
    "#......#..#..............#",
    "##########################",
];

fn is_wall(x: i32, y: i32) -> bool {
    if x < 0 || y < 0 || x >= MAP_W || y >= MAP_H {
        return false; // out of bounds = different terrain (open edge)
    }
    MAP[y as usize].as_bytes()[x as usize] == b'#'
}

// ── glyph table ─────────────────────────────────────────────────────────────────
//
// We only need the four cardinal connections (N, E, S, W) to choose a
// box-drawing glyph, so the SimpleTileTable maps each 8-bit mask down to a
// 0..16 cardinal index, and `CARDINAL_GLYPHS` indexes that.
//
// cardinal index bits: N=1, E=2, S=4, W=8
const CARDINAL_GLYPHS: [char; 16] = [
    '·', // 0000  isolated
    '╵', // 0001  N
    '╶', // 0010  E
    '└', // 0011  N+E
    '╷', // 0100  S
    '│', // 0101  N+S
    '┌', // 0110  E+S
    '├', // 0111  N+E+S
    '╴', // 1000  W
    '┘', // 1001  N+W
    '─', // 1010  E+W
    '┴', // 1011  N+E+W
    '┐', // 1100  S+W
    '┤', // 1101  N+S+W
    '┬', // 1110  E+S+W
    '┼', // 1111  N+E+S+W
];

/// Reduce a full 8-bit auto-tile mask to its 4-bit cardinal index.
fn cardinal_index(mask: u8) -> u32 {
    let n = (mask & (1 << 0) != 0) as u32;
    let e = (mask & (1 << 2) != 0) as u32;
    let s = (mask & (1 << 4) != 0) as u32;
    let w = (mask & (1 << 6) != 0) as u32;
    n | (e << 1) | (s << 2) | (w << 3)
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    // Build the mask → cardinal-index lookup once (all 256 masks).
    let mut table = SimpleTileTable::new();
    for m in 0..=255u16 {
        table.set(m as u8, cardinal_index(m as u8));
    }

    // Compute the full mask map in one pass.
    let masks = compute_all(MAP_W, MAP_H, is_wall);

    // ── render ────────────────────────────────────────────────────────────────
    let mut screen = Screen::new(SCREEN_W, SCREEN_H);
    screen.clear(Cell {
        glyph: ' ',
        fg: STAT_FG,
        bg: BG,
    });

    // Title bar.
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
        " AUTO-TILING   compute_mask / compute_all / SimpleTileTable",
        TITLE_FG,
        TITLE_BG,
    );

    // Column headers.
    screen.draw_str(RAW_X, 2, "RAW MAP (# = wall)", HDR_FG, BG);
    screen.draw_str(TILE_X, 2, "AUTO-TILED (8-bit neighbour mask)", HDR_FG, BG);
    for x in 0..SCREEN_W as i32 {
        screen.set(x, 3, '─', HDR_FG, BG);
    }

    // Both maps.
    for y in 0..MAP_H {
        for x in 0..MAP_W {
            let wall = is_wall(x, y);

            // Raw panel.
            let (raw_g, raw_fg) = if wall { ('#', RAW_FG) } else { ('.', FLOOR_FG) };
            screen.set(RAW_X + x, MAP_Y + y, raw_g, raw_fg, BG);

            // Auto-tiled panel.
            let (tile_g, tile_fg) = if wall {
                let mask = masks[(y * MAP_W + x) as usize];
                let idx = table.get(mask) as usize;
                (CARDINAL_GLYPHS[idx], WALL_FG)
            } else {
                ('·', FLOOR_FG)
            };
            screen.set(TILE_X + x, MAP_Y + y, tile_g, tile_fg, BG);
        }
    }

    // ── legend ──────────────────────────────────────────────────────────────────
    let legend_y = MAP_Y + MAP_H + 1;
    screen.draw_str(RAW_X, legend_y, "Cardinal-bit glyphs:", HDR_FG, BG);
    let legend: &[(&str, char)] = &[
        ("none", '·'),
        ("─", '─'),
        ("│", '│'),
        ("┌", '┌'),
        ("┐", '┐'),
        ("└", '└'),
        ("┘", '┘'),
        ("├", '├'),
        ("┤", '┤'),
        ("┬", '┬'),
        ("┴", '┴'),
        ("┼", '┼'),
    ];
    let mut lx = RAW_X;
    let ly = legend_y + 1;
    for (label, glyph) in legend {
        screen.set(lx, ly, *glyph, WALL_FG, BG);
        screen.draw_str(lx + 1, ly, label, LEGEND_FG, BG);
        lx += 2 + label.chars().count() as i32 + 1;
    }

    // ── stats ─────────────────────────────────────────────────────────────────
    let wall_cells = masks
        .iter()
        .enumerate()
        .filter(|(i, _)| {
            let x = (*i as i32) % MAP_W;
            let y = (*i as i32) / MAP_W;
            is_wall(x, y)
        })
        .count();

    // Spot-check one cell with the single-cell `compute_mask`.
    let probe = (5, 4); // a wall on the room divider
    let probe_mask = compute_mask(probe.0, probe.1, is_wall);
    let probe_idx = cardinal_index(probe_mask);

    let stat_line = format!(
        " map={}×{}  wall_cells={}  masks={}  probe({},{})=0b{:08b}→'{}'",
        MAP_W,
        MAP_H,
        wall_cells,
        masks.len(),
        probe.0,
        probe.1,
        probe_mask,
        CARDINAL_GLYPHS[probe_idx as usize],
    );
    screen.draw_str(0, SCREEN_H as i32 - 1, &stat_line, STAT_HI, BG);

    // ── output ────────────────────────────────────────────────────────────────
    screen.present();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(b"\x1b[2J");
    let _ = out.write_all(screen.to_ansi().as_bytes());
    let _ = out.write_all(b"\r\n");
    let _ = out.flush();

    eprintln!(
        "\nAuto-tiling demo.\n\
         map={}×{}  wall_cells={}  total_masks={}\n\
         compute_all() produced every cell's 8-bit neighbour mask in one pass;\n\
         SimpleTileTable reduced each mask to a 4-bit cardinal index → box glyph.\n\
         probe({},{}) mask=0b{:08b} cardinal_idx={} glyph='{}'",
        MAP_W,
        MAP_H,
        wall_cells,
        masks.len(),
        probe.0,
        probe.1,
        probe_mask,
        probe_idx,
        CARDINAL_GLYPHS[probe_idx as usize],
    );
}
