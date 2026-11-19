//! Bundle-level determinism verification — the SyncTest, productised.
//!
//! GGRS's `SyncTestSession` is the tool rollback developers value most
//! ("the only takeaway" in the practical rollback literature), yet it stays
//! a developer session that panics on the first mismatch — no engine ships
//! it as a standard command with a CI recipe. This module closes that gap
//! for `.game` bundles: it loads every level, runs a canonical deterministic
//! workload over the loaded world, and reports whether the run reproduces
//! itself — in one call, with a report instead of a panic.
//!
//! ## What is checked
//!
//! [`verify_bundle`] runs three independent checks per level:
//!
//! 1. **Reload determinism** — the level is loaded twice from scratch and
//!    the two initial states are hashed. A mismatch means the content
//!    pipeline itself (parse → validate → load) is not reproducible.
//! 2. **Double run** — the same input script is run twice from the same
//!    start and the per-tick hash traces are compared
//!    ([`sim::audit`](crate::sim::audit)). Catches nondeterminism that is
//!    stable within a run but varies across runs.
//! 3. **Sync test** — every frame is rolled back and resimulated
//!    ([`rollback::sync_test`](crate::rollback::sync_test)). Catches
//!    order-of-operations bugs that only appear on partial re-execution —
//!    the path a rollback netcode session actually takes.
//!
//! On any failure the report names the **first diverging tick** with both
//! hashes, so the hunt starts at a tick instead of a hunch.
//!
//! ## The canonical workload ("soak")
//!
//! A bundle defines content, not behaviour — there is no game sim to run.
//! So verification runs a documented reference workload over the loaded
//! world instead: each tick, every entity moves one cell in a seeded
//! pseudo-random direction and, when it carries stats, its first stat is
//! nudged by a small seeded amount. The workload is deliberately boring —
//! it exercises the substrate a real game stands on (component stores,
//! canonical iteration order, the seeded RNG, hashing) without pretending
//! to be the author's game. The input script is derived from the level
//! name, so each level gets its own script and the same level always gets
//! the same script.
//!
//! ## What is *not* checked
//!
//! The same contract as the validator: "well-formed and self-consistent"
//! is the bar — balance, intent and whether the content is *good* remain
//! the author's. The soak is not a substitute for auditing the author's
//! own simulation with [`sim::audit`](crate::sim::audit); it is the
//! bundle-side gate that runs before the author's sim ever sees the data.
//!
//! ## CI
//!
//! `gamec --verify <file.game>` runs this check from the command line and
//! exits `0` iff every level is deterministic, `1` on divergence (or on
//! content that fails validation), `2` on usage or unreadable input — so it
//! drops straight into a CI job. See the kit README's `gamec` section for a
//! ready-to-paste workflow snippet.

use crate::content::Content;
use crate::entity::Entity;
use crate::loader::{load_level, LoadedLevel, Position, Render, Stats};
use crate::replay::Divergence;
use crate::rng::SplitMix64;
use crate::rollback::SyncTestFailure;
use crate::sim::{audit, Simulation};
use crate::world_hash::{hash_state, DetHash, Fnv1a};

/// Default number of soak ticks per level for [`verify_bundle`] callers
/// that do not choose their own. Long enough that the RNG stream and every
/// component store are exercised many times over; short enough that the
/// whole check stays a fraction of a second on a content-sized level.
pub const DEFAULT_TICKS: usize = 256;

/// Rollback depth (in frames) for the sync test inside [`audit`]. Kept
/// small: the check runs at every frame, so depth 4 already exercises the
/// partial re-execution path without turning the gate quadratic in a way
/// anyone would feel.
const SYNC_CHECK_DISTANCE: usize = 4;

/// Tag mixed into the input-script seed so scripts for this workload cannot
/// collide with any other seeded stream in the kit.
const SCRIPT_TAG: &str = "izanagi_kit::bundle_verify soak";

/// One actor in the soak state: the components a loaded level spawns, in
/// spawn order. Render never changes during the soak but is hashed every
/// tick, so a corruption of the render store is caught too.
#[derive(Clone)]
struct Actor {
    /// The entity handle, for canonical (ascending-index) hashing.
    entity: Entity,
    /// Grid position — moved by the soak step.
    pos: Position,
    /// Terminal appearance — hashed, never mutated.
    render: Render,
    /// Numeric stats from the prefab template, if any — the first entry is
    /// nudged by the soak step.
    stats: Option<Stats>,
}

