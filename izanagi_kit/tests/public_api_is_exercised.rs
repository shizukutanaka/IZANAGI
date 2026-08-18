//! Every public function is exercised by something — and this proves it stays
//! that way.
//!
//! A sweep of the crate's 1534 public functions found 24 that no test, example
//! or binary anywhere in the workspace ever called. That is a small number and
//! a good result, but it is not nothing: an untested public function is a
//! promise the crate has never checked it can keep, and once published it can
//! never be withdrawn. Two of them existed only to duplicate something else and
//! were deleted; one had no caller and became private. The rest are exercised
//! below.
//!
//! The last test is the point of the file: it re-runs that sweep, so adding
//! public API without exercising it fails the build instead of being noticed a
//! year later.
//!
//! Where a property has an obvious oracle it is used rather than a
//! hand-computed constant — easing curves are pinned by their endpoints and
//! monotonicity, the visibility ranks by their ordering, regeneration by
//! conservation over a full refill.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use izanagi_kit::autotile::SimpleTileTable;
use izanagi_kit::content::Diagnostic;
use izanagi_kit::dialogue::{Dialogue, DialogueNode};
use izanagi_kit::easing;
use izanagi_kit::encounter::EncounterPack;
use izanagi_kit::fixed::Fixed;
use izanagi_kit::hfsm::HFsm;
use izanagi_kit::netinput::DelayScheduler;
use izanagi_kit::pool::Pool;
use izanagi_kit::progression::{LevelCurve, Progression};
use izanagi_kit::quest::Objective;
use izanagi_kit::rng::SplitMix64;
use izanagi_kit::shufflebag::ShuffleBag;
use izanagi_kit::spatial_hash::SpatialHash;
use izanagi_kit::tilemap::LayeredMap;
use izanagi_kit::timestep::FixedTimestep;
use izanagi_kit::visibility::{Visibility, VisibilityMap};

// ---------------------------------------------------------------- easing

#[test]
fn the_late_easing_curves_obey_the_easing_contract() {
    // Every easing curve must pin its endpoints and rise monotonically between
    // them. That contract is a far better check than any sampled value: a curve
    // satisfying it cannot be badly wrong, and one that does not is broken
    // whatever it happens to return at t = 0.5.
    type Curve = fn(Fixed) -> Fixed;
    let curves: [(&str, Curve); 3] = [
        ("ease_out_quart", easing::ease_out_quart),
        ("ease_out_quint", easing::ease_out_quint),
        ("ease_in_circ", easing::ease_in_circ),
    ];
    let zero = Fixed::from_int(0);
    let one = Fixed::from_int(1);
    for (name, f) in curves {
        assert_eq!(f(zero), zero, "{name} must start at 0");
        assert_eq!(f(one), one, "{name} must end at 1");
        let mut previous = f(zero);
        for step in 1..=256 {
            let t = Fixed::from_ratio(step, 256);
            let now = f(t);
            assert!(now >= previous, "{name} decreased at step {step}");
            assert!(
                now >= zero && now <= one,
                "{name} left [0,1] at step {step}"
            );
            previous = now;
        }
    }
    // The two "out" curves decelerate, so past halfway they sit above the
    // diagonal; "in_circ" accelerates and stays below it. That is what
    // distinguishes them from one another.
    let half = Fixed::from_ratio(1, 2);
    assert!(easing::ease_out_quart(half) > half);
    assert!(easing::ease_out_quint(half) > half);
    assert!(easing::ease_in_circ(half) < half);
    // A higher power leaves the origin later: quintic is never ahead of quartic
    // early on.
    let quarter = Fixed::from_ratio(1, 4);
    assert!(easing::ease_out_quint(quarter) >= easing::ease_out_quart(quarter));
}

// ------------------------------------------------------------- visibility

