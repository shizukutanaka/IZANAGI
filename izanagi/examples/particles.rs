//! Particles — fountain of color. Demonstrates RNG + math + ECS at scale.
//!
//! Spawns 500 particles per second, each with random velocity, gravity,
//! and an eased fade. Quits after 3 seconds in headless mode.

use izanagi::{ease, Color, Engine, Vec2};

#[derive(Clone, Copy)]
struct Particle {
    pos: Vec2,
    vel: Vec2,
    age: f32,
    life: f32,
    color: Color,
}

fn main() {
    let center = Vec2::new(400.0, 500.0);
    let mut particles: Vec<Particle> = Vec::with_capacity(2000);
    let mut spawn_accum = 0.0_f32;

    let use_terminal = std::env::args().any(|a| a == "--terminal");
    let engine = if use_terminal {
        Engine::terminal()
    } else {
        Engine::new()
    };

    engine
        .seed(0xDEADBEEF)
        .run(|e| {
            if e.frame() == 0 {
                e.render.resize(800, 600);
                e.render.set_clear(Color::rgb8(8, 8, 16));
            }
            let dt = e.dt();

            // Spawn rate: 500 / sec.
            spawn_accum += dt * 500.0;
            while spawn_accum >= 1.0 && particles.len() < 2000 {
                spawn_accum -= 1.0;
                let angle = e.rng.range(-2.5, -0.6); // upward fan
                let speed = e.rng.range(120.0, 280.0);
                let life = e.rng.range(1.0, 2.5);
                let hue = e.rng.range(0.0, 1.0);
                particles.push(Particle {
                    pos: center,
                    vel: Vec2::new(speed * angle.cos(), speed * angle.sin()),
                    age: 0.0,
                    life,
                    color: hsv(hue, 0.8, 1.0),
                });
            }

            // Update + draw.
            let g = Vec2::new(0.0, 220.0);
            particles.retain_mut(|p| {
                p.age += dt;
                p.vel += g * dt;
                p.pos += p.vel * dt;
                if p.age >= p.life {
                    return false;
                }
                let t = 1.0 - p.age / p.life;
                let a = ease::cubic_out(t);
                let c = Color { a, ..p.color };
                e.render.rect(p.pos.x - 2.0, p.pos.y - 2.0, 4.0, 4.0, c);
                true
            });

            if e.frame() % 30 == 0 {
                println!("t={:.2}s  particles={}", e.elapsed(), particles.len());
            }

            if e.elapsed() > 3.0 {
                e.quit();
            }
        })
        .unwrap();
}

/// Hue/saturation/value to RGB. Mainstream formula.
fn hsv(h: f32, s: f32, v: f32) -> Color {
    let h = (h.fract() * 6.0).max(0.0);
    let i = h.floor() as i32;
    let f = h - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    let (r, g, b) = match i % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    Color::rgba(r, g, b, 1.0)
}
