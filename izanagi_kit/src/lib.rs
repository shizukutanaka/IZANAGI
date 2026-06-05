//! `izanagi_kit` — zero-dependency reference modules extracted from a design
//! review of the IZANAGI engine (v4.4.0). Each module is a drop-in candidate
//! addressing a P1 improvement:
//!
//! - [`entity`] / [`sparse_set`] — sparse-set ECS storage with generational
//!   handles (cheap composition changes, O(1) lookup).
//! - [`fixed`] — Q16.16 fixed-point for cross-platform-deterministic math.
//! - [`fov`] — symmetric shadowcasting field-of-view (integer, deterministic).
//! - [`rng`] — SplitMix64 seeded PRNG (replay-safe randomness).
//! - [`timestep`] — fixed-timestep accumulator with death-spiral guard.
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
pub mod loader;
pub mod parser;
pub mod rng;
pub mod serializer;
pub mod sparse_set;
pub mod timestep;
pub mod validator;
pub mod world_hash;

pub use content::{Content, Diagnostic, Prefab, Severity, Tile};
pub use entity::{Entity, EntityAllocator};
pub use fixed::Fixed;
pub use fov::compute_fov;
pub use loader::{load_level, LoadedLevel, Position, Render};
pub use parser::parse;
pub use rng::SplitMix64;
pub use serializer::{content_eq, serialize};
pub use sparse_set::SparseSet;
pub use timestep::FixedTimestep;
pub use validator::{is_loadable, validate};
pub use world_hash::{DetHash, Fnv1a};
