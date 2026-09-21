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
/// comments stripped so this doc comment's own prose ("no native-endian ...
/// leakage") does not count as a use of the banned methods.
fn library_sources(src_root: &Path) -> BTreeMap<String, String> {
    // Whitespace may sit on either side of `::`, `.`, `!`, `#` and before
    // `(`/`[` — `env :: var`, `cfg ! (x)` and `# [cfg]` all compile while
    // spelling no needle (verified: `std :: env :: var` in a test file
    // passed the whole suite green). Scanners must see the token stream
    // the compiler sees, so sigil-adjacent space is squeezed out here.
    fn squeeze_sigil_ws(code: &str) -> String {
        const SIGILS: &[char] = &[':', '.', '!', '#'];
        let chars: Vec<char> = code.chars().collect();
        // Pass 1: block comments are tokens too (`env /*..*/ :: var`
        // compiles while spelling no needle — verified green by injection)
        // — drop them with a ` ` marker; strings and char literals keep
        // their contents (a fake `'/*'` must not open comment mode).
        let mut pass = String::with_capacity(code.len());
        let mut in_str = false;
        let mut esc = false;
        let mut depth = 0usize;
        let mut i = 0usize;
        while i < chars.len() {
            let c = chars[i];
            if in_str {
                pass.push(c);
                if esc {
                    esc = false;
                } else if c == '\\' {
                    esc = true;
                } else if c == '"' {
                    in_str = false;
                }
                i += 1;
                continue;
            }
            if c == '\'' {
                // A char literal must be consumed whole — `'/*'` cannot be
                // allowed to fake-open comment mode and swallow real code.
                // A lifetime ('a, 'static) stays code.
                pass.push(c);
                let n1 = chars.get(i + 1).copied();
                let n2 = chars.get(i + 2).copied();
                if n1 == Some('\\') {
                    i += 2;
                    while i < chars.len() && chars[i] != '\'' {
                        i += 1;
                    }
                    i += 1;
                } else if n1.is_some() && n2 == Some('\'') {
                    i += 3;
                } else {
                    i += 1;
                }
                continue;
            }
            if depth > 0 {
                if c == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                    continue;
                }
                if c == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                    continue;
                }
                i += 1;
                continue;
            }
            if c == '/' && chars.get(i + 1) == Some(&'*') {
                depth = 1;
                pass.push(' ');
                i += 2;
                continue;
            }
            if c == '"' {
                in_str = true;
            }
            pass.push(c);
            i += 1;
        }
        // Pass 2: sigil-adjacent whitespace reads as one token.
        let chars: Vec<char> = pass.chars().collect();
        let mut out = String::with_capacity(pass.len());
        for (i, &c) in chars.iter().enumerate() {
            if c.is_whitespace() {
                let prev = out.chars().last();
                let next = chars[i + 1..].iter().find(|n| !n.is_whitespace());
                let squeeze = matches!(prev, Some(p) if SIGILS.contains(&p))
                    || matches!(next, Some(&n) if SIGILS.contains(&n) || n == '(' || n == '[');
                if !squeeze {
                    out.push(c);
                }
                continue;
            }
            out.push(c);
        }
        out
    }

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
                out.insert(rel, squeeze_sigil_ws(&stripped));
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
fn no_library_code_converts_a_number_to_native_or_big_endian_bytes() {
    // `to_le_bytes` is the only endian-explicit conversion this crate should
    // ever call. `to_ne_bytes` silently adopts whatever the build target's
    // native order is; `to_be_bytes` is simply the wrong order for the
    // convention this crate already committed to. Both are banned everywhere
    // in library code, not just in world_hash.rs — a module that hand-rolled
    // its own byte folding instead of using `Fnv1a` would be just as able to
    // introduce the same platform dependence.
    let mut offenders: Vec<String> = Vec::new();
    for (name, code) in library_sources(&kit_src()) {
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
