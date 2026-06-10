# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `arch::ArchTable::count_where<F>(pred) -> usize`: count rows where
  `pred(entity, &row)` is true. Allocation-free complement to `iter().filter().count()`
  boilerplate. Useful for "how many enemies have > 50 HP?" queries.
- `arch::ArchTable::any<F>(pred) -> bool`: short-circuit scan returning `true` as
  soon as one row satisfies the predicate. Mirrors `Iterator::any`; avoids
  constructing and discarding iterators in "does any entity have X?" guards.
- `mapgen::Rect::area() -> u32`: product `w × h`. The fundamental size metric for
  rooms — used by `largest_room` internally; now exposed for caller sorting, area
  budget checks, and treasure density calculations.
- `mapgen::Dungeon::room_count() -> usize`: count placed rooms without exposing the
  `rooms` field. An O(1) `rooms.len()` shorthand; zero for caves and all-wall maps.
- `wfc::WfcGrid::count_uncollapsed() -> usize`: cells still awaiting collapse
  (bitmask popcount ≠ 1). Complement of `is_fully_collapsed`; useful for progress
  bars and "abort after N tries" limits during WFC post-processing.
- `wfc::WfcRules::is_valid_tile(tile) -> bool`: range-check `tile < tile_count()`.
  The `allow`/`disallow` family silently ignores out-of-range tiles; this lets
  callers validate before mutating rules.
- `profiler::EventLog::oldest() -> Option<&LogEntry<E>>`: oldest entry still
  retained in the ring (complement of `last`). Useful for "how long since the first
  visible event?" TTL calculations and replay-window diagnostics.
- `replay::replay_ok(expected, actual) -> bool`: `first_divergence().is_ok()`
  shorthand. Removes the `.is_ok()` boilerplate at assert sites and CI gate checks
  that only need a boolean result.
- `fixed::Fixed::clamp01(self) -> Fixed`: clamp a fixed-point value into `[0, 1]`
  with one call. Delegates to `clamp(Fixed::ZERO, Fixed::ONE)`. Eliminates the
  two-argument `clamp` boilerplate for the most common case — normalised weights,
  alpha values, and easing `t` parameters.
- `rng::SplitMix64::range_u32(lo, hi) -> u32`: uniform draw from `[lo, hi)` over
  `u32` values. Mirrors `range` but without the `i32` sign-extension concern;
  `lo >= hi` returns `lo` without drawing (same contract as `range`). Deterministic
  and replay-safe.
- `fov::fov_ring(origin, radius, is_opaque) -> Vec<(i32,i32)>`: visible cells
  exactly on the Chebyshev shell at `radius`. Filters `fov_to_vec` to cells where
  `max(|dx|, |dy|) == radius`. Useful for "what can I see at exactly range R?"
  ring queries (area denial, light halos).
- `passability::PassabilityGrid::invert()`: flip every cell: passable becomes
  blocked, blocked becomes passable. The "negative" of the current grid. Useful for
  building complement masks, "invert the dungeon" effects, and test scaffolding.
- `influence::InfluenceMap::max_value() -> Option<i32>`: maximum value stored in
  any cell (`None` for a zero-size map). O(n) scan via `Iterator::max`. Useful for
  normalising influence layers before rendering a heatmap overlay.
- `hud::BarWidget::is_full() -> bool`: `true` when `current >= max` (or `max <= 0`
  for degenerate bars). Lets callers skip the render or suppress a "healing
  available" prompt without a manual comparison.
- `autotile::SimpleTileTable::set_count() -> usize`: number of entries with a
  non-zero tile id — i.e. how many mask patterns have been explicitly assigned.
  Useful for validating that a table is fully or partially initialised.
- `vec::Vec2::is_zero(self) -> bool`: `true` when both `x` and `y` are
  `Fixed::ZERO`. Delegates to `Fixed::is_zero` so it shares the same zero-check
  semantics. Useful for direction guards ("don't normalise a zero vector").
- `aabb::Aabb::perimeter() -> i32`: sum of all four edge lengths `2 × (w + h)`,
  with saturating arithmetic. Returns `0` for empty boxes. For AoE radius
  estimation, hitbox tuning, and "perimeter ≤ budget?" guards.
- `geometry::midpoint(a, b) -> (i32, i32)`: integer midpoint of two grid cells,
  flooring toward `a` on odd separations. Overflow-safe. Useful for marker
  placement, segment bisection, and symmetric projectile arcs.
- `tilemap::TileMap::any_where(pred) -> bool`: short-circuit scan that returns
  `true` as soon as one cell satisfies the predicate. Avoids the allocating
  `find_all(pred).is_empty()` pattern for simple "does a wall exist here?" checks.
- `combat::Stats::is_bloodied() -> bool`: `true` when HP is below 50% of
  `max_hp`. Classic D&D 4e "bloodied" condition. Useful for AI aggression
  thresholds, conditional abilities, and low-health visual indicators.
- `spatial_hash::SpatialHash::count_in_radius_euclidean(qx, qy, radius) -> usize`:
  count entities within Euclidean radius without allocating a `Vec`. Allocation-
  free counterpart to `query_radius_euclidean` for "how many enemies are nearby?"
  density checks.
- `pathfinding::is_reachable(start, goal, is_blocked) -> bool`: thin wrapper
  around `astar` that discards the path and returns only the reachability boolean.
  Avoids constructing the full path Vec for connectivity-only checks.
- `easing::lerp(a, b, t) -> Fixed`: linear interpolation `a + (b−a)×t`. The
  fundamental building block for all easing curves — pair with any ease function
  via `lerp(a, b, ease_fn(t))`.
- `noise::noise_2d_in_range(x, y, seed, lo, hi) -> i32`: convenience combinator
  for `hash_range(hash_2d(x, y, seed), lo, hi)`. Scatter distinct values across a
  map with a single call; avoids repeating the two-call pattern everywhere.
- `camera::Camera::world_to_screen_unclamped(wx, wy) -> (i32, i32)`: convert a
  world-space point to a screen-space offset without bounds checking. Returns
  negative values or out-of-range coordinates for off-screen points. Complement
  of `world_to_screen` for renderers that need raw offset math (e.g. partial
  sprites, infinite worlds).
- `change::Changed::if_changed(since_tick) -> Option<&T>`: return the inner value
  only when changed at or after `since_tick`, otherwise `None`. Combines
  `is_changed_since` with value access to avoid the two-step pattern at every call
  site in system code.
- `status::StatusSet::active_keys() -> Vec<&K>`: list all keys with active effects
  in application order. Useful for rendering a status HUD, serialising active
  debuffs, or iterating all active effects without exposing the internal `entries`
  field.
- `menu::Menu::cursor_label() -> Option<&str>`: label of the item currently under
  the cursor, or `None` on an empty menu. Avoids `current()?.label.as_str()` at
  every rendering call site.
- `random_table::RandomTable::max_weight() -> u32`: highest single-entry weight in
  the table (`0` for empty/all-zero tables). Useful for "is this table uniform?"
  checks and tuning loot balance without manual iteration.
- `cmdqueue::CmdQueue::contains(pred) -> bool`: check whether any queued command
  satisfies a predicate without consuming the queue. Useful for abort-before-commit
  patterns ("is a Cancel command already queued?").
- `inputbuf::InputBuffer::is_repeating(key) -> bool`: `true` if the key is held
  and past the `initial_delay` threshold — i.e. in the repeat phase. Lets UI code
  distinguish first-press from auto-repeat to suppress single-fire effects.
- `relations::Relations::child_count(entity) -> usize`: count direct children
  without allocating a `Vec`. Drop-in replacement for `children_of(e).len()` in
  hot loops and capacity guards.
- `dice::Dice::span() -> u32`: spread between minimum and maximum outcomes
  (`max() − min()`). Modifier cancels so only `count × (sides − 1)` matters;
  useful for balance heuristics and variance comparisons.
- `timer::Cooldown::extend(extra)`: add ticks to an existing cooldown (saturating).
  The "slow" counterpart to `tick`; pushes back ability availability without
  resetting the full timer.
- `msglog::MsgLog::push_unique(msg)`: push only when the message differs from the
  most recent entry. Deduplicates consecutive identical strings ("you hit the orc"
  spam) without requiring the caller to track the last message.
- `inventory::Inventory::move_to_slot(from, to) -> bool`: relocate an item between
  two slots without a remove/add round-trip. Returns `false` if `from` is empty,
  `to` is occupied, or either index is out of bounds.
- `fsm::Fsm::is_in(state) -> bool`: `self.state() == state` shorthand. Avoids
  importing and comparing state variants at every AI call site.
- `turn::Scheduler::drain() -> Vec<A>`: remove all actors and return their ids in
  insertion order. End-of-floor cleanup in one pass without iterating the scheduler.
- `multimap::MultiMap::remove_connector(from_floor, from_x, from_y) -> bool`:
  remove the connector at a source position. The inverse of `add_connector`; returns
  `false` if no matching connector is found.
- `keymap::KeyMap::action_count() -> usize`: count distinct actions with at least
  one binding. Useful for "are all N actions mapped?" completeness checks.
- `entity::EntityAllocator::free_count() -> usize`: O(1) count of freed (reusable)
  slots. Equivalent to `total_slots() - count()`. Memory diagnostics for systems
  monitoring allocator pressure.
