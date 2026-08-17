//! Bounded model checking: prove an invariant holds in **every** reachable
//! state, or produce the shortest input sequence that breaks it.
//!
//! ## The gap this fills
//!
//! Everything else in the kit hunts for bugs. [`dst`](crate::dst) samples
//! seeds, [`prop`](crate::prop) samples inputs, [`explore`](mod@crate::explore)
//! samples the frontier — all of them find violations and none of them can
//! ever tell you there are none. [`plan_inputs`](crate::plan::plan_inputs)
//! comes closest, since it searches breadth-first over the whole state space,
//! but its `None` is ambiguous by construction: the search either emptied its
//! queue (there is genuinely no such state) or tripped `max_states` (it gave
//! up), and the caller cannot tell which. That ambiguity is the difference
//! between a bug hunt and a proof.
//!
//! [`check_invariant`] resolves it. The result is three-way:
//!
//! - [`Holds`](Verification::Holds) — the reachable state space was
//!   **exhausted** and the invariant held in all of it. Within the model, this
//!   is a proof.
//! - [`Violated`](Verification::Violated) — here is the **shortest** input
//!   sequence reaching a state that breaks it.
//! - [`Exhausted`](Verification::Exhausted) — the bound was hit first;
//!   nothing is proved either way.
//!
//! That is the same three-valued shape as
//! [`temporal::Verdict`](crate::temporal::Verdict), and for the same reason:
//! "I could not settle this" is a distinct answer from "yes" or "no", and
//! collapsing it into either one is how a verification tool starts lying.
//!
//! ## What this is
//!
//! This is explicit-state model checking in the tradition of Clarke, Emerson &
//! Sistla (*Automatic Verification of Finite-State Concurrent Systems Using
//! Temporal Logic Specifications*, ACM TOPLAS 8(2), 1986) and SPIN (Holzmann,
//! *The Model Checker SPIN*, IEEE TSE 23(5), 1997): enumerate reachable states
//! breadth-first, check the property at each, and report a shortest
//! counterexample trace. Game logic is an unusually good fit — the interesting
//! state spaces (a puzzle room, a crafting economy, an ability's interaction
//! with a status effect) are small and finite, even though the whole game is
//! not.
//!
//! Because the search is breadth-first, a reported counterexample is
//! **minimal in length**, so it is already the regression test you want; and
//! because it is an ordinary input sequence it replays through
//! [`resimulate`](crate::replay::resimulate) and shrinks further through
//! [`shrink_inputs`](crate::shrink::shrink_inputs) if the inputs themselves
//! can be simplified.
//!
//! ## The one caveat, stated precisely
//!
//! States are deduplicated by their 64-bit [`DetHash`] digest rather than by
//! equality, which is what lets this work for any `S: DetHash + Clone` without
//! demanding `S: Eq + Hash` (the same trade
//! [`plan_inputs`](crate::plan::plan_inputs) makes). A hash collision would
//! make the checker treat two different states as one and skip a subtree, so
//! [`Holds`](Verification::Holds) is a proof *modulo* collisions. For `n`
//! distinct states the birthday bound puts that at about `n² / 2⁶⁵` — below
//! 3 × 10⁻⁸ for a million states, and
//! [`hash_state_mixed`](crate::world_hash::hash_state_mixed) tightens the
//! distribution further. [`Holds::states`](Verification::Holds) is reported so
//! the bound can be computed for a given run rather than taken on trust.
//!
//! ```
//! use izanagi_kit::verify::{check_invariant, Verification};
//! use izanagi_kit::world_hash::{DetHash, Fnv1a};
//!
//! // A tiny economy: spend 3 or earn 2, never allowed to go negative.
//! #[derive(Clone)]
//! struct Purse { coins: i32 }
//! impl DetHash for Purse {
//!     fn det_hash(&self, h: &mut Fnv1a) { h.write_i32(self.coins); }
//! }
//!
//! // Clamped to a purse that holds 0..=10: a finite space, so the checker
//! // exhausts it and the floor is *proved*.
//! let ok = check_invariant(
//!     Purse { coins: 5 },
//!     &[-3, 2],
//!     |p: &Purse, d: &i32| Purse { coins: (p.coins + d).clamp(0, 10) },
//!     |p: &Purse| p.coins >= 0,
//!     10_000,
//! );
//! assert!(matches!(ok, Verification::Holds { .. }));
//!
//! // Unclamped: it finds the shortest way to go bankrupt — two spends.
//! let bad = check_invariant(
//!     Purse { coins: 5 },
//!     &[-3, 2],
//!     |p: &Purse, d: &i32| Purse { coins: p.coins + d },
//!     |p: &Purse| p.coins >= 0,
//!     10_000,
//! );
//! match bad {
//!     Verification::Violated(cx) => assert_eq!(cx.path, vec![-3, -3]),
//!     other => panic!("expected a counterexample, got {other}"),
//! }
//!
//! // Guarded only at the floor, so earning is unbounded and the space is
//! // infinite. Nothing is provable here — and the checker says so rather
//! // than claiming a proof it cannot make.
//! let unbounded = check_invariant(
//!     Purse { coins: 5 },
//!     &[-3, 2],
//!     |p: &Purse, d: &i32| Purse { coins: (p.coins + d).max(0) },
//!     |p: &Purse| p.coins >= 0,
//!     10_000,
//! );
//! assert!(unbounded.is_exhausted());
//! ```

