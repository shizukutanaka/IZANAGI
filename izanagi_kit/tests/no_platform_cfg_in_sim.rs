//! The same source must compile to the same simulation on every target,
//! profile, and feature set — nothing may select different code.
//!
//! `hashes_are_width_independent.rs` (SPEC G9) keeps pointer-sized *values*
//! out of the hash, and `hashes_are_endian_independent.rs` (G10) keeps
//! native-endian *bytes* out of it. Both work by banning things that can be
//! named in code: a `usize` `DetHash` impl, a `to_ne_bytes` call. Neither of
//! them stops the third way of making two honest builds differ:
//!
//! ```text
//! #[cfg(target_pointer_width = "64")]
//! hash.write_u64(state);
//! #[cfg(target_pointer_width = "32")]
//! hash.write_u32(state);
//! ```
//!
//! Not one banned token appears there — no `usize`, no `to_ne_bytes`, no
//! `HashMap`, no clock — and the hash still differs between targets, because
//! it is not a *value* that reached the hash but a different *program*. The
//! same applies to `debug_assertions` (debug vs release), `feature` (this
//! crate ships none — a `cfg(feature)` item is either dead code today or a
//! second simulation tomorrow), `panic` strategy, and the `unix`/`windows`
//! shorthand families. Conditional compilation is the one mechanism that can
//! swap whole code paths while leaving every value-level check green.
//!
//! Measured today: zero target/profile/feature predicates in library code —
//! the design is right by discipline, exactly as it was for G9 and G10
//! before those became invariants. This file is SPEC.md **G11**: it makes
//! the absence enforced rather than accidental.
//!
//! The rule is a whitelist, matching the repository's convention that a new
//! entry must come with its reason: the only atoms a `cfg` predicate may
//! name are `test`, `doctest`, `doc`, and `docsrs` (compile-time-only
//! markers that cannot change what a consumer's binary runs), plus the
//! combinators `not`/`any`/`all`. `cfg!` is banned outright in library code:
//! every predicate it could legitimately evaluate here is in the whitelist,
//! and every one of those would mean the simulation behaves one way under
//! `cargo test` and another way shipped — which is the same defect wearing a
//! smaller coat.
//!
//! The engine is deliberately out of scope: `izanagi` promises same-input
//! determinism for a single build, not bit-identity across platforms — its
//! math is `f32` — and a backend is expected to be platform-shaped (a
//! Windows console is not a Unix terminal). The kit is the crate whose
//! promise this invariant protects.
//!
//! Adjacent holes close here too rather than in files of their own — the
//! manifest tables and Cargo config files that feed the compiler outside
//! the sections the other scanners read: `[target.'cfg(...)'.dependencies]`
//! is a platform-conditional dependency invisible to the `[dependencies]`
//! scan, `[features]` declares inputs to a `cfg(feature)` nobody may write,
//! `[build-dependencies]` feeds a build script that is already banned,
//! `[lints]`/`[patch]`/`[replace]` add configuration the gate never reads,
//! and `.cargo/config.toml` can pass `rustflags` — including `--cfg` — that
//! bypass this file's whole predicate scan. None exist today; now none may.
//!

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crate directory has a parent")
        .to_path_buf()
}

fn kit_src() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// The only atoms a conditional-compilation predicate may name in library
/// code. Everything else — `target_os`, `target_pointer_width`,
/// `target_endian`, `unix`, `windows`, `debug_assertions`, `feature`,
/// `panic`, and every cfg key Rust adds in the future — makes the compiled
/// program depend on something the input does not.
const ALLOWED_ATOMS: &[&str] = &[
    // combinators
    "not", "any", "all",
    // compile-time-only markers: a `#[cfg(test)]` module is not shipped code,
    // `doc`/`doctest` exist only while documenting, and `docsrs` is set only
    // by docs.rs's own build
    "test", "doctest", "doc", "docsrs",
];

