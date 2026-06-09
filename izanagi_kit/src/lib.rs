//! `izanagi_kit` — zero-dependency reference modules extracted from a design
//! review of the IZANAGI engine (v4.4.0). Each module is a drop-in candidate
//! addressing a P1 improvement:
//!
//! - [`entity`] / [`sparse_set`] — sparse-set ECS storage with generational
//!   handles (cheap composition changes, O(1) lookup).
//! - [`fixed`] — Q16.16 fixed-point for cross-platform-deterministic math.
//! - [`fov`] — symmetric shadowcasting field-of-view (integer, deterministic).
//! - [`geometry`] — integer Bresenham line drawing and line-of-sight.
//! - [`mapgen`] — seed-driven procedural dungeon generation (deterministic).
//! - [`pathfinding`] — deterministic 8-way A* and weighted A* (ε-admissible) grid pathfinding.
//! - [`replay`] — replay trace recording, desync detection and rollback.
//! - [`rng`] — SplitMix64 seeded PRNG (replay-safe randomness).
//! - [`msglog`] — bounded ring-buffer message log with `DetHash`.
//! - [`terminal`] — headless cell screen buffer with 24-bit ANSI output.
//! - [`timestep`] — fixed-timestep accumulator with death-spiral guard.
//! - [`timer`] — tick-based `Cooldown` and `TimerQueue<E>` for delayed events.
//! - [`turn`] — energy/speed-based turn scheduler.
//! - [`vec`] — fixed-point Vec2/Vec3 (dot/cross/len/normalize/scale/DetHash).
//! - [`world_hash`] — FNV-1a per-frame state checksum for bit-exact replay.
//! - [`camera`] — integer camera / viewport (world↔screen coordinate mapping).
//! - [`change`] — dirty-flag change detection (`Changed<T>`, `ChangeTracker`).
//! - [`combat`] — integer combat formula (stats, melee/ranged, hit roll).
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
//! - [`hud`] — HUD primitives: fill bar (`BarWidget`), stat line, panel layout (`HudPanel`).
//! - [`autotile`] — bitmask auto-tiling (`compute_mask`, `SimpleTileTable`).
//! - [`diag_json`] — machine-readable JSON serialization of pipeline diagnostics (`diag_json`).
//! - [`passability`] — grid-based passability / collision layer (`PassabilityGrid`).
//! - [`savefile`] — versioned binary save-file framing (`save_bytes`, `load_bytes`, `SaveHeader`).
//! - [`wfc`] — Wave Function Collapse procedural tile-map generation (`WfcRules`, `wfc_solve`, `WfcGrid`).
//! - [`multimap`] — multi-floor dungeon stack (`MultiMap`, `Connector`).
//!
//! All modules are `std`-only and contain no `unsafe`.

#![forbid(unsafe_code)]

pub mod aabb;
pub mod arch;
pub mod assets;
pub mod autotile;
pub mod camera;
pub mod change;
pub mod cmdqueue;
pub mod combat;
pub mod content;
pub mod diag_json;
pub mod dice;
pub mod easing;
pub mod entity;
pub mod fixed;
pub mod fov;
pub mod fsm;
pub mod geometry;
pub mod hud;
pub mod influence;
pub mod inputbuf;
pub mod inventory;
pub mod keymap;
pub mod loader;
pub mod mapgen;
pub mod menu;
pub mod msglog;
pub mod multimap;
pub mod noise;
pub mod parser;
pub mod passability;
pub mod pathfinding;
pub mod profiler;
pub mod random_table;
pub mod relations;
pub mod replay;
pub mod rng;
pub mod savefile;
pub mod serializer;
pub mod sparse_set;
pub mod spatial_hash;
pub mod status;
pub mod terminal;
pub mod textlayout;
pub mod tilemap;
pub mod timer;
pub mod timestep;
pub mod turn;
pub mod validator;
pub mod vec;
pub mod wfc;
pub mod world_hash;

pub use aabb::Aabb;
pub use arch::ArchTable;
pub use assets::{AssetHandle, AssetStore};
pub use autotile::{compute_all, compute_mask, SimpleTileTable};
pub use camera::Camera;
pub use change::{ChangeTracker, Changed};
pub use cmdqueue::CmdQueue;
pub use combat::{
    base_damage, critical_strike, melee_attack, ranged_attack, roll_to_hit, Stats, StrikeResult,
};
pub use content::{Content, Diagnostic, Prefab, Severity, Tile};
pub use dice::Dice;
pub use easing::{
    ease_in_back, ease_in_bounce, ease_in_circ, ease_in_cubic, ease_in_expo, ease_in_out_back,
    ease_in_out_bounce, ease_in_out_circ, ease_in_out_cubic, ease_in_out_expo, ease_in_out_quad,
    ease_in_out_quart, ease_in_out_quint, ease_in_out_sine, ease_in_quad, ease_in_quart,
    ease_in_quint, ease_in_sine, ease_out_back, ease_out_bounce, ease_out_circ, ease_out_cubic,
    ease_out_expo, ease_out_quad, ease_out_quart, ease_out_quint, ease_out_sine, linear,
};
pub use entity::{Entity, EntityAllocator};
pub use fixed::Fixed;
pub use fov::{compute_fov, compute_fov_dist};
pub use fsm::Fsm;
pub use geometry::{diamond, line, line_of_sight, rect_contains, rect_perimeter, Distance};
pub use hud::{BarWidget, HudPanel, StatLine};
pub use influence::InfluenceMap;
pub use inputbuf::InputBuffer;
pub use inventory::Inventory;
pub use keymap::KeyMap;
pub use loader::{load_level, LoadedLevel, Position, Render};
pub use mapgen::{
    generate_bsp, generate_cave, generate_dungeon, BspParams, CaveParams, Dungeon, GenParams, Rect,
};
pub use menu::{Menu, MenuItem};
pub use msglog::MsgLog;
pub use multimap::{Connector, MultiMap};
pub use noise::{
    fbm_1d, fbm_1d_wrap, fbm_2d, fbm_2d_wrap, hash_1d, hash_2d, normalize_noise, ridge_noise_2d,
    value_noise_1d, value_noise_1d_wrap, value_noise_2d, value_noise_2d_wrap,
};
pub use parser::parse;
pub use passability::PassabilityGrid;
pub use pathfinding::{astar, descend, dijkstra_map, smooth_path, step_toward, weighted_astar};
pub use profiler::{EventLog, LogEntry, Profiler};
pub use random_table::RandomTable;
pub use relations::Relations;
pub use replay::{check_trace, first_divergence, record_trace, resimulate, Divergence};
pub use rng::SplitMix64;
pub use savefile::{load_bytes, load_bytes_owned, save_bytes, LoadError, SaveHeader};
pub use serializer::{content_eq, serialize};
pub use sparse_set::{join, join_mut, SparseSet};
pub use spatial_hash::SpatialHash;
pub use status::{Effect, StatusSet};
pub use terminal::{Cell, Screen};
pub use textlayout::{center, justify, pad_left, pad_right, truncate, wrap_words};
pub use tilemap::{LayeredMap, TileMap};
pub use timer::{Cooldown, TimerQueue};
pub use timestep::FixedTimestep;
pub use turn::Scheduler;
pub use validator::{is_loadable, validate};
pub use vec::{Vec2, Vec3};
pub use wfc::{wfc_solve, WfcGrid, WfcResult, WfcRules};
pub use world_hash::{hash_state, DetHash, Fnv1a};