/// The soak state: a level distilled into canonical order plus the seeded
/// RNG that drives it. Kept private on purpose — this is a verification
/// harness, not a wire format, so it carries no golden hash pin; its
/// stability is enforced by the audit tests themselves.
#[derive(Clone)]
struct SoakState {
    /// Actors in spawn order (canonical iteration order).
    actors: Vec<Actor>,
    /// The seeded stream that drives the soak. Part of the state so a
    /// desync in the RNG itself is caught by the trace.
    rng: SplitMix64,
}

impl SoakState {
    /// Distil a freshly loaded level into canonical order. The actor list
    /// follows `entities` (spawn order); component lookups are by handle,
    /// so an entity without a stats component simply has `stats: None`.
    fn from_level(level: &LoadedLevel) -> SoakState {
        let mut actors = Vec::with_capacity(level.entities.len());
        for &entity in &level.entities {
            actors.push(Actor {
                entity,
                pos: level
                    .positions
                    .get(entity)
                    .copied()
                    .unwrap_or(Position { x: 0, y: 0 }),
                render: level.renders.get(entity).copied().unwrap_or(Render {
                    glyph: '?',
                    color: crate::content::Color::rgb(255, 255, 255),
                }),
                stats: level.stats.get(entity).cloned(),
            });
        }
        SoakState {
            actors,
            rng: SplitMix64::new(0xB17E_5EED),
        }
    }
}

impl DetHash for SoakState {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.actors.len() as u32);
        for actor in &self.actors {
            actor.entity.det_hash(hasher);
            actor.pos.det_hash(hasher);
            actor.render.det_hash(hasher);
            match &actor.stats {
                Some(stats) => {
                    hasher.write_bool(true);
                    stats.det_hash(hasher);
                }
                None => {
                    hasher.write_bool(false);
                }
            }
        }
        self.rng.det_hash(hasher);
    }
}

impl Simulation for SoakState {
    type Input = u64;

    /// One soak tick. For each actor in canonical order, draw from the
    /// seeded stream mixed with the tick's input word, move one cell in
    /// `{-1, 0, 1}` on each axis (clamped to the grid's non-negative
    /// quadrant), and nudge the first stat by `{-2, -1, 0, 1, 2}` when the
    /// actor carries stats. Integer arithmetic only — no floats, no
    /// wall-clock, no unordered iteration.
    fn step(&mut self, input: &u64) {
        for actor in &mut self.actors {
            let r = self.rng.next_u64() ^ *input;
            let dx = (r % 3) as i64 - 1;
            let dy = ((r >> 8) % 3) as i64 - 1;
            actor.pos.x = (actor.pos.x as i64 + dx).max(0).min(i64::from(u32::MAX)) as u32;
            actor.pos.y = (actor.pos.y as i64 + dy).max(0).min(i64::from(u32::MAX)) as u32;
            if let Some(stats) = &mut actor.stats {
                let delta = ((r >> 16) % 5) as i32 - 2;
                // Extract the first entry owned, so the iterator's borrow
                // ends before `set` mutates the component.
                let first = stats.iter().next().map(|(k, v)| (k.to_string(), v));
                if let Some((key, value)) = first {
                    stats.set(&key, value.wrapping_add(delta));
                }
            }
        }
    }
}

/// The per-tick input script for one level: `ticks` words from a seeded
/// stream whose seed is the level name, so each level gets its own script
/// and the same level always gets the same script.
fn soak_inputs(level_name: &str, ticks: usize) -> Vec<u64> {
    let mut seed_hasher = Fnv1a::new();
    seed_hasher.write_str(SCRIPT_TAG);
    seed_hasher.write_str(level_name);
    let mut rng = SplitMix64::new(seed_hasher.finish());
    let mut inputs = Vec::with_capacity(ticks);
    for _ in 0..ticks {
        inputs.push(rng.next_u64());
    }
    inputs
}

/// What [`verify_level`] found for one level. [`is_deterministic`](Self::is_deterministic)
/// is the one-line verdict; the individual fields say *how* it failed.
/// The `final_hash` is stable for a given bundle and tick count — pin it as
/// a regression value once the check passes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelVerification {
    /// The level this report is about.
    pub level: String,
    /// Entities spawned for this level.
    pub entities: usize,
    /// Soak ticks run.
    pub ticks: usize,
    /// State hash of the freshly loaded level, before any tick.
    pub initial_hash: u64,
    /// State hash after the last soak tick, from the first run.
    pub final_hash: u64,
    /// `true` when two independent loads of the same level hashed
    /// differently — the content pipeline itself is not reproducible.
    pub reload_mismatch: bool,
    /// Set when two identical runs produced different hash traces.
    pub double_run: Option<Divergence>,
    /// Set when rolling back and resimulating produced a different state
    /// than running straight through.
    pub sync_test: Option<SyncTestFailure>,
}

