//! `Content` -> `.game` text serializer, the inverse of [`crate::parser`].
//!
//! Having a serializer enables the strongest correctness check for the pipeline:
//! the round-trip property `parse(serialize(c)) ≅ c`. Per the property-based /
//! structure-aware fuzzing literature (arXiv:2604.01442, arXiv:2603.00311), a
//! generator that emits structurally valid values and a round-trip oracle find
//! semantic bugs that byte-level mutation cannot reach.
//!
//! Output is canonical: prefabs, then tiles, then levels, each field in a fixed
//! order. Stats are already ordered (`BTreeMap`), so the text is deterministic —
//! the same `Content` always serializes to the same bytes.

use crate::content::{Content, Level, Prefab};
use std::fmt::Write;

/// Serializes a bundle into canonical `.game` text that the parser accepts.
pub fn serialize(content: &Content) -> String {
    let mut out = String::new();

    for prefab in &content.prefabs {
        write_prefab(&mut out, prefab);
    }
    for tile in &content.tiles {
        writeln!(
            out,
            "tile {} {} {}",
            tile.name,
            tile.glyph,
            tile.color.to_hex()
        )
        .ok();
    }
    for level in &content.levels {
        write_level(&mut out, level);
    }
    out
}

fn write_prefab(out: &mut String, p: &Prefab) {
    writeln!(out, "prefab {}", p.name).ok();
    writeln!(out, "  glyph {}", p.glyph).ok();
    writeln!(out, "  color {}", p.color.to_hex()).ok();
    for (key, value) in &p.stats {
        writeln!(out, "  stat {key} {value}").ok();
    }
    for flag in &p.flags {
        writeln!(out, "  flag {flag}").ok();
    }
}

fn write_level(out: &mut String, lv: &Level) {
    writeln!(out, "level {} {}x{}", lv.name, lv.width, lv.height).ok();
    for row in &lv.rows {
        writeln!(out, "  row {row}").ok();
    }
    for s in &lv.spawns {
        writeln!(out, "  spawn {} {} {}", s.prefab, s.x, s.y).ok();
    }
}

/// Semantic equality for round-trip checking. Compares the authored meaning,
/// not incidental representation. (Parsing always sets a prefab glyph/color, so
/// a freshly built `Prefab` with defaults round-trips to the same values.)
pub fn content_eq(a: &Content, b: &Content) -> bool {
    prefabs_eq(a, b) && tiles_eq(a, b) && levels_eq(a, b)
}

/// Returns a human-readable description of the first semantic difference
/// between `a` and `b`, or `None` when they are equal under [`content_eq`].
/// Use this to diagnose round-trip failures: instead of knowing only that two
/// `Content` values differ, callers learn *where* the first divergence is.
pub fn first_diff(a: &Content, b: &Content) -> Option<String> {
    if a.prefabs.len() != b.prefabs.len() {
        return Some(format!(
            "prefab count {} vs {}",
            a.prefabs.len(),
            b.prefabs.len()
        ));
    }
    if a.tiles.len() != b.tiles.len() {
        return Some(format!("tile count {} vs {}", a.tiles.len(), b.tiles.len()));
    }
    if a.levels.len() != b.levels.len() {
        return Some(format!(
            "level count {} vs {}",
            a.levels.len(),
            b.levels.len()
        ));
    }
    for (i, (pa, pb)) in a.prefabs.iter().zip(&b.prefabs).enumerate() {
        if pa.name != pb.name {
            return Some(format!("prefab[{i}].name {:?} vs {:?}", pa.name, pb.name));
        }
        if pa.glyph != pb.glyph {
            return Some(format!(
                "prefab[{i}].glyph {:?} vs {:?}",
                pa.glyph, pb.glyph
            ));
        }
        if pa.color != pb.color {
            return Some(format!(
                "prefab[{i}].color {:?} vs {:?}",
                pa.color, pb.color
            ));
        }
        if pa.stats != pb.stats {
            return Some(format!("prefab[{i}].stats differ"));
        }
        if pa.flags != pb.flags {
            return Some(format!("prefab[{i}].flags differ"));
        }
    }
    for (i, (la, lb)) in a.levels.iter().zip(&b.levels).enumerate() {
        if la.name != lb.name {
            return Some(format!("level[{i}].name {:?} vs {:?}", la.name, lb.name));
        }
        if la.rows != lb.rows {
            return Some(format!("level[{i}].rows differ"));
        }
    }
    None
}

