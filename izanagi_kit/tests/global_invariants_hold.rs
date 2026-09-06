//! `SPEC.md` §2 lists eight invariants every module must satisfy. This file
//! connects that table to the build.
//!
//! Five of the eight were already enforced somewhere — the float ban, the
//! wall-clock ban, the pinned hashes, the panic-free API, the MSRV. Three were
//! not, and the largest of those is the one this repository says most often:
//!
//! **G1, zero runtime dependencies.** It is the first line of the README, a
//! rule in both CLAUDE.md files, a hard constraint in AGENT_INSTRUCTIONS.md,
//! and the closing sentence of the crate's own package description. Nothing
//! checked it. `cargo add` anything and every one of those documents silently
//! becomes false — and for a crate whose pitch is a small audit surface, that
//! is the promise with the most riding on it.
//!
//! The last test is the one that keeps this file honest: it reads the `G*`
//! rows out of `SPEC.md` and fails if any of them is not accounted for here.
//! Writing a ninth invariant into the specification therefore fails the build
//! until someone says where it is enforced, which is the moment to notice
//! whether it can be.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crate directory has a parent")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    fs::read_to_string(repo_root().join(rel)).unwrap_or_else(|e| panic!("reading {rel}: {e}"))
}

/// The body of one `[section]` of a Cargo manifest: every line after the
/// header until the next one, with comments and blanks dropped.
///
/// Deliberately a hand-rolled parser rather than a TOML crate — a test that
/// enforces "zero dependencies" cannot itself add one, not even a dev one.
fn manifest_section(manifest: &str, section: &str) -> Vec<String> {
    let header = format!("[{section}]");
    let mut out = Vec::new();
    let mut inside = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed == header;
            continue;
        }
        if !inside || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        out.push(trimmed.to_string());
    }
    out
}

/// The value of a `key = "value"` line in the manifest's `[package]` table.
fn package_key(manifest: &str, key: &str) -> Option<String> {
    manifest_section(manifest, "package")
        .into_iter()
        .find_map(|l| {
            let (k, v) = l.split_once('=')?;
            if k.trim() != key {
                return None;
            }
            Some(v.trim().trim_matches('"').to_string())
        })
}

const CRATES: [&str; 2] = ["izanagi_kit", "izanagi"];

#[test]
fn g1_neither_crate_declares_a_runtime_dependency() {
    for krate in CRATES {
        let manifest = read(&format!("{krate}/Cargo.toml"));
        let deps = manifest_section(&manifest, "dependencies");
        assert!(
            deps.is_empty(),
            "{krate} declares runtime dependencies: {deps:?}\n\nSPEC.md G1, both \
             READMEs, both CLAUDE.md files and the crate's own package \
             description all promise there are none. Adding one means editing \
             every one of those, not just this test."
        );
    }
}

#[test]
fn g1_dev_dependencies_are_path_only_and_never_reach_a_consumer() {
    // A dev-dependency does not ship, so it does not break G1 — but a
    // *registry* dev-dependency would still put a third-party crate in the
    // build of anyone who runs the test suite, which is most of the audit
    // surface G1 exists to keep small. The engine's single dev-dependency is
    // the sibling kit, by path, used only by examples/kit_bridge.rs.
    for krate in CRATES {
        let manifest = read(&format!("{krate}/Cargo.toml"));
        for dep in manifest_section(&manifest, "dev-dependencies") {
            assert!(
                dep.contains("path ="),
                "{krate} has the dev-dependency `{dep}`, which is not a path \
                 dependency. Only sibling crates in this workspace are allowed."
            );
        }
    }
}

#[test]
fn g1_neither_crate_has_a_build_script() {
    // "Zero dependencies, zero config" also means nothing runs at build time.
    // A build.rs is arbitrary code executed on a consumer's machine during
    // compilation — the same audit surface a dependency opens, without even
    // appearing in the lockfile.
    for krate in CRATES {
        let path = repo_root().join(krate).join("build.rs");
        assert!(
            !path.exists(),
            "{krate}/build.rs exists. Nothing in this workspace needs code to \
             run at build time, and a build script is unreviewed code in every \
             consumer's compile."
        );
        let manifest = read(&format!("{krate}/Cargo.toml"));
        assert!(
            package_key(&manifest, "build").is_none(),
            "{krate}'s manifest points at a build script"
        );
    }
}

#[test]
fn g2_both_crates_forbid_unsafe_code() {
    // While the attribute is present the compiler enforces it absolutely.
    // Nothing enforced that it stays present — deleting one line would end the
    // guarantee without any test noticing.
    for (krate, lib) in [
        ("izanagi_kit", "izanagi_kit/src/lib.rs"),
        ("izanagi", "izanagi/src/lib.rs"),
    ] {
        assert!(
            read(lib).contains("#![forbid(unsafe_code)]"),
            "{krate} no longer declares #![forbid(unsafe_code)] in {lib}. That \
             attribute is SPEC.md G2 and a rule in CLAUDE.md; it is the whole \
             of the memory-safety claim."
        );
    }
}

#[test]
fn g8_both_crates_declare_edition_2021_and_their_stated_msrv() {
    // `tests/msrv_is_respected.rs` checks that the *code* stays inside each
    // MSRV. This checks the other half: that the manifests still declare the
    // MSRV that scanner is written against, and the edition SPEC.md names.
    // Bumping `rust-version` without telling the scanner would leave it
    // enforcing a promise nobody is making any more.
    for (krate, msrv) in [("izanagi_kit", "1.75"), ("izanagi", "1.65")] {
        let manifest = read(&format!("{krate}/Cargo.toml"));
        assert_eq!(
            package_key(&manifest, "edition").as_deref(),
            Some("2021"),
            "{krate} must declare edition 2021 (SPEC.md G8)"
        );
        assert_eq!(
            package_key(&manifest, "rust-version").as_deref(),
            Some(msrv),
            "{krate}'s declared MSRV changed. Update tests/msrv_is_respected.rs \
             and SPEC.md G8 in the same commit, or the scanner enforces a \
             version nobody promises."
        );
    }
}

