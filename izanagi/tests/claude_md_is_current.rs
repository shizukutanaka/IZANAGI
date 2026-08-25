//! CLAUDE.md's Map block, checked against the directories it describes.
//!
//! The Map is the first thing an agent or contributor reads to orient in this
//! crate, and it was found listing 16 of 24 source modules — the fourth
//! hand-maintained inventory in this repository discovered to have drifted
//! from reality. The other three (module counts, test counts, the engine
//! version quoted in the kit's docs) are already build-checked; this closes
//! the last one.
//!
//! The rule is bidirectional: every `.rs` file in `src/`, `examples/` and
//! `tests/` must appear in the Map, and the Map must not name a file that
//! does not exist. Add or remove a file and forget the Map, and the build
//! says so.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The `.rs` file stems actually present in one directory.
fn files_in(dir: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let entries =
        fs::read_dir(crate_root().join(dir)).unwrap_or_else(|e| panic!("cannot list {dir}: {e}"));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "rs").unwrap_or(false) {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                out.insert(stem.to_string());
            }
        }
    }
    out
}

/// The file stems CLAUDE.md's Map block claims for one directory heading.
///
/// The Map is a fenced block with `src/`, `examples/`, `tests/` headings, each
/// followed by indented `  <name>.rs — description` lines (continuation lines
/// are indented further and carry no `.rs` name at the start).
fn mapped(section: &str) -> BTreeSet<String> {
    let text = fs::read_to_string(crate_root().join("CLAUDE.md")).expect("CLAUDE.md");
    let fence = text
        .split("```")
        .nth(1)
        .expect("CLAUDE.md must contain the fenced Map block");
    let mut out = BTreeSet::new();
    let mut in_section = false;
    for line in fence.lines() {
        // A section heading is a bare `name/` on its own line. A description
        // that merely *ends* with a path ("…checked against src/") is not.
        let bare = line.trim();
        if bare.ends_with('/') && !bare.contains(' ') {
            in_section = bare == section;
            continue;
        }
        if !in_section {
            continue;
        }
        let trimmed = line.trim_start();
        if let Some(name) = trimmed.split_whitespace().next() {
            if let Some(stem) = name.strip_suffix(".rs") {
                out.insert(stem.to_string());
            }
        }
    }
    assert!(
        !out.is_empty(),
        "found no `{section}` entries in CLAUDE.md's Map — has its format \
         changed? This test would silently stop checking."
    );
    out
}

fn assert_in_sync(dir: &str, section: &str) {
    let real = files_in(dir);
    let claimed = mapped(section);

    let unlisted: Vec<&String> = real.difference(&claimed).collect();
    assert!(
        unlisted.is_empty(),
        "these files exist in {dir} but CLAUDE.md's Map does not list them: \
         {unlisted:?}. Add each with a one-line description."
    );
    let phantom: Vec<&String> = claimed.difference(&real).collect();
    assert!(
        phantom.is_empty(),
        "CLAUDE.md's Map lists {phantom:?} under {section}, which do not \
         exist in {dir}. Remove them or restore the files."
    );
}

#[test]
fn the_map_lists_exactly_the_source_modules() {
    assert_in_sync("src", "src/");
}

#[test]
fn the_map_lists_exactly_the_examples() {
    assert_in_sync("examples", "examples/");
}

#[test]
fn the_map_lists_exactly_the_test_files() {
    assert_in_sync("tests", "tests/");
}
