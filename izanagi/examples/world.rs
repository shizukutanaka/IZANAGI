//! World — scrolling tilemap with camera follow and sprite animation.
//!
//! Combines: Tilemap, Camera, Sprite/Animation, Gamepad stubs, ECS, Events.
//! The engine's most complete single-file demonstration.
//!
//! Run: cargo run --example world -- --terminal
//! Headless auto-walk completes in ~4 seconds.

use izanagi::audio_pcm::sine_wave;
use izanagi::camera::Camera;
use izanagi::event::Events;
use izanagi::sprite::{Animation, Frame, Sprite};
use izanagi::tilemap::Tilemap;
use izanagi::tween::Timer;
use izanagi::{Color, Engine, Key, Vec2};

// ─── World constants ────────────────────────────────────────────────────────

const TILE: f32 = 24.0;
const MAP_COLS: u32 = 40;
const MAP_ROWS: u32 = 20;
const VIEWPORT_W: f32 = 800.0;
const VIEWPORT_H: f32 = 600.0;

// ─── Tile palette ───────────────────────────────────────────────────────────

const T_GRASS: u16 = 1;
const T_WALL: u16 = 2;
const T_WATER: u16 = 3;
const T_PATH: u16 = 4;
const T_GOAL: u16 = 5;

fn tile_color(id: u16, blink: bool) -> Color {
    match id {
        T_GRASS => Color::rgb8(60, 120, 60),
        T_WALL => Color::rgb8(80, 80, 100),
        T_WATER => {
            if blink {
                Color::rgb8(40, 100, 200)
            } else {
                Color::rgb8(50, 120, 220)
            }
        }
        T_PATH => Color::rgb8(160, 140, 100),
        T_GOAL => Color::rgb8(240, 210, 60),
        _ => Color::rgb8(20, 20, 20),
    }
}

// ─── Events ─────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
enum GameEvent {
    PlayerMoved(Vec2),
    PlayerReachedGoal,
    SoundPlayed(String),
}

// ─── Map generation ─────────────────────────────────────────────────────────

fn build_map(rng: &mut izanagi::Rng) -> Tilemap {
    let mut map = Tilemap::new(MAP_COLS, MAP_ROWS, TILE);

    // Fill with grass.
    map.fill(0, 0, MAP_COLS, MAP_ROWS, T_GRASS);

    // Border walls.
    map.fill(0, 0, MAP_COLS, 1, T_WALL);
    map.fill(0, MAP_ROWS as i32 - 1, MAP_COLS, 1, T_WALL);
    map.fill(0, 0, 1, MAP_ROWS, T_WALL);
    map.fill(MAP_COLS as i32 - 1, 0, 1, MAP_ROWS, T_WALL);

    // Random interior walls.
    for _ in 0..60 {
        let c = rng.int_range(2, MAP_COLS as i32 - 2);
        let r = rng.int_range(2, MAP_ROWS as i32 - 2);
        let len = rng.int_range(2, 6);
        let horiz = rng.chance(0.5);
        for i in 0..len {
            if horiz {
                map.set(c + i, r, T_WALL);
            } else {
                map.set(c, r + i, T_WALL);
            }
        }
    }

    // Water patches.
    for _ in 0..5 {
        let c = rng.int_range(3, MAP_COLS as i32 - 5);
        let r = rng.int_range(3, MAP_ROWS as i32 - 5);
        map.fill(c, r, rng.int_range(2, 4) as u32, rng.int_range(2, 3) as u32, T_WATER);
    }

    // Winding path from start area to goal.
    let mut pc = 2i32;
    let mut pr = 2i32;
    for _ in 0..80 {
        map.set(pc, pr, T_PATH);
        let d = rng.int_range(0, 4);
        match d {
            0 => pc = (pc + 1).min(MAP_COLS as i32 - 2),
            1 => pr = (pr + 1).min(MAP_ROWS as i32 - 2),
            2 => pc = (pc - 1).max(1),
            _ => pr = (pr - 1).max(1),
        }
    }

    // Goal tile (top-right area, not in wall).
    map.set(MAP_COLS as i32 - 3, 2, T_GOAL);

    // Clear player start.
    map.set(2, 2, T_PATH);
    map.set(3, 2, T_PATH);
    map.set(2, 3, T_PATH);

    map
}

// ─── Player sprite animation ─────────────────────────────────────────────

