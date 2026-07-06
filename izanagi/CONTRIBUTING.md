# Contributing to IZANAGI

## Ground rules

1. **Tests.** Every PR adds tests for what it changes. `cargo test` must pass.
2. **Zero dependencies.** New code adds no `[dependencies]`. Backend crates
   (e.g. `izanagi-winit`) may have deps; the core must not.
3. **No warnings.** `cargo build --all-targets` must produce no warnings.
4. **Formatted.** Run `cargo fmt` before committing.

## Development workflow

```bash
git clone https://github.com/shizukutanaka/izanagi
cd izanagi
cargo test            # 120+ tests
cargo run --example pong
cargo run --example roguelike -- --terminal
```

## What belongs in core vs a separate crate

| Core (`izanagi`) | Separate crate |
|---|---|
| ECS, math, input, audio mixer | Window system (winit, SDL2) |
| State machine, tween, events | GPU renderer (wgpu, OpenGL) |
| Collision, RNG, save | Audio backend (cpal, rodio) |
| TerminalBackend | Asset pipeline (texture atlas, etc.) |

## PR checklist

- [ ] `cargo fmt --check` passes
- [ ] `cargo test` passes (all 3 test files)
- [ ] `cargo build --all-targets` has zero warnings
- [ ] New public items have doc comments
- [ ] CHANGELOG.md updated under `[Unreleased]`

## Philosophy

Read `ARCHITECTURE.md` first. The short version:

> If a change makes `hello.rs` or `pong.rs` harder to write, it is wrong.

When in doubt, do less. The engine earns trust by being predictable.
