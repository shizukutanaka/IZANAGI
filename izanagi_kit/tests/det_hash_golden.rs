//! Golden-value regression guard for the `DetHash` **wire format**.
//!
//! ## Why this exists
//!
//! `izanagi_kit`'s headline guarantee is *bit-identical* state hashing across
//! platforms and builds — the foundation of replay, lockstep netcode, and
//! save-file checksums. `tests/determinism.rs` pins one end-to-end simulation,
//! but that sim only exercises a handful of types (`EntityAllocator`,
//! `SparseSet`, `Fixed`, `SplitMix64`). Most other `DetHash`
//! implementations are pinned nowhere — the exact set is enumerated and kept
//! current by `UNPINNED_DET_HASH` in `tests/global_invariants_hold.rs`, which
//! fails if an impl is neither pinned here nor declared there.
//!
//! The per-module unit tests only assert `hash(x) == hash(x)` (self-consistency)
//! and `hash(x) != hash(y)` (discrimination). **Both survive a wire-format
//! change** — if a refactor reorders fields in a `det_hash` impl, or swaps a
//! `write_u32` for a `write_u64`, both sides of those assertions move together
//! and the tests still pass. The breakage is invisible until an *old* replay or
//! save (hashed by a previous build) is loaded against the new format.
//!
//! This file converts that "stability by discipline" into a mechanical
//! tripwire: it constructs a fixed, representative instance of each public
//! `DetHash` type and pins its exact `hash_state` value. Any change to a
//! wire format flips exactly one line here, forcing a deliberate decision
//! ("did we mean to break replay compatibility?") instead of a silent
//! regression.
//!
//! ## Regenerating after an intentional format change
//!
//! Run `cargo test --test det_hash_golden print_golden -- --ignored --nocapture`,
//! copy the printed table into `EXPECTED`, and document the break in
//! `CHANGELOG.md`. Treat any *unexpected* diff as a determinism regression.

use izanagi_kit::{
    ability::{Ability, AbilitySet},
    behavior::{BehaviorNode, BehaviorTree},
    generate_dungeon, hash_state, Aabb, BarWidget, Camera, Connector, Cooldown, DamageType, Dice,
    Dungeon, EntityAllocator, Fixed, GenParams, HFsm, HudPanel, MsgLog, MultiMap, Relations,
    ResistanceProfile, Screen, SplitMix64, StatLine, Stats, TimerQueue, Vec2, Vec3,
};

/// Build the canonical (name, golden-hash) table. Each entry constructs a
/// fixed instance whose `hash_state` must never change without a deliberate,
/// documented format break. Keep names stable and unique — they are the diff
/// anchors.
fn cases() -> Vec<(&'static str, u64)> {
    // A deterministic entity (index 0, generation 0).
    let mut alloc = EntityAllocator::new();
    let e0 = alloc.allocate();
    let e1 = alloc.allocate();

    // Relations: e1 parented to e0.
    let mut rel = Relations::new();
    rel.attach(e1, e0);

    // Message log with two entries.
    let mut log = MsgLog::new(8);
    log.push("hello");
    log.push("world");

    // Hierarchical FSM over plain integer states/events.
    let hfsm: HFsm<u32, u8> = HFsm::new(1)
        .with_parent(2, 1)
        .on(1, 10u8, 2)
        .on_any(20u8, 3);

    // Ability set: one keyed ability with an integer effect payload.
    let abilities: AbilitySet<u32, u32> =
        AbilitySet::new().with(1, Ability::new("Firebolt", 5, 3, 6, 42u32));

    // Behavior tree: a sequence of two action leaves.
    let bt: BehaviorTree<u32> = BehaviorTree::new(BehaviorNode::sequence(vec![
        BehaviorNode::action(1),
        BehaviorNode::action(2),
    ]));

    // Timer queue: one one-shot and one recurring entry. Pins the `period`
    // field whose omission was the Round 10 bug — a re-omission flips this.
    let mut timers: TimerQueue<u32> = TimerQueue::new();
    timers.schedule(5, 99);
    timers.schedule_repeat(5, 3, 99);

    // A small fixed-seed dungeon. Pins the `rooms` field (Round 11) plus the
    // tile bitmap; a re-omission of rooms — or a generation/wire change — flips it.
    let dungeon: Dungeon = {
        let mut rng = SplitMix64::new(0xD17A9E);
        generate_dungeon(24, 16, &mut rng, GenParams::default())
    };

    // A two-floor multi-map with one connector. Pins the `floors` field
    // (Round 12) alongside current_floor and connectors.
    let multimap: MultiMap = {
        let mut a = SplitMix64::new(1);
        let mut b = SplitMix64::new(2);
        let f0 = generate_dungeon(20, 12, &mut a, GenParams::default());
        let f1 = generate_dungeon(20, 12, &mut b, GenParams::default());
        let mut m = MultiMap::new(vec![f0, f1], 0);
        m.add_connector(Connector {
            from_floor: 0,
            from_x: 3,
            from_y: 3,
            to_floor: 1,
            to_x: 5,
            to_y: 5,
        });
        m
    };

    vec![
        ("Fixed::from_int(3)", hash_state(&Fixed::from_int(3))),
        ("Entity(0,0)", hash_state(&e0)),
        (
            "Vec2(2,-5)",
            hash_state(&Vec2::new(Fixed::from_int(2), Fixed::from_int(-5))),
        ),
        (
            "Vec3(1,2,3)",
            hash_state(&Vec3::new(
                Fixed::from_int(1),
                Fixed::from_int(2),
                Fixed::from_int(3),
            )),
        ),
        ("Aabb(1,2,3,4)", hash_state(&Aabb::new(1, 2, 3, 4))),
        (
            "Camera(5,5,16,16,64,64)",
            hash_state(&Camera::new(5, 5, 16, 16, 64, 64)),
        ),
        ("Cooldown(5)", hash_state(&Cooldown::new(5))),
        ("Dice(3d6+1)", hash_state(&Dice::new(3, 6, 1))),
        ("Stats(20,5,2)", hash_state(&Stats::new(20, 5, 2))),
        ("DamageType::Fire", hash_state(&DamageType::Fire)),
        (
            "ResistanceProfile{Fire:50}",
            hash_state(&ResistanceProfile::new().with(DamageType::Fire, 50)),
        ),
        ("Screen(2x2)", hash_state(&Screen::new(2, 2))),
        ("BarWidget(7,10,20)", hash_state(&BarWidget::new(7, 10, 20))),
        ("StatLine(HP,42)", hash_state(&StatLine::new("HP", 42))),
        (
            "HudPanel(0,0,10,5)",
            hash_state(&HudPanel::new(0, 0, 10, 5)),
        ),
        ("Relations{e1->e0}", hash_state(&rel)),
        ("MsgLog[hello,world]", hash_state(&log)),
        ("HFsm<u32,u8>", hash_state(&hfsm)),
        ("AbilitySet<u32,u32>", hash_state(&abilities)),
        ("BehaviorTree<u32>", hash_state(&bt)),
        ("TimerQueue<u32>[5;5r3]", hash_state(&timers)),
        ("Dungeon(24x16,seed)", hash_state(&dungeon)),
        ("MultiMap(2floors,1conn)", hash_state(&multimap)),
    ]
}

