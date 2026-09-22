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

use crate::content::{Content, Diagnostic, ExtendsError};
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

    diags
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
}
