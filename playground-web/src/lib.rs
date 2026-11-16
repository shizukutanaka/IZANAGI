//! IZANAGI playground (web) — real `izanagi_kit` samples, compiled to
//! WebAssembly and run in the browser.
//!
//! The samples are the kit's own demos, re-driven through the same public
//! APIs the terminal examples use (`izanagi_kit/examples/noise_terrain_demo.rs`,
//! `wfc_demo.rs`, and the sim half of `izanagi/examples/kit_bridge.rs`).
//! Nothing is ported or reimplemented in JavaScript: the page only hands a
//! sample id and a seed across the wasm boundary and paints the returned
//! [`Screen`] cells. Because the kit is integer-only and seed-driven, the
//! frame a visitor sees is bit-identical to what the same code produces
//! natively — the dungeon sample prints its world hash so that claim can
//! be checked, not just believed.
//!
//! The wasm ABI is deliberately hand-rolled (no wasm-bindgen): three
//! exported functions, one static output buffer, one documented byte
//! format. See `README.md` for the frame format.

use std::cell::RefCell;

use izanagi_kit::content::Color;
use izanagi_kit::fov::compute_fov;
use izanagi_kit::mapgen::{generate_dungeon, Dungeon, GenParams};
use izanagi_kit::noise::{fbm_2d, hash_2d};
use izanagi_kit::pathfinding::astar;
use izanagi_kit::terminal::{Cell, Screen};
use izanagi_kit::{compute_all, hash_state, wfc_solve, Fnv1a, SplitMix64, WfcResult, WfcRules};

/// Number of samples exposed to the page. Ids are `0..SAMPLE_COUNT`.
pub const SAMPLE_COUNT: u32 = 3;

/// Sample id: deterministic value-noise terrain (after `noise_terrain_demo`).
pub const SAMPLE_TERRAIN: u32 = 0;
/// Sample id: wave-function-collapse island (after `wfc_demo`).
pub const SAMPLE_WFC: u32 = 1;
/// Sample id: dungeon + A* + FOV with world hash (after `kit_bridge`).
pub const SAMPLE_DUNGEON: u32 = 2;

/// The seeds the original examples ship with; the page starts from these.
pub const DEFAULT_SEEDS: [u64; 3] = [0x0DEC_A1EA, 0x4C01_1234, 0x001D_A57E9];

/// Render one sample for `seed`, returning its cell screen and a one-line
/// status string (the same facts the terminal demos print). An unknown id
/// is an `Err` naming the valid range — the caller surfaces it, never
/// swallows it.
pub fn render_sample(id: u32, seed: u64) -> Result<(Screen, String), String> {
    match id {
        SAMPLE_TERRAIN => Ok(sample_terrain(seed)),
        SAMPLE_WFC => Ok(sample_wfc(seed)),
        SAMPLE_DUNGEON => Ok(sample_dungeon(seed)),
        _ => Err(format!("unknown sample id {id} (valid: 0..{SAMPLE_COUNT})")),
    }
}

// ── terrain ──────────────────────────────────────────────────────────────
//
// Mirrors izanagi_kit/examples/noise_terrain_demo.rs: fbm value noise →
// seven biomes by height threshold → glyph + 24-bit colour per cell, plus
// the sparse "feature" landmarks the demo scatters by hash.

