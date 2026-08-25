//! Test-case reduction: shrink a failing input sequence to a minimal one.
//!
//! The rest of the kit can *find* a failure ([`dst`](crate::dst),
//! [`sim::audit`](crate::sim::audit)) and *reproduce* it exactly (a seed, or a
//! recorded input log). What it could not do is make the failure **small**. A
//! sweep that reports "seed 41 violates the invariant at tick 847" still leaves
//! a human reading 847 steps to find the three that mattered.
//!
//! [`shrink_inputs`] closes that gap with **delta debugging** — Zeller &
//! Hildebrandt's `ddmin` (*Simplifying and Isolating Failure-Inducing Input*,
//! IEEE Transactions on Software Engineering 28(2), 2002), the algorithm behind
//! QuickCheck-style shrinking and C-Reduce. It repeatedly removes chunks of the
//! input and keeps whichever smaller sequence still fails, at ever finer
//! granularity, until the result is **1-minimal**: deleting any single
//! remaining element makes the failure go away.
//!
//! ## Why this fits a deterministic simulation especially well
//!
//! Delta debugging needs a test predicate that is *stable* — the same candidate
//! must give the same verdict every time, or the search wanders and the result
//! is meaningless. For a nondeterministic system that is a real obstacle, and
//! practical shrinkers add retries and flakiness heuristics.
//!
//! Here it is free. A [`Simulation`] is a pure function
//! of `(initial state, input sequence)`, so "does this subsequence still fail?"
//! is a genuine mathematical predicate. The kit's central guarantee is exactly
//! the precondition the algorithm wants, which is why shrinking belongs in this
//! crate rather than being left to the caller.
//!
//! ```
//! use izanagi_kit::shrink::shrink_inputs;
//!
//! // A failure that really depends on just two of the inputs.
//! let inputs: Vec<i32> = (0..40).collect();
//! let minimal = shrink_inputs(&inputs, |c| c.contains(&7) && c.contains(&23));
//! assert_eq!(minimal, vec![7, 23]);
//! ```
//!
//! ## Cost
//!
//! `ddmin` is worst-case `O(n²)` predicate calls for `n` inputs (typically far
//! fewer). Each call re-runs the simulation over a candidate subsequence, so
//! shrink *after* a sweep has already isolated one failing run — not inside the
//! sweep.

use crate::sim::Simulation;

/// Reduce `inputs` to a **1-minimal** subsequence that still satisfies `fails`,
/// using Zeller & Hildebrandt's `ddmin` (see the module docs).
///
/// `fails(candidate)` must report whether that candidate still exhibits the
/// failure, and must be a pure function of the candidate — which it is for any
/// deterministic simulation. Element order is always preserved: the result is a
/// subsequence of `inputs`, never a reordering.
///
/// Returns `inputs` unchanged when it is empty or does not fail in the first
/// place (there is nothing to reduce). Otherwise the result is guaranteed to
/// still fail, and to be 1-minimal — removing any single element from it makes
/// `fails` return `false`. [`is_one_minimal`] checks that property directly.
pub fn shrink_inputs<I, F>(inputs: &[I], mut fails: F) -> Vec<I>
where
    I: Clone,
    F: FnMut(&[I]) -> bool,
{
    let mut current: Vec<I> = inputs.to_vec();
    if current.is_empty() || !fails(&current) {
        return current;
    }

    // Granularity: how many chunks the sequence is split into this round.
    let mut n = 2usize;
    while current.len() >= 2 {
        let chunk_len = current.len().div_ceil(n);
        let mut progressed = false;

        // Phase 1 — "reduce to subset": does one chunk alone still fail? This
        // is the big win when it hits, so it is tried first and resets the
        // granularity.
        let mut start = 0;
        while start < current.len() {
            let end = (start + chunk_len).min(current.len());
            if end - start < current.len() && fails(&current[start..end]) {
                current = current[start..end].to_vec();
                n = 2;
                progressed = true;
                break;
            }
            start = end;
        }
        if progressed {
            continue;
        }

        // Phase 2 — "reduce to complement": does removing one chunk still
        // fail? Coarser progress, so granularity only steps back by one.
        let mut start = 0;
        while start < current.len() {
            let end = (start + chunk_len).min(current.len());
            let mut candidate: Vec<I> = Vec::with_capacity(current.len() - (end - start));
            candidate.extend_from_slice(&current[..start]);
            candidate.extend_from_slice(&current[end..]);
            if fails(&candidate) {
                current = candidate;
                n = (n - 1).max(2);
                progressed = true;
                break;
            }
            start = end;
        }
        if progressed {
            continue;
        }

        // Neither worked: refine the granularity, or stop once chunks are
        // single elements and nothing more can be removed.
        if n >= current.len() {
            break;
        }
        n = (2 * n).min(current.len());
    }
    current
}

