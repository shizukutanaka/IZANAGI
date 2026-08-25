//! Cave + spawn-table demo: `generate_cave` / `RandomTable` / `Distance`.
//!
//! Ties together three recently added primitives into one screen:
//! - `mapgen::generate_cave` carves an organic, fully-connected cavern with the
//!   cellular-automata method (no rectangular rooms).
//! - `RandomTable<Spawn>` is a depth-scaled spawn table: each scattered floor
//!   cell rolls a monster or item with weight proportional to dungeon depth.
//! - `geometry::Distance` (Chebyshev) tints the player's awareness radius and
//!   reports how many spawns fall inside it.
//!
//! Everything is deterministic: the cave, the spawn positions, and every table
//! roll all draw from a single seeded `SplitMix64`.
//!
//! Run with `cargo run --example cave_spawn_demo`.

use izanagi_kit::content::Color;
use izanagi_kit::{generate_cave, CaveParams, Cell, Distance, RandomTable, Screen, SplitMix64};
use std::io::{self, Write};

// ── layout ────────────────────────────────────────────────────────────────────

const SCREEN_W: u32 = 80;
const SCREEN_H: u32 = 24;
const MAP_W: u32 = 64;
const MAP_H: u32 = 20;
const MAP_X: i32 = 0;
const MAP_Y: i32 = 2;
const PANEL_X: i32 = 65;
const DEPTH: i32 = 7; // dungeon depth drives spawn weights
const AWARE_R: i32 = 6; // player awareness radius (Chebyshev)
const SEED: u64 = 0xCA7E_5EED;

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
const WALL_FG: Color = Color {
    r: 70,
    g: 66,
    b: 96,
};
const WALL_BG: Color = Color {
    r: 14,
    g: 13,
    b: 22,
};
const FLOOR_FG: Color = Color {
    r: 46,
    g: 50,
    b: 66,
};
const AWARE_BG: Color = Color {
    r: 20,
    g: 28,
    b: 30,
};
const PLAYER_FG: Color = Color {
    r: 255,
    g: 240,
    b: 120,
};
const HDR_FG: Color = Color {
    r: 120,
    g: 120,
    b: 175,
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

// ── spawn kinds ─────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
struct Spawn {
    glyph: char,
    name: &'static str,
    color: Color,
    is_monster: bool,
}

fn main() {
    let mut rng = SplitMix64::new(SEED);

    // ── carve the cave ────────────────────────────────────────────────────────
    let cave = generate_cave(MAP_W, MAP_H, &mut rng, CaveParams::default());

    // Pick a player start: the first floor cell (deterministic scan order).
    let mut player = (MAP_W as i32 / 2, MAP_H as i32 / 2);
    'find: for y in 0..MAP_H as i32 {
        for x in 0..MAP_W as i32 {
            if cave.is_floor(x, y) {
                player = (x, y);
                break 'find;
            }
        }
    }

    // ── depth-scaled spawn table ────────────────────────────────────────────
    // Monster weights climb with depth; consumables stay roughly flat. This is
    // the classic data-driven spawn-table-by-depth pattern.
    let rat = Spawn {
        glyph: 'r',
        name: "giant rat",
        color: Color {
            r: 150,
            g: 120,
            b: 90,
        },
        is_monster: true,
    };
    let bat = Spawn {
        glyph: 'b',
        name: "cave bat",
        color: Color {
            r: 130,
            g: 110,
            b: 160,
        },
        is_monster: true,
    };
    let orc = Spawn {
        glyph: 'o',
        name: "orc",
        color: Color {
            r: 110,
            g: 200,
            b: 110,
        },
        is_monster: true,
    };
    let troll = Spawn {
        glyph: 'T',
        name: "troll",
        color: Color {
            r: 90,
            g: 220,
            b: 150,
        },
        is_monster: true,
    };
    let potion = Spawn {
        glyph: '!',
        name: "potion",
        color: Color {
            r: 230,
            g: 110,
            b: 200,
        },
        is_monster: false,
    };
    let gold = Spawn {
        glyph: '$',
        name: "gold",
        color: Color {
            r: 240,
            g: 210,
            b: 90,
        },
        is_monster: false,
    };

    let table: RandomTable<Spawn> = RandomTable::new()
        .with((12 - DEPTH).max(1) as u32, rat) // common early, fades with depth
        .with(8, bat)
        .with((DEPTH * 2) as u32, orc) // scales up with depth
        .with((DEPTH - 3).max(0) as u32, troll) // only deep down
        .with(6, potion)
        .with(5, gold);

    // ── scatter spawns onto floor cells ────────────────────────────────────────
    // Roll a fixed number of placement attempts; each picks a random floor cell
    // and rolls the table. (One cell may host at most one spawn.)
    let mut spawns: Vec<(i32, i32, Spawn)> = Vec::new();
    let mut occupied = vec![false; (MAP_W * MAP_H) as usize];
    let occ_idx = |x: i32, y: i32| (y as u32 * MAP_W + x as u32) as usize;
    occupied[occ_idx(player.0, player.1)] = true;

    let attempts = 60;
    for _ in 0..attempts {
        let x = rng.range(1, MAP_W as i32 - 1);
        let y = rng.range(1, MAP_H as i32 - 1);
        if !cave.is_floor(x, y) || occupied[occ_idx(x, y)] {
            continue;
        }
        if let Some(&spawn) = table.roll(&mut rng) {
            occupied[occ_idx(x, y)] = true;
            spawns.push((x, y, spawn));
        }
    }

    // ── awareness query via Chebyshev distance ────────────────────────────────
    let in_radius = |x: i32, y: i32| Distance::Chebyshev.between(player, (x, y)) <= AWARE_R;
    let nearby_monsters = spawns
        .iter()
        .filter(|(x, y, s)| s.is_monster && in_radius(*x, *y))
        .count();

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
        " CAVE + SPAWN TABLE   generate_cave / RandomTable / Distance",
        TITLE_FG,
        TITLE_BG,
    );

    screen.draw_str(1, 1, "CELLULAR-AUTOMATA CAVE", HDR_FG, BG);
    screen.draw_str(PANEL_X, 1, "SPAWN TABLE", HDR_FG, BG);

    // Cave tiles (awareness radius gets a faint background tint).
    for y in 0..MAP_H as i32 {
        for x in 0..MAP_W as i32 {
            let aware = in_radius(x, y);
            let (glyph, fg, bg) = if cave.is_wall(x, y) {
                ('#', WALL_FG, WALL_BG)
            } else {
                ('·', FLOOR_FG, if aware { AWARE_BG } else { BG })
            };
            screen.set(MAP_X + x, MAP_Y + y, glyph, fg, bg);
        }
    }

    // Spawns.
    for (x, y, s) in &spawns {
        let aware = in_radius(*x, *y);
        let bg = if aware { AWARE_BG } else { BG };
        screen.set(MAP_X + *x, MAP_Y + *y, s.glyph, s.color, bg);
    }
    // Player on top.
    screen.set(MAP_X + player.0, MAP_Y + player.1, '@', PLAYER_FG, AWARE_BG);

    // ── right panel: table weights ─────────────────────────────────────────────
    screen.draw_str(PANEL_X, 2, &format!("depth {}", DEPTH), STAT_HI, BG);
    let mut py = 3;
    for (weight, spawn) in table.iter() {
        if py >= MAP_Y + MAP_H as i32 {
            break;
        }
        screen.set(PANEL_X, py, spawn.glyph, spawn.color, BG);
        let line = format!(" w{:<2} {}", weight, spawn.name);
        let line: String = line.chars().take(14).collect();
        screen.draw_str(PANEL_X + 1, py, &line, STAT_FG, BG);
        py += 1;
    }
    py += 1;
    if py < MAP_Y + MAP_H as i32 {
        screen.draw_str(PANEL_X, py, "@ = you", PLAYER_FG, BG);
    }

    // ── stats ─────────────────────────────────────────────────────────────────
    let monsters = spawns.iter().filter(|(_, _, s)| s.is_monster).count();
    let items = spawns.len() - monsters;
    let floor_cells = (0..MAP_H as i32)
        .flat_map(|y| (0..MAP_W as i32).map(move |x| (x, y)))
        .filter(|(x, y)| cave.is_floor(*x, *y))
        .count();

    let stat = format!(
        " floor={}  spawns={} (mon {}, item {})  near@(r{})={}  total_w={}",
        floor_cells,
        spawns.len(),
        monsters,
        items,
        AWARE_R,
        nearby_monsters,
        table.total_weight(),
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
        "\nCave + spawn-table demo (seed {SEED:#x}).\n\
         generate_cave → {floor_cells} connected floor cells.\n\
         RandomTable(depth {DEPTH}, total_weight {}) scattered {} spawns \
         ({} monsters, {} items).\n\
         Chebyshev awareness radius {AWARE_R}: {} monsters near the player.",
        table.total_weight(),
        spawns.len(),
        monsters,
        items,
        nearby_monsters,
    );
}
