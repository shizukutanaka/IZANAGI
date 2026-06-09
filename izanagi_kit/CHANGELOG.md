# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `rng::SplitMix64::pick(slice)`: uniform random element from a non-empty slice,
  returning `Option<&T>`. Returns `None` without drawing for an empty slice so
  draw count stays a deterministic function of arguments (mirrors `rot.js getItem`).
- `rng::SplitMix64::next_u32()`: advances the stream and returns the upper 32 bits
  of the 64-bit output. Useful when only a 32-bit integer is needed.
- `sparse_set::SparseSet::clear()`: bulk removal of all entries; resets the sparse
  index so no stale slots remain. Equivalent to `retain(|_,_| false)` but O(n)
  without the per-element predicate call overhead.
- `sparse_set::SparseSet::entities()`: entity-handle-only dense iterator. Mirrors
  the `keys()` / `iter_entities()` convention of Bevy and EnTT, avoiding an
  unnecessary component dereference when only entity membership matters.
- `sparse_set::SparseSet::values()` / `values_mut()`: component-value-only iterators.
  Avoids the entity-handle unpacking boilerplate when the caller only needs to read
  or mutate all stored values (e.g. ticking every AI state machine).
- `dice::Dice::roll_advantage(rng)` / `roll_disadvantage(rng)`: D&D 5e advantage
  mechanics — roll twice, keep max / min respectively. Each call consumes exactly
  two draws so the draw count is fixed and replay-deterministic.
- `dice::Dice`: `std::fmt::Display` — pretty-prints as `"3d6+2"`, `"d20"`,
  `"2d8-1"`, etc. Enables round-trip `parse(dice.to_string())` and human-readable
  logging without heap allocation beyond the format string.
- `easing`: back easing family — `ease_in_back`, `ease_out_back`, `ease_in_out_back`.
  Standard Penner overshoot curves (c1 ≈ 1.70158). The ease-in version briefly goes
  below 0; ease-out briefly exceeds 1. Endpoints are exactly 0→1. Pure polynomial,
  no trig, no float.
- `easing`: bounce easing family — `ease_out_bounce`, `ease_in_bounce`,
  `ease_in_out_bounce`. Piecewise polynomial (7.5625·t² in 4 segments) approximating
  a bouncing ball. Stays in `[0, 1]` by construction. No float, no trig.
- `vec::Vec2`: `abs()`, `min(a, b)`, `max(a, b)`, `clamp(lo, hi)` — component-wise
  utilities present in bracket-lib's geometry crate. `abs` maps each component
  through `Fixed::abs`; `min`/`max` use component-wise comparisons; `clamp`
  composes them. All operations are deterministic and saturating, safe to include
  in world hashes.
- `vec::Vec3`: same `abs()`, `min()`, `max()`, `clamp()` additions plus `xy()` —
  project to `Vec2` by dropping `z`. Mirrors the `glam::Vec3::xy()` convention and
  covers the common top-down 2D view of a 3D world-space position.
- `vec::Vec2::Neg`, `vec::Vec3::Neg`, `vec::Vec2::perp`: replaced `Fixed::ZERO - v`
  workarounds with idiomatic `-v` now that `Fixed::Neg` is implemented. Behaviour
  is identical (saturating subtraction and saturating negation are equivalent for
  all Q16.16 values); the change is purely for readability.
- `terminal::Screen::draw_box(x, y, w, h, fg, bg)`: draws a single-line box border
  using Unicode box-drawing characters `┌┐└┘─│`. The interior is untouched.
  Out-of-bounds positions are silently clipped via the existing `put` contract —
  no panic. The missing primitive for roguelike panels, inventory windows, and HUD
  borders.
- `terminal::Screen::resize(width, height)`: replace the front and back buffers
  with blank cells at the new dimensions, discarding all previous content. Required
  for real-world `SIGWINCH` / terminal-resize handling where the caller redraws
  from scratch.