/// Collect **all** semantic differences between `a` and `b` as human-readable
/// strings, in the order they are discovered. Returns an empty `Vec` when
/// `content_eq(a, b)` is `true`. Unlike [`first_diff`], which stops at the
/// first divergence, `diff` provides a complete picture — useful for "show
/// all errors after a failed round-trip" debugging and structured test output.
pub fn diff(a: &Content, b: &Content) -> Vec<String> {
    let mut out = Vec::new();
    if a.prefabs.len() != b.prefabs.len() {
        out.push(format!(
            "prefab count {} vs {}",
            a.prefabs.len(),
            b.prefabs.len()
        ));
    }
    if a.tiles.len() != b.tiles.len() {
        out.push(format!("tile count {} vs {}", a.tiles.len(), b.tiles.len()));
    }
    if a.levels.len() != b.levels.len() {
        out.push(format!(
            "level count {} vs {}",
            a.levels.len(),
            b.levels.len()
        ));
    }
    for (i, (pa, pb)) in a.prefabs.iter().zip(&b.prefabs).enumerate() {
        if pa.name != pb.name {
            out.push(format!("prefab[{i}].name {:?} vs {:?}", pa.name, pb.name));
        }
        if pa.glyph != pb.glyph {
            out.push(format!(
                "prefab[{i}].glyph {:?} vs {:?}",
                pa.glyph, pb.glyph
            ));
        }
        if pa.color != pb.color {
            out.push(format!(
                "prefab[{i}].color {:?} vs {:?}",
                pa.color, pb.color
            ));
        }
        if pa.stats != pb.stats {
            out.push(format!("prefab[{i}].stats differ"));
        }
        if pa.flags != pb.flags {
            out.push(format!("prefab[{i}].flags differ"));
        }
    }
    for (i, (ta, tb)) in a.tiles.iter().zip(&b.tiles).enumerate() {
        if ta.name != tb.name {
            out.push(format!("tile[{i}].name {:?} vs {:?}", ta.name, tb.name));
        }
        if ta.glyph != tb.glyph {
            out.push(format!("tile[{i}].glyph {:?} vs {:?}", ta.glyph, tb.glyph));
        }
        if ta.color != tb.color {
            out.push(format!("tile[{i}].color {:?} vs {:?}", ta.color, tb.color));
        }
    }
    for (i, (la, lb)) in a.levels.iter().zip(&b.levels).enumerate() {
        if la.name != lb.name {
            out.push(format!("level[{i}].name {:?} vs {:?}", la.name, lb.name));
        }
        if la.rows != lb.rows {
            out.push(format!("level[{i}].rows differ"));
        }
    }
    out
}

fn prefabs_eq(a: &Content, b: &Content) -> bool {
    if a.prefabs.len() != b.prefabs.len() {
        return false;
    }
    a.prefabs.iter().zip(&b.prefabs).all(|(x, y)| {
        x.name == y.name
            && x.glyph == y.glyph
            && x.color == y.color
            && x.stats == y.stats
            && x.flags == y.flags
    })
}

fn tiles_eq(a: &Content, b: &Content) -> bool {
    if a.tiles.len() != b.tiles.len() {
        return false;
    }
    a.tiles
        .iter()
        .zip(&b.tiles)
        .all(|(x, y)| x.name == y.name && x.glyph == y.glyph && x.color == y.color)
}

