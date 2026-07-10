# Changelog

All notable changes to IZANAGI are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/) and this project adheres
to [Semantic Versioning](https://semver.org/).

## [Unreleased]

Note: `Cargo.toml` had already moved to `4.1.0` with no corresponding
entry here before this session began; that gap is not reconstructed
below since its actual content is unknown. Everything below is what
changed in this session.

### Added
- **Cargo workspace** — this crate joined a root workspace alongside the
  sibling `izanagi_kit` deterministic simulation crate (previously
  shipped as a separate zip archive at the repository root, never
  extracted or built as part of this tree). `[profile.*]` sections moved
  to the workspace root, since Cargo ignores profiles declared in
  member crates.
- **`examples/kit_bridge.rs`** — a dev-dependency-only example
  demonstrating the engine and `izanagi_kit` composing: a deterministic
  roguelike turn loop runs entirely in kit types and is rendered through
  this crate's `Backend` trait, with the per-turn world-hash trace
  asserted identical between a headless run and one hosted in
  `Engine::run`.
- **`rust-version = "1.65"`** declared in `Cargo.toml`, matching
  `CLAUDE.md`'s existing documented MSRV policy (previously undeclared
  in package metadata).

### Fixed
- **`World`'s per-component storage used `HashMap<u32, T>`, giving
  non-deterministic iteration order** (`ecs.rs`) — `query`/`for_each`/etc.
  iterate this map directly, so the entity visit order any game system saw
  depended on `HashMap`'s per-process random hasher seed: the identical
  program run twice (or on two machines) could visit entities in different
  orders, silently breaking replay for any system whose result depends on
  iteration order (e.g. "apply damage falloff across enemies until a shared
  budget runs out"). Switched to `BTreeMap<u32, T>`, which iterates in
  ascending entity-index order deterministically, at an O(log n) rather
  than O(1) cost per access. `World.columns` (keyed by `TypeId`) did not
  need the same fix — it's only iterated in `despawn` to apply the same
  side-effecting removal to every column, and that result never depends on
  column order. No public API changed. Added a regression test that
  spawns entities out of index order, then asserts `query`/`for_each`
  visit them ascending both before and after a mid-set despawn; confirmed
  it actually catches the bug by temporarily reverting to `HashMap` and
  observing the test fail with a scrambled order before restoring the fix.
  All 188 tests and all 7 examples (including `kit_bridge`, whose
  world-hash trace is unchanged) produce identical output before and after.
- **`Rng::from_entropy` docstring didn't connect to the module's replay
  guarantee** (`rng.rs`) — the module doc promises "same seed always
  produces the same sequence, which matters for replays and networked
  games," but `from_entropy`'s own doc only said "non-deterministic; use
  sparingly," leaving the *consequence* (this call breaks the very
  guarantee the module leads with) implicit. Expanded the docstring to
  say so explicitly and point back to `Rng::new` for anything that must
  be reproducible. Docs only, no behavior change.
- **First clippy pass ever run against this crate** (its CI had never
  executed while the source sat inside an unextracted zip) surfaced 19
  warnings, all resolved: a dead private trait method
  (`Column::contains`), several `map_or(false, ..)` simplifications,
  a redundant `i32` cast, a hex-literal digit grouping style nit, and
  the `world` example silently discarding every event payload it
  drained (now folds them into session state and prints them).
- **MSRV violation introduced by that same clippy pass**: two of the
  `map_or` simplifications used clippy's suggested `is_some_and`
  (stable since Rust 1.70) and `is_none_or` (stable since Rust 1.82) —
  both newer than this crate's declared 1.65 MSRV. Reverted to
  `map_or` with a per-call-site `#[allow(clippy::unnecessary_map_or)]`
  so `-D warnings` CI doesn't silently re-introduce the violation.

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
