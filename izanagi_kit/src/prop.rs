//! Property-based testing: generate random inputs, check a property, and
//! shrink any counterexample to a minimal one — automatically.
//!
//! This is the core loop of QuickCheck (Claessen & Hughes, *QuickCheck: A
//! Lightweight Tool for Random Testing of Haskell Programs*, ICFP 2000): state
//! a property that should hold for **all** inputs, sample the input space at
//! random, and when a counterexample appears, report it *small* rather than
//! raw. Twenty-five years on it is the default testing style in every major
//! ecosystem (proptest/quickcheck in Rust, Hypothesis in Python) — none of
//! which this zero-dependency crate can pull in, so the loop lives here.
//!
//! Every piece already existed in the kit; this module is only their
//! composition:
//!
//! - **generation** draws from [`SplitMix64`] through a named sub-stream
//!   (`split`), the same discipline as
//!   [`dst_swarm_sweep`](crate::dst::dst_swarm_sweep) — so generating test
//!   data never disturbs a simulation's own RNG, and the seed alone
//!   reproduces the exact run;
//! - **checking** is a plain predicate over the generated sequence;
//! - **shrinking** is [`shrink_inputs`] (Zeller
//!   & Hildebrandt's `ddmin`), so a failure comes back **1-minimal**: remove
//!   any single element and the property holds again.
//!
//! The deterministic core is what makes the composition trivial. QuickCheck
//! implementations spend real machinery on re-running flaky properties and
//! keeping shrinking stable; here "does this input still fail?" is a pure
//! function of the input, because that is the crate's founding guarantee.
//!
//! ```
//! use izanagi_kit::prop::forall_inputs;
//!
//! // Property: reversing twice is the identity. Holds — so Ok.
//! let ok = forall_inputs(0..50u64, 16, |rng| rng.below(100) as i32, |xs| {
//!     let mut twice = xs.to_vec();
//!     twice.reverse();
//!     twice.reverse();
//!     twice == xs
//! });
//! assert!(ok.is_ok());
//!
//! // A false property: "no sequence sums past 300". The counterexample
//! // comes back already shrunk to a minimal witness.
//! let bad = forall_inputs(0..200u64, 24, |rng| rng.below(100) as i32, |xs| {
//!     xs.iter().sum::<i32>() <= 300
//! });
//! let failure = bad.unwrap_err();
//! assert!(failure.counterexample.iter().sum::<i32>() > 300);
//! ```

use crate::rng::SplitMix64;
use crate::shrink::shrink_inputs;
use crate::sim::Simulation;

/// Named sub-stream for input generation, so drawing test data neither
/// consumes from nor correlates with a simulation's own seeded streams.
const PROP_GEN_STREAM: u64 = 0x0050_524F_5047_454E; // "PROPGEN"

/// A falsified property: the seed that found it, how large the raw
/// counterexample was, and the [`shrink`](crate::shrink)-minimised
/// counterexample itself.
///
/// The counterexample is **1-minimal** (removing any single element makes the
/// property hold), and the seed alone re-derives the original failing
/// sequence, since generation reads only from a seed-derived sub-stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropFailure<T> {
    /// Seed of the failing run — regenerates the raw sequence exactly.
    pub seed: u64,
    /// Length of the sequence as generated, before shrinking.
    pub original_len: usize,
    /// The minimal failing subsequence, in original order.
    pub counterexample: Vec<T>,
}

impl<T: core::fmt::Debug> core::fmt::Display for PropFailure<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "property falsified by seed {}: {:?} (shrunk from {} to {} element(s))",
            self.seed,
            self.counterexample,
            self.original_len,
            self.counterexample.len()
        )
    }
}

