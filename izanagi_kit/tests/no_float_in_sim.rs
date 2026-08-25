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

fn is_ident_byte(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphanumeric()
}

/// `tok` appears in `line` as a standalone identifier (not a substring of a
/// longer identifier such as `from_f64_lossy` or `myf32`).
fn contains_token(line: &str, tok: &str) -> bool {
    let bytes = line.as_bytes();
    let mut from = 0;
    while let Some(rel) = line[from..].find(tok) {
        let start = from + rel;
        let end = start + tok.len();
        let before_ok = start == 0 || !is_ident_byte(bytes[start - 1]);
        let after_ok = end >= bytes.len() || !is_ident_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        from = start + 1;
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
        for (lineno, raw) in src.lines().enumerate() {
            // Stop at the (single, trailing) test module: everything after is
            // legitimately allowed to use floats.
            if raw.contains("#[cfg(test)]") {
                break;
            }
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