use std::collections::{HashSet, VecDeque};

use crate::sim::Simulation;
use crate::world_hash::{hash_state, DetHash};

/// A shortest input sequence driving the simulation into a state that breaks
/// the invariant.
///
/// `path` is minimal in length because the search is breadth-first: no shorter
/// sequence reaches any violating state. An empty `path` means the *initial*
/// state already violates the invariant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Counterexample<I> {
    /// The shortest input sequence reaching a violating state.
    pub path: Vec<I>,
    /// How many distinct states had been visited when it was found.
    pub states: usize,
}

/// The outcome of [`check_invariant`] — three-way, because "could not settle
/// it" is a real answer. See the module docs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verification<I> {
    /// The reachable state space was exhausted and the invariant held
    /// throughout. A proof, modulo hash collisions (see the module docs).
    Holds {
        /// Number of distinct reachable states, by [`DetHash`] digest.
        states: usize,
        /// The longest shortest-path from the initial state — the diameter of
        /// the reachable state space.
        diameter: usize,
    },
    /// A reachable state breaks the invariant, and here is the shortest way
    /// to get there.
    Violated(Counterexample<I>),
    /// `max_states` was reached before the space was exhausted. Nothing is
    /// proved: there may or may not be a violation beyond the bound.
    Exhausted {
        /// Number of distinct states visited before giving up.
        states: usize,
        /// The greatest depth fully expanded before giving up.
        depth: usize,
    },
}

impl<I> Verification<I> {
    /// Whether this is a proof that the invariant holds everywhere reachable.
    ///
    /// Note this is **not** `!is_violated()` — an
    /// [`Exhausted`](Verification::Exhausted) result is neither.
    pub fn holds(&self) -> bool {
        matches!(self, Verification::Holds { .. })
    }

    /// Whether a counterexample was found.
    pub fn is_violated(&self) -> bool {
        matches!(self, Verification::Violated(_))
    }

    /// Whether the search ran out of budget without settling the question.
    pub fn is_exhausted(&self) -> bool {
        matches!(self, Verification::Exhausted { .. })
    }

    /// The counterexample, if there is one.
    pub fn counterexample(&self) -> Option<&Counterexample<I>> {
        match self {
            Verification::Violated(cx) => Some(cx),
            _ => None,
        }
    }

    /// Number of distinct states visited, whatever the outcome.
    pub fn states(&self) -> usize {
        match self {
            Verification::Holds { states, .. } | Verification::Exhausted { states, .. } => *states,
            Verification::Violated(cx) => cx.states,
        }
    }
}

