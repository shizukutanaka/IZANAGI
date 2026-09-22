# izanagi_kit

Zero-dependency building blocks for a game simulation that **replays
bit-identically** — and the tools to prove yours does.

Eleven modules do nothing but interrogate a simulation: audit it for
nondeterminism, search its state space for one that breaks an invariant,
*prove* no such state exists, minimise any counterexample to its shortest form,
monitor properties that span time, and check that saving and reloading changes
nothing. The rest of the crate is the deterministic substrate they rest on
(fixed-point maths, seeded RNG, world hashing) plus ordinary game systems
written so they stay hashable and replay-safe.

Every module is `std`-only, contains no `unsafe` (`#![forbid(unsafe_code)]`),
cannot panic in shipped code (`clippy::unwrap_used`/`expect_used`/`panic` are
`deny`), and is covered by tests. There are no runtime dependencies.

## Why

A game engine has a hard floor under a zero-dependency constraint: GPU, window,
and audio I/O all require FFI. Rather than fight that ceiling, this kit leans
into a terminal/headless target where the whole frame is text — which makes the
simulation **fully inspectable and deterministically replayable**. Bit-exact
replay is treated as a first-class feature, not an afterthought.

## Quickstart

Implement one trait, and every tool in the crate applies to your simulation.

```rust
use izanagi_kit::sim::{audit, Simulation};
use izanagi_kit::verify::{check_invariant, Verification};
use izanagi_kit::world_hash::{DetHash, Fnv1a};

// Your game state, however you model it.
#[derive(Clone)]
struct Purse {
    coins: i32,
}

// Two impls: how it advances, and how it hashes.
impl Simulation for Purse {
    type Input = i32;
    fn step(&mut self, delta: &i32) {
        self.coins = (self.coins + delta).clamp(0, 10);
    }
}
impl DetHash for Purse {
    fn det_hash(&self, h: &mut Fnv1a) {
        h.write_i32(self.coins);
    }
}

// Is it deterministic? `audit` double-runs it and rolls it back at every
// frame, then hands you a hash to pin as a regression value.
let report = audit(&Purse { coins: 5 }, &[-3, 2, -3], 2);
assert!(report.is_deterministic());

// Does a rule hold? Not "did random play find a violation" — whether one
// exists at all, across every state the simulation can reach.
let proof = check_invariant(
    Purse { coins: 5 },
    &[-3, 2],
    |p: &Purse, d: &i32| {
        let mut next = p.clone();
        next.step(d);
        next
    },
    |p: &Purse| p.coins >= 0,
    10_000,
);
assert!(matches!(proof, Verification::Holds { .. }));
```

See the whole pipeline applied to one simulation with a planted bug:

```text
cargo run --example verify_pipeline_demo
```

The rest of the crate is what a game needs around that core — entities and
sparse-set storage, fixed-point maths, seeded RNG with named sub-streams,
procedural generation, pathfinding, FOV, a text content pipeline — all written
so nothing they touch can break the replay guarantee.

## Modules

Everything here serves one promise: **a simulation that replays
bit-identically**. Not every module carries the same weight in keeping that
promise, so they fall into four tiers — read top-down:

## Start here

```text
cargo run --example verify_pipeline_demo
```

One dungeon room, one planted bug, and every verification tool in the crate
applied to it in turn — so you can see what the eleven checking modules add up
to without reading eleven sets of docs. Three unrelated techniques (bounded
model checking, property testing, delta debugging) converge on the same
two-input minimal cause, and only one of them can then go on to *prove* the
fixed version has no such state at all.