- `sparse_set::SparseSet::remove_where(pred) -> usize`: bulk-remove all entries
  matching a predicate; returns the count removed. Avoids a temporary Vec for
  filtered despawn passes.
- `world_hash::Fnv1a::write_str(s)` + `DetHash for str` / `DetHash for String`:
  fold UTF-8 string bytes into the hasher. Lets entity names, config keys, and
  string fields participate in world state checksums without manual `.as_bytes()`.
- `content::Color::from_hex(s) -> Result<Color, String>`: inverse of `to_hex`;
  delegates to `parse_color` for consistent validation and error messages.
- `content::Content::tile(name) -> Option<&Tile>`: look up a tile definition by
  name. Symmetric with `Content::prefab` and `Content::level`.
- `parser::warning_count(diags) -> usize`: count warning-severity diagnostics —
  the complement of `error_count`. Exposes the warning tally for CI gates that
  report warnings separately from errors.
- `savefile::estimate_save_size(payload_len) -> usize`: predict the byte length
  of a `save_bytes` buffer (`20 + payload_len`) before allocating or writing to
  storage. Avoids over-allocation for streaming writers.
- `textlayout::truncate_lines(lines, max_cols) -> Vec<String>`: apply `truncate`
  to every line in a slice. Batch-clips multi-line HUD panels and dialogue boxes
  to a fixed column width with one call.
- `content::Color::alpha_blend(fg, bg, alpha) -> Color`: composite `fg` over
  `bg` with integer alpha (`0` = fg, `255` = bg) using per-channel
  `(fg*(255-alpha) + bg*alpha)/255`. No float; deterministic. For UI overlays,
  particle fading, and damage indicators.
- `parser::error_count(diags) -> usize`: count error-severity diagnostics in a
  slice — CI / tool convenience, avoids manual iteration.
- `serializer::first_diff(a, b) -> Option<String>`: describe the first semantic
  difference between two `Content` values (prefab count, level rows, etc.) for
  debugging round-trip failures.
- `validator::error_count(diags) -> usize`: same helper as `parser::error_count`
  but exposed from the validator module for gate-check idioms.
- `entity::EntityAllocator::highest_generation() -> u32`: max generation counter
  across all slots (`0` when empty). Useful for diagnostics and save-file audits
  to detect near-exhausted generation space.
- `fov::fov_to_vec_dist(origin, radius, max_dist_sq, is_opaque) -> Vec<(i32,i32)>`:
  like `fov_to_vec` but further filtered to `dist_sq <= max_dist_sq`. One-pass
  light-falloff/torch-radius queries without a second Vec filter.
- `mapgen::Dungeon::largest_room() -> Option<Rect>`: room with the greatest area
  (`w × h`); `None` for all-wall dungeons (caves, tiny maps). Spawn bosses,
  stairs, and treasure in the biggest room.
- `savefile::LoadError::is_recoverable() -> bool`: `true` for `BadMagic` (wrong
  file — treat as empty slot), `false` for `TooShort` or `ChecksumMismatch`
  (corruption — warn and offer to clear). Guides save-slot UI error handling.
- `fixed::Fixed::pow(exp: u32) -> Fixed`: integer exponentiation via repeated
  saturating multiplication. `pow(0)` is `Fixed::ONE`, `pow(1)` is `self`
  exactly; overflow pins to `Fixed::MAX`/`MIN` rather than wrapping. Float-free
  and deterministic.
- `geometry::line_len(a, b) -> usize`: number of cells [`line`] returns without
  allocating — Chebyshev distance plus one. For sizing buffers or range checks
  before tracing a Bresenham ray.
- `menu::Menu::label_at(idx) -> Option<&str>`: display label of the item at
  `idx` (`None` if out of range), without exposing the whole `MenuItem`.
- `keymap::KeyMap::unbind_action(action) -> usize`: remove every binding for a
  given action, returning the count removed. The inverse of `bind_multiple`.
- `noise::hash_range(h, lo, hi) -> i32`: map a raw `u32` hash into `[lo, hi)`
  with an unbiased wide multiply (`lo` for degenerate ranges). Suits per-cell
  scatter tables; distinct from `normalize_noise` which scales the `[0,65535]`
  smooth-noise range. Deterministic and float-free.
- `pathfinding::octile_distance(a, b) -> i32`: the octile heuristic cost A* pays
  to cross open ground (`10`/`14` scale). Estimate path length or range without
  running a search.
- `textlayout::count_lines(text, max_cols) -> usize`: number of lines
  `wrap_words` would produce, for pre-measuring wrapped-block height (dialogue
  boxes, tooltips) without keeping the wrapped `Vec`.
- `timestep::FixedTimestep::reset_accumulator() -> u64`: discard accumulated
  sub-step time (returning the dropped nanoseconds) so a resume after a pause or
  level load does not replay buffered time as a burst of catch-up steps. Leaves
  `total_steps` and the configured rate untouched.
- `aabb::Aabb::touches(&self, other: &Aabb) -> bool`: true when two boxes are
  adjacent (shared edge or corner) but do not overlap. Empty boxes never touch;
  diagonal corner contact counts. Built on `grow(1)` + `overlaps`, all saturating.
  Supports roguelike adjacency / "reach" interaction checks.
- `combat::Stats::missing_hp(&self) -> i32`: health deficit below maximum
  (`max(0, max_hp − hp)`), i.e. healing needed to reach full. Clamps at 0 for
  over-full HP. Useful for healing AI and damage-preview UI.
- `inventory::Inventory::find_mut<F>(pred) -> Option<&mut T>`: mutable complement
  to `find`; returns the first item matching the predicate so it can be modified
  in place without a slot round-trip.
- `msglog::MsgLog::filtered<P>(pred) -> Vec<&str>`: collect references to all
  messages matching a predicate, oldest-to-newest, without mutating the log
  (unlike `retain`). Supports "show only combat messages" filtered views.
- `rng::SplitMix64::pick_index(len) -> Option<usize>`: uniform random index in
  `0..len` (`None` for `len == 0`, no draw). The index-only primitive behind
  `pick`/`pick_mut`, for when data lives in parallel arrays or an ECS store.
  Consumes exactly one draw — replay-deterministic.
- `sparse_set::SparseSet::find_entity_where<F>(pred) -> Option<Entity>`: return
  the first entity whose component satisfies the predicate. The component-query
  complement to `count_matching`; scans in dense order.
- `tilemap::TileMap::bounds_of<P>(pred) -> Option<Aabb>`: minimal half-open
  `Aabb` enclosing every cell matching the predicate (`None` if none match). A
  single cell yields a 1×1 box. Useful for room / spell-area / painted-region
  bounds without manual min/max bookkeeping.
- `turn::Scheduler::peek_next_turn(&self) -> Option<A>`: id of the actor who
  would act on the next `next_turn`, without advancing time or deducting energy.
  Uses the same smallest-id tie-break, so it is a true non-destructive preview
  for "whose turn?" UI and AI look-ahead.
- `change::Changed::is_fresh(max_age_ticks, current_tick) -> bool`: logical
  inverse of `is_stale`; returns `true` when the component was marked within the
  last `max_age_ticks` ticks. Avoids double-negation at call sites and makes
  "only process recently updated components" filters self-documenting.
- `fsm::Fsm::transition_count(from: &S) -> usize`: count outgoing transitions
  from a state without constructing an iterator. Equivalent to
  `transitions_from(state).count()` but avoids the closure overhead for the
  common "does this state have any exits?" guard.
- `status::StatusSet::magnitude_range() -> (i32, i32)`: return the `(min, max)`
  magnitude across all active effects. Returns `(0, 0)` for an empty set. Useful
  for "net buff range" queries and AI tuning without iterating manually.
- `inputbuf::InputBuffer::set_timing(initial_delay, repeat_period)`: update
  hold-repeat timing parameters without clearing the buffer or releasing held keys.
  Clamps `repeat_period` to ≥ 1. Supports "haste" power-ups and in-session
  accessibility setting changes.
- `vec::Vec2::reflect(self, normal: Vec2) -> Vec2`: reflect a vector off a
  surface with the given unit normal via `self − 2·(self·n)·n`. Assumes
  `normal` is already unit length. Useful for physics-style bounce responses
  in collision resolution and beam/projectile ricochet.
- `multimap::MultiMap::connectors_between(from_floor, to_floor) -> Vec<&Connector>`:
  all connectors originating on `from_floor` and leading to `to_floor`. Returns
  an empty `Vec` when none exist. Supports multi-staircase levels where several
  exits between the same floor pair need to be enumerated.
- `dice::Dice::roll_with_reroll(reroll_on, rng) -> i32`: roll once; if the
  result is ≤ `reroll_on`, roll a second time and return that result regardless
  of whether it is better. Consumes two draws on reroll, one otherwise. Implements
  the "reroll on 1" mechanic from roguelike ability checks and tabletop RPGs.
- `timer::Cooldown::fractional_progress(original_ticks) -> Fixed`: fractional
  progress through the cooldown as a Q16.16 `Fixed` value in `[0, 1]` (`0` = just
  started, `1` = ready). `original_ticks == 0` returns `Fixed::ONE`. Suitable as
  a lerp parameter for smooth animation without converting to float or percent.
