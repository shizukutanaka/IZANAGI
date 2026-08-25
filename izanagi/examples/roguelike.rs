//! Roguelike — dungeon explorer. The engine's showpiece.
//!
//! Demonstrates: procedural map (BSP rooms), ECS entities, state machine,
//! event bus, timers, scene graph, and the terminal backend — all in one file.
//!
//! Controls: WASD / arrow keys to move. ESC to quit.
//! Run: cargo run --example roguelike -- --terminal
//!
//! Headless (no args): auto-plays for 3 seconds then prints results.

use izanagi::backend::NullBackend;
use izanagi::event::Events;
use izanagi::state::States;
use izanagi::tween::{Timer, Tween};
use izanagi::{ease, Color, Engine, Key, Rect, Vec2};

// ─── Map constants ──────────────────────────────────────────────────────────

const MAP_W: usize = 40;
const MAP_H: usize = 20;
const TILE_PX: f32 = 16.0;

const FLOOR: u8 = 0;
const WALL: u8 = 1;

// ─── Game world structs ─────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
struct Pos {
    x: i32,
    y: i32,
}

impl Pos {
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
    fn to_vec2(self) -> Vec2 {
        Vec2::new(self.x as f32 * TILE_PX, self.y as f32 * TILE_PX)
    }
}