| Tier | What it is | Modules |
|---|---|---|
| **1. Determinism substrate** | Load-bearing. Break one of these and replay breaks. | `fixed`, `vec`, `rng`, `rng_xoshiro`, `noise`, `world_hash`, `replay`, `rollback`, `sim`, `dst`, `shrink`, `prop`, `plan`, `explore`, `temporal`, `recovery`, `verify`, `netinput`, `cmdqueue`, `bits`, `savefile`, `timestep` |
| **2. Deterministic algorithms** | Where nondeterminism usually sneaks into a game (unordered iteration, float, address dependence) — the vetted versions. | `pathfinding`, `fov`, `geometry`, `gridcast`, `graph`, `pack`, `zorder`, `msquares`, `flow`, `hungarian`, `lsystem`, `poly`, `rdp`, `fenwick`, `ahocor`, `diff`, `mapgen`, `wfc`, `tilemap`, `spatial_hash`, `influence`, `voronoi`, `delaunay`, `hexgrid`, `maze`, `passability`, `autotile`, `turn`, `entity`, `sparse_set`, `observe`, `arch`, `relations`, `multimap` |
| **3. Content pipeline** | Author game data as text, then prove it well-formed before it reaches the sim. | `content`, `parser`, `serializer`, `validator`, `loader`, `diag_json` |
| **4. Gameplay conveniences** | Ordinary systems (inventory, shops, quests, UI…) written to be hashable and replay-safe. Nothing in tier 1 depends on them — worked examples you may freely replace. | everything else |

If you adopt one thing, adopt tier 1: implement `sim::Simulation` for your
state and call `sim::audit` on it — one call runs a double-run check and a
rollback-resimulation check and reports the final hash to pin.

The capability map — with per-feature implementation status — lives in
[`GAME_DEV_TAXONOMY.md`](./GAME_DEV_TAXONOMY.md); contracts are in
[`SPEC.md`](./SPEC.md). The core foundations:

