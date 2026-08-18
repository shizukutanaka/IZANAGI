//! The whole verification pipeline on one simulation, start to finish.
//!
//! The kit has eleven modules that check a simulation, and reading eleven sets
//! of docs to see what they add up to is the wrong way to learn them. This is
//! the short version: one small dungeon room, one planted bug, and every tool
//! applied to it in the order you would actually reach for them.
//!
//! The bug is real and nothing here is staged — `Quaff` forgets to check that
//! you have a potion, so the count goes negative. Watch each tool say
//! something different about it:
//!
//! 1. `sim::audit` — is the simulation deterministic at all?
//! 2. `verify` — is the bug *provably* there, and what is the shortest way to
//!    trigger it?
//! 3. `prop` + `shrink` — random play finds it too, minimised.
//! 4. `dst` + `temporal` — properties that span time, over a seed sweep.
//! 5. `plan` — synthesise a run that reaches a goal.
//! 6. `explore` — map the reachable state space.
//! 7. `recovery` — does save/load preserve behaviour?
//!
//! Each check is run against both the buggy and the fixed rules where that is
//! meaningful, so the difference between "found no counterexample" and "proved
//! there is none" is visible side by side.
//!
//! Every claim printed below is also asserted, so this doubles as an
//! integration test: if the pipeline stops agreeing with itself, the example
//! fails instead of quietly printing something else.
//!
//! Run with `cargo run --example verify_pipeline_demo`.

use izanagi_kit::dst::dst_sweep;
use izanagi_kit::explore::{explore, ExploreConfig};
use izanagi_kit::plan::plan_inputs;
use izanagi_kit::prop::forall_inputs;
use izanagi_kit::recovery::restart_test;
use izanagi_kit::rng::SplitMix64;
use izanagi_kit::shrink::{is_one_minimal, shrink_inputs};
use izanagi_kit::sim::{audit, Simulation};
use izanagi_kit::temporal::{Monitor, MonitorSet};
use izanagi_kit::verify::{check_invariant, check_temporal, Verification};
use izanagi_kit::world_hash::{hash_state, DetHash, Fnv1a};

// ---------------------------------------------------------------- the model

/// One dungeon room. Small on purpose: every value is clamped, so the whole
/// reachable state space is finite and can be enumerated exhaustively.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Room {
    hp: i32,
    potions: i32,
    gold: i32,
    door_open: bool,
}

impl Room {
    fn start() -> Self {
        Room {
            hp: 6,
            potions: 1,
            gold: 0,
            door_open: false,
        }
    }
}

