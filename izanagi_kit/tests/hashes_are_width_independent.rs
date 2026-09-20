//! Nothing pointer-sized may reach the world hash.
//!
//! The crate's headline is that a simulation "replays bit-identically". The
//! pinned hashes prove that for one machine and one compiler. Across machines,
//! the argument has always been about floats — but `usize` is the other way a
//! state hash can differ between two correct runs of the same code, and it is
//! not hypothetical here: the CI workflow builds the kit for
//! `wasm32-unknown-unknown`, where `usize` is 32 bits and this one is 64.
//!
//! The design already gets it right, and does so structurally rather than by
//! remembering:
//!
//! - `DetHash` has no `usize` or `isize` implementation, so a pointer-sized
//!   value cannot be hashed by simply being handed to the hasher.
//! - `Fnv1a`, the hasher, exposes `write_u8/u16/u32/u64/i16/i32/i64`, and no
//!   `write_usize`. The width is always chosen explicitly at the call site.
//! - Collection lengths are hashed as `u32` (`self.len() as u32`), which is
//!   the same value on any target.
//!
//! Being right by construction is worth more than being right by care, but
//! only if the construction is held in place. Adding `impl DetHash for usize`
//! is a three-line change that would break cross-platform replay everywhere at
//! once, and the wasm job would not notice: it compiles the crate, it does not
//! compare a hash against another target's. So the shape is checked here.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn kit_src() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// The offset of a file's test module — the first `#[cfg(test)]` that is a
/// real attribute line, not the text of a comment that mentions it. lib.rs
/// once discussed the marker inside a `//` comment; searching the raw source
/// found that first and scanned only the doc header.
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

/// Library code of every module: everything before `#[cfg(test)]`, with line
/// comments stripped so prose about `usize` does not count as a use of it.
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

fn world_hash_source() -> String {
    fs::read_to_string(kit_src().join("world_hash.rs")).expect("world_hash.rs")
}

#[test]
fn det_hash_has_no_pointer_sized_implementation() {
    let src = world_hash_source();
    for ty in ["usize", "isize"] {
        assert!(
            !src.contains(&format!("impl DetHash for {ty}")),
            "`impl DetHash for {ty}` exists. A {ty} is 64 bits here and 32 on \
             wasm32, so any state containing one hashes differently on the two \
             targets and the crate's bit-identical-replay claim fails across \
             platforms. Hash the value at an explicit width instead."
        );
    }
}

#[test]
fn the_hasher_offers_no_pointer_sized_write() {
    // The API is the enforcement: a caller cannot hash a `usize` without
    // writing the cast, and writing the cast is the moment to think about
    // which width the protocol wants.
    let src = world_hash_source();
    for method in ["write_usize", "write_isize"] {
        assert!(
            !src.contains(&format!("pub fn {method}")),
            "Fnv1a::{method} exists. Every write must name a fixed width, so \
             that the same state produces the same bytes on a 32-bit and a \
             64-bit target."
        );
    }
    // And the fixed-width writes it does offer are still there, so this test
    // cannot pass by the hasher having lost its API entirely.
    for method in [
        "write_u8",
        "write_u16",
        "write_u32",
        "write_u64",
        "write_i64",
    ] {
        assert!(
            src.contains(&format!("pub fn {method}")),
            "Fnv1a::{method} is missing — the fixed-width API is what makes \
             the absence of write_usize a design rather than a gap"
        );
    }
}

#[test]
fn no_det_hash_implementation_mentions_a_pointer_sized_type() {
    // `self.items.len() as u32` is the correct idiom and does not name `usize`
    // at all. Anything that does name it inside a `det_hash` body is either
    // hashing a pointer-sized value or casting one, and both deserve the
    // reader's attention.
    let mut offenders: Vec<String> = Vec::new();
    for (name, code) in library_sources(&kit_src()) {
        let mut rest = code.as_str();
        while let Some(at) = rest.find("fn det_hash(") {
            rest = &rest[at..];
            let Some(open) = rest.find('{') else { break };
            let mut depth = 0i32;
            let mut end = open;
            for (i, c) in rest[open..].char_indices() {
                if c == '{' {
                    depth += 1;
                } else if c == '}' {
                    depth -= 1;
                    if depth == 0 {
                        end = open + i;
                        break;
                    }
                }
            }
            let body = &rest[open..end];
            for ty in ["usize", "isize"] {
                if body.contains(ty) {
                    offenders.push(format!("{name}: a det_hash body mentions `{ty}`"));
                }
            }
            rest = &rest[end.max(open + 1)..];
        }
    }
    assert!(
        offenders.is_empty(),
        "pointer-sized values are reaching the world hash: {offenders:#?}\n\n\
         Hash lengths as `u32` and indices at an explicit width. A hash that \
         depends on the target's pointer size is not a replay hash."
    );
}

#[test]
fn library_code_uses_no_pointer_sized_sentinel() {
    // `usize::MAX` is 2^64-1 here and 2^32-1 on wasm32. As a sentinel, a
    // saturating bound or a "not found" marker it changes behaviour between
    // targets even when it never reaches a hash directly. The kit's only use
    // is in a test that deliberately feeds an out-of-range index to a
    // `CellSelector` to check the contradiction path, which is why this scans
    // library code only.
    let mut offenders: Vec<String> = Vec::new();
    for (name, code) in library_sources(&kit_src()) {
        for sentinel in [
            "usize::MAX",
            "isize::MAX",
            "isize::MIN",
            // `BITS` is the same leak in constant form: usize::BITS is 64
            // here and 32 on wasm32.
            "usize::BITS",
            "isize::BITS",
        ] {
            if code.contains(sentinel) {
                offenders.push(format!("{name}: {sentinel}"));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "library code uses a pointer-sized sentinel: {offenders:#?}\n\n\
         Its value differs between 32-bit and 64-bit targets. Use an explicit \
         `u32::MAX`/`u64::MAX`, or an `Option`, so behaviour is the same \
         everywhere."
    );
}

#[test]
fn lengths_are_hashed_at_a_fixed_width() {
    // The positive half: the pattern that makes the absence of a usize impl
    // workable must actually be in use, or the modules would have had to find
    // some other way to hash a length — and the interesting ways are all
    // wrong.
    let src = world_hash_source();
    assert!(
        src.contains("self.len() as u32"),
        "world_hash no longer hashes lengths as u32. Whatever replaced it has \
         to be width-independent too, and this test should be updated to say \
         how."
    );
}

#[test]
fn the_body_scanner_finds_bodies_and_ignores_prose() {
    // Both directions on synthetic input: a scanner that found no det_hash
    // bodies would make the check above pass vacuously.
    let hits = library_sources(&kit_src())
        .iter()
        .filter(|(_, code)| code.contains("fn det_hash("))
        .count();
    assert!(
        hits > 3,
        "found det_hash implementations in only {hits} modules — has the trait \
         been renamed? This test would stop checking anything."
    );
}