fn levels_eq(a: &Content, b: &Content) -> bool {
    if a.levels.len() != b.levels.len() {
        return false;
    }
    a.levels.iter().zip(&b.levels).all(|(x, y)| {
        x.name == y.name
            && x.width == y.width
            && x.height == y.height
            && x.rows == y.rows
            && x.spawns.len() == y.spawns.len()
            && x.spawns
                .iter()
                .zip(&y.spawns)
                .all(|(s, t)| s.prefab == t.prefab && s.x == t.x && s.y == t.y)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    const SAMPLE: &str = "\
prefab goblin
  glyph g
  color #F85149
  stat atk 3
  stat hp 10
  flag hostile
tile floor . #3A3A3A
level cave 5x3
  row #####
  row #.g.#
  row #####
  spawn goblin 2 1
";

    #[test]
    fn test_roundtrip_sample() {
        let (c1, d1) = parse(SAMPLE);
        assert!(d1.iter().all(|x| !x.is_error()));
        let text = serialize(&c1);
        let (c2, d2) = parse(&text);
        assert!(
            d2.iter().all(|x| !x.is_error()),
            "serialized text must re-parse: {d2:?}"
        );
        assert!(content_eq(&c1, &c2), "round-trip must preserve meaning");
    }

    #[test]
    fn test_serialize_is_idempotent() {
        let (c1, _) = parse(SAMPLE);
        let t1 = serialize(&c1);
        let (c2, _) = parse(&t1);
        let t2 = serialize(&c2);
        assert_eq!(
            t1, t2,
            "serialize∘parse∘serialize == serialize (canonical form)"
        );
    }

    #[test]
    fn test_empty_content_roundtrips() {
        let c = Content::default();
        let (c2, _) = parse(&serialize(&c));
        assert!(content_eq(&c, &c2));
    }

    #[test]
    fn test_content_eq_detects_difference() {
        let (a, _) = parse(SAMPLE);
        let (b, _) = parse("prefab x\n  glyph x\n");
        assert!(!content_eq(&a, &b));
    }

    #[test]
    fn test_first_diff_equal_content_is_none() {
        let (c1, _) = parse(SAMPLE);
        let (c2, _) = parse(SAMPLE);
        assert!(first_diff(&c1, &c2).is_none());
    }

    #[test]
    fn test_first_diff_detects_prefab_count_mismatch() {
        let (a, _) = parse(SAMPLE);
        let (b, _) = parse("tile floor . #3A3A3A\n");
        let diff = first_diff(&a, &b);
        assert!(diff.is_some());
        assert!(
            diff.as_ref().unwrap().contains("prefab count"),
            "got: {diff:?}"
        );
    }

    #[test]
    fn test_first_diff_detects_level_row_change() {
        let (a, _) = parse(SAMPLE);
        let modified = SAMPLE.replace("  row #.g.#", "  row #####");
        let (b, _) = parse(&modified);
        let d = first_diff(&a, &b);
        assert!(d.is_some());
        assert!(d.as_ref().unwrap().contains("rows"), "got: {d:?}");
    }

    #[test]
    fn test_diff_equal_content_is_empty() {
        let (c1, _) = parse(SAMPLE);
        let (c2, _) = parse(SAMPLE);
        assert!(diff(&c1, &c2).is_empty());
    }

    #[test]
    fn test_diff_collects_count_mismatch() {
        let (a, _) = parse(SAMPLE);
        let (b, _) = parse("tile floor . #3A3A3A\n");
        let ds = diff(&a, &b);
        assert!(!ds.is_empty(), "expected at least one diff");
        assert!(ds.iter().any(|s| s.contains("prefab count")), "got: {ds:?}");
    }

    #[test]
    fn test_diff_reports_row_change() {
        let (a, _) = parse(SAMPLE);
        let modified = SAMPLE.replace("  row #.g.#", "  row #####");
        let (b, _) = parse(&modified);
        let ds = diff(&a, &b);
        assert!(ds.iter().any(|s| s.contains("rows")), "got: {ds:?}");
    }
}