- `wfc::WfcRules::clear_adjacencies(tile: u8)`: clear all adjacency bitmasks for
  `tile` in every direction, resetting it to "forbidden everywhere". Silently
  ignores out-of-range tile indices. Supports rule-mutation workflows where a tile
  is conditionally disabled at runtime.
- `passability::PassabilityGrid::random_passable(rng: &mut SplitMix64) -> Option<(i32, i32)>`:
  pick a uniformly random passable cell via `SplitMix64::pick`. Draws exactly one
  RNG value when at least one passable cell exists; draws nothing for an all-blocked
  grid. Primary use: spawn-placement, random patrol targets.
- `profiler::EventLog::count_by<F>(pred: F) -> usize`: count stored events
  matching a predicate without allocating — `self.iter().filter(|e| pred(&e.event)).count()`
  packaged as a named method. Avoids the iterator boilerplate in common "how many
  damage events this tick?" patterns.
- `random_table::RandomTable::weighted_idx(&self, rng: &mut SplitMix64) -> Option<usize>`:
  like `roll` but returns the entry index instead of the value. Draws once;
  returns `None` for empty/all-zero tables. Useful when the caller needs to inspect
  or remove the rolled entry rather than just use its value.
- `replay::count_divergences(expected: &[u64], actual: &[u64]) -> usize`: count
  all diverging ticks (unlike `first_divergence` which stops at the first one).
  Ticks beyond the shorter trace count as divergences. Also re-exported at the
  crate root.
- `relations::Relations::reparent_all(old_parent, new_parent) -> bool`: move all
  children of `old_parent` to `new_parent` in one call. Returns `true` if at least
  one child was moved; `false` for same-parent or childless cases. Builds on
  `detach_all_children` + `attach` and preserves the existing `DetHash` contract.
- `assets::AssetStore::handles() -> impl Iterator<Item = AssetHandle<T>>`: iterate
  over live handles without borrowing values. Useful when constructing a list of
  handles for batch remove / validity checks without holding a value borrow.
- `spatial_hash::SpatialHash::all_in_cell(cx, cy) -> Vec<K>`: return an owned
  `Vec` of all keys in cell-space cell `(cx, cy)`. Unlike `query_cell` which
  returns a borrowed slice, the owned `Vec` lets the caller mutate the hash during
  iteration. Returns an empty `Vec` for unoccupied cells.
- `arch::ArchTable::with_capacity(n) -> Self`: pre-allocate both the dense Vec
  and the HashMap for `n` entities, avoiding repeated reallocation on bulk
  insertions. Equivalent to `new()` in all other respects; deterministic hash is
  unchanged.
- `camera::Camera::screen_distance(sx1, sy1, sx2, sy2) -> u32`: Chebyshev
  distance between two screen-space cells — `max(|sx1−sx2|, |sy1−sy2|)`.
  Static method; requires no camera state. Matches the king-move metric used by
  `chebyshev_to_center`. Useful for "is this enemy within cursor range?" UI checks.
- `cmdqueue::CmdQueue::as_slice(&self) -> &[C]`: view pending commands as a
  slice without consuming them. Follows the Rust `as_slice` convention and
  complements the existing `peek()`; callers that need slice-specific methods
  (`windows`, `chunks`, `starts_with`) can use this directly.
- `diag_json::severity_filter(diags, severity) -> Vec<Diagnostic>`: filter a
  diagnostic slice to a given severity level, preserving order. Useful for
  routing errors and warnings to separate outputs (stderr vs. log) before calling
  `diag_json`. Also re-exported at the crate root.
- `hud::HudPanel::pad(left, top, right, bottom) -> HudPanel`: shrink a panel by
  explicit per-side margins, returning a new panel. Width and height saturate at 0
  when margins exceed the panel size. Complements the fixed-1-cell `inner_*`
  helpers with caller-controlled insets.
- `influence::InfluenceMap::gradient_at(x, y) -> Option<(i32, i32)>`: the step
  direction `(dx, dy)` of steepest ascent from `(x, y)` — equivalent to
  `highest_neighbour` without the value. Returns `None` when there are no
  in-bounds neighbours (same condition as `highest_neighbour`). Primary use:
  AI pathfinding step "move toward maximum influence."
- `easing::ease_reversed(t, ease_in) -> Fixed`: converts any ease-in function to
  its ease-out mirror by time-reversal (`1 − f(1 − t)`). Works with all standard
  Penner ease-in functions. Also re-exported at the crate root.
- `autotile::SimpleTileTable::fill_range(start, end, tile_id)`: set all 256-entry
  table slots in the inclusive range `[start, end]` to `tile_id`. Replaces
  repeated `set()` calls when initialising consecutive mask groups (e.g. all
  cardinal-only masks). No-op when `start > end`.
- `rng::SplitMix64::pick_mut(&mut [T]) -> Option<&mut T>`: mutable variant of
  `pick` — select a uniform-random element and hand back a mutable reference so
  the caller can modify it in-place without an index round-trip. No draw for
  empty slices.
- `fixed::Fixed::hypot(a, b) -> Fixed`: Euclidean distance `sqrt(a²+b²)`,
  computed with the existing `mul`+`sqrt` chain. Replaces the manual
  `(a.mul(a).saturating_add(b.mul(b))).sqrt()` pattern at call sites. Saturates
  for very large inputs the same way `mul` does.
- `aabb::Aabb::from_center_size(cx, cy, w, h) -> Aabb`: construct a box from a
  center point and dimensions. Top-left is `(cx - w/2, cy - h/2)` (integer
  division, same truncation bias as `center()`). Negative sizes are clamped to 0.
  Mirrors `glam`'s `from_center_half_size` convention.
- `tilemap::TileMap::mutate_where(pred, transform)`: apply a `FnMut(T) -> T`
  transformation to every cell for which `pred` returns `true`, modifying in
  place. The standard "apply poison decay / environmental effect to all matching
  tiles" primitive. O(n) in map size; no allocation.
- `menu::Menu::remove_item(idx)`: delete the item at `idx` and keep the cursor
  valid — decrements if the cursor was past the removed item, clamps to the new
  last item otherwise. No-op for out-of-bounds indices. Needed for dynamic menus
  (quest log entries, loot picks).
- `inventory::Inventory::get_mut(slot) -> Option<&mut T>`: mutable borrow of the
  item at `slot`. Allows in-place modification (durability tick, stack count)
  without the `remove` + `add` round-trip that changes the slot index.
- `turn::Scheduler::time_until_ready(id) -> Option<i32>`: time units until actor
  `id` first accumulates `ACTION_COST` energy. Returns `Some(0)` when already
  ready, `None` when the actor is unknown. Does not advance the queue — pure read.
  Useful for AI look-ahead ("the goblin acts in N turns") and UI countdowns.
- `msglog::MsgLog::pop() -> Option<String>`: remove and return the most recently
  pushed message (LIFO). Returns `None` for an empty log. Useful for "undo last
  event message" and round-trip tests.
- `fov::fov_to_vec(origin, radius, is_opaque) -> Vec<(i32,i32)>`: collect all
  visible cells into a `Vec` in one call. Equivalent to bracket-lib's
  `field_of_view_set` — wraps `compute_fov` so callers don't need to manage
  a closure-owned collection. Re-exported as `izanagi_kit::fov_to_vec`.
- `combat::roll_damage(rng, base, variance) -> i32`: base damage plus a uniform
  random bonus in `[0, variance]`. `variance == 0` returns `base.max(0)` without
  consuming an RNG draw. Gives attacks a natural spread (e.g. `roll_damage(rng, 5,
  3)` → 5–8) without spelling out the `Dice` formula at every call site.
