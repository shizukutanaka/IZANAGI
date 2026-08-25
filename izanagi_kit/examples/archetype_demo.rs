//! Archetype storage demo: `ArchTable<Row>` ECS component layout.
//!
//! Two archetypes model a swarm simulation:
//! - `Mobile { pos, vel }` — entities that move each tick.
//! - `Static { pos }` — entities that have come to rest.
//!
//! A movement system walks the dense `Mobile` table with `iter_mut` (the
//! cache-friendly struct-of-arrays iteration the module is built for), bounces
//! entities off the arena walls, and — when an entity's velocity decays to
//! zero — **migrates** it from `Mobile` to `Static` via `remove` + `insert`,
//! the O(1) swap-remove handoff archetype ECS is known for.
//!
//! Modules exercised:
//! - `ArchTable::insert` / `remove` / `get` / `iter` / `iter_mut` / `len`.
//! - Entity migration between two tables (swap-remove + push).
//! - `world_hash::hash_state` over each table (`DetHash` is canonical, so the
//!   checksum is independent of swap-remove history).
//!
//! Run with `cargo run --example archetype_demo`.

use izanagi_kit::content::Color;
use izanagi_kit::entity::{Entity, EntityAllocator};
use izanagi_kit::world_hash::{hash_state, DetHash, Fnv1a};
use izanagi_kit::{ArchTable, Cell, Screen};
use std::io::{self, Write};

// ── layout ────────────────────────────────────────────────────────────────────

const SCREEN_W: u32 = 80;
const SCREEN_H: u32 = 24;
const ARENA_W: i32 = 44;
const ARENA_H: i32 = 18;
const ARENA_X: i32 = 1;
const ARENA_Y: i32 = 3;
const PANEL_X: i32 = 47;
const TICKS: u32 = 14;

// ── palette ───────────────────────────────────────────────────────────────────

const BG: Color = Color { r: 8, g: 8, b: 16 };
const TITLE_BG: Color = Color {
    r: 16,
    g: 18,
    b: 52,
};
const TITLE_FG: Color = Color {
    r: 200,
    g: 205,
    b: 255,
};
const WALL_FG: Color = Color {
    r: 60,
    g: 58,
    b: 92,
};
const HDR_FG: Color = Color {
    r: 118,
    g: 115,
    b: 175,
};
const MOBILE_FG: Color = Color {
    r: 120,
    g: 215,
    b: 255,
};
const STATIC_FG: Color = Color {
    r: 255,
    g: 180,
    b: 90,
};
const TRAIL_FG: Color = Color {
    r: 50,
    g: 70,
    b: 95,
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
const LOG_FG: Color = Color {
    r: 150,
    g: 220,
    b: 160,
};

// ── component rows ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct Mobile {
    x: i32,
    y: i32,
    vx: i32,
    vy: i32,
    /// Remaining ticks of motion before the entity comes to rest.
    fuel: i32,
}

impl DetHash for Mobile {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_i32(self.x);
        hasher.write_i32(self.y);
        hasher.write_i32(self.vx);
        hasher.write_i32(self.vy);
        hasher.write_i32(self.fuel);
    }
}

#[derive(Clone, Debug)]
struct Static {
    x: i32,
    y: i32,
}

