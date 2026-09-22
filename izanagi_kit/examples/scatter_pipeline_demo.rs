//! Scatter → territory → connectivity → wire packet demo.
//!
//! The full procgen placement pipeline in four deterministic stages:
//!   1. `mapgen::poisson_disc` scatters seeds with a minimum separation
//!      (Bridson's algorithm — the "blue noise" placement primitive).
//!   2. `voronoi::voronoi_partition` assigns every cell to its nearest seed —
//!      territories, rendered here as coloured letter regions.
//!   3. `voronoi::mst_edges` wires the seeds into a minimum spanning tree —
//!      the TinyKeep-style "connect the scattered sites cheaply" backbone.
//!   4. `bits::BitWriter` packs the whole result — seed count, coordinates,
//!      edge list — into a lockstep-style bit packet, hexdumped on the right.
//!
//! Run with `cargo run --example scatter_pipeline_demo`.

use izanagi_kit::bits::BitWriter;
use izanagi_kit::content::Color;
use izanagi_kit::geometry::{line, Distance};
use izanagi_kit::mapgen::poisson_disc;
use izanagi_kit::rng::SplitMix64;
use izanagi_kit::voronoi::{mst_edges, voronoi_partition};
use izanagi_kit::{Cell, Screen};
use std::io::{self, Write};

// ── layout ────────────────────────────────────────────────────────────────────

const SCREEN_W: u32 = 80;
const SCREEN_H: u32 = 24;
const MAP_W: i32 = 48;
const MAP_H: i32 = 20;
const MAP_X: i32 = 1;
const MAP_Y: i32 = 2;
const PANEL_X: i32 = 52;
const SEED: u64 = 0x5CA7_7E12;
const RADIUS: i32 = 9;

// ── palette ───────────────────────────────────────────────────────────────────

const BG: Color = Color { r: 8, g: 8, b: 14 };
const TITLE_BG: Color = Color {
    r: 18,
    g: 22,
    b: 52,
};
const TITLE_FG: Color = Color {
    r: 200,
    g: 210,
    b: 255,
};
const PANEL_FG: Color = Color {
    r: 190,
    g: 190,
    b: 210,
};
const DIM_FG: Color = Color {
    r: 110,
    g: 110,
    b: 140,
};
const EDGE_FG: Color = Color {
    r: 255,
    g: 220,
    b: 90,
};
const SEED_FG: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

const REGION_COLORS: [Color; 8] = [
    Color {
        r: 90,
        g: 130,
        b: 200,
    },
    Color {
        r: 200,
        g: 130,
        b: 90,
    },
    Color {
        r: 110,
        g: 190,
        b: 110,
    },
    Color {
        r: 190,
        g: 100,
        b: 170,
    },
    Color {
        r: 100,
        g: 190,
        b: 190,
    },
    Color {
        r: 210,
        g: 180,
        b: 80,
    },
    Color {
        r: 150,
        g: 120,
        b: 220,
    },
    Color {
        r: 170,
        g: 170,
        b: 170,
    },
];
// Region glyphs derive from the seed index ('a', 'b', …) so every seed gets a
// unique letter for up to 26 regions; colours cycle through the 8-entry
// palette above.
fn region_glyph(i: usize) -> char {
    (b'a' + (i % 26) as u8) as char
}

