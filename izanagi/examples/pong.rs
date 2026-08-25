//! Pong — a complete game in one file.
//!
//! Two paddles, one ball, score. Pass `--terminal` to play it in the terminal.
//! This is the demo. If this feels good, the engine is right.

use izanagi::{Color, Engine, Key};

struct Paddle {
    x: f32,
    y: f32,
    h: f32,
    speed: f32,
    score: u32,
    up: Key,
    down: Key,
}

struct Ball {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    r: f32,
}

fn main() {
    let (w, h) = (800.0, 600.0);
    let mut left = Paddle {
        x: 20.0,
        y: h / 2.0 - 40.0,
        h: 80.0,
        speed: 300.0,
        score: 0,
        up: Key::W,
        down: Key::S,
    };
    let mut right = Paddle {
        x: w - 30.0,
        y: h / 2.0 - 40.0,
        h: 80.0,
        speed: 300.0,
        score: 0,
        up: Key::Up,
        down: Key::Down,
    };
    let mut ball = Ball {
        x: w / 2.0,
        y: h / 2.0,
        vx: 250.0,
        vy: 150.0,
        r: 8.0,
    };

    let use_terminal = std::env::args().any(|a| a == "--terminal");
    let engine = if use_terminal {
        Engine::terminal()
    } else {
        Engine::new()
    };

    engine
        .run(|e| {
            if e.frame() == 0 {
                e.render.resize(w as u32, h as u32);
                e.render.set_clear(Color::rgb8(18, 18, 24));
            }

            let dt = e.dt();

            // Movement
            if e.input.down(left.up) {
                left.y -= left.speed * dt;
            }
            if e.input.down(left.down) {
                left.y += left.speed * dt;
            }
            if e.input.down(right.up) {
                right.y -= right.speed * dt;
            }
            if e.input.down(right.down) {
                right.y += right.speed * dt;
            }
            left.y = left.y.clamp(0.0, h - left.h);
            right.y = right.y.clamp(0.0, h - right.h);

            // Ball physics
            ball.x += ball.vx * dt;
            ball.y += ball.vy * dt;
            if ball.y < ball.r || ball.y > h - ball.r {
                ball.vy = -ball.vy;
            }

            // Paddle collision (AABB vs point)
            let hit_left = ball.x - ball.r <= left.x + 10.0
                && ball.y >= left.y
                && ball.y <= left.y + left.h
                && ball.vx < 0.0;
            let hit_right = ball.x + ball.r >= right.x
                && ball.y >= right.y
                && ball.y <= right.y + right.h
                && ball.vx > 0.0;
            if hit_left || hit_right {
                ball.vx = -ball.vx * 1.05;
                e.audio.play("bounce", 0.5);
            }

            // Scoring
            if ball.x < 0.0 {
                right.score += 1;
                ball = Ball {
                    x: w / 2.0,
                    y: h / 2.0,
                    vx: 250.0,
                    vy: 150.0,
                    r: 8.0,
                };
            } else if ball.x > w {
                left.score += 1;
                ball = Ball {
                    x: w / 2.0,
                    y: h / 2.0,
                    vx: -250.0,
                    vy: 150.0,
                    r: 8.0,
                };
            }

            // Quit
            if e.input.pressed(Key::Escape) {
                e.quit();
            }

            // Draw
            let fg = Color::WHITE;
            e.render.rect(left.x, left.y, 10.0, left.h, fg);
            e.render.rect(right.x, right.y, 10.0, right.h, fg);
            e.render
                .rect(ball.x - ball.r, ball.y - ball.r, ball.r * 2.0, ball.r * 2.0, fg);
            e.render.text(
                w / 2.0 - 60.0,
                20.0,
                32.0,
                fg,
                format!("{}  :  {}", left.score, right.score),
            );

            // Headless demo: quit after 5 simulated seconds so `cargo run` returns.
            if e.elapsed() > 5.0 {
                println!("Final score: {} - {}", left.score, right.score);
                println!("Draws submitted: {}", e.render.len());
                e.quit();
            }
        })
        .unwrap();
}
