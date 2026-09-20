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

fn test_module_boundary(src: &str) -> Option<usize> {
    // The marker only counts in real code at the start of its own line. A
    // fake inside a comment, a string or char literal, or sharing its line
    // with other tokens would truncate the impl region early and hide code
    // from every scan below — so the boundary is found by a tiny lexer:
    // block comments nest, raw strings carry their own delimiter count, and
    // `'a` may be a char literal or a lifetime.
    let b = src.as_bytes();
    let mut i = 0usize;
    while i < b.len() {
        match b[i] {
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                let mut depth = 1usize;
                i += 2;
                while i < b.len() && depth > 0 {
                    if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
                        depth += 1;
                        i += 2;
                    } else if b[i] == b'*' && b.get(i + 1) == Some(&b'/') {
                        depth -= 1;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
            }
            b'"' => {
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    if b[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                i += 1;
            }
            b'r' => {
                // Raw string r"..." / r#"..."# — a string only when any #s
                // are followed by '"'; otherwise r is identifier text.
                let mut j = i + 1;
                while b.get(j) == Some(&b'#') {
                    j += 1;
                }
                if b.get(j) == Some(&b'"') {
                    let hashes = j - i - 1;
                    i = j + 1;
                    while i < b.len() {
                        if b[i] == b'"' {
                            let mut k = 0usize;
                            while k < hashes && b.get(i + 1 + k) == Some(&b'#') {
                                k += 1;
                            }
                            if k == hashes {
                                i += 1 + hashes;
                                break;
                            }
                        }
                        i += 1;
                    }
                } else {
                    i += 1;
                }
            }
            b'\'' => match (b.get(i + 1), b.get(i + 2)) {
                // 'x' / '\n' are char literals; 'a followed by code is a
                // lifetime.
                (Some(&b'\\'), _) => {
                    i += 2;
                    while i < b.len() && b[i] != b'\'' {
                        if b[i] == b'\\' {
                            i += 1;
                        }
                        i += 1;
                    }
                    i += 1;
                }
                (_, Some(&b'\'')) => i += 3,
                _ => i += 1,
            },
            b'#' if src[i..].starts_with("#[cfg(test)]") => {
                let start = src[..i].rfind('\n').map_or(0, |p| p + 1);
                if src[start..i].trim().is_empty() {
                    return Some(i);
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    None
}

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
                let src = fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
                let end = test_module_boundary(&src).unwrap_or(src.len());
                let stripped = src[..end]
                    .lines()
                    .filter(|l| !l.trim_start().starts_with("//"))
                    .collect::<Vec<_>>()
                    .join("\n");
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                out.insert(rel, stripped);
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(src_root, src_root, &mut out);
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

/// Every comma-separated item at the top level of `inner`, in order.
fn top_level_items(inner: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (i, c) in inner.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                out.push(&inner[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&inner[start..]);
    out
}

/// If `item` — one top-level item from a `cfg_attr` apply list — is itself
/// a `cfg`/`cfg!`/`cfg_attr` construct, surface its predicate as an entry
/// in `out` so it faces the same whitelist as a real attribute. A nested
/// `cfg_attr` recurses: its apply list can hide the same trick again.
///
/// Only a *top-level* item is treated this way: `doc(cfg(all()))` keeps its
/// `cfg` inside `doc`'s own parentheses, where it is an argument to the doc
/// attribute (the docs.rs badge idiom), not conditional compilation being
/// applied to the item.
fn push_applied_cfg(item: &str, out: &mut Vec<(&'static str, String)>) {
    let item = item.trim();
    for (form, name) in [("cfg_attr", "cfg_attr"), ("cfg!", "cfg!"), ("cfg", "cfg")] {
        let Some(rest) = item.strip_prefix(name) else {
            continue;
        };
        let Some(inner) = rest.trim_start().strip_prefix('(') else {
            continue;
        };
        let balanced = take_balanced(inner);
        if form == "cfg_attr" {
            let items = top_level_items(balanced);
            out.push((
                "cfg_attr",
                items.first().copied().unwrap_or("").trim().to_string(),
            ));
            for extra in &items[1..] {
                push_applied_cfg(extra, out);
            }
        } else {
            out.push((form, balanced.trim().to_string()));
        }
        return;
    }
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
        if form == "cfg_attr" {
            // The predicate is only the first item. Everything after it is
            // the attribute being applied — where `#[cfg_attr(not(test),
            // cfg(unix))]` hides a platform conditional the predicate scan
            // alone never reads. Each applied item that is itself a
            // cfg/cfg!/cfg_attr is conditional compilation smuggled past
            // the whitelist; surface its own predicate so it is checked.
            for item in top_level_items(balanced).into_iter().skip(1) {
                push_applied_cfg(item, &mut out);
            }
        }
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
    for (name, code) in library_sources(&kit_src()) {
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

/// The `key` half of every `key = value` pair on a manifest line, including
/// keys inside inline `{ ... }` tables. String literals are blanked first so
/// a description that merely *contains* `=` cannot masquerade as a key.
fn manifest_line_keys(line: &str) -> Vec<String> {
    let mut dequoted = String::with_capacity(line.len());
    let mut in_str: Option<char> = None;
    for c in line.chars() {
        match in_str {
            Some(q) if c == q => in_str = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => in_str = Some(c),
            // A `#` outside a string opens a TOML comment to end of line —
            // a comment that happens to contain `key =` is not a key.
            None if c == '#' => break,
            None => dequoted.push(c),
        }
    }
    let mut keys = Vec::new();
    let bytes = dequoted.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b != b'=' || i == 0 {
            continue;
        }
        // Skip whitespace between the key and `=` (`debug-assertions = ...`).
        let mut end = i;
        while end > 0 && (bytes[end - 1] == b' ' || bytes[end - 1] == b'\t') {
            end -= 1;
        }
        let mut start = end;
        while start > 0 && {
            let c = bytes[start - 1];
            c.is_ascii_alphanumeric() || c == b'_' || c == b'-'
        } {
            start -= 1;
        }
        let key = &dequoted[start..end];
        if !key.is_empty() {
            keys.push(key.to_string());
        }
    }
    keys
}

#[test]
fn no_manifest_section_or_cargo_config_smuggles_build_variation() {
    // Every table header below would add inputs to the build that the
    // section-level scanners in this file and global_invariants_hold.rs
    // never read: `target` is a conditional dependency or flag source,
    // `features` declares inputs to a `cfg(feature)` no source file may
    // name, `build-dependencies` feeds a build.rs that is already banned,
    // and `lints`/`patch`/`replace` are configuration the gate ignores.
    // `bin`/`test`/`bench`/`example` are the `[[...]]` target tables: they
    // can redirect `path` at unscanned code or set `harness = false`, which
    // compiles and "runs" a suite that executes nothing. Banning the header
    // atoms keeps these manifests' grammar closed.
    const BANNED_SECTION_ATOMS: &[&str] = &[
        "target",
        "features",
        "build-dependencies",
        "lints",
        "patch",
        "replace",
        "bin",
        "test",
        "bench",
        "example",
    ];
    // The same smuggles exist as bare keys (in `[package]`, `[lib]`, or an
    // inline table): `autotests`/`autobenches`/`autoexamples`/`autobins`
    // switch off auto-discovery so the filesystem enumeration in gate.sh
    // stops agreeing with what cargo builds; `harness`/`test`/`bench`/
    // `doctest` can quietly disable a runner; `crate-type`/`proc-macro`
    // change what linking the library even produces. `links` declares a
    // native library the manifest's empty [dependencies] never lists —
    // cargo refuses it without a build script today (build.rs is banned),
    // but the grammar stays closed only if the key itself is named.
    const BANNED_KEYS: &[&str] = &[
        "harness",
        "autotests",
        "autobenches",
        "autoexamples",
        "autobins",
        "crate-type",
        "proc-macro",
        "test",
        "bench",
        "doctest",
        "links",
    ];
    // Profile tables exist legitimately (the workspace root sets opt-level/
    // lto/strip) — but three of their keys rewrite program semantics rather
    // than tuning output: `debug-assertions` switches every `debug_assert!`
    // off, `overflow-checks` flips wrap-vs-panic, and `panic = "abort"`
    // removes unwinding entirely.
    const BANNED_PROFILE_KEYS: &[&str] = &["debug-assertions", "overflow-checks", "panic"];
    for manifest_rel in ["Cargo.toml", "izanagi_kit/Cargo.toml", "izanagi/Cargo.toml"] {
        let manifest = fs::read_to_string(repo_root().join(manifest_rel))
            .unwrap_or_else(|e| panic!("reading {manifest_rel}: {e}"));
        let mut section = String::new();
        for line in manifest.lines().map(str::trim) {
            if let Some(rest) = line.strip_prefix('[') {
                // `[[bin]]` and `[a.b]` both reduce to their bare keys: take
                // the header text and split it on '.' so
                // `[target.'cfg(unix)'.d]` yields `target` and
                // `[workspace.lints]` yields `lints` too.
                let header = rest.trim_start_matches('[');
                let header = header.split(']').next().unwrap_or(header);
                section = header.trim().to_string();
                for key in header.split('.') {
                    let key = key.trim().trim_matches(|c| c == '\'' || c == '"');
                    assert!(
                        !BANNED_SECTION_ATOMS.contains(&key),
                        "{manifest_rel} declares `{line}` — the `{key}` table is \
                         build configuration the gate never reads. The section \
                         grammar of these manifests is closed."
                    );
                }
                // `[workspace]` itself is required at the root, but its
                // `dependencies` subtable is a place to hide a registry
                // dependency where the path-only scan of [dependencies]
                // never looks.
                assert!(
                    section != "workspace.dependencies",
                    "{manifest_rel} declares `[workspace.dependencies]` — \
                     dependencies declared there are invisible to the \
                     zero-dependency scans of the member manifests"
                );
                continue;
            }
            for key in manifest_line_keys(line) {
                assert!(
                    !BANNED_KEYS.contains(&key.as_str()),
                    "{manifest_rel} sets `{key}` — this key can redirect or \
                     disable build/test targets without changing a line of \
                     source. The manifest grammar is closed."
                );
                if section.starts_with("profile") {
                    assert!(
                        !BANNED_PROFILE_KEYS.contains(&key.as_str()),
                        "{manifest_rel} sets `{key}` inside `[{section}]` — this \
                         profile key changes program semantics (assertions, \
                         overflow behaviour, panic strategy), so the same \
                         source compiles to a different simulation"
                    );
                }
                // `[lib]` is the one target table left legal — but its
                // `path` is the library's root file. Pointing it anywhere
                // but src/lib.rs compiles a file the source scanners never
                // enumerate (verified by injection: a redirected root
                // carried banned needles while every scan stayed green).
                if section == "lib" && key == "path" {
                    let squashed: String = line.chars().filter(|c| !c.is_whitespace()).collect();
                    assert!(
                        squashed == "path=\"src/lib.rs\"",
                        "{manifest_rel} redirects the lib root with `{line}` — \
                         the library's entry point must be the file the \
                         scanners read"
                    );
                }
            }
        }
    }

    // `.cargo/config.toml` (or the extensionless `config`) is read from the
    // package directory upward: `build.rustflags` there could pass `--cfg`
    // past the predicate scan above, or `--cap-lints allow` to demote the
    // deny-level safety lints. `rust-toolchain{,.toml}` and `Cross.toml`
    // silently change *which compiler* or *which target* builds the code —
    // the same class of unlisted build input. The ban covers the workspace
    // root and both member crates.
    for dir in ["", "izanagi_kit", "izanagi"] {
        for rel in [
            ".cargo/config",
            ".cargo/config.toml",
            "rust-toolchain",
            "rust-toolchain.toml",
            "Cross.toml",
            // A clippy.toml can soften the lints the suite relies on
            // (`allow-unwrap-in-tests`, a different `msrv`, ...) — the deny
            // attributes mean nothing if the linter's own config rewrites
            // what they measure.
            "clippy.toml",
            ".clippy.toml",
        ] {
            let path = repo_root().join(dir).join(rel);
            assert!(
                !path.exists(),
                "{} exists — repo-checked-in build configuration outside the \
                 manifests can change flags, toolchain, or target around \
                 every scan in this suite",
                path.display()
            );
        }
        // rustfmt.toml exists legitimately (izanagi/ carries the workspace
        // style), but three of its keys shrink what `cargo fmt --check`
        // checks: `ignore` lists paths to skip, `disable_all_formatting`
        // turns the formatter off, and `skip_children` skips out-of-line
        // modules. The file may exist; the escape hatches may not.
        for name in ["rustfmt.toml", ".rustfmt.toml"] {
            let path = repo_root().join(dir).join(name);
            if !path.exists() {
                continue;
            }
            let cfg = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
            for line in cfg.lines().map(str::trim) {
                for key in manifest_line_keys(line) {
                    for banned in ["ignore", "disable_all_formatting", "skip_children"] {
                        assert!(
                            key != banned,
                            "{} sets `{key}` — this rustfmt key removes files \
                             from `cargo fmt --check`, so unformatted code \
                             would pass the gate",
                            path.display()
                        );
                    }
                }
            }
        }
    }

    // Cargo.lock is deliberately gitignored (library crates do not ship a
    // lock), so it cannot be a checked input — asserting its contents would
    // fail a fresh clone. Its content is pinned by a different route: the
    // zero-dependency manifests make a third-party entry impossible to
    // resolve.
}

#[test]
fn the_scanner_finds_every_form_that_exists_in_library_code() {
    // Vacuity guard, both directions: the scan must find the cfg constructs
    // library code actually contains today (lib.rs's `cfg_attr(not(test))`
    // lint gate and `cfg(doctest)` README wiring), or a silent zero would
    // make the check above pass on an empty scan.
    let mut forms: Vec<&'static str> = Vec::new();
    let mut total = 0usize;
    for (_, code) in library_sources(&kit_src()) {
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

    // A conditional nested in a cfg_attr's apply list must surface too:
    // `#[cfg_attr(not(test), cfg(unix))]` applied `#[cfg(unix)]` while the
    // predicate-level scan saw only `not(test)` — verified by injection.
    // `doc(cfg(...))` is exempt: its cfg is an argument to `doc`, not an
    // applied item.
    for (snippet, atom) in [
        ("#[cfg_attr(not(test), cfg(unix))]", "unix"),
        (
            "#[cfg_attr(test, cfg_attr(any(), cfg(target_endian = \"big\")))]",
            "target_endian",
        ),
        (
            "#![cfg_attr(any(), cfg!(target_arch = \"wasm32\"))]",
            "target_arch",
        ),
    ] {
        let preds = cfg_predicates(snippet);
        assert!(
            preds
                .iter()
                .skip(1)
                .any(|(_, pred)| predicate_atoms(pred).iter().any(|a| a == atom)),
            "{snippet:?} must surface nested atom {atom:?} as its own \
             predicate, got {preds:?}"
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
