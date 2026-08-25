//! Integration tests — public API contracts only.
//!
//! These exercise IZANAGI the way a user would: through the crate root.

use izanagi::backend::NullBackend;
use izanagi::{Color, Engine, Key, Rect, Vec2};

#[test]
fn full_game_loop_runs_for_n_frames() {
    let backend = Box::new(NullBackend::new().with_frames(100));
    let mut frames_seen = 0u64;

    Engine::with_backend(backend)
        .run(|e| {
            frames_seen = e.frame();
            e.render.rect(0.0, 0.0, 10.0, 10.0, Color::WHITE);
        })
        .unwrap();

    assert_eq!(frames_seen, 99);
}

#[test]
fn ecs_components_persist_across_frames() {
    let backend = Box::new(NullBackend::new().with_frames(5));
    let mut final_count = 0u64;

    Engine::with_backend(backend)
        .run(|e| {
            if e.frame() == 0 {
                for _ in 0..50 {
                    e.world.spawn();
                }
            }
            final_count = e.world.len();
        })
        .unwrap();

    assert_eq!(final_count, 50);
}

#[test]
fn input_edge_events_clear_between_frames() {
    let backend = Box::new(NullBackend::new().with_frames(3));
    let mut frame_pressed_counts = Vec::new();

    Engine::with_backend(backend)
        .run(|e| {
            if e.frame() == 0 {
                e.input.on_key_down(Key::Space);
            }
            frame_pressed_counts.push(e.input.pressed(Key::Space));
        })
        .unwrap();

    assert_eq!(frame_pressed_counts, vec![true, false, false]);
}

#[test]
fn render_drains_each_frame() {
    let backend = Box::new(NullBackend::new().with_frames(3));
    let mut peak = 0;

    Engine::with_backend(backend)
        .run(|e| {
            for _ in 0..10 {
                e.render.rect(0.0, 0.0, 1.0, 1.0, Color::WHITE);
            }
            peak = peak.max(e.render.len());
        })
        .unwrap();

    // After drain at end of frame, the next frame starts with 0.
    assert_eq!(peak, 10);
}

#[test]
fn collision_with_swept_aabb() {
    let player = Rect::new(0.0, 0.0, 10.0, 10.0);
    let wall = Rect::new(50.0, 0.0, 10.0, 10.0);
    let hit = izanagi::collide::swept_aabb(&player, Vec2::new(100.0, 0.0), &wall);
    assert!(hit.is_some());
    assert!(hit.unwrap().t > 0.0 && hit.unwrap().t < 1.0);
}

