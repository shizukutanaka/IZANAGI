//! Platformer — gravity, jumping, swept-AABB collision, state machine.
//!
//! Single-screen platformer demonstrating most of the engine in one file.
//! Headless run quits after 4 seconds.

use izanagi::state::States;
use izanagi::{collide, Color, Engine, Key, Rect, Vec2};

#[derive(Clone, Debug, PartialEq)]
enum Phase {
    Title,
    Play,
    Win,
}

struct Player {
    pos: Vec2,
    vel: Vec2,
    on_ground: bool,
}

const W: f32 = 800.0;
const H: f32 = 600.0;
const PLAYER_SIZE: f32 = 24.0;
const GRAVITY: f32 = 1500.0;
const JUMP_VEL: f32 = -520.0;
const RUN_SPEED: f32 = 280.0;

fn main() {
    let platforms = vec![
        Rect::new(0.0, H - 32.0, W, 32.0), // floor
        Rect::new(120.0, 470.0, 140.0, 16.0),
        Rect::new(360.0, 380.0, 140.0, 16.0),
        Rect::new(580.0, 290.0, 160.0, 16.0),
        Rect::new(300.0, 200.0, 100.0, 16.0),
    ];
    let goal = Rect::new(330.0, 170.0, 30.0, 30.0);

    let mut player = Player {
        pos: Vec2::new(40.0, H - 100.0),
        vel: Vec2::ZERO,
        on_ground: false,
    };
    let mut states = States::new(Phase::Title);

    let use_terminal = std::env::args().any(|a| a == "--terminal");
    let engine = if use_terminal {
        Engine::terminal()
    } else {
        Engine::new()
    };

    engine
        .run(|e| {
            if e.frame() == 0 {
                e.render.resize(W as u32, H as u32);
                e.render.set_clear(Color::rgb8(20, 24, 38));
            }
            let dt = e.dt();

            match states.current().clone() {
                Phase::Title => {
                    e.render.text(
                        W / 2.0 - 150.0,
                        H / 2.0,
                        32.0,
                        Color::WHITE,
                        "PLATFORMER — press Space",
                    );
                    if e.input.pressed(Key::Space) || e.elapsed() > 0.5 {
                        states.replace(Phase::Play);
                    }
                }
                Phase::Play => {
                    // Horizontal input.
                    let mut dx = 0.0;
                    if e.input.down(Key::Left) || e.input.down(Key::A) {
                        dx -= 1.0;
                    }
                    if e.input.down(Key::Right) || e.input.down(Key::D) {
                        dx += 1.0;
                    }
                    player.vel.x = dx * RUN_SPEED;

                    // Headless test driver: walk right, jump occasionally.
                    if !use_terminal {
                        player.vel.x = RUN_SPEED;
                        if player.on_ground && (e.frame() % 60 == 30) {
                            player.vel.y = JUMP_VEL;
                        }
                    }

                    // Jump.
                    if (e.input.pressed(Key::Space) || e.input.pressed(Key::W)) && player.on_ground
                    {
                        player.vel.y = JUMP_VEL;
                    }

                    // Gravity.
                    player.vel.y += GRAVITY * dt;

                    // Collide swept against every platform.
                    let motion = player.vel * dt;
                    let aabb = Rect::new(player.pos.x, player.pos.y, PLAYER_SIZE, PLAYER_SIZE);
                    let mut t_remaining = 1.0;
                    let mut motion = motion;
                    player.on_ground = false;

                    for _ in 0..4 {
                        let mut earliest: Option<(f32, Vec2)> = None;
                        for p in &platforms {
                            if let Some(hit) = collide::swept_aabb(&aabb, motion * t_remaining, p) {
                                if earliest.map_or(true, |(t, _)| hit.t < t) {
                                    earliest = Some((hit.t, hit.normal));
                                }
                            }
                        }
                        match earliest {
                            Some((t, n)) => {
                                player.pos += motion * (t_remaining * t);
                                // Slide along the surface.
                                if n.y < 0.0 {
                                    player.on_ground = true;
                                }
                                let dot = player.vel.x * n.x + player.vel.y * n.y;
                                player.vel =
                                    Vec2::new(player.vel.x - n.x * dot, player.vel.y - n.y * dot);
                                motion = player.vel * dt;
                                t_remaining *= 1.0 - t;
                                if t_remaining < 1e-4 {
                                    break;
                                }
                            }
                            None => {
                                player.pos += motion * t_remaining;
                                break;
                            }
                        }
                    }

                    // Clamp to screen horizontally.
                    player.pos.x = player.pos.x.clamp(0.0, W - PLAYER_SIZE);

                    // Win check.
                    let body = Rect::new(player.pos.x, player.pos.y, PLAYER_SIZE, PLAYER_SIZE);
                    if body.overlaps(&goal) {
                        states.replace(Phase::Win);
                    }

                    // Draw platforms.
                    for p in &platforms {
                        e.render.rect(p.x, p.y, p.w, p.h, Color::rgb8(70, 90, 130));
                    }
                    e.render
                        .rect(goal.x, goal.y, goal.w, goal.h, Color::rgb8(255, 220, 80));
                    e.render.rect(
                        player.pos.x,
                        player.pos.y,
                        PLAYER_SIZE,
                        PLAYER_SIZE,
                        Color::WHITE,
                    );
                }
                Phase::Win => {
                    e.render.text(
                        W / 2.0 - 80.0,
                        H / 2.0,
                        48.0,
                        Color::rgb8(255, 220, 80),
                        "YOU WIN!",
                    );
                    if e.elapsed() > 3.5 {
                        e.quit();
                    }
                }
            }

            states.end_frame();
            if e.elapsed() > 4.0 {
                e.quit();
            }
        })
        .unwrap();

    println!("Platformer ended in state: {:?}", states.current());
    println!("Player final pos: ({:.0}, {:.0})", player.pos.x, player.pos.y);
}
