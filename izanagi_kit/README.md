# izanagi_kit

Zero-dependency reference modules for building a deterministic, terminal-first
game engine in Rust. Extracted from a design review of the IZANAGI engine and
grounded in published work on ECS storage, lockstep determinism, fixed-point
arithmetic, error-recovering parsers, and structure-aware testing.

Every module is `std`-only, contains no `unsafe` (`#![forbid(unsafe_code)]`),
and is covered by tests. There are no runtime dependencies.

## Why

A game engine has a hard floor under a zero-dependency constraint: GPU, window,
and audio I/O all require FFI. Rather than fight that ceiling, this kit leans
into a terminal/headless target where the whole frame is text — which makes the
simulation **fully inspectable and deterministically replayable**. Bit-exact
replay is treated as a first-class feature, not an afterthought.

## Modules

| Module | Responsibility |
|--------|----------------|
| `entity` | Generational entity handles; stale handles are rejected. |
| `sparse_set` | O(1) component storage; cheap composition changes. |
| `fixed` | Q16.16 fixed-point with **saturating** arithmetic for cross-platform determinism. |
| `rng` | SplitMix64 seeded PRNG; replay-safe randomness. |
| `timestep` | Fixed-timestep accumulator with a death-spiral guard. |
| `world_hash` | FNV-1a per-frame state checksum for bit-exact replay assertions. |
| `content` | Authored data model: prefabs, tiles, levels, spawns. |
| `parser` | Line-based content parser; panic-free, bounded, column-aware diagnostics. |
| `serializer` | Inverse of the parser; canonical, idempotent output. |
| `validator` | Cross-reference and bounds checks; collects every finding. |
| `loader` | Instantiates validated content into the ECS world. |

## The content pipeline

```
text --[parser]--> Content --[validator]--> --[loader]--> ECS world
            ^                                   |
            +-------------[serializer]----------+
```

Author game elements as plain text, validate them, and load them into a live
world. The round-trip `parse(serialize(c)) == c` is property-tested over
thousands of generated bundles.

### `.game` format

```
prefab <name>
  glyph <char>
  color <#RRGGBB>
  stat  <key> <int>
  flag  <name>
tile  <name> <glyph> <#RRGGBB>
level <name> <W>x<H>
  row   <cells>
  spawn <prefab> <x> <y>
```

## The `gamec` tool

```
gamec <file.game>        # validate; non-zero exit on error (CI content gate)
gamec --fmt <file.game>  # validate and emit canonical text to stdout
```

Diagnostics are rustc/clang-style, with the offending source line and a caret:

```
dungeon.game:2:9: error: glyph must be one character
  glyph @@
        ^
```

## Build and test

```
cargo test          # all unit + integration tests
cargo run --bin gamec -- examples/dungeon.game
```

## Development

Enable the local git hooks (format + clippy on commit, tests on push):

```
git config core.hooksPath .githooks
```

CI runs on every push and pull request:

- **test** — build and run all unit and integration tests
- **lint** — `cargo fmt --check` and `cargo clippy -D warnings`
- **audit** — `cargo audit` for known advisories
- **content-gate** — `gamec` validates every shipped `.game` file and confirms
  the broken fixture is rejected

All builds treat warnings as errors (`RUSTFLAGS=-D warnings`).

## Determinism notes

- Gameplay math uses `fixed::Fixed`, not floats, so results are reproducible
  across CPUs and compilers.
- Randomness flows through one seeded `SplitMix64` stream advanced in a fixed
  order; never seed from the wall clock in code that must replay.
- `timestep` uses integer nanoseconds, introducing no nondeterminism.
- Fold `world_hash` over canonical (sorted) state each frame and assert the
  sequence in CI to catch divergence.

## License

Dual-licensed under either of [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE) at your option.
