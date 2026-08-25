//! Headless terminal demo: roguelike simulation rendered with izanagi_kit.
//!
//! Generates a dungeon (seed 0x5EED_1234), places a player and monsters,
//! runs 200 turns of the energy-based scheduler, and renders two snapshots
//! (initial state + final state) using the terminal module's 24-bit ANSI
//! serialiser. Run with `cargo run --example roguelike_demo`.
//!
//! Pipe to a real tty (or iTerm / Windows Terminal) for full-colour output.
//! In CI the ANSI bytes still flow to stdout, proving the rendering pipeline
//! is exercised end-to-end without any OS dependency.

use izanagi_kit::content::Color;
use izanagi_kit::Dungeon;
use izanagi_kit::{
    astar, compute_fov, generate_dungeon, melee_attack, Camera, Cell, GenParams, MsgLog,
    PassabilityGrid, Scheduler, Screen, SplitMix64, Stats,
};
use std::collections::BTreeMap;
use std::io::{self, Write};

// ── palette ─────────────────────────────────────────────────────────────────

const BLACK: Color = Color { r: 0, g: 0, b: 0 };
const DIM_WALL: Color = Color {
    r: 55,
    g: 55,
    b: 55,
};
const LIT_WALL: Color = Color {
    r: 130,
    g: 130,
    b: 130,
};
const LIT_FLOOR: Color = Color {
    r: 70,
    g: 60,
    b: 50,
};
const YELLOW: Color = Color {
    r: 220,
    g: 200,
    b: 60,
};
const RED: Color = Color {
    r: 200,
    g: 80,
    b: 80,
};
const UI_BG: Color = Color {
    r: 18,
    g: 18,
    b: 18,
};
const HP_OK: Color = Color {
    r: 80,
    g: 200,
    b: 80,
};
const HP_WARN: Color = Color {
    r: 220,
    g: 210,
    b: 40,
};
const HP_CRIT: Color = Color {
    r: 220,
    g: 60,
    b: 60,
};
const MSG_FG: Color = Color {
    r: 170,
    g: 170,
    b: 170,
};

// ── layout constants ─────────────────────────────────────────────────────────

const SCREEN_W: u32 = 80;
const SCREEN_H: u32 = 24;
const MAP_H: u32 = 20; // rows 0..20 are the viewport
const STATUS_Y: i32 = 20;
const SEP_Y: i32 = 21;
const LOG_Y: i32 = 22; // 2 log rows: 22 and 23
const LOG_ROWS: usize = 2;

const PLAYER: u32 = 0;
const FOV_RADIUS: i32 = 8;
const MONSTER_GLYPHS: [char; 5] = ['g', 'G', 'o', 'r', 'T'];

// ── data types ───────────────────────────────────────────────────────────────

struct Actor {
    pos: (i32, i32),
    stats: Stats,
    glyph: char,
}

struct Sim {
    dungeon: Dungeon,
    grid: PassabilityGrid,
    actors: BTreeMap<u32, Actor>,
    scheduler: Scheduler<u32>,
    log: MsgLog,
    /// FOV bitmask: index = y * dungeon_width + x.
    visible: Vec<bool>,
    turn: u32,
}

// ── simulation ───────────────────────────────────────────────────────────────

impl Sim {
    fn new(seed: u64) -> Self {
        let mut rng = SplitMix64::new(seed);
        let dungeon = generate_dungeon(48, 32, &mut rng, GenParams::default());
        let grid = PassabilityGrid::from_dungeon(&dungeon);
        let vis_len = dungeon.width() as usize * dungeon.height() as usize;

        let mut actors: BTreeMap<u32, Actor> = BTreeMap::new();
        let mut scheduler: Scheduler<u32> = Scheduler::new();

        if let Some(room) = dungeon.rooms.first() {
            let pos = room.center();
            actors.insert(
                PLAYER,
                Actor {
                    pos,
                    stats: Stats::new(40, 8, 3),
                    glyph: '@',
                },
            );
            scheduler.add(PLAYER, 10);
        }
        for (i, room) in dungeon.rooms.iter().enumerate().skip(1) {
            let id = i as u32;
            actors.insert(
                id,
                Actor {
                    pos: room.center(),
                    stats: Stats::new(12 + (i as i32 % 5), 4, 1),
                    glyph: MONSTER_GLYPHS[i % MONSTER_GLYPHS.len()],
                },
            );
            scheduler.add(id, 7 + (i as i32 % 4));
        }

        let mut sim = Sim {
            dungeon,
            grid,
            actors,
            scheduler,
            log: MsgLog::new(8),
            visible: vec![false; vis_len],
            turn: 0,
        };
        sim.refresh_fov();
        sim
    }

