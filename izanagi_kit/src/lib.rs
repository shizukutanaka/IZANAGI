//! `izanagi_kit` — zero-dependency reference modules extracted from a design
//! review of the IZANAGI engine (v4.4.0). Each module is a drop-in candidate
//! addressing a P1 improvement:
//!
//! - [`entity`] / [`sparse_set`] — sparse-set ECS storage with generational
//!   handles (cheap composition changes, O(1) lookup).
//! - [`fixed`] — Q16.16 fixed-point for cross-platform-deterministic math.
//! - [`fov`] — symmetric shadowcasting field-of-view (integer, deterministic).
//! - [`geometry`] — integer Bresenham line drawing and line-of-sight.
//! - [`mapgen`] — seed-driven procedural dungeon generation (rooms, cellular caves, BSP, drunkard's-walk; deterministic).
//! - [`pathfinding`] — deterministic 8-way A*, weighted A* (ε-admissible), Jump Point Search, Dijkstra maps + rescanned flee/safety maps, auto-explore.
//! - [`replay`] — replay trace recording, desync detection and rollback.
//! - [`netinput`] — deterministic multi-player input prediction and misprediction detection (`NetInputBuffer<P,I>`).
//! - [`rng`] — SplitMix64 seeded PRNG (replay-safe randomness) with named independent sub-streams (`SplitMix64::split`).
//! - [`rng_xoshiro`] — opt-in xoshiro256++ PRNG (`Xoshiro256pp`): 2²⁵⁶ period, higher statistical quality, seeded from SplitMix64, with `jump()` for parallel streams.
//! - [`msglog`] — bounded ring-buffer message log with `DetHash`.
//! - [`terminal`] — headless cell screen buffer with 24-bit ANSI output.
//! - [`timestep`] — fixed-timestep accumulator with death-spiral guard.
//! - [`timer`] — tick-based `Cooldown` and `TimerQueue<E>` for delayed events.
//! - [`turn`] — energy/speed-based turn scheduler with non-destructive turn-order forecast.
//! - [`mod@vec`] — fixed-point Vec2/Vec3 (dot/cross/len/normalize/scale/DetHash).
//! - [`shufflebag`] — draw-without-replacement bag randomizer with auto-refill (`ShuffleBag<T>`).
//! - [`equipment`] — worn-item loadout per body slot with aggregate `StatsModifier` (`Equipment<T>`, `EquipSlot`).
//! - [`progression`] — experience accumulation and integer level curves (`Progression`, `LevelCurve`).
//! - [`meta`] — cross-run meta-progression: permanent unlock flags and all-time best records (`MetaProgress<K,R>`).
//! - [`identify`] — scrambled per-seed item appearances revealed on demand (`Identification<T,L>`).
//! - [`lightmap`] — additive integer illumination map for torchlit dungeons (`LightMap`).
//! - [`faction`] — inter-faction reputation and alignment queries (`FactionMap<K>`).
//! - [`threat`] — per-combatant aggro / target-selection table (`ThreatTable<K>`).
//! - [`pool`] — bounded regenerating resource pool: mana/stamina/hunger (`Pool`).
//! - [`tween`] — time-driven eased value interpolation over a tick span (`Tween`, `TweenSequence`).
//! - [`wallet`] — fungible currency balances for shops/economy (`Wallet<C>`).
//! - [`shop`] — buy/sell price listings against a wallet-backed till (`Shop<K,C>`, `Listing`).
//! - [`dialogue`] — branching NPC conversation tree (`Dialogue`, `DialogueNode`, `Choice`).
//! - [`trigger`] — condition→action rule set for scripted game events (`TriggerSet<K,C,A>`, `Trigger<C,A>`).
//! - [`eventqueue`] — intra-tick FIFO game event queue (`EventQueue<E>`).
//! - [`quest`] — quest and objective tracking (`Quest`, `Objective`, `QuestState`).
//! - [`calendar`] — cyclical integer time-of-day / day-night cycle (`Calendar`).
//! - [`recipe`] — item crafting / recipe system (`Recipe<K,O>`, `Ingredient<K>`).
//! - [`visibility`] — tri-state fog-of-war / exploration memory (`VisibilityMap`, `Visibility`) layered on top of FOV.
//! - [`world_hash`] — FNV-1a per-frame state checksum for bit-exact replay, `hash_unordered` for permutation-invariant multiset hashing, and `LabeledDigest` for per-subsystem hash breakdowns that localize desyncs.
//! - [`camera`] — integer camera / viewport (world↔screen coordinate mapping).
//! - [`change`] — dirty-flag change detection (`Changed<T>`, `ChangeTracker`).
//! - [`combat`] — integer combat formula (stats, melee/ranged, hit roll).
//! - [`damage`] — typed damage (`DamageType`) and per-type resistance/vulnerability profiles (`ResistanceProfile`).
//! - [`encounter`] — procedural group-encounter rolling (`EncounterPack`: count ranges + appearance chances per slot).
//! - [`affix`] — procedural item affixes (`AffixGenerator`: weighted prefix/suffix pools → "Rusty Sword of Dragonslaying").
//! - [`fsm`] — table-driven finite state machine for game AI (`Fsm<S,E>`).
//! - [`inventory`] — slot-based inventory (`Inventory<T>`) for roguelike items.
//! - [`keymap`] — key-to-action mapping (`KeyMap<K,A>`) for deterministic input.
//! - [`easing`] — integer easing curves (quad/cubic in/out/in-out) over `Fixed`.
//! - [`status`] — timed status effects / buff-debuff tracking (`StatusSet<K>`).
//! - [`cmdqueue`] — deterministic command queue (replay-safe input abstraction).
//! - [`content`] / [`parser`] / [`serializer`] / [`validator`] / [`loader`] —
//!   the content pipeline: author game elements as text, serialize them back,
//!   validate them, load into the ECS.
//!
//! - [`ability`] — unified ability/skill system (`AbilitySet<K,E>`, `Ability<E>`, `AbilityResult`) with mana, cooldown, and range checks.
//! - [`behavior`] — hierarchical behavior trees for game AI (`BehaviorTree<A>`, `BehaviorNode<A>`, `BehaviorStatus`).
//! - [`aabb`] — axis-aligned bounding box (`Aabb`) collision detection.
//! - [`arch`] — archetype-based component storage (`ArchTable<Row>`) for cache-friendly multi-component iteration.
//! - [`menu`] — keyboard-navigable list menu (`Menu<T>`) for roguelike UI.
//! - [`textlayout`] — word-wrap, truncate, and alignment helpers for terminal UI.
//! - [`inputbuf`] — input buffer with hold/repeat detection (`InputBuffer<K>`).
//! - [`spatial_hash`] — spatial hash grid (`SpatialHash<K>`) for broad-phase queries.
//! - [`noise`] — deterministic integer value noise and hash functions.
//! - [`tilemap`] — multi-layer tile map (`TileMap<T>`, `LayeredMap<T>`).
//! - [`influence`] — grid-based influence map (`InfluenceMap`) for AI steering.
//! - [`relations`] — entity parent/child relationships (`Relations`).
//! - [`assets`] — typed asset handle store (`AssetStore<T>`, `AssetHandle<T>`).
//! - [`profiler`] — tick profiler (`Profiler`) and structured event log (`EventLog<E>`).
//! - [`hfsm`] — hierarchical FSM (`HFsm<S,E>`): parent states + wildcard transitions + `is_in` ancestry queries.
//! - [`hud`] — HUD primitives: fill bar (`BarWidget`), stat line, panel layout (`HudPanel`).
//! - [`autotile`] — bitmask auto-tiling (`compute_mask`, `SimpleTileTable`).
//! - [`diag_json`] — machine-readable diagnostic serialization: a bespoke JSON schema (`diag_json`) and industry-standard SARIF 2.1.0 (`diag_sarif`) for CI code-scanning integration.
//! - [`passability`] — grid-based passability / collision layer (`PassabilityGrid`).
//! - [`savefile`] — versioned binary save-file framing (`save_bytes`, `load_bytes`, `SaveHeader`).
//! - [`wfc`] — Wave Function Collapse procedural tile-map generation (`WfcRules`, `wfc_solve`, `WfcGrid`).
//! - [`multimap`] — multi-floor dungeon stack (`MultiMap`, `Connector`).
//!
//! All modules are `std`-only and contain no `unsafe`.