impl<I: core::fmt::Debug> core::fmt::Display for Verification<I> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Verification::Holds { states, diameter } => write!(
                f,
                "invariant holds in all {states} reachable state(s) \
                 (diameter {diameter})"
            ),
            Verification::Violated(cx) => write!(
                f,
                "invariant violated after {} input(s): {:?} (found in {} state(s))",
                cx.path.len(),
                cx.path,
                cx.states
            ),
            Verification::Exhausted { states, depth } => write!(
                f,
                "inconclusive: bound reached at {states} state(s), depth {depth} \
                 — raise max_states or shrink the model"
            ),
        }
    }
}

/// Check `invariant` against every state reachable from `initial` by applying
/// `inputs`, breadth-first.
///
/// - `step(state, input) -> S` is the pure transition function, the same shape
///   [`plan_inputs`](crate::plan::plan_inputs) and
///   [`resimulate`](crate::replay::resimulate) use.
/// - `invariant(state) -> bool` must hold in every reachable state; `false`
///   means a violation.
/// - `max_states` bounds the search. Reaching it yields
///   [`Exhausted`](Verification::Exhausted) — never a false
///   [`Holds`](Verification::Holds).
///
/// Inputs are expanded in slice order, so among equally short counterexamples
/// the returned one is stable across runs and machines.
pub fn check_invariant<S, I, F, C>(
    initial: S,
    inputs: &[I],
    step: F,
    invariant: C,
    max_states: usize,
) -> Verification<I>
where
    S: DetHash + Clone,
    I: Clone,
    F: Fn(&S, &I) -> S,
    C: Fn(&S) -> bool,
{
    if !invariant(&initial) {
        return Verification::Violated(Counterexample {
            path: Vec::new(),
            states: 1,
        });
    }

    let mut visited: HashSet<u64> = HashSet::new();
    visited.insert(hash_state(&initial));

    // Parent links for shortest-path reconstruction: node -> (parent, input).
    // The root has no parent.
    let mut parents: Vec<Option<(usize, usize)>> = vec![None];
    let mut queue: VecDeque<(S, usize, usize)> = VecDeque::new();
    queue.push_back((initial, 0, 0));
    let mut diameter = 0usize;

    while let Some((state, depth, node)) = queue.pop_front() {
        diameter = diameter.max(depth);

        for (slot, input) in inputs.iter().enumerate() {
            let next = step(&state, input);
            if !visited.insert(hash_state(&next)) {
                continue; // already reached by an equal-or-shorter path
            }
            let next_node = parents.len();
            parents.push(Some((node, slot)));

            if !invariant(&next) {
                return Verification::Violated(Counterexample {
                    path: reconstruct(&parents, next_node, inputs),
                    states: visited.len(),
                });
            }
            if visited.len() >= max_states {
                return Verification::Exhausted {
                    states: visited.len(),
                    depth,
                };
            }
            queue.push_back((next, depth + 1, next_node));
        }
    }

    Verification::Holds {
        states: visited.len(),
        diameter,
    }
}

/// Walk parent links back to the root and return the inputs in forward order.
fn reconstruct<I: Clone>(
    parents: &[Option<(usize, usize)>],
    mut node: usize,
    inputs: &[I],
) -> Vec<I> {
    let mut path = Vec::new();
    while let Some((parent, slot)) = parents[node] {
        path.push(inputs[slot].clone());
        node = parent;
    }
    path.reverse();
    path
}

/// [`check_invariant`] for a [`Simulation`], deriving the pure transition from
/// `Clone` + [`Simulation::step`] so the simulation is not written a second
/// way.
pub fn check_invariant_sim<S, C>(
    initial: S,
    inputs: &[S::Input],
    invariant: C,
    max_states: usize,
) -> Verification<S::Input>
where
    S: Simulation + DetHash + Clone,
    S::Input: Clone,
    C: Fn(&S) -> bool,
{
    check_invariant(
        initial,
        inputs,
        |s: &S, i: &S::Input| {
            let mut next = s.clone();
            next.step(i);
            next
        },
        invariant,
        max_states,
    )
}

