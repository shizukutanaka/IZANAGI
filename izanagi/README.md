# IZANAGI

A Rust game engine that just works.

```rust
use izanagi::Engine;

fn main() {
    Engine::new().run(|e| {
        if e.frame() > 60 { e.quit(); }
    }).unwrap();
}
```

That is the whole engine. One type, one method. No builder. No config. No plugins.

---

## Why

Every Rust game engine asks you to learn its world before you can write a game.
IZANAGI does not. Spawn an entity, draw a rectangle, play a sound. The engine
gets out of your way.

- **Zero dependencies.** Only the standard library.
- **One type.** `Engine` owns everything. Field-access for the rest.
- **Headless first.** Tests run in CI environments unchanged.
- **Pluggable backend.** Swap `NullBackend` for terminal, winit, or wgpu.
- **Deterministic.** Seed the RNG, replay the run.

## A 30-second tour

```rust
use izanagi::{Engine, Key, Color, Vec2};

fn main() {
    Engine::new().seed(42).run(|e| {
        // ECS — sparse, generational, no archetype dance.
        let enemy = e.world.spawn();
        e.world.insert(enemy, Vec2::new(10.0, 5.0));

        // Input — query, no callbacks.
        if e.input.pressed(Key::Space) { /* jump */ }

        // Drawing — immediate, drains every frame.
        e.render.rect(0.0, 0.0, 10.0, 10.0, Color::WHITE);

        // Audio — one call, no setup.
        e.audio.play("coin", 0.8);

        // RNG — deterministic from the seed.
        let dx = e.rng.range(-1.0, 1.0);

        if e.frame() > 60 { e.quit(); }
    }).unwrap();
}
```

## Run a real game

```bash
cargo run --example hello              # 3-line hello world
cargo run --example pong               # complete Pong, ~100 lines
cargo run --example pong -- --terminal # play it in your terminal
cargo run --example particles          # 500 particles/sec, gravity, fade
cargo run --example platformer         # gravity + jumping + swept-AABB
cargo run --example roguelike          # BSP map, combat, screen-shake
cargo run --example world              # tilemap + camera + sprite-anim
```

The terminal backend renders 24-bit colored half-blocks (`▀`) with ANSI escape
sequences. No window system required.

## Modules (24 total)

| Module | Purpose |
| --- | --- |
| `izanagi` | `Engine` — owns everything |
| `world` (ecs) | sparse storage, generational entities |
| `input` | keyboard, mouse, edge events |
| `gamepad` | up to 4 controllers, sticks, triggers, deadzone |
| `render` | immediate-mode draw list |
| `camera` | world↔screen, follow, clamp, zoom, rotation |
| `tilemap` | grid storage, camera culling, solid queries |
| `sprite` | `Sprite`, `Animation`, frame sequencing |
| `audio` | voice mixer |
| `audio_pcm` | WAV loader (PCM 8/16-bit, mono/stereo) + sine generator |
| `assets` | byte cache, filesystem loader |
| `scene` | parent-child 2D transforms |
| `state` | pushdown automaton (menu/play/pause) |
| `tween` | `Tween` (eased animation) + `Timer` (one-shot/repeating) |
| `event` | typed event bus |
| `math` | `Vec2`, `Vec3`, `Mat3`, `Rect` with operator overloads |
| `collide` | AABB, swept AABB, ray, circle |
| `ease` | linear, quad, cubic, back, elastic, bounce, smoothstep |
| `rng` | xorshift64, deterministic |
| `save` | versioned binary save format |
| `error` | one `Error`, one `Result` |
| `backend` | `Backend` trait, `NullBackend`, `TerminalBackend` |
| `debug` | `Metrics` — FPS, worst_ms, rolling history |

## Quality

- **159 tests** — 121 unit + 19 integration (incl. 7 property-based / 200-round fuzz) + 10 benchmark + 9 doctest
- **0 warnings**, **0 dependencies**, **`cargo fmt --check` passes**
- **MSRV: Rust 1.75**
- **CI:** Linux + macOS + Windows
- **~6,500 LOC total** (3,800 source + 1,200 examples + 1,500 tests)

## Design

Three rules:

1. **Customer experience first.** `cargo run --example pong` runs. No setup.
2. **Say no.** Every API surface is a tax.
3. **Demo-driven.** If `pong.rs` gets harder to write, the change is wrong.

Read `ARCHITECTURE.md` for the rationale behind each design decision.

## Contributing

See `CONTRIBUTING.md`. Two rules:

- New code adds tests.
- New code adds no dependencies.

## License

MIT — see [LICENSE](LICENSE).
