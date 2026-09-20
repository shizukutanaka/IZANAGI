//! The engine holds no unordered container, checked by the build.
//!
//! `BTreeMap`/`BTreeSet` iterate in key order; `HashMap`/`HashSet` iterate in
//! an order that depends on a per-process random hasher seed. Two runs of the
//! same build can therefore visit entities, voices, or buttons in different
//! orders — and any code folding over that order (the audio mixer adds `f32`
//! samples; `f32` addition is not associative) produces bit-different output
//! on each run. `ecs.rs` once fixed only the iterated columns and left a
//! `HashMap<TypeId, _>` under `World`; `audio.rs` kept `voices: HashMap`.
//! Naming exceptions one by one cannot last, so the rule is now the type
//! itself: these tokens may not appear in `izanagi/src` at all.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn engine_src() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
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
/// Banned in every `izanagi/src` production file. Hash-based containers and
/// the std hashers that seed them. `Hash`/`Hasher` derive-traits stay legal —
/// handles keep `#[derive(Hash)]` for downstream users — it is only the
/// *container and its seed* that decide iteration order.
const BANNED: &[(&str, &str)] = &[
    ("HashMap", "unordered map"),
    ("HashSet", "unordered set"),
    ("hash_map", "unordered map module"),
    ("hash_set", "unordered set module"),
    ("DefaultHasher", "default hasher"),
    ("RandomState", "random hasher seed"),
    ("SipHash", "seeded hasher"),
    ("BuildHasher", "hasher factory"),
];

#[test]
fn engine_sources_hold_no_unordered_containers() {
    let sources = library_sources(&engine_src());
    assert!(!sources.is_empty(), "the engine scan found no sources");
    let mut offenders = Vec::new();
    for (file, code) in &sources {
        for (needle, label) in BANNED {
            if count_token(code, needle) > 0 {
                offenders.push(format!("{file} mentions {needle} ({label})"));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "unordered containers in the engine:\n{}",
        offenders.join("\n")
    );
}

/// `Vec`/`BTreeMap`/`BTreeSet` usage must not trigger the ban — the needle
/// matcher requires identifier boundaries on both sides.
#[test]
fn ordered_containers_and_hash_derives_do_not_trip_the_scan() {
    for clean in [
        "use std::collections::BTreeMap;",
        "let m: BTreeMap<u32, u32> = BTreeMap::new();",
        "let s: BTreeSet<u32> = BTreeSet::new();",
        "#[derive(Hash)] struct Handle(u64);",
        "impl Hash for X {}",
        "map.insert(k, v); set.contains(&k);",
    ] {
        for (needle, _) in BANNED {
            assert_eq!(count_token(clean, needle), 0, "clean code tripped on {needle}: {clean:?}");
        }
    }
    // And a real offender is caught even without `std::` qualification.
    for bad in [
        "use std::collections::HashMap;",
        "let m: HashMap<u32, u32> = HashMap::new();",
        "let s = HashSet::new();",
        "std::collections::hash_map::Entry",
        "DefaultHasher::new()",
        "let h = RandomState::new();",
    ] {
        assert!(
            BANNED.iter().any(|(n, _)| count_token(bad, n) > 0),
            "an offender slipped past the needles: {bad:?}"
        );
    }
}