- `relations::Relations::siblings_of(entity) -> Vec<Entity>`: entities that share
  the same parent as `entity`, excluding `entity` itself. Returns an empty `Vec`
  for roots and only children. Useful for item-group queries ("other items in the
  same container") and squad-member adjacency patterns.
- `timer::Cooldown::percent_remaining(original_ticks) -> u32`: percentage of the
  cooldown still pending as an integer in `[0, 100]`. The inverse of `elapsed`;
  `original_ticks == 0` returns 0 (ready). Useful for progress-bar rendering.
- `savefile::validate_integrity(data) -> Result<(), LoadError>`: check that `data`
  is a structurally valid save file without deserialising the payload. Equivalent
  to `load_bytes(data).map(|_| ())`. Useful for save-slot browser UI that needs a
  "valid / corrupt" indicator cheaply. Re-exported as `izanagi_kit::validate_integrity`.
- `textlayout::fit_to_box(text, width, height) -> Vec<String>`: wrap text to fit
  a `width × height` box in one call — combines `wrap_words_max_lines` with the
  `…` overflow indicator. The typical "dialogue box" call site in roguelike UIs.
  Re-exported as `izanagi_kit::fit_to_box`.
- `change::Changed::is_stale(age_threshold, current_tick) -> bool`: returns `true`
  if the component has not been marked for at least `age_threshold` ticks
  (`ticks_since_change >= age_threshold`). Eliminates the comparison boilerplate
  at TTL / cache-invalidation call sites.
- `spatial_hash::SpatialHash::iter_keys() -> impl Iterator<Item = &K>`: flat
  iterator over every registered key across all cells, without intermediate
  allocation. Useful for "process all entities in the spatial index" end-of-frame
  passes where cell membership is irrelevant.
- `keymap::KeyMap::contains_action(action)`: returns `true` if at least one key
  is currently bound to `action`. Allocation-free (no collect) — use
  `get_keys_for_action` when you also need the key list.
- `keymap::KeyMap::bind_multiple(keys, action)`: bind every key in a slice to
  the same action in one call; existing bindings for any key in the slice are
  replaced. Ergonomic alias for the repeated `bind` pattern seen in default
  key-layout setup code.
- `entity::EntityAllocator::total_slots()`: O(1) count of all slots ever
  created (both live and freed). Useful for memory-budget checks and save-file
  headers that need to know the allocator high-water mark.
- `geometry::rect_contains(x, y, w, h, px, py)`: fast inline rectangle
  point-in-bounds check. Returns `false` for empty rectangles (`w ≤ 0` or
  `h ≤ 0`). Replaces the recurring `px >= x && px < x+w && py >= y && py < y+h`
  pattern at call sites.
- `world_hash::DetHash for (A, B)` and `DetHash for (A, B, C)`: folds tuple
  fields left-to-right, identical to sequential individual `det_hash` calls.
  Enables hashing heterogeneous pairs/triples (e.g. `(Entity, Position)`) used
  as canonical map keys without an intermediate struct.
- `noise::ridge_noise_2d(x, y, seed, octaves)`: ridged multifractal noise —
  folds each FBM octave through `|raw − 32768|` to produce sharp ridges.
  Normalised to `[0, 65535]`. Builds mountain-range and river-valley heightmaps
  from the same deterministic value-noise primitive.
- `aabb::Aabb::from_corners(x1, y1, x2, y2)`: construct from two corners in any
  order — the result always has non-negative `w`/`h`. Matches the bracket-lib and
  glam `from_min_max` convention; avoids manual `min`/`max` arithmetic at call sites.
- `aabb::Aabb::grow(amount)` / `shrink(amount)`: expand or contract by a uniform
  margin on all four sides. Saturating arithmetic. Negative `amount` to `grow` is
  equivalent to `shrink`. Size clamps to zero so the result is always a valid AABB.
- `noise::value_noise_2d_wrap(x, y, seed, period_x, period_y)`: 2-D value noise
  that tiles seamlessly at the given integer period. Corner hashes use `rem_euclid`
  to wrap, so `noise(0, y) == noise(period_x, y)` exactly. Essential for seamless
  dungeon level textures and world-map wrapping.
- `noise::fbm_2d_wrap(x, y, seed, octaves, period)`: tileable 2-D FBM — each octave
  wraps at `period << octave_index` so harmonics also tile. Builds naturally-looking
  wrapping terrain via layered `value_noise_2d_wrap`.
- `random_table::RandomTable::roll_n(n, rng)`: draw `n` independent samples with
  replacement, returning `Vec<T>`. Consumes no draws for `n == 0` or an empty table.
  The canonical "roll the encounter table three times" primitive.
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

- `change::Changed::reset(tick)`: acknowledge a change without modifying the
  wrapped value — sets `changed_at` to `tick` so a subsequent
  `is_changed_since(tick + 1)` returns `false`. The canonical "mark as seen"
  operation for multi-pass systems that should not re-process a change they
  already handled.
- `change::ChangeTracker::reset()`: reset the tick counter to `0`. Needed when
  restoring world state from a save file or rewinding a replay so that all
  `is_changed_since` queries treat restored state as fresh.
- `profiler::Profiler::budget_exceeded(section, budget)`: returns `true` if the
  current-tick total for `section` exceeds `budget`. Zero-overhead convenience
  check for per-frame budget monitoring ("did pathfinding exceed 1 ms this
  tick?") without repeating the `this_tick > N` comparison at each call site.
- `profiler::EventLog::filter_by_tick_range(start, end)`: iterate entries whose
  tick falls within `[start, end]` (inclusive), oldest first. Enables replay
  inspection and assertion patterns like "what events occurred during turns 5–10?"
- `autotile::compute_region(x, y, w, h, is_same)`: compute auto-tile masks for
  only the `w × h` subregion starting at `(x, y)` rather than the entire map.
  `is_same` receives absolute coordinates so diagonal-corner clearing works
  correctly across region boundaries. Use this after any terrain mutation to
  avoid recalculating masks for the whole map.
- `fsm::Fsm::transitions_from(state)`: iterate all events with a defined
  outgoing transition from `state`. Returns `&E` in insertion order. Does not
  include self-loops for unmapped events — only explicitly added transitions.
  Useful for "available actions" UIs and AI planners that need to enumerate what
  can happen from a given state.
- `inputbuf::InputBuffer::all_held()`: iterator over all currently held key
  values. The missing modifier-query API: "is Shift/Ctrl held while I process
  this event?" without having to track modifier keys separately.
- `cmdqueue::CmdQueue::drain_if(pred)`: drain only commands matching a
  predicate; unmatched commands stay in the queue in their original order.
  Enables per-system selective consumption when multiple subsystems share one
  queue (e.g. movement commands vs. UI commands).
- `menu::Menu::set_enabled(idx, enabled)`: toggle the disabled flag on an item
  at runtime. Fills the gap between create-time `add_disabled` and run-time
  "grey out unavailable actions" (shop items you can't afford, abilities on
  cooldown). Silently ignores out-of-range indices.
- `menu::Menu::find_by_label(label)`: linear search for the first item with
  a matching label, returning `Some(idx)` or `None`. Case-sensitive. The missing
  link for search-and-jump-to-item patterns in large menus.
- `arch::ArchTable::retain(pred)`: remove all entries for which `pred(entity,
  &row)` returns `false`. Iterates in reverse dense-array order so swap-removes
  stay O(n) — the canonical "cull dead entities" frame-cleanup primitive,
  matching `Vec::retain` semantics. Parallel to `SparseSet::retain`.
- `arch::ArchTable::values()` / `values_mut()`: entity-handle-free row iterators.
  Mirrors the `SparseSet::values()`/`values_mut()` additions — avoids unpacking
  the entity handle when only component data is needed (e.g. ticking all AI rows).
- `content::Color::invert()`: channel-wise complement `rgb(255−r, 255−g, 255−b)`.
  `const` fn, zero-cost. Useful for selection highlights, contrast checks, and
  visual effects. Double inversion is the identity: `c.invert().invert() == c`.
- `content::Color::luminance()`: perceived luma as a single `u8` using integer
  Rec. 601 weights — the same value as each channel of `grayscale()`, exposed
  directly. Avoids constructing a full `Color` when only the brightness scalar is
  needed (e.g. "is the foreground readable on this background?").
- `loader::Stats`: numeric stats component. Loaded from each spawned prefab's
  `stats` BTreeMap in deterministic (alphabetical) order. Stored in
  `LoadedLevel::stats` (a `SparseSet<Stats>`); absent for prefabs with no stats so
  the common case (no stat block) pays no memory. `get(key)` / `iter()` / `len()`
  / `is_empty()` API. `DetHash`-capable for world hashing.
- `textlayout::wrap_words_max_lines(text, max_cols, max_lines)`: like `wrap_words`
  but caps the output at `max_lines`. When overflow occurs, the last visible line is
  truncated with `…` to signal cut-off content. Useful for bounded text areas such
  as in-game book pages, dialogue boxes, and log panels.
- `hud::HudPanel::split_h(n)` / `split_v(n)`: divide a panel into `n` equal
  horizontal strips (top-to-bottom) or vertical strips (left-to-right). Heights /
  widths are distributed by integer division; any remainder goes to the first strip.
  Returns an empty `Vec` for `n == 0`. Enables declarative multi-pane HUD layouts
  without hard-coding pixel offsets.
- `validator::validate`: glyph printability check — control characters (U+0000–
  U+001F, U+007F) in a prefab or tile glyph are reported as errors. Terminal cells
  have no sensible rendering for control chars; the check prevents invisible/corrupt
  glyphs from reaching `loader::load_level`.
- `validator::validate`: unused-prefab warning — prefabs defined but never
  referenced by any level spawn emit a `Severity::Warning`. Helps authors catch
  dead definitions and renamed-but-not-updated spawn tables.
- `passability::PassabilityGrid::is_passable(x, y)`: convenience inverse of
  `is_blocked` — returns `true` when the cell is open, `false` when blocked or
  out-of-bounds. Eliminates the repetitive `!grid.is_blocked(…)` negation at
  call sites where the positive condition ("can the actor step here?") is cleaner
  to read.
- `camera::Camera::set_screen_size(screen_w, screen_h, world_w, world_h)`:
  resize the viewport dimensions and re-clamp so the full viewport stays within
  the world. The world-space centre is preserved so the view expands
  symmetrically — the correct behaviour for terminal `SIGWINCH` handlers where
  the game redraws after a resize.
- `relations::Relations::detach_all_children(parent)`: orphan every direct child
  of `parent`, making them root entities, and return them as a `Vec<Entity>`.
  Leaves `parent`'s own parent relationship intact. The canonical "drop all
  carried items on death" pattern — call it, then dispatch the returned entities
  to the spawner or inventory-drop system.
- `msglog::MsgLog::get(index)`: random access by logical position (0 = oldest).
  Returns `None` for `index >= len`. Complements `iter()` when a scrollable log
  cursor needs to read a specific line without collecting the full iterator.
- `msglog::MsgLog::first()`: the oldest visible message, or `None` if empty.
  Mirrors `last()` — useful for "show oldest unread event" patterns.
- `msglog::MsgLog::retain(pred)`: keep only messages for which `pred` returns
  `true`, preserving oldest-to-newest order. Rebuilds the ring buffer in-place;
  capacity is unchanged. Useful for filtering combat-only vs. exploration events
  in a multi-channel log.
- `tilemap::TileMap::contains(x, y)`: `true` if `(x, y)` is within map bounds.
  Saves the `get(x,y).is_some()` pattern; works as a bounds guard before calling
  `get_mut` or `swap`.
- `tilemap::TileMap::swap(x1, y1, x2, y2)`: exchange tiles at two cells. No-op
  if either coordinate is out of bounds or both are equal. Useful for sliding-
  puzzle mechanics and map editing without a temporary variable.
- `tilemap::TileMap::count_where(pred)`: count cells for which `pred` returns
  `true`. The idiomatic way to answer "how many walls?", "how many lit cells?",
  etc. without a manual `iter().filter().count()` chain.
- `tilemap::LayeredMap::fill_all(tile)`: fill every cell of every layer with
  `tile`. The single-call equivalent of iterating `layer_mut(i).fill(tile)` for
  all layers — useful for resetting the entire world state on scene transitions.
- `timer::Cooldown::set_ready()`: instantly set `remaining = 0` so
  `is_ready()` returns `true` on the next check. Used by "haste" effects and
  level-up resets that clear all ability cooldowns.
- `timer::Cooldown::elapsed(original)`: ticks consumed from the original charge
  (`original.saturating_sub(remaining)`). Turns a cooldown into a forward
  progress counter for UI fill-bars without storing the original value twice.
- `timer::TimerQueue::peek_next()`: the minimum `remaining` ticks across all
  scheduled entries, or `None` if empty. Answers "when does the next event fire?"
  for UI countdown labels and headless simulation fast-forward.
- `timer::TimerQueue::count_repeating()`: count entries added via
  `schedule_repeat`. Useful for diagnostics and save-file inspection that need
  to distinguish persistent periodic timers from one-shot callbacks.
- `entity::EntityAllocator::live_entities()`: returns all currently allocated
  (not freed) entities as a `Vec<Entity>` in ascending index order. Essential
  for serialisation, debug overlays, and end-of-frame cleanup passes that need
  to enumerate every spawned entity without iterating a component store.
- `fixed::Fixed::floor()`: round toward negative infinity. Implemented with
  arithmetic-shift-right then shift-left — no branches, no overflow.
- `fixed::Fixed::ceil()`: round toward positive infinity. No-op on integers;
  adds one unit then floors otherwise.
- `fixed::Fixed::round()`: round to nearest integer, ties round away from
  negative infinity (0.5 → 1, -0.5 → 0).
- `fixed::Fixed::fract()`: fractional part `self - self.floor()`. Always in
  `[0, 1)` — follows the floor convention rather than IEEE 754's sign-preserving
  convention.
- `replay::Divergence`: implements `Display` — formats as "replay divergence at
  tick N: expected 0x…, got 0x…". Allows desync reports to be printed directly
  in logs and error messages without manual formatting.
- `geometry::rect(x, y, w, h)`: all cells in the rectangle `[x, x+w) × [y, y+h)`
  in row-major order — returns empty for non-positive dimensions. The natural
  primitive for filling rectangular map regions and iterating tile areas without
  a manual nested loop. Mirrors `bracket-lib`'s `Rect::for_each`/`point_set`
  patterns.
- `geometry::ring_annulus(cx, cy, inner_r, outer_r)`: all integer-coordinate points
  whose squared Euclidean distance from `(cx, cy)` falls in `[inner_r², outer_r²]`.
  A negative `inner_r` is treated as 0 (filled circle). Returns empty if
  `outer_r ≤ 0` or `inner_r ≥ outer_r`. Useful for area-of-effect rings, blast
  radii, and torchlight annuli without float sqrt.
- `wfc::WfcRules::get_allowed(tile, dir)`: read back the adjacency bitmask for a
  tile+direction pair — the symmetric counterpart of `allow`/`disallow`. Returns `0`
  for out-of-range arguments. Enables serialising rule sets and writing assertions
  on rule construction without re-exposing the internal `adj` array.
- `wfc::WfcGrid::count_tiles(tile)`: count collapsed cells whose value equals
  `tile`. Useful for post-collapse assertions and debugging ("how many floor tiles
  were generated?").
- `wfc::WfcGrid::to_vec()`: export the grid as `Vec<Option<u8>>` in row-major order.
  `Some(t)` for cells collapsed to tile `t`; `None` for cells still in superposition
  or contradiction. The ergonomic bridge between the WFC representation and downstream
  `TileMap`/renderer code.
- `fsm::Fsm::remove_transition(from, event)`: remove a specific `(from, event)`
  entry from the transition table. No-op if absent. Enables runtime AI behaviour
  modification without rebuilding the entire FSM (e.g. temporarily strip all
  "FleeFromPlayer" transitions to create a berserk state).
- `fsm::Fsm::clear_transitions()`: remove all transitions, leaving the current
  state unchanged. Simplifies the "stunned / frozen" pattern where every event
  must be a self-loop until a recovery signal fires.
- `menu::Menu::select_by_label(label)`: find the first item with a matching
  label, move the cursor there, and return its value (or `None` if not found or
  disabled). The "click by name" primitive for script-driven menus and tests.
- `menu::Menu::next_enabled()`: return the index of the next enabled item after
  the current cursor (wrapping), without moving the cursor. Useful for "preview
  next selection" UI indicators and accessible navigation lookaheads.
- `textlayout::measure_lines(lines)`: return `(max_width, line_count)` for a
  slice of `&str` lines in one allocation-free pass. Allows layout engines to
  size containers before rendering.
- `textlayout::pad_lines(lines, width)`: pad every line to `width` columns using
  `pad_right`, returning a uniform-width block. The standard step before
  rendering a bordered text panel where all content rows must be the same width.
- `hud::HudPanel::merge(panels)`: compute the smallest enclosing `HudPanel` for
  a slice of panels. Returns `None` for an empty slice. Useful for computing
  composite HUD region bounding boxes in layout code.
- `influence::InfluenceMap::fill(value)`: set every cell to `value`. The
  inverse of `clear()` — used to establish a non-zero baseline (e.g. neutral
  territory, ambient light level) before layering sources on top.
- `influence::InfluenceMap::find_peaks(threshold)`: return `(x, y)` coordinates
  of all cells whose value is ≥ `threshold`, in row-major order. The standard
  "gather candidate targets / spawn points" AI query without a manual iteration.
- `change::ChangeTracker::delta_since(last_tick)`: ticks elapsed since
  `last_tick` (`current − last_tick`, saturating). Eliminates the repetitive
  subtraction at call sites — the canonical "has N ticks passed since last
  action?" check for cooldown logic and idle-trigger patterns.
- `cmdqueue::CmdQueue::prepend(cmd)`: insert a command at the front of the
  queue (LIFO priority). For "abort / interrupt" commands that must be processed
  before already-queued actions.
- `cmdqueue::CmdQueue::retain(pred)`: keep only commands for which `pred`
  returns `true`, discard the rest in-place. The complement of `drain_if` — use
  when you want to filter without taking ownership of discarded items.
- `inputbuf::InputBuffer::reset_hold(key)`: reset the hold counter for a key to
  `0` without releasing it. The key stays pressed but the repeat timer restarts,
  so the next fire will re-trigger the initial press. For interrupt patterns such
  as "player jumps mid-repeat — pause movement until the key is re-evaluated."
- `spatial_hash::SpatialHash::contains(key, x, y)`: targeted membership test —
  `true` if `key` is registered in the cell that contains world point `(x, y)`.
  O(k) where k is the cell occupancy (usually very small). Avoids the
  `query_cell(x,y).contains(&key)` pattern that unpacks a slice unnecessarily.
- `spatial_hash::SpatialHash::density(x, y)`: entity count in the cell
  containing `(x, y)`. Returns `0` for absent cells. Allocation-free crowding
  heuristic used by mob AI and spawn systems to avoid over-populated cells.
- `spatial_hash::SpatialHash::iter_cells()`: iterate all non-empty cells as
  `(cell_coord, &[K])` pairs. Enables bulk serialisation and rendering passes
  that want to enumerate every occupied cell without repeated spatial queries.
- `savefile::SaveHeader::new(version)`: `const fn` constructor for `SaveHeader`.
  Removes `SaveHeader { version }` struct-literal boilerplate at call sites and
  enables `const` save-header constants in user code.
- `savefile::load_bytes_owned(data)`: like `load_bytes` but returns an owned
  `Vec<u8>` payload. Useful when the caller cannot hold a reference to the raw
  buffer long enough, or when the payload must outlive the source slice.
  Re-exported at the crate root.
- `noise::value_noise_1d_wrap(x, seed, period)`: 1-D value noise that tiles
  seamlessly at integer period `period`. Completes the wrap family alongside
  `value_noise_2d_wrap` — the missing primitive for seamless 1-D terrain height
  profiles, audio ramps, and scrolling textures.
- `noise::fbm_1d_wrap(x, seed, octaves, period)`: tileable 1-D FBM, mirroring
  `fbm_2d_wrap`. Each octave tiles at `period << octave_index` so all harmonics
  also tile. Returns `[0, 65535]`; `octaves == 0` returns `0`. Re-exported at
  the crate root alongside the other noise functions.
- `terminal::Screen::draw_line(from, to, glyph, fg, bg)`: draw a Bresenham line
  from `from` to `to` (both endpoints inclusive), setting every visited cell to
  `glyph`/`fg`/`bg`. Out-of-bounds cells are silently clipped via the existing `set`
  contract. Delegates to `geometry::line` for the coordinate sequence — the standard
  primitive for laser beams, aiming cursors, and debug overlays.
- `aabb::Aabb::clamp_point(px, py)`: return the nearest point inside the AABB to
  `(px, py)`. Points already inside are returned unchanged; points outside are
  clamped to the boundary. An empty box returns its origin `(x, y)`. The standard
  primitive for confining entity positions to a region without overlap checks.
- `rng::SplitMix64::reseed(seed)`: replace the generator state with `seed`,
  restarting the stream from that point. Equivalent to constructing a new
  `SplitMix64::new(seed)` but in-place — the canonical "restart simulation from
  a new random seed" operation without reallocating the RNG struct.
- `rng::SplitMix64::next_bool()`: return a uniformly random `bool` (50/50
  coin flip) via `coin(1, 2)`. Consumes one draw so the stream advances
  deterministically; mirrors the `coin` method but for callers who want a direct
  boolean rather than a conditional.
- `turn::Scheduler::speed(id)`: return the current speed for actor `id`, or
  `None` if not registered. The read-only counterpart to `set_speed` — useful
  for status displays ("Actor X has speed 150") and save-file serialisation.
- `turn::Scheduler::iter_actors()`: iterate all registered actor ids in insertion
  order. Does not drive the turn queue. Used for serialisation, debug overlays,
  and tests that need to enumerate every actor independently of turn order.
- `profiler::Profiler::section_count(section)`: number of `record()` calls for
  `section` in the current (unflushed) tick. Returns `0` for an unknown or silent
  section. Complements `this_tick` with a call-frequency view — useful for
  detecting runaway per-frame invocations ("pathfinding was called 47 times this
  tick").
- `profiler::Profiler::min(section)`: all-time minimum single-call elapsed for
  `section`. Persists across `begin_tick` calls, mirroring `peak`. Returns `0` for
  an unknown section. Enables displaying the best-case timing alongside `peak` in
  profiler overlays and automated regression checks ("min pathfinding latency
  regressed above baseline").
- `dice::Dice::roll_n_keep_highest(n, keep, rng)`: roll `n` copies of the
  expression, sum the `keep` highest results (classic "4d6 drop lowest" ability-
  score mechanic). `keep` is clamped to `min(keep, n)`; `n == 0` or `keep == 0`
  returns the modifier without drawing. Consumes exactly `n` draws. Sum saturates.
- `relations::Relations::path_to_root(entity)`: return the full ancestor chain
  from `entity` to its root, inclusive — `[entity, parent, grandparent, …, root]`.
  Single-element for root entities. Useful for debug visualisation, access-control
  checks, and any system that needs to walk the ownership hierarchy.
- `relations::Relations::find_common_ancestor(e1, e2)`: lowest common ancestor of
  two entities, or `None` if they share no ancestor (disjoint trees). Builds
  `path_to_root(e1)` as the search set, then walks `e2`'s chain. Essential for
  evaluating whether two entities are in the same "inventory tree" or
  ownership group.
- `passability::PassabilityGrid::set_region(x1, y1, x2, y2, blocked)`: bulk-set
  all cells in the axis-aligned rectangle `[x1, x2] × [y1, y2]` (both endpoints
  inclusive, coordinates in any order). Out-of-bounds cells are silently clipped.
  Equivalent to calling `set_blocked` in a nested loop but O(area) without
  allocation — the standard primitive for sealing doors, opening corridors, and
  placing rectangular obstacles.
- `entity::EntityAllocator::batch_free(entities)`: free multiple entities in one
  call. Stale and duplicate entries are silently ignored (same contract as `free`).
  Enables frame-end bulk cleanup ("free all dead entities this tick") without an
  explicit loop at each call site.
- `multimap::MultiMap::find_connector_to(dest_floor)`: return the first
  `Connector` whose `to_floor` matches `dest_floor`, or `None`. The "find the
  staircase that returns to floor N" navigation primitive — avoids scanning
  `exits_from` on every floor when the caller only knows the destination.
- `status::StatusSet::max_remaining()`: the longest remaining duration across
  all active effects (0 when empty). Complements `total_magnitude` with a
  duration-based aggregate — useful for "how long before all buffs wear off?" UI.
- `status::StatusSet::first_expiring()`: the key and remaining ticks of the
  effect that expires soonest (`None` when empty). Predictive UI primitive for
  "next status change in N ticks" indicators and AI decisions.
- `inventory::Inventory::count_where(pred)`: count occupied slots for which
  `pred` returns `true`. Avoids `.iter().filter().count()` boilerplate for
  common queries like "how many potions?", "how many identified items?".
- `inventory::Inventory::first_empty_slot()`: index of the first unoccupied slot,
  or `None` if full. Non-consuming (read-only). Used for "show next free slot"
  UI indicators and pre-flight add-item checks.
- `msglog::MsgLog::contains(needle)`: `true` if any stored message contains
  `needle` as a substring. One-liner for the common "has the player seen X yet?"
  check without an explicit `iter().any()` chain.
- `msglog::MsgLog::count_where(pred)`: count messages matching a predicate.
  Useful for "how many combat hits this session?" counters and test assertions
  on log content.
- `keymap::KeyMap::get_keys_for_action(action)`: all keys currently bound to
  `action` in insertion order (empty `Vec` if none). The reverse-lookup
  complement of `get` — required for "which key opens inventory?" UI tooltips
  and conflict-detection during control remapping.
- `keymap::KeyMap::swap_bindings(key1, key2)`: atomically exchange the actions
  at two keys. If only one is bound the action migrates to the unbound key; if
  neither is bound it is a no-op. The canonical one-call rebind primitive.
- `change::Changed::was_written_at(tick)`: `true` iff `changed_at == tick` —
  exact-match check stricter than `is_changed_since`, for "did this change THIS
  frame?" patterns without risk of matching earlier unprocessed ticks.
- `change::Changed::ticks_since_change(current_tick)`: saturating elapsed ticks
  since the last `mark` call (`current_tick − changed_at`). Per-component age
  query without passing `ChangeTracker` everywhere — use for cache invalidation,
  freshness indicators, and idle-trigger conditions.
- `combat::Stats::hp_percent()`: current HP as an integer percentage `[0, 100]`.
  Uses `u64` intermediate arithmetic to avoid overflow for large HP values; clamps
  at 100; returns 0 for dead or zero-`max_hp` actors. Replaces the common
  `(hp * 100 / max_hp)` one-liner which overflows for `hp > i32::MAX / 100`.
- `combat::Stats::take_overkill_damage(amount)`: apply `amount` damage and return
  how many hit points overshot zero (overkill). Negative amounts are treated as
  zero. Useful for chaining "excess damage propagates" mechanics (e.g. shield →
  HP spillover, cleave damage to adjacent targets, one-hit-kill detection).
- `camera::Camera::contains_rect(l, t, r, b)`: test whether the world-space
  axis-aligned rectangle `[l, r) × [t, b)` (exclusive right/bottom) overlaps the
  camera's current viewport. Empty rects (`l >= r` or `t >= b`) always return
  `false`. Standard broad-phase visibility cull for particle systems, light
  sources, and HUD markers — cheaper than projecting each corner with
  `world_to_screen`.
- `camera::Camera::chebyshev_to_center(wx, wy)`: Chebyshev ("king-moves") distance
  from the viewport's world-space centre to `(wx, wy)`. `max(|cx−wx|, |cy−wy|)` —
  the natural metric for 8-directional grids. Returns `0` when the point equals
  the centre. Useful for depth-of-field / fog-of-war intensity calculations and
  radial scroll-speed curves without branching on octant.
- `sparse_set::SparseSet::count_matching(pred)`: count entries for which
  `pred(&value)` returns `true`. Non-allocating (no intermediate `Vec`);
  equivalent to `.values().filter(pred).count()` but named for clarity at call
  sites ("how many poisoned entities?", "how many full-HP enemies?").
- `timer::TimerQueue::reschedule(pred, new_delay)`: cancel the first pending entry
  matching `pred` and immediately re-schedule its event as a one-shot at
  `new_delay` ticks. Returns `true` if an entry was found. The rescheduled entry
  is always one-shot regardless of whether the original was a repeating timer —
  for "reset patrol timer without losing the event" patterns.
- `easing::ease_in_expo` / `ease_out_expo` / `ease_in_out_expo`: exponential
  easing family — the last major Penner group. Implements `2^(10·(t−1))` via
  integer bit-shift for the power-of-two component and a 3-term Taylor series
  for the fractional part (max error ≈ 0.4%). Exact 0 at t=0, exact 1 at t=1.
  Re-exported at the crate root alongside the other easing functions.
- `geometry::rect_perimeter(x, y, w, h)`: cells on the outer border of the
  rectangle `[x, x+w) × [y, y+h)` in row-major order. Degenerates to `rect` for
  1-wide or 1-tall inputs. Cell count is `2*(w+h-2)` for w,h ≥ 2. Fills the gap
  between `rect` (solid fill) and `ring_annulus` (Euclidean ring): the
  axis-aligned rectangular outline needed for dungeon-room borders and UI panels.
- `geometry::diamond(cx, cy, r)`: all cells at exactly Manhattan distance `r` from
  `(cx, cy)` — the "4-directional blast ring". Returns empty for `r < 0`, just
  the centre for `r == 0`, and `4·r` cells (in ascending x,y order) otherwise.
  Complements `circle` (Bresenham outline) and `ring_annulus` (Euclidean annulus)
  with the Manhattan-metric ring needed for 4-way movement range queries.
- `noise::normalize_noise(v, lo, hi)`: map a noise value from the standard
  `[0, 65535]` output range to any integer range `[lo, hi]`. Returns `lo` for
  degenerate ranges (`lo >= hi`), saturates at `hi` for `v == 65535`. Eliminates
  the repetitive `lo + v * range / 65535` one-liners at call sites.
- `lib.rs`: re-export `ease_in_back`, `ease_out_back`, `ease_in_out_back`,
  `ease_in_bounce`, `ease_out_bounce`, `ease_in_out_bounce` at the crate root
  (these were implemented in easing.rs but not yet re-exported).
- `arch::ArchTable::swap_rows(entity1, entity2) -> bool`: exchange the row data
  of two entities in-place (O(1) — two indexed moves, no search). Returns `true`
  when both entities are present; `false` if either is absent (no partial swap).
  Useful for sort-swap passes and "trade equipment" mechanics without a temporary
  staging variable.
- `wfc::WfcRules::allowed_count(tile, dir) -> usize`: count of permitted
  neighbour tiles for `(tile, dir)` — the popcount of the adjacency bitmask.
  The cheapest entropy proxy before running full WFC: the lower the count, the
  more constrained the slot. Returns `0` for out-of-range arguments.
- `easing::ease_smoothstep(t) -> Fixed`: classic cubic Hermite smoothstep
  `3t² − 2t³`. Zero first derivative at both endpoints; monotonically increasing
  in `[0, 1]`. Equivalent to the GLSL `smoothstep` shape without branching or
  float. Useful for camera lerp, fade-in/out, and any curve where a gentle
  ease-in/ease-out is needed without Penner's asymmetry.
- `camera::Camera::follow(wx, wy, margin, world_w, world_h)`: lazy camera follow
  — pan the minimum distance needed to keep world point `(wx, wy)` within the
  inner rectangle that is `margin` cells inside all four viewport edges. No-op
  when the target is already inside the inner rect. Pans via the existing
  `pan(dx, dy, …)` so world-boundary clamping is automatic. The standard
  "keep the player on screen" primitive for roguelikes without snap-to-center.
- `relations::Relations::subtree_size(entity) -> usize`: count of all entities
  in the subtree rooted at `entity` (inclusive). Equivalent to
  `1 + descendants_of(entity).len()`. Single-node (leaf/root) returns `1`;
  useful for inventory-weight and hierarchy-depth budgets.
- `loader::Stats::set(key, value)`: insert a new stat or overwrite the value of
  an existing one. Preserves all other entries. The runtime "apply buff" primitive
  — lets systems write stat changes back without remove+re-insert. Complements
  the read-only `get` / `iter` API.
- `world_hash::Fnv1a::write_i64(value)` and `impl DetHash for i64`: extends the
  integer-write family (`write_u32`, `write_u64`, `write_i32`) with signed 64-bit
  support. Useful for hashing large tick counters, cumulative damage totals, or
  other wide simulation state fields. `write_i64` folds the 8 little-endian bytes
  identically across targets; `DetHash for i64` delegates to it.
- `influence::InfluenceMap::clamp_cells(min, max)`: clamp every cell value to
  `[min, max]` in-place. Prevents influence values from growing unbounded when
  many sources overlap, and implements "floor of zero" for threat maps that must
  not go negative. O(n) in map size; no allocation.
- `spatial_hash::SpatialHash::query_rect_count(x, y, w, h) -> usize`: count of
  keys in any cell overlapping the rectangle without allocating a `Vec` — the
  allocation-free complement to `query_rect(…).len()`. Returns `0` for
  non-positive `w` or `h`. Suitable for hot broad-phase budget checks.
- `passability::PassabilityGrid::fill(blocked)`: set every cell to `blocked`
  without reallocation. The single-call "seal entire map" / "clear all walls"
  primitive before carving a new layout. O(n) in map size.
- `cmdqueue::CmdQueue::index(i) -> Option<&C>`: read the command at position
  `i` (0 = oldest) without consuming it. The random-access complement to `peek()`;
  equivalent to `peek().get(i)`. Does not advance the queue.
- `random_table::RandomTable::set_weight(idx, weight)`: update the weight of
  the entry at `idx`; adjusts `total_weight` to stay consistent. Setting to `0`
  makes an entry permanently unselectable ("sold out") without removing it from
  the table. Silently ignores out-of-range indices.
- `replay::find_all_divergences(expected, actual) -> Vec<Divergence>`: collect
  every diverging tick into a `Vec`, unlike `first_divergence` (stops at first)
  or `count_divergences` (count only). Ticks beyond the shorter trace count as
  divergences. Re-exported at the crate root.
- `profiler::EventLog::last() -> Option<&LogEntry<E>>`: the most recently pushed
  entry, or `None` for an empty log. Equivalent to `recent(1).last()` but avoids
  constructing an iterator. The "show last event" status-line primitive.
- `autotile::count_set_bits(mask: u8) -> u32`: popcount of an 8-bit auto-tile
  neighbour mask — `0` = isolated, `8` = fully surrounded interior. Delegates to
  `u8::count_ones`. Useful for terrain classification without decoding individual
  bit directions.
- `assets::AssetStore::find_all_by<F>(pred) -> Vec<AssetHandle<T>>`: collect
  handles of all live assets matching `pred`, in ascending index order. The plural
  complement to `find_by` — use when multiple assets can satisfy a condition.
  Returns an empty `Vec` when the store is empty or no assets match.
- `hud::BarWidget::empty_cells() -> u32`: unfilled cell count (`width −
  filled_cells()`). The complement of `filled_cells` — returns the number of
  blank segments in a health/mana bar without a manual subtraction at every
  call site.
- `diag_json::diag_count(diags) -> (usize, usize)`: count `(errors, warnings)`
  in a diagnostic slice in one pass. Avoids two separate `filter().count()` calls
  when both totals are needed (e.g. CI gate: "fail if errors > 0, warn if
  warnings > 0").
- `terminal::Screen::draw_double_box(x, y, w, h, fg, bg)`: draws a double-line
  box border using Unicode box-drawing characters `╔╗╚╝═║`. Interior untouched;
  out-of-bounds positions silently clipped via `set`. The "dialogue / title"
  variant of `draw_box` — standard in roguelike UIs for emphasising important
  panels.
- `vec::Vec2::from_angle(angle: Fixed) -> Vec2`: construct a unit vector from a
  heading angle (radians as Q16.16 `Fixed`). Delegates to `Fixed::sin_cos`
  (CORDIC) — `(x: cos, y: sin)`. Float-free and deterministic. The inverse of
  `angle()`, enabling angle→vector round-trips for steering, projectile launch,
  and FOV cone calculations.
- `noise::hash_3d(x, y, z, seed) -> u32`: deterministic 3-D integer hash —
  folds `z` into the seed with a mixing multiply, then delegates to `hash_2d`.
  Consistent with `hash_2d` in distribution; distinct from it for any non-zero
  `z`. Useful for voxel terrain, layered dungeon generation, and 3-D particle
  systems.
- `geometry::chebyshev_ring(cx, cy, r) -> Vec<(i32, i32)>`: all cells at
  exactly Chebyshev distance `r` from `(cx, cy)` — the rectangular perimeter
  `rect_perimeter(cx-r, cy-r, 2r+1, 2r+1)`. Returns empty for `r < 0`, the
  single centre cell for `r == 0`. Complements `diamond` (Manhattan ring) with
  the 8-directional movement ring needed for king-move range queries and aura
  effects.
- `timestep::FixedTimestep::accumulator_ns() -> u64`: current sub-step time
  buffered in the accumulator (always in `[0, step_ns)`). Exposes the value
  normally hidden inside the struct so callers can serialise it alongside
  `total_steps` for an exact save/restore of the timestep state — or verify
  that two replay runs are at identical accumulator positions.
- `aabb::Aabb::expand_to_include(px, py) -> Aabb`: grow the box to include
  point `(px, py)`. Points already inside return `self` unchanged; an empty
  `Aabb` becomes a 1×1 box at the point. Saturating arithmetic; never creates
  an invalid AABB. Useful for computing aggregate bounding boxes over a stream
  of points without pre-collecting them.
- `fixed::Fixed::is_zero() -> bool`, `is_positive() -> bool`,
  `is_negative() -> bool`: inline sign-predicate helpers. Eliminate the
  `.raw() == 0` / `.raw() > 0` / `.raw() < 0` patterns at call sites —
  especially useful in guard clauses and branch predicates where the
  intent ("the velocity is positive") is clearer than the raw comparison.
- `rng::SplitMix64::with_state(state: u64) -> Self`: construct a generator
  from a raw state snapshot — the inverse of `state()`. Semantically
  distinct from `new(seed)`: restoring an exact stream position rather than
  seeding from a game-world integer. The canonical "load RNG from save file"
  constructor.
- `pathfinding::path_cost(path: &[(i32, i32)]) -> i32`: total octile cost of
  a path — sums `octile_distance` for each consecutive step pair. An empty
  or single-cell path has cost 0. Matches the A* internal scale (10 ortho,
  14 diag), so the result is directly comparable to A* `g`-scores and
  heuristic estimates.
- `tilemap::TileMap::find_first<P>(pred) -> Option<(i32, i32)>`: first
  matching cell in row-major order, or `None`. The single-result complement
  of `find_all` — avoids allocating a full `Vec` when only the first match
  is needed (e.g. "where is the exit?").
- `combat::apply_resistance(damage: i32, resist_percent: u32) -> i32`:
  reduce damage by a percentage resistance (`max(0, damage*(100−r)/100)`),
  clamping `resist_percent` to `[0, 100]`. Negative damage returns 0.
  Complements `base_damage` (subtraction model) with a percentage-based
  defensive layer common in action RPGs and card games.
- `mapgen::Dungeon::floor_cells() -> Vec<(i32, i32)>`: all floor cells in
  row-major order. The primary convenience primitive for spawn placement when
  the full list is needed at once — avoids a manual `is_floor` scan at every
  call site.
- `easing`: elastic family — `ease_in_elastic`, `ease_out_elastic`,
  `ease_in_out_elastic`. Penner elastic curves: the in-variant oscillates
  below 0 near `t=0` (spring pull-back), the out-variant overshoots above
  1 near `t=1` (spring finish). Computed with the existing CORDIC `sin` and
  Taylor-series `exp2` (same primitives as the expo family). Float-free and
  deterministic. Brings total easing coverage to 21 functions.
- `fov::fov_count(origin, radius, is_opaque) -> usize`: count visible cells
  without allocating a `Vec`. Equivalent to `fov_to_vec(...).len()` but
  skips the intermediate allocation. Use for broad-phase lighting-budget
  queries and per-frame FOV coverage checks.

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
- `fixed::Fixed::to_int_round()`: round to nearest integer and return as `i32`.
  Combines `round()` + `to_int_trunc()` in a single call — the natural counterpart
  to the existing `to_int_trunc()` for callers that want normal rounding semantics.
- `fixed::Fixed::min(other)` / `max(other)`: component-wise saturating minimum and
  maximum. Deterministic ordering — same as `Ord`-based comparison on the raw `i32`
  representation. The missing scalar clamp primitives; complement `Vec2::min`/`max`
  with the same API on the underlying scalar type.
- `fixed::Fixed::recip()`: multiplicative reciprocal `1 / self`. Implemented as
  `Fixed::ONE.div(self)` — inherits the existing saturating-division contract
  (division by zero returns `Fixed::MAX` or `Fixed::MIN` by sign). Useful for
  computing inverse speeds and normalisation denominators without a double divide.
- `rng::SplitMix64::skip(n)`: advance the stream by `n` draws without returning
  values. Useful for fast-forwarding a seeded replay to a known position, or
  skipping over the draws used by a sub-system without branching on the draw
  count. Consumes exactly `n` draws; `skip(0)` is a no-op.
- `turn::Scheduler::actors_ready()`: returns all actor IDs whose banked energy is
  ≥ `ACTION_COST` — i.e. every actor that would be returned by the next
  `next_actor()` call if ties were broken differently. Does not advance the queue.
  Useful for "who can act this tick?" batch queries in parallel-resolution
  turn modes and test assertions.
- `turn::Scheduler::reset_actor(id)`: set the banked energy for `id` to `0`,
  effectively spending a full turn's worth of energy. The canonical "this actor
  just acted — deduct energy" primitive. No-op for unknown actors. Mirrors the
  `set_energy` contract but named for clarity at call sites.
- `pathfinding::step_toward(from, goal, is_blocked)`: single-step A* helper —
  returns the first waypoint on the shortest path from `from` toward `goal`, or
  `None` if `from == goal` or no path exists. The canonical "monster chases player"
  one-liner that wraps `astar` and plucks `path[1]`. `is_blocked` must return
  `true` for out-of-bounds coordinates to bound the search. Re-exported at the
  crate root as `izanagi_kit::step_toward`.
- `inventory::Inventory::is_full()`: shorthand for `!has_space()` — `true` when
  every slot is occupied. Eliminates the double negation at call sites where the
  positive "full?" condition is clearer than "not has_space?".
- `status::StatusSet::extend_duration(key, added_ticks)`: add ticks to the
  remaining duration of an active effect. Saturating. No-op when the effect is
  not active. The "haste doubles buff duration" primitive without requiring a
  `remove` + `apply` pair that would lose the current magnitude.
- `status::StatusSet::count_with(pred)`: count effects for which
  `pred(key, &effect)` is `true`. Allocation-free alternative to
  `iter().filter().count()` — the natural query for "how many buffs active?",
  "how many debuffs with magnitude ≤ −5?".
- `tilemap::TileMap::find_all(pred)`: collect `(x, y)` of all cells for which
  `pred(&tile)` returns `true`, in row-major order. Closes the gap between
  `count_where` (count only) and `iter()` (full iteration with manual collect).
  Mirrors `bracket-lib`'s `TileMap::find_all` semantics.
- `assets::AssetStore::clear()`: remove all assets in one call, bumping every
  occupied slot's generation so all existing handles are permanently invalidated.
  Repopulates the free list with all indices so subsequent inserts reuse slots.
  O(n) — implements the "flush the asset cache" pattern without a loop over
  individual `remove` calls.
- `assets::AssetStore::find_by(pred)`: return the handle of the first live asset
  for which `pred(&asset)` is `true` (ascending index order), or `None`. The
  standard "where is my player sprite?" asset-manager lookup without requiring
  callers to maintain their own reverse index.
- `aabb::Aabb::corners()`: return all four corners in clockwise order from
  top-left — `[top-left, top-right, bottom-right, bottom-left]` — as
  inclusive-coordinate `(i32, i32)` pairs. Empty boxes return all four equal to
  `(x, y)`. Useful for raycasting, decal placement, and bounding-polygon tests.
- `aabb::Aabb::distance_to_point(px, py)`: Chebyshev distance from `(px, py)` to
  the nearest cell on (or inside) the box. Returns `0` for points inside or for
  empty boxes. The standard "how far is this entity from the room?" query for
  aggro radius, fog-of-war, and attraction-field seeding.
- `menu::Menu::prev_enabled()`: symmetric counterpart to `next_enabled()` —
  return the index of the previous enabled item (wrapping backwards), or `None`
  when all items are disabled. Does not move the cursor. The missing reverse-
  navigation lookahead for "show previous selectable" UI indicators and
  accessible backward-traversal patterns.
- `menu::Menu::count_enabled()`: number of selectable (non-disabled) items.
  Allocation-free alternative to `.iter().filter(|(_, it)| !it.disabled).count()`.
  Useful for "are any items available?" checks and progress indicators ("3 of 5
  abilities unlocked").
- `inputbuf::InputBuffer::release_all()`: release all currently held keys at
  once — semantic alias for `clear()`, named for focus-loss handlers where the
  intent is "simulate a key-up for every pressed key." Ensures no held state
  survives a window-blur or scene transition.
- `fsm::Fsm::peek_next(event)`: returns `Option<&S>` — the state this FSM would
  transition to if `event` were fired now, without actually changing state.
  Returns `None` for unmapped events (self-loop). Useful for AI lookahead ("would
  attacking now trigger Dead?") and "show next state" UI tooltips without
  committing the transition.
- `change::ChangeTracker::set_tick(tick)`: set the tick counter to an explicit
  value. The companion to `reset()` (which always resets to `0`): needed when
  restoring exact simulation state from a save file to preserve the tick offsets
  recorded in all `Changed<T>` components.
- `textlayout::justify(s, width)`: typographic full-justification — distribute
  spaces between words so the line fills exactly `width` columns. Extra spaces
  (when `total_spaces % gaps != 0`) are placed left-to-right. Single-word lines
  and lines wider than `width` fall back to `pad_right`. Re-exported at the crate
  root as `izanagi_kit::justify`.
- `cmdqueue::CmdQueue::pop_front()`: remove and return the front (oldest /
  first-in) command, or `None` if empty. O(n) — shifts remaining elements. For
  the "process exactly one command per tick" rate-limiting pattern without
  draining the entire queue.
- `cmdqueue::CmdQueue::pop_back()`: remove and return the back (newest /
  last-in) command, or `None` if empty. O(1). For LIFO / "cancel last queued
  command" patterns.
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