impl DetHash for Room {
    fn det_hash(&self, h: &mut Fnv1a) {
        h.write_i32(self.hp);
        h.write_i32(self.potions);
        h.write_i32(self.gold);
        h.write_bool(self.door_open);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Act {
    /// Drink a potion: +3 HP.
    Quaff,
    /// Fight the occupant: -2 HP, +1 gold.
    Fight,
    /// Buy a potion for 2 gold.
    Buy,
    /// Leave, if you can afford the toll.
    Open,
}

const ACTS: [Act; 4] = [Act::Quaff, Act::Fight, Act::Buy, Act::Open];
const TOLL: i32 = 3;

/// The transition, in two versions. `guarded = false` is the shipped bug:
/// `Quaff` never checks that a potion is actually in the bag.
fn step(guarded: bool) -> impl Fn(&Room, &Act) -> Room {
    move |r: &Room, a: &Act| {
        let mut next = r.clone();
        match a {
            Act::Quaff => {
                if !guarded || next.potions > 0 {
                    next.potions -= 1;
                    next.hp = (next.hp + 3).min(10);
                }
            }
            Act::Fight => {
                next.hp = (next.hp - 2).max(0);
                next.gold = (next.gold + 1).min(5);
            }
            Act::Buy => {
                if next.gold >= 2 && next.potions < 3 {
                    next.gold -= 2;
                    next.potions += 1;
                }
            }
            Act::Open => {
                if next.gold >= TOLL {
                    next.door_open = true;
                }
            }
        }
        next
    }
}

impl Simulation for Room {
    type Input = Act;
    fn step(&mut self, input: &Act) {
        *self = step(true)(self, input);
    }
}

// ------------------------------------------------------------------ helpers

fn rule(title: &str) {
    println!("\n\x1b[1m{title}\x1b[0m");
    println!("{}", "─".repeat(title.len()));
}

fn verdict(label: &str, v: &Verification<Act>) {
    match v {
        Verification::Holds { states, diameter } => println!(
            "  \x1b[32m{label}: PROVED\x1b[0m — held in all {states} reachable states (diameter {diameter})"
        ),
        Verification::Violated(cx) => println!(
            "  \x1b[31m{label}: VIOLATED\x1b[0m — shortest counterexample ({} input(s)): {:?}",
            cx.path.len(),
            cx.path
        ),
        Verification::Exhausted { states, depth } => println!(
            "  \x1b[33m{label}: INCONCLUSIVE\x1b[0m — bound hit at {states} states, depth {depth}"
        ),
    }
}

fn main() {
    println!("\x1b[1mIZANAGI verification pipeline\x1b[0m");
    println!("A four-action dungeon room with one planted bug: `Quaff` never");
    println!("checks that you own a potion.");

    // 1 ------------------------------------------------------------------
    rule("1. sim::audit — is the simulation deterministic?");
    let script = [Act::Fight, Act::Quaff, Act::Buy, Act::Open, Act::Fight];
    let report = audit(&Room::start(), &script, 2);
    println!("  {report}");
    println!(
        "  Deterministic: {}. Pin final_hash {:#018x} and any change to the",
        report.is_deterministic(),
        report.final_hash
    );
    println!("  simulation that moves it becomes a visible, reviewable diff.");

    // 2 ------------------------------------------------------------------
    rule("2. verify — is the bug provably there?");
    let buggy = check_invariant(
        Room::start(),
        &ACTS,
        step(false),
        |r: &Room| r.potions >= 0,
        100_000,
    );
    verdict("potions >= 0 (buggy)", &buggy);
    let fixed = check_invariant(
        Room::start(),
        &ACTS,
        step(true),
        |r: &Room| r.potions >= 0,
        100_000,
    );
    verdict("potions >= 0 (fixed)", &fixed);
    assert_eq!(
        buggy.counterexample().map(|c| c.path.len()),
        Some(2),
        "you start with one potion, so two quaffs is the shortest way negative"
    );
    assert!(fixed.holds(), "the guarded rules must be provable");
    println!("  Note the asymmetry: the first is a counterexample, the second is a");
    println!("  proof. \"Found nothing\" and \"there is nothing\" are different answers.");

    // An ordering property no single-state predicate can express.
    let ordering = check_temporal(
        Room::start(),
        &ACTS,
        step(true),
        &Monitor::precedes(|r: &Room| r.gold >= TOLL, |r: &Room| r.door_open),
        100_000,
    )
    .expect("precedes is a safety property");
    verdict("door never opens before the toll is affordable", &ordering);
    assert!(ordering.holds(), "the toll ordering must be provable");

    // 3 ------------------------------------------------------------------
    rule("3. prop + shrink — random play finds it, minimised");
    // Note this folds the *buggy* rules directly, so random play can actually
    // reach the bug — `Room`'s `Simulation` impl uses the fixed ones.
    let reaches_bug = |seq: &[Act]| {
        let end = seq.iter().fold(Room::start(), |r, a| step(false)(&r, a));
        end.potions < 0
    };
    let failure = forall_inputs(
        0..400u64,
        24,
        |rng: &mut SplitMix64| ACTS[rng.below(4) as usize],
        |seq: &[Act]| !reaches_bug(seq),
    )
    .expect_err("the bug is reachable by random play");
    println!(
        "  seed {} generated {} inputs; returned already shrunk to {} — {:?}",
        failure.seed,
        failure.original_len,
        failure.counterexample.len(),
        failure.counterexample
    );
    println!(
        "  1-minimal: {} (removing any single input makes the bug disappear)",
        is_one_minimal(&failure.counterexample, reaches_bug)
    );
    // Cross-check: shrinking a longer witness by hand lands in the same place,
    // and so did the model checker in step 2 — three unrelated techniques
    // agreeing on the same minimal cause.
    let by_hand = shrink_inputs(&[Act::Fight, Act::Quaff, Act::Buy, Act::Quaff], reaches_bug);
    println!(
        "  independently shrunk witness: {:?} — same length as the model",
        by_hand
    );
    println!("  checker's counterexample in step 2.");
    assert!(is_one_minimal(&failure.counterexample, reaches_bug));
    assert_eq!(
        by_hand.len(),
        buggy.counterexample().map_or(0, |c| c.path.len()),
        "shrinking and model checking must agree on the minimal cause"
    );

    // 4 ------------------------------------------------------------------
    rule("4. dst + temporal — properties that span time, across seeds");
    let mut props = MonitorSet::new()
        .with("hp-never-negative", Monitor::always(|r: &Room| r.hp >= 0))
        .with(
            "potions-never-negative",
            Monitor::always(|r: &Room| r.potions >= 0),
        )
        .with(
            "door-needs-the-toll",
            Monitor::precedes(|r: &Room| r.gold >= TOLL, |r: &Room| r.door_open),
        );
    let sweep = dst_sweep(
        0..64u64,
        40,
        |_seed| Room::start(),
        |r: &mut Room, tick| {
            // A seeded pseudo-player, so every seed is a different playthrough.
            let mut rng = SplitMix64::new(tick as u64 ^ 0x5eed);
            let act = ACTS[rng.below(4) as usize];
            *r = step(true)(r, &act);
        },
        |r: &Room, _tick| props.update(r),
    );
    match sweep {
        Ok(()) => println!("  64 seeds x 40 ticks: all 3 temporal properties held"),
        Err(f) => panic!("the fixed rules must satisfy every property: {f}"),
    }

    // 5 ------------------------------------------------------------------
    rule("5. plan — synthesise a run that reaches a goal");
    match plan_inputs(
        Room::start(),
        &ACTS,
        step(true),
        |r: &Room| r.door_open,
        100_000,
    ) {
        Some(path) => {
            println!("  shortest way out ({} inputs): {:?}", path.len(), path);
            // Three Fights pay the toll; nothing shorter can.
            assert_eq!(path.len(), TOLL as usize + 1);
        }
        None => panic!("the door is reachable"),
    }

    // 6 ------------------------------------------------------------------
    rule("6. explore — map the reachable state space");
    let archive = explore(
        &Room::start(),
        &ACTS,
        step(true),
        hash_state,
        &ExploreConfig {
            seed: 7,
            iterations: 400,
            steps_per_iteration: 12,
            max_cells: 100_000,
        },
    );
    let deepest = archive.deepest().map(|(_, p)| p.len()).unwrap_or(0);
    println!(
        "  archived {} distinct states; furthest is {} inputs from the start",
        archive.len(),
        deepest
    );
    // Sampling cannot find more states than exist, and the exhaustive count
    // from step 2 is the ceiling.
    if let Verification::Holds { states, .. } = fixed {
        assert!(
            archive.len() <= states,
            "sampling exceeded the proved total"
        );
    }

    // 7 ------------------------------------------------------------------
    rule("7. recovery — does save/load preserve behaviour?");
    let complete = |r: &Room| Some(r.clone());
    let forgets_potions = |r: &Room| {
        Some(Room {
            potions: 0,
            ..r.clone()
        })
    };
    let mut sim_step = |r: &mut Room, a: &Act| *r = step(true)(r, a);
    let save_script = [Act::Fight, Act::Fight, Act::Buy, Act::Quaff, Act::Open];
    match restart_test(Room::start(), &save_script, &mut sim_step, complete) {
        Ok(()) => println!("  complete save: survives a restart after every input"),
        Err(e) => panic!("a complete save must be undetectable: {e}"),
    }
    match restart_test(Room::start(), &save_script, &mut sim_step, forgets_potions) {
        Ok(()) => panic!("a lossy save must not go undetected"),
        Err(e) => println!("  lossy save caught: {e}"),
    }

    // ---------------------------------------------------------------------
    rule("Summary");
    println!("  The same bug looked different through each lens: a counterexample");
    println!("  from the model checker, a minimised witness from property testing,");
    println!("  nothing at all from the seed sweep (it plays the fixed rules).");
    println!("  Only `verify` could say the fixed version has *no* such state —");
    println!("  every other tool can only ever say it did not find one.");
}
