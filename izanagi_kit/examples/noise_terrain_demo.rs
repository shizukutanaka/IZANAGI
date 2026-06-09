//! Procedural terrain demo using deterministic integer noise.
//!
//! Generates a 80×22 terrain map using `value_noise_2d` (fractional Brownian
//! motion — three octaves of integer noise summed and renormalised), maps the
//! height values to six biomes by threshold, and renders with 24-bit ANSI
//! colour. A two-row status bar shows the seed, octave count, and biome
//! distribution (cells per biome as ASCII bar charts).
//!
//! The `hash_2d` function is also exercised as a scatter layer (sparse
//! "feature" symbols placed at high-entropy hash positions).
//!
//! Run with `cargo run --example noise_terrain_demo`.

use izanagi_kit::content::Color;
use izanagi_kit::noise::{fbm_2d, hash_2d};
use izanagi_kit::{Cell, Screen};
use std::io::{self, Write};

// ── biome palette ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Biome {
    DeepWater,
    ShallowWater,
    Sand,
    Grass,
    Forest,
    Mountain,
    Snow,
}

impl Biome {
    fn from_height(h: u32) -> Self {
        match h {
            0..=18000 => Biome::DeepWater,
            18001..=27000 => Biome::ShallowWater,
            27001..=32000 => Biome::Sand,
            32001..=44000 => Biome::Grass,
            44001..=54000 => Biome::Forest,
            54001..=60000 => Biome::Mountain,
            _ => Biome::Snow,
        }
    }

    fn glyph(self) -> char {
        match self {
            Biome::DeepWater => '≈',
            Biome::ShallowWater => '~',
            Biome::Sand => '.',
            Biome::Grass => ',',
            Biome::Forest => 't',
            Biome::Mountain => '^',
            Biome::Snow => '*',
        }
    }

    fn fg(self) -> Color {
        match self {
            Biome::DeepWater => Color {
                r: 20,
                g: 40,
                b: 140,
            },
            Biome::ShallowWater => Color {
                r: 40,
                g: 110,
                b: 200,
            },
            Biome::Sand => Color {
                r: 210,
                g: 190,
                b: 100,
            },
            Biome::Grass => Color {
                r: 60,
                g: 180,
                b: 50,
            },
            Biome::Forest => Color {
                r: 20,
                g: 110,
                b: 20,
            },
            Biome::Mountain => Color {
                r: 160,
                g: 160,
                b: 160,
            },
            Biome::Snow => Color {
                r: 240,
                g: 240,
                b: 255,
            },
        }
    }

    fn bg(self) -> Color {
        match self {
            Biome::DeepWater => Color { r: 5, g: 15, b: 70 },
            Biome::ShallowWater => Color {
                r: 15,
                g: 50,
                b: 130,
            },
            Biome::Sand => Color {
                r: 170,
                g: 150,
                b: 60,
            },
            Biome::Grass => Color {
                r: 20,
                g: 80,
                b: 15,
            },
            Biome::Forest => Color {
                r: 10,
                g: 50,
                b: 10,
            },
            Biome::Mountain => Color {
                r: 80,
                g: 80,
                b: 80,
            },
            Biome::Snow => Color {
                r: 200,
                g: 200,
                b: 220,
            },
        }
    }

    fn name(self) -> &'static str {
        match self {
            Biome::DeepWater => "deep",
            Biome::ShallowWater => "coast",
            Biome::Sand => "sand",
            Biome::Grass => "grass",
            Biome::Forest => "forest",
            Biome::Mountain => "mount",
            Biome::Snow => "snow",
        }
    }
}

const ALL_BIOMES: [Biome; 7] = [
    Biome::DeepWater,
    Biome::ShallowWater,
    Biome::Sand,
    Biome::Grass,
    Biome::Forest,
    Biome::Mountain,
    Biome::Snow,
];

// ── noise / terrain ───────────────────────────────────────────────────────────
//
// Terrain height comes from `noise::fbm_2d` (fractional Brownian motion: octaves
// of value noise summed at doubling frequency / halving amplitude).

// ── layout ────────────────────────────────────────────────────────────────────

