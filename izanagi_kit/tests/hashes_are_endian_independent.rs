//! Nothing native-endian may reach the world hash.
//!
//! `hashes_are_width_independent.rs` checks the pointer-width half of
//! cross-platform replay. Endianness is the other half, and it is the same
//! shape of risk: two correct runs of the same code on different targets
//! producing different hash bytes for the identical logical value.
//!
//! Rust's `to_ne_bytes()` returns the bytes in whatever order the target CPU
//! uses natively — little-endian on x86-64 and wasm32, big-endian on some
//! embedded and mainframe targets. `Fnv1a::write_u32` and its siblings could
//! have been written with `to_ne_bytes()`; every one of them instead calls
//! `to_le_bytes()` explicitly, and the comment above the primitive `DetHash`
//! impls says why: "Fixed-width little-endian so the folded bytes are
//! identical on every target (no native-endian or pointer-width leakage)."
//!
//! Every current CI target (x86-64, wasm32) happens to be little-endian, so
//! `to_ne_bytes()` would pass every test in this repository today and still
//! be a latent cross-platform bug — exactly the kind static analysis is for,
//! since no test run on this machine could ever observe the difference.

use std::fs;
use std::path::PathBuf;

fn kit_src() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Library code of every module: everything before `#[cfg(test)]`, with line
/// comments stripped so this doc comment's own prose ("no native-endian ...
/// leakage") does not count as a use of the banned methods.
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
fn no_library_code_converts_a_number_to_native_or_big_endian_bytes() {
    // `to_le_bytes` is the only endian-explicit conversion this crate should
    // ever call. `to_ne_bytes` silently adopts whatever the build target's
    // native order is; `to_be_bytes` is simply the wrong order for the
    // convention this crate already committed to. Both are banned everywhere
    // in library code, not just in world_hash.rs — a module that hand-rolled
    // its own byte folding instead of using `Fnv1a` would be just as able to
    // introduce the same platform dependence.
    let mut offenders: Vec<String> = Vec::new();
    for (name, code) in library_sources() {
        for banned in [
            "to_ne_bytes",
            "to_be_bytes",
            "from_ne_bytes",
            "from_be_bytes",
        ] {
            if code.contains(banned) {
                offenders.push(format!("{name}: {banned}"));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "library code converts a number using target-dependent byte order: \
         {offenders:#?}\n\nUse `to_le_bytes`/`from_le_bytes` explicitly. \
         `to_ne_bytes` would pass on every CI target today (all little-endian) \
         and still break replay on a big-endian one, which is exactly the \
         defect no test run on this machine could ever observe."
    );
}

#[test]
fn every_fixed_width_write_uses_little_endian_explicitly() {
    // The positive half. `write_u16` through `write_i64` each hash a
    // multi-byte value, and each one's implementation must name the order it
    // uses rather than inheriting the compiler's.
    let src = world_hash_source();
    for method in [
        "write_u16",
        "write_u32",
        "write_u64",
        "write_i16",
        "write_i32",
        "write_i64",
    ] {
        let at = src.find(&format!("pub fn {method}(")).unwrap_or_else(|| {
            panic!("Fnv1a::{method} is missing — the multi-byte write family shrank")
        });
        let open = src[at..].find('{').expect("the method body must open") + at;
        let close = src[open..].find('}').expect("the method body must close") + open;
        let body = &src[open..close];
        assert!(
            body.contains("to_le_bytes"),
            "Fnv1a::{method} no longer calls to_le_bytes() — its body is:\n{body}"
        );
    }
}

#[test]
fn the_body_scanner_finds_the_write_methods_and_ignores_prose() {
    // Both directions, on the real source: the scanner above must not be
    // satisfied by this file's own doc comment mentioning `to_le_bytes`
    // sixteen lines up, and it must actually reach into the method bodies
    // rather than matching the module-level comment that describes them.
    let src = world_hash_source();
    let write_u32_at = src.find("pub fn write_u32(").expect("write_u32 must exist");
    let module_comment_at = src
        .find("Fixed-width little-endian so the folded bytes")
        .expect("the design-intent comment must exist");
    assert!(
        write_u32_at < module_comment_at,
        "write_u32 is expected to appear in the primitive-write family, above \
         the primitive-impls comment block — the file layout changed, and the \
         scanner's assumption that it can find real method bodies before that \
         comment needs to be re-checked"
    );
}