impl LevelVerification {
    /// `true` when every check passed: the level loads reproducibly and
    /// the soak is deterministic across repeated runs *and* under rollback
    /// resimulation.
    pub fn is_deterministic(&self) -> bool {
        !self.reload_mismatch && self.double_run.is_none() && self.sync_test.is_none()
    }
}

impl core::fmt::Display for LevelVerification {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.is_deterministic() {
            return write!(
                f,
                "level '{}': deterministic over {} tick(s), {} entit(y/ies); final hash {:#018x}",
                self.level, self.ticks, self.entities, self.final_hash
            );
        }
        write!(
            f,
            "level '{}': NONDETERMINISTIC over {} tick(s):",
            self.level, self.ticks
        )?;
        if self.reload_mismatch {
            write!(f, " reload mismatch;")?;
        }
        if let Some(d) = &self.double_run {
            write!(f, " double-run: {d};")?;
        }
        if let Some(s) = &self.sync_test {
            write!(f, " sync test: {s}")?;
        }
        Ok(())
    }
}

/// What [`verify_bundle`] found for a whole bundle: one report per level,
/// in the bundle's declared level order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BundleVerification {
    /// Per-level reports, in declared level order.
    pub levels: Vec<LevelVerification>,
}

impl BundleVerification {
    /// `true` when every level in the bundle is deterministic. Vacuously
    /// `true` for a bundle with no levels — the report says so explicitly
    /// instead of silently passing.
    pub fn is_deterministic(&self) -> bool {
        self.levels.iter().all(|l| l.is_deterministic())
    }
}

impl core::fmt::Display for BundleVerification {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.levels.is_empty() {
            return write!(f, "verify: no levels in bundle");
        }
        for level in &self.levels {
            writeln!(f, "{level}")?;
        }
        let ok = self.levels.iter().filter(|l| l.is_deterministic()).count();
        write!(
            f,
            "verify: {ok}/{} level(s) deterministic",
            self.levels.len()
        )
    }
}

/// Verify one level of an already-parsed bundle: reload determinism plus
/// the double-run and sync-test audit over the canonical soak workload.
///
/// Returns `Err` when the level cannot be loaded (unknown level, broken
/// prefab reference) — that is a content error, not a divergence.
pub fn verify_level(
    content: &Content,
    level_name: &str,
    ticks: usize,
) -> Result<LevelVerification, String> {
    let first = load_level(content, level_name)?;
    let second = load_level(content, level_name)?;
    let state_a = SoakState::from_level(&first);
    let state_b = SoakState::from_level(&second);
    let initial_hash = hash_state(&state_a);
    let reload_mismatch = initial_hash != hash_state(&state_b);
    let inputs = soak_inputs(level_name, ticks);
    let report = audit(&state_a, &inputs, SYNC_CHECK_DISTANCE);
    Ok(LevelVerification {
        level: level_name.to_string(),
        entities: state_a.actors.len(),
        ticks,
        initial_hash,
        final_hash: report.final_hash,
        reload_mismatch,
        double_run: report.double_run,
        sync_test: report.sync_test,
    })
}

