//! Mechanical guard for the panic surface of library (and bin) code.
//!
//! `detect_never_panics.rs` and `parse_never_panics.rs` show the contract
//! holds on the inputs the fuzz corpus reaches, but fuzzing can never show
//! the other direction — that no code path *can* panic. The corpus is finite
//! and shaped by today's detectors; a new parser can ship an indexing bug
//! that stays silent until the corpus happens to tickle it, or forever.
//!
//! The strong form of the property is structural: the crate's convention is
//! that hostile input reaches `Option`/`?`/`get()`/`unwrap_or`, while panic
//! is reserved for the *laziness* primitives nobody ever has a good reason
//! to reach for — and, separately, for a small closed set of *invariant*
//! primitives whose panic can only fire on a programmer error.
//!
//! The audit this file records found exactly that split in the wild:
//!
//! - `.unwrap()`, `.unwrap_err()`, `.expect(...)`, `.expect_err(...)`,
//!   `panic!`, `todo!`, `unimplemented!`: **zero** occurrences in production
//!   code across all ~2,400 modules. (wkt.rs once had a private lexer
//!   helper named `expect` — it shadowed `Option::expect` visually, so it
//!   was renamed `want` to keep this scan unconditional.)
//! - `assert!`/`assert_eq!`/`assert_ne!`/`debug_assert!`/`unreachable!`:
//!   present, and every site is an invariant — constructor contracts
//!   (`Timestep::new(0, 0)` is a caller bug, not input), or `unreachable!`
//!   arms provably dead by a `len` check two lines up. Those stay legal,
//!   but each one must be named below with its reason, the way
//!   `no_nondeterminism_in_sim.rs` names its hash maps. A new invariant
//!   assert trips the test until someone writes the sentence — which is
//!   the moment to check it really cannot fire on input.
//!
//! The rule of thumb, enforced below: anything reachable from user bytes
//! goes through `Option`/`Result`; anything reachable only through a wrong
//! caller may `assert!`, with a message. Forgetting which is which is what
//! turns a library into a remote crash.
//!
//! ## How this is checked
//!
//! Production code is everything before the first real `#[cfg(test)]`
//! attribute line (found by the same tiny lexer the nondeterminism scan
//! uses, so `//`-comments and raw strings cannot fake the boundary), with
//! `//` comment lines then dropped. `src/bin/` is *included*: a CLI that
//! panics on user input is still a crash, and the binary happens to be
//! clean too.
//!
//! Two scans:
//!
//! 1. `production_code_has_no_laziness_panics` — the unconditional ban
//!    list. Every needle is token-checked so `todo_item` does not trip
//!    `todo!` and `MyUnwrap.unwrap()` still does.
//! 2. `invariant_asserts_are_allowlisted` — per-file exact counts plus a
//!    reason for each `assert!`-family site.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Panic primitives rejected outright in production code, `(name, needle)`.
///
/// `assert!` is *not* here: it is the sanctioned spelling for an
/// invariant-with-a-message, and the allowlist below tracks its sites.
/// `.expect(msg)` is the same contract in `.unwrap()` clothing — the ban
/// keeps one spelling so a reviewable grep has one needle, not two.
const BANNED: &[(&str, &str)] = &[
    // The four spellings of "I hope this is Some": each is a branch the
    // type system already forces you to write — `?`, `unwrap_or`,
    // `map_or`, `let-else` — where the None arm cannot crash a process.
    ("unwrap", ".unwrap()"),
    ("unwrap_err", ".unwrap_err("),
    ("expect", ".expect("),
    ("expect_err", ".expect_err("),
    // Bare panics and placeholder holes. `panic!` reaching production
    // code means an `Err` variant or a None was skipped; `todo!` /
    // `unimplemented!` mean the function was never finished.
    ("panic", "panic!("),
    ("todo", "todo!"),
    ("unimplemented", "unimplemented!"),
];

/// Invariant primitives allowed in production code, per file, with the
/// expected total count across `assert!`/`assert_eq!`/`assert_ne!`/
/// `debug_assert!`/`unreachable!` and why each site can only fire on a
/// programmer error.
///
/// The counts are deliberately exact — a module that grows a new invariant
/// assert trips this even though it was already listed, because "this file
/// was fine before" is not an argument about the check just added. If a
/// site can fire on input bytes, it does not belong here at all: convert
/// it to `Option`/`Result` control flow.
fn allowed() -> BTreeMap<&'static str, (usize, &'static str)> {
    let mut m = BTreeMap::new();
    m.insert(
        "fov.rs",
        (
            1,
            "debug_assert!: `Frac`'s denominator is constructed positive and \
             only ever re-derived from subtraction of two positive parts; \
             the assert documents the invariant for debug builds",
        ),
    );
    m.insert(
        "identify.rs",
        (
            1,
            "Identification::new asserts labels.len() covers the distinct \
             kinds — a caller-side resource contract, checked before any \
             shuffling reads either slice",
        ),
    );
    m.insert(
        "netinput.rs",
        (
            3,
            "AdaptiveDelay::new asserts min<=max delay, a nonzero window, \
             and ordered permille bounds — constructor arguments, not file \
             or network input",
        ),
    );
    m.insert(
        "replay.rs",
        (
            2,
            "two unreachable! arms: `tick < max(len_a, len_b)` already \
             guarantees at least one side yields Some, so the (None, None) \
             case is dead by construction",
        ),
    );
    m.insert(
        "rollback.rs",
        (
            2,
            "SnapshotRing::new asserts capacity>0 and stride>0 — degenerate \
             rings are a caller bug that would silently never snapshot",
        ),
    );
    m.insert(
        "timestep.rs",
        (
            2,
            "Timestep::new asserts steps_per_second>0 and max_steps>0 — a \
             zeroed timestep would divide by zero downstream",
        ),
    );
    m.insert(
        "wallet.rs",
        (
            1,
            "debug_assert! on the merge path's balance check — deposits \
             maintain positive balances by construction; debug builds \
             re-verify, release stays branch-free",
        ),
    );
    m
}

