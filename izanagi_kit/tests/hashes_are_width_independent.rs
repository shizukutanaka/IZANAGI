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

use std::fs;
use std::path::PathBuf;

fn kit_src() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Library code of every module: everything before `#[cfg(test)]`, with line
/// comments stripped so prose about `usize` does not count as a use of it.
fn library_sources() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for entry in fs::read_dir(kit_src()).expect("kit src").flatten() {
        let path = entry.path();
        if path.extension().map(|e| e != "rs").unwrap_or(true) {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let src = fs::read_to_string(&path).unwrap_or_default();
        let end = src.find("#[cfg(test)]").unwrap_or(src.len());
        let code = src[..end]
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        out.push((name, code));
    }
    out.sort();
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
    for (name, code) in library_sources() {
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
    for (name, code) in library_sources() {
        for sentinel in ["usize::MAX", "isize::MAX", "isize::MIN"] {
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
    let hits = library_sources()
        .iter()
        .filter(|(_, code)| code.contains("fn det_hash("))
        .count();
    assert!(
        hits > 3,
        "found det_hash implementations in only {hits} modules — has the trait \
         been renamed? This test would stop checking anything."
    );
}
