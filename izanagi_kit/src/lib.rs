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
//! - [`pathfinding`] — deterministic 8-way A* grid pathfinding.
//! - [`replay`] — replay trace recording, desync detection and rollback.
//! - [`rng`] — SplitMix64 seeded PRNG (replay-safe randomness).
//! - [`msglog`] — bounded ring-buffer message log with `DetHash`.
//! - [`terminal`] — headless cell screen buffer with 24-bit ANSI output.
//! - [`timestep`] — fixed-timestep accumulator with death-spiral guard.
//! - [`turn`] — energy/speed-based turn scheduler.
//! - [`vec`] — fixed-point Vec2/Vec3 (dot/cross/len/normalize/scale/DetHash).
//! - [`world_hash`] — FNV-1a per-frame state checksum for bit-exact replay.
//! - [`content`] / [`parser`] / [`serializer`] / [`validator`] / [`loader`] —
//!   the content pipeline: author game elements as text, serialize them back,
//!   validate them, load into the ECS.
//!
//! All modules are `std`-only and contain no `unsafe`.

#![forbid(unsafe_code)]

pub mod content;
pub mod entity;
pub mod fixed;
pub mod fov;
pub mod geometry;
pub mod loader;
pub mod mapgen;
pub mod msglog;
pub mod parser;
pub mod pathfinding;
pub mod replay;
pub mod rng;
pub mod serializer;
pub mod sparse_set;
pub mod terminal;
pub mod timestep;
pub mod turn;
pub mod validator;
pub mod vec;
pub mod world_hash;

pub use content::{Content, Diagnostic, Prefab, Severity, Tile};
pub use entity::{Entity, EntityAllocator};
pub use fixed::Fixed;
pub use fov::compute_fov;
pub use geometry::{line, line_of_sight};
pub use loader::{load_level, LoadedLevel, Position, Render};
pub use mapgen::{generate_dungeon, Dungeon, GenParams, Rect};
pub use msglog::MsgLog;
pub use parser::parse;
pub use pathfinding::astar;
pub use replay::{check_trace, first_divergence, record_trace, resimulate, Divergence};
pub use rng::SplitMix64;
pub use serializer::{content_eq, serialize};
pub use sparse_set::{join, join_mut, SparseSet};
pub use terminal::{Cell, Screen};
pub use timestep::FixedTimestep;
pub use turn::Scheduler;
pub use validator::{is_loadable, validate};
pub use vec::{Vec2, Vec3};
pub use world_hash::{hash_state, DetHash, Fnv1a};
