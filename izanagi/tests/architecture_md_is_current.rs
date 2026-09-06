//! ARCHITECTURE.md's file map and subsystem count, checked against the code.
//!
//! `CLAUDE.md`'s Map is build-checked because it was found listing 16 of 24
//! modules. ARCHITECTURE.md — the same crate, the same kind of document, in
//! the same directory — had the identical defect and no check: it listed 16 of
//! 25 files, omitting `audio_pcm`, `camera`, `debug`, `event`, `gamepad`,
//! `log`, `sprite`, `tilemap` and `tween`. A reader orienting in the engine
//! from its architecture document would not learn that a third of it exists.
//!
//! Two more claims in that file were wrong in the same way:
//!
//! - "`Engine` owns six subsystems as public fields" — there are ten, and the
//!   diagram above the sentence drew five.
//! - "Total: ~1700 lines, ~85 tests" — the real figures were 5,898 and 209,
//!   wrong by 3.5x and 2.5x. Those numbers are now simply gone rather than
//!   corrected: nothing checks a line count, and this repository has already
//!   deleted four other hand-maintained totals for drifting.
//!
//! What remains is what can be checked, and this is the check.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn architecture() -> String {
    fs::read_to_string(crate_root().join("ARCHITECTURE.md")).expect("ARCHITECTURE.md")
}

/// The `.rs` stems actually present in `src/`.
fn source_files() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for entry in fs::read_dir(crate_root().join("src"))
        .expect("src/")
        .flatten()
    {
        let path = entry.path();
        if path.extension().map(|e| e == "rs").unwrap_or(false) {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                out.insert(stem.to_string());
            }
        }
    }
    out
}

/// The `.rs` stems ARCHITECTURE.md's file-map tree claims.
///
/// Lines look like `├── ecs.rs        # World, Entity, sparse columns` or
/// `└── backend.rs    # …`.
fn mapped_files() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in architecture().lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed
            .strip_prefix("├── ")
            .or_else(|| trimmed.strip_prefix("└── "))
        else {
            continue;
        };
        if let Some(name) = rest.split_whitespace().next() {
            if let Some(stem) = name.strip_suffix(".rs") {
                out.insert(stem.to_string());
            }
        }
    }
    out
}

#[test]
fn the_file_map_lists_exactly_the_source_modules() {
    let real = source_files();
    let claimed = mapped_files();
    assert!(
        claimed.len() > 10,
        "found only {} entries in ARCHITECTURE.md's file map — has the tree's \
         format changed? This test would silently stop checking.",
        claimed.len()
    );

    let unlisted: Vec<&String> = real.difference(&claimed).collect();
    assert!(
        unlisted.is_empty(),
        "these modules exist in src/ but ARCHITECTURE.md's file map omits \
         them: {unlisted:?}. A reader orienting from this document would not \
         learn they exist."
    );
    let phantom: Vec<&String> = claimed.difference(&real).collect();
    assert!(
        phantom.is_empty(),
        "ARCHITECTURE.md's file map lists {phantom:?}, which src/ does not \
         contain."
    );
}

/// The number of `pub` fields on `struct Engine`.
fn engine_public_field_count() -> usize {
    let lib = fs::read_to_string(crate_root().join("src/lib.rs")).expect("src/lib.rs");
    let start = lib
        .find("pub struct Engine")
        .expect("src/lib.rs must define `pub struct Engine`");
    let body = &lib[start..];
    let end = body.find("\n}").expect("the Engine struct must be closed");
    body[..end]
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with("pub ") && t.contains(':') && !t.starts_with("pub struct")
        })
        .count()
}

#[test]
fn the_shape_section_states_the_real_number_of_subsystems() {
    // The sentence said "six" while the struct had ten, and the diagram drew
    // five. Three descriptions of one thing, none of them agreeing with it.
    const NUMERALS: [(usize, &str); 8] = [
        (5, "five"),
        (6, "six"),
        (7, "seven"),
        (8, "eight"),
        (9, "nine"),
        (10, "ten"),
        (11, "eleven"),
        (12, "twelve"),
    ];
    let fields = engine_public_field_count();
    assert!(
        (5..=12).contains(&fields),
        "Engine has {fields} public fields, outside the range this test can \
         spell — extend NUMERALS"
    );
    let word = NUMERALS
        .iter()
        .find(|(n, _)| *n == fields)
        .map(|(_, w)| *w)
        .expect("the count is in range");
    let text = architecture();
    assert!(
        text.contains(&format!("`Engine` owns {word} subsystems")),
        "Engine has {fields} public fields, so ARCHITECTURE.md must say \
         \"`Engine` owns {word} subsystems\". Update the ASCII diagram in the \
         same edit — it is the part a reader believes."
    );
    // Every field must also appear in the diagram, so the picture and the
    // sentence cannot drift apart again.
    let lib = fs::read_to_string(crate_root().join("src/lib.rs")).expect("src/lib.rs");
    let start = lib.find("pub struct Engine").expect("struct Engine");
    let body = &lib[start..];
    let end = body.find("\n}").expect("closed");
    let shape = text
        .split("## Decision: one engine type")
        .next()
        .expect("the shape section precedes the first decision");
    for line in body[..end].lines() {
        let t = line.trim_start();
        let Some(rest) = t.strip_prefix("pub ") else {
            continue;
        };
        let Some((name, _)) = rest.split_once(':') else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() || name == "struct Engine" {
            continue;
        }
        assert!(
            shape.contains(name),
            "the Engine field `{name}` is missing from ARCHITECTURE.md's shape \
             diagram"
        );
    }
}

#[test]
fn architecture_md_states_no_line_or_test_total() {
    // These are the numbers that rot. The file used to end its map with
    // "Total: ~1700 lines, ~85 tests"; both were wrong by more than a factor
    // of two by the time anyone looked. `cargo test` prints the real count on
    // every run, so the document does not need to guess.
    let text = architecture();
    for banned in ["~1700 lines", "~85 tests", "Total: ~"] {
        assert!(
            !text.contains(banned),
            "ARCHITECTURE.md states `{banned}`. A total nobody checks is a \
             total that drifts — leave it to `cargo test`."
        );
    }
}

#[test]
fn the_map_parser_reads_a_tree_and_ignores_prose() {
    // Both directions: a parser that returned nothing would make the sync test
    // pass vacuously, and one that matched prose would report phantom files.
    let real = mapped_files();
    for expected in ["lib", "ecs", "backend", "gamepad", "tilemap"] {
        assert!(
            real.contains(expected),
            "the parser missed `{expected}.rs`, which the map lists"
        );
    }
    // Prose mentioning a path must not be read as a map entry. The file
    // contains sentences like "checked against `pub struct Engine`"; none of
    // them start with a tree connector, which is what the parser requires.
    assert!(
        !real.iter().any(|f| f.contains(' ') || f.contains('`')),
        "the parser picked up prose as a file name: {real:?}"
    );
}
