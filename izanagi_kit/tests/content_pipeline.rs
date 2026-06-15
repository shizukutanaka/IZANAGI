//! End-to-end content pipeline test + parser robustness (panic-free) check.

use izanagi_kit::{is_loadable, load_level, parse, validate, SplitMix64};

#[test]
fn test_full_pipeline_valid_file() {
    let src = "\
// a tiny dungeon
prefab hero
  glyph @
  color #00C4CC
  stat hp 20
prefab goblin
  glyph g
  color #f85149
  stat hp 8
  flag hostile
tile floor . #3A3A3A
tile wall # #6E7681
level room 6x4
  row ######
  row #@..g#
  row #...g#
  row ######
  spawn hero 1 1
  spawn goblin 4 1
  spawn goblin 4 2
";
    let (content, pd) = parse(src);
    let vd = validate(&content);
    assert!(is_loadable(&pd, &vd), "parse={pd:?} validate={vd:?}");

    let world = load_level(&content, "room").unwrap();
    assert_eq!(world.entity_count(), 3, "hero + two goblins");
}

#[test]
fn test_pipeline_reports_but_survives_broken_file() {
    let src = "\
prefab a
  glyph @
tile floor . #3A3A3A
glyph x
level x 3x2
  row ...
  row ...
  spawn ghost 9 9
wobble nonsense
";
    let (content, pd) = parse(src);
    let vd = validate(&content);
    // Must NOT be loadable, and must surface multiple distinct problems.
    assert!(!is_loadable(&pd, &vd));
    let all: Vec<String> = pd
        .iter()
        .chain(vd.iter())
        .map(|d| d.message.clone())
        .collect();
    assert!(all.iter().any(|m| m.contains("outside a prefab block"))); // glyph after tile
    assert!(all.iter().any(|m| m.contains("undefined prefab"))); // ghost
    assert!(all.iter().any(|m| m.contains("is outside"))); // 9,9 OOB
    assert!(all.iter().any(|m| m.contains("unknown"))); // wobble
}

/// Feed the parser pseudo-random lines built from the grammar's alphabet and
/// assert it never panics and always terminates. Deterministic via the kit's
/// seeded RNG — reproducible in CI. (cargo-fuzz proper needs nightly; this is
/// the zero-dependency stand-in.)
#[test]
fn test_parser_never_panics_on_garbage() {
    let tokens = [
        "prefab",
        "tile",
        "level",
        "glyph",
        "color",
        "stat",
        "flag",
        "row",
        "spawn",
        "#00C4CC",
        "#ZZZ",
        "#aéABC", // 7 bytes, multi-byte char: regression for parse_color panic
        "#é0000",
        "g",
        "@@",
        "5x3",
        "999999x1",
        "-1",
        "2",
        "//c",
        "",
        "   ",
        "name",
        "0x0",
        "hp",
        "10",
        "\u{1F600}",
        "verylongtokenverylongtokenverylongtoken",
    ];
    let mut rng = SplitMix64::new(0xFEE5u64.wrapping_mul(3));
    for _ in 0..5000 {
        let line_len = rng.below(6) as usize;
        let mut line = String::new();
        for _ in 0..line_len {
            line.push_str(tokens[rng.below(tokens.len() as u32) as usize]);
            line.push(' ');
        }
        let mut src = String::new();
        let rows = rng.below(5);
        for _ in 0..rows {
            src.push_str(&line);
            src.push('\n');
        }
        // The contract: no panic, returns. Result content is irrelevant here.
        let (c, _d) = parse(&src);
        let _ = validate(&c);
    }
}

/// Complements the grammar-token fuzz with raw character diversity: arbitrary
/// Unicode scalar values (many multi-byte), control characters, whitespace
/// variants, comment marks, and structural chars — interleaved without the
/// neat space separators above. This stresses the tokenizer's char-boundary
/// slicing (`&raw[sb..byte_idx]`) and per-character column arithmetic far more
/// adversarially than a fixed vocabulary can. Deterministic via the seeded RNG.
#[test]
fn test_parser_never_panics_on_random_unicode() {
    // Structural characters that drive the parser into its various branches:
    // line breaks, the comment marker, dimension `x`, sign, digits, hex `#`,
    // and tabs/spaces for column counting.
    let structural = [
        '\n', ' ', '\t', '\r', '#', 'x', '/', '-', '0', '9', 'g', '@', ':',
    ];
    let mut rng = SplitMix64::new(0x1A2A3A4A5A6A7A8A);
    for _ in 0..4000 {
        let len = rng.below(48) as usize;
        let mut src = String::new();
        for _ in 0..len {
            if rng.below(3) == 0 {
                // ~1/3 structural to actually reach keyword/arg parsing paths.
                src.push(structural[rng.below(structural.len() as u32) as usize]);
            } else {
                // Any Unicode scalar value (from_u32 yields None for surrogates,
                // which we simply skip) — exercises multi-byte boundary slicing.
                let cp = rng.below(0x110000);
                if let Some(ch) = char::from_u32(cp) {
                    src.push(ch);
                }
            }
        }
        // Contract: parse + validate must terminate without panicking, whatever
        // the bytes. The produced content/diagnostics are irrelevant here.
        let (c, _d) = parse(&src);
        let _ = validate(&c);
    }
}