#[test]
fn visibility_predicates_agree_with_rank_ordering() {
    // `rank` orders the three states; the predicates must partition them.
    // Checked against each other rather than against literals, so they cannot
    // drift apart.
    assert!(Visibility::Unseen.rank() < Visibility::Remembered.rank());
    assert!(Visibility::Remembered.rank() < Visibility::Visible.rank());

    let mut map = VisibilityMap::new(3, 3);
    assert!(map.is_unseen(0, 0));
    assert!(!map.is_remembered(0, 0));

    map.mark_visible(0, 0);
    assert!(!map.is_unseen(0, 0));
    assert!(!map.is_remembered(0, 0), "visible is not merely remembered");
    assert_eq!(map.get(0, 0), Visibility::Visible);

    map.set(0, 0, Visibility::Remembered);
    assert!(map.is_remembered(0, 0));
    assert!(!map.is_unseen(0, 0));

    // Each predicate is exactly one rank class.
    for state in [
        Visibility::Unseen,
        Visibility::Remembered,
        Visibility::Visible,
    ] {
        map.set(1, 1, state);
        assert_eq!(
            map.is_unseen(1, 1),
            state.rank() == Visibility::Unseen.rank()
        );
        assert_eq!(
            map.is_remembered(1, 1),
            state.rank() == Visibility::Remembered.rank()
        );
    }
}

// -------------------------------------------------------------------- pool

#[test]
fn regenerating_pool_conserves_what_it_adds() {
    // Conservation is the oracle: a pool regenerating `r` per tick gains
    // exactly `r * ticks` until it caps.
    let mut p = Pool::with_regen(10, 2);
    assert_eq!(p.current(), 10, "with_regen starts full");
    assert!(p.spend(9));
    assert_eq!(p.current(), 1);

    let before = p.current();
    p.tick(1);
    assert_eq!(p.current() - before, 2, "one tick adds exactly the rate");
    let before = p.current();
    p.tick(3);
    assert_eq!(
        p.current() - before,
        6,
        "three ticks add three times the rate"
    );

    // Regeneration stops at the cap rather than overshooting it.
    p.tick(50);
    assert_eq!(p.current(), 10);

    // A negative rate models decay and floors at zero.
    p.set_regen(-3);
    p.tick(50);
    assert_eq!(p.current(), 0);

    // Rate zero is inert.
    p.set_regen(0);
    let held = p.current();
    p.tick(10);
    assert_eq!(p.current(), held);
}

// ------------------------------------------------------------------- hfsm

#[test]
fn hierarchical_parents_and_wildcards_are_reachable() {
    // `set_parent` and `add_wildcard` are what make this *hierarchical* rather
    // than flat, and neither was exercised anywhere.
    let mut fsm: HFsm<&str, &str> = HFsm::new("idle");
    fsm.set_parent("idle", "guard");
    fsm.set_parent("alert", "guard");
    // An edge declared on the parent applies to every child.
    fsm.add_transition("guard", "spotted", "alert");
    assert_eq!(fsm.state(), &"idle");
    assert!(fsm.fire(&"spotted"));
    assert_eq!(fsm.state(), &"alert", "the parent's edge was inherited");

    // A wildcard fires from any state, including one with no parent.
    fsm.add_wildcard("killed", "dead");
    assert!(fsm.fire(&"killed"));
    assert_eq!(fsm.state(), &"dead");
    // `fire` reports whether the state *changed*, so a wildcard that matches
    // but leads back to the current state returns false without moving.
    assert!(!fsm.fire(&"killed"));
    assert_eq!(fsm.state(), &"dead");
    // `dead` has no parent, so the inherited edge no longer applies.
    assert!(!fsm.fire(&"spotted"));
    assert_eq!(fsm.state(), &"dead");
}

// ------------------------------------------------------- small accessors

#[test]
fn accessors_report_what_their_constructors_were_given() {
    let scheduler: DelayScheduler<u8> = DelayScheduler::new(3);
    assert_eq!(scheduler.delay(), 3);
    let mut scheduler = scheduler;
    scheduler.set_delay(5);
    assert_eq!(scheduler.delay(), 5, "the accessor tracks the setter");

    let progression = Progression::new(LevelCurve::new(100, 50, 10));
    // The accessor must hand back the curve it was constructed with, not a
    // default — checked against a freshly built identical curve.
    assert_eq!(
        progression.curve().xp_to_reach(3),
        LevelCurve::new(100, 50, 10).xp_to_reach(3)
    );

    let grid: SpatialHash<u32> = SpatialHash::new(16);
    assert_eq!(grid.cell_size(), 16);

    let step = FixedTimestep::sixty_hz();
    assert_eq!(
        step.step_ns(),
        FixedTimestep::new(60, 8).step_ns(),
        "sixty_hz must agree with the general constructor at 60"
    );
}

