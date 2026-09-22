//! Vector clocks — Lamport/Fidge–Mattern causality tracking for
//! replicated simulation state. Each actor owns one counter; a clock
//! is `Before` another only when every component is ≤ and at least one
//! is <, so *concurrent* histories are distinguishable — the exact
//! predicate a lockstep desync report needs ("did peer A's frame see
//! peer B's input?").
//!
//! ```
//! use izanagi_kit::vclock::{VClock, Order};
//! let mut a = VClock::new();
//! let mut b = VClock::new();
//! a.tick(1);                       // A's own event
//! let snap = a.clone();
//! b.merge(&snap);                  // B receives A's state
//! b.tick(2);
//! a.tick(1);                       // A does something B hasn't seen
//! assert_eq!(snap.compare(&b), Order::Before);
//! assert_eq!(a.compare(&b), Order::Concurrent);
//! ```

use std::collections::BTreeMap;

/// Partial order between two [`VClock`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    /// Every component ≤ and at least one < — strict happens-before.
    Before,
    /// The reverse — strict happens-after.
    After,
    /// Identical clocks.
    Equal,
    /// Each clock has at least one strictly-greater component — the
    /// histories diverged; neither causally precedes the other.
    Concurrent,
}

/// A vector clock: actor id → logical counter. `BTreeMap` storage keeps
/// iteration and serialization order deterministic.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VClock {
    c: BTreeMap<u32, u64>,
}

impl VClock {
    /// An empty clock (bottom of the causal order).
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a local event by `actor`.
    pub fn tick(&mut self, actor: u32) {
        *self.c.entry(actor).or_insert(0) += 1;
    }

    /// Merge `other` (a receive): elementwise maximum — the join of the
    /// two clocks in the causal lattice.
    pub fn merge(&mut self, other: &VClock) {
        for (&a, &v) in &other.c {
            let e = self.c.entry(a).or_insert(0);
            if *e < v {
                *e = v;
            }
        }
    }

    /// `actor`'s counter (absent = 0).
    pub fn get(&self, actor: u32) -> u64 {
        self.c.get(&actor).copied().unwrap_or(0)
    }

    /// Number of actors with nonzero counters.
    pub fn len(&self) -> usize {
        self.c.len()
    }

    /// Whether no events have been recorded.
    pub fn is_empty(&self) -> bool {
        self.c.is_empty()
    }

    /// The causal order between `self` and `other`.
    pub fn compare(&self, other: &VClock) -> Order {
        let mut less = false;
        let mut greater = false;
        // Union of key sets — BTreeMap iteration is sorted so the walk
        // is deterministic.
        for (&a, &x) in &self.c {
            let y = other.get(a);
            if x < y {
                less = true;
            } else if x > y {
                greater = true;
            }
        }
        for (&a, &y) in &other.c {
            if !self.c.contains_key(&a) && y > 0 {
                less = true;
            }
        }
        match (less, greater) {
            (true, false) => Order::Before,
            (false, true) => Order::After,
            (false, false) => Order::Equal,
            (true, true) => Order::Concurrent,
        }
    }

    /// `self` strictly happened before `other`.
    pub fn happened_before(&self, other: &VClock) -> bool {
        self.compare(other) == Order::Before
    }

    /// The two clocks are causally unrelated (diverged histories).
    pub fn concurrent(&self, other: &VClock) -> bool {
        self.compare(other) == Order::Concurrent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use std::collections::BTreeSet;

    #[test]
    fn axioms_hold_on_random_clocks() {
        let mut rng = SplitMix64::new(0xCA05);
        // Random clocks by random (actor, count) events.
        let mut clocks = Vec::new();
        for _ in 0..40 {
            let mut v = VClock::new();
            for _ in 0..rng.below(6) {
                for _ in 0..1 + rng.below(4) {
                    v.tick(rng.below(5));
                }
            }
            clocks.push(v);
        }
        for (i, a) in clocks.iter().enumerate() {
            // Reflexive: compare(self) == Equal.
            assert_eq!(a.compare(a), Order::Equal);
            for (j, b) in clocks.iter().enumerate() {
                let ab = a.compare(b);
                let ba = b.compare(a);
                // Antisymmetry: Before↔After dual, Equal/Concurrent symmetric.
                assert_eq!(
                    ab,
                    match ba {
                        Order::Before => Order::After,
                        Order::After => Order::Before,
                        o => o,
                    },
                    "{i} vs {j}: {ab:?}/{ba:?}"
                );
                if i == j {
                    assert_eq!(ab, Order::Equal);
                }
            }
        }
        // Transitivity (sampled): a<b ∧ b<c ⟹ a<c.
        for a in &clocks {
            for b in &clocks {
                if !a.happened_before(b) {
                    continue;
                }
                for c in &clocks {
                    if b.happened_before(c) {
                        assert!(a.happened_before(c));
                    }
                }
            }
        }
    }

    #[test]
    fn merge_is_least_upper_bound() {
        let mut rng = SplitMix64::new(0xBEEF);
        for _ in 0..300 {
            let mut a = VClock::new();
            let mut b = VClock::new();
            for _ in 0..1 + rng.below(8) {
                a.tick(rng.below(4));
                b.tick(rng.below(4));
            }
            let mut m = a.clone();
            m.merge(&b);
            // Join: m ≥ a ∧ m ≥ b.
            assert!(matches!(m.compare(&a), Order::After | Order::Equal));
            assert!(matches!(m.compare(&b), Order::After | Order::Equal));
            // Least: each component is the max.
            let actors: BTreeSet<u32> = a.c.keys().chain(b.c.keys()).copied().collect();
            for act in actors {
                assert_eq!(m.get(act), a.get(act).max(b.get(act)));
            }
            // Merge is commutative/idempotent.
            let mut m2 = b.clone();
            m2.merge(&a);
            assert_eq!(m, m2);
            m2.merge(&a);
            assert_eq!(m, m2);
        }
    }

    #[test]
    fn causality_simulation_respects_message_flow() {
        // Simulate actors exchanging snapshots; every received snapshot
        // must be ≤ the receiver's clock after merge — the content
        // definition of "the receiver saw this history".
        let mut rng = SplitMix64::new(0xAB1E);
        let actors = 4u32;
        let mut clocks = vec![VClock::new(); actors as usize];
        for _ in 0..2_000 {
            let a = rng.below(actors);
            match rng.below(4) {
                0 => {
                    // Send snapshot a→b.
                    let b = (a + 1 + rng.below(actors - 1)) % actors;
                    let snap = clocks[a as usize].clone();
                    clocks[b as usize].merge(&snap);
                    assert!(matches!(
                        snap.compare(&clocks[b as usize]),
                        Order::Before | Order::Equal
                    ));
                }
                _ => clocks[a as usize].tick(a),
            }
            // A clock can never be strictly-before a copy of itself
            // after its own tick.
            let c = &clocks[a as usize];
            assert_eq!(c.compare(c), Order::Equal);
        }
    }

    #[test]
    fn degenerate_cases() {
        let e = VClock::new();
        assert!(e.is_empty());
        assert_eq!(e.len(), 0);
        assert_eq!(e.get(9), 0);
        let mut a = VClock::new();
        a.tick(3);
        assert_eq!(e.compare(&a), Order::Before);
        assert_eq!(a.compare(&e), Order::After);
        assert!(!e.concurrent(&a));
        // Disjoint actors → concurrent.
        let mut x = VClock::new();
        let mut y = VClock::new();
        x.tick(1);
        y.tick(2);
        assert_eq!(x.compare(&y), Order::Concurrent);
        assert!(x.concurrent(&y));
    }
}