| Module | Responsibility |
|--------|----------------|
| `entity` | Generational entity handles; stale handles are rejected. |
| `sparse_set` / `arch` | O(1) component storage and an archetype table; cheap composition changes. |
| `observe` | `Observed<T>` wraps `SparseSet` and emits `ComponentEvent`s (added / replaced / removed) into a drainable queue — the push half of change detection, complementing `change`'s pull. |
| `fixed` | Q16.16 fixed-point with **saturating** arithmetic for cross-platform determinism. |
| `rng` | SplitMix64 seeded PRNG; replay-safe randomness. |
| `timestep` | Fixed-timestep accumulator with a death-spiral guard. |
| `world_hash` | FNV-1a per-frame state checksum for bit-exact replay assertions. |
| `replay` | Trace recording, desync localisation, and snapshot resimulation (rollback). |
| `sim` | The one `Simulation` trait every verification tool consumes, plus `audit()` — a single call that double-runs the sim, rollback-resimulates it, and reports the final hash. |
| `rollback` | Bounded snapshot ring (`SnapshotRing`) and a GGRS-style `sync_test` that rolls back every frame to catch step-function nondeterminism. |
| `dst` | Deterministic Simulation Testing: seed sweeps with per-tick invariants and one-line `(seed, tick)` failure reproduction. |
| `plan` | Planning-based test synthesis — BFS over the state space for a shortest input sequence satisfying a goal ("can the player reach X" becomes an executable replay). |
| `explore` | Archive-based exploration (Go-Explore) — remember every state reached, return to it deterministically, explore onward. Scales past where BFS stalls; hands back a replayable path per state. |
| `temporal` | Temporal property monitors (runtime verification) — `always` / `eventually` / `until` / `precedes` / `responds_within` over a tick stream, with LTL₃ anytime verdicts and a definite finite-trace verdict. Drops straight into a `dst_sweep` invariant. |
| `recovery` | Crash-recovery testing — inject a save/restore cycle after every input and prove the run continues identically. Catches save formats that silently drop a field, including fields the state hash does not cover. |
| `verify` | Bounded model checking — enumerate every reachable state and either prove an invariant holds throughout, or return the shortest input sequence that breaks it. A proof is reported distinctly from "ran out of budget". `check_temporal` crosses the state space with a `temporal` monitor to verify ordering properties no single-state predicate can express. |
| `shrink` | Delta debugging (`ddmin`) — reduce a failing input sequence to a 1-minimal one. |
| `prop` | Property-based testing (QuickCheck) — random sequences, checked property, counterexample returned already shrunk. Plus model-based / differential testing (`forall_model`) — run the real sim in lockstep with a trusted reference model and shrink any diverging command sequence. |
| `netinput` | Transport-agnostic multi-player input prediction and misprediction detection for rollback netcode (`NetInputBuffer`), plus `AdaptiveDelay` tuning input delay to the measured misprediction rate. |
| `content` / `parser` / `serializer` / `validator` / `loader` | The text→ECS content pipeline (see below). |
| `mapgen` / `wfc` / `multimap` / `maze` | Procedural dungeons (room-placement, cellular-automata caves, BSP partitions, drunkard's-walk caverns, Bridson Poisson-disc scatter, `carve_corridors` TinyKeep-style weighted tunneling, Wilson's uniform-spanning-tree perfect mazes), Wave Function Collapse, multi-level worlds. |
| `fov` / `pathfinding` / `influence` / `fsm` / `hfsm` | Symmetric FOV (binary + distance-attenuated), (weighted) A* + `min_cost_path` over arbitrary enter-costs + path smoothing + Dijkstra flow maps + rescanned flee/safety maps + frontier-seeking auto-explore, influence maps, flat and hierarchical (parent-state + wildcard) state machines. |
| `ability` / `behavior` | Unified skill system (mana/cooldown/range/effect resolution in one call) and hierarchical behavior trees (sequence/selector/invert/repeat leaves) for game AI. |
| `aabb` / `spatial_hash` / `passability` | Axis-aligned bounding-box overlap, spatial-hash broad-phase queries, and a grid passability/collision layer. |
| `geometry` | Bresenham lines / line-of-sight and integer distance metrics (Manhattan, Chebyshev, Euclidean). |
| `gridcast` | Amanatides–Woo grid ray traversal (`grid_ray`, `ray_blocked_at`, `clear_los`) — every cell a segment enters, exact corner-exit semantics, symmetric in either direction. |
| `graph` | Deterministic algorithms on `u32` adjacency lists — Tarjan SCC (`strongly_connected`), `articulation_points`, `bridges`, Kahn lexicographic `topo_sort`, and `UnionFind`. |
| `pack` | Skyline (bottom-left) rectangle bin packing (Jylänki 2010) — deterministic `pack_skyline` for atlases, inventories, dialog tiling. |
| `zorder` | Morton (z-order) and Hilbert space-filling-curve codes (`morton_encode`/`decode`, 2D+3D and 64-bit variants, `hilbert_encode`/`decode`, `spatial_sort`, `morton_key`) — locality-preserving deterministic ordering for spatial keys. |
| `msquares` | Marching-squares iso-contour extraction on integer fields (`case_index`, `contour_segments`, `contour_loops`) — canonical saddle resolution, doubled-coordinate segments, loop chaining. |
| `flow` | Edmonds–Karp max-flow / min-cut (`FlowNet`, `max_flow`, `min_cut`, `flow_on`) — bottleneck and partition analysis on capacity networks. |
| `hungarian` | `assign_min_cost`: Kuhn–Munkres minimum-cost assignment on `n ≤ m` matrices — unit→target matching and build-order style problems. |
| `lsystem` | Deterministic L-system rewriting + integer turtle (`expand`, `turtle_cells`, `DIRS_4`/`DIRS_8`/`DIRS_HEX`) — branching plants and procedural structures drawn through `gridcast`. |
| `poly` | Exact `i128` 2D polygon ops (`area2` shoelace, `point_in_polygon` even-odd + boundary, `convex_hull` Andrew monotone chain, `ear_clip` triangulation). |
| `rdp` | Ramer–Douglas–Peucker polyline simplification (`simplify`, `simplify_loop`) — integer-exact, the canonical downstream pass for `msquares` contours. |
| `fenwick` | Fenwick tree / BIT over `i64` (`add`, `prefix_sum`, `range_sum`, `lower_bound` order statistic) — `O(log n)` running totals for leaderboards and weighted picks. |
| `ahocor` | Aho–Corasick multi-pattern `&[u8]` matcher (`scan` → `(end_pos, pattern)` in scan order, `O(text + hits)`, overlaps included). |
| `diff` | Myers `O(ND)` minimal edit scripts (`diff`, `hunks`, `apply`) + `levenshtein` / `lcs_len` — field-level diffs for desync reports and diagnostics. |
| `voronoi` / `delaunay` | Exact nearest-seed partition (`voronoi_partition`, `voronoi_flood` through passable terrain), `mst_edges` / `mst_edges_over` (Kruskal MST over a complete or restricted graph), and integer-exact Delaunay triangulation (`delaunay`, `delaunay_edges`) — scatter → territory → connectivity. |
| `hexgrid` | Axial-coordinate hex math (`Hex`, `DIRECTIONS`, `distance`, `line`, `ring`, `spiral`, odd/even-r offset conversion, `random_in_range`, `hex_astar` shortest paths) — the redblobgames recipe set, integer-exact and `DetHash`-pinned. |
| `terminal` / `camera` | Headless cell buffer with 24-bit ANSI output, diffing, and a world→screen camera. |
| `turn` / `combat` / `inventory` / `status` / `random_table` / `dice` | Energy scheduler, integer combat, items, buff/debuff timers, weighted loot/spawn tables, `NdM±K` dice notation. |
| `damage` / `encounter` / `affix` / `equipment` | Typed damage + resistance profiles, procedural group encounters, item enchantment, worn loadouts with stat aggregation. |
| `equipment` / `progression` / `lightmap` / `faction` | Worn-item loadout per body slot with cursed/locked-item support, XP/level curves, ambient illumination grid, inter-faction reputation. |
| `meta` | Cross-run meta-progression: permanent unlock flags and all-time best records that survive permadeath (`MetaProgress`). |
| `identify` | Scrambled per-seed item appearances (unidentified potions/scrolls) revealed on demand (`Identification`). |
| `threat` / `pool` / `tween` / `wallet` / `shop` / `dialogue` / `trigger` | Per-combatant aggro tables, bounded regenerating resources (mana/stamina), eased time-driven value interpolation (`Tween`) plus single-clock chained playback (`TweenSequence`), fungible currency wallets, wallet-backed buy/sell price listings, branching NPC conversation trees, condition→action rule sets for scripted events. |
| `savefile` | Versioned, checksummed binary save framing. |
| `bits` | LSB-first bit-level wire codec (`BitWriter`/`BitReader`): packed bitfields, ranged integers, canonical varint+zigzag — the lockstep packet primitive. |
| `noise` | Deterministic integer value-noise and hashing for procedural generation — value/fbm/ridge/turbulence plus Worley cellular noise (exact F1/F2). |