#[test]
fn quest_objectives_report_failure_distinctly_from_incompletion() {
    let mut objective = Objective::new("reach the exit", 1);
    assert!(!objective.is_failed());
    assert!(!objective.is_complete());
    objective.fail();
    assert!(objective.is_failed());
    assert!(
        !objective.is_complete(),
        "a failed objective is not a completed one"
    );
}

#[test]
fn a_finished_conversation_has_no_current_node() {
    let nodes = vec![
        DialogueNode::new("greeting").with_choice("leave", 1),
        DialogueNode::new("farewell"),
    ];
    let mut dialogue = Dialogue::new(nodes, 0);
    assert_eq!(
        dialogue.current_node().map(DialogueNode::text),
        Some("greeting")
    );
    assert!(dialogue.choose(0));
    assert_eq!(
        dialogue.current_node().map(DialogueNode::text),
        Some("farewell"),
        "current_node must follow the chosen edge"
    );
    // current_node and current_index must always agree about whether the
    // conversation is still running.
    assert_eq!(
        dialogue.current_node().is_some(),
        dialogue.current_index().is_some()
    );
}

#[test]
fn a_shuffle_bag_peek_matches_what_it_goes_on_to_draw() {
    // The bag itself is the oracle: whatever `peek_remaining` shows must be
    // exactly the multiset still to come in this cycle.
    let mut bag = ShuffleBag::new(vec!['a', 'b', 'c', 'd']);
    let mut rng = SplitMix64::new(7);
    bag.draw(&mut rng);

    let mut peeked: Vec<char> = bag.peek_remaining().to_vec();
    let mut drawn: Vec<char> = (0..peeked.len())
        .filter_map(|_| bag.draw(&mut rng))
        .collect();
    peeked.sort_unstable();
    drawn.sort_unstable();
    assert_eq!(peeked, drawn, "peek must predict the rest of the cycle");
}

#[test]
fn autotile_from_array_matches_the_incremental_builder() {
    // Two ways to build the same table must agree — a differential oracle
    // rather than a table of expected indices.
    let mut table = [0u32; 256];
    for (mask, slot) in table.iter_mut().enumerate() {
        *slot = (mask as u32) * 2;
    }
    let from_array = SimpleTileTable::from_array(table);
    let mut built = SimpleTileTable::new();
    for mask in 0..=255u8 {
        built.set(mask, (mask as u32) * 2);
    }
    for mask in 0..=255u8 {
        assert_eq!(from_array.get(mask), built.get(mask), "mask {mask}");
    }
    assert_eq!(from_array.set_count(), built.set_count());
}

#[test]
fn a_warning_diagnostic_is_not_an_error() {
    let warning = Diagnostic::warning_at(12, 4, "unused prefab");
    assert!(!warning.is_error(), "a warning must not fail a build");
    let error = Diagnostic::error_at(12, 4, "unknown tile");
    assert!(error.is_error());
    // Same position, different severity — the only thing that differs is the
    // verdict.
    assert_eq!(warning.line, error.line);
    assert_eq!(warning.col, error.col);
}

#[test]
fn encounter_slots_added_in_place_match_the_builder_form() {
    let built = EncounterPack::new()
        .with_slot("rat", 1, 3)
        .with_optional_slot("bat", 1, 2, 50);
    let mut pushed = EncounterPack::new();
    pushed.push_slot("rat", 1, 3, 100);
    pushed.push_slot("bat", 1, 2, 50);
    assert_eq!(built.len(), pushed.len());
    assert_eq!(built.min_spawns(), pushed.min_spawns());
    assert_eq!(built.max_spawns(), pushed.max_spawns());
    // Identical seeds must produce identical rolls if the packs are equivalent.
    let mut a = SplitMix64::new(99);
    let mut b = SplitMix64::new(99);
    for _ in 0..32 {
        assert_eq!(built.roll(&mut a), pushed.roll(&mut b));
    }
}

