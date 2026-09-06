//! `SPEC.md` §9.1 gives a formal EBNF grammar for the `.game` format and says
//! it "corresponds 1:1 with the implementation in `src/parser.rs`". This file
//! is what makes that sentence true rather than aspirational.
//!
//! Three claims are checked:
//!
//! 1. **The keyword sets agree.** Adding a statement to the parser without
//!    adding it to the grammar makes the specification quietly wrong, and the
//!    grammar is the only document a consumer writing `.game` files can read.
//! 2. **The numeric bounds agree.** The grammar states a 1024-byte line limit,
//!    a 32-character name limit and a 1..=256 grid dimension; the parser
//!    declares those as constants. Changing one side alone is a lie.
//! 3. **Hostile input produces diagnostics, not panics.** The grammar's
//!    lexical rules end with a promise: unknown keywords, wrong arity and
//!    failed parses all "emit a diagnostic and skip the line", collecting
//!    later diagnostics rather than stopping.
//!
//! The third is the one with real risk behind it. `gamec` is documented as a
//! CI content gate — its entire job is being pointed at files its operator did
//! not write — so `parse` is this workspace's only untrusted-input boundary,
//! and it does 25 slicing and indexing operations. The existing fuzzer
//! (`roundtrip_fuzz.rs`) generates *well-formed* content on purpose, so the
//! hostile path had never been exercised. It turns out to be sound; this keeps
//! it that way.

use izanagi_kit::{content_eq, parse, validate, SplitMix64};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

/// Every keyword the `.game` format has. Named once, then checked against both
/// the grammar and the parser, so the two can never drift apart silently.
const KEYWORDS: [&str; 9] = [
    "prefab", "tile", "level", "glyph", "color", "stat", "flag", "row", "spawn",
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crate directory has a parent")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    fs::read_to_string(repo_root().join(rel)).unwrap_or_else(|e| panic!("reading {rel}: {e}"))
}

/// The string literals used as arms of `parser.rs`'s `match keyword` block —
/// that is, exactly the words the parser dispatches on.
///
/// Brace depth is tracked because the arms contain nested `match` expressions
/// of their own. The first draft of this scanner stopped at the first `_ =>`
/// it saw, which belongs to a nested match inside the `"prefab"` arm, and so
/// reported three keywords instead of nine — a scanner that silently
/// under-reports is worse than none, since the equality check would then pass
/// as soon as someone shortened the KEYWORDS list to match.
fn parser_dispatch_keywords(src: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let Some(start) = src.find("match keyword {") else {
        return out;
    };
    // Depth 1 is immediately inside `match keyword { ... }`; anything deeper
    // belongs to an arm's body.
    let mut depth: i32 = 1;
    for line in src[start..].lines().skip(1) {
        let trimmed = line.trim();
        let opens = line.matches('{').count() as i32;
        let closes = line.matches('}').count() as i32;
        if depth == 1 {
            // The catch-all arm of *this* match ends the dispatch.
            if trimmed.starts_with("_ =>") {
                break;
            }
            // Arms look like `"prefab" => …` or `"glyph" | "color" | … => …`.
            if trimmed.starts_with('"') {
                if let Some(head) = trimmed
                    .split("=>")
                    .next()
                    .filter(|_| trimmed.contains("=>"))
                {
                    for piece in head.split('|') {
                        let word = piece.trim().trim_matches('"');
                        if !word.is_empty() && word.chars().all(|c| c.is_ascii_lowercase()) {
                            out.insert(word.to_string());
                        }
                    }
                }
            }
        }
        depth += opens - closes;
        if depth <= 0 {
            break;
        }
    }
    out
}

/// The text of SPEC.md's EBNF code block.
fn grammar_block() -> String {
    let spec = read("izanagi_kit/SPEC.md");
    let start = spec
        .find("```ebnf")
        .expect("SPEC.md must contain the EBNF grammar");
    let rest = &spec[start + 7..];
    let end = rest.find("```").expect("the EBNF block must be closed");
    rest[..end].to_string()
}

#[test]
fn the_parser_dispatches_on_exactly_the_keywords_the_grammar_declares() {
    let parser = parser_dispatch_keywords(&read("izanagi_kit/src/parser.rs"));
    let declared: BTreeSet<String> = KEYWORDS.iter().map(|k| k.to_string()).collect();

    assert_eq!(
        parser, declared,
        "the parser's `match keyword` arms and this file's KEYWORDS list \
         disagree. A statement added to the parser has to be added to SPEC.md \
         §9.1's grammar too — that grammar is the only description a consumer \
         writing .game files can read."
    );

    let grammar = grammar_block();
    for kw in KEYWORDS {
        assert!(
            grammar.contains(&format!("\"{kw}\"")),
            "SPEC.md's EBNF never mentions the terminal \"{kw}\", which the \
             parser accepts"
        );
    }
}