    fn adjacent(a: (i32, i32), b: (i32, i32)) -> bool {
        (a.0 - b.0).abs() <= 1 && (a.1 - b.1).abs() <= 1 && a != b
    }

    fn occupied_by_other(&self, mover: u32, x: i32, y: i32) -> bool {
        self.actors
            .iter()
            .any(|(&id, a)| id != mover && a.pos == (x, y))
    }

    fn refresh_fov(&mut self) {
        for v in &mut self.visible {
            *v = false;
        }
        let pos = match self.actors.get(&PLAYER) {
            Some(p) => p.pos,
            None => return,
        };
        let dw = self.dungeon.width() as i32;
        let dh = self.dungeon.height() as i32;
        compute_fov(
            pos,
            FOV_RADIUS,
            |x, y| self.grid.is_blocked(x, y),
            |x, y| {
                if x >= 0 && y >= 0 && x < dw && y < dh {
                    self.visible[y as usize * dw as usize + x as usize] = true;
                }
            },
        );
    }

    fn is_visible(&self, x: i32, y: i32) -> bool {
        let dw = self.dungeon.width() as i32;
        let dh = self.dungeon.height() as i32;
        if x < 0 || y < 0 || x >= dw || y >= dh {
            return false;
        }
        self.visible[y as usize * dw as usize + x as usize]
    }

    fn take_turn(&mut self, id: u32) {
        let actor_pos = match self.actors.get(&id) {
            Some(a) => a.pos,
            None => return,
        };

        if id == PLAYER {
            // Retaliate against lowest-id adjacent monster.
            let target = self
                .actors
                .iter()
                .filter(|(&oid, a)| oid != PLAYER && Self::adjacent(actor_pos, a.pos))
                .map(|(&oid, _)| oid)
                .next();
            if let Some(tid) = target {
                let glyph = self.actors.get(&tid).map(|a| a.glyph).unwrap_or('?');
                if let Some(dmg) = self.resolve_attack(PLAYER, tid) {
                    let alive = self.actors.contains_key(&tid);
                    let msg = if alive {
                        format!("You hit '{}' for {} dmg.", glyph, dmg)
                    } else {
                        format!("You slay '{}'! ({} dmg)", glyph, dmg)
                    };
                    self.log.push(msg);
                }
            }
            return;
        }

        let player_pos = match self.actors.get(&PLAYER) {
            Some(p) => p.pos,
            None => return,
        };

        if Self::adjacent(actor_pos, player_pos) {
            let glyph = self.actors.get(&id).map(|a| a.glyph).unwrap_or('?');
            if let Some(dmg) = self.resolve_attack(id, PLAYER) {
                self.log
                    .push(format!("'{}' hits you for {} dmg!", glyph, dmg));
            }
            return;
        }

        // Path toward the player; treat walls and other actors as impassable.
        let path = astar(actor_pos, player_pos, |x, y| {
            if (x, y) == player_pos {
                return false; // allow goal cell
            }
            self.grid.is_blocked(x, y) || self.occupied_by_other(id, x, y)
        });
        if let Some(path) = path {
            if path.len() >= 2 && path[1] != player_pos {
                if let Some(a) = self.actors.get_mut(&id) {
                    a.pos = path[1];
                }
            }
        }
    }

    fn resolve_attack(&mut self, attacker: u32, defender: u32) -> Option<i32> {
        let atk = self.actors.get(&attacker)?.stats.clone();
        let mut def = self.actors.get(&defender)?.stats.clone();
        let hp_before = def.hp;
        melee_attack(&atk, &mut def);
        let dmg = hp_before - def.hp;
        let dead = !def.is_alive();
        if let Some(d) = self.actors.get_mut(&defender) {
            d.stats = def;
        }
        if dead {
            self.actors.remove(&defender);
            self.scheduler.remove(defender);
        }
        Some(dmg)
    }

    fn step(&mut self) {
        if let Some(id) = self.scheduler.next_turn() {
            self.take_turn(id);
            self.turn += 1;
            if id == PLAYER || !self.actors.contains_key(&PLAYER) {
                self.refresh_fov();
            }
        }
    }
}

// ── rendering ────────────────────────────────────────────────────────────────