/// Library sources, keyed by path relative to `src/`. `src/bin/` is excluded:
/// a CLI binary's job is to respond to the machine it runs on. Everything
/// before a file's first `#[cfg(test)]` is library code; line comments are
/// stripped so that prose — including this file's own documentation when it
/// is quoted — cannot trip the scan.
///
/// The order of those two steps matters: the boundary search must run on
/// comment-stripped text. lib.rs used to discuss `#[cfg(test)]` inside a `//`
/// comment, and a scanner that searches the raw source found *that* marker
/// first — truncating the file at 89 characters of doc header and silently
/// never seeing the `cfg_attr` lint gate or the `cfg(doctest)` wiring that
/// came after it. The sibling scanners shared the order; this one does not.
fn library_sources() -> BTreeMap<String, String> {
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
                let impl_end = stripped.find("#[cfg(test)]").unwrap_or(stripped.len());
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                out.insert(rel, stripped[..impl_end].to_string());
            }
        }
    }
    let mut out = BTreeMap::new();
    let root = kit_src();
    walk(&root, &root, &mut out);
    out
}

/// True when the `cfg` at `pos` is attribute-shaped: `#[cfg(...)]` or
/// `#![cfg(...)]`. A `fn cfg(` or `x::cfg(` is a function, not conditional
/// compilation.
fn preceded_by_attr(code: &str, pos: usize) -> bool {
    code[..pos]
        .trim_end()
        .strip_suffix('[')
        .map(|s| {
            // `#[cfg(...)]` leaves a `#` before the bracket, `#![cfg(...)]`
            // an inner-attribute `#!` — both are attributes.
            let s = s.trim_end();
            s.ends_with('#') || s.ends_with("#!")
        })
        .unwrap_or(false)
}

/// The text inside a parenthesised group whose opening `(` was just consumed.
/// Reads until the matching `)`; callers use it on input known to contain one.
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

/// For `cfg_attr(pred, attr...)`, only the predicate — everything up to the
/// first comma at the top level. The trailing items are the attribute being
/// applied, not part of the condition.
fn first_top_level_arg(pred: &str) -> &str {
    let mut depth = 0usize;
    for (i, c) in pred.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => return &pred[..i],
            _ => {}
        }
    }
    pred
}

/// Every conditional-compilation predicate in `code`, as `(form, predicate)`
/// with form one of `"cfg"`, `"cfg!"`, `"cfg_attr"`.
///
/// `match_indices("cfg")` alone is not enough: it would also match the `cfg`
/// in a `fn cfg(...)`, a `some::cfg(...)`, or a `macro_rules! cfg`. The word
/// boundary is checked, `cfg_attr` and `cfg!` are taken in that order, and a
/// bare `cfg(` counts only when it is attribute-shaped.
fn cfg_predicates(code: &str) -> Vec<(&'static str, String)> {
    let bytes = code.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(rel) = code[i..].find("cfg") {
        let at = i + rel;
        i = at + 3;
        // `cfg` must start a token: `my_cfg(` is not conditional compilation.
        if at > 0 && (bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_') {
            continue;
        }
        let after = &code[at + 3..];
        let (form, rest) = if let Some(r) = after.strip_prefix("_attr") {
            if !preceded_by_attr(code, at) {
                continue;
            }
            ("cfg_attr", r)
        } else if let Some(r) = after.strip_prefix('!') {
            // `path::cfg!(...)` is a differently-named macro, not this one.
            if code[..at].trim_end().ends_with(':') {
                continue;
            }
            ("cfg!", r)
        } else {
            if !preceded_by_attr(code, at) {
                continue;
            }
            ("cfg", after)
        };
        let Some(inner) = rest.trim_start().strip_prefix('(') else {
            continue;
        };
        let balanced = take_balanced(inner);
        let pred = if form == "cfg_attr" {
            first_top_level_arg(balanced)
        } else {
            balanced
        };
        out.push((form, pred.trim().to_string()));
    }
    out
}

