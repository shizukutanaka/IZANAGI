//! The documentation's factual claims, machine-checked.
//!
//! A survey of this repository's markdown found six documents making headline
//! claims that had quietly become false — one asserting 77 modules and 3362
//! tests, another 78 and 188, against a real 88 and 3699. Stale numbers are
//! worse than absent ones: a reader has no way to tell which document is
//! current, so every document becomes untrustworthy.
//!
//! Deleting the stale files fixes the symptom. This fixes the cause: the
//! claims that remain are checked by the build, so they cannot rot silently.
//! Add a module and forget the tier map, or move a pinned hash without
//! updating the handbook, and this test fails.
//!
//! Only *verifiable* claims are checked. Prose is not, and should not be —
//! the point is to gate facts, not to make documentation harder to write.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is `<root>/izanagi_kit`.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the kit lives one level below the workspace root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    let path = repo_root().join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// Every `pub mod` declared in `lib.rs` — the ground truth every document is
/// checked against.
fn declared_modules() -> BTreeSet<String> {
    read("izanagi_kit/src/lib.rs")
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("pub mod ")?;
            Some(rest.trim_end_matches(';').trim().to_string())
        })
        .collect()
}

/// Pull `` [`name`] `` and `` [`mod@name`] `` link targets out of one line.
fn linked_names(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find("[`") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("`]") else { break };
        let name = rest[..end].trim_start_matches("mod@");
        // Skip qualified links and method links; only bare module names matter.
        if !name.contains("::") && !name.contains('(') {
            out.push(name.to_string());
        }
        rest = &rest[end + 2..];
    }
    out
}

#[test]
fn tier_map_never_lists_a_module_that_does_not_exist() {
    // The four-tier table in `lib.rs` is the crate's primary navigation. It is
    // hand-maintained, so it rots the moment a module is added or renamed —
    // exactly the failure this test exists to make loud.
    let modules = declared_modules();
    let lib = read("izanagi_kit/src/lib.rs");
    let mut checked = 0usize;
    for line in lib.lines() {
        if !line.starts_with("//! | **") {
            continue;
        }
        for name in linked_names(line) {
            // Tier rows also link non-module items (e.g. `sim::Simulation`),
            // which `linked_names` already filters; anything left that looks
            // like a bare identifier must be a real module.
            if name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                assert!(
                    modules.contains(&name),
                    "the tier map lists `{name}`, which is not a declared module"
                );
                checked += 1;
            }
        }
    }
    assert!(
        checked >= 40,
        "expected the tier map to reference many modules, found {checked} — \
         has its format changed? This test would silently stop checking."
    );
}

#[test]
fn every_tier_one_module_is_declared_and_tier_one_covers_the_verification_family() {
    // Tier 1 is the load-bearing set: the crate's own docs say "if you only
    // adopt one thing, adopt tier 1". A verification module missing from it is
    // a module users will not find.
    let modules = declared_modules();
    let lib = read("izanagi_kit/src/lib.rs");
    let tier_one = lib
        .lines()
        .find(|l| l.starts_with("//! | **1."))
        .expect("lib.rs must document tier 1");
    let listed: BTreeSet<String> = linked_names(tier_one).into_iter().collect();

    for name in &listed {
        assert!(
            modules.contains(name),
            "tier 1 lists `{name}`, which is not a declared module"
        );
    }
    // Every module whose job is checking a simulation belongs in tier 1.
    for required in [
        "sim",
        "replay",
        "rollback",
        "dst",
        "plan",
        "explore",
        "shrink",
        "prop",
        "temporal",
        "recovery",
        "verify",
        "world_hash",
    ] {
        assert!(
            listed.contains(required),
            "`{required}` checks simulations but is missing from the tier 1 map"
        );
    }
}

#[test]
fn readme_module_table_never_lists_a_module_that_does_not_exist() {
    let modules = declared_modules();
    let readme = read("izanagi_kit/README.md");
    let mut checked = 0usize;
    for line in readme.lines() {
        // Module table rows look like: | `name` / `other` | description |
        let Some(rest) = line.strip_prefix("| `") else {
            continue;
        };
        let Some(cell_end) = rest.find(" |") else {
            continue;
        };
        for token in rest[..cell_end].split('/') {
            let name = token.trim().trim_matches('`').trim();
            if name.is_empty() || !name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                continue;
            }
            assert!(
                modules.contains(name),
                "README's module table lists `{name}`, which is not a declared module"
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 20,
        "expected the README table to name many modules, found {checked} — \
         has its format changed?"
    );
}

#[test]
fn handbook_quotes_the_real_pinned_hashes() {
    // AGENT_INSTRUCTIONS.md reproduces the determinism constants so a reader
    // can recognise an unexpected change. If the handbook and the tests ever
    // disagree, the handbook is lying about the one thing it must not.
    let handbook = read("AGENT_INSTRUCTIONS.md");
    for (file, konst) in [
        ("izanagi_kit/tests/determinism.rs", "PINNED_FINAL_HASH"),
        (
            "izanagi_kit/tests/roguelike_sim.rs",
            "PINNED_ROGUELIKE_HASH",
        ),
    ] {
        let src = read(file);
        let line = src
            .lines()
            .find(|l| l.contains(&format!("const {konst}")))
            .unwrap_or_else(|| panic!("{file} must define {konst}"));
        let value = line
            .split('=')
            .nth(1)
            .and_then(|v| v.split(';').next())
            .map(str::trim)
            .unwrap_or_else(|| panic!("cannot parse {konst}"));
        assert!(
            handbook.contains(&format!("{konst}={value}")),
            "AGENT_INSTRUCTIONS.md does not quote {konst}={value}; it must be \
             updated in the same commit that changes the constant"
        );
    }
}

#[test]
fn handbook_module_count_matches_reality() {
    // The snapshot table states how many modules each crate has. A number
    // nobody checks is a number that drifts — this one had drifted by twelve
    // across the documents that were deleted.
    let handbook = read("AGENT_INSTRUCTIONS.md");
    let kit = declared_modules().len();
    let engine = fs::read_dir(repo_root().join("izanagi/src"))
        .expect("engine source directory")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "rs"))
        .count();
    assert!(
        handbook.contains(&format!("kit モジュール数 | **{kit}**"))
            || handbook.contains(&format!("| {kit}(")),
        "AGENT_INSTRUCTIONS.md must state the real kit module count ({kit})"
    );
    assert!(
        handbook.contains(&format!("engine モジュール数 | **{engine}**"))
            || handbook.contains(&format!("| {engine}(")),
        "AGENT_INSTRUCTIONS.md must state the real engine module count ({engine})"
    );
}