#[test]
fn save_roundtrip_via_temp_file() {
    let dir = std::env::temp_dir();
    let path = dir.join("izanagi_integration_save.dat");
    let payload = b"level=5;hp=80;coins=42";

    izanagi::save::Save::write(&path, 1, payload).unwrap();
    let (version, bytes) = izanagi::save::Save::read(&path).unwrap();

    assert_eq!(version, 1);
    assert_eq!(bytes, payload);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn deterministic_rng_with_seed() {
    let mut a = Engine::new().seed(42);
    let mut b = Engine::new().seed(42);
    let av: Vec<u64> = (0..20).map(|_| a.rng.u64()).collect();
    let bv: Vec<u64> = (0..20).map(|_| b.rng.u64()).collect();
    assert_eq!(av, bv);
}

#[test]
fn audio_plays_and_stops_voices() {
    let backend = Box::new(NullBackend::new().with_frames(5));
    let mut max_voices = 0;

    Engine::with_backend(backend)
        .run(|e| {
            if e.frame() == 0 {
                for i in 0..10 {
                    e.audio.play(&format!("snd-{i}"), 0.5);
                }
            }
            if e.frame() == 2 {
                e.audio.stop_all();
            }
            max_voices = max_voices.max(e.audio.voice_count());
        })
        .unwrap();

    assert_eq!(max_voices, 10);
}

#[test]
fn scene_world_transform_walks_parent_chain() {
    use izanagi::Mat3;

    let mut e = Engine::new();
    let root = e.scene.add();
    let mid = e.scene.add_child(root);
    let leaf = e.scene.add_child(mid);

    e.scene
        .set_local(root, Mat3::translation(Vec2::new(10.0, 0.0)));
    e.scene
        .set_local(mid, Mat3::translation(Vec2::new(20.0, 0.0)));
    e.scene
        .set_local(leaf, Mat3::translation(Vec2::new(5.0, 0.0)));

    let p = e.scene.world(leaf).transform_point(Vec2::ZERO);
    assert!((p.x - 35.0).abs() < 1e-4);
}

#[test]
fn state_machine_pause_and_resume_pattern() {
    use izanagi::state::States;

    #[derive(Clone, Debug, PartialEq)]
    enum S {
        Menu,
        Play,
        Pause,
    }

    let mut s = States::new(S::Menu);
    s.replace(S::Play);
    assert_eq!(*s.current(), S::Play);
    s.push(S::Pause);
    assert_eq!(*s.current(), S::Pause);
    s.pop();
    assert_eq!(*s.current(), S::Play);
}

#[test]
fn assets_in_memory_insert_and_retrieve() {
    let mut e = Engine::new();
    let h = e.assets.insert("sword.png", vec![0xCA, 0xFE, 0xBA, 0xBE]);
    let bytes = e.assets.get(h).unwrap();
    assert_eq!(bytes, &[0xCA, 0xFE, 0xBA, 0xBE]);
}

#[test]
fn engine_quit_exits_loop_immediately() {
    let backend = Box::new(NullBackend::new().with_frames(10_000));
    let mut frames = 0u64;
    Engine::with_backend(backend)
        .run(|e| {
            frames += 1;
            if frames == 5 {
                e.quit();
            }
        })
        .unwrap();
    assert_eq!(frames, 5);
}

// ─── Property-based / stress tests ──────────────────────────────────────────

#[test]
fn property_ecs_spawn_despawn_200_rounds() {
    let mut world = izanagi::World::new();
    #[derive(Debug, PartialEq)]
    struct Hp(i32);
    let mut rng = izanagi::Rng::new(0xFEED);
    let mut alive: Vec<izanagi::Entity> = Vec::new();
    for _ in 0..200 {
        // Spawn or despawn randomly.
        if alive.is_empty() || rng.chance(0.6) {
            let e = world.spawn();
            world.insert(e, Hp(rng.int_range(1, 100)));
            alive.push(e);
        } else {
            let idx = rng.int_range(0, alive.len() as i32) as usize;
            let e = alive.swap_remove(idx);
            world.despawn(e);
            assert_eq!(world.get::<Hp>(e), None);
        }
    }
    assert_eq!(world.len(), alive.len() as u64);
}

#[test]
fn property_rng_no_duplicates_over_1000() {
    let mut rng = izanagi::Rng::new(42);
    let mut seen = std::collections::HashSet::new();
    for _ in 0..1000 {
        seen.insert(rng.u64());
    }
    // xorshift64 has full period; 1000 draws should be unique.
    assert_eq!(seen.len(), 1000);
}

#[test]
fn property_camera_world_screen_roundtrip_200() {
    use izanagi::camera::Camera;
    use izanagi::Vec2;
    let mut rng = izanagi::Rng::new(7);
    let mut cam = Camera::new(800.0, 600.0);
    for _ in 0..200 {
        cam.pos = Vec2::new(rng.range(-500.0, 500.0), rng.range(-500.0, 500.0));
        cam.zoom = rng.range(0.25, 4.0);
        let world = Vec2::new(rng.range(-1000.0, 1000.0), rng.range(-1000.0, 1000.0));
        let screen = cam.world_to_screen(world);
        let back = cam.screen_to_world(screen);
        assert!((back.x - world.x).abs() < 0.1, "x drift {}", (back.x - world.x).abs());
        assert!((back.y - world.y).abs() < 0.1, "y drift {}", (back.y - world.y).abs());
    }
}

#[test]
fn property_tilemap_set_get_200() {
    use izanagi::tilemap::Tilemap;
    let mut map = Tilemap::new(50, 50, 16.0);
    let mut rng = izanagi::Rng::new(99);
    for _ in 0..200 {
        let c = rng.int_range(0, 50);
        let r = rng.int_range(0, 50);
        let id = rng.int_range(1, 10) as u16;
        map.set(c, r, id);
        assert_eq!(map.get(c, r), id);
    }
}

#[test]
fn property_animation_never_panics_200() {
    use izanagi::sprite::{Animation, Frame, Sprite};
    let mut rng = izanagi::Rng::new(55);
    for _ in 0..200 {
        let n = rng.int_range(1, 8) as usize;
        let frames: Vec<Frame> = (0..n)
            .map(|i| Frame {
                sprite: Sprite::from_grid(i as u32, 0, 16, 16),
                duration: rng.range(0.05, 0.5),
            })
            .collect();
        let looping = rng.chance(0.5);
        let mut anim = Animation::new(frames, looping);
        // Advance by a random amount — must never panic.
        anim.tick(rng.range(0.0, 5.0));
    }
}

#[test]
fn property_save_roundtrip_200() {
    let mut rng = izanagi::Rng::new(13);
    for i in 0..200u16 {
        let len = rng.int_range(0, 128) as usize;
        let data: Vec<u8> = (0..len).map(|_| rng.u32() as u8).collect();
        let encoded = izanagi::save::Save::encode(i, &data);
        let (version, decoded) = izanagi::save::Save::parse(&encoded).unwrap();
        assert_eq!(version, i);
        assert_eq!(decoded, data);
    }
}

#[test]
fn property_collision_swept_aabb_200() {
    use izanagi::collide::swept_aabb;
    use izanagi::{Rect, Vec2};
    let mut rng = izanagi::Rng::new(77);
    for _ in 0..200 {
        let a = Rect::new(rng.range(0.0, 100.0), rng.range(0.0, 100.0), 10.0, 10.0);
        let b = Rect::new(rng.range(0.0, 100.0), rng.range(0.0, 100.0), 10.0, 10.0);
        let motion = Vec2::new(rng.range(-50.0, 50.0), rng.range(-50.0, 50.0));
        if let Some(hit) = swept_aabb(&a, motion, &b) {
            assert!(hit.t >= 0.0 && hit.t <= 1.0, "t out of range: {}", hit.t);
        }
    }
}