/// Measure the reachable state space from `initial` without checking anything:
/// `Some((states, diameter))` if it was fully enumerated within `max_states`,
/// `None` if the bound was hit.
///
/// Worth running before a real check — it tells you whether the model is small
/// enough to verify at all, and the diameter is a useful sanity check on
/// whether the model means what you think it does.
pub fn reachable_states<S, I, F>(
    initial: S,
    inputs: &[I],
    step: F,
    max_states: usize,
) -> Option<(usize, usize)>
where
    S: DetHash + Clone,
    I: Clone,
    F: Fn(&S, &I) -> S,
{
    match check_invariant(initial, inputs, step, |_s: &S| true, max_states) {
        Verification::Holds { states, diameter } => Some((states, diameter)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::plan_inputs;
    use crate::prop::forall_inputs;
    use crate::world_hash::Fnv1a;

    /// A counter confined to `0..=cap` by clamping — a state space whose exact
    /// size and diameter can be worked out by hand, so `Holds` can be checked
    /// against an independently known answer.
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Clamped {
        at: i32,
        cap: i32,
    }

    impl DetHash for Clamped {
        fn det_hash(&self, h: &mut Fnv1a) {
            h.write_i32(self.at);
            h.write_i32(self.cap);
        }
    }

    impl Simulation for Clamped {
        type Input = i32;
        fn step(&mut self, input: &i32) {
            self.at = (self.at + input).clamp(0, self.cap);
        }
    }

    fn step(s: &Clamped, i: &i32) -> Clamped {
        let mut next = s.clone();
        next.step(i);
        next
    }

    fn start(cap: i32) -> Clamped {
        Clamped { at: 0, cap }
    }

    #[test]
    fn test_holds_reports_the_hand_computable_state_count_and_diameter() {
        // Steps of +1/-1 clamped to 0..=9 reach exactly 10 states, and the
        // furthest (at = 9) is 9 steps away. Both numbers are known without
        // running the checker.
        let result = check_invariant(start(9), &[1, -1], step, |c: &Clamped| c.at <= 9, 10_000);
        assert_eq!(
            result,
            Verification::Holds {
                states: 10,
                diameter: 9
            }
        );
        assert!(result.holds());
        assert!(!result.is_violated());
    }

    #[test]
    fn test_reachable_states_agrees_with_the_same_numbers() {
        assert_eq!(
            reachable_states(start(9), &[1, -1], step, 10_000),
            Some((10, 9))
        );
        // With a +3 step from 0 under clamping to 0..=9, reachable values are
        // 0,3,6,9 going up and every value reachable coming back down by 1.
        let (states, _) = reachable_states(start(9), &[3, -1], step, 10_000).unwrap();
        assert_eq!(states, 10);
    }

    #[test]
    fn test_counterexample_length_matches_bfs_shortest_path() {
        // Differential oracle: the shortest violating path the checker reports
        // must be exactly as long as the shortest path plan_inputs finds to a
        // violating state. Two independent BFS implementations agreeing on the
        // distance is a real check on both.
        let bad = |c: &Clamped| c.at >= 7;
        let cx = check_invariant(start(20), &[1, -1], step, |c: &Clamped| !bad(c), 100_000)
            .counterexample()
            .cloned()
            .expect("at = 7 is reachable");
        let shortest =
            plan_inputs(start(20), &[1, -1], step, bad, 100_000).expect("plan must find it too");
        assert_eq!(cx.path.len(), shortest.len());
        assert_eq!(cx.path.len(), 7);
    }

    #[test]
    fn test_counterexample_is_shortest_not_merely_reachable() {
        // The line-shaped models above cannot tell breadth-first from
        // depth-first, because walking one input repeatedly happens to be
        // optimal there. This one separates them: with `+10` and `+1`, the
        // shortest route to 25 is two tens and five ones (7 inputs), while a
        // depth-first search that keeps taking the last-pushed child walks up
        // in ones and arrives with 25. Anything but a genuinely shortest path
        // fails here.
        let inv = |c: &Clamped| c.at != 25;
        let cx = check_invariant(start(100), &[10, 1], step, inv, 100_000)
            .counterexample()
            .cloned()
            .expect("25 is reachable");
        assert_eq!(cx.path.len(), 7, "got {:?}", cx.path);

        // Same distance from the independent BFS in `plan`.
        let shortest = plan_inputs(
            start(100),
            &[10, 1],
            step,
            |c: &Clamped| c.at == 25,
            100_000,
        )
        .expect("plan must find it too");
        assert_eq!(shortest.len(), 7);

        // And it really lands on 25.
        let end = cx.path.iter().fold(start(100), |s, i| step(&s, i));
        assert_eq!(end.at, 25);
    }

    #[test]
    fn test_counterexample_actually_reaches_a_violating_state() {
        let cx = check_invariant(start(50), &[3, -1], step, |c: &Clamped| c.at != 11, 100_000)
            .counterexample()
            .cloned()
            .expect("11 is reachable from 0 by +3/-1");
        let end = cx.path.iter().fold(start(50), |s, i| step(&s, i));
        assert_eq!(end.at, 11, "the reported path must really get there");
    }

    #[test]
    fn test_violated_at_the_initial_state_gives_an_empty_path() {
        let result = check_invariant(start(9), &[1, -1], step, |c: &Clamped| c.at > 0, 10_000);
        match result {
            Verification::Violated(cx) => {
                assert!(cx.path.is_empty());
                assert_eq!(cx.states, 1);
            }
            other => panic!("expected an immediate violation, got {other}"),
        }
    }

    #[test]
    fn test_bound_yields_exhausted_and_never_a_false_proof() {
        // An unbounded space: the checker must say "inconclusive", not "holds".
        let unbounded = |s: &i64, i: &i64| s + i;
        let result = check_invariant(0i64, &[1, -1], unbounded, |_s: &i64| true, 64);
        assert!(result.is_exhausted(), "{result}");
        assert!(!result.holds(), "an unbounded space must never be proved");
        assert!(result.states() >= 64);
        assert_eq!(reachable_states(0i64, &[1, -1], unbounded, 64), None);
    }

    #[test]
    fn test_a_violation_exactly_at_the_bound_still_wins_over_exhaustion() {
        // The bound must not mask a violation the search has already reached:
        // the invariant is checked before the budget is. Tuned so the two
        // collide — states 0,1,2,3 are discovered in order and 3 is both the
        // violating state and the 4th discovered, so `max_states = 4` makes
        // the check order decide the answer.
        let result = check_invariant(0i64, &[1], |s: &i64, i: &i64| s + i, |s: &i64| *s < 3, 4);
        match result {
            Verification::Violated(cx) => {
                assert_eq!(cx.path, vec![1, 1, 1]);
                assert_eq!(cx.states, 4, "the violation lands on the budget edge");
            }
            other => panic!("expected a violation, got {other}"),
        }
        // One state earlier, the budget really does bite.
        let earlier = check_invariant(0i64, &[1], |s: &i64, i: &i64| s + i, |s: &i64| *s < 3, 3);
        assert!(earlier.is_exhausted(), "{earlier}");
    }

    #[test]
    fn test_holds_is_never_contradicted_by_random_search() {
        // Soundness property: whenever the checker proves an invariant, random
        // play must never falsify it. Checked over random input sequences with
        // prop, which is an independent implementation of "run the sim".
        let inv = |c: &Clamped| c.at >= 0 && c.at <= 9;
        let proof = check_invariant(start(9), &[1, -1, 4, -4], step, inv, 10_000);
        assert!(proof.holds(), "{proof}");

        let r = forall_inputs(
            0..500u64,
            32,
            |rng| [1i32, -1, 4, -4][rng.below(4) as usize],
            |seq: &[i32]| {
                let end = seq.iter().fold(start(9), |s, i| step(&s, i));
                inv(&end)
            },
        );
        assert_eq!(r, Ok(()), "random play contradicted a proof");
    }

    #[test]
    fn test_random_search_findings_are_always_found_by_the_checker() {
        // Completeness in the other direction: a violation random play can
        // reach must also be reported by the checker, with a path no longer
        // than the one random play stumbled on.
        let bad = |c: &Clamped| c.at >= 6;
        let witness = forall_inputs(
            0..500u64,
            32,
            |rng| [1i32, -1][rng.below(2) as usize],
            |seq: &[i32]| {
                let end = seq.iter().fold(start(20), |s, i| step(&s, i));
                !bad(&end)
            },
        )
        .expect_err("random play reaches at >= 6");

        let cx = check_invariant(start(20), &[1, -1], step, |c: &Clamped| !bad(c), 100_000)
            .counterexample()
            .cloned()
            .expect("the checker must find it too");
        assert!(
            cx.path.len() <= witness.counterexample.len(),
            "checker path {} longer than the random witness {}",
            cx.path.len(),
            witness.counterexample.len()
        );
    }

    #[test]
    fn test_empty_input_alphabet_proves_over_the_initial_state_alone() {
        let result = check_invariant(start(9), &[], step, |c: &Clamped| c.at == 0, 10_000);
        assert_eq!(
            result,
            Verification::Holds {
                states: 1,
                diameter: 0
            }
        );
        // And a broken invariant is still caught with no inputs at all.
        assert!(
            check_invariant(start(9), &[], step, |c: &Clamped| c.at == 5, 10_000).is_violated()
        );
    }

    #[test]
    fn test_result_is_deterministic() {
        let run = || check_invariant(start(30), &[2, -3], step, |c: &Clamped| c.at != 17, 100_000);
        assert_eq!(run(), run());
    }

    #[test]
    fn test_tie_break_follows_input_order() {
        // Two single-input paths both reach a violating state; the one listed
        // first in `inputs` must win, so results are stable across runs.
        let first = check_invariant(start(9), &[1, 2], step, |c: &Clamped| c.at == 0, 10_000);
        let swapped = check_invariant(start(9), &[2, 1], step, |c: &Clamped| c.at == 0, 10_000);
        assert_eq!(first.counterexample().unwrap().path, vec![1]);
        assert_eq!(swapped.counterexample().unwrap().path, vec![2]);
    }

    #[test]
    fn test_sim_adapter_matches_the_closure_form() {
        let inv = |c: &Clamped| c.at != 5;
        assert_eq!(
            check_invariant(start(9), &[1, -1], step, inv, 10_000),
            check_invariant_sim(start(9), &[1, -1], inv, 10_000)
        );
        assert!(check_invariant_sim(start(9), &[1, -1], inv, 10_000).is_violated());
    }

    #[test]
    fn test_display_distinguishes_the_three_outcomes() {
        let holds: Verification<i32> = Verification::Holds {
            states: 12,
            diameter: 4,
        };
        assert!(holds.to_string().contains("holds in all 12"), "{holds}");

        let violated = Verification::Violated(Counterexample {
            path: vec![1, 2],
            states: 9,
        });
        assert!(
            violated.to_string().contains("after 2 input(s)"),
            "{violated}"
        );

        let stuck: Verification<i32> = Verification::Exhausted {
            states: 64,
            depth: 3,
        };
        assert!(stuck.to_string().contains("inconclusive"), "{stuck}");
        // The three predicates are mutually exclusive — `holds` is not the
        // negation of `is_violated`, which is the whole point of three values.
        assert!(!stuck.holds() && !stuck.is_violated() && stuck.is_exhausted());
    }
}