impl DetHash for Static {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_i32(self.x);
        hasher.write_i32(self.y);
    }
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let mut alloc = EntityAllocator::new();
    let mut mobile: ArchTable<Mobile> = ArchTable::new();
    let static_table_seed: ArchTable<Static> = ArchTable::new();
    let mut stationary = static_table_seed;

    // Seed five mobile entities with distinct velocities and fuel budgets.
    // fuel < TICKS → migrates to Static; fuel ≥ TICKS → stays Mobile.
    let seeds: [(i32, i32, i32, i32, i32); 5] = [
        (4, 4, 2, 1, 6),
        (38, 5, -2, 1, 9),
        (10, 14, 1, -1, 5),
        (30, 12, -1, -2, 18), // outlives the run → stays Mobile
        (20, 9, 2, -1, 16),   // outlives the run → stays Mobile
    ];
    let mut ids: Vec<Entity> = Vec::new();
    for &(x, y, vx, vy, fuel) in &seeds {
        let e = alloc.allocate();
        mobile.insert(e, Mobile { x, y, vx, vy, fuel });
        ids.push(e);
    }

    // Trail buffer for rendering (last visited cells).
    let mut trail: Vec<(i32, i32)> = Vec::new();
    let mut migrations: Vec<(u32, u32, i32, i32)> = Vec::new(); // (tick, entity_idx, x, y)

    // ── simulate ────────────────────────────────────────────────────────────────
    for tick in 0..TICKS {
        // Movement system: walk the dense Mobile table.
        let mut to_migrate: Vec<Entity> = Vec::new();
        for (e, m) in mobile.iter_mut() {
            trail.push((m.x, m.y));
            // Integrate position.
            m.x += m.vx;
            m.y += m.vy;
            // Bounce off interior walls (arena border is solid).
            if m.x <= 1 || m.x >= ARENA_W - 2 {
                m.vx = -m.vx;
                m.x = m.x.clamp(1, ARENA_W - 2);
            }
            if m.y <= 1 || m.y >= ARENA_H - 2 {
                m.vy = -m.vy;
                m.y = m.y.clamp(1, ARENA_H - 2);
            }
            m.fuel -= 1;
            if m.fuel <= 0 {
                to_migrate.push(e);
            }
        }

        // Migrate spent entities Mobile → Static (O(1) swap-remove + push).
        for e in to_migrate {
            if let Some(m) = mobile.remove(e) {
                stationary.insert(e, Static { x: m.x, y: m.y });
                migrations.push((tick, e.index(), m.x, m.y));
            }
        }
    }

    let mobile_hash = hash_state(&mobile);
    let static_hash = hash_state(&stationary);

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
        " ARCHETYPE STORAGE   ArchTable: dense iter + O(1) migration",
        TITLE_FG,
        TITLE_BG,
    );
    screen.draw_str(ARENA_X, 2, "ARENA (swarm sim)", HDR_FG, BG);
    screen.draw_str(PANEL_X, 2, "ARCHETYPE TABLES", HDR_FG, BG);

    // Arena border.
    for x in 0..ARENA_W {
        screen.set(ARENA_X + x, ARENA_Y, '─', WALL_FG, BG);
        screen.set(ARENA_X + x, ARENA_Y + ARENA_H - 1, '─', WALL_FG, BG);
    }
    for y in 0..ARENA_H {
        screen.set(ARENA_X, ARENA_Y + y, '│', WALL_FG, BG);
        screen.set(ARENA_X + ARENA_W - 1, ARENA_Y + y, '│', WALL_FG, BG);
    }

    // Trails (dim).
    for &(tx, ty) in &trail {
        if tx > 0 && tx < ARENA_W - 1 && ty > 0 && ty < ARENA_H - 1 {
            screen.set(ARENA_X + tx, ARENA_Y + ty, '·', TRAIL_FG, BG);
        }
    }

    // Static (rested) entities — orange ▣.
    for (e, s) in stationary.iter() {
        screen.set(ARENA_X + s.x, ARENA_Y + s.y, '▣', STATIC_FG, BG);
        let _ = e;
    }
    // Mobile (still moving) entities — cyan ●.
    for (e, m) in mobile.iter() {
        screen.set(ARENA_X + m.x, ARENA_Y + m.y, '●', MOBILE_FG, BG);
        let _ = e;
    }

    // ── right panel: archetype table contents ────────────────────────────────
    let mut py = 3;
    screen.draw_str(PANEL_X, py, "Mobile{pos,vel,fuel}", MOBILE_FG, BG);
    py += 1;
    let mut mrows: Vec<(Entity, Mobile)> = mobile.iter().map(|(e, m)| (e, m.clone())).collect();
    mrows.sort_by_key(|(e, _)| e.index());
    for (e, m) in &mrows {
        if py >= 12 {
            break;
        }
        let line = format!(
            "e{} ({:>2},{:>2}) v({:>2},{:>2}) f{}",
            e.index(),
            m.x,
            m.y,
            m.vx,
            m.vy,
            m.fuel.max(0),
        );
        screen.draw_str(PANEL_X, py, &line, STAT_FG, BG);
        py += 1;
    }
    if mrows.is_empty() {
        screen.draw_str(PANEL_X, py, "(all rested)", TRAIL_FG, BG);
    }

    py = 13;
    screen.draw_str(PANEL_X, py, "Static{pos}  ← migrated", STATIC_FG, BG);
    py += 1;
    let mut srows: Vec<(Entity, Static)> = stationary.iter().map(|(e, s)| (e, s.clone())).collect();
    srows.sort_by_key(|(e, _)| e.index());
    for (e, s) in &srows {
        if py >= 20 {
            break;
        }
        let line = format!("e{} rest at ({:>2},{:>2})", e.index(), s.x, s.y);
        screen.draw_str(PANEL_X, py, &line, STAT_FG, BG);
        py += 1;
    }

    // ── migration log under the arena ───────────────────────────────────────
    let log_y = ARENA_Y + ARENA_H;
    screen.draw_str(ARENA_X, log_y, "Migrations Mobile→Static:", HDR_FG, BG);
    let recent = migrations.iter().rev().take(2);
    let mut lx = ARENA_X;
    let ly = log_y + 1;
    for (t, idx, x, y) in recent {
        let s = format!("t{} e{}→({},{})  ", t, idx, x, y);
        screen.draw_str(lx, ly, &s, LOG_FG, BG);
        lx += s.chars().count() as i32;
    }

    // ── stats ─────────────────────────────────────────────────────────────────
    let stat = format!(
        " mobile={} static={} migrations={} ticks={}  hash(M)={:#018x}",
        mobile.len(),
        stationary.len(),
        migrations.len(),
        TICKS,
        mobile_hash,
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
        "\nArchetype storage demo.\n\
         {} entities seeded into Mobile; movement system iterated the dense\n\
         table each tick. Entities out of fuel migrated Mobile→Static via\n\
         O(1) swap-remove + insert.\n\
         final: mobile={} static={} migrations={}\n\
         canonical hashes — Mobile={:#018x}  Static={:#018x}",
        seeds.len(),
        mobile.len(),
        stationary.len(),
        migrations.len(),
        mobile_hash,
        static_hash,
    );
}
