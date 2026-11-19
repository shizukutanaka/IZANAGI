//! Semantic validation of parsed [`Content`].
//!
//! The parser answers "is each line well-formed?"; the validator answers "does
//! the bundle hang together?" — undefined references, duplicate names,
//! out-of-bounds spawns, grid/dimension mismatches. All findings are collected
//! (never short-circuit) so one run surfaces every problem.
//!
//! This is the gate that catches the class of mistakes an LLM is prone to
//! (silently referencing a renamed prefab, off-by-one coordinates): exactly the
//! "verify machine-generated content" requirement.
//!
//! The spatial checks (sealed-off regions, unreachable spawns) key off the
//! one piece of terrain semantics a bundle can state — the tile named
//! `wall` — and stay silent when the bundle does not declare one. See
//! `check_spatial` below.

use crate::content::{Content, Diagnostic, ExtendsError, Level, Spawn};
use crate::pathfinding::ConnectivityMap;
use std::collections::{BTreeSet, HashSet};

/// Returns all semantic diagnostics. Empty error set == loadable bundle.
pub fn validate(content: &Content) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    check_unique(
        content.prefabs.iter().map(|p| p.name.as_str()),
        "prefab",
        &mut diags,
    );
    check_unique(
        content.tiles.iter().map(|t| t.name.as_str()),
        "tile",
        &mut diags,
    );
    check_unique(
        content.levels.iter().map(|l| l.name.as_str()),
        "level",
        &mut diags,
    );

    // Glyph printability: control characters (e.g. NUL, ESC, DEL) are never
    // renderable in a terminal cell and almost certainly an authoring error.
    for prefab in &content.prefabs {
        if prefab.glyph.is_control() {
            diags.push(Diagnostic::error(
                0,
                format!(
                    "prefab '{}': glyph U+{:04X} is a control character",
                    prefab.name, prefab.glyph as u32
                ),
            ));
        }
    }
    for tile in &content.tiles {
        if tile.glyph.is_control() {
            diags.push(Diagnostic::error(
                0,
                format!(
                    "tile '{}': glyph U+{:04X} is a control character",
                    tile.name, tile.glyph as u32
                ),
            ));
        }
    }

    // `extends` chains: every prefab's chain must resolve. Missing bases and
    // cycles surface via `resolve_prefab`; the message set is deduplicated
    // because a broken chain reports the same error from each member.
    let mut extends_diags = BTreeSet::new();
    for prefab in &content.prefabs {
        if let Err(e) = content.resolve_prefab(&prefab.name) {
            let msg = match &e {
                ExtendsError::MissingBase { base, .. } => {
                    match suggest_name(base, content.prefabs.iter().map(|p| p.name.as_str())) {
                        Some(s) => format!("{e} (did you mean '{s}'?)"),
                        None => e.to_string(),
                    }
                }
                ExtendsError::Cycle { .. } => e.to_string(),
            };
            extends_diags.insert(msg);
        }
    }
    for msg in extends_diags {
        diags.push(Diagnostic::error(0, msg));
    }

    let known_prefabs: HashSet<&str> = content.prefabs.iter().map(|p| p.name.as_str()).collect();

    // Unused-prefab warnings: defined but never spawned in any level. A prefab
    // serving only as an `extends` base counts as used — its fields flow into
    // every child that spawns.
    let used_prefabs: HashSet<&str> = content
        .levels
        .iter()
        .flat_map(|l| l.spawns.iter())
        .map(|s| s.prefab.as_str())
        .chain(content.prefabs.iter().filter_map(|p| p.extends.as_deref()))
        .collect();
    for prefab in &content.prefabs {
        if !used_prefabs.contains(prefab.name.as_str()) {
            diags.push(Diagnostic::warning(
                0,
                format!("prefab '{}' is defined but never spawned", prefab.name),
            ));
        }
    }

    // Unused-tile warning: a tile is defined but its glyph never appears in
    // any level's grid, so nothing on the map ever renders with it. This is
    // an authoring-consistency check only — as of writing, `loader.rs` builds
    // the ECS world purely from `Spawn`s and never reads `Level::rows` or
    // `Content::tiles`, so an unused tile has no effect on what actually
    // loads. It is still worth flagging: an unreferenced tile is very likely
    // a stale/renamed entry that a human or LLM author should either wire up
    // or delete.
    let used_tile_glyphs: HashSet<char> = content
        .levels
        .iter()
        .flat_map(|l| l.rows.iter())
        .flat_map(|row| row.chars())
        .collect();
    for tile in &content.tiles {
        if !used_tile_glyphs.contains(&tile.glyph) {
            diags.push(Diagnostic::warning(
                0,
                format!(
                    "tile '{}' is defined but its glyph '{}' never appears in any level",
                    tile.name, tile.glyph
                ),
            ));
        }
    }

    for level in &content.levels {
        // Grid consistency.
        if level.rows.len() as u32 != level.height {
            diags.push(Diagnostic::error(
                0,
                format!(
                    "level '{}': declared height {} but {} row(s) given",
                    level.name,
                    level.height,
                    level.rows.len()
                ),
            ));
        }
        for (ri, row) in level.rows.iter().enumerate() {
            let w = row.chars().count() as u32;
            if w != level.width {
                diags.push(Diagnostic::error(
                    0,
                    format!(
                        "level '{}': row {} width {} != declared width {}",
                        level.name, ri, w, level.width
                    ),
                ));
            }
        }
        // Spawn references and bounds.
        let mut occupied: HashSet<(u32, u32)> = HashSet::new();
        for spawn in &level.spawns {
            if !known_prefabs.contains(spawn.prefab.as_str()) {
                let msg = match suggest_name(
                    &spawn.prefab,
                    content.prefabs.iter().map(|p| p.name.as_str()),
                ) {
                    Some(s) => format!(
                        "level '{}': spawn references undefined prefab '{}' (did you mean '{}'?)",
                        level.name, spawn.prefab, s
                    ),
                    None => format!(
                        "level '{}': spawn references undefined prefab '{}'",
                        level.name, spawn.prefab
                    ),
                };
                diags.push(Diagnostic::error(0, msg));
            }
            if spawn.x >= level.width || spawn.y >= level.height {
                diags.push(Diagnostic::error(
                    0,
                    format!(
                        "level '{}': spawn '{}' at ({},{}) is outside {}x{}",
                        level.name, spawn.prefab, spawn.x, spawn.y, level.width, level.height
                    ),
                ));
            }
            // Overlapping spawns: two entities authored onto the same cell. The
            // loader allows stacking (e.g. an item on a monster), so this is a
            // warning, not an error — but it is a frequent machine-generation
            // slip worth surfacing.
            if !occupied.insert((spawn.x, spawn.y)) {
                diags.push(Diagnostic::warning(
                    0,
                    format!(
                        "level '{}': multiple spawns at ({},{}) (e.g. '{}')",
                        level.name, spawn.x, spawn.y, spawn.prefab
                    ),
                ));
            }
        }
    }

    // Spatial checks run last, after every per-level finding above.
    check_spatial(content, &mut diags);

    diags
}

