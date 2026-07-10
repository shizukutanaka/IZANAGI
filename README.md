# IZANAGI

A Cargo workspace with two crates that split one product in half:

| Crate | Role | Design center |
| --- | --- | --- |
| [`izanagi`](./izanagi/) | Real-time engine | `f32`, immediate-mode rendering, one `Engine` type — get a game on screen fast |
| [`izanagi_kit`](./izanagi_kit/) | Deterministic simulation kit | Integers/fixed-point only, bit-exact replay across platforms |

They compose: [`izanagi/examples/kit_bridge.rs`](./izanagi/examples/kit_bridge.rs)
runs a roguelike turn loop entirely in `izanagi_kit` (seeded RNG →
procedural dungeon → A* → field-of-view), renders it through `izanagi`'s
`Engine`, and asserts — not just claims — that crossing the bridge into the
engine's frame loop cannot change a single bit of the simulation's world-hash
trace.

```
cargo test --workspace   # 3362 tests: 188 engine + 3174 kit
```

---

## `izanagi` — the engine

```rust
use izanagi::Engine;

fn main() {
    Engine::new().run(|e| {
        if e.frame() > 60 { e.quit(); }
    }).unwrap();
}
```

That is the whole engine. One type, one method. No builder. No config. No plugins.

### Why

Every Rust game engine asks you to learn its world before you can write a game.
IZANAGI does not. Spawn an entity, draw a rectangle, play a sound. The engine
gets out of your way.

- **Zero dependencies.** Only the standard library.
- **One type.** `Engine` owns everything. Field-access for the rest.
- **Headless first.** Tests run in CI environments unchanged.
- **Pluggable backend.** Swap `NullBackend` for terminal, winit, or wgpu.
- **Deterministic single-run replay.** Seed the `Rng` (avoid `from_entropy()`,
  which deliberately breaks this), and the same inputs reproduce the same
  run — RNG draws and ECS iteration order are both seed-stable. Not bit-exact
  *across* different CPUs/compilers, since core math is `f32`; for that
  stronger guarantee, use `izanagi_kit`.

### A 30-second tour

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

### Run a real game

```bash
cargo run -p izanagi --example hello       # 3-line hello world
cargo run -p izanagi --example pong        # complete Pong, ~100 lines
cargo run -p izanagi --example pong -- --terminal   # play it in your terminal
cargo run -p izanagi --example particles   # 500 particles/sec, gravity, fade
cargo run -p izanagi --example platformer  # gravity + jumping + swept-AABB
cargo run -p izanagi --example roguelike   # BSP map, combat, screen-shake
cargo run -p izanagi --example world       # tilemap + camera + sprite-anim
cargo run -p izanagi --example kit_bridge  # izanagi_kit sim rendered through Engine
```

The terminal backend renders 24-bit colored half-blocks (`▀`) with ANSI escape
sequences. No window system required.

### Modules (24 total)

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

### Quality

- **188 tests** (unit + integration + property-based + benchmark + doctest)
- **0 clippy warnings**, **0 dependencies**, **`cargo fmt --check` passes**
- **MSRV: Rust 1.75**

### Design

Three rules:

1. **Customer experience first.** `cargo run --example pong` runs. No setup.
2. **Say no.** Every API surface is a tax.
3. **Demo-driven.** If `pong.rs` gets harder to write, the change is wrong.

Read [`izanagi/ARCHITECTURE.md`](./izanagi/ARCHITECTURE.md) for the rationale
behind each design decision.

---

## `izanagi_kit` — the deterministic simulation kit

78 zero-dependency modules covering the roguelike/simulation stack a
lockstep-replay game actually needs: sparse-set ECS, Q16.16 fixed-point math,
seeded RNG with named independent sub-streams, symmetric-shadowcasting FOV,
procedural dungeon generation (rooms/BSP/caves/drunkard's-walk/WFC), A*/JPS
pathfinding, replay/desync detection, a text content pipeline with its own
`.game` format and `gamec` CLI, and much more — see
[`izanagi_kit/README.md`](./izanagi_kit/README.md) for the full module table
and a Rust quickstart.

The central guarantee: **identical inputs produce a bit-identical simulation
on every OS and CPU**, pinned by regression tests
(`PINNED_FINAL_HASH`/`PINNED_ROGUELIKE_HASH`) rather than merely asserted.

```
cargo test -p izanagi_kit   # 3174 tests, 0 clippy warnings, fmt clean
```

---

## This repository

Both crates live in one Cargo workspace (root `Cargo.toml`). Two audit
documents track what's implemented, what's missing, and why, at two
different levels of detail:

- [`PRODUCT_AUDIT.md`](./PRODUCT_AUDIT.md) — product-level: what each crate
  provides, where they overlap, what's missing between them
- [`izanagi_kit/FEATURE_AUDIT.md`](./izanagi_kit/FEATURE_AUDIT.md) —
  kit-internal: a module-by-module sufficiency/excess audit

## Contributing

See [`izanagi/CONTRIBUTING.md`](./izanagi/CONTRIBUTING.md). Two rules:

- New code adds tests.
- New code adds no dependencies.

## License

`izanagi`: MIT — see [`izanagi/LICENSE`](./izanagi/LICENSE).
`izanagi_kit`: MIT OR Apache-2.0 — see
[`izanagi_kit/LICENSE-MIT`](./izanagi_kit/LICENSE-MIT) /
[`izanagi_kit/LICENSE-APACHE`](./izanagi_kit/LICENSE-APACHE).