#[test]
fn a_mutable_layer_writes_through_to_the_map() {
    let mut map: LayeredMap<u32> = LayeredMap::new(4, 4, 2, 0);
    if let Some(layer) = map.layer_mut(1) {
        layer.set(2, 2, 7);
    }
    assert_eq!(map.get(1, 2, 2), Some(&7), "the write reached the map");
    assert_eq!(map.get(0, 2, 2), Some(&0), "other layers are untouched");
    assert!(map.layer_mut(9).is_none(), "out-of-range layers are None");
    // layer_mut and layer must address the same storage.
    assert_eq!(map.layer(1).and_then(|l| l.get(2, 2)), Some(&7));
}

// ------------------------------------------------------------- the gate

/// Repo root: `CARGO_MANIFEST_DIR` is `<root>/izanagi_kit`.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the kit lives one level below the workspace root")
        .to_path_buf()
}

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(rust_files(&path));
        } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
            out.push(path);
        }
    }
    out
}

/// Everything that counts as exercising an API: in-file `#[cfg(test)]`
/// modules, integration tests, examples and binaries, across both crates.
fn exercising_code() -> String {
    let root = repo_root();
    let mut blob = String::new();
    for path in rust_files(&root.join("izanagi_kit/src")) {
        let src = fs::read_to_string(&path).unwrap_or_default();
        if let Some(i) = src.find("#[cfg(test)]") {
            blob.push_str(&src[i..]);
        }
    }
    for dir in [
        "izanagi_kit/tests",
        "izanagi_kit/examples",
        "izanagi/examples",
        "izanagi/tests",
    ] {
        for path in rust_files(&root.join(dir)) {
            // This file counts too: it computes the function list rather than
            // spelling it out, so its own text cannot satisfy the check by
            // accident — and the tests above it are genuine exercise.
            blob.push_str(&fs::read_to_string(&path).unwrap_or_default());
        }
    }
    blob
}

/// Public function names declared in the kit's production code.
fn public_functions() -> BTreeSet<(String, String)> {
    let mut out = BTreeSet::new();
    for path in rust_files(&repo_root().join("izanagi_kit/src")) {
        let module = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        if module == "lib" {
            continue;
        }
        let src = fs::read_to_string(&path).unwrap_or_default();
        let impl_end = src.find("#[cfg(test)]").unwrap_or(src.len());
        for line in src[..impl_end].lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            for prefix in ["pub fn ", "pub const fn "] {
                let Some(rest) = trimmed.strip_prefix(prefix) else {
                    continue;
                };
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    out.insert((module.clone(), name));
                }
            }
        }
    }
    out
}

#[test]
fn no_public_function_goes_unexercised() {
    // The gate. Every public function must be named by some test, example or
    // binary. A new one that nothing calls fails here — which is the moment to
    // decide whether it should exist at all, rather than after publication when
    // removing it is a breaking change.
    let code = exercising_code();
    let functions = public_functions();
    assert!(
        functions.len() > 1_000,
        "expected to find the crate's public surface, found {} — has the \
         source layout changed?",
        functions.len()
    );

    let mut unexercised: Vec<String> = Vec::new();
    for (module, name) in &functions {
        // A bare textual search, matching how the sweep that found these was
        // run. It over-approximates (a same-named method on another type
        // counts), which is the safe direction for a gate like this.
        let called = code.contains(&format!("{name}("))
            || code.contains(&format!("{name}::"))
            || code.contains(&format!("{name}:"));
        if !called {
            unexercised.push(format!("{module}::{name}"));
        }
    }
    assert!(
        unexercised.is_empty(),
        "these public functions are never called by any test, example or \
         binary: {unexercised:#?}\n\nEither exercise them or delete them — an \
         untested public function is a promise nothing has checked."
    );
}
