//! `bundle_verify` exercised end-to-end against the shipped fixture.
//!
//! The unit tests inside the module cover the report shapes; this file pins
//! the behaviour a CI job actually depends on: the shipped `dungeon.game`
//! verifies deterministically, and its soak final hash is a regression
//! value — any change to the parser, the loader, the component stores, the
//! seeded RNG or the soak itself that alters the run flips exactly this
//! test, which is the moment to decide whether the change was intended.

use izanagi_kit::bundle_verify::{verify_bundle, verify_level, DEFAULT_TICKS};
use izanagi_kit::{is_loadable, parse, validate};

/// The soak final hash of the fixture's `room` level at [`DEFAULT_TICKS`].
/// Regenerate only as a deliberate act, the way `det_hash_golden` treats
/// its table: run the check, confirm the new value is the change you meant
/// to make, and update this constant in the same commit.
const DUNGEON_ROOM_FINAL_HASH: u64 = 0x8bea_c1d1_d6e1_ee8e;

fn dungeon() -> izanagi_kit::content::Content {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/dungeon.game");
    let source = std::fs::read_to_string(path).expect("the shipped fixture must be readable");
    let (content, parse_diags) = parse(&source);
    let validate_diags = validate(&content);
    assert!(
        is_loadable(&parse_diags, &validate_diags),
        "the shipped fixture must be loadable"
    );
    content
}

#[test]
fn the_shipped_dungeon_verifies_deterministically() {
    let content = dungeon();
    let report = verify_bundle(&content, DEFAULT_TICKS).unwrap();
    assert_eq!(report.levels.len(), 1);
    let room = &report.levels[0];
    assert_eq!(room.level, "room");
    assert_eq!(room.entities, 4);
    assert_eq!(room.ticks, DEFAULT_TICKS);
    assert!(room.is_deterministic(), "{room}");
    assert!(report.is_deterministic());
}

#[test]
fn the_dungeon_soak_final_hash_is_pinned() {
    let content = dungeon();
    let room = verify_level(&content, "room", DEFAULT_TICKS).unwrap();
    assert_eq!(
        room.final_hash, DUNGEON_ROOM_FINAL_HASH,
        "the soak run over the shipped fixture changed — if that was \
         intended, update DUNGEON_ROOM_FINAL_HASH in the same commit"
    );
    // And the pin is not vacuous: a shorter run is a different run.
    let shorter = verify_level(&content, "room", 16).unwrap();
    assert_ne!(shorter.final_hash, room.final_hash);
    assert_eq!(shorter.initial_hash, room.initial_hash);
}

#[test]
fn verify_level_rejects_an_unknown_level() {
    let content = dungeon();
    assert!(verify_level(&content, "nope", 8).is_err());
}