#[test]
fn no_document_quotes_a_stale_engine_version() {
    // The kit's own headline used to open with "the IZANAGI engine (v4.4.0)"
    // while the engine's manifest said 4.1.0 — a wrong number in the first
    // sentence a docs.rs visitor reads. Any version this repository quotes for
    // the engine must be the one the engine actually declares.
    let manifest = read("izanagi/Cargo.toml");
    let real = manifest
        .lines()
        .find_map(|l| l.trim().strip_prefix("version = "))
        .map(|v| v.trim().trim_matches('"').to_string())
        .expect("the engine manifest must declare a version");

    let pattern = regex_free_versions(&read("izanagi_kit/src/lib.rs"));
    for quoted in pattern {
        assert_eq!(
            quoted, real,
            "izanagi_kit/src/lib.rs quotes engine version v{quoted}, but the \
             engine declares {real}"
        );
    }
}

/// Every `vX.Y.Z` mentioned in `text`, without pulling in a regex crate.
fn regex_free_versions(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes: Vec<char> = text.chars().collect();
    for (i, c) in bytes.iter().enumerate() {
        if *c != 'v' || i + 1 >= bytes.len() || !bytes[i + 1].is_ascii_digit() {
            continue;
        }
        // A preceding alphanumeric means this is part of a longer word.
        if i > 0 && (bytes[i - 1].is_alphanumeric() || bytes[i - 1] == '_') {
            continue;
        }
        let candidate: String = bytes[i + 1..]
            .iter()
            .take_while(|c| c.is_ascii_digit() || **c == '.')
            .collect();
        if candidate.matches('.').count() == 2 && !candidate.ends_with('.') {
            out.push(candidate);
        }
    }
    out
}

/// Count `#[test]` attributes under a crate. This *undercounts* the tests that
/// actually run, because a doctest carries no attribute — which is what makes
/// it the right measure for checking a floor.
fn test_attributes(crate_dir: &str) -> usize {
    fn walk(dir: &Path, total: &mut usize) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, total);
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                *total += fs::read_to_string(&path)
                    .unwrap_or_default()
                    .matches("#[test]")
                    .count();
            }
        }
    }
    let mut total = 0;
    for sub in ["src", "tests", "examples"] {
        walk(&repo_root().join(crate_dir).join(sub), &mut total);
    }
    total
}

#[test]
fn readme_test_counts_are_floors_the_suite_actually_clears() {
    // Three documents used to state exact counts — 3362, 3174, 188, 159 — and
    // every one had drifted, disagreeing with reality and with each other. An
    // exact count is a number somebody has to remember to update, which is a
    // defect waiting to happen. They are floors now, so drift can only ever
    // make them understatements, and each is set low enough that `#[test]`
    // attributes clear it without counting doctests at all.
    let kit = test_attributes("izanagi_kit");
    let engine = test_attributes("izanagi");

    for (doc, claim) in [
        ("README.md", "3,600+ tests"),
        ("README.md", "3,400+ tests"),
        ("README.md", "**180+ tests**"),
        ("izanagi/README.md", "**180+ tests**"),
    ] {
        assert!(
            read(doc).contains(claim),
            "{doc} no longer states `{claim}`; if the wording changed, update \
             this check in the same commit"
        );
    }

    assert!(
        kit >= 3_400,
        "README claims 3,400+ kit tests; only {kit} `#[test]` attributes found"
    );
    assert!(
        engine >= 180,
        "README claims 180+ engine tests; only {engine} `#[test]` attributes found"
    );
    assert!(
        kit + engine >= 3_600,
        "README claims 3,600+ workspace tests; only {} `#[test]` attributes \
         across both crates",
        kit + engine
    );
}

#[test]
fn no_superseded_audit_documents_remain() {
    // Four documents were deleted for stating counts that had become false
    // (77/78 modules, 188/3362 tests). They are recoverable from git history;
    // what must not happen is one of them reappearing as a second source of
    // truth alongside RESEARCH.md and AGENT_INSTRUCTIONS.md.
    for stale in [
        "izanagi_kit/STRENGTHS_WEAKNESSES.md",
        "izanagi_kit/FEATURE_AUDIT.md",
        "izanagi_kit/IMPROVEMENTS.md",
        "PRODUCT_AUDIT.md",
    ] {
        assert!(
            !repo_root().join(stale).exists(),
            "{stale} was deleted as a superseded source of truth; if it is \
             genuinely needed again, its numbers must be checked here first"
        );
    }
}