const SCREEN_W: u32 = 80;
const SCREEN_H: u32 = 24;
const MAP_H: u32 = 22; // rows 0..22 = terrain
const STATUS_Y: i32 = 22;
const LEGEND_Y: i32 = 23;

const UI_BG: Color = Color {
    r: 12,
    g: 12,
    b: 18,
};
const UI_FG: Color = Color {
    r: 180,
    g: 180,
    b: 180,
};
const UI_HI: Color = Color {
    r: 120,
    g: 200,
    b: 255,
};

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    const SEED: u64 = 0x0DEC_A1EA;
    const OCTAVES: u32 = 3;
    // World scale: each screen cell = 2^16 / SCALE world units.
    // Higher SCALE = more zoomed-in (finer features).
    const SCALE: u32 = 2; // each screen cell spans 1/2 of a noise grid unit

    let mut screen = Screen::new(SCREEN_W, SCREEN_H);
    let mut biome_counts = [0u32; 7];

    // ── terrain ───────────────────────────────────────────────────────────────
    for sy in 0..MAP_H as i32 {
        for sx in 0..SCREEN_W as i32 {
            // Convert screen cell to Q16.16 noise coordinate.
            let nx = (sx << 16) / SCALE as i32;
            let ny = (sy << 16) / SCALE as i32;
            let height = fbm_2d(nx, ny, SEED, OCTAVES);
            let biome = Biome::from_height(height);

            // Sparse "feature" symbols at high-entropy hash positions.
            let feature = hash_2d(sx, sy, SEED ^ 0xF0F0) >> 24;
            let glyph = if feature == 0 {
                // 1/256 chance — place a landmark
                match biome {
                    Biome::DeepWater | Biome::ShallowWater => '♦',
                    Biome::Sand => '○',
                    Biome::Grass => '❀',
                    Biome::Forest => '↟',
                    Biome::Mountain => '▲',
                    Biome::Snow => '✦',
                }
            } else {
                biome.glyph()
            };

            screen.set(sx, sy, glyph, biome.fg(), biome.bg());
            let idx = ALL_BIOMES.iter().position(|&b| b == biome).unwrap_or(0);
            biome_counts[idx] += 1;
        }
    }

    // ── status bar ────────────────────────────────────────────────────────────
    screen.fill_rect(
        0,
        STATUS_Y,
        SCREEN_W,
        1,
        Cell {
            glyph: ' ',
            fg: UI_FG,
            bg: UI_BG,
        },
    );
    let status = format!(
        " seed={:#018x}  octaves={}  scale=1/{}  cells={}×{}",
        SEED, OCTAVES, SCALE, SCREEN_W, MAP_H,
    );
    screen.draw_str(0, STATUS_Y, &status, UI_HI, UI_BG);

    // ── legend bar ────────────────────────────────────────────────────────────
    screen.fill_rect(
        0,
        LEGEND_Y,
        SCREEN_W,
        1,
        Cell {
            glyph: ' ',
            fg: UI_FG,
            bg: UI_BG,
        },
    );
    let total_cells = SCREEN_W * MAP_H;
    let mut lx = 0i32;
    for (i, &biome) in ALL_BIOMES.iter().enumerate() {
        let pct = biome_counts[i] * 100 / total_cells.max(1);
        let label = format!("{} {}% ", biome.name(), pct);
        screen.draw_str(lx, LEGEND_Y, &label, biome.fg(), UI_BG);
        lx += label.len() as i32;
    }

    screen.present();

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(b"\x1b[2J");
    let _ = out.write_all(screen.to_ansi().as_bytes());
    let _ = out.write_all(b"\r\n");
    let _ = out.flush();

    eprintln!(
        "\nNoise terrain.  seed={:#018x}  octaves={}  water={}%  land={}%",
        SEED,
        OCTAVES,
        (biome_counts[0] + biome_counts[1]) * 100 / total_cells.max(1),
        (biome_counts[2] + biome_counts[3] + biome_counts[4] + biome_counts[5] + biome_counts[6])
            * 100
            / total_cells.max(1),
    );
}
