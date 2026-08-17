//! Crash-recovery testing: prove that saving, quitting and reloading is
//! indistinguishable from never having stopped.
//!
//! ## The gap this fills
//!
//! Deterministic Simulation Testing as practised on real systems is not just
//! "run the same seed twice". FoundationDB's simulator (Will Wilson, *Testing
//! Distributed Systems w/ Deterministic Simulation*, Strange Loop 2014) —
//! the direct ancestor of the [`dst`](crate::dst) module here — spends most of
//! its value **injecting faults**: killing machines, failing disks, and
//! restarting processes mid-run, then checking the system still behaves. The
//! kit's sweeps had no fault model at all: [`dst_sweep`](crate::dst::dst_sweep)
//! varies the seed, [`dst_swarm_sweep`](crate::dst::dst_swarm_sweep) varies the
//! action set, and neither ever interrupts the run.
//!
//! For a game the fault that matters most is the ordinary one: **the player
//! saves and quits, then loads and carries on.** If the save silently drops a
//! field, the loaded game is not the game that was saved — and the symptom
//! surfaces hours later as a desynced replay or an impossible state, nowhere
//! near the save that caused it. [`savefile`](crate::savefile) protects the
//! *container* (magic number, version, checksum); nothing checked that the
//! *payload* round-trips without loss of behaviour.
//!
//! ## Inject at every point, not at a plausible one
//!
//! [`restart_test`] injects a save/restore cycle after **every** input and
//! checks the run continues identically, which is the discipline Pillai et al.
//! used to find crash-consistency bugs in widely-deployed applications
//! (*All File Systems Are Not Created Equal: On the Complexity of Crafting
//! Crash-Consistent Applications*, OSDI 2014): a crash injected only at
//! "interesting" moments finds only the bugs you already suspected.
//!
//! It is the same shape as [`sync_test`](crate::rollback::sync_test), which
//! rolls back at every frame rather than at plausible ones — and it catches a
//! different fault. `sync_test` re-enters from an *in-memory snapshot*, so it
//! never exercises serialisation; `restart_test` re-enters from whatever
//! survived the round trip, which is the only way to find state that the save
//! format forgets.
//!
//! ## The three ways a restart can be wrong
//!
//! [`RestartFailure`] distinguishes them, and the distinction is the useful
//! part of the report:
//!
//! - [`Failed`](RestartFailure::Failed) — the round trip did not produce a
//!   state at all. A parse or version error; the save is unloadable.
//! - [`Lossy`](RestartFailure::Lossy) — the restored state's [`DetHash`]
//!   already differs. The save dropped something the hash covers, and the bug
//!   is squarely in the save format.
//! - [`Divergent`](RestartFailure::Divergent) — the restored state hashes
//!   *equal*, yet the simulation diverges later. This is the interesting one:
//!   the state carries something that affects the simulation but which the
//!   hash does not cover, so the save dropped it and nothing noticed. Reach for
//!   [`hash_covers`](crate::world_hash::hash_covers) to find the uncovered
//!   field — and note that a nondeterministic `step` produces the same symptom,
//!   which [`sync_test`](crate::rollback::sync_test) will tell apart.
//!
//! ```
//! use izanagi_kit::recovery::restart_test;
//! use izanagi_kit::world_hash::{DetHash, Fnv1a};
//!
//! #[derive(Clone)]
//! struct Save {
//!     gold: i32,
//!     streak: i32,
//! }
//! impl DetHash for Save {
//!     fn det_hash(&self, h: &mut Fnv1a) {
//!         h.write_i32(self.gold);
//!         h.write_i32(self.streak);
//!     }
//! }
//!
//! // A complete round trip survives a restart after every single input.
//! let ok = restart_test(
//!     Save { gold: 0, streak: 0 },
//!     &[3, 4, 5],
//!     |s: &mut Save, i: &i32| {
//!         s.streak += 1;
//!         s.gold += i * s.streak;
//!     },
//!     |s: &Save| Some(Save { gold: s.gold, streak: s.streak }),
//! );
//! assert!(ok.is_ok());
//!
//! // Forget one field and the test says exactly where it broke.
//! let bad = restart_test(
//!     Save { gold: 0, streak: 0 },
//!     &[3, 4, 5],
//!     |s: &mut Save, i: &i32| {
//!         s.streak += 1;
//!         s.gold += i * s.streak;
//!     },
//!     |s: &Save| Some(Save { gold: s.gold, streak: 0 }), // streak not saved
//! );
//! assert!(bad.is_err());
//! ```

