//! Performance benchmarks — timing-based, no external dependencies.
//!
//! Run: `cargo test --test bench -- --nocapture`
//!
//! Each benchmark runs 10,000 iterations, discards the first 100 as
//! warm-up, then reports median and 99th-percentile nanoseconds.

use izanagi::backend::NullBackend;
use izanagi::{Engine, Vec2, World};
use std::time::Instant;

// ─────────────────────────────────────────────────────────────────
// Harness
// ─────────────────────────────────────────────────────────────────

fn bench(name: &str, iterations: usize, warmup: usize, mut f: impl FnMut()) {
    // Warm up.
    for _ in 0..warmup {
        f();
    }

    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let t = Instant::now();
        f();
        samples.push(t.elapsed().as_nanos() as u64);
    }

    samples.sort_unstable();
    let median = samples[samples.len() / 2];
    let p99 = samples[samples.len() * 99 / 100];
    let min = samples[0];
    println!("[{name:40}] min={min:6}ns  median={median:6}ns  p99={p99:6}ns");
}

// ─────────────────────────────────────────────────────────────────
// ECS
// ─────────────────────────────────────────────────────────────────

#[test]
fn bench_ecs_spawn() {
    let n = 10_000;
    bench("ecs::spawn x1000", n, 100, || {
        let mut w = World::new();
        for _ in 0..1000 {
            w.spawn();
        }
    });
}

#[test]
fn bench_ecs_insert() {
    let n = 10_000;
    let mut world = World::new();
    let entities: Vec<_> = (0..1000).map(|_| world.spawn()).collect();

    bench("ecs::insert<Vec2> x1000", n, 100, || {
        for &e in &entities {
            world.insert(e, Vec2::new(1.0, 2.0));
        }
    });
}

#[test]
fn bench_ecs_query_allocating() {
    let mut w = World::new();
    for i in 0..1000u32 {
        let e = w.spawn();
        w.insert(e, Vec2::new(i as f32, 0.0));
    }
    bench("ecs::query<Vec2> x1 (allocates)", 10_000, 100, || {
        let results = w.query::<Vec2>();
        assert_eq!(results.len(), 1000);
    });
}

#[test]
fn bench_ecs_for_each_zero_alloc() {
    let mut w = World::new();
    for i in 0..1000u32 {
        let e = w.spawn();
        w.insert(e, Vec2::new(i as f32, 0.0));
    }
    bench("ecs::for_each<Vec2> x1 (zero-alloc)", 10_000, 100, || {
        let mut sum = 0.0f32;
        w.for_each::<Vec2>(|_, v| sum += v.x);
        assert!(sum > 0.0);
    });
}

#[test]
fn bench_ecs_for_each_mut() {
    let mut w = World::new();
    for _ in 0..1000u32 {
        let e = w.spawn();
        w.insert(e, Vec2::ZERO);
    }
    bench("ecs::for_each_mut<Vec2> x1", 10_000, 100, || {
        w.for_each_mut::<Vec2>(|_, v| {
            v.x += 1.0;
        });
    });
}

// ─────────────────────────────────────────────────────────────────
// Math
// ─────────────────────────────────────────────────────────────────

#[test]
fn bench_vec2_math() {
    bench("Vec2::normalize x10000", 10_000, 100, || {
        let mut v = Vec2::new(3.0, 4.0);
        for _ in 0..10_000 {
            v = (v + Vec2::ONE).normalize();
        }
        assert!(v.len() > 0.9);
    });
}

#[test]
fn bench_mat3_multiply() {
    use izanagi::Mat3;
    bench("Mat3::mul x10000", 10_000, 100, || {
        let r = Mat3::rotation(0.01);
        let mut m = Mat3::IDENTITY;
        for _ in 0..10_000 {
            m = m * r;
        }
    });
}

// ─────────────────────────────────────────────────────────────────
// RNG
// ─────────────────────────────────────────────────────────────────

#[test]
fn bench_rng_f32() {
    use izanagi::Rng;
    let mut rng = Rng::new(1);
    bench("Rng::f32 x10000", 10_000, 100, || {
        let mut s = 0.0f32;
        for _ in 0..10_000 {
            s += rng.f32();
        }
        assert!(s > 0.0);
    });
}

// ─────────────────────────────────────────────────────────────────
// Full frame
// ─────────────────────────────────────────────────────────────────

#[test]
fn bench_full_frame_1000_entities() {
    use izanagi::{Color, Vec2};

    // Simulate one frame of a physics+render game.
    let mut world = World::new();
    for _ in 0..1000 {
        let e = world.spawn();
        world.insert(e, Vec2::ZERO);
    }

    bench("full frame: update 1000 entities + 1000 draw calls", 1_000, 50, || {
        // Physics update.
        world.for_each_mut::<Vec2>(|_, p| {
            p.x += 0.016;
        });

        // Draw.
        let mut render = izanagi::Render::new();
        world.for_each::<Vec2>(|_, p| {
            render.rect(p.x, p.y, 8.0, 8.0, Color::WHITE);
        });
        let (_c, list, _t) = render.drain();
        assert_eq!(list.len(), 1000);
    });
}

#[test]
fn bench_engine_run_60_frames() {
    bench("Engine::run 60 frames, 100 draw calls/frame", 500, 50, || {
        let backend = Box::new(NullBackend::new().with_frames(60));
        Engine::with_backend(backend)
            .run(|e| {
                for _ in 0..100 {
                    e.render.rect(0.0, 0.0, 10.0, 10.0, izanagi::Color::WHITE);
                }
            })
            .unwrap();
    });
}
