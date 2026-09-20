//! Mechanical guard for the nondeterminism sources that are **not** floats.
//!
//! `no_float_in_sim.rs` rejects `f32`/`f64` in library code, and for a long
//! time that was the only automated half of "replay-safe". It is not the only
//! way to break a replay. Hash-map iteration order varies per process (the
//! standard hasher is randomly seeded), wall-clock reads vary per run, thread
//! interleaving varies per schedule, and pointer values vary per allocation.
//! Any of them reaching simulation state desyncs a replay exactly as a float
//! would, and none of them was checked.
//!
//! This is not hypothetical in this repository. The engine's `World` stored
//! components in `HashMap<u32, T>` and iterated it directly, so the entity
//! visit order any system saw depended on the process's hasher seed; it was
//! found and fixed by switching to `BTreeMap` (see izanagi/CHANGELOG.md). The
//! kit was never checked for the same class until this file, and it had one:
//! `SpatialHash` stored cells in a `HashMap` and exposed `iter_keys` and
//! `all_occupied_cells`, both documented as unordered while suggesting
//! "process every entity" passes as a use case. Reverting that struct to
//! `HashMap` still fails `spatial_hash`'s own tests today.
//!
//! ## How this is checked
//!
//! Static analysis cannot decide whether a particular map's iteration order
//! reaches output, so the rule is an **allowlist with reasons** rather than a
//! judgement. Every hash-map use in library code must be named below with why
//! it is safe. A new one fails until someone writes that sentence, which is
//! the moment to think about it. Wall-clock, threading (spawned or
//! thread-local), environment reads, pointer identity, and `std`'s
//! version-unstable `DefaultHasher` are rejected outright — no library use
//! of them here is legitimate.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Modules permitted to use `HashMap`/`HashSet` in library code, with the
/// count expected and the reason the iteration order cannot reach output.
///
/// The counts are deliberately exact. A module that grows a new hash map
/// trips this even though it was already on the list, because "this module
/// was fine before" is not an argument about the map you just added.
fn allowed() -> BTreeMap<&'static str, (usize, &'static str)> {
    let mut m = BTreeMap::new();
    m.insert(
        "arch.rs",
        (
            4,
            "entity -> dense-index lookup only; iteration is over the dense \
             Vec, which is insertion-ordered",
        ),
    );
    m.insert(
        "explore.rs",
        (
            3,
            "cell -> archive-index lookup only; the archive itself is a Vec \
             in discovery order, and Archive::iter walks that Vec",
        ),
    );
    m.insert(
        "pathfinding.rs",
        (
            37,
            "search bookkeeping (visited/came_from/g_score) is lookup-only; \
              the two maps that *are* iterated handle order explicitly — \
              farthest_cell breaks ties on a row-major total order, and the \
              flee rescan sorts its cell list before relaxing",
        ),
    );
    m.insert(
        "plan.rs",
        (
            3,
            "visited-hash set, membership queries only; the BFS frontier is a \
             VecDeque",
        ),
    );
    m.insert(
        "validator.rs",
        (
            7,
            "name/glyph interning and occupancy, all membership queries; \
             diagnostics are emitted in source order from the Content vectors",
        ),
    );
    m.insert(
        "verify.rs",
        (
            5,
            "visited-hash set for state dedup, membership queries only; the \
             search frontier is a VecDeque and parent links are a Vec",
        ),
    );
    m
}

/// Sources rejected outright in library code: `(name, needle)`.
const BANNED: &[(&str, &str)] = &[
    ("SystemTime", "SystemTime"),
    ("Instant", "Instant"),
    ("thread spawning", "thread::spawn"),
    ("raw pointer address", ".as_ptr("),
    ("explicit RandomState", "RandomState"),
    ("environment access", "env::"),
    ("thread-local state", "thread_local"),
    ("unversioned std hashing", "DefaultHasher"),
    // Ambient inputs the input log cannot replay: the filesystem, the
    // process table, std I/O. `fs::`/`process::` catch `use`-shortened calls;
    // the `std::` spellings catch the fully qualified path a bare needle
    // would miss at the end of a `use` line. (bin/ is out of scope — a CLI
    // exists to read argv and files.)
    ("process module", "std::process"),
    ("process control", "process::"),
    ("filesystem module", "std::fs"),
    ("filesystem access", "fs::"),
    ("I/O module", "std::io"),
    // CPU feature detection (`std::arch::is_x86_feature_detected!` and the
    // intrinsics behind it) is a runtime branch on *which machine* runs the
    // binary — the same simulation code would take different paths on two
    // honest builds on different hardware.
    ("CPU feature detection", "std::arch"),
    ("CPU feature detection", "core::arch"),
    // `catch_unwind`/`panic::` are panic *machinery*: in a crate whose public
    // contract is saturate/None/no-op on bad input (G7), catching a panic is
    // how a real panic gets laundered into a passing result. The lint denies
    // panic! but the runtime API around it is a different door to the same
    // room.
    ("panic machinery", "catch_unwind"),
    ("panic machinery", "panic::"),
];

