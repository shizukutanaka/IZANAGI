//! Structure-aware round-trip fuzzing.
//!
//! Per arXiv:2604.01442 / arXiv:2603.00311: byte-level mutation mostly produces
//! inputs rejected by shallow validation, never reaching deep logic. A
//! generator that emits *structurally valid* content instead exercises the full
//! parse/serialize path. Here the generator builds random-but-well-formed
//! `Content`, and the oracle is the round-trip property:
//!
//!     parse(serialize(c)) ≅ c
//!
//! Deterministic via the kit's seeded RNG, so any failure reproduces in CI from
//! its seed.

use izanagi_kit::content::{Color, Content, Level, Prefab, Spawn, Tile};
use izanagi_kit::{content_eq, parse, serialize, SplitMix64};
use std::collections::BTreeMap;

/// Builds a well-formed `Content` from a seed. Names avoid whitespace and stay
/// within the parser's length limits, so the result is always serializable and
/// re-parseable.
fn gen_content(rng: &mut SplitMix64) -> Content {
    let mut c = Content::default();

    let n_prefabs = rng.below(4) as usize;
    let glyphs = b"@g#.r!woxYZ";
    let mut prefab_names = Vec::new();

    for i in 0..n_prefabs {
        let name = format!("p{i}");
        prefab_names.push(name.clone());
        let mut p = Prefab::new(name);
        p.glyph = glyphs[rng.below(glyphs.len() as u32) as usize] as char;
        p.color = gen_color(rng);
        let n_stats = rng.below(4) as usize;
        let mut stats = BTreeMap::new();
        for s in 0..n_stats {
            stats.insert(format!("s{s}"), rng.below(200) as i32 - 100);
        }
        p.stats = stats;
        let n_flags = rng.below(3) as usize;
        for f in 0..n_flags {
            p.flags.push(format!("f{f}"));
        }
        c.prefabs.push(p);
    }

    let n_tiles = rng.below(3) as usize;
    for i in 0..n_tiles {
        c.tiles.push(Tile {
            name: format!("t{i}"),
            glyph: glyphs[rng.below(glyphs.len() as u32) as usize] as char,
            color: gen_color(rng),
        });
    }

    let n_levels = rng.below(3) as usize;
    for i in 0..n_levels {
        let w = 1 + rng.below(8);
        let h = 1 + rng.below(6);
        let mut rows = Vec::new();
        for _ in 0..h {
            // Row cells must be exactly `w` non-space glyphs.
            let mut row = String::new();
            for _ in 0..w {
                row.push(glyphs[rng.below(glyphs.len() as u32) as usize] as char);
            }
            rows.push(row);
        }
        let mut spawns = Vec::new();
        if !prefab_names.is_empty() {
            let n_spawns = rng.below(4) as usize;
            for _ in 0..n_spawns {
                let pname = &prefab_names[rng.below(prefab_names.len() as u32) as usize];
                spawns.push(Spawn {
                    prefab: pname.clone(),
                    x: rng.below(w),
                    y: rng.below(h),
                });
            }
        }
        c.levels.push(Level {
            name: format!("l{i}"),
            width: w,
            height: h,
            rows,
            spawns,
        });
    }

    c
}

fn gen_color(rng: &mut SplitMix64) -> Color {
    Color {
        r: rng.below(256) as u8,
        g: rng.below(256) as u8,
        b: rng.below(256) as u8,
    }
}

#[test]
fn test_generated_content_roundtrips() {
    let mut rng = SplitMix64::new(0x5EED1234);
    for iter in 0..3000 {
        let original = gen_content(&mut rng);
        let text = serialize(&original);
        let (reparsed, diags) = parse(&text);
        assert!(
            diags.iter().all(|d| !d.is_error()),
            "iter {iter}: serialized content failed to re-parse: {diags:?}\n---\n{text}"
        );
        assert!(
            content_eq(&original, &reparsed),
            "iter {iter}: round-trip changed meaning\n---\n{text}"
        );
    }
}

#[test]
fn test_serialize_is_canonical_under_fuzz() {
    // serialize must be a fixed point through parse: re-serializing the
    // re-parsed content yields byte-identical text.
    let mut rng = SplitMix64::new(0xA11CE);
    for iter in 0..2000 {
        let original = gen_content(&mut rng);
        let t1 = serialize(&original);
        let (reparsed, _) = parse(&t1);
        let t2 = serialize(&reparsed);
        assert_eq!(t1, t2, "iter {iter}: serialization not canonical");
    }
}
