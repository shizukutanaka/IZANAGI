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
}