## Runnable examples

20+ self-contained demos render to the terminal via the `terminal` module
(24-bit ANSI, zero OS dependencies — they run unchanged in CI):

```text
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
cargo run --example scatter_pipeline_demo    # poisson_disc → voronoi → MST → BitWriter
```

Pipe any of them to a truecolor terminal for full colour; in a plain pipe the
ANSI bytes still flow to stdout and the summary line prints to stderr.

## The content pipeline

```text
text --[parser]--> Content --[validator]--> --[loader]--> ECS world
            ^                                   |
            +-------------[serializer]----------+
```

Author game elements as plain text, validate them, and load them into a live
world. The round-trip `parse(serialize(c)) == c` is property-tested over
thousands of generated bundles.

### `.game` format

```text
prefab <name> [extends <base>]
  glyph <char>
  color <#RRGGBB>
  stat  <key> <int>
  flag  <name>
tile  <name> <glyph> <#RRGGBB>
level <name> <W>x<H>
  row   <cells>
  spawn <prefab> <x> <y>
```

`extends` makes a prefab a field-level patch of its base: `stat` keys and
`flag`s are merged (the child wins a shared stat key), while `glyph`/`color`
override only where the child writes the line itself. Bases can extend bases;
missing bases and cycles are validation errors. `gamec --fmt` keeps the
overlay — it never flattens a child into its resolved form.

