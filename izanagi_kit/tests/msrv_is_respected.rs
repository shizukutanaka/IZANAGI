//! The declared MSRVs, checked without a toolchain.
//!
//! Both crates promise a minimum supported Rust version in `Cargo.toml`
//! (`izanagi` 1.65, `izanagi_kit` 1.75), which cargo enforces for consumers.
//! Nothing enforced it for *us*: a contributor reaching for a newer method
//! compiles fine on stable and breaks every consumer on the declared minimum,
//! and the failure only surfaces in CI — or, until CI exists, not at all.
//!
//! Installing the older toolchains here is not possible (this sandbox's network
//! policy blocks `static.rust-lang.org`, measured), so this scans the library
//! sources for APIs stabilised after each crate's minimum instead. That is a
//! lower bound on what a real MSRV build would catch, but it catches the thing
//! that actually happens: someone types `.is_some_and(...)` because clippy
//! suggested it.
//!
//! Test and example code is deliberately out of scope. `cargo check -p <crate>`
//! on the old toolchain builds the library only, and the MSRV promise is to
//! consumers, not to our own test suite.

use std::fs;
use std::path::{Path, PathBuf};

/// `(name, stabilised-in, needle)`. Token matching, not parsing — this is a
/// tripwire, not a compiler. Entries are the APIs a contributor is most likely
/// to reach for by accident, several of them because clippy suggests them.
///
/// A needle beginning with an identifier character (or `c` for the C-string
/// literal) must additionally start at a token boundary. The first draft used
/// plain `contains` and reported two violations that were not: `c\"` matched
/// the tail of `nondeterministic\"` and `\"resync\"`.
const POST_MSRV_APIS: &[(&str, (u32, u32), &str)] = &[
    ("Option/Result::is_some_and", (1, 70), ".is_some_and("),
    ("Option::is_none_or", (1, 82), ".is_none_or("),
    ("int::div_ceil", (1, 73), ".div_ceil("),
    ("int::isqrt", (1, 84), ".isqrt("),
    ("std::sync::LazyLock", (1, 80), "LazyLock"),
    ("std::sync::OnceLock", (1, 70), "OnceLock"),
    ("Option::take_if", (1, 80), ".take_if("),
    ("slice::split_at_checked", (1, 80), ".split_at_checked("),
    ("c\"…\" literals", (1, 77), "c\""),
    // Integer and slice helpers a fixed-point / grid / hashing crate reaches
    // for by reflex. None appear today; the point is the day one does.
    ("int::next_multiple_of", (1, 73), ".next_multiple_of("),
    (
        "Option::inspect is deliberately absent — see the note below",
        (1, 76),
        "\u{0}",
    ),
    ("slice::chunk_by", (1, 77), ".chunk_by("),
    ("slice::first_chunk", (1, 77), ".first_chunk"),
    ("slice::last_chunk", (1, 77), ".last_chunk"),
    ("[u8]::trim_ascii", (1, 80), ".trim_ascii("),
    ("std::cell::LazyCell", (1, 80), "LazyCell"),
    ("#[expect(…)] attribute", (1, 81), "#[expect("),
    ("slice/Iterator::is_sorted", (1, 82), ".is_sorted("),
    ("num::midpoint", (1, 85), ".midpoint("),
    ("int::cast_signed", (1, 87), ".cast_signed("),
    ("int::cast_unsigned", (1, 87), ".cast_unsigned("),
    ("int::is_multiple_of", (1, 87), ".is_multiple_of("),
    ("slice::as_chunks", (1, 88), ".as_chunks"),
    ("slice::as_chunks_mut", (1, 88), ".as_chunks_mut"),
    // Syntax, not an API: a let-chain is a *parse error* on an older
    // compiler, which is the worst kind of MSRV break — the diagnostic points
    // at the syntax rather than at the version.
    ("let-chains (`&& let`)", (1, 88), "&& let "),
];

/// `Option::inspect` and `Result::inspect` stabilised in 1.76, but
/// `Iterator::inspect` has existed since 1.0 and they share a spelling. A
/// `.inspect(` needle would fire on every iterator chain in the workspace, and
/// a scanner that cries wolf is one somebody switches off. It is left out
/// deliberately rather than forgotten; the entry above holds its place with a
/// needle that cannot occur in source text.
const _: () = ();

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the kit lives one level below the workspace root")
        .to_path_buf()
}

fn declared_msrv(crate_dir: &str) -> (u32, u32) {
    let manifest = fs::read_to_string(repo_root().join(crate_dir).join("Cargo.toml"))
        .unwrap_or_else(|e| panic!("cannot read {crate_dir}/Cargo.toml: {e}"));
    let raw = manifest
        .lines()
        .find_map(|l| l.trim().strip_prefix("rust-version = "))
        .map(|v| v.trim().trim_matches('"').to_string())
        .unwrap_or_else(|| panic!("{crate_dir} must declare rust-version"));
    let mut parts = raw.split('.').map(|p| p.parse::<u32>().unwrap_or(0));
    (parts.next().unwrap_or(0), parts.next().unwrap_or(0))
}

/// Library sources only, with the trailing test module and line comments
/// stripped — the same convention `no_float_in_sim.rs` uses.
fn library_sources(crate_dir: &str) -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    walk(&repo_root().join(crate_dir).join("src"), &mut files);
    files
        .into_iter()
        .map(|path| {
            let src = fs::read_to_string(&path).unwrap_or_default();
            let impl_end = src.find("#[cfg(test)]").unwrap_or(src.len());
            let code = src[..impl_end]
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            (path.display().to_string(), code)
        })
        .collect()
}

