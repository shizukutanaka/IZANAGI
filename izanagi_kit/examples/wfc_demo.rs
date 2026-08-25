//! WFC tile-generation demo rendered with the terminal module.
//!
//! Defines a simple 5-tile tileset (deep water, shallow water, sand, grass,
//! mountain), encodes adjacency constraints (you can't place mountain next to
//! water without a grass or sand transition), runs `wfc_solve`, and renders
//! the result into an 80×24 `Screen` using 24-bit ANSI colour. The auto-tile
//! mask for the "grass" layer is also computed and displayed in the status bar.
//!
//! Run with `cargo run --example wfc_demo`.

use izanagi_kit::content::Color;
use izanagi_kit::{compute_all, wfc_solve};
use izanagi_kit::{Cell, Screen, SplitMix64};
use izanagi_kit::{WfcResult, WfcRules};
use std::io::{self, Write};

// ── tileset ──────────────────────────────────────────────────────────────────

/// Tile indices used as bit positions in WFC bitmasks.
const DEEP: u8 = 0; // deep water
const SHALLOW: u8 = 1; // shallow water / coast
const SAND: u8 = 2; // beach / desert
const GRASS: u8 = 3; // grassland
const MOUNTAIN: u8 = 4; // mountain / rock
const TILE_COUNT: u8 = 5;

/// Visual appearance of each tile type.
struct TileAppearance {
    glyph: char,
    fg: Color,
    bg: Color,
}

fn appearance(tile: u8) -> TileAppearance {
    match tile {
        DEEP => TileAppearance {
            glyph: '≈',
            fg: Color {
                r: 30,
                g: 80,
                b: 180,
            },
            bg: Color {
                r: 10,
                g: 30,
                b: 100,
            },
        },
        SHALLOW => TileAppearance {
            glyph: '~',
            fg: Color {
                r: 60,
                g: 150,
                b: 220,
            },
            bg: Color {
                r: 20,
                g: 70,
                b: 160,
            },
        },
        SAND => TileAppearance {
            glyph: '.',
            fg: Color {
                r: 220,
                g: 200,
                b: 120,
            },
            bg: Color {
                r: 180,
                g: 160,
                b: 80,
            },
        },
        GRASS => TileAppearance {
            glyph: ',',
            fg: Color {
                r: 80,
                g: 200,
                b: 60,
            },
            bg: Color {
                r: 30,
                g: 100,
                b: 20,
            },
        },
        MOUNTAIN => TileAppearance {
            glyph: '^',
            fg: Color {
                r: 200,
                g: 200,
                b: 200,
            },
            bg: Color {
                r: 100,
                g: 100,
                b: 100,
            },
        },
        _ => TileAppearance {
            glyph: '?',
            fg: Color {
                r: 255,
                g: 0,
                b: 255,
            },
            bg: Color { r: 0, g: 0, b: 0 },
        },
    }
}

// ── adjacency rules ───────────────────────────────────────────────────────────

/// Encodes a natural terrain transition gradient:
///   deep ↔ shallow ↔ sand ↔ grass ↔ mountain
/// Each tier may be adjacent to itself and its immediate neighbours.
/// Long-range adjacencies (e.g. deep ↔ mountain) are forbidden.
fn build_rules() -> WfcRules {
    let mut r = WfcRules::new(TILE_COUNT);

    // Allowed same-direction adjacencies for each tile (symmetric).
    let pairs: &[(u8, u8)] = &[
        (DEEP, DEEP),
        (DEEP, SHALLOW),
        (SHALLOW, SHALLOW),
        (SHALLOW, SAND),
        (SAND, SAND),
        (SAND, GRASS),
        (GRASS, GRASS),
        (GRASS, MOUNTAIN),
        (MOUNTAIN, MOUNTAIN),
    ];

    for &(a, b) in pairs {
        for dir in 0..4 {
            r.allow_symmetric(a, dir, b);
        }
    }
    r
}

// ── rendering ─────────────────────────────────────────────────────────────────

const SCREEN_W: u32 = 80;
const SCREEN_H: u32 = 24;
const MAP_W: u32 = SCREEN_W;
const MAP_H: u32 = 22; // rows 0..22