#![forbid(unsafe_code)]
// Publication-grade doc hygiene: every public item must be documented, and
// intra-doc links must resolve. Both are `deny` (not `warn`) — the crate is
// already clean, so this keeps it that way as a hard gate rather than a
// warning that can accumulate unnoticed.
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

pub mod aabb;
pub mod ability;
pub mod affix;
pub mod arch;
pub mod assets;
pub mod autotile;
pub mod behavior;
pub mod calendar;
pub mod camera;
pub mod change;
pub mod cmdqueue;
pub mod combat;
pub mod content;
pub mod damage;
pub mod diag_json;
pub mod dialogue;
pub mod dice;
pub mod easing;
pub mod encounter;
pub mod entity;
pub mod equipment;
pub mod eventqueue;
pub mod faction;
pub mod fixed;
pub mod fov;
pub mod fsm;
pub mod geometry;
pub mod hfsm;
pub mod hud;
pub mod identify;
pub mod influence;
pub mod inputbuf;
pub mod inventory;
pub mod keymap;
pub mod lightmap;
pub mod loader;
pub mod mapgen;
pub mod menu;
pub mod meta;
pub mod msglog;
pub mod multimap;
pub mod netinput;
pub mod noise;
pub mod parser;
pub mod passability;
pub mod pathfinding;
pub mod pool;
pub mod profiler;
pub mod progression;
pub mod quest;
pub mod random_table;
pub mod recipe;
pub mod relations;
pub mod replay;
pub mod rng;
pub mod rng_xoshiro;
pub mod savefile;
pub mod serializer;
pub mod shop;
pub mod shufflebag;
pub mod sparse_set;
pub mod spatial_hash;
pub mod status;
pub mod terminal;
pub mod textlayout;
pub mod threat;
pub mod tilemap;
pub mod timer;
pub mod timestep;
pub mod trigger;
pub mod turn;
pub mod tween;
pub mod validator;
pub mod vec;
pub mod visibility;
pub mod wallet;
pub mod wfc;
pub mod world_hash;