#[derive(Clone, Copy)]
struct Health {
    hp: i32,
    max_hp: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Team {
    Player,
    Enemy,
}

struct Map {
    tiles: [u8; MAP_W * MAP_H],
}

impl Map {
    fn new() -> Self {
        Self {
            tiles: [WALL; MAP_W * MAP_H],
        }
    }
    fn at(&self, x: i32, y: i32) -> u8 {
        if x < 0 || y < 0 || x >= MAP_W as i32 || y >= MAP_H as i32 {
            return WALL;
        }
        self.tiles[y as usize * MAP_W + x as usize]
    }
    fn set(&mut self, x: i32, y: i32, v: u8) {
        if x < 0 || y < 0 || x >= MAP_W as i32 || y >= MAP_H as i32 {
            return;
        }
        self.tiles[y as usize * MAP_W + x as usize] = v;
    }
    fn carve(&mut self, r: &Rect) {
        let x0 = r.x as i32;
        let y0 = r.y as i32;
        let x1 = x0 + r.w as i32;
        let y1 = y0 + r.h as i32;
        for y in y0..y1 {
            for x in x0..x1 {
                self.set(x, y, FLOOR);
            }
        }
    }
    fn is_walkable(&self, x: i32, y: i32) -> bool {
        self.at(x, y) == FLOOR
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Phase {
    Title,
    Play,
    Dead,
    Win,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct CombatEvent {
    attacker: Team,
    damage: i32,
}

// ─── Procedural map (BSP rooms) ─────────────────────────────────────────────

fn generate_map(rng: &mut izanagi::Rng) -> (Map, Vec<Rect>) {
    let mut map = Map::new();
    let mut rooms: Vec<Rect> = Vec::new();

    for _ in 0..30 {
        let w = rng.int_range(4, 9) as f32;
        let h = rng.int_range(3, 7) as f32;
        let x = rng.int_range(1, (MAP_W as i32 - w as i32 - 1).max(2)) as f32;
        let y = rng.int_range(1, (MAP_H as i32 - h as i32 - 1).max(2)) as f32;
        let room = Rect::new(x, y, w, h);

        let overlap = rooms.iter().any(|r| {
            let pad = Rect::new(room.x - 1.0, room.y - 1.0, room.w + 2.0, room.h + 2.0);
            pad.overlaps(r)
        });
        if !overlap {
            map.carve(&room);
            // Corridor to previous room.
            if let Some(prev) = rooms.last() {
                let cx1 = (prev.x + prev.w / 2.0) as i32;
                let cy1 = (prev.y + prev.h / 2.0) as i32;
                let cx2 = (room.x + room.w / 2.0) as i32;
                let cy2 = (room.y + room.h / 2.0) as i32;
                let (lo_x, hi_x) = if cx1 < cx2 { (cx1, cx2) } else { (cx2, cx1) };
                let (lo_y, hi_y) = if cy1 < cy2 { (cy1, cy2) } else { (cy2, cy1) };
                for x in lo_x..=hi_x {
                    map.set(x, cy1, FLOOR);
                }
                for y in lo_y..=hi_y {
                    map.set(cx2, y, FLOOR);
                }
            }
            rooms.push(room);
        }
    }
    (map, rooms)
}

// ─── Main ────────────────────────────────────────────────────────────────────

fn main() {
    let use_terminal = std::env::args().any(|a| a == "--terminal");

    // ── World state (captured by closure) ──────────────────────────────────
    let mut states = States::new(Phase::Title);
    let mut combat: Events<CombatEvent> = Events::new();
    let mut messages: Vec<String> = Vec::new();

    // Map + entities.
    let mut map = Map::new();
    let mut rooms: Vec<Rect> = Vec::new();
    let mut player_pos = Pos::new(2, 2);
    let mut player_hp = Health { hp: 30, max_hp: 30 };
    let mut enemies: Vec<(Pos, Health)> = Vec::new();
    let mut goal_pos = Pos::new(0, 0);
    let mut steps = 0u32;

    // Tweens / timers.
    let mut flash_timer = Timer::once(0.3);
    let mut shake = Tween::new(4.0, 0.0, 0.3, ease::cubic_out);
    shake.tick(1.0); // pre-advance so it's done at start
    let mut turn_timer = Timer::every(0.18); // headless auto-walk speed

    // ── Engine ─────────────────────────────────────────────────────────────
    let engine = if use_terminal {
        Engine::terminal()
    } else {
        Engine::with_backend(Box::new(NullBackend::new().with_frames(200)))
    };

    engine
        .seed(0x0DEA_DCA7) // same value as the old 0xDEAD_CA7, in equal digit groups
        .run(|e| {
            if e.frame() == 0 {
                e.render.resize(
                    (MAP_W as u32 * TILE_PX as u32).max(640),
                    (MAP_H as u32 * TILE_PX as u32 + 60).max(400),
                );
                e.render.set_clear(Color::rgb8(10, 10, 18));

                // Generate world.
                let (m, r) = generate_map(&mut e.rng);
                map = m;
                rooms = r;
                if let Some(first) = rooms.first() {
                    player_pos = Pos::new((first.x + 1.0) as i32, (first.y + 1.0) as i32);
                }
                if let Some(last) = rooms.last() {
                    goal_pos =
                        Pos::new((last.x + last.w / 2.0) as i32, (last.y + last.h / 2.0) as i32);
                    // Spawn enemies in middle rooms.
                    for room in rooms.iter().skip(1).take(rooms.len().saturating_sub(2)) {
                        if e.rng.chance(0.6) {
                            enemies.push((
                                Pos::new((room.x + 1.0) as i32, (room.y + 1.0) as i32),
                                Health { hp: 5, max_hp: 5 },
                            ));
                        }
                    }
                }
                messages.push("Find the exit! WASD to move.".into());
            }

            let dt = e.dt();
            flash_timer.tick(dt);
            shake.tick(dt);
            turn_timer.tick(dt);

            let bg = Color::rgb8(10, 10, 18);
            let wall_col = Color::rgb8(60, 65, 90);
            let floor_col = Color::rgb8(28, 30, 46);
            let player_col = Color::rgb8(120, 200, 255);
            let enemy_col = Color::rgb8(220, 80, 80);
            let goal_col = Color::rgb8(255, 230, 80);

            match states.current().clone() {
                Phase::Title => {
                    let cx = (MAP_W as f32 * TILE_PX - 260.0) / 2.0;
                    let cy = MAP_H as f32 * TILE_PX / 2.0 - 40.0;
                    e.render
                        .rect(cx - 20.0, cy - 20.0, 300.0, 100.0, Color::rgb8(30, 30, 50));
                    e.render.text(cx, cy, 32.0, Color::WHITE, "DUNGEON DESCENT");
                    e.render.text(
                        cx + 20.0,
                        cy + 40.0,
                        20.0,
                        Color::rgb8(180, 180, 200),
                        "WASD to move",
                    );
                    if e.input.pressed(Key::Space)
                        || e.input.pressed(Key::W)
                        || e.input.pressed(Key::A)
                        || e.input.pressed(Key::S)
                        || e.input.pressed(Key::D)
                        || !use_terminal
                    {
                        states.replace(Phase::Play);
                    }
                }

                Phase::Play => {
                    // ── Input / headless auto-move ───────────────────────────
                    let mut dx = 0i32;
                    let mut dy = 0i32;
                    if use_terminal {
                        if e.input.pressed(Key::W) || e.input.pressed(Key::Up) {
                            dy = -1;
                        }
                        if e.input.pressed(Key::S) || e.input.pressed(Key::Down) {
                            dy = 1;
                        }
                        if e.input.pressed(Key::A) || e.input.pressed(Key::Left) {
                            dx = -1;
                        }
                        if e.input.pressed(Key::D) || e.input.pressed(Key::Right) {
                            dx = 1;
                        }
                    } else if turn_timer.just_finished() {
                        // Auto-walk: head toward goal crudely.
                        let gx = goal_pos.x - player_pos.x;
                        let gy = goal_pos.y - player_pos.y;
                        if gx.abs() > gy.abs() {
                            dx = gx.signum();
                        } else {
                            dy = gy.signum();
                        }
                        if !map.is_walkable(player_pos.x + dx, player_pos.y + dy) {
                            if map.is_walkable(player_pos.x, player_pos.y + gy.signum()) {
                                dx = 0;
                                dy = gy.signum();
                            } else if map.is_walkable(player_pos.x + gx.signum(), player_pos.y) {
                                dy = 0;
                                dx = gx.signum();
                            }
                        }
                    }

                    if e.input.pressed(Key::Escape) {
                        e.quit();
                    }

                    if dx != 0 || dy != 0 {
                        let nx = player_pos.x + dx;
                        let ny = player_pos.y + dy;
                        // Check enemy collision first.
                        let enemy_idx = enemies.iter().position(|(p, _)| p.x == nx && p.y == ny);
                        if let Some(idx) = enemy_idx {
                            let dmg = e.rng.int_range(3, 8);
                            enemies[idx].1.hp -= dmg;
                            combat.send(CombatEvent {
                                attacker: Team::Player,
                                damage: dmg,
                            });
                            messages.push(format!("Hit for {}!", dmg));
                            shake.restart();
                            if enemies[idx].1.hp <= 0 {
                                enemies.remove(idx);
                                messages.push("Enemy slain!".into());
                            }
                            steps += 1;
                        } else if map.is_walkable(nx, ny) {
                            player_pos = Pos::new(nx, ny);
                            steps += 1;
                        }
                    }

                    // Enemy turn: after player moves, each enemy steps toward player.
                    if steps > 0 && (dx != 0 || dy != 0) {
                        let ppos = player_pos;
                        for (epos, ehp) in enemies.iter_mut() {
                            let ex = ppos.x - epos.x;
                            let ey = ppos.y - epos.y;
                            let (mut mx, mut my) = (0i32, 0i32);
                            if ex.abs() > ey.abs() {
                                mx = ex.signum();
                            } else {
                                my = ey.signum();
                            }
                            let (tx, ty) = (epos.x + mx, epos.y + my);
                            if map.is_walkable(tx, ty) && Pos::new(tx, ty) != ppos {
                                epos.x = tx;
                                epos.y = ty;
                            }
                            // Adjacent attack.
                            if (epos.x - ppos.x).abs() <= 1 && (epos.y - ppos.y).abs() <= 1 {
                                let dmg = e.rng.int_range(1, 5);
                                player_hp.hp -= dmg;
                                combat.send(CombatEvent {
                                    attacker: Team::Enemy,
                                    damage: dmg,
                                });
                                messages.push(format!("You take {}!", dmg));
                                flash_timer.reset();
                                let _ = ehp.hp; // suppress unused
                            }
                        }
                    }

                    // Win / lose checks.
                    if player_hp.hp <= 0 {
                        states.replace(Phase::Dead);
                    }
                    if player_pos == goal_pos {
                        states.replace(Phase::Win);
                    }

                    // ── Draw map ────────────────────────────────────────────
                    let shake_off = if !shake.done() { shake.value() } else { 0.0 };
                    let off_x = shake_off * e.rng.range(-1.0, 1.0);
                    let off_y = shake_off * e.rng.range(-1.0, 1.0);

                    for ty in 0..MAP_H as i32 {
                        for tx in 0..MAP_W as i32 {
                            let px = tx as f32 * TILE_PX + off_x;
                            let py = ty as f32 * TILE_PX + off_y;
                            let col = if map.at(tx, ty) == WALL {
                                wall_col
                            } else {
                                floor_col
                            };
                            e.render.rect(px, py, TILE_PX - 1.0, TILE_PX - 1.0, col);
                        }
                    }

                    // Goal.
                    e.render.rect(
                        goal_pos.to_vec2().x + off_x,
                        goal_pos.to_vec2().y + off_y,
                        TILE_PX - 1.0,
                        TILE_PX - 1.0,
                        goal_col,
                    );

                    // Enemies.
                    for (ep, eh) in &enemies {
                        let v = ep.to_vec2();
                        e.render.rect(
                            v.x + 2.0 + off_x,
                            v.y + 2.0 + off_y,
                            TILE_PX - 4.0,
                            TILE_PX - 4.0,
                            enemy_col,
                        );
                        // HP bar.
                        let bar = (eh.hp as f32 / eh.max_hp as f32) * (TILE_PX - 2.0);
                        e.render.rect(
                            v.x + off_x,
                            v.y - 4.0 + off_y,
                            bar,
                            3.0,
                            Color::rgb8(200, 50, 50),
                        );
                    }

                    // Player — flash red when hit.
                    let pcol = if flash_timer.fraction() < 0.8 && !flash_timer.finished() {
                        Color::rgb8(255, 80, 80)
                    } else {
                        player_col
                    };
                    let pv = player_pos.to_vec2();
                    e.render.rect(
                        pv.x + 2.0 + off_x,
                        pv.y + 2.0 + off_y,
                        TILE_PX - 4.0,
                        TILE_PX - 4.0,
                        pcol,
                    );

                    // HUD.
                    let hud_y = MAP_H as f32 * TILE_PX + 4.0;
                    e.render
                        .rect(0.0, hud_y, 400.0, 50.0, Color::rgb8(18, 18, 30));
                    let hp_ratio = (player_hp.hp as f32 / player_hp.max_hp as f32).max(0.0);
                    e.render.rect(
                        4.0,
                        hud_y + 4.0,
                        120.0 * hp_ratio,
                        12.0,
                        Color::rgb8(80, 200, 80),
                    );
                    e.render
                        .rect(4.0, hud_y + 4.0, 120.0, 12.0, Color::rgba(1.0, 1.0, 1.0, 0.15));
                    e.render.text(
                        130.0,
                        hud_y + 4.0,
                        14.0,
                        Color::WHITE,
                        format!(
                            "HP {}/{}  steps={}  enemies={}",
                            player_hp.hp,
                            player_hp.max_hp,
                            steps,
                            enemies.len()
                        ),
                    );
                    // Last message.
                    if let Some(msg) = messages.last() {
                        e.render.text(
                            4.0,
                            hud_y + 28.0,
                            12.0,
                            Color::rgb8(200, 200, 150),
                            if msg.len() > 55 {
                                &msg[..55]
                            } else {
                                msg.as_str()
                            },
                        );
                    }

                    // Combat event log.
                    let _events = combat.drain();
                }

                Phase::Dead => {
                    e.render.rect(
                        0.0,
                        0.0,
                        MAP_W as f32 * TILE_PX,
                        MAP_H as f32 * TILE_PX,
                        Color::rgba(0.4, 0.0, 0.0, 0.7),
                    );
                    e.render
                        .text(120.0, 150.0, 48.0, Color::rgb8(255, 80, 80), "YOU DIED");
                    e.render.text(
                        140.0,
                        210.0,
                        20.0,
                        Color::rgb8(200, 200, 200),
                        format!(
                            "Steps: {}  Enemies defeated: {}",
                            steps,
                            rooms.len().saturating_sub(enemies.len() + 1)
                        ),
                    );
                    if e.elapsed() > 2.5 {
                        e.quit();
                    }
                }

                Phase::Win => {
                    e.render.rect(
                        0.0,
                        0.0,
                        MAP_W as f32 * TILE_PX,
                        MAP_H as f32 * TILE_PX,
                        Color::rgba(0.0, 0.3, 0.0, 0.6),
                    );
                    e.render.text(100.0, 150.0, 48.0, goal_col, "ESCAPED!");
                    e.render
                        .text(110.0, 210.0, 20.0, Color::WHITE, format!("Steps: {}", steps));
                    if e.elapsed() > 2.5 {
                        e.quit();
                    }
                }
            }

            states.end_frame();
            if e.elapsed() > 60.0 {
                e.quit();
            }
            let _ = bg;
        })
        .unwrap();

    println!("Final state: {:?}", states.current());
    println!("Steps taken: {}", steps);
    println!("Rooms generated: {}", rooms.len());
    println!("Enemies remaining: {}", enemies.len());
}