/// Check that `prop` holds for randomly generated input sequences — the
/// QuickCheck loop (see the module docs).
///
/// For each seed, a sequence of `0..=max_len` elements is drawn via `gen_one`
/// from a seed-derived sub-stream, and `prop` is applied (`true` = the
/// property holds). The first falsifying sequence is shrunk with
/// [`shrink_inputs`] and returned as a
/// [`PropFailure`] whose counterexample is 1-minimal.
///
/// `prop` must be a pure function of the sequence — trivially true for
/// anything built on this crate's deterministic primitives. Note that `prop`
/// is also probed with *subsequences* during shrinking, so it should be
/// meaningful for any subsequence, not just generated ones (the usual
/// QuickCheck contract). Empty `seeds` or a property that never falsifies
/// returns `Ok(())`.
pub fn forall_inputs<T, G, P>(
    seeds: impl IntoIterator<Item = u64>,
    max_len: usize,
    mut gen_one: G,
    mut prop: P,
) -> Result<(), PropFailure<T>>
where
    T: Clone,
    G: FnMut(&mut SplitMix64) -> T,
    P: FnMut(&[T]) -> bool,
{
    // `below` takes a u32 bound; clamp pathological max_len rather than wrap.
    let bound = max_len.min((u32::MAX - 1) as usize) as u32 + 1;
    for seed in seeds {
        let mut rng = SplitMix64::new(seed).split(PROP_GEN_STREAM);
        let len = rng.below(bound) as usize;
        let inputs: Vec<T> = (0..len).map(|_| gen_one(&mut rng)).collect();
        if !prop(&inputs) {
            let counterexample = shrink_inputs(&inputs, |candidate| !prop(candidate));
            return Err(PropFailure {
                seed,
                original_len: len,
                counterexample,
            });
        }
    }
    Ok(())
}