#[test]
fn the_bounds_the_grammar_states_are_the_constants_the_parser_declares() {
    let parser = read("izanagi_kit/src/parser.rs");
    let spec = read("izanagi_kit/SPEC.md");
    for (constant, value, spec_phrase) in [
        ("MAX_LINE_LEN", "1024", "行長 ≤ 1024 バイト"),
        ("MAX_NAME_LEN", "32", "1..=32 文字"),
        ("MAX_DIM", "256", "`1..=256`"),
    ] {
        assert!(
            parser.contains(&format!("const {constant}: usize = {value};"))
                || parser.contains(&format!("const {constant}: u32 = {value};")),
            "parser.rs no longer declares {constant} = {value}; SPEC.md §9.1 \
             still states it"
        );
        assert!(
            spec.contains(spec_phrase),
            "SPEC.md §9.1 no longer states `{spec_phrase}`, which parser.rs \
             enforces as {constant} = {value}. Change both or neither."
        );
    }
}

/// Inputs a consumer of `gamec` could plausibly be handed, none of them
/// well-formed. Each is paired with whether the grammar's rules require a
/// diagnostic — `false` where the construct is legal under the grammar even
/// though it looks hostile, because asserting otherwise would be asserting
/// something the specification does not say.
const HOSTILE: [(&str, bool); 22] = [
    ("a line longer than the limit", true),
    ("dimension that overflows u32", true),
    ("zero dimension", true),
    ("integer that overflows i32", true),
    ("negative spawn coordinate", true),
    ("glyph with no character", true),
    ("glyph with two characters", true),
    ("glyph that is an emoji sequence", true),
    ("glyph with a combining mark", true),
    ("colour whose hex digits are multi-byte", true),
    ("colour with too few digits", true),
    ("colour with too many digits", true),
    ("name past the length limit", true),
    ("prefab child with no open block", true),
    ("spawn coordinate that overflows", true),
    ("every keyword with no arguments", true),
    ("unknown keywords", true),
    // Legal under the grammar: a name is any non-whitespace token, and NUL is
    // not whitespace. Nothing in §9.1 excludes it.
    ("a name containing NUL", false),
    ("i32::MIN as a stat value", false),
    ("comments and blank lines only", false),
    ("tabs used as the separator", false),
    ("the empty file", false),
];

fn hostile_source(case: &str) -> String {
    match case {
        "a line longer than the limit" => format!("prefab {}", "a".repeat(2000)),
        "dimension that overflows u32" => "level big 999999999999x1".into(),
        "zero dimension" => "level z 0x0".into(),
        "integer that overflows i32" => "prefab p\nstat hp 99999999999999999999".into(),
        "negative spawn coordinate" => "level l 4x4\nspawn g -1 -1".into(),
        "glyph with no character" => "prefab p\nglyph ".into(),
        "glyph with two characters" => "prefab p\nglyph ab".into(),
        "glyph that is an emoji sequence" => "prefab p\nglyph \u{1F468}\u{200D}\u{1F4BB}".into(),
        "glyph with a combining mark" => "prefab p\nglyph e\u{0301}".into(),
        "colour whose hex digits are multi-byte" => "tile t . #\u{00E9}\u{00E9}\u{00E9}".into(),
        "colour with too few digits" => "tile t . #FFF".into(),
        "colour with too many digits" => "tile t . #FFFFFFFF".into(),
        "name past the length limit" => format!("prefab {}", "n".repeat(200)),
        "prefab child with no open block" => "glyph @\ncolor #FFFFFF\nrow ....".into(),
        "spawn coordinate that overflows" => "level l 4x4\nspawn g 99999999999 1".into(),
        "every keyword with no arguments" => KEYWORDS.join("\n"),
        "unknown keywords" => "wobble\nfoo bar\n// c\n\n   \n".into(),
        "a name containing NUL" => "prefab p\0q\nglyph \0".into(),
        "i32::MIN as a stat value" => "prefab p\nstat hp -2147483648".into(),
        "comments and blank lines only" => "//\n///\n// prefab p\n\n   \n".into(),
        "tabs used as the separator" => "\t\tprefab\tp\n\t glyph\t@".into(),
        "the empty file" => String::new(),
        other => panic!("no source defined for hostile case {other:?}"),
    }
}