/// Verify every level of an already-parsed bundle, in declared level
/// order. Fails on the first level that cannot be loaded; divergence in
/// any level is reported, not returned as an error — inspect
/// [`BundleVerification::is_deterministic`] and the per-level reports.
pub fn verify_bundle(content: &Content, ticks: usize) -> Result<BundleVerification, String> {
    let mut levels = Vec::with_capacity(content.levels.len());
    for level in &content.levels {
        levels.push(verify_level(content, &level.name, ticks)?);
    }
    Ok(BundleVerification { levels })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    const TWO_LEVELS: &str = "\
prefab hero\n  glyph @\n  color #00c4cc\n  stat hp 20\nprefab goblin\n  glyph g\n  color #f85149\n  stat hp 8\nlevel room 5x3\n  row #####\n  row #@..#\n  row #####\n  spawn hero 1 1\nlevel cellar 4x3\n  row ####\n  row #g.#\n  row ####\n  spawn goblin 1 1\n";

    fn parsed(src: &str) -> Content {
        let (content, diags) = parse(src);
        assert!(
            diags.iter().all(|d| !d.is_error()),
            "sample bundle must parse cleanly: {diags:?}"
        );
        content
    }

    #[test]
    fn test_verify_level_is_deterministic_and_stable() {
        let content = parsed(TWO_LEVELS);
        let first = verify_level(&content, "room", 64).unwrap();
        let second = verify_level(&content, "room", 64).unwrap();
        assert!(first.is_deterministic(), "{}", first);
        assert_eq!(first, second, "same bundle + ticks must verify identically");
        assert_eq!(first.level, "room");
        assert_eq!(first.entities, 1);
        assert_eq!(first.ticks, 64);
        assert!(!first.reload_mismatch);
        assert!(first.double_run.is_none());
        assert!(first.sync_test.is_none());
    }

    #[test]
    fn test_verify_bundle_checks_every_level() {
        let content = parsed(TWO_LEVELS);
        let report = verify_bundle(&content, 32).unwrap();
        assert_eq!(report.levels.len(), 2);
        assert!(report.is_deterministic(), "{}", report);
        assert_eq!(report.levels[0].level, "room");
        assert_eq!(report.levels[1].level, "cellar");
        let text = report.to_string();
        assert!(text.contains("2/2 level(s) deterministic"), "{text}");
    }

    #[test]
    fn test_verify_level_zero_ticks_hashes_the_loaded_state() {
        let content = parsed(TWO_LEVELS);
        let report = verify_level(&content, "room", 0).unwrap();
        assert!(report.is_deterministic());
        assert_eq!(report.final_hash, report.initial_hash);
    }

    #[test]
    fn test_verify_level_unknown_name_errors() {
        let content = parsed(TWO_LEVELS);
        assert!(verify_level(&content, "nope", 16).is_err());
    }

    #[test]
    fn test_empty_bundle_is_vacuously_deterministic_and_says_so() {
        let content = parsed("prefab g\n  glyph g\n");
        let report = verify_bundle(&content, 16).unwrap();
        assert!(report.levels.is_empty());
        assert!(report.is_deterministic());
        assert_eq!(report.to_string(), "verify: no levels in bundle");
    }

    #[test]
    fn test_soak_actually_moves_state() {
        let content = parsed(TWO_LEVELS);
        let level = load_level(&content, "room").unwrap();
        let mut state = SoakState::from_level(&level);
        let before = hash_state(&state);
        let inputs = soak_inputs("room", 16);
        for input in &inputs {
            state.step(input);
        }
        assert_ne!(hash_state(&state), before, "the soak must move the state");
    }

    #[test]
    fn test_input_script_is_stable_per_level_and_distinct_across_levels() {
        let a = soak_inputs("room", 8);
        let b = soak_inputs("room", 8);
        let c = soak_inputs("cellar", 8);
        assert_eq!(a, b, "same level must always get the same script");
        assert_ne!(a, c, "different levels must get different scripts");
        assert_eq!(a.len(), 8);
    }

    #[test]
    fn test_divergence_report_names_the_tick_and_hashes() {
        let report = LevelVerification {
            level: "room".to_string(),
            entities: 1,
            ticks: 64,
            initial_hash: 1,
            final_hash: 2,
            reload_mismatch: false,
            double_run: Some(Divergence {
                tick: 3,
                expected: 0xAAAA,
                actual: 0xBBBB,
            }),
            sync_test: None,
        };
        assert!(!report.is_deterministic());
        let text = report.to_string();
        assert!(text.contains("NONDETERMINISTIC"), "{text}");
        assert!(text.contains("tick 3"), "{text}");
        assert!(text.contains("0x000000000000aaaa"), "{text}");
        assert!(text.contains("0x000000000000bbbb"), "{text}");
    }

    #[test]
    fn test_reload_mismatch_report_says_so() {
        let mut report = LevelVerification {
            level: "room".to_string(),
            entities: 1,
            ticks: 64,
            initial_hash: 1,
            final_hash: 2,
            reload_mismatch: true,
            double_run: None,
            sync_test: None,
        };
        assert!(!report.is_deterministic());
        assert!(report.to_string().contains("reload mismatch"));
        report.reload_mismatch = false;
        assert!(report.is_deterministic());
        assert!(report.to_string().contains("deterministic"));
    }
}
