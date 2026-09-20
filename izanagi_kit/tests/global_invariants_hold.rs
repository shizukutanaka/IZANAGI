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

use std::collections::{BTreeMap, BTreeSet};
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
    // the sibling kit, by path, used only by examples/kit_bridge.rs — and
    // the path itself is pinned: a `path =` that points *somewhere else*
    // keeps the shape check green while the build reads a crate no scanner
    // enumerates (verified by injection: the line retargeted to a renamed
    // sibling copy carrying `env::var`, every check stayed green).
    for krate in CRATES {
        let manifest = read(&format!("{krate}/Cargo.toml"));
        let deps = manifest_section(&manifest, "dev-dependencies");
        for dep in &deps {
            let squashed: String = dep.chars().filter(|c| !c.is_whitespace()).collect();
            assert!(
                squashed == "izanagi_kit={path=\"../izanagi_kit\"}",
                "{krate} declares dev-dependency `{dep}` — the sibling path \
                 dep is pinned verbatim: it must resolve to the crate the \
                 scanners read"
            );
        }
        for dep in deps {
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
         not/any/all combinators — cfg_attr apply items are walked \
         recursively with the same allowlist; manifest section grammar is \
         closed — no target/features/lints/patch/replace/build-dependencies \
         tables, no [[bin]]/[[test]]/[[bench]]/[[example]] target tables, \
         no harness/auto*/crate-type/proc-macro/doctest/links keys, [lib] \
         path is pinned to izanagi_kit/src/lib.rs, no semantic profile keys; \
         no .cargo/rust-toolchain/Cross.toml files; tools/gate.sh unsets \
         every CARGO*/RUST*/RUSTUP* env var and pins CARGO_HOME into \
         target/)",
    );
    m.insert(
        "G12",
        "izanagi/tests/no_unordered_containers.rs (engine src may not \
         mention HashMap/HashSet/hash_map/hash_set/DefaultHasher/\
         RandomState/SipHash/BuildHasher at all) + \
         izanagi_kit/tests/no_nondeterminism_in_sim.rs hash-map allowlist \
         for the kit",
    );
    m.insert(
        "G8",
        "this file: g8_*, plus izanagi_kit/tests/msrv_is_respected.rs for the code",
    );
    m
}

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

/// `use` statements flattened so `use a::{b, c::{d}}` also appears as the
/// separate paths `a::b` and `a::c::d`. A brace (or brace + `as`) import is
/// how a banned `a::b` token disappears from the text — `use std::env::{var}`
/// compiles `env::var` without ever spelling it (verified by injection).
/// `x as y` keeps the real path: the alias is the evasion, not the path.
fn flattened_use_paths(code: &str) -> Vec<String> {
    fn split_top_level_commas(items: &str) -> Vec<&str> {
        let mut out = Vec::new();
        let mut depth = 0usize;
        let mut start = 0usize;
        for (i, c) in items.char_indices() {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                ',' if depth == 0 => {
                    out.push(&items[start..i]);
                    start = i + 1;
                }
                _ => {}
            }
        }
        out.push(&items[start..]);
        out
    }
    fn expand(prefix: &str, items: &str, out: &mut Vec<String>) {
        for item in split_top_level_commas(items) {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            if let Some(brace) = item.find("::{") {
                let inner = &item[brace + 3..item.len().saturating_sub(1)];
                expand(&format!("{prefix}::{}", &item[..brace]), inner, out);
            } else if let Some(inner) = item.strip_prefix('{') {
                expand(prefix, inner.strip_suffix('}').unwrap_or(inner), out);
            } else {
                // `a::b as c` keeps the whole spelling — `a::b` needles still
                // match, and an `a::b as `-needle can see the alias import.
                let (base, alias) = match item.split_once(" as ") {
                    Some((b, a)) => (b.trim(), Some(a.trim())),
                    None => (item, None),
                };
                match base {
                    "self" => out.push(match alias {
                        Some(a) => format!("{prefix} as {a}"),
                        None => prefix.to_string(),
                    }),
                    "*" | "" => {}
                    b => out.push(format!(
                        "{prefix}::{b}{}",
                        alias.map_or(String::new(), |a| format!(" as {a}"))
                    )),
                }
            }
        }
    }
    let mut flat = Vec::new();
    let mut i = 0usize;
    while let Some(p) = code[i..].find("use ") {
        let start = i + p;
        // `use` must start a statement: preceded only by non-ident text —
        // `reuse`/`misuse` or `x::use` (not legal Rust) must not count. The
        // previous byte is ASCII here: a multi-byte char ends in bytes that
        // are never alphanumeric, which is exactly the boundary we want.
        if start > 0 && {
            let p = code.as_bytes()[start - 1];
            p.is_ascii_alphanumeric() || p == b'_'
        } {
            i = start + 4;
            continue;
        }
        let end = match code[start..].find(';') {
            Some(e) => start + e,
            // A `use` without `;` is not a statement — doc prose can carry
            // the word; skip the token itself and keep scanning for a real
            // terminated `use` later in the file.
            None => {
                i = start + 4;
                continue;
            }
        };
        let stmt = code[start + 4..end].trim();
        // `use a::b::{c}` — expand from the crate root (a leading `::`? no:
        // statements are `use path;` where path may itself start with braces
        // or bare names like `crate::x`).
        if let Some(brace) = stmt.find("::{") {
            let head = stmt[..brace].to_string();
            let inner = stmt[brace + 3..]
                .strip_suffix('}')
                .unwrap_or(&stmt[brace + 3..]);
            expand(&head, inner, &mut flat);
        }
        flat.push(stmt.to_string());
        i = end + 1;
    }
    flat
}