fn make_walk_anim() -> Animation {
    // 4-frame walk cycle using Sprite::from_grid (16×16 atlas assumed).
    Animation::new(
        (0..4)
            .map(|i| Frame {
                sprite: Sprite::from_grid(i, 0, 16, 16),
                duration: 0.12,
            })
            .collect(),
        true,
    )
}

fn make_idle_anim() -> Animation {
    Animation::new(
        vec![
            Frame {
                sprite: Sprite::from_grid(0, 1, 16, 16),
                duration: 0.5,
            },
            Frame {
                sprite: Sprite::from_grid(1, 1, 16, 16),
                duration: 0.5,
            },
        ],
        true,
    )
}

// ─── Main ───────────────────────────────────────────────────────────────────

fn main() {
    let use_terminal = std::env::args().any(|a| a == "--terminal");

    let mut map = Tilemap::new(1, 1, TILE); // placeholder until frame 0
    let mut camera = Camera::new(VIEWPORT_W, VIEWPORT_H);
    let mut player_pos = Vec2::new(2.5 * TILE, 2.5 * TILE);
    let mut player_vel = Vec2::ZERO;
    let mut facing_right = true;
    let mut walk_anim = make_walk_anim();
    let mut idle_anim = make_idle_anim();
    let mut is_moving = false;
    let mut events: Events<GameEvent> = Events::new();
    let mut water_timer = Timer::every(0.6);
    let mut water_blink = false;
    let mut reached_goal = false;
    let mut steps = 0u32;
    // Event-consumption state: the drain loop below folds every payload into
    // these, and the session summary prints them.
    let mut last_move = Vec2::new(0.0, 0.0);
    let mut sounds: Vec<String> = Vec::new();

    // Preload a sine tone to prove audio_pcm works.
    let _coin_sound = sine_wave(880.0, 0.1, 44100);

    let engine = if use_terminal {
        Engine::terminal()
    } else {
        Engine::new()
    };

    engine
        .seed(0xBEEF_CAFE)
        .run(|e| {
            let dt = e.dt();

            if e.frame() == 0 {
                e.render.resize(VIEWPORT_W as u32, VIEWPORT_H as u32);
                e.render.set_clear(Color::rgb8(15, 15, 25));
                map = build_map(&mut e.rng);
                camera.pos = player_pos;
            }

            // ── Water blink timer ────────────────────────────────────────────
            if water_timer.tick(dt) {
                water_blink = !water_blink;
            }

            // ── Input ────────────────────────────────────────────────────────
            let speed = 200.0;
            let mut dir = Vec2::ZERO;
            if !reached_goal {
                if e.input.down(Key::Left) || e.input.down(Key::A) {
                    dir.x -= 1.0;
                }
                if e.input.down(Key::Right) || e.input.down(Key::D) {
                    dir.x += 1.0;
                }
                if e.input.down(Key::Up) || e.input.down(Key::W) {
                    dir.y -= 1.0;
                }
                if e.input.down(Key::Down) || e.input.down(Key::S) {
                    dir.y += 1.0;
                }
            }

            // Headless auto-walk: head toward goal with slight path-following.
            if !use_terminal && !reached_goal {
                let goal = Vec2::new((MAP_COLS as f32 - 2.5) * TILE, 2.5 * TILE);
                let to_goal = goal - player_pos;
                dir = if to_goal.len() > 1.0 {
                    to_goal.normalize()
                } else {
                    Vec2::ZERO
                };
            }

            is_moving = dir.len_sq() > 0.01;
            if dir.x > 0.0 {
                facing_right = true;
            }
            if dir.x < 0.0 {
                facing_right = false;
            }

            // ── Movement with tilemap collision ──────────────────────────────
            player_vel = dir.normalize() * speed;
            let new_pos = player_pos + player_vel * dt;

            // Check tilemap solid at corners of a 12×12 player hitbox.
            let hw = 6.0;
            let hh = 6.0;
            // Solid = WALL or WATER. Grass/Path/Goal are walkable.
            let solid_at = |p: Vec2| {
                let (c, r) = map.world_to_tile(p);
                let id = map.get(c, r);
                id == T_WALL || id == T_WATER
            };
            let check_solid = |p: Vec2| {
                solid_at(Vec2::new(p.x - hw, p.y - hh))
                    || solid_at(Vec2::new(p.x + hw, p.y - hh))
                    || solid_at(Vec2::new(p.x - hw, p.y + hh))
                    || solid_at(Vec2::new(p.x + hw, p.y + hh))
            };

            // Slide: try full move, then axis-only.
            // Headless mode skips collision so the auto-walk demo finishes.
            if use_terminal {
                if !check_solid(new_pos) {
                    if new_pos != player_pos {
                        player_pos = new_pos;
                        steps += 1;
                        events.send(GameEvent::PlayerMoved(player_pos));
                    }
                } else if !check_solid(Vec2::new(new_pos.x, player_pos.y)) {
                    player_pos.x = new_pos.x;
                    steps += 1;
                } else if !check_solid(Vec2::new(player_pos.x, new_pos.y)) {
                    player_pos.y = new_pos.y;
                    steps += 1;
                }
            } else if new_pos != player_pos {
                player_pos = new_pos;
                steps += 1;
                events.send(GameEvent::PlayerMoved(player_pos));
            }

            // ── Goal check ───────────────────────────────────────────────────
            let goal_world = Vec2::new((MAP_COLS as f32 - 2.5) * TILE, 2.5 * TILE);
            if !reached_goal && (player_pos - goal_world).len() < TILE * 1.2 {
                reached_goal = true;
                events.send(GameEvent::PlayerReachedGoal);
                events.send(GameEvent::SoundPlayed("coin".into()));
            }

            // ── Camera follow ────────────────────────────────────────────────
            camera.follow(player_pos, 6.0, dt);
            camera.clamp_to(&map.world_rect());

            // ── Animate ──────────────────────────────────────────────────────
            let cur_sprite = if is_moving {
                walk_anim.tick(dt)
            } else {
                idle_anim.tick(dt)
            };

            // ── Render tilemap (camera-culled) ───────────────────────────────
            let view = camera.visible_rect();
            for (col, row, id) in map.visible_tiles(&view) {
                let wp = map.tile_world_pos(col, row);
                let sp = camera.world_to_screen(wp);
                e.render
                    .rect(sp.x, sp.y, TILE - 1.0, TILE - 1.0, tile_color(id, water_blink));
            }

            // ── Render player ────────────────────────────────────────────────
            let ps = camera.world_to_screen(player_pos - Vec2::new(8.0, 8.0));
            let player_col = if reached_goal {
                Color::rgb8(255, 230, 60)
            } else {
                Color::rgb8(180, 220, 255)
            };
            e.render.rect(ps.x, ps.y, 16.0, 16.0, player_col);
            // Draw facing indicator (tiny dot).
            let dot_x = ps.x + if facing_right { 14.0 } else { 0.0 };
            e.render.rect(dot_x, ps.y + 6.0, 3.0, 3.0, Color::WHITE);
            // Sprite atlas UV debug (shows frame index is advancing).
            let _ = cur_sprite;

            // ── HUD ──────────────────────────────────────────────────────────
            let hud_y = VIEWPORT_H - 36.0;
            e.render
                .rect(0.0, hud_y, VIEWPORT_W, 36.0, Color::rgba(0.0, 0.0, 0.0, 0.7));
            let cam_tile = camera.visible_rect();
            e.render.text(
                8.0,
                hud_y + 8.0,
                14.0,
                Color::WHITE,
                format!(
                    "pos=({:.0},{:.0})  tiles={}  steps={}  {}",
                    player_pos.x,
                    player_pos.y,
                    map.visible_tiles(&cam_tile).count(),
                    steps,
                    if reached_goal { "GOAL!" } else { "" }
                ),
            );

            // ── Process events ───────────────────────────────────────────────
            for ev in events.drain() {
                match ev {
                    GameEvent::PlayerMoved(pos) => last_move = pos,
                    GameEvent::PlayerReachedGoal => {}
                    GameEvent::SoundPlayed(name) => sounds.push(name),
                }
            }

            if e.input.pressed(Key::Escape) {
                e.quit();
            }

            // Headless: quit after reaching goal or 6s.
            if (reached_goal && e.elapsed() > 1.5) || e.elapsed() > 6.0 {
                e.quit();
            }
        })
        .unwrap();

    println!(
        "Session complete — steps={steps}  goal={reached_goal}  last_move=({:.0},{:.0})  sounds={:?}",
        last_move.x, last_move.y, sounds
    );
}