## The `gamec` tool

```text
gamec <file.game>          # validate; non-zero exit on error (CI content gate)
gamec --fmt <file.game>    # validate and emit canonical text to stdout
gamec --check <file.game>  # validate formatting only, no output (like `cargo fmt --check`)
gamec --json <file.game>   # emit all diagnostics as machine-readable JSON to stdout
gamec --sarif <file.game>  # emit diagnostics as SARIF 2.1.0, for GitHub Code Scanning
```

Diagnostics are rustc/clang-style, with the offending source line and a caret:

```text
dungeon.game:2:9: error: glyph must be one character
  glyph @@
        ^
```

### The generate → verify → repair loop

When content is machine-authored — procedural templates or an LLM — this
pipeline is the gate that output must pass before it reaches the sim:

```text
generate text → gamec --json (or --sarif) → feed diagnostics back → repeat
```

The diagnostics are designed for the mistakes machine authors actually make:
references to prefabs that were renamed or never defined, spawn coordinates
outside the level, duplicate names, glyphs that cannot render. `--json` /
`--sarif` exist so the verifier is itself machine-readable — a generator can
consume them and retry without a human parsing prose.

What it does not check: that the content is *good*. "Well-formed and
self-consistent" is the contract; balance and intent remain the author's.

## Build and test

```text
cargo test          # all unit + integration tests
cargo run --bin gamec -- examples/dungeon.game
cargo run --example roguelike_demo            # see "Runnable examples" above
```

## Development

The whole verification gate is one command, run from the **repository root**:

```text
tools/gate.sh
```

It runs fmt, the workspace tests, clippy and rustdoc at zero warnings, the
pinned determinism hashes, the `kit_bridge` integration hash, the
self-asserting pipeline demo, and `cargo package` for both crates. Enable the
git hooks (fast checks on commit, the full gate on push) from the repository
root:

```text
git config core.hooksPath .githooks
```

CI is **not yet enabled** on the hosted repository — the agent sessions that
develop this branch cannot push workflow files. The ready-to-install
definition lives at [`../docs/ci/ci.yml`](../docs/ci/ci.yml), with
instructions in [`../docs/ci/README.md`](../docs/ci/README.md); its main job
runs the same `tools/gate.sh`, so local green and CI green mean the same
thing.

## Determinism notes

- Gameplay math uses `fixed::Fixed`, not floats, so results are reproducible
  across CPUs and compilers.
- Randomness flows through one seeded `SplitMix64` stream advanced in a fixed
  order; never seed from the wall clock in code that must replay.
- `timestep` uses integer nanoseconds, introducing no nondeterminism.
- Fold `world_hash` over canonical (sorted) state each frame and assert the
  sequence in CI to catch divergence.

## Project documents

- [`SPEC.md`](./SPEC.md) — module contracts and invariants.
- [`GAME_DEV_TAXONOMY.md`](./GAME_DEV_TAXONOMY.md) — capability map with
  per-feature implementation status, organized by game-dev discipline.
- [`RESEARCH.md`](./RESEARCH.md) — category-by-category survey of external
  prior art (arXiv papers, comparable OSS), with what was implemented (and
  the commit) or deliberately deferred (and why).
- [`CHANGELOG.md`](./CHANGELOG.md) — release history.

(Superseded audit documents were deleted rather than left to mislead; they
remain in git history, and the facts they tracked are now build-checked —
see `tests/docs_are_current.rs`.)

## License

Dual-licensed under either of [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE) at your option.