/// Where each `SPEC.md` §2 invariant is actually enforced.
///
/// Every `G*` row in the specification must appear here. The value is not
/// decoration: it is the answer to "what would fail if this stopped being
/// true?", and a row that cannot be given one is a promise the build does not
/// keep.
fn enforcement_sites() -> BTreeMap<&'static str, &'static str> {
    let mut m = BTreeMap::new();
    m.insert(
        "G1",
        "this file: g1_* (empty [dependencies], path-only dev-deps, no build.rs)",
    );
    m.insert(
        "G2",
        "this file: g2_both_crates_forbid_unsafe_code, then the compiler",
    );
    m.insert(
        "G3",
        "izanagi_kit/tests/no_float_in_sim.rs, izanagi/tests/float_boundary.rs",
    );
    m.insert(
        "G4",
        "izanagi_kit/tests/no_nondeterminism_in_sim.rs (clock, threads, addresses)",
    );
    m.insert(
        "G5",
        "izanagi_kit/tests/determinism.rs + roguelike_sim.rs (pinned hashes)",
    );
    m.insert(
        "G6",
        "izanagi_kit/tests/no_nondeterminism_in_sim.rs (hash-map allowlist with reasons)",
    );
    m.insert(
        "G7",
        "the crate-level deny(clippy::unwrap_used, expect_used, panic) in both lib.rs",
    );
    m.insert(
        "G8",
        "this file: g8_*, plus izanagi_kit/tests/msrv_is_respected.rs for the code",
    );
    m
}

#[test]
fn every_global_invariant_in_the_spec_says_where_it_is_enforced() {
    let spec = read("izanagi_kit/SPEC.md");
    let sites = enforcement_sites();

    // Rows look like: `| G1 | **zero runtime dependencies**（…） | 監査面の最小化 |`
    let declared: Vec<String> = spec
        .lines()
        .filter_map(|l| {
            let rest = l.trim().strip_prefix("| G")?;
            let id: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if id.is_empty() {
                return None;
            }
            Some(format!("G{id}"))
        })
        .collect();

    assert!(
        declared.len() >= 8,
        "expected to find SPEC.md's global invariant table, found {declared:?} — \
         has the table's shape changed?"
    );

    let unaccounted: Vec<&String> = declared
        .iter()
        .filter(|g| !sites.contains_key(g.as_str()))
        .collect();
    assert!(
        unaccounted.is_empty(),
        "SPEC.md declares these global invariants with nothing recorded as \
         enforcing them: {unaccounted:?}\n\nAdd each to enforcement_sites() \
         naming the check — and if there is no check to name, that is the \
         finding, not a formality."
    );

    let stale: Vec<&&str> = sites
        .keys()
        .filter(|g| !declared.contains(&g.to_string()))
        .collect();
    assert!(
        stale.is_empty(),
        "enforcement_sites() names invariants SPEC.md no longer declares: \
         {stale:?}"
    );
}

#[test]
fn the_manifest_parser_reads_the_sections_it_claims_to() {
    // Both directions, on a manifest shaped like the real ones. A parser that
    // returned an empty section for everything would make g1 pass vacuously,
    // which is the failure mode worth ruling out explicitly.
    let sample = "\
[package]
name = \"probe\"
edition = \"2021\"
# a comment
rust-version = \"1.75\"

[dependencies]
serde = \"1\"

[dev-dependencies]
sibling = { path = \"../sibling\" }

[lib]
path = \"src/lib.rs\"
";
    assert_eq!(
        manifest_section(sample, "dependencies"),
        vec!["serde = \"1\""]
    );
    assert_eq!(
        manifest_section(sample, "dev-dependencies"),
        vec!["sibling = { path = \"../sibling\" }"]
    );
    assert_eq!(package_key(sample, "edition").as_deref(), Some("2021"));
    assert_eq!(package_key(sample, "rust-version").as_deref(), Some("1.75"));
    assert_eq!(package_key(sample, "build"), None);
    // A section that is present but empty, and one that is absent, both read
    // as empty — which is exactly what the real manifests rely on.
    assert!(manifest_section("[dependencies]\n\n[lib]\n", "dependencies").is_empty());
    assert!(manifest_section(sample, "no-such-section").is_empty());
    // And `[dependencies]` must not swallow `[dev-dependencies]`.
    assert!(!manifest_section(sample, "dependencies")
        .iter()
        .any(|l| l.contains("sibling")));

    // The path-only rule, exercised on both shapes. It cannot be probed by
    // editing the real manifest: a registry dependency does not resolve
    // offline, so the build fails before the test runs. Checking the predicate
    // against a synthetic manifest is the honest way to cover that branch.
    let with_registry_dev_dep = "[dev-dependencies]\nsibling = { path = \"../s\" }\nfoo = \"1\"\n";
    let offenders: Vec<String> = manifest_section(with_registry_dev_dep, "dev-dependencies")
        .into_iter()
        .filter(|d| !d.contains("path ="))
        .collect();
    assert_eq!(
        offenders,
        vec!["foo = \"1\""],
        "the path-only rule must reject a registry dev-dependency and accept a path one"
    );
}