fn main() {
    // ── 1. scatter ────────────────────────────────────────────────────────────
    let mut rng = SplitMix64::new(SEED);
    let seeds = poisson_disc(MAP_W as u32, MAP_H as u32, RADIUS, &mut rng, 30);

    // ── 2. territory ──────────────────────────────────────────────────────────
    let grid = voronoi_partition(&seeds, MAP_W, MAP_H, Distance::EuclideanSquared);

    // ── 3. connectivity ───────────────────────────────────────────────────────
    let edges = mst_edges(&seeds);

    // ── 4. wire packet ─────────────────────────────────────────────────────
    let mut w = BitWriter::new();
    w.write_varint(seeds.len() as u64);
    for &(x, y) in &seeds {
        w.write_zigzag(x as i64);
        w.write_zigzag(y as i64);
    }
    w.write_varint(edges.len() as u64);
    for &(i, j) in &edges {
        w.write_ranged(i as i64, 0, 255);
        w.write_ranged(j as i64, 0, 255);
    }
    let packet = w.bytes();

    // ── render ────────────────────────────────────────────────────────────────
    let mut screen = Screen::new(SCREEN_W, SCREEN_H);
    screen.clear(Cell {
        glyph: ' ',
        fg: PANEL_FG,
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
        " SCATTER PIPELINE   poisson_disc → voronoi → MST → BitWriter",
        TITLE_FG,
        TITLE_BG,
    );

    // Region fills.
    for y in 0..MAP_H {
        for x in 0..MAP_W {
            if let Some(owner) = grid.get(x, y) {
                let i = owner as usize;
                screen.set(
                    MAP_X + x,
                    MAP_Y + y,
                    region_glyph(i),
                    REGION_COLORS[i % REGION_COLORS.len()],
                    BG,
                );
            }
        }
    }
    // MST edges: dim lines between seed centres (the dungeon's corridor plan).
    for &(i, j) in &edges {
        for (x, y) in line(seeds[i as usize], seeds[j as usize]) {
            screen.set(MAP_X + x, MAP_Y + y, '·', EDGE_FG, BG);
        }
    }
    // Seeds on top.
    for &(x, y) in &seeds {
        screen.set(MAP_X + x, MAP_Y + y, '◆', SEED_FG, BG);
    }

    // ── side panel ────────────────────────────────────────────────────────────
    screen.draw_str(PANEL_X, 2, "seeds", PANEL_FG, BG);
    screen.draw_str(PANEL_X + 7, 2, &format!("{}", seeds.len()), SEED_FG, BG);
    screen.draw_str(PANEL_X, 3, "radius", PANEL_FG, BG);
    screen.draw_str(PANEL_X + 7, 3, &format!("{RADIUS}"), PANEL_FG, BG);

    screen.draw_str(PANEL_X, 5, "region sizes:", PANEL_FG, BG);
    let sizes = grid.region_sizes();
    // Three entries per row keeps every region visible inside the 80x24 frame.
    for (i, &s) in sizes.iter().enumerate() {
        let c = REGION_COLORS[i % REGION_COLORS.len()];
        let col = PANEL_X + 1 + (i as i32 % 3) * 9;
        let row = 6 + i as i32 / 3;
        screen.set(col, row, region_glyph(i), c, BG);
        screen.draw_str(col + 1, row, &format!(" {s:>4}"), PANEL_FG, BG);
    }

    let edge_y = 7 + sizes.len().div_ceil(3) as i32;
    screen.draw_str(
        PANEL_X,
        edge_y,
        &format!("MST edges: {}", edges.len()),
        PANEL_FG,
        BG,
    );
    let edge_str: Vec<String> = edges.iter().map(|&(i, j)| format!("{i}-{j}")).collect();
    for (row, chunk) in edge_str.chunks(5).enumerate() {
        screen.draw_str(
            PANEL_X,
            edge_y + 1 + row as i32,
            &chunk.join(" "),
            DIM_FG,
            BG,
        );
    }

    let packet_y = edge_y + 1 + edge_str.len().div_ceil(5) as i32 + 1;
    screen.draw_str(
        PANEL_X,
        packet_y,
        &format!("packet: {} bits / {} bytes", w.bit_len(), packet.len()),
        PANEL_FG,
        BG,
    );
    for (row, chunk) in packet.chunks(8).enumerate() {
        let hex: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
        screen.draw_str(
            PANEL_X,
            packet_y + 1 + row as i32,
            &hex.join(" "),
            DIM_FG,
            BG,
        );
    }

    // ── output ────────────────────────────────────────────────────────────────
    screen.present();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(b"\x1b[2J");
    let _ = out.write_all(screen.to_ansi().as_bytes());
    let _ = out.write_all(b"\r\n");
    let _ = out.flush();

    eprintln!(
        "\nScatter pipeline demo.\n\
         seeds={}  radius={}  mst_edges={}  packet={} bytes ({} bits)\n\
         Every stage is deterministic under SEED={SEED:#x}: same seed →\n\
         byte-identical map, edge set, and wire packet.",
        seeds.len(),
        RADIUS,
        edges.len(),
        packet.len(),
        w.bit_len(),
    );
}