use crate::sim::Simulation;
use crate::world_hash::{hash_state, DetHash};

/// How a save/restore cycle failed to be behaviour-preserving. See the module
/// docs for what each case implicates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestartFailure {
    /// The round trip produced no state — the save could not be loaded back.
    Failed {
        /// Number of inputs applied before the restart was injected.
        restart_tick: usize,
    },
    /// The restored state's hash differs immediately: the save format dropped
    /// something the hash covers.
    Lossy {
        /// Number of inputs applied before the restart was injected.
        restart_tick: usize,
        /// Hash of the state that was saved.
        expected: u64,
        /// Hash of the state that came back.
        actual: u64,
    },
    /// The restored state hashed equal but the simulation diverged afterwards:
    /// the save dropped state the hash does not cover (or `step` is not
    /// deterministic).
    Divergent {
        /// Number of inputs applied before the restart was injected.
        restart_tick: usize,
        /// Number of inputs applied when the divergence was detected.
        detected_tick: usize,
        /// Hash the uninterrupted run produced there.
        expected: u64,
        /// Hash the restarted run produced there.
        actual: u64,
    },
}

impl RestartFailure {
    /// Number of inputs applied before the restart that exposed the fault.
    pub fn restart_tick(&self) -> usize {
        match self {
            RestartFailure::Failed { restart_tick }
            | RestartFailure::Lossy { restart_tick, .. }
            | RestartFailure::Divergent { restart_tick, .. } => *restart_tick,
        }
    }
}

impl core::fmt::Display for RestartFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RestartFailure::Failed { restart_tick } => write!(
                f,
                "restart after {restart_tick} input(s): the saved state could \
                 not be loaded back"
            ),
            RestartFailure::Lossy {
                restart_tick,
                expected,
                actual,
            } => write!(
                f,
                "restart after {restart_tick} input(s): restored state differs \
                 immediately — saved {expected:#018x} != loaded {actual:#018x}; \
                 the save format drops a hashed field"
            ),
            RestartFailure::Divergent {
                restart_tick,
                detected_tick,
                expected,
                actual,
            } => write!(
                f,
                "restart after {restart_tick} input(s): restored state hashed \
                 equal but diverged at input {detected_tick} — {expected:#018x} \
                 != {actual:#018x}; the save drops state the hash does not cover"
            ),
        }
    }
}

/// Inject a save/restore cycle after every input and check the run continues
/// identically to the uninterrupted one. Returns the first
/// [`RestartFailure`], or `Ok(())` if every restart point was
/// behaviour-preserving.
///
/// `round_trip(&state) -> Option<S>` stands for "save this, then load it
/// back": returning `None` means the load failed. The restart is injected at
/// every position from *before the first input* through *after the last*, so a
/// run of `n` inputs is probed at `n + 1` points.
///
/// Cost is O(n²) simulation steps — every restart point replays the remainder
/// of the run — which is the same trade
/// [`sync_test`](crate::rollback::sync_test) makes and is why this belongs in a
/// test, not a frame loop.
pub fn restart_test<S, I, F, R>(
    initial: S,
    inputs: &[I],
    mut step: F,
    mut round_trip: R,
) -> Result<(), RestartFailure>
where
    S: DetHash + Clone,
    F: FnMut(&mut S, &I),
    R: FnMut(&S) -> Option<S>,
{
    // Pass 1 — the uninterrupted reference trace: one hash per input applied.
    let initial_hash = hash_state(&initial);
    let mut reference = Vec::with_capacity(inputs.len());
    {
        let mut state = initial.clone();
        for input in inputs {
            step(&mut state, input);
            reference.push(hash_state(&state));
        }
    }

    // Pass 2 — restart after k inputs, for every k, and replay the remainder.
    let mut state = initial.clone();
    for k in 0..=inputs.len() {
        let expected_here = if k == 0 {
            initial_hash
        } else {
            reference[k - 1]
        };

        let restored = match round_trip(&state) {
            Some(restored) => restored,
            None => return Err(RestartFailure::Failed { restart_tick: k }),
        };
        let restored_hash = hash_state(&restored);
        if restored_hash != expected_here {
            return Err(RestartFailure::Lossy {
                restart_tick: k,
                expected: expected_here,
                actual: restored_hash,
            });
        }

        // The hashes agree; keep simulating to catch state the hash misses.
        let mut probe = restored;
        for (offset, input) in inputs[k..].iter().enumerate() {
            step(&mut probe, input);
            let position = k + offset;
            let actual = hash_state(&probe);
            if actual != reference[position] {
                return Err(RestartFailure::Divergent {
                    restart_tick: k,
                    detected_tick: position + 1,
                    expected: reference[position],
                    actual,
                });
            }
        }

        if k < inputs.len() {
            step(&mut state, &inputs[k]);
        }
    }

    Ok(())
}