- `profiler::Profiler::avg(section)`: rolling average tick-total for a section
  over the history window. The method was documented in the module doc-comment but
  never implemented; this fills the gap. Only ticks where the section recorded at
  least one sample are counted (silent ticks don't dilute the average).
- `fixed::Fixed`: `core::ops::Neg` implementation. Previous code had to write
  `Fixed::ZERO - v` to negate; the new impl makes `-v` work directly. Saturating:
  `-Fixed::MIN` returns `Fixed::MAX` instead of overflowing.
- `assets::AssetStore::iter_mut()`: mutable `(handle, &mut asset)` iteration in
  ascending slot-index order. Fills the symmetric gap left by `iter()`. Enables
  in-place mass updates (e.g. ticking all sprite animation frames) without
  collect-then-update round-trips.
- `assets::AssetStore::retain(pred)`: remove all assets for which
  `pred(handle, &asset)` returns `false`. Parallel to `SparseSet::retain` — the
  end-of-frame cleanup pattern without a collect+remove loop. Removed slots are
  freed and handles permanently invalidated.
- `keymap::KeyMap::clear()`: remove all bindings at once. Useful for rebuilding a
  key layout at runtime (e.g. when the player changes the control scheme in the
  options menu).
- `wfc::WfcRules::disallow(tile, dir, neighbor)`: the symmetric counterpart of
  `allow`. Clears the corresponding adjacency bit so the combination is no longer
  permitted. Out-of-range arguments are silently ignored (same contract as
  `allow`).
- `savefile::LoadError`: `std::fmt::Display` and `std::error::Error` implementations.
  The type previously only derived `Debug`; the new impls allow it to be used in
  `?`-chains with `anyhow` / `thiserror`-style error plumbing, and to be printed
  as a human-readable message (e.g. "save file checksum mismatch").
- `rng::SplitMix64::shuffle(slice)`: Fisher-Yates in-place shuffle. Draws
  `slice.len() − 1` times using `below` so the draw count is deterministic and
  the sequence replays identically given the same seed and position. Handles
  empty and single-element slices without drawing. The missing primitive that
  every roguelike needs for shuffling room lists, random encounter tables, and
  ability orderings. Re-exported as `izanagi_kit::SplitMix64` (method on the type).
- `hud::BarWidget::percentage()`: fill percentage as an integer in `[0, 100]`.
  Useful for "85% HP" status lines without float arithmetic. Saturates: negative
  current returns 0, over-max returns 100, max ≤ 0 returns 0.
- `arch::ArchTable::upsert(entity, row)`: insert if absent, overwrite if present.
  Complements `insert` (which silently ignores duplicates) with the common
  "move entity = update position" pattern. Implemented as a branch in the hot
  path (checks the HashMap index, updates in-place if found).
- `tilemap::TileMap::iter_mut()`: mutable `(x, y, &mut tile)` row-major iteration.
  Fills the symmetric gap left by the existing shared `iter()`. Needed for any
  in-place map update (fog-of-war ageing, post-gen tile transformations, render
  passes that write back per-cell state). Follows the same coordinate computation
  as `iter()` so no separate index book-keeping is required.
- `random_table::RandomTable::roll_owned(rng)`: returns a cloned `T` instead of
  a `&T`. Removes the borrow chain that `roll()` carries so callers can store the
  loot result in a variable, pass it to another function, or push it into a
  container without lifetime headaches. Requires `T: Clone`; implemented as
  `.roll(rng).cloned()` — no separate draw logic.
- `sparse_set::SparseSet::retain(pred)`: bulk removal of all entries for which
  `pred(entity, &value)` returns `false`. O(n) — each removed entry pays one
  swap-remove. The canonical "remove dead entities at end-of-frame cleanup" pattern,
  matching `Vec::retain` semantics. Avoids collecting entity IDs and calling
  `remove` in a separate pass.
- `influence::InfluenceMap::combine(other, num, den)`: cell-wise weighted addition
  — `self[i] += other[i] * num / den` (saturating). Enables composing multiple
  influence layers with different weights in one call (e.g. threat at −1× plus
  food at +⅔). `den == 0` is treated as 1; maps of different sizes are a no-op.
- `easing::ease_in_out_cubic`: completes the cubic easing family (the `in` and
  `out` variants existed but `in_out` was missing). Formula: `4t³` for `t < 0.5`,
  `1 − 4(1−t)³` for `t ≥ 0.5`. The most widely used of the cubic trio.
- `aabb::Aabb::union(other)`: the smallest AABB enclosing both `self` and `other`.
  Empty boxes are excluded from the result (a union with an empty box returns the
  non-empty side). Equivalent to `bracket-lib`'s `Rect::union`. Useful for
  computing aggregate bounding boxes over a set of colliders.
- `spatial_hash::SpatialHash::query_radius(cx, cy, radius)`: all keys within
  Chebyshev distance `radius` of a center point (equivalent to a square query
  `[cx-r, cx+r] × [cy-r, cy+r]`). Complements `query_rect` with a center+radius
  ergonomic API common to most roguelike spatial indices. Returns empty for
  negative `radius`.
- `multimap::MultiMap::current_mut()`: mutable borrow of the currently active
  `Dungeon` floor. Complements the existing `current()` shared borrow — needed for
  any map-edit operation (door open/close, terrain mutation, item placement) on the
  live floor.
- `entity::EntityAllocator::count()`: O(1) live entity count (`total_slots −
  free_slots`). Useful for diagnostics and save-file headers ("32 entities saved").
- `camera::Camera::pan(dx, dy, world_w, world_h)`: scroll the viewport by a
  world-space delta, clamped at the world boundary. The natural mapping for
  arrow-key map scrolling or tracking a fast-moving entity.
- `camera::Camera::center()`: world-space coordinate at the centre of the
  viewport (`top_left + screen_size/2`). Replaces the manual arithmetic callers
  previously wrote to answer "what is the camera looking at?".
- `passability::PassabilityGrid::iter_passable()`: row-major iterator over `(x,
  y)` of passable cells. Collect it and index with `rng.below(count)` for uniform
  random spawn placement without scanning the full map.
- `passability::PassabilityGrid::iter_blocked()`: companion iterator for blocked
  cells — useful for visualisation and testing that map generation placed walls
  in the expected positions.
- `relations::Relations::descendants_of(entity)`: BFS over the entity's subtree
  (children, grandchildren, …), excluding the root itself. Enables "kill all
  carried items" and hierarchical cleanup without repeated `children_of` calls.
- `relations::Relations::root_of(entity)`: walk up the parent chain to the
  topmost ancestor. Returns `entity` itself if it has no parent.
- `msglog::MsgLog::last()`: returns a reference to the most recently pushed
  message, or `None` if the log is empty. Avoids allocating a full `recent(1)`
  iterator for the common "show the last line in the status bar" pattern.
- `combat::StrikeResult` and `combat::critical_strike`: melee attack with a
  critical-hit chance — rolls `crit_chance` (1..=100 percent) via `roll_to_hit`,
  multiplies base damage by `crit_multiplier` on a crit (minimum 1×), applies to
  the defender, and returns `StrikeResult { damage, critical }`. The RNG contract
  matches `roll_to_hit`: degenerate `crit_chance` (≤ 0 or ≥ 100) consumes no draw.
  Deterministic and replay-safe. Re-exported as `izanagi_kit::{critical_strike, StrikeResult}`.
- `combat::Stats::restore()`: refills HP to `max_hp` in one call (level-up
  rest, potion of full healing).
- `combat::Stats::set_max_hp(new_max)`: adjusts max HP and clamps current HP
  to the new ceiling — the standard level-up HP-increase pattern.
- `status::StatusSet::clear()`: removes all active effects at once (e.g.
  "cure all conditions" spell, death cleanup).
- `status::StatusSet::magnitude_of(key)`: direct magnitude lookup returning 0
  when inactive — eliminates the `.get(k).map_or(0, |e| e.magnitude)` boilerplate.
- `status::StatusSet::remaining_of(key)`: direct duration lookup returning 0
  when inactive — mirrors `magnitude_of`.
- `inventory::Inventory::clear()`: empties all slots while preserving capacity
  (store re-stock, respawn, container reset).
- `inventory::Inventory::remove_where(pred)`: removes and returns the first
  item matching a predicate — the "remove first consumable of type X" operation.
- `inventory::Inventory::iter_mut()`: mutable `(slot_index, &mut item)` iteration
  for in-place updates (durability, stack-size changes, re-identification).
- `fov::compute_fov_dist`: distance-attenuated FOV variant — same symmetric
  shadowcasting as `compute_fov` but the callback receives an extra `dist_sq: i32`
  (squared Euclidean distance from origin). Callers use it to implement light
  falloff, range-scaled fog-of-war, or ambient darkness without a separate sqrt
  pass. Origin is always reported with `dist_sq == 0`. Deterministic and replay-safe;
  the underlying scan logic is shared with `compute_fov` (no duplication). Re-exported
  as `izanagi_kit::compute_fov_dist` (cf. libtcod / rot.js light-source patterns).
- `pathfinding::smooth_path`: greedy Bresenham LOS path simplification ("string
  pull"). Post-processes any `astar`/`weighted_astar` path by skipping waypoints
  for which a straight Bresenham segment has no blocked interior cells, reducing
  the staircase visual of grid paths. Suitable for AI waypoint navigation: the
  actor still steps through each smoothed segment, so no-corner-cutting is
  enforced at runtime. Fully deterministic (same path + same `is_blocked` → same
  output). Re-exported as `izanagi_kit::smooth_path`.
- `pathfinding::dijkstra_map` and `pathfinding::descend` re-exported at the crate
  root (`izanagi_kit::{dijkstra_map, descend}`). These existed in the module but
  were not previously in `lib.rs` — closing the API gap.
- `turn::Scheduler::energy(id)`: returns the current banked energy for an actor
  (`None` if not registered). Useful for save/load and diagnostics; values ≥
  `ACTION_COST` indicate the actor is immediately ready.
- `turn::Scheduler::set_energy(id, energy)`: directly sets banked energy — negative
  values add a delay, values ≥ `ACTION_COST` grant an immediate turn. Designed for
  restoring exact scheduler state from a save file.
- `timer::TimerQueue::cancel_where(pred)`: selectively cancels pending entries whose
  event satisfies a predicate. Returns the count removed. Both one-shot and repeating
  entries are eligible. Fills the gap between `cancel_all` (nuke everything) and
  the lack of any partial cancellation API.
- `dice::Dice`: tabletop dice-notation parsing and rolling — `Dice::parse("3d6+2")`
  (panic-free, `None` on malformed input), `roll(&mut rng)` (replay-deterministic
  via `SplitMix64::dice`), plus `min`/`max`/`average_x100` range queries for
  balancing without floats. The standard data-driven way to author damage / hit
  dice (cf. `bracket-random`'s `roll_str`). Re-exported as `izanagi_kit::Dice`.
- `content::Color::from_hsv`: integer HSV→RGB conversion (`hue` in degrees mod
  360, `sat`/`val` in `0..=255`) for procedural palettes and rainbow/heat
  gradients — no float, deterministic across targets.
- `mapgen::generate_bsp` / `mapgen::BspParams`: binary-space-partition dungeon
  generation — the third classic family alongside `generate_dungeon` (room
  rejection) and `generate_cave` (cellular automata); cf. `bracket-lib`'s BSP
  builders. Recursively splits the interior (orientation biased by aspect ratio,
  split point from `rng`) until partitions hit `min_leaf` or `max_depth`, carves
  one room per leaf (disjoint by construction), and joins each split's two child
  rooms with an L-corridor on the way back up — guaranteeing full connectivity.
  Returns the same `Dungeon` type; too-small maps return all-wall without
  panicking. Re-exported as `izanagi_kit::{generate_bsp, BspParams}`.
- `noise::fbm_2d` / `noise::fbm_1d`: fractional Brownian motion — sum `octaves`
  layers of value noise at doubling frequency / halving amplitude, renormalised
  to `[0, 65535]`. This is the standard primitive for natural terrain; it was
  previously hand-rolled in `noise_terrain_demo`, which now calls the library
  function (output unchanged). Frequency shifts and the amplitude taper are
  bounded so any octave count is panic-free and deterministic. Re-exported at the
  crate root.
- `easing`: extended the Penner family with twelve new integer easers —
  `ease_{in,out,in_out}_quart` (t⁴), `_quint` (t⁵), `_sine` (via the CORDIC
  `Fixed::sin_cos`), and `_circ` (via `Fixed::sqrt`). All float-free and
  bit-identical across targets, matching the kit's determinism guarantee. Brings
  easing coverage from 6 to 18 functions; all re-exported at the crate root.
- `content::Color` operations: `Color::rgb` (const constructor), `lerp(a, b,
  num, den)` (integer-ratio per-channel interpolation for heat-map gradients and
  fades), `grayscale()` (integer Rec. 601 luma), and `scale(num, den)`
  (ratio dimming/brightening). All integer-only and clamping — replaces the
  hand-rolled channel math the demos previously needed.
- `aabb::Aabb` region queries: `area()` (saturating `w*h`), `is_empty()`,
  `center()` (matching `mapgen::Rect::center`'s top-left bias), `contains(&Aabb)`
  (rect-in-rect, boundary-inclusive), and `iter_points()` — row-major iteration
  over the interior cells, closing the parity gap with `bracket-geometry`'s
  `Rect::for_each`/`point_set` for filling and scanning rectangular regions.
- `vec::Vec2` rotation and interpolation: `rotate(angle)` (2-D rotation matrix
  driven by the fixed-point CORDIC `Fixed::sin_cos` — no float, deterministic),
  `angle()` (vector heading via `Fixed::atan2`), `lerp(a, b, t)`,
  `distance(rhs)` and `distance_sq(rhs)`. Closes the gap where the kit shipped
  fixed-point trig but vectors could not rotate or report their heading — the
  core operation for steering, projectiles, and tweening. `Vec3` gains the
  matching `lerp` / `distance` / `distance_sq` for parity.
- `examples/cave_spawn_demo.rs`: ties the three new primitives into one screen — `generate_cave` carves a 64×20 connected cavern, a depth-scaled `RandomTable<Spawn>` (monster weights rising with depth, items flat) scatters monsters and items onto floor cells, and `Distance::Chebyshev` tints the player's awareness radius and counts nearby monsters. Fully deterministic from one seed; the right panel lists the table's weights. (`cargo run --example cave_spawn_demo`)
- `mapgen::generate_cave` / `mapgen::CaveParams`: organic cave generation via the
  **cellular-automata** method (RogueBasin "cave-like levels"; cf. `bracket-lib`
  ch.27 and `rot.js` `Cellular`), complementing the rectangular
  `generate_dungeon`. Deterministic three-stage pipeline driven by `SplitMix64`:
  (1) random seed at `wall_percent`, border kept solid; (2) `steps` passes of the
  4-5 rule (cell becomes wall with 5+ wall neighbours, out-of-bounds counts as
  wall); (3) connectivity cull — every floor cell outside the largest 4-connected
  region is filled back to wall, so the returned cave is guaranteed to be a single
  connected space with no isolated pockets. Returns the same `Dungeon` type (with
  `rooms` empty), so it plugs straight into `fov`/`pathfinding` via
  `Dungeon::is_wall`. Re-exported as `izanagi_kit::{generate_cave, CaveParams}`.
- `random_table::RandomTable<T>`: weighted random selection table for loot drops
  and depth-scaled spawn tables — the canonical roguelike pattern (cf. the *Rust
  Roguelike Tutorial* `random_table.rs` and `bracket-lib`). A typed layer over
  `SplitMix64::weighted_index` that *owns* the candidate values, so `roll` yields
  the value directly. Builder (`with`) and in-place (`push`) APIs; entries with
  weight 0 are stored but never selected; an empty/all-zero table rolls `None`
  without drawing, so selection stays replay-deterministic (exactly one draw per
  non-empty roll). `DetHash` (gated on `T: DetHash`) folds the table config into
  world/replay hashes. Re-exported as `izanagi_kit::RandomTable`.
- `geometry::Distance`: grid distance metrics (`Manhattan`, `Chebyshev`,
  `EuclideanSquared`, `Euclidean`) via `Distance::between(a, b)` — mirrors
  `bracket-lib`'s `DistanceAlg` for picking a metric that matches movement rules
  (Manhattan for 4-way, Chebyshev for 8-way, the Euclidean pair for radial
  range). `Euclidean` returns the floored integer distance using an integer
  square root (Newton's method), so it remains float-free and cross-platform
  deterministic; all metrics saturate rather than overflow. Re-exported as
  `izanagi_kit::Distance`.

### Fixed
- `content::parse_color`: no longer panics on a 7-byte input containing a
  multi-byte UTF-8 char (e.g. `#aéABC`). It now parses hex straight from bytes
  instead of slicing the `&str` at fixed offsets, restoring the parser's
  panic-free guarantee. Regression covered in unit tests and the garbage-input
  fuzz.
- `fixed::from_int`: saturates instead of silently flipping sign for
  out-of-range inputs (`from_int(32768)` previously wrapped to `i32::MIN` via an
  unchecked `<< 16`), closing the last hole in the "never wrap to the opposite
  extreme" invariant.
- `fixed::from_ratio`: a zero denominator now saturates toward the numerator's
  sign (consistent with `div`) rather than panicking with a divide-by-zero, and
  an out-of-range quotient clamps via `from_wide` instead of a wrapping cast.
- `rng::below(0)`: now returns `0` without consuming a draw, identically in
  debug and release. Previously the bound check was `debug_assert!`-only, so
  release builds could silently advance the stream and desync a replay.
- `ci`: the security-audit job now runs `cargo generate-lockfile` before
  `cargo audit`; `Cargo.lock` is `.gitignored`, so the job previously failed
  with "Couldn't find Cargo.lock" on every run.

### Changed
- `README.md`: documented the seven runnable examples (`cargo run --example …`) and refreshed the module overview from 11 to a representative cross-section of the ~50 shipped modules, linking to `GAME_DEV_TAXONOMY.md` and `SPEC.md` for the full capability map and contracts.

### Added
- `examples/timestep_demo.rs`: fixed-timestep accumulator demo. A 9-frame variable real-frame trace (17/17/8/9/50/500/17/16/33 ms) is fed into a 60 Hz `FixedTimestep` (max 5 catch-up steps). The left panel tabulates, per frame, the steps emitted, the leftover accumulator (as a fraction-of-step bar), and the interpolation `alpha`. The 500 ms stall (f5) would demand 30 steps but is **CLAMPED** to 5 (death-spiral guard), and f6 then takes exactly 1 step — proving no catch-up debt is carried over. The right panel also demonstrates frame-pacing independence: the same total time delivered as one big frame vs 400 tiny frames yields an identical `total_steps` (100), confirming render cadence cannot change simulation results. Exercises `FixedTimestep::new`/`advance`/`step_ns`/`total_steps`/`alpha_ratio`. (`cargo run --example timestep_demo`)
- `examples/hud_panels_demo.rs`: HUD widgets demo. Lays out a four-panel RPG character-status screen (VITALS / ATTRIBUTES / PROGRESS / STATUS EFFECTS) entirely from `HudPanel`, `BarWidget`, and `StatLine`. Each `HudPanel` owns a bordered region whose content origin comes from `inner_x`/`inner_y`/`inner_w` (1-cell margin). `BarWidget::render` draws fill bars with distinct fill glyphs (HP `█`, MP `▓`, Stamina `▒`, XP `·`, effects `■`); `StatLine::render`/`with_unit` format attributes and unit-tagged progress values (gold gp, depth floors, time min). A cursor point is hit-tested against all four panels via `HudPanel::contains` and a tooltip is positioned with `HudPanel::translate`. `filled_cells` is reported in the stats line. (`cargo run --example hud_panels_demo`)
- `examples/asset_store_demo.rs`: generational-handle asset store demo. Four `Sprite` assets are inserted into an `AssetStore<Sprite>`, yielding opaque `AssetHandle<Sprite>` keys. The demo recolours one via `get_mut`, upgrades one via `replace` (returning the old value), then `remove`s the torch — invalidating its handle. Two stale-handle reads are then shown to be rejected: `get(torch)` after removal returns `None` (no use-after-free), and after a new `amulet` insert reuses the freed slot with a bumped generation, the old torch handle still returns `None` rather than silently aliasing the amulet. The left panel renders the live sprite table (handle index, coloured glyph, name) plus a sprite-sheet strip; the right panel logs each operation with ok/REJ/info markers. Final: live=4, both stale gets rejected 2/2. (`cargo run --example asset_store_demo`)
- `examples/archetype_demo.rs`: archetype-storage ECS demo. Two `ArchTable<Row>` tables model a swarm: `Mobile{pos,vel,fuel}` and `Static{pos}`. A movement system walks the dense `Mobile` table with `iter_mut` (cache-friendly SoA iteration), integrates positions, bounces entities off arena walls, and decrements fuel; when an entity runs out of fuel it **migrates** Mobile→Static via O(1) `remove` (swap-remove) + `insert`. Five entities are seeded (three with fuel < ticks migrate, two outlive the run and stay mobile); the arena renders trails plus live `●` mobile and `▣` rested markers, with both archetype tables' contents and a migration log shown on the right. `world_hash::hash_state` produces a canonical per-table checksum (independent of swap-remove order). Final: mobile=2, static=3, migrations=3. (`cargo run --example archetype_demo`)
- `examples/multimap_demo.rs`: multi-floor dungeon demo. Generates three deterministic floors via `generate_dungeon` (per-floor `SplitMix64` seeds), wraps them in a `MultiMap`, and wires stair `Connector` records between consecutive floors (down-stair at floor N's last-room centre → up-stair at floor N+1's first-room centre). All three floors render side by side with the active floor (index 1) brightened; `>` descend and `<` ascend glyphs are placed purely from the connector table via `exits_from`, never hard-coded into the map. `connector_at` is probed on the floor-0 down-stair. Exercises `MultiMap::new`/`floor`/`current`/`set_floor`/`floor_count`/`add_connector`/`exits_from`/`connector_at`. (`cargo run --example multimap_demo`)
- `examples/relations_demo.rs`: entity relationship tree demo. Builds a 6-entity ownership forest (Hero wielding Sword[socketed Rune] + Shield, commanding a Familiar[Spark]) via `Relations::attach`, then exercises every query: `parent_of`/`children_of` navigation, `depth`/`is_root`/`is_leaf`/`is_ancestor` structural checks. A cycle attempt `attach(Hero→Rune)` is rejected (returns `false`), shown REJ in red. `remove_entity(Familiar)` orphans Spark (becomes a root), which is then reparented onto Hero via a second `attach`. The left panel renders the final hierarchy as an indented DFS tree coloured by role (root=gold, node=blue, leaf=green) with per-node depth; the right panel logs each operation with ok/REJ/info markers. Final: relations=4, hero_children=3, spark_depth=1. (`cargo run --example relations_demo`)
- `examples/autotile_demo.rs`: bitmask auto-tiling demo. A 26×14 hand-drawn `#` dungeon is auto-tiled into connected box-drawing walls purely from each cell's 8-bit neighbour mask — no manual tile placement. `compute_all(w, h, is_same)` produces every cell's mask in one row-major pass; a 256-entry `SimpleTileTable` reduces each mask to a 4-bit cardinal index (N/E/S/W) which indexes a box-drawing glyph table (`─│┌┐└┘├┤┬┴┼`). The left panel shows the raw `#` map, the right panel the auto-tiled result, with a cardinal-bit glyph legend. `compute_mask(x, y, is_same)` spot-checks one cell in the stats line (probe(5,4) → `0b01000100` → `─`). The diagonal-corner-clearing rule is handled inside `compute_mask`. (`cargo run --example autotile_demo`)
- `examples/input_pipeline_demo.rs`: deterministic input pipeline demo. A 13-tick scripted session demonstrates the three-layer input abstraction: `InputBuffer<char>` (initial_delay=1, repeat_period=2) tracks held keys — initial presses fire immediately (yellow), hold-repeats fire after the delay then every period (orange). `KeyMap<char, Action>` translates raw fired chars to `Action` variants (`MoveNorth/South/East/West`, `Wait`, `OpenInventory`, `Descend`, `Quit`). `CmdQueue<Action>` accumulates translated actions during a tick and drains them in one batch at the tick boundary. The left panel shows all 8 bindings; the right panel shows the per-tick log with yellow=initial and orange=repeat colour coding. Final stats: 13 ticks, 10 commands (6 moves, 2 repeats). (`cargo run --example input_pipeline_demo`)
- `examples/geometry_easing_demo.rs`: geometry and easing curves demo. Left panel (60 cols) renders a two-room dungeon (`RA` cols 0–20, `RB` cols 28–58, corridor cols 20–28 rows 5–8) built from `Aabb` constants. Eight LOS rays from viewer `@` at (10,6) are drawn using `geometry::line`; each ray's visibility is determined by `geometry::line_of_sight` (interior cells only). Five rays land green (visible), three are blocked red. Right panel shows 8-step sparklines for all six easing functions (`linear`, `ease_in_quad`, `ease_out_quad`, `ease_in_out_quad`, `ease_in_cubic`, `ease_out_cubic`) evaluated over `Fixed::from_ratio` with `Fixed::raw()` mapped to block characters `▁▂▃▄▅▆▇█`. Bottom rows report `Aabb::overlaps`, `Aabb::intersection`, `Aabb::contains_point` for four test rectangles and LOS summary counts. (`cargo run --example geometry_easing_demo`)
- `examples/camera_viewport_demo.rs`: camera viewport and world-navigation demo. A 120×36 `TileMap<u8>` (border walls + horizontal/vertical corridor walls with gaps) is viewed through a 60×18 `Camera` viewport embedded in an 80×24 screen. A ten-step scripted player path drives `Camera::recenter` each tick; the camera clamps to the world boundary twice (right edge at t=3, top edge at t=6). `PassabilityGrid::from_tilemap` builds the blocker layer and reports passable/blocked cell counts. `Changed<(i32,i32)>` + `ChangeTracker` demonstrate dirty-flag tracking per sim tick. `MsgLog` (capacity 18) records human-readable step events; `EventLog<MoveEvent>` (capacity 24) records structured events (Stepped + CameraClamp) which are destructured in the stderr summary. `Profiler` records four named work-unit sections (world_gen, pass_build, path_sim, render) with peak query. (`cargo run --example camera_viewport_demo`)
- `examples/menu_textlayout_demo.rs`: character-class selection screen demonstrating `Menu<T>` and the five text-layout helpers. Builds a six-item `Menu<CharClass>` (five enabled classes, Paladin disabled with `add_disabled`), simulates four `move_down()` calls — the fourth auto-skips the locked Paladin entry and lands on Shaman — then renders a two-panel 80×24 screen: left panel lists all classes with a `▶` cursor on the selected row (using `pad_right` to fill each label to the panel width), right panel shows the class name centred with `center`, the description word-wrapped with `wrap_words` (4 lines at 50 cols), and a stats row with `pad_right`/`pad_left` for aligned key/value pairs. The bottom hint bar is clipped to 79 chars with `truncate`, showing the ellipsis. (`cargo run --example menu_textlayout_demo`)
- `examples/ai_behavior_demo.rs`: AI behaviour demo. Four guards patrol a 55×20 dungeon room, each owning a `Fsm<GuardState, GuardEvent>` (Idle → Alert → Chase → Dead), a `Cooldown` (attack rate gate), and a `TimerQueue<()>` (patrol beat, fires every 5 ticks to advance to the next waypoint). `SpatialHash<u32>` (cell_size=4) tracks all entity positions; each tick `query_rect(player ± detect_r)` finds nearby guards and fires `PlayerSpotted`/`PlayerLost` events into each FSM. Chasing guards step toward the player; attacking guards reset their `Cooldown` and trigger player retaliation. The map renders guards coloured by FSM state (g=Idle, G=Alert, C=Chase, %=Dead) with a detection-zone background tint; the right panel shows guard state, HP, CD, and a timestamped event log. (`cargo run --example ai_behavior_demo`)
- `examples/status_effects_demo.rs`: status effects, inventory, and turn scheduling demo. Three actors (Hero / Orc / Mage) fight over 60 scheduler ticks. Demonstrates `StatusSet<K>` (Regen, Haste, Poison applied/ticked/expired), `Inventory<T>` (HealthPotion / Antidote / SpeedDraught consumed in-combat), `Scheduler<u32>` (speed-weighted turn order with runtime `set_speed` for Haste), `Stats` + `melee_attack` / `ranged_attack` (integer combat), and `BarWidget` HP fill-bar rendering. Left panel shows final actor state; right panel shows the turn event log with colour-coded entries (hits orange, heals green, deaths red). (`cargo run --example status_effects_demo`)
- `examples/noise_terrain_demo.rs`: procedural terrain demo. Generates an 80×22 biome map using 3-octave fractional Brownian motion (`value_noise_2d`) and `hash_2d` for sparse landmark scatter. Seven biomes (deep water → coast → sand → grass → forest → mountain → snow) with 24-bit background gradients and a biome-distribution legend bar. (`cargo run --example noise_terrain_demo`)
- `examples/content_pipeline_demo.rs`: content pipeline demo. Runs the full `parse → validate → load_level → render` pipeline on an embedded DSL bundle (hero, goblin, potion, dungeon room). Side by side: left panel shows entity overlay on the tile grid; right panel shows parser diagnostics for an intentionally broken DSL — matching `gamec` human-mode output. (`cargo run --example content_pipeline_demo`)
- `examples/influence_demo.rs`: influence-map + HUD demo. Generates a dungeon, seeds an `InfluenceMap` with monster sources (positive, r=10) and trap sources (negative, r=6), renders the scalar field as a 24-bit heat-map (blue→grey→red), overlays a HUD panel with `BarWidget`/`StatLine` widgets and `highest_neighbour`/`lowest_neighbour` steering arrows. (`cargo run --example influence_demo`)
- `examples/savefile_demo.rs`: save-file framing demo. Exercises four scenarios — clean round-trip, corrupted payload byte (→ `ChecksumMismatch`), truncated buffer (→ `TooShort`), version mismatch (→ caller-side rejection) — with a hex dump of the 20-byte framing header rendered to the terminal. (`cargo run --example savefile_demo`)
- `examples/replay_demo.rs`: replay & desync-detection demo. Exercises all four `replay` primitives: `record_trace`, `check_trace`, `first_divergence`, `resimulate`. Runs four checks (clean replay → OK, wrong seed → diverges at tick 0, tampered hash → diverges at the correct tick, rollback resimulate == direct run), renders a green/red ANSI report card, and exits with code 1 on any failure — making it directly usable as a smoke test. (`cargo run --example replay_demo`)
- `examples/wfc_demo.rs`: Wave Function Collapse terrain demo. Defines a 5-tile biome tileset (deep water→shallow→sand→grass→mountain) with gradient adjacency constraints, generates an 80×44 fully-collapsed grid via `wfc_solve`, computes grass-layer autotile masks via `compute_all`, and renders an 80×24 24-bit-ANSI snapshot with a tile legend. (`cargo run --example wfc_demo`)
- `examples/roguelike_demo.rs`: runnable terminal demo wiring the full kit end-to-end. Generates a dungeon (seed `0x5EED_1234`), places a player (`@`) and monster roster (`g`, `G`, `o`, `r`, `T`), runs 200 energy-scheduler ticks, and renders two full-colour ANSI snapshots (initial state + final state) using `terminal::Screen`, `Camera`, `MsgLog`, `compute_fov`, `astar`, and `melee_attack`. The demo has zero OS dependencies and runs clean in CI as well as in real colour terminals. (`cargo run --example roguelike_demo`)
- `tests/roguelike_sim.rs`: end-to-end composition determinism proof. A realistic turn-based roguelike loop wires together `mapgen`, `passability`, `pathfinding` (A*), `fov`, `turn` (scheduler), and `combat` (melee): monsters path toward and attack the player, the player retaliates, and a per-turn world hash folds positions, HP, scheduler state, and the player's visible-cell count. Asserts same-seed → bit-identical 400-turn trace, different-seed divergence, non-triviality, and pins a final hash (`0x5286_d142_0200_fe66`, identical in debug and release) as a regression tripwire for the gameplay-module stack — complementing `determinism.rs` which pins the core RNG+fixed+storage stack. 4 new integration tests.
- `passability`: grid-based passability / collision layer (`PassabilityGrid`) (K1). Stores a flat `Vec<bool>` grid (true = blocked, false = passable). Constructors: `new` (all passable), `from_fn(w, h, |x,y|…)`, `from_tilemap(map, is_blocked_pred)`, `from_dungeon(dungeon)`. `is_blocked(x,y)` returns `true` for out-of-bounds. `set_blocked` for runtime mutation. `blocker()` produces a borrow closure directly passable to `astar` / `weighted_astar`. Also: `blocked_count`, `passable_count`, `len`, `is_empty`, `DetHash`. Upgrades K1 from 🔶 to ✅. 15 new tests.
- `savefile`: versioned binary save-file framing (N2/N3). `save_bytes(header, payload)` encodes a `SaveHeader { version: u32 }` + arbitrary payload into a self-describing buffer: magic `b"IZNG"`, version (LE u32), FNV-1a 64-bit checksum (LE u64), payload length (LE u32), payload bytes. `load_bytes(data)` validates magic, checksum, and declared length, returning `(SaveHeader, payload_slice)` or a `LoadError` (TooShort / BadMagic / ChecksumMismatch). The caller uses `header.version` to implement forward/backward compatibility (N3). Zero external deps; pure standard library. Satisfies taxonomy N2 and N3. 13 new tests.
- `arch`: archetype-based component storage (`ArchTable<Row>`) (C4). Stores `(Entity, Row)` pairs in a dense contiguous array for O(n) cache-friendly iteration; a secondary `HashMap<Entity, usize>` index provides O(1) insert/lookup/remove. `Row` is a caller-defined struct bundling multiple component fields (the archetype row), decoupled from `SparseSet` single-component storage. Swap-remove keeps the array packed. API: `insert(entity, row) -> bool`, `remove(entity) -> Option<Row>`, `get/get_mut`, `contains`, `len/is_empty`, `iter/iter_mut`, `clear`. `DetHash` sorts by entity index for canonical replay-safe hashing. Satisfies taxonomy C4. 14 new tests.
- `wfc`: Wave Function Collapse procedural tile-map generation (I5). `WfcRules` stores per-tile adjacency bitmasks (up to 64 tile types as `u64` bits, 4 cardinal directions); `allow(tile, dir, neighbor)` and `allow_symmetric` build the rule set. `wfc_solve(w, h, rules, rng)` entropy-guided BFS collapse: finds the minimum-entropy cell, picks a random tile (`SplitMix64`), propagates constraints via AC-3-style BFS queue, and returns `WfcResult::Ok(WfcGrid)` or `WfcResult::Contradiction`. `WfcGrid` provides `tile_at(x,y)`, `is_fully_collapsed()`, `iter_collapsed()`, `len`, `DetHash`. All arithmetic is integer (bitmask popcount for entropy). Deterministic: identical seed/rules → identical output. 14 new tests.
- `diag_json`: machine-readable JSON serialization of pipeline diagnostics (P4). `diag_json(file, diags)` emits a self-contained JSON object with `file`, `diagnostics` array (`severity`, `line`, `col`, `message` per entry), `errors`, and `warnings` counts. All string values are correctly JSON-escaped (quotes, backslashes, control chars). `gamec` gains a `--json` flag that outputs this format to stdout and suppresses human-readable stderr output, making it consumable by CI tools, editors, and LSP clients. Zero external dependencies; pure hand-written JSON building. Satisfies taxonomy P4. 14 new tests.
- `pathfinding::weighted_astar`: weighted (ε-admissible) A* pathfinding. `weighted_astar(start, goal, is_blocked, weight)` inflates the octile heuristic `f = g + weight × h`; at `weight = 1` it is identical to `astar` (optimal), at `weight > 1` it expands fewer nodes while bounding the returned path cost to `weight × optimal_cost`. Determinism invariants preserved: open-set key `(f, weight*h, x, y)` is total and unique. Weight `0` is clamped to `1`. Satisfies taxonomy J8. 8 new tests.
- `multimap`: multi-floor dungeon stack (`MultiMap`, `Connector`). `MultiMap` wraps a `Vec<Dungeon>` floors with an active `current_floor` index; floors are connected by `Connector` records (positions on floor A → position on floor B, directional). API: `new` (clamps `current_floor` to valid range), `floor_count`, `current_floor`, `set_floor` (clamped), `floor(i)`, `current` (active floor borrow), `add_connector`, `exits_from(floor)`, `connector_at(floor, x, y)`. `DetHash` folds `current_floor` + connectors sorted by `(from_floor, from_x, from_y)` for canonical replay-safe ordering. Both `MultiMap` and `Connector` implement `DetHash`. Satisfies taxonomy I6. 10 new tests.
- `autotile`: bitmask auto-tiling for terrain rendering. `compute_mask(x, y, is_same)` computes an 8-bit neighbour mask (N/NE/E/SE/S/SW/W/NW bits) with automatic diagonal-suppression when flanking cardinals are absent; `compute_all(w, h, is_same)` computes the full map in row-major order. `SimpleTileTable` is a 256-entry `u8→u32` lookup mapping masks to tile variant IDs; supports `set`/`get`/`from_array` and `DetHash`. Satisfies taxonomy I4. 14 new tests.
- `hud`: HUD data primitives for terminal roguelike display. `BarWidget` renders a fill-bar (`[====    ]`, configurable fill/empty chars) using integer `current/max * width` with clamping; `StatLine` formats `"label: value [unit]"` lines; `HudPanel` tracks a positioned bounding box with `inner_*` content-region helpers and `contains`/`translate`. All implement `DetHash`. Satisfies taxonomy M4. 16 new tests.
- `profiler`: tick profiler and structured event log. `Profiler` tracks named sections (identified by `&'static str`) with per-tick totals, call counts, and all-time peak; `begin_tick()` flushes current totals to rolling history; no OS clock dependency (callers supply timestamps as `u64`). `EventLog<E>` is a bounded ring-buffer log of tick-stamped typed events (oldest dropped when full); `push(tick, event)`, `iter()`, `recent(n)`, `clear`. Both implement `DetHash`. Satisfies taxonomy P3. 13 new tests.
- `assets`: typed asset handle store (`AssetStore<T>`, `AssetHandle<T>`). Generational handles prevent stale-read bugs (old handle resolves to `None` after remove + reuse). API: `insert` (→ handle), `get`, `get_mut`, `replace`, `remove`, `is_live`, `len`, `iter` (ascending index order). `DetHash` (gated on `T: DetHash`) folds live assets in index order. Satisfies taxonomy H7. 13 new tests.
- `relations`: entity parent/child relationship store (`Relations`). Forest-of-trees structure; each entity has at most one parent and any number of children. `attach` / `detach` / `remove_entity` (children become roots); cycle detection prevents infinite loops; `parent_of`, `children_of`, `is_ancestor`, `depth`, `is_root`, `is_leaf`. `DetHash` folds (child, parent) pairs in canonical ascending entity-index order. Satisfies taxonomy C6. 14 new tests.
- `influence`: grid-based influence map (`InfluenceMap`) for game AI steering. Integer `i32` scalar field; `add_source(x, y, strength, radius)` radiates Chebyshev-linear falloff; `add_raw` for direct cell edits; `decay(num, den)` for time-based fading (saturating integer multiply); `highest_neighbour`/`lowest_neighbour` for 8-directional steering (chase/flee); `clear`. `DetHash` folds dimensions + cells in row-major order. Satisfies taxonomy J6. 15 new tests.
- `tilemap`: multi-layer tile map (`TileMap<T>`, `LayeredMap<T>`). `TileMap` is a 2-D grid in row-major order with `get`/`set` (OOB = `None`/no-op), `fill`, `fill_rect` (clips to boundary), `iter` (row-major). `LayeredMap` bundles `N` same-size layers; `layer(i)`, `layer_mut(i)`, convenience `get`/`set` by layer index. `DetHash` for both (gated on `T: DetHash`). Also added `DetHash for u8` to `world_hash` (zero-pad to u32). Satisfies taxonomy I3. 15 new tests.
- `noise`: deterministic integer noise functions for procedural generation. `hash_1d(x, seed)` and `hash_2d(x, y, seed)` — fast integer coordinate hashes (SplitMix64-style mixing, output `[0, u32::MAX]`). `value_noise_1d(x, seed)` and `value_noise_2d(x, y, seed)` — smooth cubic Hermite (smoothstep) interpolated integer noise in `[0, 65535]`, using Q16.16 fixed-point arithmetic throughout (no float). Coordinates are Q16.16 (upper 16 bits = integer, lower = fraction) so callers can sample at sub-integer positions. Bit-identical across targets. Satisfies taxonomy D6. 16 new tests.
- `spatial_hash`: integer spatial hash grid (`SpatialHash<K>`) for O(1)-average broad-phase collision queries. Fixed-size cells; Euclidean division for negative-coordinate correctness. API: `insert`, `remove`, `move_entity`, `query_cell`, `query_rect` (spans multiple cells), `clear`, `len`, `cell_count`. `DetHash` (gated on `K: DetHash + Ord`) folds cells in canonical coordinate order, buckets in sorted-key order. Satisfies taxonomy K3. 15 new tests.
- `inputbuf`: input buffer with hold-detection and key-repeat (`InputBuffer<K>`). Tracks held keys by tick count; initial press fires immediately; after `initial_delay` ticks repeats every `repeat_period` ticks (standard roguelike long-press). API: `press`, `release`, `is_held`, `tick(n) -> Vec<K>`, `clear`, `held_count`, `held_ticks`. `DetHash` (gated on `K: DetHash + Ord`) folds held-key state in canonical key order. Pairs with `KeyMap` and `CmdQueue` for the full input pipeline. Satisfies taxonomy G3. 14 new tests.
- `textlayout`: word-wrap and text alignment helpers for fixed-width terminal rendering. `wrap_words(text, max_cols)` breaks at word boundaries (falling back to hard mid-word splits for words longer than the column width); `truncate(s, n)` clips to `n` chars appending `…`; `center(s, w)` / `pad_right(s, w)` / `pad_left(s, w)` pad with spaces for alignment. All functions are pure, allocation-minimal, and char-width based (one scalar = one column). Satisfies taxonomy M3. 26 new tests.
- `menu`: keyboard-navigable list menu (`Menu<T>`, `MenuItem<T>`) for roguelike UI. Items carry a typed payload `T`; each has a display label and optional `disabled` flag. Navigation via `move_up`/`move_down` (wrapping, skips disabled items); `set_cursor` for direct jump; `select` returns the current payload or `None` if disabled/empty. `DetHash` (gated on `T: DetHash`) folds labels + disabled flags + values + cursor position. Pairs with `terminal::Screen` for rendering and `KeyMap`/`CmdQueue` for input. Satisfies taxonomy M2. 18 new tests.
- `aabb`: axis-aligned bounding box (`Aabb`) collision detection. Integer rectangle `[x, x+w) × [y, y+h)` with exclusive right/bottom edges. API: `new` (negative dimensions clamped to zero), `right`/`bottom` edge accessors, `overlaps` (touching edges don't overlap), `contains_point`, `intersection` (returns sub-box or `None`), `translate` (saturating). `DetHash` folds all four fields. Satisfies taxonomy K2. 22 new tests.
- `inventory`: slot-based `Inventory<T>` for roguelike items. Fixed-capacity array of optional slots; items added to the first free slot; removed by index; retrieved by index or predicate. Slot layout is stable (gaps are not compacted, preserving acquisition order). API: `new`, `capacity`, `len`, `is_empty`, `has_space`, `add`, `remove`, `get`, `find`, `iter`, `swap`. `DetHash` (gated on `T: DetHash`) folds slot indices and values in canonical ascending order plus capacity so two identically-filled inventories hash the same regardless of fill history. Satisfies taxonomy L3. 14 new tests.
- `combat`: integer combat formula — `Stats` (hp/max_hp/attack/defense with
  `take_damage`, `heal`, `is_alive`, `hp_fraction`), `base_damage` (attack −
  defense, min 1), `melee_attack` (resolve and apply), `roll_to_hit` (draws
  one RNG value; degenerate 0% / 100% resolved without drawing), and
  `ranged_attack` (hit roll + melee). All integer, no float. `Stats` implements
  `DetHash`. Satisfies taxonomy L2. 17 new tests.
- `fsm`: table-driven finite state machine (`Fsm<S,E>`) for game AI. Transitions
  are `(from_state, event) → to_state` triples; missing transitions are silent
  self-loops (no panic). `fire` returns true on state change; `set_state` forces
  a state bypass (e.g. on death). `DetHash` folds only the current state (the
  table is constant configuration). Demonstrates a guard AI (Idle/Alert/Chase/
  Dead) in tests. Satisfies taxonomy J7. 12 new tests.
- `keymap`: key-to-action mapping (`KeyMap<K,A>`). Translates raw key events
  into typed game actions via a configurable binding table (one key → one
  action). `bind` adds/replaces; `unbind` removes; `get` looks up; `is_bound`
  checks; `translate_all` maps a slice of keys discarding unmapped ones —
  the typical per-tick call before pushing into `CmdQueue`. Replay-safe by
  construction (stateless function). Satisfies taxonomy G1. 13 new tests.
- `change`: dirty-flag change detection (`Changed<T>` and `ChangeTracker`).
  `Changed<T>` wraps a value with its `changed_at` tick; `is_changed_since(n)`
  skips updates in O(1); `get_mut(tick)` mutates and auto-marks. `ChangeTracker`
  is a saturating tick counter incremented each sim step. `DetHash` folds value
  + tick so a spurious re-mark is visible in the world hash. Mirrors Bevy's
  `Changed<T>` filter but without global state — fully replay-safe. Satisfies
  taxonomy C5. 10 new tests.
- `status`: timed buff/debuff tracking (`StatusSet<K>`). Each effect has a
  remaining tick duration and a signed magnitude. `apply` inserts or refreshes
  (max-duration stacking); `tick(n)` advances all effects, removes expired ones
  and returns their keys; `remove` discards immediately. `total_magnitude` sums
  all active magnitudes for a net modifier. `DetHash` folds in canonical key
  order so the hash is insertion-order-independent. Satisfies taxonomy L4.
  11 new tests.
- `easing`: integer easing curves over `Fixed` — `ease_in_quad`,
  `ease_out_quad`, `ease_in_out_quad`, `ease_in_cubic`, `ease_out_cubic`, and
  `linear`. All take `t ∈ [0,1]`, return values in the same range, and are
  bit-identical across targets (no float). Satisfies taxonomy B5. 10 tests.
- `fixed`: added `abs` (saturates MIN→MAX), `sign` (-1/0/1), `clamp(lo,hi)`,
  and `lerp(a,b,t)` — completing taxonomy B6. Also added `Fixed::MAX` /
  `Fixed::MIN` constants (used by `vec` module). 9 new tests.
- `camera`: integer camera and viewport (`Camera`) for world↔screen coordinate
  mapping. `new(cx, cy, screen_w, screen_h, world_w, world_h)` centres on the
  focus and clamps so the full viewport fits within the world. `world_to_screen`
  returns `None` for out-of-viewport world coords; `screen_to_world` clamps to
  the viewport edge. `recenter` moves the focus; `world_rect` returns the
  covered world rectangle. Implements `DetHash`. Satisfies taxonomy F5 —
  completing the F (Presentation) row. 15 new tests.
- `timer`: tick-based `Cooldown` and `TimerQueue<E>`. `Cooldown` is a simple
  saturating countdown (`tick(n)` returns `true` on the ready transition).
  `TimerQueue<E>` is a collection of future events — `schedule(delay, event)`
  and `schedule_repeat(delay, period, event)` — advanced by `advance(ticks)`
  which fires (and returns) all expired events in order and requeues repeating
  ones. Both implement `DetHash`; no float or OS clock anywhere. Satisfies
  taxonomy A4. 14 new tests.
- `cmdqueue`: deterministic command queue (`CmdQueue<C>`) — the replay-safe
  input abstraction. Commands are pushed between simulation ticks and drained
  in one atomic FIFO batch at the tick boundary; draining is the only way to
  consume commands so none are processed twice. Provides `push`, `push_batch`,
  `drain`, `clear`, `peek`, `len`, `is_empty`. `DetHash` (gated on `C:
  DetHash`) folds pending commands in insertion order so the queue participates
  in replay hash checks. Satisfies taxonomy G2. 11 new tests.
- `msglog`: bounded ring-buffer message log (`MsgLog`) for roguelike UI.
  Push is O(1); oldest messages are silently dropped when capacity is reached
  so memory stays bounded. Exposes `push`, `iter` (oldest-to-newest), `recent(n)`,
  `clear`, `len`, `is_empty`, and `capacity`. Implements `DetHash` (folds the
  visible history in order, independent of internal ring-buffer position) so the
  event log participates in snapshot/replay checks. A capacity-0 log discards
  immediately, useful in headless tests. 11 new tests.
- `vec`: fixed-point 2-D and 3-D vectors (`Vec2`/`Vec3`) over `Fixed`. Each
  type exposes `new`/`ZERO`, `dot`, `len_sq`, `len` (integer isqrt), `scale`,
  `normalize` (returns `None` for the zero vector — no panic, no garbage),
  `Add`/`Sub`/`Neg` operators (all saturating, no wrap), and `DetHash`.
  `Vec2` additionally has `perp` (90° CCW rotation). `Fixed` gains `MAX`/`MIN`
  constants. Covered by 21 new unit tests across arithmetic, Pythagorean
  identity, cross-product anti-commutativity, normalize, and saturation.
- `rng::weighted_index`: loot/spawn table draw — picks an index in proportion to
  its weight using a single wide-multiply draw (u64 analogue of `below`). Returns
  `None` without drawing for an empty slice or all-zero weights. Zero-weight
  entries are never chosen; the weight sum accumulates in `u64` so many large
  `u32` weights don't overflow. Covered by empty/all-zero, single-nonzero,
  proportional-distribution, and determinism tests.
- `rng::dice`: tabletop `NdM` roll — sums `count` independent draws from
  `1..=sides`; `sides == 0` or `count == 0` return `0` without drawing. Sum
  saturates instead of wrapping. Draws exactly `count` times, so draw position is
  a deterministic function of the arguments. Covered by sides-zero, count-zero,
  within-bounds, and determinism tests.
- `turn`: energy/speed-based turn scheduler — the roguelike progression core.
  Actors bank energy proportional to `speed` and act on reaching `ACTION_COST`;
  faster actors act more often and leftover energy carries over. Time advances by
  closed-form integer `ceil` division (no float), and same-unit ties break by
  smallest id — fully deterministic. Generic over the actor id; `det_hash` folds
  it (canonical id order) into replay-checked state. Covered by proportionality,
  tie-break, add/remove, set_speed, and determinism tests.
- `terminal`: the presentation layer the "terminal-first" engine was missing. A
  headless `Screen` of `Cell`s (glyph + 24-bit fg/bg) with edge-clipped drawing
  primitives (`set`/`put`/`clear`/`fill_rect`/`draw_str`), double-buffered
  `diff`/`present` change tracking, and a deterministic 24-bit ANSI serialiser
  (`to_ansi`). `Cell`/`Screen` implement `DetHash` so a frame folds into the
  world hash / snapshot tests. No OS I/O — writing to a tty is the caller's job.
- `GAME_DEV_TAXONOMY.md`: thorough categorisation of what a game needs
  (time/math/ECS/rng/determinism/rendering/input/content/world/AI/physics/
  gameplay/UI/persistence/net/tooling), subdivided and mapped to coverage, as
  the engine-enhancement roadmap.
- `geometry`: integer Bresenham `line` (king-move-contiguous, endpoints
  inclusive) and `line_of_sight` (single-ray check; opaque endpoints don't
  block). Float-free and deterministic. Covered by axis/diagonal/adjacency and
  LOS (open / interior wall / opaque-endpoint) tests.
- `replay`: deterministic replay & desync-detection harness, generalising the
  by-hand loop in `tests/determinism.rs`. `record_trace` produces a per-tick
  state-hash trace; `check_trace`/`first_divergence` report the first diverging
  tick (the desync-hunt starting point); `resimulate` clones a snapshot and
  replays inputs onto it (the rollback primitive). Engine-agnostic via a
  `step(&mut state, &input)` closure; state hashed through `DetHash`.
- `world_hash`: `hash_state` (fold any `DetHash` value to a `u64`); `DetHash`
  for `SplitMix64` (folds stream position) so RNG divergence shows in the hash.
- `sparse_set`: multi-component queries `join` and `join_mut` — the inner join
  of two component stores (entities present in both), returned in canonical
  ascending-index order so iteration is deterministic. `join_mut` yields a
  mutable reference to the first component for systems that update `A` from `B`
  (e.g. integrate `Position` by `Velocity`). The smaller store drives the scan.
- `mapgen`: deterministic procedural dungeon generation. Seed-driven rooms
  (rejection-placed with a one-cell border) joined by L-shaped corridors, so the
  map is always fully connected; all randomness comes from a supplied
  `SplitMix64` in fixed order, giving byte-identical maps per `(seed, params,
  size)`. `Dungeon` exposes `is_wall`/`is_floor` (OOB = wall, drop-in for `fov`
  and `pathfinding`) and implements `DetHash`. Covered by reproducibility,
  border, disjoint-rooms, full-connectivity (flood via `dijkstra_map`), and
  tiny-map tests.
- `SPEC.md`: a specification of the kit's API contracts, global invariants
  (zero-dep, no-unsafe, no-float-in-sim, replay-determinism) and a completeness
  checklist used to find and close gaps.
- `world_hash`: `DetHash` implementations for the primitives (`u32`, `u64`,
  `i32`, `bool`, `char`) and for the kit's value types (`Fixed`, `Entity`,
  `Position`, `Render`, `Color`), plus `SparseSet::det_hash` which folds a
  component store in canonical (ascending-index) order with its length — wiring
  the determinism checksum through the actual data types.
- `pathfinding`: `dijkstra_map` (multi-source Dijkstra distance field / flow
  field, budgeted by `max_cost`) and `descend` (deterministic steepest-descent
  step for chase/flee AI).
- `rng`: `range(lo, hi)` (uniform `[lo, hi)`, empty range returns `lo` without
  drawing) and `coin(num, den)` (probability `num/den`, degenerate odds resolved
  without drawing) — both low-bias and draw-count-deterministic.
- `pathfinding`: deterministic 8-directional A* grid pathfinding. Integer octile
  costs (10 orthogonal / 14 diagonal) and heuristic (no float), open set ordered
  by a total `(f, h, x, y)` key so the result is reproducible, fixed-order
  neighbour expansion, and no diagonal corner-cutting through walls. Generic over
  an `is_blocked` closure. Covered by straight-line, diagonal, detour,
  corner-cut, unreachable, and determinism tests.
- `fov`: symmetric recursive shadowcasting field-of-view (Albert Ford's
  algorithm). Integer-only rational slopes (no float), fixed quadrant/column
  iteration order, zero allocation — bit-identical across targets. Generic over
  `is_opaque`/`mark_visible` closures so it works with any grid. Guarantees
  symmetric visibility (A sees B iff B sees A); covered by open-field,
  shadow-casting, symmetry-property, and determinism tests.
- `fixed`: transcendental math without floats — `sqrt` (integer bit-by-bit
  square root) and `sin`/`cos`/`sin_cos`/`atan2` (rotation- and vectoring-mode
  CORDIC, 16 iterations, constants as integer literals). All results are
  bit-identical across targets and safe to feed the world hash. Negative `sqrt`
  saturates to zero; `atan2` is four-quadrant and degenerate-input safe. Covered
  by cardinal-angle, Pythagorean-identity, inverse, and determinism tests.
- `parser`: column-aware diagnostics. Every parse error now carries a 1-based
  column, and `Diagnostic::render` prints the source line with a caret under the
  offending token (rustc/clang style).
- `serializer`: `Content` -> `.game` text, the inverse of the parser. Output is
  canonical and idempotent (`serialize(parse(serialize(c))) == serialize(c)`).
- `content_eq`: semantic equality for round-trip checking.
- `timestep`: fixed-timestep accumulator with a death-spiral guard
  (`max_steps`), using integer nanoseconds.
- `gamec --fmt`: emit canonical `.game` text to stdout.
- Structure-aware round-trip fuzz test (3000+ generated bundles) and a
  canonical-form fuzz test (2000+ bundles).

### Changed
- `fixed`: `Add`/`Sub` now **saturate** instead of wrapping, and `mul`/`div`
  clamp the wide intermediate into range. Division by zero saturates by sign
  instead of panicking. This fixes a latent bug where an overflowing coordinate
  could silently flip to the opposite extreme.

## [0.1.0]

### Added
- `entity` / `sparse_set`: sparse-set ECS storage with generational handles.
- `fixed`: Q16.16 fixed-point scalar.
- `rng`: SplitMix64 seeded PRNG.
- `world_hash`: FNV-1a deterministic state checksum, with an end-to-end
  bit-exact replay test.
- `content` / `parser` / `validator` / `loader`: the text-to-ECS content
  pipeline.
- `gamec`: content checker CLI usable as a CI gate.
