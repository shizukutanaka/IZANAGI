# Architecture

This file explains *why* the engine is shaped the way it is. The README
covers *what* it does. If a future change makes one of the rationales
below no longer hold, the change is probably wrong.

## The shape

```
                ┌─────────────────────────────┐
                │           Engine            │
                │  (the only public type)     │
                └──────────────┬──────────────┘
                               │
        ┌──────────┬───────────┼───────────┬──────────┐
        ▼          ▼           ▼           ▼          ▼
      World      Input       Time     Render+Audio  Scene
       (ecs)                          (drainable)   (graph)
                               ▲
                               │
                        ┌──────┴──────┐
                        │   Backend   │   ← NullBackend (default)
                        │   (trait)   │   ← TerminalBackend (ANSI)
                        └─────────────┘   ← user backends (winit/wgpu)
```

`Engine` owns six subsystems as public fields. There is no service locator,
no registry, no plugin trait. Field-access is the API.

## Decision: one engine type

We considered builder pattern, plugin trait, and config struct. All three
were rejected because they add a learning step before the user writes code.

- **Builder:** `Engine::builder().with_audio(...).with_input(...).build()`
  forces the user to know which subsystems exist before they need any of them.
- **Plugin trait:** `engine.add_plugin(MyPlugin)` is what big engines do.
  It pushes complexity onto every game.
- **Config struct:** `Engine::new(EngineConfig { ... })` requires reading the
  config docs before anything compiles.

`Engine::new()` works. Default behavior runs in CI. Swap a backend when you
need rendering. That is the whole story.

## Decision: zero dependencies in core

`Cargo.toml` lists no dependencies. This is non-negotiable. Every dep is a
breakage risk, an audit burden, a license question, and a build slowdown.

- Vectors and matrices: written here in 250 lines.
- RNG: xorshift64 in 60 lines.
- Save format: 4-byte magic + version + length + bytes. 100 lines.
- Hashing: `std::collections::HashMap` is enough.

When real graphics are needed, a separate crate (`izanagi-winit`, planned)
wraps winit and wgpu and implements the `Backend` trait. The core never
takes that dependency.

## Decision: ECS is sparse, not archetypal

Bevy and EnTT use archetypes — entities with the same component set live in
the same memory block, queries iterate columns linearly. This is faster at
millions of entities. It is also significantly more code, requires unsafe,
and exposes lifetime gymnastics in the public API.

Sparse storage (one `HashMap<Entity, T>` per component type) is slower in
the limit but:

- has no `unsafe`,
- has no lifetimes leaking through,
- inserts/removes in O(1) without restructuring,
- and is fast enough for tens of thousands of entities, which is more than
  any indie game built with this engine will need.

If a user proves they need archetypes, `Engine` will eventually grow a
parallel storage option. Until then: sparse.

## Decision: immediate-mode rendering

Retained-mode (scene-graph-as-render-state) means the user mutates a tree,
the engine diffs, and a renderer draws. Every state change touches three
abstractions.

Immediate mode means: every frame, the user pushes draws onto a list. The
backend drains the list and presents. There is no diff. There is no
"current state" to forget about.

`Render::drain()` returns `(clear_color, draws, texts)` and resets. The
backend never queries; it only consumes.

## Decision: input is queried, not subscribed

Callbacks (`on_keydown`) are easier to write but harder to debug — control
flow leaves your code. IZANAGI keeps input as polled state:

```rust
if e.input.pressed(Key::Space) { jump() }
```

`pressed` is the edge event (true the frame the key went down). `down` is
held state. The engine clears edges between frames automatically.

## Decision: backend is a trait object, not a generic

`Box<dyn Backend>` instead of `Engine<B: Backend>`. The generic version
forces every example, integration test, and downstream user to specify the
backend in every signature. The cost of the vtable on a once-per-frame
`poll`/`present` call is unmeasurable.

## File map

```
src/
├── lib.rs        # Engine, public re-exports
├── ecs.rs        # World, Entity, sparse columns
├── input.rs      # Input, Key, edge events
├── time.rs       # Time, dt, elapsed
├── assets.rs     # Assets, Handle
├── audio.rs      # Audio, Voice
├── render.rs     # Render, Color, Draw
├── scene.rs      # Scene, Node, parent-child Mat3
├── state.rs      # States<S> pushdown automaton
├── math.rs       # Vec2/Vec3/Mat3/Rect, ops
├── collide.rs    # aabb_vs_aabb, swept_aabb, ray
├── ease.rs       # linear/quad/cubic/back/elastic/bounce
├── rng.rs        # Rng, xorshift64
├── save.rs       # Save::write/read/encode/parse
├── error.rs      # Error, Result
└── backend.rs    # Backend trait, NullBackend, TerminalBackend
```

Total: ~1700 lines, ~85 tests, 0 deps.

## Non-goals

- 3D scene graph (Vec3 exists for math; meshes/materials are out of scope).
- Networking (a `Backend` could expose it; the engine will not).
- Asset pipeline (build-time tools are a separate crate).
- Editor (separate program; the engine is a library).
- Hot reload (interesting but expensive — not now).
