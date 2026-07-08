# CLAUDE.md — IZANAGI engine

## Why
A Rust 2D/3D game engine. Zero deps, zero config. Run `cargo run --example pong` to validate.

## Map
```
src/
  lib.rs       — Engine type + public re-exports
  ecs.rs       — World, Entity (sparse storage, generational)
  input.rs     — Input, Key (polled, not subscribed)
  time.rs      — Time (dt, elapsed)
  assets.rs    — Assets, Handle (byte cache)
  audio.rs     — Audio, Voice (null backend)
  render.rs    — Render, Color, Draw (immediate-mode)
  scene.rs     — Scene, Node (parent-child Mat3)
  state.rs     — States<S> (pushdown automaton)
  tween.rs     — Tween, Timer (f32 animation)
  math.rs      — Vec2, Vec3, Mat3, Rect
  collide.rs   — swept_aabb, ray, circle
  ease.rs      — easing curves
  rng.rs       — xorshift64 RNG
  save.rs      — binary save format
  error.rs     — Error, Result
  backend.rs   — Backend trait, NullBackend, TerminalBackend

examples/
  hello.rs     — 3-line smoke test
  pong.rs      — full game, --terminal flag
  particles.rs — 500/s, HSV, fade, --terminal
  platformer.rs — gravity + swept-AABB + state machine
  roguelike.rs — BSP dungeon, ECS, state machine, events, --terminal
  world.rs     — scrolling tilemap + camera + sprite animation, --terminal
  kit_bridge.rs — izanagi_kit sim (mapgen/A*/FOV) rendered via Engine;
                  asserts headless and engine-hosted world-hash traces match

tests/
  integration.rs — cross-module API contracts
```

## Rules
- `#![forbid(unsafe_code)]` — no exceptions.
- Zero dependencies in core (`[dependencies]` stays empty). The one
  exception is `[dev-dependencies]` on the sibling `izanagi_kit` crate,
  used solely by `examples/kit_bridge.rs` — it never ships in the
  published crate. Backends in sub-crates only.
- All public items need doc comments. `cargo doc --no-deps` must be clean.
- `cargo fmt --check` must pass.
- `cargo test` must be green before any PR.
- Never add a config option without a question: "does the user actually need this?"
- MSRV is Rust 1.65. Do not use features newer than this without a feature flag.

## Workflows

### Add a module
1. Create `src/<name>.rs` with doc-comment and tests.
2. Add `pub mod <name>;` in `lib.rs`.
3. Add re-exports to `lib.rs` if types are commonly used.
4. Add integration test in `tests/integration.rs`.

### Add an example
1. Create `examples/<name>.rs`.
2. Register in `Cargo.toml` `[[example]]`.
3. Example must run headless (`cargo run --example <name>` completes without hanging).
4. Example must print something useful at the end.

### Add a backend
1. Implement `backend::Backend` in a new `izanagi-<name>` crate.
2. The new crate depends on `izanagi`. Core does not depend on it.
3. See `TerminalBackend` as the reference implementation.

### Performance investigation
1. Find the hot path with `cargo build --release` + `time`.
2. Before changing: write a test that measures (use `std::time::Instant`).
3. After changing: confirm the measurement improved.
4. Document the result in the commit message.

## gstack skills
- `.claude/skills/add-module.md`
- `.claude/skills/add-example.md`
- `.claude/skills/debug-ecs.md`

## Key design decisions
- `query()` allocates (convenience). `for_each()` does not (hot loop).
- `for_each2<T, U>` iterates the intersection without allocation.
- `Backend` is a trait object, not a generic parameter, to avoid propagating type vars.
- `States<S>` is a pushdown automaton; `replace()` swaps top, `push()` keeps history.

## Do not
- Add `anyhow` or `thiserror`. `error.rs` is the error.
- Add `serde`. Save format is in `save.rs`.
- Add `glam` or `nalgebra`. Math is in `math.rs`.
- Add `rand`. RNG is in `rng.rs`.
- Remove headless operation from any module.
- Break the `Engine::new().run(|e| { ... })` pattern.
