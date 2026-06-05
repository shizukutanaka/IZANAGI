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

    let known_prefabs: HashSet<&str> = content.prefabs.iter().map(|p| p.name.as_str()).collect();

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
}