fn render_map(grid: &izanagi_kit::WfcGrid, screen: &mut Screen) {
    let w = grid.width as u32;
    let h = grid.height as u32;

    for sy in 0..MAP_H {
        for sx in 0..MAP_W {
            // Map screen cell → grid cell (scale to grid dimensions).
            let gx = (sx * w / MAP_W) as i32;
            let gy = (sy * h / MAP_H) as i32;
            let cell = match grid.tile_at(gx, gy) {
                Some(t) => {
                    let a = appearance(t);
                    Cell {
                        glyph: a.glyph,
                        fg: a.fg,
                        bg: a.bg,
                    }
                }
                None => Cell {
                    glyph: '?',
                    fg: Color {
                        r: 255,
                        g: 0,
                        b: 255,
                    },
                    bg: Color { r: 0, g: 0, b: 0 },
                },
            };
            screen.put(sx as i32, sy as i32, cell);
        }
    }
}

fn render_status(
    screen: &mut Screen,
    seed: u64,
    grid_w: i32,
    grid_h: i32,
    grass_cells: usize,
    autotile_nonzero: usize,
) {
    let bg = Color {
        r: 15,
        g: 15,
        b: 15,
    };
    let fg = Color {
        r: 200,
        g: 200,
        b: 200,
    };
    let hi = Color {
        r: 120,
        g: 200,
        b: 120,
    };

    screen.fill_rect(0, 22, SCREEN_W, 1, Cell { glyph: ' ', fg, bg });
    screen.fill_rect(0, 23, SCREEN_W, 1, Cell { glyph: ' ', fg, bg });

    screen.draw_str(
        0,
        22,
        &format!(
            " seed={:#018x}  grid={}×{}  grass={}  autotile variants>0={}",
            seed, grid_w, grid_h, grass_cells, autotile_nonzero
        ),
        hi,
        bg,
    );

    // Legend
    let legend = [
        (DEEP, "deep"),
        (SHALLOW, "~coast"),
        (SAND, ".sand"),
        (GRASS, ",grass"),
        (MOUNTAIN, "^mount"),
    ];
    let mut x = 1i32;
    for (tile, label) in &legend {
        let a = appearance(*tile);
        screen.set(x, 23, a.glyph, a.fg, a.bg);
        x += 1;
        screen.draw_str(x, 23, label, fg, bg);
        x += label.len() as i32 + 2;
    }
}

// ── entry point ───────────────────────────────────────────────────────────────

fn main() {
    const SEED: u64 = 0x4C01_1234;
    const GRID_W: i32 = 80;
    const GRID_H: i32 = 44;

    let rules = build_rules();
    let mut rng = SplitMix64::new(SEED);

    // Retry up to 5 times on contradiction (rare with these rules).
    let grid = loop {
        let mut rng2 = SplitMix64::new(rng.state());
        rng.below(1); // advance to produce a different seed next attempt
        match wfc_solve(GRID_W, GRID_H, &rules, &mut rng2) {
            WfcResult::Ok(g) => break g,
            WfcResult::Contradiction => continue,
        }
    };

    // Compute autotile masks for the grass layer.
    let masks = compute_all(GRID_W, GRID_H, |x, y| grid.tile_at(x, y) == Some(GRASS));
    let grass_total = (0..GRID_H)
        .flat_map(|y| (0..GRID_W).map(move |x| (x, y)))
        .filter(|&(x, y)| grid.tile_at(x, y) == Some(GRASS))
        .count();
    let autotile_nonzero = masks.iter().filter(|&&m| m > 0).count();

    let mut screen = Screen::new(SCREEN_W, SCREEN_H);
    render_map(&grid, &mut screen);
    render_status(
        &mut screen,
        SEED,
        GRID_W,
        GRID_H,
        grass_total,
        autotile_nonzero,
    );
    screen.present();

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(b"\x1b[2J");
    let _ = out.write_all(screen.to_ansi().as_bytes());
    let _ = out.write_all(b"\r\n");
    let _ = out.flush();

    eprintln!(
        "\nWFC demo complete.  seed={:#018x}  {}×{}  cells={}  fully_collapsed={}",
        SEED,
        GRID_W,
        GRID_H,
        grid.len(),
        grid.is_fully_collapsed(),
    );
}
