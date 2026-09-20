//! Mechanical guard for the **"no float in the deterministic layer"** invariant.
//!
//! ## Why this exists
//!
//! Bit-identical cross-platform determinism is the crate's core promise.
//! `#![forbid(unsafe_code)]` makes the *no-unsafe* half of that promise
//! compiler-enforced — but the equally load-bearing *no-float* half
//! (`f32`/`f64` arithmetic rounds differently across x87 vs. SSE, with vs.
//! without FMA contraction, and across optimization levels) is enforced by
//! nothing but discipline and code review. A contributor adding
//! `let speed = dist as f32 * 0.5;` to `mapgen`, `combat`, or `pathfinding`
//! would compile cleanly, pass every existing test, and silently break replay
//! / lockstep bit-identity.
//!
//! This test turns that discipline into a tripwire: it scans every production
//! (non-`#[cfg(test)]`) source line in `src/` and rejects `f32` / `f64` type
//! tokens and decimal float literals. Floats remain fine in `#[cfg(test)]`
//! modules (e.g. comparing a filled-circle area to π·r²), which are excluded.
//!
//! ## Scope & known limits
//!
//! - **Scanned:** all `.rs` under `src/`, only the region *before* each file's
//!   `#[cfg(test)]` marker (the crate convention is one trailing test module per
//!   file). Line comments (`//…`) are stripped first; the crate uses no block
//!   comments.
//! - **Caught:** `f32` / `f64` tokens (typed fields, `as` casts, `f64::`
//!   paths, fn signatures) and `<digits>.<digits>` float literals.
//! - **Blind spot:** a fully type-inferred float binding with no literal and no
//!   `f32`/`f64` annotation (`let a = some_f(); let b = a * a;`). In practice a
//!   float reaching a deterministic surface needs a typed field or cast that this
//!   guard sees. If the invariant is ever *intentionally* relaxed for a module,
//!   relax this test deliberately and document it — do not silently exempt code.

use std::fs;
use std::path::{Path, PathBuf};

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

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read_dir src") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// `tok` appears in `line` as a token: a boundary is required on each side
/// only where the needle itself ends in an identifier character, so `f32`
/// does not match inside `myf32` but `.as_ptr(` still matches `v.as_ptr(x`.
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

/// A decimal float literal — a run of digits, *not* itself preceded by `.`
/// (which would make it a tuple-field access like `pair.0.1`), immediately
/// followed by `.` and another digit.
fn contains_float_literal(line: &str) -> bool {
    let b = line.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_digit() {
            let run_start = i;
            while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'_') {
                i += 1;
            }
            let preceded_by_dot = run_start > 0 && b[run_start - 1] == b'.';
            let followed_by_frac =
                i < b.len() && b[i] == b'.' && i + 1 < b.len() && b[i + 1].is_ascii_digit();
            if !preceded_by_dot && followed_by_frac {
                return true;
            }
        } else {
            i += 1;
        }
    }
    false
}

/// Strip a `//` line comment. Approximate (does not parse string literals), but
/// the crate has no production strings containing `//`, and any miss is a false
/// *negative* (a hidden float) rather than a false positive.
fn strip_comment(line: &str) -> &str {
    match line.find("//") {
        Some(idx) => &line[..idx],
        None => line,
    }
}

#[test]
fn test_no_float_in_production_code() {
    let mut files = Vec::new();
    collect_rs(Path::new("src"), &mut files);
    assert!(!files.is_empty(), "found no source files to scan");
    files.sort();

    let mut violations: Vec<String> = Vec::new();

    for path in &files {
        let src = fs::read_to_string(path).expect("read source");
        // Stop at the (single, trailing) test module: everything after is
        // legitimately allowed to use floats.
        let end = test_module_boundary(&src).unwrap_or(src.len());
        for (lineno, raw) in src[..end].lines().enumerate() {
            let code = strip_comment(raw);
            let kind = if contains_token(code, "f32") || contains_token(code, "f64") {
                Some("f32/f64 type")
            } else if contains_float_literal(code) {
                Some("float literal")
            } else {
                None
            };
            if let Some(kind) = kind {
                violations.push(format!(
                    "{}:{} [{}]  {}",
                    path.display(),
                    lineno + 1,
                    kind,
                    code.trim()
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "float usage found in deterministic (non-test) code — this breaks \
         cross-platform bit-identity. Use `Fixed` (Q16.16) instead, or move the \
         code into a `#[cfg(test)]` module if it is test-only:\n{}",
        violations.join("\n")
    );
}

#[test]
fn test_scanner_self_check() {
    // The detectors must actually fire, or the guard above is vacuous.
    assert!(contains_token("let x: f64 = a;", "f64"));
    assert!(contains_token("d as f32", "f32"));
    assert!(
        !contains_token("from_f64_lossy(x)", "f64"),
        "substring must not match"
    );
    assert!(
        !contains_token("myf32_var", "f32"),
        "substring must not match"
    );
    assert!(contains_float_literal("let r = 0.5;"));
    assert!(contains_float_literal("3.14159"));
    assert!(
        !contains_float_literal("pair.0.1"),
        "tuple access is not a float"
    );
    assert!(!contains_float_literal("0..10"), "range is not a float");
    assert!(!contains_float_literal("0xC0FFEE00"), "hex is not a float");
    assert_eq!(strip_comment("let x = 1; // 0.5 note"), "let x = 1; ");
}
