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

The kit spans the full stack a terminal roguelike needs. The capability map —
with per-feature implementation status — lives in
[`GAME_DEV_TAXONOMY.md`](./GAME_DEV_TAXONOMY.md); contracts are in
[`SPEC.md`](./SPEC.md). The core foundations:

| Module | Responsibility |
|--------|----------------|
| `entity` | Generational entity handles; stale handles are rejected. |
| `sparse_set` / `arch` | O(1) component storage and an archetype table; cheap composition changes. |
| `fixed` | Q16.16 fixed-point with **saturating** arithmetic for cross-platform determinism. |
| `rng` | SplitMix64 seeded PRNG; replay-safe randomness. |
| `timestep` | Fixed-timestep accumulator with a death-spiral guard. |
| `world_hash` | FNV-1a per-frame state checksum for bit-exact replay assertions. |
| `replay` | Trace recording, desync localisation, and snapshot resimulation (rollback). |
| `netinput` | Transport-agnostic multi-player input prediction and misprediction detection for rollback netcode (`NetInputBuffer`). |
| `content` / `parser` / `serializer` / `validator` / `loader` | The text→ECS content pipeline (see below). |
| `mapgen` / `wfc` / `multimap` | Procedural dungeons (room-placement, cellular-automata caves, BSP partitions, drunkard's-walk caverns), Wave Function Collapse, multi-level worlds. |
| `fov` / `pathfinding` / `influence` / `fsm` / `hfsm` | Symmetric FOV (binary + distance-attenuated), (weighted) A* + path smoothing + Dijkstra flow maps + rescanned flee/safety maps + frontier-seeking auto-explore, influence maps, flat and hierarchical (parent-state + wildcard) state machines. |
| `ability` / `behavior` | Unified skill system (mana/cooldown/range/effect resolution in one call) and hierarchical behavior trees (sequence/selector/invert/repeat leaves) for game AI. |
| `aabb` / `spatial_hash` / `passability` | Axis-aligned bounding-box overlap, spatial-hash broad-phase queries, and a grid passability/collision layer. |
| `geometry` | Bresenham lines / line-of-sight and integer distance metrics (Manhattan, Chebyshev, Euclidean). |
| `terminal` / `camera` | Headless cell buffer with 24-bit ANSI output, diffing, and a world→screen camera. |
| `turn` / `combat` / `inventory` / `status` / `random_table` / `dice` | Energy scheduler, integer combat, items, buff/debuff timers, weighted loot/spawn tables, `NdM±K` dice notation. |
| `damage` / `encounter` / `affix` / `equipment` | Typed damage + resistance profiles, procedural group encounters, item enchantment, worn loadouts with stat aggregation. |
| `equipment` / `progression` / `lightmap` / `faction` | Worn-item loadout per body slot with cursed/locked-item support, XP/level curves, ambient illumination grid, inter-faction reputation. |
| `meta` | Cross-run meta-progression: permanent unlock flags and all-time best records that survive permadeath (`MetaProgress`). |
| `identify` | Scrambled per-seed item appearances (unidentified potions/scrolls) revealed on demand (`Identification`). |
| `threat` / `pool` / `tween` / `wallet` / `shop` / `dialogue` / `trigger` | Per-combatant aggro tables, bounded regenerating resources (mana/stamina), eased time-driven value interpolation (`Tween`) plus single-clock chained playback (`TweenSequence`), fungible currency wallets, wallet-backed buy/sell price listings, branching NPC conversation trees, condition→action rule sets for scripted events. |
| `savefile` | Versioned, checksummed binary save framing. |
| `noise` | Deterministic integer value-noise and hashing for procedural generation. |

## Runnable examples

Twenty-one self-contained demos render to the terminal via the `terminal` module
(24-bit ANSI, zero OS dependencies — they run unchanged in CI):

```
cargo run --example roguelike_demo          # mapgen + A* + FOV + scheduler + combat
cargo run --example wfc_demo                 # Wave Function Collapse biome generation
cargo run --example noise_terrain_demo       # 3-octave FBM terrain + biome heat-map
cargo run --example influence_demo           # influence-map heat-map + HUD panel + steering
cargo run --example content_pipeline_demo    # parse → validate → load → render, with diagnostics
cargo run --example replay_demo              # desync detection + rollback (exits non-zero on failure)
cargo run --example savefile_demo            # save framing: round-trip, corruption, versioning
cargo run --example status_effects_demo      # StatusSet + Inventory + Scheduler + combat
cargo run --example ai_behavior_demo         # FSM + SpatialHash + Cooldown + TimerQueue
cargo run --example menu_textlayout_demo     # Menu navigation + word-wrap + text layout helpers
cargo run --example camera_viewport_demo     # Camera viewport + TileMap + ChangeTracker + Profiler
cargo run --example geometry_easing_demo     # line / line_of_sight / Aabb / Fixed easing sparklines
cargo run --example input_pipeline_demo      # KeyMap / InputBuffer / CmdQueue deterministic input
cargo run --example autotile_demo            # bitmask auto-tiling: compute_all + SimpleTileTable
cargo run --example relations_demo           # entity parent/child forest + cycle guard
cargo run --example multimap_demo            # multi-floor dungeon stack + stair connectors
cargo run --example archetype_demo           # ArchTable ECS: dense iteration + O(1) migration
cargo run --example asset_store_demo         # AssetStore<T> generational handle safety
cargo run --example hud_panels_demo          # HudPanel + BarWidget + StatLine status screen
cargo run --example timestep_demo            # FixedTimestep accumulator + death-spiral guard
cargo run --example cave_spawn_demo          # cellular cave + depth-scaled RandomTable + Distance
```

Pipe any of them to a truecolor terminal for full colour; in a plain pipe the
ANSI bytes still flow to stdout and the summary line prints to stderr.

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
gamec <file.game>          # validate; non-zero exit on error (CI content gate)
gamec --fmt <file.game>    # validate and emit canonical text to stdout
gamec --check <file.game>  # validate formatting only, no output (like `cargo fmt --check`)
gamec --json <file.game>   # emit all diagnostics as machine-readable JSON to stdout
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
cargo run --example roguelike_demo            # see "Runnable examples" above
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

## Project documents

- [`FEATURE_AUDIT.md`](./FEATURE_AUDIT.md) — self-contained audit sorting every
  capability into sufficient / rejected-excess / fixed-deficiency /
  deliberate-non-goal / remaining-open-item, written to be readable with zero
  prior context.
- [`SPEC.md`](./SPEC.md) — module contracts and invariants.
- [`GAME_DEV_TAXONOMY.md`](./GAME_DEV_TAXONOMY.md) — capability map with
  per-feature implementation status, organized by game-dev discipline.
- [`STRENGTHS_WEAKNESSES.md`](./STRENGTHS_WEAKNESSES.md) — strategic
  strengths/weaknesses/gap inventory that drives what gets built next, with
  a Socratic-gap rationale for each addition.
- [`RESEARCH.md`](./RESEARCH.md) — category-by-category survey of external
  prior art (arXiv papers, comparable OSS) informing the improvement backlog.
- [`IMPROVEMENTS.md`](./IMPROVEMENTS.md) — confirmed bug-fix log.
- [`CHANGELOG.md`](./CHANGELOG.md) — release history.

## License

Dual-licensed under either of [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE) at your option.