/// [`forall_inputs`] for a [`Simulation`]: generate input sequences, run each
/// from a clone of `initial`, and require that `is_bad` never holds on the
/// resulting state. A failure is shrunk to a minimal input sequence that
/// still drives the simulation into a bad state.
///
/// This closes the loop with the rest of the kit: [`plan`](crate::plan) asks
/// "*can* the simulation reach a state?" by exhaustive search, this asks
/// "does random play ever reach a state it *shouldn't*?" — and hands back the
/// shortest such play it can distil.
///
/// ```
/// use izanagi_kit::prop::forall_states;
/// use izanagi_kit::sim::Simulation;
///
/// #[derive(Clone)]
/// struct Acc { total: i32 }
/// impl Simulation for Acc {
///     type Input = i32;
///     fn step(&mut self, input: &i32) { self.total += *input; }
/// }
///
/// // "The total never exceeds 500" is false; the witness comes back minimal.
/// let failure = forall_states(
///     0..300u64,
///     32,
///     &Acc { total: 0 },
///     |rng| rng.below(100) as i32,
///     |acc: &Acc| acc.total > 500,
/// )
/// .unwrap_err();
/// let sum: i32 = failure.counterexample.iter().sum();
/// assert!(sum > 500);
/// ```
pub fn forall_states<S, G, B>(
    seeds: impl IntoIterator<Item = u64>,
    max_len: usize,
    initial: &S,
    gen_one: G,
    mut is_bad: B,
) -> Result<(), PropFailure<S::Input>>
where
    S: Simulation + Clone,
    S::Input: Clone,
    G: FnMut(&mut SplitMix64) -> S::Input,
    B: FnMut(&S) -> bool,
{
    forall_inputs(seeds, max_len, gen_one, |inputs: &[S::Input]| {
        let mut state = initial.clone();
        for input in inputs {
            state.step(input);
        }
        !is_bad(&state)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shrink::is_one_minimal;

    #[test]
    fn test_true_property_passes_all_seeds() {
        // Sum is invariant under double reversal — vacuously true property.
        let r = forall_inputs(
            0..100u64,
            20,
            |rng| rng.below(1000) as i64,
            |xs| {
                let sum: i64 = xs.iter().sum();
                let mut rev = xs.to_vec();
                rev.reverse();
                sum == rev.iter().sum::<i64>()
            },
        );
        assert_eq!(r, Ok(()));
    }

    #[test]
    fn test_false_property_yields_one_minimal_counterexample() {
        // "No sequence contains a multiple of 17" is false. The shrunk
        // counterexample must be exactly one offending element — and we check
        // 1-minimality with the shrink module's own checker, not by trusting
        // forall_inputs.
        let prop = |xs: &[u32]| xs.iter().all(|v| v % 17 != 0);
        let failure = forall_inputs(0..200u64, 24, |rng| rng.below(1000), prop)
            .expect_err("multiples of 17 are common in 0..1000");
        assert!(!prop(&failure.counterexample), "counterexample must fail");
        assert!(
            is_one_minimal(&failure.counterexample, |c| !prop(c)),
            "not 1-minimal: {:?}",
            failure.counterexample
        );
        assert_eq!(failure.counterexample.len(), 1);
        assert_eq!(failure.counterexample[0] % 17, 0);
        assert!(failure.original_len >= 1);
    }

    #[test]
    fn test_counterexample_len_never_exceeds_original() {
        let failure = forall_inputs(
            0..200u64,
            24,
            |rng| rng.below(100) as i32,
            |xs| xs.iter().sum::<i32>() <= 300,
        )
        .expect_err("large sums are reachable");
        assert!(failure.counterexample.len() <= failure.original_len);
        assert!(failure.counterexample.iter().sum::<i32>() > 300);
    }

    #[test]
    fn test_same_seeds_give_identical_failure() {
        let run = || {
            forall_inputs(
                0..200u64,
                24,
                |rng| rng.below(100) as i32,
                |xs| xs.iter().sum::<i32>() <= 300,
            )
        };
        assert_eq!(run(), run(), "the whole loop must be deterministic");
    }

    #[test]
    fn test_generation_does_not_disturb_sim_stream() {
        // A generator seeded with the raw seed must observe the identical
        // stream inside and outside the prop loop (named sub-stream isolation,
        // same discipline as dst_swarm_sweep).
        let mut direct = SplitMix64::new(4242);
        let expect: Vec<u64> = (0..3).map(|_| direct.next_u64()).collect();

        let mut observed = Vec::new();
        let _ = forall_inputs(
            [4242u64],
            8,
            |rng| rng.below(10),
            |_xs| {
                let mut sim_rng = SplitMix64::new(4242);
                observed = (0..3).map(|_| sim_rng.next_u64()).collect();
                true
            },
        );
        assert_eq!(observed, expect);
    }

    #[test]
    fn test_empty_seeds_is_ok() {
        let r = forall_inputs(0..0u64, 8, |rng| rng.below(10), |_xs: &[u32]| false);
        assert_eq!(r, Ok(()), "no seeds → nothing checked → Ok");
    }

    #[test]
    fn test_max_len_zero_only_probes_the_empty_sequence() {
        // With max_len 0 every generated sequence is empty; a property that
        // rejects the empty sequence falsifies immediately with an empty
        // counterexample.
        let failure = forall_inputs(0..3u64, 0, |rng| rng.below(10), |xs: &[u32]| !xs.is_empty())
            .expect_err("the empty sequence violates the property");
        assert_eq!(failure.original_len, 0);
        assert!(failure.counterexample.is_empty());
        // And a property that accepts the empty sequence passes.
        let ok = forall_inputs(0..3u64, 0, |rng| rng.below(10), |xs: &[u32]| xs.is_empty());
        assert_eq!(ok, Ok(()));
    }

    #[test]
    fn test_failure_display_names_seed_and_shrink() {
        let f = PropFailure {
            seed: 9,
            original_len: 14,
            counterexample: vec![51u32],
        };
        let text = f.to_string();
        assert!(text.contains("seed 9"), "{text}");
        assert!(text.contains("[51]"), "{text}");
        assert!(text.contains("14 to 1"), "{text}");
    }

    // --- forall_states / Simulation integration ---

    #[derive(Clone)]
    struct Acc {
        total: i32,
    }

    impl Simulation for Acc {
        type Input = i32;
        fn step(&mut self, input: &i32) {
            self.total += *input;
        }
    }

    #[test]
    fn test_forall_states_finds_and_shrinks_a_bad_state() {
        let failure = forall_states(
            0..300u64,
            32,
            &Acc { total: 0 },
            |rng| rng.below(100) as i32,
            |acc: &Acc| acc.total > 500,
        )
        .expect_err("random play can exceed 500");

        // Replay the witness through the sim adapter: it must reach the bad
        // state, and be 1-minimal for "still reaches the bad state".
        let end = crate::sim::resimulate(&Acc { total: 0 }, &failure.counterexample);
        assert!(end.total > 500, "witness must reproduce the bad state");
        let reaches_bad = |c: &[i32]| crate::sim::resimulate(&Acc { total: 0 }, c).total > 500;
        assert!(
            is_one_minimal(&failure.counterexample, reaches_bad),
            "witness not minimal: {:?}",
            failure.counterexample
        );
    }

    #[test]
    fn test_forall_states_passes_when_bad_state_unreachable() {
        // Inputs are 0..10 and at most 8 of them: total can never reach 100.
        let r = forall_states(
            0..100u64,
            8,
            &Acc { total: 0 },
            |rng| rng.below(10) as i32,
            |acc: &Acc| acc.total >= 100,
        );
        assert_eq!(r, Ok(()));
    }

    #[test]
    fn test_forall_states_leaves_initial_untouched() {
        let initial = Acc { total: 7 };
        let _ = forall_states(
            0..50u64,
            16,
            &initial,
            |rng| rng.below(100) as i32,
            |acc: &Acc| acc.total > 200,
        );
        assert_eq!(initial.total, 7, "candidates must run on clones");
    }
}