#[derive(Clone, Copy, PartialEq, Eq)]
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
    fn from_height(h: u32) -> Biome {
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

fn sample_terrain(seed: u64) -> (Screen, String) {
    const W: u32 = 80;
    const H: u32 = 24;
    const MAP_H: u32 = 22;
    const OCTAVES: u32 = 3;
    // Each screen cell spans 1/2 of a noise grid unit (as in the demo).
    const SCALE: i32 = 2;

    let ui_bg = Color {
        r: 12,
        g: 12,
        b: 18,
    };
    let ui_fg = Color {
        r: 180,
        g: 180,
        b: 180,
    };
    let ui_hi = Color {
        r: 120,
        g: 200,
        b: 255,
    };

    let mut screen = Screen::new(W, H);
    let mut counts = [0u32; 7];
    for sy in 0..MAP_H as i32 {
        for sx in 0..W as i32 {
            let nx = (sx << 16) / SCALE;
            let ny = (sy << 16) / SCALE;
            let biome = Biome::from_height(fbm_2d(nx, ny, seed, OCTAVES));
            let feature = hash_2d(sx, sy, seed ^ 0xF0F0) >> 24;
            let glyph = if feature == 0 {
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
            counts[idx] += 1;
        }
    }

    screen.fill_rect(
        0,
        22,
        W,
        1,
        Cell {
            glyph: ' ',
            fg: ui_fg,
            bg: ui_bg,
        },
    );
    screen.fill_rect(
        0,
        23,
        W,
        1,
        Cell {
            glyph: ' ',
            fg: ui_fg,
            bg: ui_bg,
        },
    );
    let status =
        format!(" seed={seed:#018x}  octaves={OCTAVES}  scale=1/{SCALE}  cells={W}×{MAP_H}");
    screen.draw_str(0, 22, &status, ui_hi, ui_bg);

    let total = W * MAP_H;
    let mut lx = 0i32;
    for (i, &biome) in ALL_BIOMES.iter().enumerate() {
        let pct = counts[i] * 100 / total.max(1);
        let label = format!("{} {}% ", biome.name(), pct);
        screen.draw_str(lx, 23, &label, biome.fg(), ui_bg);
        lx += label.len() as i32;
    }

    let water = (counts[0] + counts[1]) * 100 / total.max(1);
    (
        screen,
        format!(
            "terrain — seed {seed:#018x}, water {water}%, land {}%",
            100 - water
        ),
    )
}

// ── wave-function collapse ───────────────────────────────────────────────
//
// Mirrors izanagi_kit/examples/wfc_demo.rs: a 5-tile terrain gradient
// (deep ↔ shallow ↔ sand ↔ grass ↔ mountain) solved on an 80×44 grid and
// scaled into the screen, with the grass autotile statistics the demo
// reports.

const DEEP: u8 = 0;
const SHALLOW: u8 = 1;
const SAND: u8 = 2;
const GRASS: u8 = 3;
const MOUNTAIN: u8 = 4;
const TILE_COUNT: u8 = 5;

fn wfc_appearance(tile: u8) -> (char, Color, Color) {
    match tile {
        DEEP => (
            '≈',
            Color {
                r: 30,
                g: 80,
                b: 180,
            },
            Color {
                r: 10,
                g: 30,
                b: 100,
            },
        ),
        SHALLOW => (
            '~',
            Color {
                r: 60,
                g: 150,
                b: 220,
            },
            Color {
                r: 20,
                g: 70,
                b: 160,
            },
        ),
        SAND => (
            '.',
            Color {
                r: 220,
                g: 200,
                b: 120,
            },
            Color {
                r: 180,
                g: 160,
                b: 80,
            },
        ),
        GRASS => (
            ',',
            Color {
                r: 80,
                g: 200,
                b: 60,
            },
            Color {
                r: 30,
                g: 100,
                b: 20,
            },
        ),
        MOUNTAIN => (
            '^',
            Color {
                r: 200,
                g: 200,
                b: 200,
            },
            Color {
                r: 100,
                g: 100,
                b: 100,
            },
        ),
        _ => (
            '?',
            Color {
                r: 255,
                g: 0,
                b: 255,
            },
            Color { r: 0, g: 0, b: 0 },
        ),
    }
}

fn wfc_rules() -> WfcRules {
    let mut r = WfcRules::new(TILE_COUNT);
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

fn sample_wfc(seed: u64) -> (Screen, String) {
    const GRID_W: i32 = 80;
    const GRID_H: i32 = 44;
    const W: u32 = 80;
    const H: u32 = 24;
    const MAP_H: u32 = 22;

    let rules = wfc_rules();
    let mut rng = SplitMix64::new(seed);
    // Retry on contradiction with a fresh stream each attempt, exactly as
    // the demo does; bounded so a pathological rule set cannot hang the page.
    let mut solved = None;
    for _ in 0..32 {
        let mut attempt = SplitMix64::new(rng.state());
        rng.below(1); // advance to produce a different seed next attempt
        if let WfcResult::Ok(g) = wfc_solve(GRID_W, GRID_H, &rules, &mut attempt) {
            solved = Some(g);
            break;
        }
    }
    let status_bg = Color {
        r: 15,
        g: 15,
        b: 15,
    };
    let status_fg = Color {
        r: 200,
        g: 200,
        b: 200,
    };
    let status_hi = Color {
        r: 120,
        g: 200,
        b: 120,
    };

    let mut screen = Screen::new(W, H);
    let grid = match solved {
        Some(g) => g,
        None => {
            screen.draw_str(
                0,
                0,
                "wfc: no solution found in 32 attempts",
                status_fg,
                status_bg,
            );
            return (
                screen,
                format!("wfc — seed {seed:#018x}: contradiction on every attempt"),
            );
        }
    };

    let masks = compute_all(GRID_W, GRID_H, |x, y| grid.tile_at(x, y) == Some(GRASS));
    let grass_total = (0..GRID_H)
        .flat_map(|y| (0..GRID_W).map(move |x| (x, y)))
        .filter(|&(x, y)| grid.tile_at(x, y) == Some(GRASS))
        .count();
    let autotile_nonzero = masks.iter().filter(|&&m| m > 0).count();

    for sy in 0..MAP_H {
        for sx in 0..W {
            let gx = (sx * (GRID_W as u32) / W) as i32;
            let gy = (sy * (GRID_H as u32) / MAP_H) as i32;
            let cell = match grid.tile_at(gx, gy) {
                Some(t) => {
                    let (glyph, fg, bg) = wfc_appearance(t);
                    Cell { glyph, fg, bg }
                }
                None => {
                    let (glyph, fg, bg) = wfc_appearance(u8::MAX);
                    Cell { glyph, fg, bg }
                }
            };
            screen.put(sx as i32, sy as i32, cell);
        }
    }

    screen.fill_rect(
        0,
        22,
        W,
        1,
        Cell {
            glyph: ' ',
            fg: status_fg,
            bg: status_bg,
        },
    );
    screen.fill_rect(
        0,
        23,
        W,
        1,
        Cell {
            glyph: ' ',
            fg: status_fg,
            bg: status_bg,
        },
    );
    screen.draw_str(
        0,
        22,
        &format!(
            " seed={seed:#018x}  grid={GRID_W}×{GRID_H}  grass={grass_total}  autotile variants>0={autotile_nonzero}"
        ),
        status_hi,
        status_bg,
    );
    let legend = [
        (DEEP, "deep"),
        (SHALLOW, "~coast"),
        (SAND, ".sand"),
        (GRASS, ",grass"),
        (MOUNTAIN, "^mount"),
    ];
    let mut x = 1i32;
    for (tile, label) in &legend {
        let (glyph, fg, bg) = wfc_appearance(*tile);
        screen.set(x, 23, glyph, fg, bg);
        x += 1;
        screen.draw_str(x, 23, label, status_fg, status_bg);
        x += label.len() as i32 + 2;
    }

    (
        screen,
        format!(
            "wfc — seed {seed:#018x}, {grass_total} grass cells, {autotile_nonzero} autotile variants"
        ),
    )
}

// ── dungeon ──────────────────────────────────────────────────────────────
//
// The sim half of izanagi/examples/kit_bridge.rs, run to completion:
// generate a dungeon, A* from the first room to the last, FOV from the
// player, one world hash per turn. The page shows the final frame; the
// status line carries the final hash so a visitor can compare it against
// a native run of the same code — determinism as a checkable fact.

const DUNGEON_W: u32 = 48;
const DUNGEON_H: u32 = 20;
const FOV_RADIUS: i32 = 8;

fn dungeon_frame(dungeon: &Dungeon, path: &[(i32, i32)], step: usize, turn: u32) -> Screen {
    let ink = |r, g, b| Color { r, g, b };
    let black = ink(0, 0, 0);
    let player = path[step.min(path.len() - 1)];

    let mut visible = vec![false; (DUNGEON_W * DUNGEON_H) as usize];
    compute_fov(
        player,
        FOV_RADIUS,
        |x, y| dungeon.is_wall(x, y),
        |x, y| {
            if x >= 0 && y >= 0 && (x as u32) < DUNGEON_W && (y as u32) < DUNGEON_H {
                visible[(y * DUNGEON_W as i32 + x) as usize] = true;
            }
        },
    );

    let mut screen = Screen::new(DUNGEON_W, DUNGEON_H + 1);
    for y in 0..DUNGEON_H as i32 {
        for x in 0..DUNGEON_W as i32 {
            let lit = visible[(y * DUNGEON_W as i32 + x) as usize];
            let (glyph, fg) = if dungeon.is_wall(x, y) {
                (
                    '#',
                    if lit {
                        ink(200, 180, 120)
                    } else {
                        ink(70, 70, 80)
                    },
                )
            } else {
                (
                    '.',
                    if lit {
                        ink(140, 140, 100)
                    } else {
                        ink(40, 40, 48)
                    },
                )
            };
            screen.set(x, y, glyph, fg, black);
        }
    }
    let goal = *path.last().unwrap_or(&player);
    screen.set(goal.0, goal.1, '>', ink(80, 220, 120), black);
    screen.set(player.0, player.1, '@', ink(255, 255, 255), black);
    let hud = format!("turn {turn:03}  pos ({:02},{:02})", player.0, player.1);
    screen.draw_str(0, DUNGEON_H as i32, &hud, ink(180, 180, 180), black);
    screen
}

fn dungeon_world_hash(screen: &Screen, player: (i32, i32), turn: u32) -> u64 {
    let mut h = Fnv1a::new();
    h.write_u64(hash_state(screen));
    h.write_i32(player.0);
    h.write_i32(player.1);
    h.write_u32(turn);
    h.finish()
}

fn sample_dungeon(seed: u64) -> (Screen, String) {
    let master = SplitMix64::new(seed);
    let mut map_rng = master.split(1);
    let dungeon = generate_dungeon(DUNGEON_W, DUNGEON_H, &mut map_rng, GenParams::default());
    let start = dungeon.rooms.first().map(|r| r.center()).unwrap_or((1, 1));
    let goal = dungeon.rooms.last().map(|r| r.center()).unwrap_or(start);
    let path = astar(start, goal, |x, y| dungeon.is_wall(x, y)).unwrap_or(vec![start]);

    // Walk the whole path headlessly, hashing every turn's frame exactly
    // as kit_bridge does; keep the final frame for the page.
    let mut step = 0usize;
    let mut turn = 0u32;
    let mut screen = dungeon_frame(&dungeon, &path, step, turn);
    let mut hash = dungeon_world_hash(&screen, path[step], turn);
    while step + 1 < path.len() {
        step += 1;
        turn += 1;
        screen = dungeon_frame(&dungeon, &path, step, turn);
        hash = dungeon_world_hash(&screen, path[step], turn);
    }

    (
        screen,
        format!(
            "dungeon — seed {seed:#018x}, {} rooms, {turn} turns, final hash {hash:016x}",
            dungeon.room_count()
        ),
    )
}

// ── frame serialization ──────────────────────────────────────────────────
//
// One format, shared by the wasm boundary and the tests:
//
//   u32le width, u32le height
//   u32le status byte length, status bytes (UTF-8)
//   per row: u32le glyph byte length, glyph bytes (UTF-8),
//            width×3 foreground RGB bytes, width×3 background RGB bytes
//
// A width/height of 0 carries a status-only frame (used for errors).

/// Serialize a screen + status line into the frame format above.
pub fn serialize(screen: &Screen, status: &str) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&screen.width().to_le_bytes());
    out.extend_from_slice(&screen.height().to_le_bytes());
    push_status(&mut out, status);
    for y in 0..screen.height() as i32 {
        let mut glyphs = String::new();
        let mut fg = Vec::with_capacity(screen.width() as usize * 3);
        let mut bg = Vec::with_capacity(screen.width() as usize * 3);
        for x in 0..screen.width() as i32 {
            let cell = screen.get(x, y).copied().unwrap_or_default();
            glyphs.push(cell.glyph);
            fg.extend_from_slice(&[cell.fg.r, cell.fg.g, cell.fg.b]);
            bg.extend_from_slice(&[cell.bg.r, cell.bg.g, cell.bg.b]);
        }
        out.extend_from_slice(&(glyphs.len() as u32).to_le_bytes());
        out.extend_from_slice(glyphs.as_bytes());
        out.extend_from_slice(&fg);
        out.extend_from_slice(&bg);
    }
    out
}

/// Serialize an error as a status-only frame (0×0 grid).
pub fn serialize_error(status: &str) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    push_status(&mut out, status);
    out
}