pub use aabb::Aabb;
pub use ability::{Ability, AbilityResult, AbilitySet};
pub use affix::{Affix, AffixGenerator, AffixSlot, AffixedItem};
pub use arch::ArchTable;
pub use assets::{AssetHandle, AssetStore};
pub use autotile::{compute_all, compute_mask, SimpleTileTable};
pub use behavior::{BehaviorNode, BehaviorStatus, BehaviorTree};
pub use calendar::Calendar;
pub use camera::Camera;
pub use change::{ChangeTracker, Changed};
pub use cmdqueue::CmdQueue;
pub use combat::{
    apply_resistance, base_damage, critical_strike, melee_attack, ranged_attack, roll_damage,
    roll_to_hit, splash_attack, Stats, StatsModifier, StrikeResult,
};
pub use content::{Content, Diagnostic, Prefab, Severity, Tile};
pub use damage::{DamageType, ResistanceProfile};
pub use diag_json::severity_filter;
pub use dialogue::{Choice, Dialogue, DialogueNode};
pub use dice::Dice;
pub use easing::{
    ease_in_back, ease_in_bounce, ease_in_circ, ease_in_cubic, ease_in_expo, ease_in_out_back,
    ease_in_out_bounce, ease_in_out_circ, ease_in_out_cubic, ease_in_out_expo, ease_in_out_quad,
    ease_in_out_quart, ease_in_out_quint, ease_in_out_sine, ease_in_quad, ease_in_quart,
    ease_in_quint, ease_in_sine, ease_out_back, ease_out_bounce, ease_out_circ, ease_out_cubic,
    ease_out_expo, ease_out_quad, ease_out_quart, ease_out_quint, ease_out_sine, ease_reversed,
    linear,
};
pub use encounter::{EncounterPack, EncounterSlot};
pub use entity::{Entity, EntityAllocator};
pub use equipment::{EquipSlot, Equipment};
pub use eventqueue::EventQueue;
pub use faction::{FactionMap, FRIENDLY_THRESHOLD, HOSTILE_THRESHOLD, MAX_REP, MIN_REP};
pub use fixed::Fixed;
pub use fov::{can_see, compute_fov, compute_fov_dist, fov_count_filtered, fov_to_vec};
pub use fsm::Fsm;
pub use geometry::{
    chebyshev_distance, cone, cone_visible, diamond, knockback, line, line_of_sight,
    manhattan_distance, ray_blocked_at, ray_cast, rect_contains, rect_perimeter, reflect_point,
    rotate_90_ccw, rotate_90_cw, vec_toward, Distance,
};
pub use hfsm::HFsm;
pub use hud::{BarWidget, HudPanel, StatLine};
pub use identify::Identification;
pub use influence::InfluenceMap;
pub use inputbuf::{InputBuffer, KeySource, ListKeySource};
pub use inventory::Inventory;
pub use keymap::KeyMap;
pub use lightmap::{LightMap, MAX_LIGHT};
pub use loader::{load_level, LoadedLevel, Position, Render};
pub use mapgen::{
    generate_bsp, generate_cave, generate_drunkard, generate_dungeon, BspParams, CaveParams,
    DrunkardParams, Dungeon, GenParams, Rect,
};
pub use menu::{Menu, MenuItem};
pub use meta::MetaProgress;
pub use msglog::MsgLog;
pub use multimap::{Connector, MultiMap};
pub use netinput::NetInputBuffer;
pub use noise::{
    fbm_1d, fbm_1d_wrap, fbm_2d, fbm_2d_in_range, fbm_2d_wrap, fbm_3d, hash_1d, hash_2d, hash_3d,
    noise_3d_in_range, normalize_noise, ridge_noise_2d, value_noise_1d, value_noise_1d_wrap,
    value_noise_2d, value_noise_2d_wrap, value_noise_3d,
};
pub use parser::{error_count, parse, warning_count};
pub use passability::PassabilityGrid;
pub use pathfinding::{
    astar, auto_explore, descend, dijkstra_map, flee_map, flood_fill, is_path_clear, is_reachable,
    jps, jps4, nearest_reachable, octile_distance, path_cost, path_to_direction_vec, smooth_path,
    step_toward, weighted_astar,
};
pub use pool::Pool;
pub use profiler::{EventLog, LogEntry, Profiler};
pub use progression::{LevelCurve, Progression};
pub use quest::{Objective, Quest, QuestState};
pub use random_table::RandomTable;
pub use recipe::{Ingredient, Recipe};
pub use relations::Relations;
pub use replay::{
    check_trace, count_divergences, first_divergence, first_divergence_labeled, record_trace,
    resimulate, Divergence, LabeledDivergence,
};
pub use rng::SplitMix64;
pub use rng_xoshiro::Xoshiro256pp;
pub use savefile::{
    estimate_save_size, load_bytes, load_bytes_migrated, load_bytes_owned, save_bytes,
    validate_integrity, LoadError, Migrator, SaveHeader,
};
pub use serializer::{content_eq, serialize};
pub use shop::{Listing, Shop};
pub use shufflebag::ShuffleBag;
pub use sparse_set::{join, join3, join3_mut, join_mut, SparseSet};
pub use spatial_hash::SpatialHash;
pub use status::{Effect, StatTarget, StatusSet};
pub use terminal::{Cell, Screen};
pub use textlayout::{
    center, count_lines, fit_to_box, justify, measure_lines, pad_left, pad_lines, pad_right,
    truncate, truncate_lines, wrap_words, wrap_words_max_lines,
};
pub use threat::ThreatTable;
pub use tilemap::{LayeredMap, TileMap};
pub use timer::{Cooldown, TimerQueue};
pub use timestep::FixedTimestep;
pub use trigger::{Trigger, TriggerSet};
pub use turn::Scheduler;
pub use tween::{Tween, TweenSequence};
pub use validator::{is_loadable, validate};
pub use vec::{Vec2, Vec3};
pub use visibility::{Visibility, VisibilityMap};
pub use wallet::Wallet;
pub use wfc::{
    wfc_solve, wfc_solve_backtrack, wfc_solve_partial, wfc_solve_retry, WfcGrid, WfcResult,
    WfcRules,
};
pub use world_hash::{hash_state, hash_unordered, DetHash, Fnv1a, LabeledDigest};