/// Pinned golden hashes. A diff in any value is a `DetHash` wire-format change.
/// Regenerate via the `print_golden` helper (see module docs) only for a
/// *deliberate*, changelog-documented break.
const EXPECTED: &[(&str, u64)] = &[
    ("Fixed::from_int(3)", 0x4d28dc7f9dd0f71e),
    ("Entity(0,0)", 0xa8c7f832281a39c5),
    ("Vec2(2,-5)", 0x62dee59804c46909),
    ("Vec3(1,2,3)", 0xf4ebf23c75a0eb95),
    ("Aabb(1,2,3,4)", 0x84c39a079fc08121),
    ("Camera(5,5,16,16,64,64)", 0x777750cf734fcf85),
    ("Cooldown(5)", 0x2d401a55eec16520),
    ("Dice(3d6+1)", 0x2fc5364c8c789d11),
    ("Stats(20,5,2)", 0x0a5ee284d3d7fbc2),
    ("DamageType::Fire", 0xaf63bc4c8601b62c),
    ("ResistanceProfile{Fire:50}", 0x25b9589630c86467),
    ("Screen(2x2)", 0x92c965a7dd27d405),
    ("BarWidget(7,10,20)", 0xae1340d60a74329c),
    ("StatLine(HP,42)", 0xb5df64faa084432d),
    ("HudPanel(0,0,10,5)", 0x9e0689ea1f4b9ada),
    ("Relations{e1->e0}", 0x5b5754e32028a8a5),
    ("MsgLog[hello,world]", 0x612662cfb2655e8d),
    ("HFsm<u32,u8>", 0x870c3270a024ab05),
    ("AbilitySet<u32,u32>", 0x6be775165615ef30),
    ("BehaviorTree<u32>", 0x733cfecebb0bc160),
    ("TimerQueue<u32>[5;5r3]", 0x9e3d87f791d59425),
    ("Dungeon(24x16,seed)", 0xe31ab41e7035e685),
    ("MultiMap(2floors,1conn)", 0xa84ad2b8abb52eb8),
];

#[test]
fn test_det_hash_wire_format_pinned() {
    let actual = cases();
    assert_eq!(
        actual.len(),
        EXPECTED.len(),
        "case count changed — update EXPECTED (regenerate with the print_golden helper)"
    );
    for ((name_a, hash_a), (name_e, hash_e)) in actual.iter().zip(EXPECTED.iter()) {
        assert_eq!(name_a, name_e, "case ordering/name drifted");
        assert_eq!(
            hash_a, hash_e,
            "DetHash wire format changed for `{name_a}`: \
             0x{hash_e:016x} (pinned) != 0x{hash_a:016x} (now). \
             If intentional, regenerate EXPECTED and document the replay/save break."
        );
    }
}