/// Spatial (map-shape) checks: regions of a level that no spawn can reach,
/// and spawn-bearing regions cut off from the level's entrance.
///
/// The `.game` format declares no solidity of its own, so these checks key
/// off the one piece of terrain semantics a bundle can state: the tile
/// named `wall`. Its glyph is treated as solid; every other glyph is open
/// floor, and a cell holding a spawn is always standable (an entity is
/// authored standing there). Without a `wall` tile the checks stay silent
/// rather than guess at the author's terrain. Movement uses the kit's
/// canonical rule — 8-directional with no corner cutting, via
/// [`ConnectivityMap`] — so "reachable" here means reachable by an entity
/// moving under the kit's own pathfinding.
///
/// The level's first in-bounds spawn (declaration order) is the entrance.
/// Two warning classes are reported per region, in row-major scan order of
/// each region's first cell:
///
/// - a region holding spawns but not the entrance: its occupants can never
///   meet the entrance spawn — the classic sealed-vault slip;
/// - a region holding no spawn at all: an isolated room, built but
///   unreachable and unoccupied.
///
/// Both are warnings: the bundle still loads. Levels with no in-bounds
/// spawn have no entrance to anchor reachability, so they are skipped.
fn check_spatial(content: &Content, diags: &mut Vec<Diagnostic>) {
    if let Some(wall) = content.tiles.iter().find(|t| t.name == "wall") {
        for level in &content.levels {
            check_level_spatial(level, wall.glyph, diags);
        }
    }
}