/// Identifier-shaped atoms in a predicate, with `"..."` string literals
/// removed: `target_os = "linux"` yields `target_os` alone, so the quoted
/// value cannot masquerade as an allowed word.
fn predicate_atoms(pred: &str) -> Vec<String> {
    let mut cleaned = String::new();
    let mut in_str = false;
    for c in pred.chars() {
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

#[test]
fn library_code_compiles_without_platform_profile_or_feature_conditionals() {
    let mut offenders: Vec<String> = Vec::new();
    for (name, code) in library_sources() {
        for (form, pred) in cfg_predicates(&code) {
            if form == "cfg!" {
                offenders.push(format!(
                    "{name}: `cfg!({pred})` — a cfg! expression evaluates the \
                     build environment inside running code. Whatever it \
                     selects on differs between builds, so the compiled \
                     program is no longer the same one the tests measured."
                ));
                continue;
            }
            for atom in predicate_atoms(&pred) {
                if !ALLOWED_ATOMS.contains(&atom.as_str()) {
                    offenders.push(format!(
                        "{name}: `{form}({pred})` names `{atom}` — only \
                         {} are permitted, because every other condition \
                         makes two honest builds of the same source compile \
                         different simulation code",
                        ALLOWED_ATOMS.join("/")
                    ));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "conditional compilation selects on the platform, profile, or \
         feature set: {offenders:#?}\n\nThe pinned hashes compare runs, not \
         builds — they cannot see a code path that exists only on another \
         target. Keep simulation code unconditional; width, endianness, and \
         profile must all be invisible to it."
    );
}

#[test]
fn no_manifest_section_or_cargo_config_smuggles_build_variation() {
    // Every table header below would add inputs to the build that the
    // section-level scanners in this file and global_invariants_hold.rs
    // never read: `target` is a conditional dependency or flag source,
    // `features` declares inputs to a `cfg(feature)` no source file may
    // name, `build-dependencies` feeds a build.rs that is already banned,
    // and `lints`/`patch`/`replace` are configuration the gate ignores.
    // Banning the header atoms keeps these manifests' grammar closed.
    const BANNED_SECTION_ATOMS: &[&str] = &[
        "target",
        "features",
        "build-dependencies",
        "lints",
        "patch",
        "replace",
    ];
    for manifest_rel in ["Cargo.toml", "izanagi_kit/Cargo.toml", "izanagi/Cargo.toml"] {
        let manifest = fs::read_to_string(repo_root().join(manifest_rel))
            .unwrap_or_else(|e| panic!("reading {manifest_rel}: {e}"));
        for line in manifest.lines().map(str::trim) {
            let Some(rest) = line.strip_prefix('[') else {
                continue;
            };
            // `[[bin]]` and `[a.b]` both reduce to their bare keys: take the
            // header text and split it on '.' so `[target.'cfg(unix)'.d]`
            // yields `target` and `[workspace.lints]` yields `lints` too.
            let header = rest.trim_start_matches('[');
            let header = header.split(']').next().unwrap_or(header);
            for key in header.split('.') {
                let key = key.trim().trim_matches(|c| c == '\'' || c == '"');
                assert!(
                    !BANNED_SECTION_ATOMS.contains(&key),
                    "{manifest_rel} declares `{line}` — the `{key}` table is \
                     build configuration the gate never reads. The section \
                     grammar of these manifests is closed."
                );
            }
        }
    }

    // `.cargo/config.toml` (or the extensionless `config`) is read from the
    // package directory upward: `build.rustflags` there could pass `--cfg`
    // past the predicate scan above, or `--cap-lints allow` to demote the
    // deny-level safety lints. The ban covers the workspace root and both
    // member crates — anywhere Cargo would look while building them.
    for dir in ["", "izanagi_kit", "izanagi"] {
        for name in ["config", "config.toml"] {
            let path = repo_root().join(dir).join(".cargo").join(name);
            assert!(
                !path.exists(),
                "{} exists — a repo-checked-in cargo config can pass rustflags \
                 (including `--cfg` and `--cap-lints`) around every source \
                 scan in this suite",
                path.display()
            );
        }
    }
}

#[test]
fn the_scanner_finds_every_form_that_exists_in_library_code() {
    // Vacuity guard, both directions: the scan must find the cfg constructs
    // library code actually contains today (lib.rs's `cfg_attr(not(test))`
    // lint gate and `cfg(doctest)` README wiring), or a silent zero would
    // make the check above pass on an empty scan.
    let mut forms: Vec<&'static str> = Vec::new();
    let mut total = 0usize;
    for (_, code) in library_sources() {
        for (form, _) in cfg_predicates(&code) {
            forms.push(form);
            total += 1;
        }
    }
    assert!(
        total >= 2,
        "found only {total} cfg predicates across the kit's library code — \
         lib.rs alone carries two; has the scanner or the source layout \
         changed?"
    );
    for expected in ["cfg", "cfg_attr"] {
        assert!(
            forms.contains(&expected),
            "no `{expected}` found in library code — the scanner must still \
             see lib.rs's documented markers before its absences mean anything"
        );
    }
}

#[test]
fn the_predicate_parser_accepts_the_markers_and_rejects_the_platform() {
    // Synthetic, in both directions — the mutation check for the rule. Every
    // predicate that is legal today must pass, and every shape that could
    // smuggle a platform fork must produce the offending atom by name.
    let legal = [
        "#[cfg(test)]",
        "#[cfg(doctest)]",
        "#[cfg(doc)]",
        "#![cfg_attr(not(test), deny(clippy::panic))]",
        "#[cfg_attr(docsrs, doc(cfg(all())))]",
        "#[ cfg ( test ) ]",
    ];
    for snippet in legal {
        let preds = cfg_predicates(snippet);
        assert_eq!(
            preds.len(),
            1,
            "expected exactly one predicate in {snippet:?}"
        );
        let (_, pred) = &preds[0];
        for atom in predicate_atoms(pred) {
            assert!(
                ALLOWED_ATOMS.contains(&atom.as_str()),
                "{snippet:?} produced non-whitelisted atom {atom:?} — the \
                 parser must not flag the markers the crate actually uses"
            );
        }
    }

    for (snippet, atom) in [
        ("#[cfg(target_os = \"linux\")]", "target_os"),
        (
            "#[cfg(target_pointer_width = \"64\")]",
            "target_pointer_width",
        ),
        ("#[cfg(target_endian = \"big\")]", "target_endian"),
        ("#[cfg(unix)]", "unix"),
        ("#[cfg(windows)]", "windows"),
        ("#[cfg(debug_assertions)]", "debug_assertions"),
        ("#[cfg(feature = \"fast\")]", "feature"),
        ("#[cfg(panic = \"abort\")]", "panic"),
        ("#[cfg(all(unix, test))]", "unix"),
        ("#[cfg(any(test, target_arch = \"wasm32\"))]", "target_arch"),
        (
            "#[cfg_attr(target_vendor = \"apple\", allow(x))]",
            "target_vendor",
        ),
    ] {
        let preds = cfg_predicates(snippet);
        assert_eq!(preds.len(), 1, "expected one predicate in {snippet:?}");
        let (_, pred) = &preds[0];
        assert!(
            predicate_atoms(pred).iter().any(|a| a == atom),
            "{snippet:?} must surface {atom:?}, got {pred:?}"
        );
    }

    // cfg! is banned as a form regardless of predicate: even `cfg!(test)`
    // makes shipped behaviour differ from tested behaviour.
    let preds = cfg_predicates("let _ = cfg!(test);");
    assert_eq!(preds.len(), 1);
    assert_eq!(preds[0].0, "cfg!");

    // And the non-cfg spellings must not trip the scanner at all.
    for decoy in [
        "fn cfg(seed: u64) -> u64 { seed }",
        "let cfg = load_cfg();",
        "my_cfg(x)",
        "foo::cfg(x)",
        "path::cfg!(x)",
    ] {
        assert!(
            cfg_predicates(decoy).is_empty(),
            "{decoy:?} is a function or path named cfg, not conditional \
             compilation — a scanner that flags it flags nothing but spelling"
        );
    }
}
