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
/// The modules whose job is checking a simulation. Named once, because two
/// documents make claims about this set and both are checked below.
const VERIFICATION_FAMILY: &[&str] = &[
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
];

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
    for required in VERIFICATION_FAMILY {
        assert!(
            listed.contains(*required),
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
        // The handbook snapshot stated an exact 3744 and was wrong two
        // commits later, in the very commit that removed the other exact
        // numbers from it. Last one converted; now nothing in the snapshot
        // carries a count that nobody checks.
        ("AGENT_INSTRUCTIONS.md", "**3,600+ passed / 0 failed**"),
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
fn no_markdown_document_links_to_a_missing_file() {
    // Deleting the superseded audit documents left eight dead relative links
    // across three files — READMEs pointing readers at files that no longer
    // exist. A link is a claim that a file exists; claims get checked.
    let docs = [
        "README.md",
        "AGENT_INSTRUCTIONS.md",
        "izanagi/README.md",
        "izanagi/CLAUDE.md",
        "izanagi/ARCHITECTURE.md",
        "izanagi/CONTRIBUTING.md",
        "izanagi_kit/README.md",
        "izanagi_kit/RESEARCH.md",
        "izanagi_kit/SPEC.md",
        "izanagi_kit/GAME_DEV_TAXONOMY.md",
        "izanagi_kit/CHANGELOG.md",
        "docs/ci/README.md",
    ];
    let mut dead: Vec<String> = Vec::new();
    let mut checked = 0usize;
    for doc in docs {
        let dir = Path::new(doc).parent().unwrap_or_else(|| Path::new(""));
        let text = read(doc);
        // Every `](target)` where target is a relative path (no scheme, no
        // pure fragment). Anchors are split off before the existence check.
        let mut rest = text.as_str();
        while let Some(open) = rest.find("](") {
            rest = &rest[open + 2..];
            let Some(close) = rest.find(')') else { break };
            let target = &rest[..close];
            rest = &rest[close..];
            if target.starts_with("http") || target.starts_with('#') || target.is_empty() {
                continue;
            }
            let path_part = target.split('#').next().unwrap_or(target);
            if path_part.is_empty() {
                continue;
            }
            checked += 1;
            let resolved = repo_root().join(dir).join(path_part);
            if !resolved.exists() {
                dead.push(format!("{doc} -> {target}"));
            }
        }
    }
    assert!(
        checked >= 10,
        "expected to find relative links to check, found {checked} — has the \
         document set changed?"
    );
    assert!(
        dead.is_empty(),
        "these markdown links point at files that do not exist: {dead:#?}"
    );
}

#[test]
fn every_manifest_version_has_a_changelog_entry() {
    // The engine's Cargo.toml said 4.1.0 while its CHANGELOG stopped at 4.0.0,
    // and nobody noticed until someone went looking for the rationale behind
    // the version number. A version is a claim about what shipped; if the
    // changelog cannot corroborate it, one of the two is wrong.
    //
    // `[Unreleased]` counts as corroboration for a version that has not been
    // cut yet — the point is that the number must be *accounted for*
    // somewhere, not that every bump needs a release section immediately.
    for (manifest, changelog) in [
        ("izanagi/Cargo.toml", "izanagi/CHANGELOG.md"),
        ("izanagi_kit/Cargo.toml", "izanagi_kit/CHANGELOG.md"),
    ] {
        let version = read(manifest)
            .lines()
            .find_map(|l| l.trim().strip_prefix("version = ").map(str::to_string))
            .map(|v| v.trim().trim_matches('"').to_string())
            .unwrap_or_else(|| panic!("{manifest} must declare a version"));
        let log = read(changelog);
        assert!(
            log.contains(&format!("[{version}]")) || log.contains(&version),
            "{manifest} declares {version}, but {changelog} never mentions it — \
             add the release section, or note why the number is what it is"
        );
    }
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

#[test]
fn the_capability_map_covers_the_verification_family() {
    // GAME_DEV_TAXONOMY.md is what the README sends readers to for "the
    // capability map, with per-feature implementation status". It was written
    // before the verification modules existed and then never grew a row for
    // any of them: ten of the twelve modules below appeared nowhere in it,
    // including every module the handbook calls this crate's defining
    // strength. A capability map that omits the headline capability sends the
    // reader away believing it is absent.
    //
    // The map is prose, not a table this test can parse, so the check is the
    // weakest one that would have caught the real defect: each module has to
    // be named somewhere. That is enough — the failure mode was silence.
    let taxonomy = read("izanagi_kit/GAME_DEV_TAXONOMY.md");
    let missing: Vec<&&str> = VERIFICATION_FAMILY
        .iter()
        .filter(|m| !taxonomy.contains(&format!("`{m}`")))
        .collect();
    assert!(
        missing.is_empty(),
        "GAME_DEV_TAXONOMY.md is the capability map the README points at, and          it never mentions these simulation-checking modules: {missing:?}. Add          a row for each in the same commit that adds the module."
    );
}

#[test]
fn no_document_points_at_an_iteration_that_has_ended() {
    // A document that says "this iteration" freezes the moment it is written.
    // It happened twice: GAME_DEV_TAXONOMY.md's roadmap still named the
    // terminal module as the current work many iterations later, and SPEC.md
    // ended its completeness checklist with a subsection listing D1/P1/R1 as
    // work to do while the table directly above it recorded all three as done.
    // A reader cannot tell a stale pointer from a live one, which makes it
    // worse than no pointer at all.
    //
    // The rule is therefore about tense, not about content: say what is true
    // now, and keep the list of what comes next in one place (RESEARCH.md's
    // candidate table), where being out of date is visible.
    let docs = [
        "README.md",
        "AGENT_INSTRUCTIONS.md",
        "izanagi/README.md",
        "izanagi/CLAUDE.md",
        "izanagi/ARCHITECTURE.md",
        "izanagi/CONTRIBUTING.md",
        "izanagi_kit/README.md",
        "izanagi_kit/RESEARCH.md",
        "izanagi_kit/SPEC.md",
        "izanagi_kit/GAME_DEV_TAXONOMY.md",
        "izanagi_kit/CHANGELOG.md",
        "docs/ci/README.md",
    ];
    // Phrases that name "the iteration being worked on" as if the reader were
    // inside it. Past-tense records ("implemented in 1e45bc4") are fine and
    // deliberately not matched.
    let frozen = [
        "本イテレーションで",
        "本ループで実装",
        "今回のイテレーション",
        "本反復で実装",
    ];
    let mut found: Vec<String> = Vec::new();
    for doc in docs {
        let text = read(doc);
        for (n, line) in text.lines().enumerate() {
            for needle in frozen {
                if line.contains(needle) {
                    found.push(format!("{doc}:{}: {}", n + 1, line.trim()));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "these lines point at an iteration the reader is not in: {found:#?}\n\n\
         Rewrite them in the present tense — what is true now — and put \
         anything still to come in RESEARCH.md's candidate table."
    );
}

#[test]
fn every_fenced_block_declares_its_language() {
    // An untagged fence is Rust as far as rustdoc is concerned. The workspace
    // README carried two blocks of shell commands in bare fences; the moment
    // that README was included as a doctest, rustdoc tried to compile
    // `cargo test --workspace` as an expression and the build broke. The
    // blocks had been wrong the whole time — nothing had ever looked at them.
    //
    // Tagging every fence costs three characters and means a document can be
    // wired up as a doctest without first auditing it.
    let docs = [
        "README.md",
        "AGENT_INSTRUCTIONS.md",
        "izanagi/README.md",
        "izanagi/CLAUDE.md",
        "izanagi/ARCHITECTURE.md",
        "izanagi/CONTRIBUTING.md",
        "izanagi_kit/README.md",
        "izanagi_kit/RESEARCH.md",
        "izanagi_kit/SPEC.md",
        "izanagi_kit/GAME_DEV_TAXONOMY.md",
        "izanagi_kit/CHANGELOG.md",
        "docs/ci/README.md",
    ];
    let mut untagged: Vec<String> = Vec::new();
    let mut checked = 0usize;
    for doc in docs {
        let text = read(doc);
        let mut inside = false;
        for (n, line) in text.lines().enumerate() {
            if !line.starts_with("```") {
                continue;
            }
            if !inside {
                checked += 1;
                if line[3..].trim().is_empty() {
                    untagged.push(format!("{doc}:{}", n + 1));
                }
            }
            inside = !inside;
        }
    }
    assert!(
        checked >= 20,
        "expected to find the documents' code blocks, found {checked} — has \
         the document set changed?"
    );
    assert!(
        untagged.is_empty(),
        "these fenced blocks declare no language, so rustdoc would read them \
         as Rust: {untagged:#?}\n\nUse ```text for diagrams and shell \
         transcripts, ```rust for code meant to compile."
    );
}

#[test]
fn both_readmes_that_can_be_doctested_are_doctested() {
    // izanagi_kit compiles its README's code blocks; the engine did not. A
    // quickstart nothing compiles is a quickstart that stops working silently,
    // and these are the first lines anyone copies.
    //
    // The workspace README is deliberately NOT included this way. It sits
    // above both crates, so `include_str!("../../README.md")` reaches outside
    // the package and the published crate cannot run its own doctests —
    // measured by unpacking the tarball, where `cargo test --doc` failed on
    // exactly that line. `cargo package --verify` does not catch it because it
    // runs a build, and `#[cfg(doctest)]` items do not exist during a build.
    // The workspace README's blocks are checked by equality instead, in
    // izanagi/tests/readme_blocks_agree.rs.
    for (lib, included) in [
        ("izanagi_kit/src/lib.rs", "../README.md"),
        ("izanagi/src/lib.rs", "../README.md"),
    ] {
        let src = read(lib);
        let wiring = format!("#[doc = include_str!(\"{included}\")]");
        assert!(
            src.contains(&wiring),
            "{lib} no longer includes {included} as a doctest. Without it the \
             README's Rust blocks are compiled by nothing."
        );
    }
}