/// The per-level half of [`check_spatial`]: label the level's regions with
/// [`ConnectivityMap`] and warn about every region except the entrance's.
fn check_level_spatial(level: &Level, wall_glyph: char, diags: &mut Vec<Diagnostic>) {
    let w = level.width;
    let h = level.height;
    // The parser caps a level at 256x256 (SPEC §9.1), so parsed bundles
    // stay below 65,536 cells. A programmatic bundle beyond that envelope
    // is skipped rather than letting an unbounded declared dimension size
    // the scratch grids below.
    if (w as u64) * (h as u64) <= 65_536 {
        // In-bounds spawns in declaration order; the first is the entrance
        // every other spawn is expected to be able to meet. (Out-of-bounds
        // spawns already drew their own error above.)
        let spawns: Vec<&Spawn> = level.spawns.iter().filter(|s| s.x < w && s.y < h).collect();
        if let Some(&entrance) = spawns.first() {
            // Terrain by glyph. Cells the rows do not cover (a short or
            // missing row — already a dimension error above) read as wall,
            // so a malformed grid seals its gaps instead of opening them.
            let mut grid = vec![wall_glyph; (w * h) as usize];
            for (y, row) in level.rows.iter().enumerate() {
                for (x, ch) in row.chars().enumerate() {
                    if x < w as usize && y < h as usize {
                        grid[y * w as usize + x] = ch;
                    }
                }
            }
            // A spawn cell is standable even when its glyph is the wall
            // glyph: an entity is authored standing there, so the flood
            // must be able to start from it.
            let spawn_cells: HashSet<(u32, u32)> = spawns.iter().map(|s| (s.x, s.y)).collect();
            let map = ConnectivityMap::new(w, h, |x, y| {
                grid[y as usize * w as usize + x as usize] == wall_glyph
                    && !spawn_cells.contains(&(x as u32, y as u32))
            });
            if let Some(entrance_comp) = map.component(entrance.x as i32, entrance.y as i32) {
                // Per-region facts, indexed by component id (ids are handed
                // out in row-major scan order, so iterating ids in order
                // emits findings in the order a reader meets the regions).
                let count = map.component_count();
                let mut first_cell: Vec<Option<(u32, u32)>> = vec![None; count];
                for y in 0..h as i32 {
                    for x in 0..w as i32 {
                        if let Some(id) = map.component(x, y) {
                            let slot = &mut first_cell[id as usize];
                            if slot.is_none() {
                                *slot = Some((x as u32, y as u32));
                            }
                        }
                    }
                }
                let mut region_spawns: Vec<Vec<&str>> = vec![Vec::new(); count];
                for s in &spawns {
                    if let Some(id) = map.component(s.x as i32, s.y as i32) {
                        region_spawns[id as usize].push(s.prefab.as_str());
                    }
                }
                for id in 0..count as u32 {
                    if id != entrance_comp {
                        let size = map.component_size(id).unwrap_or(0);
                        let (fx, fy) = first_cell[id as usize].unwrap_or((0, 0));
                        let names = &region_spawns[id as usize];
                        if names.is_empty() {
                            diags.push(Diagnostic::warning(
                                0,
                                format!(
                                    "level '{}': isolated room of {size} tile(s) at ({fx},{fy}): no spawn inside and no path to any spawn",
                                    level.name
                                ),
                            ));
                        } else {
                            let quoted: Vec<String> =
                                names.iter().map(|n| format!("'{n}'")).collect();
                            diags.push(Diagnostic::warning(
                                0,
                                format!(
                                    "level '{}': spawn(s) {} in a region of {size} tile(s) at ({fx},{fy}) cannot be reached from spawn '{}' at ({},{})",
                                    level.name,
                                    quoted.join(", "),
                                    entrance.prefab,
                                    entrance.x,
                                    entrance.y
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// True when the bundle has zero error-severity diagnostics across both phases.
pub fn is_loadable(parse_diags: &[Diagnostic], validate_diags: &[Diagnostic]) -> bool {
    !parse_diags
        .iter()
        .chain(validate_diags)
        .any(|d| d.is_error())
}

/// Count error-severity diagnostics in a slice. Convenience for CI gates and
/// tool integrations that need a tally rather than an iteration.
pub fn error_count(diags: &[Diagnostic]) -> usize {
    diags.iter().filter(|d| d.is_error()).count()
}

/// Closest defined name to `want`, or `None` when nothing is close enough
/// to plausibly be a typo. Candidates are taken in declaration order so a
/// distance tie resolves the same way every run — the diagnostic set must
/// stay deterministic. The threshold mirrors rustc's suggestion radius
/// (roughly a third of the longer name), so fat-fingered single edits
/// surface while genuinely different names do not.
fn suggest_name<'a>(want: &str, candidates: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let mut best: Option<(&str, usize)> = None;
    for cand in candidates {
        let d = edit_distance(want, cand);
        let threshold = (want.chars().count().max(cand.chars().count()) / 3).max(1);
        if d <= threshold {
            match best {
                Some((_, bd)) if bd <= d => {}
                _ => best = Some((cand, d)),
            }
        }
    }
    best.map(|(c, _)| c)
}

/// Levenshtein edit distance over `char`s (not bytes — names may hold
/// multibyte glyphs). O(len·len) table; identifiers are short so this is
/// cheap, and integer-only so it cannot drift between builds.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j] + usize::from(ca != cb))
                .min(cur[j] + 1)
                .min(prev[j + 1] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

fn check_unique<'a>(names: impl Iterator<Item = &'a str>, kind: &str, diags: &mut Vec<Diagnostic>) {
    let mut seen = HashSet::new();
    for name in names {
        if !seen.insert(name) {
            diags.push(Diagnostic::error(
                0,
                format!("duplicate {kind} name '{name}'"),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn test_valid_bundle_has_no_errors() {
        let src = "\
prefab g
  glyph g
level a 3x2
  row ...
  row ...
  spawn g 1 1
";
        let (c, pd) = parse(src);
        let vd = validate(&c);
        assert!(is_loadable(&pd, &vd), "diags: {pd:?} {vd:?}");
    }

    #[test]
    fn test_undefined_spawn_prefab_caught() {
        let src = "level a 2x2\n  row ..\n  row ..\n  spawn ghost 0 0\n";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(vd.iter().any(|d| d.message.contains("undefined prefab")));
    }

    #[test]
    fn test_spawn_out_of_bounds_caught() {
        let src = "prefab g\n  glyph g\nlevel a 2x2\n  row ..\n  row ..\n  spawn g 5 5\n";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(vd.iter().any(|d| d.message.contains("outside")));
    }

    #[test]
    fn test_row_count_mismatch_caught() {
        let src = "level a 2x3\n  row ..\n  row ..\n";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(vd.iter().any(|d| d.message.contains("height")));
    }

    #[test]
    fn test_row_width_mismatch_caught() {
        let src = "level a 3x1\n  row ....\n";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(vd.iter().any(|d| d.message.contains("width")));
    }

    #[test]
    fn test_duplicate_prefab_caught() {
        let src = "prefab g\n  glyph g\nprefab g\n  glyph h\n";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(vd.iter().any(|d| d.message.contains("duplicate prefab")));
    }

    #[test]
    fn test_unused_prefab_warns() {
        // Prefab 'ghost' is defined but not spawned anywhere.
        let src = "prefab ghost\n  glyph g\n";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(
            vd.iter()
                .any(|d| !d.is_error() && d.message.contains("never spawned")),
            "expected unused-prefab warning; got: {vd:?}"
        );
    }

    #[test]
    fn test_spawned_prefab_has_no_unused_warning() {
        let src = "\
prefab g
  glyph g
level a 1x1
  row #
  spawn g 0 0
";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(
            !vd.iter().any(|d| d.message.contains("never spawned")),
            "spawned prefab must not trigger unused warning"
        );
    }

    #[test]
    fn test_unused_tile_warns() {
        let src = "\
tile floor . #3A3A3A
level a 3x2
  row ###
  row ###
";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(vd
            .iter()
            .any(|d| d.message.contains("never appears in any level")));
    }

    #[test]
    fn test_painted_tile_has_no_unused_warning() {
        let src = "\
tile floor . #3A3A3A
level a 3x2
  row ...
  row ...
";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(!vd
            .iter()
            .any(|d| d.message.contains("never appears in any level")));
    }

    #[test]
    fn test_unused_tile_warns_with_no_levels_at_all() {
        // Zero levels means every glyph is trivially absent, so the tile is
        // still reported unused.
        let src = "tile floor . #3A3A3A\n";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(vd
            .iter()
            .any(|d| d.message.contains("never appears in any level")));
    }

    #[test]
    fn test_unused_tile_warning_is_warning_not_error() {
        let src = "\
tile floor . #3A3A3A
level a 3x2
  row ###
  row ###
";
        let (c, pd) = parse(src);
        let vd = validate(&c);
        assert!(vd.iter().any(|d| d.message.contains("never appears")));
        assert!(vd.iter().all(|d| !d.is_error()));
        assert!(is_loadable(&pd, &vd), "diags: {pd:?} {vd:?}");
    }

    #[test]
    fn test_control_glyph_in_prefab_is_error() {
        use crate::content::Prefab;
        let mut c = crate::content::Content::default();
        let mut p = Prefab::new("bad".into());
        p.glyph = '\x1B'; // ESC — a control character
        c.prefabs.push(p);
        let vd = validate(&c);
        assert!(
            vd.iter()
                .any(|d| d.is_error() && d.message.contains("control character")),
            "expected control-char error; got: {vd:?}"
        );
    }

    #[test]
    fn test_error_count_empty_is_zero() {
        assert_eq!(error_count(&[]), 0);
    }

    #[test]
    fn test_error_count_counts_only_errors_not_warnings() {
        let src = "prefab ghost\n  glyph g\n";
        let (c, _) = parse(src);
        let vd = validate(&c);
        // "ghost" is unused — a warning, not an error.
        assert_eq!(error_count(&vd), 0);
        assert!(
            vd.iter().any(|d| !d.is_error()),
            "expected an unused-prefab warning"
        );
    }

    #[test]
    fn test_error_count_detects_errors() {
        let src = "level a 2x2\n  row ..\n  row ..\n  spawn missing 0 0\n";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert_eq!(error_count(&vd), 1, "one undefined-prefab error expected");
    }

    // --- duplicate spawn position (authoring warning) ---

    #[test]
    fn test_overlapping_spawns_warn_not_error() {
        let src = "\
prefab g
  glyph g
level a 2x2
  row ..
  row ..
  spawn g 0 0
  spawn g 0 0
";
        let (c, pd) = parse(src);
        let vd = validate(&c);
        assert!(
            vd.iter()
                .any(|d| !d.is_error() && d.message.contains("multiple spawns at (0,0)")),
            "expected an overlapping-spawn warning; got: {vd:?}"
        );
        // It is a warning, so the bundle is still loadable.
        assert!(is_loadable(&pd, &vd), "overlap must not block loading");
    }

    #[test]
    fn test_distinct_spawn_positions_have_no_overlap_warning() {
        let src = "\
prefab g
  glyph g
level a 2x2
  row ..
  row ..
  spawn g 0 0
  spawn g 1 1
";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(
            !vd.iter().any(|d| d.message.contains("multiple spawns")),
            "distinct positions must not warn; got: {vd:?}"
        );
    }

    #[test]
    fn test_three_spawns_same_cell_warn_twice() {
        let src = "\
prefab g
  glyph g
level a 2x2
  row ..
  row ..
  spawn g 1 0
  spawn g 1 0
  spawn g 1 0
";
        let (c, _) = parse(src);
        let vd = validate(&c);
        let warns = vd
            .iter()
            .filter(|d| d.message.contains("multiple spawns at (1,0)"))
            .count();
        assert_eq!(warns, 2, "second and third spawn each warn; got: {vd:?}");
    }

    // --- extends ---

    #[test]
    fn test_extends_undefined_base_is_error() {
        let src = "prefab boss extends ghost\n  glyph b\n";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(
            vd.iter()
                .any(|d| d.is_error() && d.message.contains("extends undefined prefab")),
            "expected missing-base error; got: {vd:?}"
        );
    }

    #[test]
    fn test_extends_cycle_is_error() {
        let src = "prefab a extends b\nprefab b extends a\n";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(
            vd.iter()
                .any(|d| d.is_error() && d.message.contains("extends cycle")),
            "expected cycle error; got: {vd:?}"
        );
    }

    #[test]
    fn test_extends_self_cycle_is_error() {
        let src = "prefab a extends a\n";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(vd
            .iter()
            .any(|d| d.is_error() && d.message.contains("cycle")));
    }

    #[test]
    fn test_extends_base_counts_as_used() {
        // enemy is never spawned directly, but boss extends it and boss spawns.
        let src = "\
prefab enemy
  glyph e
prefab boss extends enemy
  stat hp 50
level a 1x1
  row #
  spawn boss 0 0
";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(
            !vd.iter().any(|d| d.message.contains("never spawned")),
            "an extends base must not warn as unused; got: {vd:?}"
        );
    }

    #[test]
    fn test_valid_extends_bundle_is_loadable() {
        let src = "\
prefab enemy
  glyph e
  stat hp 10
prefab boss extends enemy
  stat hp 50
level a 1x1
  row #
  spawn boss 0 0
";
        let (c, pd) = parse(src);
        let vd = validate(&c);
        assert!(is_loadable(&pd, &vd), "diags: {pd:?} {vd:?}");
    }

    #[test]
    fn test_same_position_different_levels_do_not_warn() {
        // Occupancy is tracked per level, so (0,0) in two levels is fine.
        let src = "\
prefab g
  glyph g
level a 1x1
  row #
  spawn g 0 0
level b 1x1
  row #
  spawn g 0 0
";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(
            !vd.iter().any(|d| d.message.contains("multiple spawns")),
            "per-level occupancy must not collide across levels; got: {vd:?}"
        );
    }

    // --- did-you-mean suggestions ---------------------------------------

    #[test]
    fn test_undefined_spawn_suggests_nearest_prefab() {
        let src = "\
prefab ghost
  glyph g
level a 1x1
  row .
  spawn ghast 0 0
";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(
            vd.iter()
                .any(|d| d.message.contains("(did you mean 'ghost'?)")),
            "expected a suggestion for the typo'd prefab; got: {vd:?}"
        );
    }

    #[test]
    fn test_distant_name_gets_no_suggestion() {
        let src = "\
prefab goblin
  glyph g
level a 1x1
  row .
  spawn zz 0 0
";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(
            vd.iter()
                .any(|d| d.message.contains("undefined prefab 'zz'")),
            "error must still fire; got: {vd:?}"
        );
        assert!(
            !vd.iter().any(|d| d.message.contains("did you mean")),
            "no suggestion for an unrelated name; got: {vd:?}"
        );
    }

    #[test]
    fn test_extends_missing_base_suggests_nearest_prefab() {
        let src = "\
prefab enemy
  glyph e
prefab boss extends enem
  glyph b
";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(
            vd.iter()
                .any(|d| d.message.contains("(did you mean 'enemy'?)")),
            "expected a suggestion on the missing extends base; got: {vd:?}"
        );
    }

    #[test]
    fn test_suggestion_tie_resolves_to_first_declared() {
        // 'gat' is distance 1 from both 'gab' and 'gad'; declaration order
        // must break the tie so the diagnostic set is stable run to run.
        let src = "\
prefab gab
  glyph a
prefab gad
  glyph d
level a 1x1
  row .
  spawn gat 0 0
";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(
            vd.iter()
                .any(|d| d.message.contains("(did you mean 'gab'?)")),
            "earliest-declared candidate must win the tie; got: {vd:?}"
        );
    }

    #[test]
    fn test_edit_distance_multibyte_names() {
        // Char-level distance: 'ö' is one edit from 'o', not two.
        assert_eq!(edit_distance("snäke", "snake"), 1);
        assert_eq!(edit_distance("goblin", "goblin"), 0);
        assert_eq!(edit_distance("abc", "acb"), 2);
        assert_eq!(edit_distance("", "abc"), 3);
    }

    // --- spatial checks (isolated rooms / unreachable spawns) ------------
    //
    // The spatial rules key off the tile named `wall`; every fixture that
    // expects findings declares it. `spatial_messages` filters validate()'s
    // output down to the two spatial warning classes.

    fn spatial_messages(vd: &[Diagnostic]) -> Vec<String> {
        vd.iter()
            .map(|d| d.message.clone())
            .filter(|m| m.contains("isolated room") || m.contains("cannot be reached"))
            .collect()
    }

    #[test]
    fn test_isolated_room_warns_and_still_loads() {
        let src = "\
prefab hero
  glyph @
tile floor . #3A3A3A
tile wall # #6E7681
level a 5x5
  row #####
  row #...#
  row #####
  row #...#
  row #####
  spawn hero 1 1
";
        let (c, pd) = parse(src);
        let vd = validate(&c);
        assert_eq!(
            spatial_messages(&vd),
            vec![
                "level 'a': isolated room of 3 tile(s) at (1,3): no spawn inside and no path to any spawn"
                    .to_string()
            ],
            "the sealed lower room must be reported once; got: {vd:?}"
        );
        // A warning, not an error: the bundle still loads.
        assert!(vd.iter().all(|d| !d.is_error()), "got: {vd:?}");
        assert!(is_loadable(&pd, &vd), "diags: {pd:?} {vd:?}");
    }

    #[test]
    fn test_unreachable_spawn_region_warns() {
        let src = "\
prefab hero
  glyph @
prefab goblin
  glyph g
tile floor . #3A3A3A
tile wall # #6E7681
level a 7x3
  row #######
  row #..#..#
  row #######
  spawn hero 1 1
  spawn goblin 5 1
";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert_eq!(
            spatial_messages(&vd),
            vec![
                "level 'a': spawn(s) 'goblin' in a region of 2 tile(s) at (4,1) cannot be reached from spawn 'hero' at (1,1)"
                    .to_string()
            ],
            "got: {vd:?}"
        );
    }

    #[test]
    fn test_single_room_single_spawn_no_spatial_warnings() {
        let src = "\
prefab hero
  glyph @
tile floor . #3A3A3A
tile wall # #6E7681
level a 4x3
  row ####
  row #..#
  row ####
  spawn hero 1 1
";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(
            spatial_messages(&vd).is_empty(),
            "one room, one spawn, everything reachable: got: {vd:?}"
        );
    }

    #[test]
    fn test_open_room_many_spawns_no_spatial_warnings() {
        let src = "\
prefab hero
  glyph @
prefab goblin
  glyph g
prefab potion
  glyph !
tile floor . #3A3A3A
tile wall # #6E7681
level a 5x3
  row #####
  row #...#
  row #####
  spawn hero 1 1
  spawn goblin 2 1
  spawn potion 3 1
";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(
            spatial_messages(&vd).is_empty(),
            "all spawns share one region: got: {vd:?}"
        );
    }

    #[test]
    fn test_spatial_checks_stay_silent_without_a_wall_tile() {
        // Same sealed geometry as the isolated-room fixture, but the bundle
        // declares no `wall` tile: the format then states no solidity, so
        // the spatial checks must not guess.
        let src = "\
prefab hero
  glyph @
level a 5x5
  row #####
  row #...#
  row #####
  row #...#
  row #####
  spawn hero 1 1
";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(
            spatial_messages(&vd).is_empty(),
            "no wall tile declared: got: {vd:?}"
        );
    }

    #[test]
    fn test_spatial_checks_stay_silent_without_spawns() {
        // No spawn means no entrance to anchor reachability.
        let src = "\
prefab hero
  glyph @
tile floor . #3A3A3A
tile wall # #6E7681
level a 5x5
  row #####
  row #...#
  row #####
  row #...#
  row #####
";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(
            spatial_messages(&vd).is_empty(),
            "no spawns in the level: got: {vd:?}"
        );
    }

    #[test]
    fn test_bundled_dungeon_game_has_no_spatial_warnings() {
        // Verbatim copy of izanagi_kit/examples/dungeon.game — the shipped
        // sample is fully connected, so the spatial checks (and the whole
        // validator) must stay silent on it.
        let src = "\
// IZANAGI sample content — a tiny dungeon room
prefab hero
  glyph @
  color #00C4CC
  stat atk 5
  stat hp 20
prefab goblin
  glyph g
  color #F85149
  stat hp 8
  flag hostile
prefab goblin_king extends goblin
  glyph G
  stat hp 30
prefab potion
  glyph !
  color #3FB950
  flag item
tile floor . #3A3A3A
tile wall # #6E7681
level room 8x5
  row ########
  row #@.....#
  row #..G..!#
  row #....g.#
  row ########
  spawn hero 1 1
  spawn goblin_king 3 2
  spawn goblin 5 3
  spawn potion 6 2
";
        let (c, pd) = parse(src);
        let vd = validate(&c);
        assert!(
            vd.is_empty(),
            "the bundled sample must be clean; got: {vd:?}"
        );
        assert!(is_loadable(&pd, &vd), "diags: {pd:?} {vd:?}");
    }

    #[test]
    fn test_diagonal_corner_gap_does_not_connect() {
        // (1,1) and (2,2) touch diagonally, but both shared orthogonal
        // cells are wall — the kit's no-corner-cutting rule keeps the two
        // cells in separate regions.
        let src = "\
prefab hero
  glyph @
prefab goblin
  glyph g
tile floor . #3A3A3A
tile wall # #6E7681
level a 4x4
  row ####
  row #.##
  row ##.#
  row ####
  spawn hero 1 1
  spawn goblin 2 2
";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert_eq!(
            spatial_messages(&vd),
            vec![
                "level 'a': spawn(s) 'goblin' in a region of 1 tile(s) at (2,2) cannot be reached from spawn 'hero' at (1,1)"
                    .to_string()
            ],
            "got: {vd:?}"
        );
    }

    #[test]
    fn test_spawn_on_wall_glyph_cell_anchors_its_region() {
        // The goblin is authored onto a '#' cell. A spawn cell is standable
        // by definition, so the goblin forms its own one-cell region and is
        // reported unreachable — without the spawn-cell rule it would be
        // silently invisible to the check.
        let src = "\
prefab hero
  glyph @
prefab goblin
  glyph g
tile floor . #3A3A3A
tile wall # #6E7681
level a 3x3
  row ###
  row #.#
  row ###
  spawn hero 1 1
  spawn goblin 0 0
";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert_eq!(
            spatial_messages(&vd),
            vec![
                "level 'a': spawn(s) 'goblin' in a region of 1 tile(s) at (0,0) cannot be reached from spawn 'hero' at (1,1)"
                    .to_string()
            ],
            "got: {vd:?}"
        );
    }

    #[test]
    fn test_missing_rows_read_as_wall_without_panicking() {
        // Declared 3x3 but only one row authored: the missing rows read as
        // wall (and draw their own dimension error), sealing the goblin's
        // spawn cell into a one-cell region.
        let src = "\
prefab hero
  glyph @
prefab goblin
  glyph g
tile floor . #3A3A3A
tile wall # #6E7681
level a 3x3
  row ...
  spawn hero 1 0
  spawn goblin 1 2
";
        let (c, _) = parse(src);
        let vd = validate(&c);
        assert!(
            vd.iter().any(|d| d.message.contains("declared height")),
            "the malformed grid must still draw its dimension error; got: {vd:?}"
        );
        assert_eq!(
            spatial_messages(&vd),
            vec![
                "level 'a': spawn(s) 'goblin' in a region of 1 tile(s) at (1,2) cannot be reached from spawn 'hero' at (1,0)"
                    .to_string()
            ],
            "got: {vd:?}"
        );
    }

    #[test]
    fn test_spatial_warnings_deterministic_and_scan_ordered() {
        // Four extra regions: two one-cell pockets on the upper floor, a
        // two-cell pocket and a spawn-bearing vault on the lower floor.
        // Findings must come out in row-major order of each region's first
        // cell, identically on every run.
        let src = "\
prefab hero
  glyph @
prefab goblin
  glyph g
tile floor . #3A3A3A
tile wall # #6E7681
level a 9x5
  row #########
  row #...#.#.#
  row #########
  row #..#....#
  row #########
  spawn hero 1 1
  spawn goblin 5 3
";
        let (c, _) = parse(src);
        let expected = vec![
            "level 'a': isolated room of 1 tile(s) at (5,1): no spawn inside and no path to any spawn"
                .to_string(),
            "level 'a': isolated room of 1 tile(s) at (7,1): no spawn inside and no path to any spawn"
                .to_string(),
            "level 'a': isolated room of 2 tile(s) at (1,3): no spawn inside and no path to any spawn"
                .to_string(),
            "level 'a': spawn(s) 'goblin' in a region of 4 tile(s) at (4,3) cannot be reached from spawn 'hero' at (1,1)"
                .to_string(),
        ];
        let first: Vec<String> = validate(&c).iter().map(|d| d.message.clone()).collect();
        let second: Vec<String> = validate(&c).iter().map(|d| d.message.clone()).collect();
        assert_eq!(first, second, "same input must give the same findings");
        assert_eq!(spatial_messages(&validate(&c)), expected);
    }

    // --- oracle cross-check ----------------------------------------------
    //
    // An independent BFS re-implementation of the spatial model, written
    // from the rules rather than from check_level_spatial: regions are
    // flood fills over passable cells, movement is 8-directional with no
    // corner cutting, spawn cells are always passable, cells the rows do
    // not cover read as wall. The validator's spatial findings must match
    // the oracle's region table exactly — kinds, sizes, first cells,
    // spawn lists, order — on generated levels.

    struct OracleRegion {
        size: usize,
        first: (u32, u32),
        spawns: Vec<String>,
    }

    /// Regions of `level` in row-major order of first cell, plus the index
    /// of the region holding the entrance (first in-bounds spawn).
    fn oracle_regions(level: &Level, wall_glyph: char) -> (Vec<OracleRegion>, Option<usize>) {
        let w = level.width as i32;
        let h = level.height as i32;
        let glyph = |x: i32, y: i32| -> char {
            level
                .rows
                .get(y as usize)
                .and_then(|r| r.chars().nth(x as usize))
                .unwrap_or(wall_glyph)
        };
        let in_bounds: Vec<&Spawn> = level
            .spawns
            .iter()
            .filter(|s| s.x < level.width && s.y < level.height)
            .collect();
        let spawn_cells: HashSet<(i32, i32)> =
            in_bounds.iter().map(|s| (s.x as i32, s.y as i32)).collect();
        let passable = |x: i32, y: i32| -> bool {
            x >= 0
                && y >= 0
                && x < w
                && y < h
                && (glyph(x, y) != wall_glyph || spawn_cells.contains(&(x, y)))
        };
        let mut label = vec![vec![usize::MAX; w as usize]; h as usize];
        let mut regions: Vec<OracleRegion> = Vec::new();
        for sy in 0..h {
            for sx in 0..w {
                if !passable(sx, sy) || label[sy as usize][sx as usize] != usize::MAX {
                    continue;
                }
                let id = regions.len();
                let mut size = 0usize;
                let mut queue = std::collections::VecDeque::new();
                label[sy as usize][sx as usize] = id;
                queue.push_back((sx, sy));
                while let Some((cx, cy)) = queue.pop_front() {
                    size += 1;
                    for (dx, dy) in [
                        (1, 0),
                        (1, 1),
                        (0, 1),
                        (-1, 1),
                        (-1, 0),
                        (-1, -1),
                        (0, -1),
                        (1, -1),
                    ] {
                        let (nx, ny) = (cx + dx, cy + dy);
                        if !passable(nx, ny) || label[ny as usize][nx as usize] != usize::MAX {
                            continue;
                        }
                        if dx != 0 && dy != 0 && (!passable(cx + dx, cy) || !passable(cx, cy + dy))
                        {
                            continue;
                        }
                        label[ny as usize][nx as usize] = id;
                        queue.push_back((nx, ny));
                    }
                }
                regions.push(OracleRegion {
                    size,
                    first: (sx as u32, sy as u32),
                    spawns: Vec::new(),
                });
            }
        }
        let mut entrance_region = None;
        for (i, s) in in_bounds.iter().enumerate() {
            let id = label[s.y as usize][s.x as usize];
            if id != usize::MAX {
                regions[id].spawns.push(s.prefab.clone());
                if i == 0 {
                    entrance_region = Some(id);
                }
            }
        }
        (regions, entrance_region)
    }

    fn oracle_messages(level: &Level, wall_glyph: char) -> Vec<String> {
        let (regions, entrance_region) = oracle_regions(level, wall_glyph);
        let entrance = level
            .spawns
            .iter()
            .find(|s| s.x < level.width && s.y < level.height);
        let mut out = Vec::new();
        if let (Some(entrance), Some(entrance_region)) = (entrance, entrance_region) {
            for (i, region) in regions.iter().enumerate() {
                if i == entrance_region {
                    continue;
                }
                let (fx, fy) = region.first;
                if region.spawns.is_empty() {
                    out.push(format!(
                        "level '{}': isolated room of {} tile(s) at ({fx},{fy}): no spawn inside and no path to any spawn",
                        level.name, region.size
                    ));
                } else {
                    let quoted: Vec<String> =
                        region.spawns.iter().map(|n| format!("'{n}'")).collect();
                    out.push(format!(
                        "level '{}': spawn(s) {} in a region of {} tile(s) at ({fx},{fy}) cannot be reached from spawn '{}' at ({},{})",
                        level.name,
                        quoted.join(", "),
                        region.size,
                        entrance.prefab,
                        entrance.x,
                        entrance.y
                    ));
                }
            }
        }
        out
    }

    #[test]
    fn test_spatial_findings_match_bfs_oracle_on_generated_levels() {
        use crate::content::{Color, Prefab, Tile};
        // xorshift64* — deterministic across runs and platforms.
        let mut state = 0x9E37_79B9_7F4A_7C15u64;
        let mut next = move || {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            (state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
        };
        for case in 0..40 {
            let (w, h) = (10u32, 8u32);
            let mut rows = Vec::new();
            for _y in 0..h {
                let mut row = String::new();
                for _x in 0..w {
                    row.push(if next() % 100 < 32 { '#' } else { '.' });
                }
                rows.push(row);
            }
            let names = ["hero", "goblin", "potion"];
            let mut spawns = Vec::new();
            for name in names {
                spawns.push(Spawn {
                    prefab: name.to_string(),
                    x: next() % w,
                    y: next() % h,
                });
            }
            let mut content = Content::default();
            for name in names {
                content.prefabs.push(Prefab::new(name.into()));
            }
            content.tiles.push(Tile {
                name: "floor".into(),
                glyph: '.',
                color: Color::rgb(0, 0, 0),
            });
            content.tiles.push(Tile {
                name: "wall".into(),
                glyph: '#',
                color: Color::rgb(0, 0, 0),
            });
            content.levels.push(Level {
                name: "gen".into(),
                width: w,
                height: h,
                rows,
                spawns,
            });
            let vd = validate(&content);
            assert_eq!(
                spatial_messages(&vd),
                oracle_messages(&content.levels[0], '#'),
                "case {case}: validator disagrees with the BFS oracle"
            );
        }
    }
}