fn kit_src() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// The offset of a file's test module — the first `#[cfg(test)]` that is a
/// real attribute line. A `//` comment merely *mentioning* the marker must
/// not end production code early: a bare `find("#[cfg(test)]")` on the raw
/// source truncates the scan at a doc header, silently removing the file's
/// real body from every check below.
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

/// Production sources, keyed by path relative to `src/`, with `//` comment
/// lines removed and everything from `#[cfg(test)]` onward cut. `src/bin/`
/// is included — a CLI that panics on user input is still a crash.
fn production_sources(src_root: &Path) -> BTreeMap<String, String> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, String>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, root, out);
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                let src = fs::read_to_string(&path).unwrap_or_default();
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

/// Count occurrences of `needle` as a token, rather than as part of a
/// longer word.
///
/// A boundary is only required on a side where the needle itself begins or
/// ends with an identifier character: `panic!(` must not match inside
/// `panic_handler!(`, and `todo!` must not match `todo_items` — but
/// `.unwrap()` is already delimited by its own `.` and `)` and must still
/// match in `v.unwrap()`.
fn count_token(code: &str, needle: &str) -> usize {
    fn ident_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }
    let bytes: Vec<char> = code.chars().collect();
    let pat: Vec<char> = needle.chars().collect();
    if pat.is_empty() || bytes.len() < pat.len() {
        return 0;
    }
    let check_left = pat.first().copied().map(ident_char).unwrap_or(false);
    let check_right = pat.last().copied().map(ident_char).unwrap_or(false);
    let mut n = 0;
    for i in 0..=bytes.len() - pat.len() {
        if bytes[i..i + pat.len()] != pat[..] {
            continue;
        }
        let left_ok = !check_left || i == 0 || !ident_char(bytes[i - 1]);
        let after = i + pat.len();
        let right_ok = !check_right || after >= bytes.len() || !ident_char(bytes[after]);
        if left_ok && right_ok {
            n += 1;
        }
    }
    n
}

#[test]
fn production_code_has_no_laziness_panics() {
    let sources = production_sources(&kit_src());
    assert!(
        sources.len() > 50,
        "expected to find the kit's production sources, found {} — has the \
         layout changed?",
        sources.len()
    );
    let mut problems = Vec::new();
    for (file, code) in &sources {
        for (name, needle) in BANNED {
            let n = count_token(code, needle);
            if n > 0 {
                problems.push(format!(
                    "{file}: {n} × `{needle}` — production code returns \
                     Option/Result for input it cannot handle; `{name}` is \
                     a crash, not a branch"
                ));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "laziness panic in production code:\n{}",
        problems.join("\n")
    );
}

#[test]
fn invariant_asserts_are_allowlisted() {
    const INVARIANT: &[&str] = &[
        "assert!(",
        "assert_eq!(",
        "assert_ne!(",
        "debug_assert!(",
        "unreachable!(",
    ];
    let sources = production_sources(&kit_src());
    let allowed = allowed();
    let mut problems = Vec::new();

    for (file, code) in &sources {
        let count: usize = INVARIANT.iter().map(|n| count_token(code, n)).sum();
        match (count, allowed.get(file.as_str())) {
            (0, None) => {}
            (0, Some(_)) => problems.push(format!(
                "{file} no longer has any invariant assert — remove it from \
                 the allowlist so the list keeps meaning something"
            )),
            (n, None) => problems.push(format!(
                "{file} has {n} assert!-family site(s) and is not on the \
                 allowlist. If the condition can only fail on a programmer \
                 error, add the file with that reason; if it can fail on \
                 input, return None/Err instead"
            )),
            (n, Some((expected, _))) if n != *expected => problems.push(format!(
                "{file} has {n} assert!-family site(s), allowlist says \
                 {expected}. A new one needs its own justification — being \
                 on the list already is not an argument about the check \
                 just added"
            )),
            _ => {}
        }
    }
    assert!(
        problems.is_empty(),
        "invariant-assert audit failed:\n{}",
        problems.join("\n")
    );
}

#[test]
fn the_scanner_fires() {
    // A file that literally unwraps must be reported; so must an extra
    // assert site.
    let bad = "pub fn f(b: &[u8]) -> u8 {\n    let x = Some(1u8);\n    \
               x.unwrap() + b[0]\n}\n";
    assert_eq!(count_token(bad, ".unwrap()"), 1);
    assert_eq!(count_token("todo!(\"later\")", "todo!"), 1);
    assert_eq!(count_token("let todo_list = 1;", "todo!"), 0);
    // `.unwrap_or(` is a different, safe call and must not be counted.
    assert_eq!(count_token("v.unwrap_or(0)", ".unwrap()"), 0);
    // assert_eq! counts toward the invariant total, distinct from assert!.
    assert_eq!(count_token("assert_eq!(a, b)", "assert!("), 0);
    assert_eq!(count_token("assert_eq!(a, b)", "assert_eq!("), 1);
}