fn render(sim: &Sim, screen: &mut Screen) {
    // ── camera centred on player ─────────────────────────────────────────────
    let player_pos = sim.actors.get(&PLAYER).map(|a| a.pos).unwrap_or((24, 16));
    let cam = Camera::new(
        player_pos.0,
        player_pos.1,
        SCREEN_W,
        MAP_H,
        sim.dungeon.width(),
        sim.dungeon.height(),
    );

    // ── map viewport ─────────────────────────────────────────────────────────
    for sy in 0..MAP_H as i32 {
        for sx in 0..SCREEN_W as i32 {
            let (wx, wy) = cam.screen_to_world(sx as u32, sy as u32);
            let visible = sim.is_visible(wx, wy);
            let blocked = sim.grid.is_blocked(wx, wy);
            let cell = if visible {
                if blocked {
                    Cell {
                        glyph: '#',
                        fg: LIT_WALL,
                        bg: BLACK,
                    }
                } else {
                    Cell {
                        glyph: '.',
                        fg: LIT_FLOOR,
                        bg: BLACK,
                    }
                }
            } else {
                // Out-of-map or unseen: dark.
                Cell {
                    glyph: ' ',
                    fg: BLACK,
                    bg: BLACK,
                }
            };
            screen.put(sx, sy, cell);
        }
    }

    // ── actors ───────────────────────────────────────────────────────────────
    for (&id, actor) in &sim.actors {
        if let Some((sx, sy)) = cam.world_to_screen(actor.pos.0, actor.pos.1) {
            let fg = if id == PLAYER { YELLOW } else { RED };
            screen.set(sx as i32, sy as i32, actor.glyph, fg, BLACK);
        }
    }

    // ── status line ──────────────────────────────────────────────────────────
    screen.fill_rect(0, STATUS_Y, SCREEN_W, 1, Cell::blank());
    let status_text = if let Some(p) = sim.actors.get(&PLAYER) {
        let (hp, max_hp) = p.stats.hp_fraction();
        let pct = hp * 100 / max_hp.max(1);
        let hp_col = if pct >= 75 {
            HP_OK
        } else if pct >= 40 {
            HP_WARN
        } else {
            HP_CRIT
        };
        let text = format!(
            " HP {}/{} | Monsters {} | Turn {} ",
            hp,
            max_hp,
            sim.actors.len().saturating_sub(1),
            sim.turn,
        );
        screen.draw_str(0, STATUS_Y, &text, hp_col, UI_BG);
        format!(" HP {}/{}", hp, max_hp) // unused, drawn above
    } else {
        screen.draw_str(0, STATUS_Y, " *** DEAD ***", HP_CRIT, UI_BG);
        String::new()
    };
    let _ = status_text;

    // ── separator ────────────────────────────────────────────────────────────
    let sep = "─".repeat(SCREEN_W as usize);
    screen.draw_str(0, SEP_Y, &sep, DIM_WALL, UI_BG);

    // ── message log ──────────────────────────────────────────────────────────
    for row in 0..LOG_ROWS as i32 {
        screen.fill_rect(
            0,
            LOG_Y + row,
            SCREEN_W,
            1,
            Cell {
                glyph: ' ',
                fg: MSG_FG,
                bg: UI_BG,
            },
        );
    }
    let msgs: Vec<&str> = sim.log.iter().collect();
    for (i, msg) in msgs.iter().rev().take(LOG_ROWS).enumerate() {
        let row = LOG_Y + (LOG_ROWS - 1 - i) as i32;
        screen.draw_str(0, row, &format!(" {}", msg), MSG_FG, UI_BG);
    }
}

// ── entry point ──────────────────────────────────────────────────────────────

fn main() {
    const SEED: u64 = 0x5EED_1234;
    const TURNS: u32 = 200;

    let mut sim = Sim::new(SEED);
    let mut screen = Screen::new(SCREEN_W, SCREEN_H);
    let stdout = io::stdout();
    let mut out = stdout.lock();

    // Render and emit the initial state.
    render(&sim, &mut screen);
    screen.present();
    let _ = out.write_all(b"\x1b[2J"); // clear screen
    let _ = out.write_all(sim_header(0, SEED).as_bytes());
    let _ = out.write_all(screen.to_ansi().as_bytes());
    let _ = out.write_all(b"\r\n");
    let _ = out.flush();

    // Simulate TURNS scheduler ticks.
    for _ in 0..TURNS {
        sim.step();
    }

    // Render and emit the final state.
    render(&sim, &mut screen);
    screen.present();
    let _ = out.write_all(b"\x1b[2J");
    let _ = out.write_all(sim_header(sim.turn, SEED).as_bytes());
    let _ = out.write_all(screen.to_ansi().as_bytes());
    let _ = out.write_all(b"\r\n");
    let _ = out.flush();

    // Summary (always visible, regardless of tty mode).
    eprintln!(
        "\nDemo complete.  seed={:#018x}  scheduler_ticks={}  actors_alive={}",
        SEED,
        sim.turn,
        sim.actors.len(),
    );
}

fn sim_header(turn: u32, seed: u64) -> String {
    format!(
        "\x1b[1;1H\x1b[38;2;120;120;200;48;2;0;0;0m\
         izanagi_kit roguelike demo  seed={:#018x}  turn={}\x1b[0m\r\n",
        seed, turn,
    )
}