fn kit_src() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// The offset of a file's test module — the first `#[cfg(test)]` that is a
/// real attribute line. A `//` comment merely *mentioning* the marker must not
/// end library code early: lib.rs carries such a comment, and a bare
/// `find("#[cfg(test)]")` on the raw source truncates the scan at the doc
/// header, silently removing the file's real body from every check below.
fn test_module_boundary(src: &str) -> Option<usize> {
    src.match_indices("#[cfg(test)]").find_map(|(i, _)| {
        let start = src[..i].rfind('\n').map_or(0, |p| p + 1);
        (!src[start..i].trim_start().starts_with("//")).then_some(i)
    })
}

/// Library sources, keyed by path relative to `src/`. `src/bin/` is excluded:
/// those are CLI binaries, and reading argv or the environment is their job.
fn library_sources() -> BTreeMap<String, String> {
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
                let impl_end = test_module_boundary(&src).unwrap_or(src.len());
                let code = src[..impl_end]
                    .lines()
                    .filter(|l| !l.trim_start().starts_with("//"))
                    .collect::<Vec<_>>()
                    .join("\n");
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .display()
                    .to_string();
                out.insert(rel, code);
            }
        }
    }
    let mut out = BTreeMap::new();
    let root = kit_src();
    walk(&root, &root, &mut out);
    out
}

/// Count occurrences of `needle` as a token, rather than as part of a longer
/// word.
///
/// A boundary is only required on a side where the needle itself begins or
/// ends with an identifier character: `HashMap` must not match inside
/// `MyHashMapWrapper`, but `.as_ptr(` is already delimited by its own `.` and
/// `(` and must still match in `v.as_ptr()`. Getting this wrong is how the
/// MSRV scanner in this suite shipped a false positive.
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
fn every_hash_map_in_library_code_is_accounted_for() {
    let sources = library_sources();
    assert!(
        sources.len() > 50,
        "expected to find the kit's library sources, found {} — has the \
         layout changed?",
        sources.len()
    );
    let allowed = allowed();
    let mut problems = Vec::new();

    for (file, code) in &sources {
        let count = count_token(code, "HashMap") + count_token(code, "HashSet");
        match (count, allowed.get(file.as_str())) {
            (0, None) => {}
            (0, Some(_)) => problems.push(format!(
                "{file} no longer uses HashMap/HashSet — remove it from the \
                 allowlist so the list keeps meaning something"
            )),
            (n, None) => problems.push(format!(
                "{file} uses HashMap/HashSet {n} time(s) and is not on the \
                 allowlist. If its iteration order cannot reach output, add it \
                 with that reason; if it can, use BTreeMap/BTreeSet or sort \
                 before iterating"
            )),
            (n, Some((expected, _))) if n != *expected => problems.push(format!(
                "{file} uses HashMap/HashSet {n} time(s), allowlist says \
                 {expected}. A new one needs its own justification — being on \
                 the list already is not an argument about the map just added"
            )),
            _ => {}
        }
    }
    assert!(
        problems.is_empty(),
        "hash-map audit failed:\n{}",
        problems.join("\n")
    );
}

#[test]
fn library_code_has_no_unstable_inputs() {
    let mut problems = Vec::new();
    for (file, code) in library_sources() {
        for (name, needle) in BANNED {
            if count_token(&code, needle) > 0 {
                problems.push(format!(
                    "{file}: {name} in library code — it varies per run, so \
                     anything it reaches cannot replay"
                ));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "nondeterministic source in library code:\n{}",
        problems.join("\n")
    );
}

#[test]
fn the_scanner_fires_and_stays_silent_in_the_right_places() {
    // Both directions, because the MSRV scanner in this same test suite
    // shipped a false positive from only ever checking that it fires.
    assert_eq!(count_token("let m: HashMap<u32, u32>", "HashMap"), 1);
    assert_eq!(count_token("struct MyHashMapWrapper;", "HashMap"), 0);
    assert_eq!(count_token("let t = Instant::now();", "Instant"), 1);
    assert_eq!(count_token("struct InstantiationCache;", "Instant"), 0);
    assert_eq!(count_token("v.as_ptr()", ".as_ptr("), 1);
    assert_eq!(count_token("let k = std::env::var(\"X\");", "env::"), 1);
    assert_eq!(count_token("let myenv::X = 1;", "env::"), 0);
    assert_eq!(
        count_token("thread_local! { static A: u8 = 0 }", "thread_local"),
        1
    );
    assert_eq!(count_token("DefaultHasher::new()", "DefaultHasher"), 1);
}
