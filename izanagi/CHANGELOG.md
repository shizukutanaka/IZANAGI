# Changelog

All notable changes to IZANAGI are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/) and this project adheres
to [Semantic Versioning](https://semver.org/).

## [4.0.0] - 2026-04-29

A full restart. The previous v3.x line grew to 858K lines and never
reached `cargo build`. v4.0 is a from-scratch rewrite that compiles,
tests, and ships in under 7,000 lines.

### Added

#### Core engine
- `Engine` — the only public type. Six subsystem fields, three methods.
- `World`, `Entity` — sparse-storage ECS with generational entities.
- `Input`, `Key` — polled keyboard/mouse with edge events.
- `Gamepads`, `Button`, `Stick` — up to 4 controllers, analog sticks,
  triggers, and circular deadzone.
- `Render`, `Color`, `Draw` — immediate-mode draw list.
- `Camera` — world↔screen transform, follow with exponential decay,
  clamp-to-world, rotation, zoom.
- `Tilemap` — grid storage with camera-frustum culled iteration.
- `Sprite`, `Animation`, `Frame` — texture-atlas regions with frame
  sequencing, looping, and `from_grid` helper.
- `Audio`, `Voice` — voice mixer.
- `audio_pcm::PcmBuffer`, `load_wav`, `sine_wave` — WAV decoder
  (PCM 8/16-bit, mono/stereo) and tone generator.
- `Assets`, `Handle` — byte cache with filesystem loader.
- `Scene`, `Node` — parent-child 2D transform graph.
- `States<S>` — pushdown automaton for menu/play/pause flows.
- `Tween`, `Timer` — eased animation and one-shot / repeating timers.
- `Events<E>` — typed event bus, drained per frame.
- `math` — `Vec2`, `Vec3`, `Mat3`, `Rect` with operator overloads.
- `collide` — AABB, swept-AABB, ray-vs-AABB, circle-vs-circle.
- `ease` — linear, quad, cubic, back, elastic, bounce, smoothstep.
- `rng::Rng` — xorshift64, deterministic, with `chance`, `range`,
  `int_range`, `choose`, and `from_entropy`.
- `save::Save` — versioned binary save format.
- `error::Error`, `error::Result` — single error type.
- `backend::Backend` trait, `NullBackend`, `TerminalBackend` (24-bit
  ANSI half-block renderer).
- `debug::Metrics` — FPS, worst-ms, rolling history.

#### Examples
- `hello` — 3-line hello world.
- `pong` — complete game in ~100 lines.
- `particles` — 500/sec spawn, HSV palette, gravity, fade.
- `platformer` — swept-AABB collision, jumping, state machine.
- `roguelike` — BSP-generated dungeon, combat, screen-shake.
- `world` — scrolling tilemap, camera-follow, sprite animation, events.

#### Quality
- 159 tests (121 unit + 19 integration + 10 benchmark + 9 doctest).
- 7 property-based / fuzz tests at 200 rounds each.
- `cargo fmt --check` passes.
- 0 warnings on `cargo build --all-targets`.
- 0 external dependencies — std library only.
- GitHub Actions CI for Linux, macOS, Windows.
- MSRV check at Rust 1.75.

### Removed
- The 858K-line v3.x codebase. Everything.
- All non-stdlib dependencies, including `serde` and `criterion`.
- Plugin system, builder pattern, configuration struct.
- 548 game modules that pretended to be engine code.

### Performance
- Cold `cargo build`: ~4 seconds.
- Full `cargo test`: ~12 seconds, 159 tests.
- Headless `cargo run --example world`: 244-step auto-walk to goal in 6s sim.

[4.0.0]: https://github.com/shizukutanaka/izanagi/releases/tag/v4.0.0