#[test]
fn test_golden_names_unique() {
    // Names are the diff anchors; duplicates would mask a regression.
    let mut names: Vec<&str> = cases().iter().map(|(n, _)| *n).collect();
    let total = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), total, "duplicate golden case name");
}

#[test]
fn test_golden_hashes_discriminate() {
    // Distinct fixtures should (overwhelmingly) hash distinctly; a collision
    // here would weaken the tripwire's resolution.
    let hashes: Vec<u64> = cases().iter().map(|(_, h)| *h).collect();
    let total = hashes.len();
    let mut uniq = hashes.clone();
    uniq.sort_unstable();
    uniq.dedup();
    assert_eq!(uniq.len(), total, "two golden fixtures collided");
}

#[test]
fn every_field_reaches_the_hash() {
    // A golden pin binds ONE fixture — an impl that silently stopped reading
    // a field the fixture never varies would stay green forever. The
    // complementary contract: perturbing each field alone must move the
    // hash. (FNV-1a avalanches single-bit deltas in distinct inputs
    // deterministically; these specific values cannot collide.)
    let f = |n: i32| Fixed::from_int(n);

    let base = Vec2::new(f(2), f(-5));
    assert_ne!(
        hash_state(&base),
        hash_state(&Vec2::new(f(3), f(-5))),
        "Vec2.x"
    );
    assert_ne!(
        hash_state(&base),
        hash_state(&Vec2::new(f(2), f(-4))),
        "Vec2.y"
    );

    let base = Vec3::new(f(1), f(2), f(3));
    assert_ne!(
        hash_state(&base),
        hash_state(&Vec3::new(f(9), f(2), f(3))),
        "Vec3.x"
    );
    assert_ne!(
        hash_state(&base),
        hash_state(&Vec3::new(f(1), f(9), f(3))),
        "Vec3.y"
    );
    assert_ne!(
        hash_state(&base),
        hash_state(&Vec3::new(f(1), f(2), f(9))),
        "Vec3.z"
    );

    let base = Aabb::new(1, 2, 3, 4);
    for (name, other) in [
        ("Aabb.x", Aabb::new(9, 2, 3, 4)),
        ("Aabb.y", Aabb::new(1, 9, 3, 4)),
        ("Aabb.w", Aabb::new(1, 2, 9, 4)),
        ("Aabb.h", Aabb::new(1, 2, 3, 9)),
    ] {
        assert_ne!(hash_state(&base), hash_state(&other), "{name}");
    }

    let base = Dice::new(3, 6, 1);
    for (name, other) in [
        ("Dice.count", Dice::new(4, 6, 1)),
        ("Dice.sides", Dice::new(3, 8, 1)),
        ("Dice.modifier", Dice::new(3, 6, -2)),
    ] {
        assert_ne!(hash_state(&base), hash_state(&other), "{name}");
    }

    // Camera fields are pub; vary each on one instance.
    let base = Camera {
        top_left_x: 5,
        top_left_y: 6,
        screen_w: 16,
        screen_h: 16,
    };
    for (name, other) in [
        (
            "Camera.top_left_x",
            Camera {
                top_left_x: 7,
                ..base
            },
        ),
        (
            "Camera.top_left_y",
            Camera {
                top_left_y: 8,
                ..base
            },
        ),
        (
            "Camera.screen_w",
            Camera {
                screen_w: 24,
                ..base
            },
        ),
        (
            "Camera.screen_h",
            Camera {
                screen_h: 24,
                ..base
            },
        ),
    ] {
        assert_ne!(hash_state(&base), hash_state(&other), "{name}");
    }

    // Entity { index, generation }: index via two allocations, generation via
    // free + realloc of the same slot.
    let mut alloc = EntityAllocator::new();
    let e0 = alloc.allocate();
    let e1 = alloc.allocate();
    assert_ne!(hash_state(&e0), hash_state(&e1), "Entity.index");
    alloc.free(e0);
    let e0_recycled = alloc.allocate();
    assert_eq!(e0_recycled.index(), e0.index());
    assert_ne!(
        hash_state(&e0),
        hash_state(&e0_recycled),
        "Entity.generation"
    );

    // Single-field types: the pin covers one value; here the value must move.
    assert_ne!(
        hash_state(&Cooldown::new(5)),
        hash_state(&Cooldown::new(6)),
        "Cooldown.remaining"
    );
    assert_ne!(hash_state(&f(3)), hash_state(&f(4)), "Fixed.raw");
}

/// Prints the current (name, hash) table for pasting into `EXPECTED`.
/// Ignored by default — run explicitly only when regenerating after a
/// deliberate format change.
#[test]
#[ignore]
fn print_golden() {
    println!("const EXPECTED: &[(&str, u64)] = &[");
    for (name, hash) in cases() {
        println!("    ({name:?}, 0x{hash:016x}),");
    }
    println!("];");
}