/// Whether `inputs` is **1-minimal** under `fails`: it fails, and removing any
/// single element makes it stop failing.
///
/// This is the guarantee [`shrink_inputs`] provides, exposed so callers (and
/// tests) can assert it directly rather than trusting the implementation.
/// An empty sequence is 1-minimal exactly when it fails.
pub fn is_one_minimal<I, F>(inputs: &[I], mut fails: F) -> bool
where
    I: Clone,
    F: FnMut(&[I]) -> bool,
{
    if !fails(inputs) {
        return false;
    }
    for skip in 0..inputs.len() {
        let mut candidate: Vec<I> = Vec::with_capacity(inputs.len() - 1);
        candidate.extend_from_slice(&inputs[..skip]);
        candidate.extend_from_slice(&inputs[skip + 1..]);
        if fails(&candidate) {
            return false;
        }
    }
    true
}

/// [`shrink_inputs`] for a [`Simulation`]: reduce `inputs` to a minimal
/// subsequence that still drives `initial` into a state `is_bad` rejects.
///
/// The common shape after a failing sweep — you have a starting state, the
/// input log that broke it, and a predicate for "this state is wrong". Each
/// candidate is run from a fresh clone of `initial`, so the search never
/// observes state left over from a previous attempt.
///
/// ```
/// use izanagi_kit::shrink::shrink_simulation_inputs;
/// use izanagi_kit::sim::Simulation;
///
/// #[derive(Clone)]
/// struct Acc { total: i32 }
/// impl Simulation for Acc {
///     type Input = i32;
///     fn step(&mut self, input: &i32) { self.total += *input; }
/// }
///
/// // Only a few of these are needed to push the total past 100.
/// let inputs: Vec<i32> = vec![1, 1, 60, 1, 1, 50, 1, 1];
/// let minimal = shrink_simulation_inputs(&Acc { total: 0 }, &inputs, |a: &Acc| a.total > 100);
/// assert_eq!(minimal, vec![60, 50]);
/// ```
pub fn shrink_simulation_inputs<S, P>(
    initial: &S,
    inputs: &[S::Input],
    mut is_bad: P,
) -> Vec<S::Input>
where
    S: Simulation + Clone,
    S::Input: Clone,
    P: FnMut(&S) -> bool,
{
    shrink_inputs(inputs, |candidate| {
        let mut state = initial.clone();
        for input in candidate {
            state.step(input);
        }
        is_bad(&state)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::{DetHash, Fnv1a};

    #[test]
    fn test_shrinks_to_the_two_elements_that_matter() {
        let inputs: Vec<i32> = (0..64).collect();
        let minimal = shrink_inputs(&inputs, |c| c.contains(&7) && c.contains(&23));
        assert_eq!(minimal, vec![7, 23]);
    }

    #[test]
    fn test_shrinks_to_a_single_element() {
        let inputs: Vec<i32> = (0..100).collect();
        let minimal = shrink_inputs(&inputs, |c| c.contains(&42));
        assert_eq!(minimal, vec![42]);
    }

    #[test]
    fn test_result_still_fails_and_is_one_minimal() {
        // The two guarantees, checked independently of how ddmin got there.
        let inputs: Vec<i32> = (0..50).collect();
        let predicate = |c: &[i32]| c.iter().filter(|v| **v % 7 == 0).count() >= 3;
        let minimal = shrink_inputs(&inputs, predicate);
        assert!(predicate(&minimal), "reduced input must still fail");
        assert!(
            is_one_minimal(&minimal, predicate),
            "removing any single element must fix it: {minimal:?}"
        );
        assert_eq!(minimal.len(), 3, "exactly three multiples of 7 are needed");
    }

    #[test]
    fn test_preserves_relative_order() {
        let inputs: Vec<char> = "the quick brown fox".chars().collect();
        // Needs 'q' before 'x' — order-sensitive predicate.
        let minimal = shrink_inputs(&inputs, |c| {
            let q = c.iter().position(|&ch| ch == 'q');
            let x = c.iter().position(|&ch| ch == 'x');
            matches!((q, x), (Some(a), Some(b)) if a < b)
        });
        assert_eq!(minimal, vec!['q', 'x']);
    }

    #[test]
    fn test_already_minimal_input_is_unchanged() {
        let inputs = vec![5i32];
        let minimal = shrink_inputs(&inputs, |c| c.contains(&5));
        assert_eq!(minimal, vec![5]);
    }

    #[test]
    fn test_non_failing_input_is_returned_unchanged() {
        // Nothing to reduce: the precondition (the input fails) does not hold.
        let inputs: Vec<i32> = (0..10).collect();
        let minimal = shrink_inputs(&inputs, |_| false);
        assert_eq!(minimal, inputs);
    }

    #[test]
    fn test_empty_input_is_returned_unchanged() {
        let inputs: Vec<i32> = Vec::new();
        assert!(shrink_inputs(&inputs, |_| true).is_empty());
        assert!(shrink_inputs(&inputs, |_| false).is_empty());
    }

    #[test]
    fn test_predicate_true_for_everything_shrinks_to_one_element() {
        // A predicate that always holds should reduce as far as possible.
        let inputs: Vec<i32> = (0..32).collect();
        let minimal = shrink_inputs(&inputs, |_| true);
        assert_eq!(minimal.len(), 1, "got {minimal:?}");
    }

    #[test]
    fn test_shrink_is_deterministic() {
        let inputs: Vec<i32> = (0..80).collect();
        let run = || shrink_inputs(&inputs, |c| c.contains(&11) && c.contains(&64));
        assert_eq!(run(), run());
    }

    #[test]
    fn test_whole_sequence_needed_is_not_reduced() {
        // When every element genuinely matters, ddmin must return all of them.
        let inputs: Vec<i32> = (0..8).collect();
        let predicate = |c: &[i32]| c.len() == 8;
        let minimal = shrink_inputs(&inputs, predicate);
        assert_eq!(minimal, inputs);
        assert!(is_one_minimal(&minimal, predicate));
    }

    #[test]
    fn test_is_one_minimal_rejects_a_reducible_sequence() {
        let predicate = |c: &[i32]| c.contains(&3);
        assert!(!is_one_minimal(&[1, 2, 3, 4], predicate), "4 is removable");
        assert!(is_one_minimal(&[3], predicate));
        // A sequence that does not fail is not 1-minimal by definition.
        assert!(!is_one_minimal(&[1, 2], predicate));
    }

    #[test]
    fn test_predicate_call_count_stays_polynomial() {
        // Guard against a pathological blow-up: ddmin is O(n^2) worst case.
        let inputs: Vec<i32> = (0..64).collect();
        let mut calls = 0usize;
        let _ = shrink_inputs(&inputs, |c| {
            calls += 1;
            c.contains(&5)
        });
        let n = inputs.len();
        assert!(
            calls <= n * n,
            "ddmin made {calls} calls for n={n}, above the O(n^2) bound"
        );
    }

    // --- Simulation integration ---

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

    impl DetHash for Acc {
        fn det_hash(&self, h: &mut Fnv1a) {
            h.write_i32(self.total);
        }
    }

    #[test]
    fn test_shrink_simulation_inputs_finds_the_minimal_cause() {
        let inputs: Vec<i32> = vec![1, 1, 60, 1, 1, 50, 1, 1];
        let minimal = shrink_simulation_inputs(&Acc { total: 0 }, &inputs, |a: &Acc| a.total > 100);
        assert_eq!(minimal, vec![60, 50]);
    }

    #[test]
    fn test_shrink_simulation_inputs_never_leaks_state_between_candidates() {
        // Each candidate must run from a fresh clone; if state leaked, the
        // accumulated total would grow across probes and the predicate would
        // start firing for sequences that do not actually cause it.
        let inputs: Vec<i32> = vec![10; 20];
        let minimal = shrink_simulation_inputs(&Acc { total: 0 }, &inputs, |a: &Acc| a.total >= 30);
        assert_eq!(
            minimal.len(),
            3,
            "exactly three 10s are needed: {minimal:?}"
        );
        // And the original starting state was not mutated.
        let start = Acc { total: 0 };
        let _ = shrink_simulation_inputs(&start, &inputs, |a: &Acc| a.total >= 30);
        assert_eq!(start.total, 0);
    }

    #[test]
    fn test_shrink_simulation_result_reproduces_the_failure() {
        // End-to-end: replay the minimal sequence and confirm it really does
        // reach the bad state, using the sim adapter the kit already provides.
        let inputs: Vec<i32> = (1..=30).collect();
        let is_bad = |a: &Acc| a.total > 200;
        let minimal = shrink_simulation_inputs(&Acc { total: 0 }, &inputs, is_bad);
        let final_state = crate::sim::resimulate(&Acc { total: 0 }, &minimal);
        assert!(is_bad(&final_state), "minimal sequence must still fail");
        assert!(
            minimal.len() < inputs.len(),
            "it should have shrunk: {} vs {}",
            minimal.len(),
            inputs.len()
        );
    }
}
