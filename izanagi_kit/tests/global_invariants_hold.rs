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
use std::path::{Path, PathBuf};

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
        "this file: g1_* (empty [dependencies], path-only dev-deps, no build script)",
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
        "izanagi_kit/tests/determinism.rs + izanagi_kit/tests/roguelike_sim.rs (pinned hashes)",
    );
    m.insert(
        "G6",
        "izanagi_kit/tests/no_nondeterminism_in_sim.rs (hash-map allowlist with reasons)",
    );
    m.insert(
        "G7",
        "this file: g7_* (the deny attrs are asserted present at each crate \
         root, and no allow/expect/warn/force_warn may weaken them or \
         unsafe_code)",
    );
    m.insert(
        "G9",
        "izanagi_kit/tests/hashes_are_width_independent.rs (no usize DetHash \
         impl, no write_usize, lengths as u32, no pointer-sized sentinels)",
    );
    m.insert(
        "G10",
        "izanagi_kit/tests/hashes_are_endian_independent.rs (no to_ne_bytes/ \
         to_be_bytes anywhere in library code, every fixed-width write calls \
         to_le_bytes explicitly)",
    );
    m.insert(
        "G11",
        "izanagi_kit/tests/no_platform_cfg_in_sim.rs (cfg/cfg!/cfg_attr \
         predicates may name only test/doc/doctest/docsrs and the \
         not/any/all combinators; no target/features/lints/patch/replace/ \
         build-dependencies manifest tables; no .cargo config)",
    );
    m.insert(
        "G8",
        "this file: g8_*, plus izanagi_kit/tests/msrv_is_respected.rs for the code",
    );
    m
}

/// Library sources of one crate's `src/`, keyed by path relative to it.
/// Same contract as the neighbouring scanners: line comments are stripped
/// first, then the text is cut at the first `#[cfg(test)]` marker, and
/// `src/bin/` is excluded — a CLI's job is to answer to its machine.
fn library_sources(src_root: &Path) -> BTreeMap<String, String> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, String>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().map(|n| n == "bin").unwrap_or(false) {
                    continue;
                }
                walk(&path, root, out);
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                let src = fs::read_to_string(&path).unwrap_or_default();
                let stripped = src
                    .lines()
                    .filter(|l| !l.trim_start().starts_with("//"))
                    .collect::<Vec<_>>()
                    .join("\n");
                let end = stripped.find("#[cfg(test)]").unwrap_or(stripped.len());
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                out.insert(rel, stripped[..end].to_string());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(src_root, src_root, &mut out);
    out
}

/// Text inside a parenthesised group whose opening `(` was just consumed.
fn take_balanced(s: &str) -> &str {
    let mut depth = 1usize;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return &s[..i];
                }
            }
            _ => {}
        }
    }
    s
}

/// The text up to the first comma at the top nesting level: for
/// `cfg_attr(pred, attr...)`, the predicate alone.
fn first_top_level_arg(inner: &str) -> &str {
    let mut depth = 0usize;
    for (i, c) in inner.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => return &inner[..i],
            _ => {}
        }
    }
    inner
}

/// Identifier-shaped atoms with `"..."` literals stripped, so a quoted value
/// cannot masquerade as a named lint or marker.
fn predicate_atoms(text: &str) -> Vec<String> {
    let mut cleaned = String::new();
    let mut in_str = false;
    for c in text.chars() {
        if c == '"' {
            in_str = !in_str;
        } else if !in_str {
            cleaned.push(c);
        }
    }
    cleaned
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|t| !t.is_empty() && !t.chars().all(|c| c.is_ascii_digit()))
        .map(str::to_string)
        .collect()
}

/// `(start, end)` spans of `cfg_attr(...)` argument lists whose predicate can
/// only be true while testing or documenting — `cfg_attr(test, ...)`,
/// `cfg_attr(docsrs, ...)`, `cfg_attr(doctest, ...)`. Anything nested in such
/// a span never applies to shipped code, so a weakening attribute found
/// there is out of scope. `not(...)` is deliberately not inert (`not(test)`
/// applies to every real build), so a predicate containing it still counts.
fn test_only_cfg_attr_spans(code: &str) -> Vec<(usize, usize)> {
    let needle = "cfg_attr(";
    let mut out = Vec::new();
    for (pos, _) in code.match_indices(needle) {
        let prev = code[..pos].chars().last();
        if prev
            .map(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':')
            .unwrap_or(false)
        {
            continue;
        }
        let inner = take_balanced(&code[pos + needle.len()..]);
        let atoms = predicate_atoms(first_top_level_arg(inner));
        let has_marker = atoms
            .iter()
            .any(|a| matches!(a.as_str(), "test" | "doctest" | "doc" | "docsrs"));
        let only_markers = atoms.iter().all(|a| {
            matches!(
                a.as_str(),
                "test" | "doctest" | "doc" | "docsrs" | "any" | "all"
            )
        });
        if has_marker && only_markers {
            out.push((pos + needle.len(), pos + needle.len() + inner.len()));
        }
    }
    out
}