/// Does `code` contain `needle` as a token, rather than as the tail of a
/// longer identifier or string? A needle that starts with `.` is already
/// bounded on the left by the dot; anything else must not be preceded by an
/// identifier character.
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

fn check_crate(crate_dir: &str) {
    let (major, minor) = declared_msrv(crate_dir);
    let sources = library_sources(crate_dir);
    assert!(
        sources.len() > 5,
        "expected to find {crate_dir}'s library sources, found {} — has the \
         layout changed?",
        sources.len()
    );

    let mut violations = Vec::new();
    for (api, (since_major, since_minor), needle) in POST_MSRV_APIS {
        if (*since_major, *since_minor) <= (major, minor) {
            continue; // available at this crate's minimum
        }
        for (file, code) in &sources {
            if contains_token(code, needle) {
                violations.push(format!(
                    "{file}: {api} (stable since {since_major}.{since_minor}, \
                     but {crate_dir} declares {major}.{minor})"
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "library code uses APIs newer than the declared MSRV, so consumers on \
         that toolchain cannot build it:\n{}\n\nEither avoid the API or raise \
         rust-version in Cargo.toml deliberately.",
        violations.join("\n")
    );
}

#[test]
fn the_engine_library_builds_within_its_declared_msrv() {
    check_crate("izanagi");
}

#[test]
fn the_kit_library_builds_within_its_declared_msrv() {
    check_crate("izanagi_kit");
}

#[test]
fn the_scanner_catches_real_uses_and_ignores_lookalikes() {
    // A tripwire that never fires is indistinguishable from one that is
    // miswired, and one that fires at everything is worse than useless. The
    // first draft of this file only tested the firing direction, using
    // `is_some_and`, and so shipped a `c"` needle that matched the tail of
    // `nondeterministic"`. Both directions are checked now.
    let needle_for = |name: &str| {
        POST_MSRV_APIS
            .iter()
            .find(|(n, _, _)| n.starts_with(name))
            .map(|(_, _, needle)| *needle)
            .unwrap_or_else(|| panic!("the table must list {name}"))
    };

    // Fires on the real thing.
    assert!(contains_token(
        "fn f(x: Option<u32>) -> bool { x.is_some_and(|v| v > 0) }",
        needle_for("Option/Result::is_some_and")
    ));
    assert!(contains_token(
        r#"let s = c"hello";"#,
        needle_for("c\"…\" literals")
    ));

    // Silent on the lookalikes that actually appear in this repository.
    let c_needle = needle_for("c\"…\" literals");
    assert!(
        !contains_token(
            r#"write!(f, "the step function is nondeterministic")"#,
            c_needle
        ),
        "a word ending in `c` before a quote is not a C-string literal"
    );
    assert!(
        !contains_token(r#"DesyncPolicy::Resync => "resync","#, c_needle),
        "neither is a string ending in `c`"
    );

    // The newly added needles fire on their real spellings...
    for (name, sample) in [
        ("int::next_multiple_of", "let n = x.next_multiple_of(8);"),
        ("slice::chunk_by", "for run in v.chunk_by(|a, b| a == b) {}"),
        ("[u8]::trim_ascii", "let t = bytes.trim_ascii();"),
        (
            "std::cell::LazyCell",
            "static X: LazyCell<u8> = LazyCell::new(|| 1);",
        ),
        ("#[expect(…)] attribute", "#[expect(dead_code)]\nfn f() {}"),
        ("slice/Iterator::is_sorted", "assert!(v.is_sorted());"),
        ("num::midpoint", "let m = a.midpoint(b);"),
        ("int::is_multiple_of", "if n.is_multiple_of(3) {}"),
        ("slice::as_chunks", "let (c, r) = v.as_chunks::<4>();"),
        ("slice::first_chunk", "let h = bytes.first_chunk::<8>();"),
        (
            "slice::as_chunks_mut",
            "let (c, r) = v.as_chunks_mut::<4>();",
        ),
        ("let-chains (`&& let`)", "if a && let Some(x) = o {}"),
    ] {
        assert!(
            contains_token(sample, needle_for(name)),
            "the {name} needle does not fire on `{sample}`"
        );
    }

    // ...and stay silent on the spellings they must not claim.
    for (name, sample, why) in [
        (
            "num::midpoint",
            "let p = self.midpoints.len();",
            "a field named `midpoints` is not a call to `midpoint`",
        ),
        (
            "slice/Iterator::is_sorted",
            "let flag = self.is_sorted_flag;",
            "a field starting with `is_sorted` is not the method",
        ),
        (
            "std::cell::LazyCell",
            "// a comment about laziness",
            "prose about laziness is not LazyCell",
        ),
        (
            "let-chains (`&& let`)",
            "if a && letters.is_empty() {}",
            "`letters` after `&&` is an identifier, not a let-chain",
        ),
        (
            "slice::first_chunk",
            "self.first_chunk_index = 0;",
            "dropping the trailing paren must not make the needle match a \
             longer identifier",
        ),
    ] {
        assert!(
            !contains_token(sample, needle_for(name)),
            "the {name} needle wrongly fired on `{sample}` — {why}"
        );
    }

    // And the table's premise holds: is_some_and post-dates the engine's MSRV,
    // so the engine check has something to be checking.
    let (_, (major, minor), _) = POST_MSRV_APIS
        .iter()
        .find(|(n, _, _)| n.starts_with("Option/Result::is_some_and"))
        .expect("listed above");
    assert!((*major, *minor) > declared_msrv("izanagi"));
}
