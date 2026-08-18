//! Where the engine stops being bit-identically replayable, checked by the
//! build.
//!
//! The engine renders and animates in `f32`; `izanagi_kit` simulates in
//! fixed-point and promises bit-identical replay. `examples/kit_bridge.rs`
//! shows the two composed, which raises the question this test answers: *which
//! parts of the engine may hold simulation state that has to replay?*
//!
//! Floating-point arithmetic rounds differently across x87 and SSE, with and
//! without FMA contraction, and across optimisation levels. A single `f32` in
//! the simulation path is enough to desync two machines running the same
//! build. So the boundary is not a matter of taste — it is the line between
//! state that can be replayed and state that cannot.
//!
//! `izanagi_kit` enforces its side mechanically (`tests/no_float_in_sim.rs`
//! rejects any float in the kit's production sources). The engine cannot do
//! that — rendering *needs* floats — so it does the next best thing: it names
//! exactly which modules are float-free and fails the build when that set
//! changes, so the claim in the engine's documentation can never quietly stop
//! being true.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

/// Engine modules whose production code contains no `f32` or `f64` at all.
///
/// State built only from these can take part in a bit-identical replay.
/// Float-freedom is *necessary*, not sufficient — the state must also be
/// hashed and stepped deterministically, which is what the kit's
/// `sim::Simulation` and `world_hash::DetHash` are for.
///
/// Changing this list is a determinism-relevant change. It should be made
/// deliberately, in the same commit that updates the engine's crate docs.
const FLOAT_FREE: &[&str] = &[
    "assets", "ecs", "error", "event", "log", "save", "scene", "state",
];

fn engine_src() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Production source of one module: everything before its `#[cfg(test)]`
/// marker, with line comments stripped. Mirrors the kit's `no_float_in_sim`
/// scanner, including its assumption of one trailing test module per file.
fn production_code(path: &PathBuf) -> String {
    let src = fs::read_to_string(path).expect("engine source file");
    let impl_end = src.find("#[cfg(test)]").unwrap_or(src.len());
    src[..impl_end]
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn contains_float(code: &str) -> bool {
    // Token-boundary match so `f32x4` or `nf64` would not count. The engine
    // uses no such identifiers today; the check is written this way so it
    // stays correct if it ever does.
    let bytes: Vec<char> = code.chars().collect();
    for needle in ["f32", "f64"] {
        let n: Vec<char> = needle.chars().collect();
        for i in 0..bytes.len().saturating_sub(n.len() - 1) {
            if bytes[i..i + n.len()] != n[..] {
                continue;
            }
            let before_ok = i == 0 || !(bytes[i - 1].is_alphanumeric() || bytes[i - 1] == '_');
            let after = i + n.len();
            let after_ok =
                after >= bytes.len() || !(bytes[after].is_alphanumeric() || bytes[after] == '_');
            if before_ok && after_ok {
                return true;
            }
        }
    }
    false
}

fn measured_float_free() -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for entry in fs::read_dir(engine_src()).expect("engine src directory") {
        let path = entry.expect("readable entry").path();
        if path.extension().map(|e| e != "rs").unwrap_or(true) {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("utf-8 module name")
            .to_string();
        if name == "lib" {
            continue; // the crate root re-exports; it is not a module boundary
        }
        if !contains_float(&production_code(&path)) {
            out.insert(name);
        }
    }
    out
}

#[test]
fn the_float_free_module_set_is_exactly_what_the_docs_claim() {
    let measured = measured_float_free();
    let declared: BTreeSet<String> = FLOAT_FREE.iter().map(|s| s.to_string()).collect();

    let newly_floating: Vec<&String> = declared.difference(&measured).collect();
    assert!(
        newly_floating.is_empty(),
        "these modules were float-free and no longer are: {newly_floating:?}. \
         That narrows what can hold replayable state — update FLOAT_FREE here \
         and the determinism-boundary section of src/lib.rs in this commit, \
         and make sure the float was actually intended."
    );

    let newly_clean: Vec<&String> = measured.difference(&declared).collect();
    assert!(
        newly_clean.is_empty(),
        "these modules are now float-free but are not listed: {newly_clean:?}. \
         Add them to FLOAT_FREE and to the boundary section of src/lib.rs."
    );
}

#[test]
fn the_crate_docs_name_every_float_free_module() {
    // The list above is the machine-checked truth; this makes sure the prose a
    // reader actually sees says the same thing.
    let lib = fs::read_to_string(engine_src().join("lib.rs")).expect("engine lib.rs");
    let boundary = lib
        .split("# Determinism boundary")
        .nth(1)
        .expect("src/lib.rs must document the determinism boundary");
    for module in FLOAT_FREE {
        assert!(
            boundary.contains(&format!("`{module}`")),
            "the determinism-boundary docs do not mention the float-free module `{module}`"
        );
    }
}

#[test]
fn the_rng_integer_core_is_float_free_even_though_the_module_is_not() {
    // `rng` carries floats, so the module-level classification excludes it —
    // but its integer half (`u64`, `u32`, `int_range`, `choose`) is exactly as
    // replay-safe as anything in FLOAT_FREE, and a reader who only saw the
    // module list would wrongly avoid it. This pins the split so the docs can
    // describe it honestly.
    let rng = production_code(&engine_src().join("rng.rs"));
    assert!(
        contains_float(&rng),
        "rng is expected to carry float helpers; if it no longer does, it \
         belongs in FLOAT_FREE"
    );
    for integer_api in [
        "pub fn u64(",
        "pub fn u32(",
        "pub fn int_range(",
        "pub fn choose",
    ] {
        assert!(
            rng.contains(integer_api),
            "the replay-safe integer half of Rng is missing `{integer_api}`"
        );
    }
    for float_api in ["pub fn f32(", "pub fn range(", "pub fn chance("] {
        assert!(
            rng.contains(float_api),
            "the documented float half of Rng is missing `{float_api}`"
        );
    }
}