/// The structural text of `src` from `offset` on: comments (line and nested
/// block), `"..."`/`r"..."` strings, and `'x'` char literals dropped, with a
/// space standing in for each dropped byte. Used to ask what the compiler
/// sees *after* the `#[cfg(test)]` boundary — a region every needle scan
/// deliberately never reads.
fn structural_tail(src: &str, offset: usize) -> String {
    let b = src.as_bytes();
    let mut out = String::new();
    let mut i = offset;
    while i < b.len() {
        match b[i] {
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    out.push(' ');
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                out.push_str("  ");
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
                out.push(' ');
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    if b[i] == b'\\' {
                        i += 1;
                    }
                    out.push(' ');
                    i += 1;
                }
                if i < b.len() {
                    out.push(' ');
                    i += 1;
                }
            }
            b'r' => {
                let mut j = i + 1;
                while b.get(j) == Some(&b'#') {
                    j += 1;
                }
                if b.get(j) == Some(&b'"') {
                    let hashes = j - i - 1;
                    for _ in 0..=hashes + 1 {
                        out.push(' ');
                    }
                    i = j + 1;
                    while i < b.len() {
                        out.push(' ');
                        if b[i] == b'"' {
                            let mut k = 0usize;
                            while k < hashes && b.get(i + 1 + k) == Some(&b'#') {
                                k += 1;
                            }
                            if k == hashes {
                                for _ in 0..=hashes {
                                    out.push(' ');
                                }
                                i += 1 + hashes;
                                break;
                            }
                        }
                        i += 1;
                    }
                } else {
                    out.push('r');
                    i += 1;
                }
            }
            b'\'' => match (b.get(i + 1), b.get(i + 2)) {
                (Some(&b'\\'), _) => {
                    out.push(' ');
                    i += 2;
                    while i < b.len() && b[i] != b'\'' {
                        if b[i] == b'\\' {
                            i += 1;
                        }
                        out.push(' ');
                        i += 1;
                    }
                    if i < b.len() {
                        out.push(' ');
                        i += 1;
                    }
                }
                (Some(_), Some(&b'\'')) => {
                    out.push_str("   ");
                    i += 3;
                }
                _ => {
                    out.push('\'');
                    i += 1;
                }
            },
            c => {
                out.push(c as char);
                i += 1;
            }
        }
    }
    out
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
    // Naming the lint is not the only way to lower it: an inner-scope
    // `#[allow]` on a *group* containing it overrides the outer deny just
    // as completely — `#[allow(clippy::restriction)]` silences
    // `unwrap_used`/`expect_used`/`panic` without naming any of them
    // (verified by injection: clippy stayed silent on a real unwrap).
    // `warnings`/`all`/`unused`/`deprecated` and friends are the same
    // shape on the rustc side. Every weakening attribute must name
    // specific lints, so a group atom is itself the offence.
    const GROUPS: &[&str] = &[
        "warnings",
        "all",
        "unused",
        "deprecated",
        "future_incompatible",
        "nonstandard_style",
        "rust_2018_idioms",
        "rust_2018_compatibility",
        "rust_2021_compatibility",
        "rust_2024_compatibility",
        // clippy's groups (`clippy::all`, `clippy::pedantic`, ...) appear
        // as bare atoms once `::` splits them — none is a lint name.
        "restriction",
        "pedantic",
        "nursery",
        "cargo",
        "complexity",
        "correctness",
        "perf",
        "style",
        "suspicious",
    ];

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
                    if GROUPS.contains(&atom.as_str()) {
                        offenders.push(format!(
                            "{name}: a weakening attribute names the lint \
                             group `{atom}` — a group allow at item scope \
                             overrides the crate-level denies without naming \
                             them; name the specific lint instead"
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

    // `src/bin/` is deliberately out of `library_sources` — a CLI answers to
    // its machine — but a bin target is its own crate root: the lib's
    // `#![forbid(unsafe_code)]` does not apply there, and no scanner read
    // the directory at all. Every shipped binary carries the attribute
    // itself and may not splice in code this suite never reads. `env!`
    // stays legal for cargo-provided `CARGO_*` constants
    // (`CARGO_PKG_VERSION` is the idiomatic version string) — anything else
    // bakes the build machine into a shipped binary.
    for bin_dir in ["izanagi_kit/src/bin", "izanagi/src/bin"] {
        let dir = repo_root().join(bin_dir);
        let mut found_any = false;
        // Cargo discovers `src/bin/*.rs` AND `src/bin/*/main.rs` as targets;
        // a directory walk covers both, plus nested files a `mod` could reach
        // had `mod` not been banned above.
        let mut stack = vec![dir.clone()];
        while let Some(d) = stack.pop() {
            let Ok(entries) = fs::read_dir(&d) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().map(|e| e == "rs") != Some(true) {
                    continue;
                }
                found_any = true;
                let name = path.display().to_string();
                let raw =
                    fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {name}: {e}"));
                let code = test_code(&raw);
                assert!(
                    code.contains("#![forbid(unsafe_code)]"),
                    "{name} is a bin crate root without `#![forbid(unsafe_code)]` \
                     — the lib-level forbid does not reach this target"
                );
                let body = code.replacen("#![forbid(unsafe_code)]", "", 1);
                assert!(
                    !body.contains("#![forbid(unsafe_code)]"),
                    "{name} declares `#![forbid(unsafe_code)]` twice"
                );
                for needle in ["unsafe", "include!(", "include_bytes!(", "#[path", "mod "] {
                    assert!(
                        !contains_token(&body, needle),
                        "{name} contains `{needle}` — a shipped binary may not \
                         take unsafe or splice in code the scans never read"
                    );
                }
                for arg in env_macro_args(&raw) {
                    assert!(
                        arg.as_deref()
                            .map(|a| a.starts_with("CARGO_"))
                            .unwrap_or(false),
                        "{name} bakes a non-CARGO_* build env var into a shipped \
                         binary ({arg:?})"
                    );
                }
            }
        }
        if bin_dir == "izanagi_kit/src/bin" {
            assert!(
                found_any,
                "izanagi_kit/src/bin scanned empty — gamec lives there, and \
                 an empty scan would pass vacuously"
            );
        }
    }
}

#[test]
fn nothing_compiles_after_the_test_module_boundary() {
    // Every scanner here cuts a file at the first line-start `#[cfg(test)]` —
    // that is the design: the test module may do what library code may not.
    // The cut only works if the boundary opens the file's LAST item; a pub
    // fn or impl block written below `mod tests` compiles and runs while no
    // needle in this suite ever sees its text. Verified by injection: a
    // `std::net::TcpStream` call after the boundary passed every check.
    for src in ["izanagi_kit/src", "izanagi/src"] {
        let root = repo_root().join(src);
        for (name, _) in library_sources(&root) {
            let raw = fs::read_to_string(root.join(&name))
                .unwrap_or_else(|e| panic!("reading {name}: {e}"));
            let Some(boundary) = test_module_boundary(&raw) else {
                continue;
            };
            let tail = structural_tail(&raw, boundary);
            let tail = tail.trim_start();
            let tail = tail
                .strip_prefix("#[cfg(test)]")
                .unwrap_or_else(|| panic!("{name}: boundary marker lost"));
            let tail = tail.trim_start();
            assert!(
                tail.starts_with("mod "),
                "{name}: the first `#[cfg(test)]` does not open a module — \
                 whatever follows the marker compiles but is never scanned"
            );
            let open = tail
                .find('{')
                .unwrap_or_else(|| panic!("{name}: test module has no body"));
            let mut depth = 0usize;
            let mut closed_at = None;
            // Bytes, not char_indices: `{`/`}` are ASCII and can never appear
            // inside a multi-byte char, and the position feeds a str slice.
            for (i, &c) in tail.as_bytes().iter().enumerate().skip(open) {
                match c {
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            closed_at = Some(i);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let closed =
                closed_at.unwrap_or_else(|| panic!("{name}: unbalanced braces after boundary"));
            assert!(
                tail[closed + 1..].trim().is_empty(),
                "{name} has code after the test module — cargo compiles it, \
                 but every scan stopped at `#[cfg(test)]` and never saw it"
            );
            // The module interior is scanned here and nowhere else: every
            // flat scan cut at the boundary, so an attribute inside `mod
            // tests` is invisible to them. `#[cfg(unix)] fn the_check()`
            // compiles on some platforms and vanishes on others — a test
            // that never runs while the suite stays green (proven by
            // injection past every check), and `cfg!`/`cfg_attr` fork or
            // weaken it the same way. `#[ignore]` is counted against the
            // same named-site allowlist the tests/ dirs already use, since
            // a skipped test exits green either way.
            for needle in ["#[cfg", "#![cfg", "cfg!(", "cfg_attr("] {
                assert!(
                    !tail.contains(needle),
                    "{name}: `{needle}` inside the test module — the suite                      must not fork or soften itself on build conditions its                      own scanners cannot see"
                );
            }
            const SRC_IGNORE_ALLOWLIST: &[(&str, usize, &str)] = &[(
                "savefile.rs",
                1,
                "print_golden_save is a regeneration helper, run with --ignored",
            )];
            let ignored = tail.matches("#[ignore").count();
            let allowed = SRC_IGNORE_ALLOWLIST
                .iter()
                .filter(|(f, _, _)| *f == name)
                .map(|(_, n, _)| *n)
                .sum();
            assert!(
                ignored <= allowed,
                "{name}: {ignored} `#[ignore]` inside the test module — a                  skipped test still exits the suite green; name it in                  SRC_IGNORE_ALLOWLIST with the reason"
            );
        }
    }
}

/// Panicking-macro sites that exist in library code, with the count
/// expected and the reason the site is a documented precondition rather
/// than a runtime-input panic path.
///
/// `deny(clippy::panic)` covers only `panic!` itself — `assert!`,
/// `debug_assert!`, `unreachable!`, `todo!` and `unimplemented!` lower to
/// the same trap without tripping the lint. The rule here is the same as
/// the non-determinism allowlist: every existing site is named with a
/// reason and its call-text prefix is frozen — a count alone cannot see
/// `debug_assert!` quietly strengthening to `assert!` (a debug-only audit
/// becoming a shipped panic; proven by injection), and a new site fails
/// the build until the
/// sentence is written — which is the moment to decide whether the site
/// should instead be a saturating/no-op return (G7's contract for
/// *runtime* input). Slice indexing `v[i]` is the one panic family this
/// does not enumerate: it is syntactically indistinguishable from valid
/// reads, and bounding it belongs to the code review the allowlist gate
/// cannot automate.
fn panicking_macro_allowlist() -> BTreeMap<&'static str, (&'static [&'static str], &'static str)> {
    let mut m = BTreeMap::new();
    m.insert(
        "izanagi_kit/rollback.rs",
        (
            &[
                "assert!(capacity > 0, \"SnapshotRing capacity mus",
                "assert!(stride > 0, \"SnapshotRing stride must be",
                "debug_assert_eq!(*bf, base_frame, \"window front tracks th",
            ][..],
            "SnapshotRing::new's zero-capacity/zero-stride guards are documented \
             programming-error preconditions; the debug_assert_eq self-checks \
             the window-base invariant in development builds only",
        ),
    );
    m.insert(
        "izanagi_kit/netinput.rs",
        (
            &[
                "assert!(lower_permille <= raise_permille, \"lower",
                "assert!(min_delay <= max_delay, \"min_delay must ",
                "assert!(window_cap > 0, \"window_cap must be > 0\"",
            ][..],
            "AdaptiveDelay::new's ordering/capacity preconditions — documented \
             panics on programmer error, not on network input",
        ),
    );
    m.insert(
        "izanagi_kit/timestep.rs",
        (
            &[
                "assert!(max_steps > 0, \"max_steps must be > 0\");",
                "assert!(steps_per_second > 0, \"steps_per_second ",
            ][..],
            "FixedTimestep::new requires a nonzero rate and cap — documented \
             constructor precondition",
        ),
    );
    m.insert(
        "izanagi_kit/identify.rs",
        (
            &["assert!(labels.len() >= sorted_kinds.len(), \"not"][..],
            "Identification::new requires at least as many labels as kinds — \
             documented constructor precondition",
        ),
    );
    m.insert(
        "izanagi_kit/wallet.rs",
        (
            &["debug_assert!(ok); other.deposit(c, amount); true } pu"][..],
            "post-deposit self-check, debug builds only — it is the audit \
             tool, not a panic path reachable from input",
        ),
    );
    m.insert(
        "izanagi_kit/pathfinding.rs",
        (
            &["debug_assert_eq!(cur, start, \"jump-point chain must termi"][..],
            "JPS chain-reconstruction invariant, debug builds only",
        ),
    );
    m.insert(
        "izanagi_kit/fov.rs",
        (
            &["debug_assert!(den > 0, \"Frac denominator must be posit"][..],
            "Frac denominator invariant, debug builds only",
        ),
    );
    m.insert(
        "izanagi_kit/replay.rs",
        (
            &[
                "unreachable!(\"i < max(len) so at least one side is So",
                "unreachable!(\"tick < max(len) so at least one side is",
            ][..],
            "the zip-longest (None, None) arms are unreachable by loop bound; \
             the tests that exercise the divergence table cover every arm",
        ),
    );
    m.insert(
        "izanagi/sprite.rs",
        (
            &["assert!(!frames.is_empty(), \"animation must have"][..],
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
fn panicking_macro_sites(code: &str) -> Vec<String> {
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
    let mut sites = Vec::new();
    for needle in MACROS {
        for (pos, _) in code.match_indices(needle) {
            let prev_ok = pos == 0
                || !(bytes[pos - 1].is_ascii_alphanumeric()
                    || bytes[pos - 1] == b'_'
                    || bytes[pos - 1] == b':'
                    || bytes[pos - 1] == b'.');
            if !prev_ok {
                continue;
            }
            // The site's identity is the macro plus a prefix of its call
            // text, so `debug_assert!` quietly strengthening to `assert!`
            // (a debug-only audit becoming a shipped panic — proven by
            // injection) changes the site even though the count does not.
            let tail: String = code[pos + needle.len()..]
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(40)
                .collect();
            sites.push(format!("{needle}{tail}"));
        }
    }
    sites.sort();
    sites
}

#[test]
fn g7_panicking_macros_are_frozen_at_named_sites() {
    let allowed = panicking_macro_allowlist();
    let mut live: Vec<String> = Vec::new();
    for src in ["izanagi_kit/src", "izanagi/src"] {
        for (name, code) in library_sources(&repo_root().join(src)) {
            let sites = panicking_macro_sites(&code);
            if sites.is_empty() {
                continue;
            }
            let key = format!("{}/{}", src.trim_end_matches("/src"), name);
            live.push(key.clone());
            match allowed.get(key.as_str()) {
                Some(&(expected, _)) => assert_eq!(
                    sites,
                    expected.to_vec(),
                    "{key}'s panicking-macro sites drifted — a site changed \
                     kind or text, and a count alone cannot see that"
                ),
                None => panic!(
                    "{key} contains {} panicking macro(s) outside the \
                     allowlist. G7's contract is saturate/None/no-op on bad \
                     input; a documented programming-error precondition \
                     belongs in panicking_macro_allowlist() with that reason",
                    sites.len()
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

/// `#[rustfmt::skip]` sites frozen by name. The attribute keeps
/// `cargo fmt --check` green while the block under it may hold arbitrary
/// unformatted text — the one place dense, unreadable code can hide from the
/// format gate. `cfg_attr` cannot smuggle it (`rustfmt::skip` as a token is
/// still counted) and comments/strings cannot fake it (`test_code` strips
/// both).
#[test]
fn rustfmt_skip_is_frozen_at_named_sites() {
    const ALLOWLIST: &[(&str, usize, &str)] = &[(
        "izanagi_kit/examples/autotile_demo.rs",
        1,
        "ASCII dungeon map and tile-color literals stay readable only when \
         rustfmt does not reflow them",
    )];
    let mut live: Vec<String> = Vec::new();
    for dir_rel in [
        "izanagi_kit/src",
        "izanagi_kit/tests",
        "izanagi_kit/examples",
        "izanagi/src",
        "izanagi/tests",
        "izanagi/examples",
    ] {
        let dir = repo_root().join(dir_rel);
        let mut entries: Vec<_> = fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("reading {dir_rel}: {e}"))
            .flatten()
            .collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            if path.extension().map(|e| e == "rs") != Some(true) {
                continue;
            }
            let code = test_code(
                &fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("reading {}: {e}", path.display())),
            );
            let n = code.matches("rustfmt::skip").count();
            if n == 0 {
                continue;
            }
            let key = format!("{dir_rel}/{}", entry.file_name().to_string_lossy());
            live.push(key.clone());
            match ALLOWLIST.iter().find(|(f, _, _)| *f == key) {
                Some(&(_, expected, _)) => assert_eq!(
                    n, expected,
                    "{key} has {n} `rustfmt::skip` attribute(s) where \
                     {expected} are named — a new one needs its reason \
                     written in the allowlist first"
                ),
                None => panic!(
                    "{key} contains {n} `rustfmt::skip` attribute(s) outside \
                     the allowlist — it opts code out of `cargo fmt --check`, \
                     so every site must be named here with the reason"
                ),
            }
        }
    }
    for (file, _, _) in ALLOWLIST {
        assert!(
            live.iter().any(|k| k == file),
            "the `rustfmt::skip` allowlist names {file}, which no longer \
             carries the attribute — a stale entry reads as permission"
        );
    }
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
    //
    // The same class covers inputs the scanners never read for other
    // reasons: `env!`/`option_env!` bake the *build machine's* environment
    // into the binary, and `extern`/`#[no_mangle]`/`#[link` declare linkage
    // to native code no manifest dependency lists — a dependency the
    // zero-dependency checks cannot see.
    for src in ["izanagi_kit/src", "izanagi/src"] {
        let sources = library_sources(&repo_root().join(src));
        assert!(!sources.is_empty(), "found no sources under {src}");
        for (name, code) in sources {
            for needle in [
                "#[path",
                "include!(",
                "include_bytes!(",
                "env!(",
                "option_env!",
                // `file!`/`line!`/`column!`/`module_path!` bake the build
                // machine's checkout path and edit positions into shipped
                // code — the same ambient-input class as `env!`, and a
                // byte-compare run twice on one machine cannot see it
                // (proven by injection: `pub fn _probe() { file!() }` in
                // lib src and in an example both passed every check).
                "file!(",
                "line!(",
                "column!(",
                "module_path!(",
                "extern ",
                "#[no_mangle",
                "#[link",
            ] {
                assert!(
                    !code.contains(needle),
                    "{name} contains `{needle}` — compiled code must live in \
                     the file the scanners read, not arrive by reference"
                );
            }
        }
    }

    // The same assumption breaks one level up: a symlink under a scanned
    // directory reads as the *target's* content — bytes the repository's
    // diff and review surfaces never show. Checked-in trees must be real
    // files.
    for dir in [
        "izanagi_kit/src",
        "izanagi/src",
        "izanagi_kit/tests",
        "izanagi/tests",
        "izanagi_kit/examples",
        "izanagi/examples",
    ] {
        fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
            let Ok(entries) = fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                // DirEntry::file_type does not follow links — the entry's own
                // kind is what the repository actually stores.
                if entry.file_type().map(|t| t.is_symlink()).unwrap_or(false) {
                    out.push(path);
                } else if path.is_dir() {
                    walk(&path, out);
                }
            }
        }
        let mut links = Vec::new();
        walk(&repo_root().join(dir), &mut links);
        assert!(
            links.is_empty(),
            "symlinks under {dir}: {links:?} — a scanned path must BE the \
             file, not point at bytes that live outside the reviewed tree"
        );
    }
}

/// Does `code` contain `needle` as a token — not as the tail of a longer
/// identifier? The boundary is derived from the needle's own first/last
/// character, so `unsafe` matches `unsafe {` but not `myunsafe`, while
/// `#[cfg` (ending in an ident char) requires the next character to be a
/// non-identifier, matching `#[cfg(test)]` but not `#[cfgfoo]`.
fn contains_token(code: &str, needle: &str) -> bool {
    // The boundary requirement is derived from the needle's own first and last
    // character, so a needle is never asked for a boundary that cannot exist.
    //
    // The first version returned `true` immediately for any needle starting
    // with `.`, on the reasoning that a leading dot is a boundary by itself.
    // That was sound only while every such needle also *ended* with `(`, which
    // supplied the right-hand boundary by construction. Adding `.first_chunk`
    // — paren-free, because the method is normally written with a turbofish —
    // broke the assumption, and the needle matched `self.first_chunk_index`.
    // The self-test below caught it. This is the same both-sides rule used by
    // `no_nondeterminism_in_sim.rs` and `izanagi/tests/float_boundary.rs`.
    fn ident_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }
    let bytes: Vec<char> = code.chars().collect();
    let pat: Vec<char> = needle.chars().collect();
    if pat.is_empty() || bytes.len() < pat.len() {
        return false;
    }
    let check_left = pat.first().copied().map(ident_char).unwrap_or(false);
    let check_right = pat.last().copied().map(ident_char).unwrap_or(false);
    for i in 0..=bytes.len() - pat.len() {
        if bytes[i..i + pat.len()] != pat[..] {
            continue;
        }
        let left_ok = !check_left || i == 0 || !ident_char(bytes[i - 1]);
        let after = i + pat.len();
        let right_ok = !check_right || after >= bytes.len() || !ident_char(bytes[after]);
        if left_ok && right_ok {
            return true;
        }
    }
    false
}

/// Test-file text with `//` comments dropped and `"..."` literals blanked —
/// what remains is the code the compiler actually sees. `#[cfg(...)]` quoted
/// inside a string (this suite passes several to its own parsers) must not
/// count as conditional compilation, and neither may a comment.
fn test_code(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut in_str = false;
    let mut esc = false;
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if in_str {
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                in_str = false;
            }
            out.push(' ');
            continue;
        }
        if c == '/' && chars.peek() == Some(&'/') {
            for c2 in chars.by_ref() {
                if c2 == '\n' {
                    out.push('\n');
                    break;
                }
            }
            continue;
        }
        if c == '\'' {
            // A `'` is either a char literal or a lifetime. A char literal —
            // 'x', '\n', '\\', '\'' — may carry a `"` that must not toggle
            // string mode; a lifetime ('a, 'static) is left as code.
            let mut peek = chars.clone();
            match (peek.next(), peek.next()) {
                (Some('\\'), _) => {
                    // '\x' — consume the escape and the closing quote.
                    chars.next();
                    for c2 in chars.by_ref() {
                        if c2 == '\'' {
                            break;
                        }
                    }
                    out.push(' ');
                    continue;
                }
                (Some(_), Some('\'')) => {
                    chars.next();
                    chars.next();
                    out.push(' ');
                    continue;
                }
                _ => {}
            }
        }
        if c == '"' {
            in_str = true;
            out.push(' ');
            continue;
        }
        out.push(c);
    }
    out
}

/// `env!("NAME")`/`option_env!("NAME")` arguments in `src`, at code positions
/// only. Unlike `test_code`, string contents stay readable — the env-var name
/// IS the thing being checked — so an `env!(` that sits inside a string
/// literal (a scanner needle) is skipped instead of blanked. `None` marks a
/// non-literal argument such as `env!(concat!(..))`.
fn env_macro_args(src: &str) -> Vec<Option<String>> {
    let b = src.as_bytes();
    let mut i = 0usize;
    let mut out = Vec::new();
    while i < b.len() {
        if b[i] == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if b[i] == b'"' {
            i += 1;
            while i < b.len() && b[i] != b'"' {
                if b[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            continue;
        }
        if b[i] == b'\'' {
            match (b.get(i + 1), b.get(i + 2)) {
                (Some(&b'\\'), _) => {
                    i += 2;
                    while i < b.len() && b[i] != b'\'' {
                        i += 1;
                    }
                    i += 1;
                    continue;
                }
                (_, Some(&b'\'')) => {
                    i += 3;
                    continue;
                }
                _ => {}
            }
        }
        let name = if b[i..].starts_with(b"option_env!") {
            Some(b"option_env!".len())
        } else if b[i..].starts_with(b"env!") {
            Some(b"env!".len())
        } else {
            None
        };
        if let Some(w) = name {
            let prev_ok = i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_');
            if prev_ok {
                let mut j = i + w;
                if b.get(j) == Some(&b'(') {
                    j += 1;
                    while b.get(j) == Some(&b' ') || b.get(j) == Some(&b'\t') {
                        j += 1;
                    }
                    if b.get(j) == Some(&b'"') {
                        j += 1;
                        let start = j;
                        while j < b.len() && b[j] != b'"' {
                            j += 1;
                        }
                        out.push(
                            std::str::from_utf8(&b[start..j])
                                .ok()
                                .map(|s| s.to_string()),
                        );
                    } else {
                        out.push(None);
                    }
                }
            }
            i += w;
            continue;
        }
        i += 1;
    }
    out
}

#[test]
fn the_verification_suite_cannot_quietly_skip_or_disable_its_own_checks() {
    // The suite's own files are code, and code decays silently too:
    //
    // * `#[ignore]` — the suite reports "N ignored" and still exits green;
    //   a test that never runs is verification that reads as performed.
    //   The one legitimate use is a deliberately-manual helper, and it must
    //   be named here with the reason.
    // * any `#[cfg`/`cfg!(`/`cfg_attr(` — a predicate in a test file can
    //   remove a check on one platform while leaving it running on another.
    //   A check that exists only on some machines is not a check.
    // * `unsafe` — test crates do not inherit the lib's `forbid(unsafe_code)`;
    //   UB inside the harness is a worse failure mode than a missing lint.
    // * `include!`/`include_bytes!`/`#[path]`/`mod` — each splices in code this
    //   flat walk never reads. Helpers are hand-duplicated by convention; a
    //   `mod` pulling a sibling file would be the suite scanning less than it
    //   compiles. `env!`/`option_env!` bake the *build machine* into the
    //   binary — for examples, straight into the byte-identical output the
    //   gate pins; test files may read only `CARGO_MANIFEST_DIR`.
    // * a file without `#[test]` compiles and runs nothing — the emptiest
    //   way for a check to "pass".
    const IGNORE_ALLOWLIST: &[(&str, &str)] = &[(
        "det_hash_golden.rs",
        "print_golden is a regeneration helper, run explicitly with --ignored",
    )];
    // `#[allow]`/`#[warn]`/`#[expect]` soften a lint in place — banned outright
    // in test files, but examples legitimately carry a couple. They are
    // frozen by (file, count, reason) so a *new* site fails until named,
    // and a deleted one makes its entry stale enough to fail too.
    const EXAMPLE_ALLOW_ALLOWLIST: &[(&str, usize, &str)] = &[
        (
            "platformer.rs",
            1,
            "clippy::unnecessary_map_or reads better as map_or on the option chain",
        ),
        (
            "roguelike.rs",
            1,
            "dead_code on fields kept for the documented spawn-table shape",
        ),
    ];
    // (directory, must contain #[test]) — examples are binaries the gate
    // runs for byte-identical output, so a `#[cfg]` there would fork the
    // *evidence* the pinned hashes stand on. The same rules apply without
    // the #[test] floor.
    let mut seen_allowlisted: BTreeSet<&str> = BTreeSet::new();
    for (dir_rel, require_tests) in [
        ("izanagi_kit/tests", true),
        ("izanagi/tests", true),
        ("izanagi_kit/examples", false),
        ("izanagi/examples", false),
    ] {
        let dir = repo_root().join(dir_rel);
        let mut entries: Vec<_> = fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("reading {dir_rel}: {e}"))
            .flatten()
            .collect();
        entries.sort_by_key(|e| e.file_name());
        assert!(!entries.is_empty(), "found no sources under {dir_rel}");
        for entry in &entries {
            let path = entry.path();
            if path.extension().map(|e| e == "rs") != Some(true) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let raw = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
            let code = test_code(&raw);
            // Brace imports rewrite the text the needles spell: scanning the
            // flattened paths keeps `use std::env::{var}` readable as the
            // `env::var` it actually compiles to.
            let code = format!("{code}\n{}", flattened_use_paths(&code).join("\n"));
            assert!(
                !require_tests || code.contains("#[test"),
                "{name} contains no #[test] function — a test file that \
                 runs nothing passes vacuously"
            );
            let ignored = code.matches("#[ignore").count();
            let allowed = IGNORE_ALLOWLIST.iter().filter(|(f, _)| *f == name).count();
            assert!(
                ignored <= allowed,
                "{name} has {ignored} `#[ignore]` attribute(s) — a skipped \
                 test still exits the suite green. Name it in \
                 IGNORE_ALLOWLIST with the reason, or remove the attribute"
            );
            // `#![cfg]`/`#![cfg_attr]` are the inner-attribute spellings —
            // same power, different surface. A crate-level `#![cfg(unix)]`
            // would compile the whole test file to nothing on some
            // platforms while it keeps running here.
            for needle in [
                "#[cfg",
                "#![cfg",
                "cfg!(",
                "cfg_attr(",
                "unsafe",
                // Same ambient-input class as src: `file!()` prints the
                // machine's checkout path — two runs on this machine still
                // byte-match, so only a token ban can see it.
                "file!(",
                "line!(",
                "column!(",
                "module_path!(",
            ] {
                assert!(
                    !contains_token(&code, needle),
                    "{name} contains `{needle}` — the verification suite may \
                     not fork itself on platform, weaken its own lints, or \
                     take unsafe shortcuts"
                );
            }
            // Code the flat walk never reads: `include!`/`include_bytes!`
            // splice in foreign text, `#[path]` points a `mod` anywhere, and
            // `mod x;` compiles a file under this directory that no entry in
            // this loop visits. `mod` in these dirs is banned outright —
            // helpers are hand-copied by convention.
            for needle in ["include!(", "include_bytes!(", "#[path", "mod "] {
                assert!(
                    !contains_token(&code, needle),
                    "{name} contains `{needle}` — suite crates may not splice \
                     in code the scanners never read; inline helpers are the \
                     convention, so a `mod` here can only hide a file"
                );
            }
            // Runtime skips and silencers. `env::var` reads the machine at
            // *run* time — a check behind it passes on one host and not
            // another, or writes machine data into pinned example output.
            // `catch_unwind` swallows a real assert; a detached `thread::`
            // loses its panic to a dropped JoinHandle; `should_panic` reads
            // a failure as green; `set_hook`/`take_hook` rewrite what a
            // panic means. (`env::args` stays: the engine examples use it
            // for `--terminal`, and argv is identical across gate runs.)
            for needle in [
                "catch_unwind",
                "thread::",
                "should_panic",
                "set_hook",
                "take_hook",
            ] {
                assert!(
                    !contains_token(&code, needle),
                    "{name} contains `{needle}` — a check that can be skipped \
                     at run time, or a panic that can be swallowed or \
                     reinterpreted, is a check that may not have run"
                );
            }
            // `env::var` as a needle left `env::var_os`/`env::vars`/
            // `env::current_dir`/`env::temp_dir`/`env::current_exe` unflagged —
            // the token edge after `var` is an identifier character in
            // `var_os`, and the other readers were never named (verified by
            // injection: `env::var_os` in a test and `env::current_dir` in an
            // example both stayed green). Whitelist instead: `env::` may be
            // followed only by the argv readers (the engine examples use them
            // for `--terminal`; argv is identical across gate runs) and
            // `temp_dir` (integration.rs writes a scratch save there — the
            // path itself is never asserted, so no machine data lands in a
            // pinned output).
            for (hit, _) in code.match_indices("env::") {
                let before = code[..hit].chars().last().unwrap_or(' ');
                if before.is_alphanumeric() || before == '_' {
                    continue;
                }
                let ident: String = code[hit + "env::".len()..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                assert!(
                    ident == "args" || ident == "args_os" || ident == "temp_dir",
                    "{name} contains `env::{ident}` — only `env::args`, \
                     `env::args_os` and `env::temp_dir` may read the machine \
                     at run time; every other `env::` reader is a skip switch \
                     or a machine leak"
                );
            }
            // Raw pointers and addresses: `&x as *const T as usize` puts an
            // ASLR address into the values a check compares; the pointer
            // format flag prints one into bytes the gate pins;
            // `into_raw`/`from_raw` spell a pointer without `*const` in the
            // text (all verified green by injection in a test and an
            // example). The pointer-format flag lives inside a format string
            // — invisible once literals are blanked — so the raw source is
            // scanned for it, and the needle below is spelled so this file
            // does not match itself.
            for needle in [
                "as *",
                "*const",
                "*mut",
                ".as_ptr(",
                ".as_mut_ptr(",
                "into_raw(",
                "from_raw(",
                // `std::ptr::addr_of!` makes a raw pointer with no `*const`
                // in the text; `Arc::ptr_eq`/`Rc::ptr_eq`/`std::ptr::eq`
                // compare addresses directly (both injected: green).
                "std::ptr",
                "ptr::",
                "addr_of",
                "ptr_eq",
                // `x as fn()` coerces a fn item to a fn pointer — the only
                // spelling that lets a formatter print an address (fn items
                // carry no Debug impl themselves; injected: green).
                "as fn",
            ] {
                assert!(
                    !contains_token(&code, needle),
                    "{name} contains `{needle}` — a machine address in a check \
                     is ASLR input no input log can replay"
                );
            }
            let fmt_ptr = concat!("{:", "p}");
            assert!(
                !raw.contains(fmt_ptr),
                "{name} prints a pointer address with `{fmt_ptr}` — the \
                 address is ASLR input no input log can replay"
            );
            // `Backtrace::capture`/`force_capture` harvest the ambient run —
            // symbol names, build paths, and whether RUST_BACKTRACE was set —
            // none of which the input log holds (injected into a test and an
            // example: green).
            for needle in ["backtrace", "Backtrace"] {
                assert!(
                    !contains_token(&code, needle),
                    "{name} contains `{needle}` — a backtrace is ambient \
                     machine state, not an input the suite replays"
                );
            }
            // Delegating the checked computation to outside the scanned
            // universe: a subprocess runs anything, a socket reads bytes no
            // scan can see, and env writes mutate the ambient inputs other
            // needles police. None of these appear anywhere today. The
            // trailing `as ` needles close the alias-import twin of each
            // banned module — `use std::env as e` spells no `env::var` yet
            // compiles one (`e::var`); flattened_use_paths keeps the alias
            // spelling visible here so `env as` catches it.
            for needle in [
                "Command::new",
                "process::Command",
                "std::net",
                "TcpStream",
                "TcpListener",
                "UdpSocket",
                "ToSocketAddrs",
                "env::set_var",
                "env::remove_var",
                "env as",
                "thread as",
                "process as",
            ] {
                assert!(
                    !contains_token(&code, needle),
                    "{name} contains `{needle}` — a check or example that \
                     runs a subprocess, opens a socket, or mutates the \
                     environment has delegated the verified computation to \
                     code this suite never reads"
                );
            }
            // Test-dir-only: the examples keep two legitimate uses —
            // `process::exit` on their error path and two `#[allow]` lints —
            // but inside the suite `exit` can end the harness mid-file and
            // `#[allow]`/`#[warn]`/`#[expect]` downgrade lints in place —
            // as do their `#![...]` inner-attribute spellings, which silence
            // the whole file at once (verified: `#![allow(dead_code)]`
            // escaped the outer-attribute needle until these were added).
            if require_tests {
                for needle in [
                    "process::exit",
                    "#[allow",
                    "#[warn",
                    "#[expect",
                    "#![allow",
                    "#![warn",
                    "#![expect",
                    "#![feature",
                ] {
                    assert!(
                        !contains_token(&code, needle),
                        "{name} contains `{needle}` — tests may not exit the \
                         harness early or soften a lint in place"
                    );
                }
            }
            // Examples-only: the filesystem is a channel between gate runs —
            // an example that writes bytes in run one and reads them in run
            // two reproduces perfectly while computing nothing the pinned
            // output claims it computed. No example touches the filesystem
            // today; the pinned output must come from the binary's own code,
            // not from bytes on this machine. (Tests keep `fs::` — reading
            // the repo is the scanners' whole job.)
            if !require_tests {
                for needle in [
                    "fs::",
                    "File::",
                    "OpenOptions",
                    "read_to_string",
                    "read_dir",
                    "canonicalize",
                    // The wall clock is the same ambient channel: an example
                    // printing `Instant::now().elapsed()` bytes differs run to
                    // run, and two fast-enough runs can still byte-match —
                    // nondeterminism that hides behind the pinned compare
                    // (injected `Instant::now`/`SystemTime::now` into an
                    // example: green). Tests keep the clock — bench.rs's
                    // timing sanity checks are its job.
                    "Instant",
                    "SystemTime",
                    "UNIX_EPOCH",
                    "std::time",
                    "time::",
                    // Unordered containers hash with RandomState seeded per
                    // process — iteration order differs between the two runs
                    // of the pinned compare (injected `HashMap`/`RandomState`/
                    // `DefaultHasher` into an example: green). Tests keep
                    // them; a test's own HashMap can only flake itself.
                    "HashMap",
                    "HashSet",
                    "hash_map",
                    "RandomState",
                    "DefaultHasher",
                    "SipHasher",
                ] {
                    assert!(
                        !contains_token(&code, needle),
                        "{name} contains `{needle}` — an example's pinned \
                         output must be the binary's own computation, not \
                         bytes it read off this machine"
                    );
                }
                let softening: usize = [
                    "#[allow",
                    "#[warn",
                    "#[expect",
                    "#![allow",
                    "#![warn",
                    "#![expect",
                ]
                .iter()
                .map(|n| code.matches(n).count())
                .sum();
                let allowed: usize = EXAMPLE_ALLOW_ALLOWLIST
                    .iter()
                    .filter(|(f, _, _)| *f == name)
                    .map(|(_, c, _)| c)
                    .sum();
                assert!(
                    softening == allowed,
                    "{name} has {softening} lint-softening attribute(s), \
                     allowlist names {allowed} — add a site to \
                     EXAMPLE_ALLOW_ALLOWLIST with its reason, and delete the \
                     entry when the attribute goes"
                );
                seen_allowlisted.extend(
                    EXAMPLE_ALLOW_ALLOWLIST
                        .iter()
                        .filter(|(f, _, _)| *f == name)
                        .map(|(f, _, _)| f),
                );
            }
            // `env!`/`option_env!` bake the build machine into the binary.
            // In an example that is machine data inside the byte-identical
            // output the gate pins; in a test file it is a check whose result
            // depends on who built it. The single legitimate use is
            // `env!("CARGO_MANIFEST_DIR")` — a path cargo itself defines. The
            // argument literal must be read, so this one uses raw source with
            // string-aware skipping rather than the string-blanked `code`.
            let env_args = env_macro_args(
                &fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("reading {}: {e}", path.display())),
            );
            if require_tests {
                assert!(
                    env_args
                        .iter()
                        .all(|a| a.as_deref() == Some("CARGO_MANIFEST_DIR")),
                    "{name} reads a build env var other than \
                     CARGO_MANIFEST_DIR ({env_args:?}) — a check whose \
                     outcome depends on who compiled it"
                );
            } else {
                assert!(
                    env_args.is_empty(),
                    "{name} bakes a build-machine env var into an example — \
                     the pinned output is no longer the binary's own"
                );
            }
        }
        // Cargo auto-discovers only top-level .rs files in these dirs, and
        // with `mod` banned above nothing below can be compiled — but a
        // nested .rs file would still sit here unscanned, indistinguishable
        // from a suite member. The dirs must stay flat.
        for entry in &entries {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let nested: Vec<_> = fs::read_dir(&path)
                .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
                .flatten()
                .filter(|e| e.path().extension().map(|x| x == "rs").unwrap_or(false))
                .collect();
            assert!(
                nested.is_empty(),
                "{} contains nested .rs files the flat scans cannot see: {:?}",
                path.display(),
                nested
            );
        }
    }
    // And a stale allowlist entry is permission for nobody: the file must
    // still exist AND still carry the ignored item — once the ignore is
    // gone the permission is spent.
    for (file, _) in IGNORE_ALLOWLIST {
        let path = repo_root().join("izanagi_kit/tests").join(file);
        assert!(
            path.exists(),
            "IGNORE_ALLOWLIST names {file}, which no longer exists"
        );
        let body = test_code(
            &fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("reading {}: {e}", path.display())),
        );
        assert!(
            body.matches("#[ignore").next().is_some(),
            "IGNORE_ALLOWLIST names {file} but it no longer contains \
             #[ignore] — a stale entry reads as permission"
        );
    }
    for (file, _, _) in EXAMPLE_ALLOW_ALLOWLIST {
        assert!(
            seen_allowlisted.contains(file),
            "EXAMPLE_ALLOW_ALLOWLIST names {file}, which no longer exists — \
             a stale entry reads as permission"
        );
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

/// Test files are separate crates: they cannot share code, so the small
/// scanner helpers are duplicated by hand into every file that needs them.
/// That is only honest if the copies stay identical — a drifted copy is a
/// scanner with a private blind spot (both `library_sources` and
/// `contains_token` had already drifted when this check was added).
/// Compare each helper's function body byte for byte across its copies.
#[test]
fn shared_scanner_helpers_are_identical_in_every_file() {
    let groups: &[(&str, &[&str])] = &[
        (
            "test_module_boundary",
            &[
                "izanagi_kit/tests/no_nondeterminism_in_sim.rs",
                "izanagi_kit/tests/hashes_are_endian_independent.rs",
                "izanagi_kit/tests/hashes_are_width_independent.rs",
                "izanagi_kit/tests/msrv_is_respected.rs",
                "izanagi_kit/tests/no_platform_cfg_in_sim.rs",
                "izanagi_kit/tests/no_float_in_sim.rs",
                "izanagi_kit/tests/public_api_is_exercised.rs",
                "izanagi_kit/tests/global_invariants_hold.rs",
                "izanagi/tests/float_boundary.rs",
                "izanagi/tests/public_api_is_exercised.rs",
                "izanagi/tests/no_unordered_containers.rs",
            ][..],
        ),
        (
            "library_sources",
            &[
                "izanagi_kit/tests/no_nondeterminism_in_sim.rs",
                "izanagi_kit/tests/hashes_are_endian_independent.rs",
                "izanagi_kit/tests/hashes_are_width_independent.rs",
                "izanagi_kit/tests/msrv_is_respected.rs",
                "izanagi_kit/tests/no_platform_cfg_in_sim.rs",
                "izanagi_kit/tests/global_invariants_hold.rs",
                "izanagi/tests/no_unordered_containers.rs",
            ][..],
        ),
        (
            "contains_token",
            &[
                "izanagi_kit/tests/msrv_is_respected.rs",
                "izanagi_kit/tests/no_float_in_sim.rs",
                "izanagi_kit/tests/global_invariants_hold.rs",
            ][..],
        ),
        (
            "count_token",
            &[
                "izanagi_kit/tests/no_nondeterminism_in_sim.rs",
                "izanagi/tests/float_boundary.rs",
                "izanagi/tests/no_unordered_containers.rs",
            ][..],
        ),
        (
            "engine_src",
            &[
                "izanagi/tests/float_boundary.rs",
                "izanagi/tests/no_unordered_containers.rs",
            ][..],
        ),
        (
            "kit_src",
            &[
                "izanagi_kit/tests/no_nondeterminism_in_sim.rs",
                "izanagi_kit/tests/no_platform_cfg_in_sim.rs",
                "izanagi_kit/tests/hashes_are_width_independent.rs",
                "izanagi_kit/tests/hashes_are_endian_independent.rs",
            ][..],
        ),
        (
            "take_balanced",
            &[
                "izanagi_kit/tests/global_invariants_hold.rs",
                "izanagi_kit/tests/no_platform_cfg_in_sim.rs",
            ][..],
        ),
        (
            "flattened_use_paths",
            &[
                "izanagi_kit/tests/global_invariants_hold.rs",
                "izanagi_kit/tests/no_nondeterminism_in_sim.rs",
                "izanagi_kit/tests/docs_are_current.rs",
            ][..],
        ),
    ];
    fn extract<'a>(src: &'a str, fname: &str) -> &'a str {
        let marker = format!("fn {fname}");
        let start = src
            .find(&marker)
            .unwrap_or_else(|| panic!("{fname} missing from a file that must carry it"));
        let rest = &src[start..];
        let end = rest.find("\n}").expect("unclosed fn") + 2;
        &rest[..end]
    }
    for (fname, files) in groups {
        let mut bodies = Vec::new();
        for file in *files {
            let src = fs::read_to_string(repo_root().join(file)).expect("read scanner");
            bodies.push((*file, extract(&src, fname).to_string()));
        }
        let (first_name, first) = &bodies[0];
        for (name, body) in &bodies[1..] {
            assert_eq!(body, first, "{name}'s {fname} drifted from {first_name}'s");
        }
    }
}

/// The identity freeze above proves the copies are the *same* — it cannot
/// prove they are *right*: a lexer that never found a real marker, or that
/// took fakes, would pass byte-equality while under-scanning every file it
/// serves. Pin the behavior on synthetic input instead.
#[test]
fn the_shared_boundary_lexer_finds_real_markers_and_rejects_fakes() {
    // Real: marker at line start (leading whitespace allowed), found at its
    // byte offset; nothing after it is scanned.
    let real = "fn impl_code() {}\n\n#[cfg(test)]\nmod tests {}\n";
    let at = test_module_boundary(real).expect("a real marker must be found");
    assert_eq!(&real[..at], "fn impl_code() {}\n\n");
    assert_eq!(
        test_module_boundary("    #[cfg(test)]\n"),
        Some(4),
        "indented real marker still counts"
    );

    // Fakes must not truncate: inside a line comment, a block comment
    // (including a nested one), a plain string, a raw string with #s, a
    // string on a line with real code — none of them are the boundary.
    for fake in [
        "// a note about #[cfg(test)]\nfn a() {}\n",
        "/* docs say #[cfg(test)] here */\nfn a() {}\n",
        "/* nested /* #[cfg(test)] */ comment */\nfn a() {}\n",
        "const S: &str = \"#[cfg(test)]\";\nfn a() {}\n",
        "const S: &str = r##\"#[cfg(test)] and a \" inside\"##;\nfn a() {}\n",
        "fn f<'a>() -> &'a str { \"#[cfg(test)]\" }\n",
        "let x = 1; #[cfg(test)]\n",
    ] {
        assert_eq!(
            test_module_boundary(fake),
            None,
            "a fake marker was taken as the boundary in {fake:?}"
        );
    }
    // The decisive case: a fake marker before a real one must not hide the
    // real boundary.
    let mixed = "const S: &str = \"#[cfg(test)]\";\nfn a() {}\n\n#[cfg(test)]\nmod t {}\n";
    assert_eq!(
        test_module_boundary(mixed),
        Some(mixed.rfind("#[cfg(test)]").unwrap()),
        "a fake early marker must not shadow the real test module"
    );
    // Sanity: no marker at all means the whole file is impl code.
    assert_eq!(test_module_boundary("fn only() {}\n"), None);
}
