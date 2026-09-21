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
    // `.as_mut_ptr(` is the spelling `.as_ptr(` misses; `as *`, `*const`,
    // `*mut` cover the safe cast forms `&x as *const T as usize` — every one
    // leaks an address into hashed or output bytes.
    ("raw pointer address", ".as_mut_ptr("),
    ("raw pointer cast", "as *"),
    ("raw pointer type", "*const"),
    ("raw pointer type", "*mut"),
    // Concurrency primitives: scheduling is the machine's ambient input —
    // a Mutex/RwLock/channel/atomic makes ordering depend on the OS, which
    // the input log cannot replay. `thread::`/`std::thread`/`std::sync`
    // cover the module paths; the type names cover `use`-shortened code.
    ("thread module", "std::thread"),
    ("thread calls", "thread::"),
    ("sync module", "std::sync"),
    ("mutex", "Mutex"),
    ("rwlock", "RwLock"),
    ("channel", "channel("),
    ("mpsc", "mpsc"),
    ("atomic", "Atomic"),
    ("condvar", "Condvar"),
    ("barrier", "Barrier"),
    ("explicit RandomState", "RandomState"),
    ("environment access", "env::"),
    // `use std::env as e` spells no `env::` yet compiles `e::var` — the
    // module path itself is banned so the alias import still names it.
    ("environment access", "std::env"),
    // Same alias hole for panic machinery: `use std::panic as p` dodges
    // `panic::` while keeping `p::catch_unwind` legal text.
    ("panic machinery", "std::panic"),
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
    // The network stack is ambient input of the same kind and worse: a
    // socket hands the sim bytes no input log recorded, and two honest
    // runs read different bytes. `std::net`/`net::` catch module paths
    // (`netinput` is safe — `net::` cannot match `netinput::`), the type
    // names catch `use`-shortened calls.
    ("network module", "std::net"),
    ("network module", "net::"),
    ("network", "TcpStream"),
    ("network", "TcpListener"),
    ("network", "UdpSocket"),
    ("network", "ToSocketAddrs"),
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
    // Pointer *values*, not just casts: `into_raw`/`from_raw` (covering the
    // `_parts` spellings too), the strict-provenance surface (`expose_addr`,
    // `with_addr`, `map_addr`, `.addr(`, `with_exposed_provenance`),
    // `NonNull`, and identity comparisons (`ptr_eq`, `ptr::`) all turn an
    // allocator's choices into data — different run, different addresses,
    // different hash. The pointer format flag prints the same value into
    // output bytes.
    ("pointer identity", "into_raw"),
    ("pointer identity", "from_raw"),
    ("pointer identity", "expose_addr"),
    ("pointer identity", "with_addr"),
    ("pointer identity", "map_addr"),
    ("pointer identity", ".addr("),
    ("pointer identity", "with_exposed_provenance"),
    ("pointer identity", "NonNull"),
    ("pointer identity", "ptr_eq"),
    ("pointer identity", "ptr::"),
    ("pointer identity", concat!("{:", "p")),
    // `TypeId`/`type_name` hand the compiler's own naming to the program:
    // both differ across toolchains, so a hash or log line built on them is
    // not the same computation an honest rebuild produced. `offset_of`,
    // `addr_of`, `size_of`/`align_of` (and their `_val` forms) leak layout —
    // which rustc is free to change between versions for the default repr.
    ("compiler identity", "TypeId"),
    ("compiler identity", "type_name"),
    ("layout leak", "offset_of"),
    ("layout leak", "addr_of"),
    ("layout leak", "size_of"),
    ("layout leak", "align_of"),
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

/// `use` statements flattened so `use a::{b, c::{d}}` also appears as the
/// separate paths `a::b` and `a::c::d`. A brace (or brace + `as`) import is
/// how a banned `a::b` token disappears from the text — `use std::env::{var}`
/// compiles `env::var` without ever spelling it (verified by injection).
/// `x as y` keeps the real path: the alias is the evasion, not the path.
fn flattened_use_paths(code: &str) -> Vec<String> {
    fn split_top_level_commas(items: &str) -> Vec<&str> {
        let mut out = Vec::new();
        let mut depth = 0usize;
        let mut start = 0usize;
        for (i, c) in items.char_indices() {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                ',' if depth == 0 => {
                    out.push(&items[start..i]);
                    start = i + 1;
                }
                _ => {}
            }
        }
        out.push(&items[start..]);
        out
    }
    fn expand(prefix: &str, items: &str, out: &mut Vec<String>) {
        for item in split_top_level_commas(items) {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            if let Some(brace) = item.find("::{") {
                let inner = &item[brace + 3..item.len().saturating_sub(1)];
                expand(&format!("{prefix}::{}", &item[..brace]), inner, out);
            } else if let Some(inner) = item.strip_prefix('{') {
                expand(prefix, inner.strip_suffix('}').unwrap_or(inner), out);
            } else {
                // `a::b as c` keeps the whole spelling — `a::b` needles still
                // match, and an `a::b as `-needle can see the alias import.
                let (base, alias) = match item.split_once(" as ") {
                    Some((b, a)) => (b.trim(), Some(a.trim())),
                    None => (item, None),
                };
                match base {
                    "self" => out.push(match alias {
                        Some(a) => format!("{prefix} as {a}"),
                        None => prefix.to_string(),
                    }),
                    "*" | "" => {}
                    b => out.push(format!(
                        "{prefix}::{b}{}",
                        alias.map_or(String::new(), |a| format!(" as {a}"))
                    )),
                }
            }
        }
    }
    let mut flat = Vec::new();
    let mut i = 0usize;
    while let Some(p) = code[i..].find("use ") {
        let start = i + p;
        // `use` must start a statement: preceded only by non-ident text —
        // `reuse`/`misuse` or `x::use` (not legal Rust) must not count. The
        // previous byte is ASCII here: a multi-byte char ends in bytes that
        // are never alphanumeric, which is exactly the boundary we want.
        if start > 0 && {
            let p = code.as_bytes()[start - 1];
            p.is_ascii_alphanumeric() || p == b'_'
        } {
            i = start + 4;
            continue;
        }
        let end = match code[start..].find(';') {
            Some(e) => start + e,
            // A `use` without `;` is not a statement — doc prose can carry
            // the word; skip the token itself and keep scanning for a real
            // terminated `use` later in the file.
            None => {
                i = start + 4;
                continue;
            }
        };
        let stmt = code[start + 4..end].trim();
        // `use a::b::{c}` — expand from the crate root (a leading `::`? no:
        // statements are `use path;` where path may itself start with braces
        // or bare names like `crate::x`).
        if let Some(brace) = stmt.find("::{") {
            let head = stmt[..brace].to_string();
            let inner = stmt[brace + 3..]
                .strip_suffix('}')
                .unwrap_or(&stmt[brace + 3..]);
            expand(&head, inner, &mut flat);
        }
        flat.push(stmt.to_string());
        i = end + 1;
    }
    flat
}

/// Library sources, keyed by path relative to `src/`. `src/bin/` is excluded:
/// those are CLI binaries, and reading argv or the environment is their job.
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
    let sources = library_sources(&kit_src());
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
    for (file, code) in library_sources(&kit_src()) {
        // Brace and alias imports rewrite the path the needles spell —
        // `use std::{env as e}` compiles `std::env` without writing it.
        // Scan the flattened paths alongside the literal text.
        let code = format!("{code}\n{}", flattened_use_paths(&code).join("\n"));
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