/// Argument lists of every lint-level-lowering attribute in `code` —
/// `allow`, `expect`, `warn`, `force_warn` — except ones nested inside a
/// `cfg_attr` that can only fire under test/doc builds.
fn weakening_lint_args(code: &str) -> Vec<String> {
    const WEAKENERS: &[&str] = &["allow", "expect", "warn", "force_warn"];
    let inert = test_only_cfg_attr_spans(code);
    let mut out = Vec::new();
    for w in WEAKENERS {
        let needle = format!("{w}(");
        for (pos, _) in code.match_indices(&needle) {
            let prev = code[..pos].chars().last();
            if prev
                .map(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':' || c == '.')
                .unwrap_or(false)
            {
                continue;
            }
            if inert.iter().any(|&(s, e)| pos > s && pos <= e) {
                continue;
            }
            out.push(take_balanced(&code[pos + needle.len()..]).to_string());
        }
    }
    out
}

#[test]
fn g7_the_safety_denies_are_present_and_nothing_weakens_them() {
    // G7's panic-free promise rests on `deny(clippy::unwrap_used, ...)` at
    // each crate root — but `deny` is a level, and a level yields to a single
    // `#[allow(clippy::unwrap_used)]` on any item between root and leaf.
    // `expect` is an allow with bookkeeping, `warn`/`force_warn` lower it the
    // same way. `unsafe_code` is `forbid`, which no inner attribute can lower
    // — so an `allow` naming it is flagged anyway: it exists only if someone
    // tried. Both halves are checked: the denies must be there, and no
    // weakening attribute may name a lint they carry.
    const PROTECTED: &[&str] = &["unsafe_code", "unwrap_used", "expect_used", "panic"];

    for (krate, lib) in [
        ("izanagi_kit", "izanagi_kit/src/lib.rs"),
        ("izanagi", "izanagi/src/lib.rs"),
    ] {
        assert!(
            read(lib).contains("deny(clippy::unwrap_used"),
            "{krate}'s panic-path deny attribute is gone from {lib} — G7 then \
             rests on nothing at all"
        );
    }

    let mut found = Vec::new();
    let mut offenders: Vec<String> = Vec::new();
    for src in ["izanagi_kit/src", "izanagi/src"] {
        for (name, code) in library_sources(&repo_root().join(src)) {
            for args in weakening_lint_args(&code) {
                found.push(args.clone());
                for atom in predicate_atoms(&args) {
                    if PROTECTED.contains(&atom.as_str()) {
                        offenders.push(format!(
                            "{name}: a weakening attribute names `{atom}` — \
                             the panic-free and unsafe-free claims hold only \
                             if the crate-level gates apply to every item"
                        ));
                    }
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "lint-level escape hatches found: {offenders:#?}"
    );
    // Vacuity: the scan must actually have found attributes — the crate
    // carries benign `#[allow(missing_docs)]` markers on macro-generated
    // items today, so a scan that found nothing at all is the scanner's
    // failure, not proof of absence.
    assert!(
        found.iter().any(|a| a.contains("missing_docs")),
        "the weakening scan found no `allow` attributes at all — the known \
         `#[allow(missing_docs)]` markers should have shown up"
    );
}

/// Panicking-macro sites that exist in library code, with the count
/// expected and the reason the site is a documented precondition rather
/// than a runtime-input panic path.
///
/// `deny(clippy::panic)` covers only `panic!` itself — `assert!`,
/// `debug_assert!`, `unreachable!`, `todo!` and `unimplemented!` lower to
/// the same trap without tripping the lint. The rule here is the same as
/// the non-determinism allowlist: every existing site is named with a
/// reason, the count is exact, and a new one fails the build until the
/// sentence is written — which is the moment to decide whether the site
/// should instead be a saturating/no-op return (G7's contract for
/// *runtime* input). Slice indexing `v[i]` is the one panic family this
/// does not enumerate: it is syntactically indistinguishable from valid
/// reads, and bounding it belongs to the code review the allowlist gate
/// cannot automate.
fn panicking_macro_allowlist() -> BTreeMap<&'static str, (usize, &'static str)> {
    let mut m = BTreeMap::new();
    m.insert(
        "izanagi_kit/rollback.rs",
        (
            3,
            "SnapshotRing::new's zero-capacity/zero-stride guards are documented \
             programming-error preconditions; the debug_assert_eq self-checks \
             the window-base invariant in development builds only",
        ),
    );
    m.insert(
        "izanagi_kit/netinput.rs",
        (
            3,
            "AdaptiveDelay::new's ordering/capacity preconditions — documented \
             panics on programmer error, not on network input",
        ),
    );
    m.insert(
        "izanagi_kit/timestep.rs",
        (
            2,
            "FixedTimestep::new requires a nonzero rate and cap — documented \
             constructor precondition",
        ),
    );
    m.insert(
        "izanagi_kit/identify.rs",
        (
            1,
            "Identification::new requires at least as many labels as kinds — \
             documented constructor precondition",
        ),
    );
    m.insert(
        "izanagi_kit/wallet.rs",
        (
            1,
            "post-deposit self-check, debug builds only — it is the audit \
             tool, not a panic path reachable from input",
        ),
    );
    m.insert(
        "izanagi_kit/pathfinding.rs",
        (1, "JPS chain-reconstruction invariant, debug builds only"),
    );
    m.insert(
        "izanagi_kit/fov.rs",
        (1, "Frac denominator invariant, debug builds only"),
    );
    m.insert(
        "izanagi_kit/replay.rs",
        (
            2,
            "the zip-longest (None, None) arms are unreachable by loop bound; \
             the tests that exercise the divergence table cover every arm",
        ),
    );
    m.insert(
        "izanagi/sprite.rs",
        (
            1,
            "Animation::new requires a nonempty frame list — documented \
             constructor precondition",
        ),
    );
    m
}

/// Occurrences of panic-lowering macros in `code`. `panic!` itself is
/// absent: `deny(clippy::panic)` already enforces it at compile time.
/// Identifier boundaries are required so `debug_assert!` is not double
/// counted under `assert!` and a `my_assert!` helper is not miscounted.
fn count_panicking_macros(code: &str) -> usize {
    const MACROS: &[&str] = &[
        "debug_assert_eq!(",
        "debug_assert_ne!(",
        "debug_assert!(",
        "assert_eq!(",
        "assert_ne!(",
        "assert!(",
        "unreachable!(",
        "todo!(",
        "unimplemented!(",
    ];
    let bytes = code.as_bytes();
    let mut n = 0;
    for needle in MACROS {
        for (pos, _) in code.match_indices(needle) {
            let prev_ok = pos == 0
                || !(bytes[pos - 1].is_ascii_alphanumeric()
                    || bytes[pos - 1] == b'_'
                    || bytes[pos - 1] == b':'
                    || bytes[pos - 1] == b'.');
            if prev_ok {
                n += 1;
            }
        }
    }
    n
}

#[test]
fn g7_panicking_macros_are_frozen_at_named_sites() {
    let allowed = panicking_macro_allowlist();
    let mut live: Vec<String> = Vec::new();
    for src in ["izanagi_kit/src", "izanagi/src"] {
        for (name, code) in library_sources(&repo_root().join(src)) {
            let n = count_panicking_macros(&code);
            if n == 0 {
                continue;
            }
            let key = format!("{}/{}", src.trim_end_matches("/src"), name);
            live.push(key.clone());
            match allowed.get(key.as_str()) {
                Some(&(expected, _)) => assert_eq!(
                    n, expected,
                    "{key} has {n} panicking macros where {expected} are \
                     named — a new one needs its reason written here first"
                ),
                None => panic!(
                    "{key} contains {n} panicking macro(s) outside the \
                     allowlist. G7's contract is saturate/None/no-op on bad \
                     input; a documented programming-error precondition \
                     belongs in panicking_macro_allowlist() with that reason"
                ),
            }
        }
    }
    let stale: Vec<&&str> = allowed
        .keys()
        .filter(|k| !live.contains(&k.to_string()))
        .collect();
    assert!(
        stale.is_empty(),
        "stale panicking-macro allowlist entries read as permission: {stale:?}"
    );
}

#[test]
fn shipped_code_cannot_come_from_outside_the_scanned_tree() {
    // Every source scanner in this suite shares one load-bearing assumption:
    // the compiled code is the text under src/. `#[path]` points a `mod` at
    // an arbitrary file, and `include!`/`include_bytes!` splice in bytes no
    // scan saw — either is a complete bypass of every invariant in this
    // file. `include_str!` is exempt: it produces a &'static str used by
    // docs, not code, and the package-boundary check already constrains
    // where it may point.
    for src in ["izanagi_kit/src", "izanagi/src"] {
        let sources = library_sources(&repo_root().join(src));
        assert!(!sources.is_empty(), "found no sources under {src}");
        for (name, code) in sources {
            for needle in ["#[path", "include!(", "include_bytes!("] {
                assert!(
                    !code.contains(needle),
                    "{name} contains `{needle}` — compiled code must live in \
                     the file the scanners read, not arrive by reference"
                );
            }
        }
    }
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
fn every_named_enforcement_site_is_a_file_that_exists() {
    // The map above only counts if the files it names are real: renaming or
    // deleting a check would leave SPEC.md pointing at a ghost — an
    // invariant that reads as enforced and is not.
    for (g, site) in enforcement_sites() {
        for token in site.split(|c: char| {
            !(c.is_ascii_alphanumeric() || c == '_' || c == '/' || c == '.' || c == '-')
        }) {
            if token.ends_with(".rs") || token.ends_with(".md") || token.ends_with(".sh") {
                assert!(
                    token.contains('/'),
                    "{g}'s enforcement site names `{token}` without a path — \
                     write it relative to the repo root so it can be checked"
                );
                assert!(
                    repo_root().join(token).exists(),
                    "{g} claims {token} as its enforcement, but that file does \
                     not exist"
                );
            }
        }
    }
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