/// [`restart_test`] for a real byte-level save format: `write` serialises,
/// `read` parses back.
///
/// This is the shape an actual save file has, and the one worth testing —
/// `write`/`read` can be the same functions the game ships, wrapped around
/// [`save_bytes`](crate::savefile::save_bytes) and
/// [`load_bytes`](crate::savefile::load_bytes).
pub fn restart_test_bytes<S, I, F, W, R>(
    initial: S,
    inputs: &[I],
    step: F,
    write: W,
    read: R,
) -> Result<(), RestartFailure>
where
    S: DetHash + Clone,
    F: FnMut(&mut S, &I),
    W: Fn(&S) -> Vec<u8>,
    R: Fn(&[u8]) -> Option<S>,
{
    restart_test(initial, inputs, step, |state: &S| read(&write(state)))
}

/// [`restart_test`] for a [`Simulation`], deriving the step function from
/// [`Simulation::step`] so the simulation is not written a second way.
pub fn restart_test_sim<S, R>(
    initial: S,
    inputs: &[S::Input],
    round_trip: R,
) -> Result<(), RestartFailure>
where
    S: Simulation + DetHash + Clone,
    R: FnMut(&S) -> Option<S>,
{
    restart_test(
        initial,
        inputs,
        |s: &mut S, i: &S::Input| s.step(i),
        round_trip,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prop::forall_inputs;
    use crate::world_hash::Fnv1a;

    /// A state with a hashed field and a hidden one that the hash forgets.
    /// `gold` depends on `combo`, so dropping `combo` in a save is invisible at
    /// the moment of restore and only shows up on the next step — exactly the
    /// `Divergent` case.
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Chest {
        gold: i32,
        combo: i32,
    }

    impl DetHash for Chest {
        fn det_hash(&self, h: &mut Fnv1a) {
            h.write_i32(self.gold); // note: `combo` is deliberately not hashed
        }
    }

    impl Simulation for Chest {
        type Input = i32;
        fn step(&mut self, input: &i32) {
            self.combo = self.combo.wrapping_add(*input);
            self.gold = self.gold.wrapping_add(self.combo);
        }
    }

    fn step(s: &mut Chest, i: &i32) {
        s.combo = s.combo.wrapping_add(*i);
        s.gold = s.gold.wrapping_add(s.combo);
    }

    fn start() -> Chest {
        Chest { gold: 0, combo: 0 }
    }

    fn complete(s: &Chest) -> Option<Chest> {
        Some(s.clone())
    }

    #[test]
    fn test_complete_round_trip_survives_every_restart_point() {
        assert_eq!(restart_test(start(), &[3, 4, 5, 6], step, complete), Ok(()));
    }

    #[test]
    fn test_complete_round_trip_survives_random_runs() {
        // Property: for any input sequence, a lossless save is undetectable.
        let r = forall_inputs(
            0..300u64,
            20,
            |rng| rng.below(200) as i32 - 100,
            |inputs: &[i32]| restart_test(start(), inputs, step, complete).is_ok(),
        );
        assert_eq!(r, Ok(()), "a complete round trip must never be detected");
    }

    #[test]
    fn test_dropping_a_hashed_field_is_reported_as_lossy() {
        // `gold` is hashed, so the loss is visible the moment it is restored —
        // at the first restart point where gold is non-zero.
        let failure = restart_test(start(), &[3, 4, 5], step, |s: &Chest| {
            Some(Chest {
                gold: 0,
                combo: s.combo,
            })
        })
        .expect_err("dropping gold must be caught");

        match failure {
            RestartFailure::Lossy {
                restart_tick,
                expected,
                actual,
            } => {
                // gold is 0 at tick 0, so tick 1 is the earliest detection.
                assert_eq!(restart_tick, 1);
                assert_ne!(expected, actual);
                assert_eq!(actual, hash_state(&Chest { gold: 0, combo: 0 }));
            }
            other => panic!("expected Lossy, got {other:?}"),
        }
    }

    #[test]
    fn test_dropping_an_unhashed_field_is_reported_as_divergent() {
        // The pedagogically important case: `combo` is not hashed, so the
        // restored state looks identical and only the *next* step reveals the
        // loss. A test that compared hashes at the restore point alone would
        // pass this save format.
        let failure = restart_test(start(), &[3, 4, 5], step, |s: &Chest| {
            Some(Chest {
                gold: s.gold,
                combo: 0,
            })
        })
        .expect_err("dropping combo must be caught");

        match failure {
            RestartFailure::Divergent {
                restart_tick,
                detected_tick,
                expected,
                actual,
            } => {
                // combo is 0 at tick 0; after input 3 it is 3, so a restart at
                // tick 1 loses it and the very next input exposes that.
                assert_eq!(restart_tick, 1);
                assert_eq!(detected_tick, 2);
                assert_ne!(expected, actual);
            }
            other => panic!("expected Divergent, got {other:?}"),
        }
    }

    #[test]
    fn test_divergent_case_is_invisible_to_a_hash_only_check() {
        // Pins the claim above: at the restart point the hashes genuinely do
        // agree, which is why simulating onward is what finds the bug.
        let mut s = start();
        step(&mut s, &3);
        let restored = Chest {
            gold: s.gold,
            combo: 0,
        };
        assert_eq!(hash_state(&s), hash_state(&restored));
        assert_ne!(s, restored);
    }

    #[test]
    fn test_unloadable_save_is_reported_as_failed() {
        let failure = restart_test(start(), &[1, 2], step, |_s: &Chest| None)
            .expect_err("a save that never loads must be caught");
        assert_eq!(failure, RestartFailure::Failed { restart_tick: 0 });
        assert_eq!(failure.restart_tick(), 0);
    }

    #[test]
    fn test_a_save_that_only_breaks_late_reports_the_earliest_restart() {
        // Corrupt only when combo has grown past a threshold: the harness must
        // report the first restart point that exposes it, not the last.
        let failure = restart_test(start(), &[1, 1, 1, 1, 1], step, |s: &Chest| {
            Some(Chest {
                gold: s.gold,
                combo: if s.combo >= 3 { 0 } else { s.combo },
            })
        })
        .expect_err("must be caught once combo reaches 3");
        assert_eq!(failure.restart_tick(), 3, "{failure}");
    }

    #[test]
    fn test_empty_input_run_still_probes_the_initial_state() {
        // With no inputs there is exactly one restart point: before anything
        // happens. A save format broken at rest must still be caught.
        assert_eq!(restart_test(start(), &[], step, complete), Ok(()));

        let failure = restart_test(Chest { gold: 7, combo: 0 }, &[], step, |_s: &Chest| {
            Some(Chest { gold: 0, combo: 0 })
        })
        .expect_err("a broken save is broken with zero inputs too");
        assert_eq!(failure.restart_tick(), 0);
    }

    #[test]
    fn test_result_is_deterministic() {
        let run = || {
            restart_test(start(), &[5, -3, 9, 2], step, |s: &Chest| {
                Some(Chest {
                    gold: s.gold,
                    combo: 0,
                })
            })
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn test_sim_adapter_matches_the_closure_form() {
        let inputs = [4, -2, 7];
        let via_closure = restart_test(start(), &inputs, step, |s: &Chest| {
            Some(Chest {
                gold: s.gold,
                combo: 0,
            })
        });
        let via_sim = restart_test_sim(start(), &inputs, |s: &Chest| {
            Some(Chest {
                gold: s.gold,
                combo: 0,
            })
        });
        assert_eq!(via_closure, via_sim);
        assert!(via_closure.is_err());

        assert_eq!(restart_test_sim(start(), &inputs, complete), Ok(()));
    }

    // --- Integration with the real savefile container. ---

    const SAVE_VERSION: u32 = 1;

    fn encode(s: &Chest, include_combo: bool) -> Vec<u8> {
        use crate::savefile::{save_bytes, SaveHeader};
        let mut payload = Vec::new();
        payload.extend_from_slice(&s.gold.to_le_bytes());
        if include_combo {
            payload.extend_from_slice(&s.combo.to_le_bytes());
        }
        save_bytes(&SaveHeader::new(SAVE_VERSION), &payload)
    }

    fn decode(data: &[u8]) -> Option<Chest> {
        use crate::savefile::load_bytes;
        let (header, payload) = load_bytes(data).ok()?;
        if header.version != SAVE_VERSION {
            return None;
        }
        let gold = i32::from_le_bytes(payload.get(0..4)?.try_into().ok()?);
        // A payload without the combo field silently restores it as 0 — the
        // realistic form of the bug, since the file still parses.
        let combo = match payload.get(4..8) {
            Some(bytes) => i32::from_le_bytes(bytes.try_into().ok()?),
            None => 0,
        };
        Some(Chest { gold, combo })
    }

    #[test]
    fn test_real_savefile_round_trip_passes_when_complete() {
        let r = restart_test_bytes(
            start(),
            &[3, 4, 5, 6],
            step,
            |s: &Chest| encode(s, true),
            decode,
        );
        assert_eq!(r, Ok(()), "a complete save must survive every restart");
    }

    #[test]
    fn test_real_savefile_round_trip_catches_a_forgotten_field() {
        // The file is structurally valid — right magic, right version, correct
        // checksum — and `savefile` is perfectly happy with it. Only replaying
        // the simulation reveals that a field never made it in.
        let bytes = encode(&Chest { gold: 9, combo: 4 }, false);
        assert!(crate::savefile::validate_integrity(&bytes).is_ok());

        let failure = restart_test_bytes(
            start(),
            &[3, 4, 5, 6],
            step,
            |s: &Chest| encode(s, false),
            decode,
        )
        .expect_err("the forgotten combo field must be caught");
        assert!(
            matches!(failure, RestartFailure::Divergent { .. }),
            "{failure}"
        );
    }

    #[test]
    fn test_version_mismatch_surfaces_as_failed() {
        let r = restart_test_bytes(
            start(),
            &[1, 2],
            step,
            |s: &Chest| encode(s, true),
            |data: &[u8]| {
                let (header, _) = crate::savefile::load_bytes(data).ok()?;
                // Pretend the running build expects a newer format.
                if header.version != SAVE_VERSION + 1 {
                    return None;
                }
                unreachable!("version never matches in this test")
            },
        );
        assert_eq!(r, Err(RestartFailure::Failed { restart_tick: 0 }));
    }

    #[test]
    fn test_failure_display_names_the_fault_class() {
        let lossy = RestartFailure::Lossy {
            restart_tick: 2,
            expected: 0xaaaa,
            actual: 0xbbbb,
        };
        let text = lossy.to_string();
        assert!(text.contains("restart after 2 input(s)"), "{text}");
        assert!(text.contains("hashed field"), "{text}");

        let divergent = RestartFailure::Divergent {
            restart_tick: 1,
            detected_tick: 4,
            expected: 1,
            actual: 2,
        };
        let text = divergent.to_string();
        assert!(text.contains("diverged at input 4"), "{text}");
        assert!(text.contains("does not cover"), "{text}");

        assert!(RestartFailure::Failed { restart_tick: 0 }
            .to_string()
            .contains("could not be loaded"));
    }
}