fn push_status(out: &mut Vec<u8>, status: &str) {
    out.extend_from_slice(&(status.len() as u32).to_le_bytes());
    out.extend_from_slice(status.as_bytes());
}

// ── wasm boundary ────────────────────────────────────────────────────────
//
// The page calls `run_sample`, then reads the frame through
// `frame_ptr`/`frame_len`. The buffer lives in a thread-local (the page
// is single-threaded) and stays valid until the next `run_sample` call;
// the page copies what it needs synchronously. No imports are required
// from the host — the module is self-contained.

thread_local! {
    static FRAME: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Number of samples the page can offer. Exported for the page/tests so
/// the count is never hard-coded in two places.
#[no_mangle]
pub extern "C" fn sample_count() -> u32 {
    SAMPLE_COUNT
}

/// Run sample `id` with the 64-bit seed given as two 32-bit halves and
/// store the serialized frame for `frame_ptr`/`frame_len`.
#[no_mangle]
pub extern "C" fn run_sample(id: u32, seed_lo: u32, seed_hi: u32) {
    let seed = ((seed_hi as u64) << 32) | (seed_lo as u64);
    let bytes = match render_sample(id, seed) {
        Ok((screen, status)) => serialize(&screen, &status),
        Err(msg) => serialize_error(&msg),
    };
    FRAME.with(|f| *f.borrow_mut() = bytes);
}

/// Pointer to the current frame's bytes. Valid until the next
/// `run_sample` call.
#[no_mangle]
pub extern "C" fn frame_ptr() -> *const u8 {
    FRAME.with(|f| f.borrow().as_ptr())
}

/// Length of the current frame in bytes.
#[no_mangle]
pub extern "C" fn frame_len() -> usize {
    FRAME.with(|f| f.borrow().len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame_bytes(id: u32, seed: u64) -> Vec<u8> {
        let (screen, status) = render_sample(id, seed).expect("sample renders");
        serialize(&screen, &status)
    }

    // House test shape: normal case / boundary / determinism / integration.

    #[test]
    fn every_sample_renders_a_nonempty_screen() {
        for (id, seed) in [
            (0, DEFAULT_SEEDS[0]),
            (1, DEFAULT_SEEDS[1]),
            (2, DEFAULT_SEEDS[2]),
        ] {
            let (screen, status) = render_sample(id, seed).expect("sample renders");
            assert!(screen.width() > 0 && screen.height() > 0, "sample {id}");
            assert!(!status.is_empty(), "sample {id} reports a status");
            let mut distinct = std::collections::BTreeSet::new();
            for y in 0..screen.height() as i32 {
                for x in 0..screen.width() as i32 {
                    if let Some(cell) = screen.get(x, y) {
                        distinct.insert(cell.glyph);
                    }
                }
            }
            assert!(
                distinct.len() >= 4,
                "sample {id} should paint a varied scene, got {distinct:?}"
            );
        }
    }

    #[test]
    fn screens_have_the_documented_dimensions() {
        let (terrain, _) = render_sample(SAMPLE_TERRAIN, DEFAULT_SEEDS[0]).unwrap();
        assert_eq!((terrain.width(), terrain.height()), (80, 24));
        let (wfc, _) = render_sample(SAMPLE_WFC, DEFAULT_SEEDS[1]).unwrap();
        assert_eq!((wfc.width(), wfc.height()), (80, 24));
        let (dungeon, _) = render_sample(SAMPLE_DUNGEON, DEFAULT_SEEDS[2]).unwrap();
        assert_eq!((dungeon.width(), dungeon.height()), (48, 21));
    }

    #[test]
    fn every_sample_is_deterministic_for_a_fixed_seed() {
        for id in 0..SAMPLE_COUNT {
            let seed = DEFAULT_SEEDS[id as usize];
            assert_eq!(
                frame_bytes(id, seed),
                frame_bytes(id, seed),
                "sample {id} must reproduce byte-for-byte"
            );
        }
    }

    #[test]
    fn a_different_seed_changes_the_terrain() {
        assert_ne!(
            frame_bytes(SAMPLE_TERRAIN, DEFAULT_SEEDS[0]),
            frame_bytes(SAMPLE_TERRAIN, DEFAULT_SEEDS[0] ^ 0x5EED)
        );
    }

    #[test]
    fn wfc_resolves_every_cell_for_the_default_seed() {
        let (screen, _) = render_sample(SAMPLE_WFC, DEFAULT_SEEDS[1]).unwrap();
        for y in 0..22 {
            for x in 0..screen.width() as i32 {
                assert_ne!(
                    screen.get(x, y).map(|c| c.glyph),
                    Some('?'),
                    "cell ({x},{y}) left unresolved"
                );
            }
        }
    }

    #[test]
    fn dungeon_final_hash_is_pinned() {
        // The same value the Node wasm test asserts: native and wasm runs
        // of this code must agree bit-for-bit, which is the kit's whole
        // claim. If this changes, the sim's RNG consumption or arithmetic
        // changed — treat it like the kit's own pinned hashes.
        let (_, status) = render_sample(SAMPLE_DUNGEON, DEFAULT_SEEDS[2]).unwrap();
        assert!(
            status.contains("final hash 353498ec4fbcd160"),
            "unexpected dungeon status: {status}"
        );
    }

    #[test]
    fn unknown_sample_id_is_an_error_not_a_panic() {
        let err = render_sample(SAMPLE_COUNT, 0).unwrap_err();
        assert!(err.contains("unknown sample id"), "{err}");
        let frame = serialize_error(&err);
        assert_eq!(&frame[0..4], &0u32.to_le_bytes());
    }

    #[test]
    fn the_wasm_boundary_round_trips_a_frame() {
        run_sample(SAMPLE_DUNGEON, 0x001D_A57E9, 0);
        let len = frame_len();
        assert!(len > 0);
        let bytes = unsafe { std::slice::from_raw_parts(frame_ptr(), len) }.to_vec();
        assert_eq!(bytes, frame_bytes(SAMPLE_DUNGEON, DEFAULT_SEEDS[2]));
    }
}
