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

use crate::content::{Content, Diagnostic};
use std::collections::HashSet;

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

    let known_prefabs: HashSet<&str> = content.prefabs.iter().map(|p| p.name.as_str()).collect();

    // Unused-prefab warnings: defined but never spawned in any level.
    let used_prefabs: HashSet<&str> = content
        .levels
        .iter()
        .flat_map(|l| l.spawns.iter())
        .map(|s| s.prefab.as_str())
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
                diags.push(Diagnostic::error(
                    0,
                    format!(
                        "level '{}': spawn references undefined prefab '{}'",
                        level.name, spawn.prefab
                    ),
                ));
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
}