#[test]
fn hostile_input_produces_diagnostics_and_never_panics() {
    for (case, must_diagnose) in HOSTILE {
        let src = hostile_source(case);
        // Reaching the next line at all is most of the test: `parse` slices and
        // indexes in 25 places, and a panic here is a crash in `gamec`, which
        // exists to be pointed at files nobody trusted.
        let (content, diags) = parse(&src);
        if must_diagnose {
            assert!(
                !diags.is_empty(),
                "{case:?} is rejected by the grammar but parse() reported no \
                 diagnostic — the line was skipped silently, which SPEC.md \
                 §9.1 explicitly forbids"
            );
        }
        // Whatever survived must still be coherent enough to hand onward: the
        // validator is the next stage and it must not panic either.
        let _ = validate(&content);
    }
}

#[test]
fn parse_survives_random_bytes() {
    // Complements roundtrip_fuzz.rs, which generates well-formed content on
    // purpose and therefore never reaches the rejection paths. Seeded, so a
    // failure reproduces from the seed printed in the panic message.
    let alphabet: Vec<char> = "prefabtilelvglyphcorstn #@.0123456789x-/\t\u{00E9}\u{1F600}\n"
        .chars()
        .collect();
    for seed in 0..400u64 {
        let mut rng = SplitMix64::new(seed);
        let len = rng.below(300) as usize;
        let src: String = (0..len)
            .map(|_| alphabet[rng.below(alphabet.len() as u32) as usize])
            .collect();
        let (content, _diags) = parse(&src);
        // Every level the parser produced must be internally consistent enough
        // for the validator to describe it rather than crash on it.
        let _ = validate(&content);
        // Parsing is a pure function of the text: the same bytes twice must
        // give the same content, or the content pipeline is not replay-safe
        // either.
        let (again, _) = parse(&src);
        assert!(
            content_eq(&content, &again),
            "parse is not deterministic for the input generated by seed {seed}"
        );
    }
}

#[test]
fn the_grammars_dimension_rule_is_enforced_one_stage_later_by_the_validator() {
    // §9.1 annotates `row-stmt` with "W cells wide, exactly H rows". The parser
    // does not check that, and should not: it is a lexical stage, and the
    // constraint relates two statements. Recording where it *is* enforced stops
    // the annotation reading like a parser bug — and fails if the validator
    // ever stops catching it.
    for (case, src) in [
        (
            "rows wider than W",
            "level l 2x2\nrow ##########\nrow ##########",
        ),
        ("rows narrower than W", "level l 8x2\nrow ##\nrow ##"),
        ("too few rows", "level l 2x9\nrow ##"),
        ("too many rows", "level l 2x1\nrow ##\nrow ##\nrow ##"),
    ] {
        let (content, parse_diags) = parse(src);
        assert!(
            parse_diags.is_empty(),
            "{case}: the parser is lexical and should not diagnose this"
        );
        assert!(
            !validate(&content).is_empty(),
            "{case}: no stage rejects it. A level whose rows do not match its \
             declared size would reach the simulation malformed, which is the \
             one thing `gamec` exists to prevent."
        );
    }
    let (ok, _) = parse("level l 2x2\nrow ##\nrow ##");
    assert!(
        validate(&ok).is_empty(),
        "a level whose rows match its declared size must validate cleanly"
    );
}

#[test]
fn the_dispatch_scanner_reads_arms_and_stops_at_the_catch_all() {
    // Both directions on a synthetic source, so the keyword test above cannot
    // pass by finding nothing.
    let sample = "\
        match keyword {\n\
            \"alpha\" => a(),\n\
            \"beta\" | \"gamma\" => b(),\n\
            _ => unknown(),\n\
            \"delta\" => never_reached(),\n\
        }\n";
    let found = parser_dispatch_keywords(sample);
    assert_eq!(
        found,
        ["alpha", "beta", "gamma"]
            .iter()
            .map(|s| s.to_string())
            .collect::<BTreeSet<_>>(),
        "the scanner must read every arm and stop at the catch-all"
    );

    // The shape that actually defeated the first draft: an arm whose body is
    // itself a match, so a `_ =>` appears before the dispatch's own catch-all.
    let nested = "\
        match keyword {\n\
            \"alpha\" => match inner {\n\
                Some(x) => f(x),\n\
                _ => g(),\n\
            },\n\
            \"beta\" | \"gamma\" => b(),\n\
            _ => unknown(),\n\
        }\n";
    assert_eq!(
        parser_dispatch_keywords(nested),
        ["alpha", "beta", "gamma"]
            .iter()
            .map(|s| s.to_string())
            .collect::<BTreeSet<_>>(),
        "a nested match's catch-all must not be mistaken for the dispatch's"
    );

    assert!(
        parser_dispatch_keywords("no dispatch here").is_empty(),
        "a source with no dispatch yields nothing, and the keyword test fails \
         loudly on that rather than passing vacuously"
    );
}
