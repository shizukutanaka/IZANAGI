//! Item identification — scrambled per-seed appearances, revealed on demand.
//!
//! Rogue, NetHack, and Angband all share a defining mechanic that nothing in
//! the kit could express: a potion of healing looks like a "swirly potion"
//! this game and a "fizzy potion" the next — the mapping from true item kind
//! to displayed appearance is **shuffled once per seed** and hidden from the
//! player until identified (by use, by a scroll, by price-ID). [`random_table`](crate::random_table)
//! draws *values* from a weighted pool, but nothing built a **scrambled
//! bijection** between two fixed sets plus a per-kind reveal flag —
//! [`Identification`] is that primitive.
//!
//! ```
//! use izanagi_kit::identify::Identification;
//! use izanagi_kit::rng::SplitMix64;
//!
//! #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
//! enum Potion { Healing, Poison, Strength }
//!
//! let mut rng = SplitMix64::new(42);
//! let kinds = [Potion::Healing, Potion::Poison, Potion::Strength];
//! let labels = ["swirly", "fizzy", "murky"];
//! let mut id = Identification::new(&kinds, &labels, &mut rng);
//!
//! // Before identifying, the player only sees the scrambled label.
//! assert!(!id.is_identified(Potion::Healing));
//! let seen_label = *id.appearance(Potion::Healing).unwrap();
//!
//! assert!(id.identify(Potion::Healing), "newly identified");
//! assert!(!id.identify(Potion::Healing), "already identified — no-op");
//! assert!(id.is_identified(Potion::Healing));
//! // Identifying never changes what the label *was* — only whether it's revealed.
//! assert_eq!(id.appearance(Potion::Healing), Some(&seen_label));
//! ```
//!
//! ## Design
//!
//! - The constructor sorts `kinds` (for input-order independence — the same
//!   `kinds`/`labels`/seed always produce the same mapping regardless of the
//!   order `kinds` was passed in) and shuffles `labels` via
//!   [`SplitMix64::shuffle`](crate::rng::SplitMix64::shuffle) — the same
//!   Fisher-Yates already used throughout the kit — before zipping the two.
//!   `labels` must have at least as many entries as `kinds` has distinct
//!   values.
//! - `identified: BTreeSet<T>` mirrors [`meta::MetaProgress`](crate::meta::MetaProgress)'s
//!   idempotent unlock flags, but deliberately lives in its own type: item
//!   identification resets every run, while `MetaProgress` explicitly never
//!   does. Reusing one type for both would blur that lifecycle distinction.
//! - [`appearance`](Identification::appearance) always returns the assigned
//!   label, identified or not — callers show the true name only when
//!   [`is_identified`](Identification::is_identified) is `true`, and the
//!   label otherwise. The label itself never changes after construction.

use crate::rng::SplitMix64;
use crate::world_hash::{DetHash, Fnv1a};
use std::collections::{BTreeMap, BTreeSet};

/// A scrambled `kind -> label` assignment plus per-kind identification flags.
#[derive(Clone, Debug)]
pub struct Identification<T: Ord + Clone, L: Clone> {
    appearance: BTreeMap<T, L>,
    identified: BTreeSet<T>,
}

impl<T: Ord + Clone, L: Clone> Identification<T, L> {
    /// Build a scrambled assignment: `kinds` (deduplicated, sorted for
    /// input-order independence) each get a distinct label drawn from a
    /// Fisher-Yates shuffle of `labels`. Panics if `labels` has fewer entries
    /// than `kinds` has distinct values — there aren't enough labels to
    /// assign one to every kind.
    pub fn new(kinds: &[T], labels: &[L], rng: &mut SplitMix64) -> Self {
        let mut sorted_kinds: Vec<T> = kinds.to_vec();
        sorted_kinds.sort();
        sorted_kinds.dedup();
        assert!(
            labels.len() >= sorted_kinds.len(),
            "not enough labels ({}) to assign to every kind ({})",
            labels.len(),
            sorted_kinds.len()
        );
        let mut shuffled_labels: Vec<L> = labels.to_vec();
        rng.shuffle(&mut shuffled_labels);
        let appearance: BTreeMap<T, L> = sorted_kinds.into_iter().zip(shuffled_labels).collect();
        Identification {
            appearance,
            identified: BTreeSet::new(),
        }
    }

    /// The scrambled label assigned to `kind`, or `None` if `kind` was not
    /// part of the set this instance was built with. Available regardless of
    /// identification status — this is what an unidentified item displays.
    pub fn appearance(&self, kind: T) -> Option<&L> {
        self.appearance.get(&kind)
    }

    /// Mark `kind` as identified. Returns `true` if it was not already
    /// identified. A no-op (returns `false`, no state change) for a `kind`
    /// outside the original set, or one already identified.
    pub fn identify(&mut self, kind: T) -> bool {
        if !self.appearance.contains_key(&kind) {
            return false;
        }
        self.identified.insert(kind)
    }

    /// `true` if `kind` has been identified.
    pub fn is_identified(&self, kind: T) -> bool {
        self.identified.contains(&kind)
    }

    /// The number of distinct kinds identified so far.
    pub fn identified_count(&self) -> usize {
        self.identified.len()
    }

    /// The total number of distinct kinds this instance tracks (post-dedup).
    pub fn total_count(&self) -> usize {
        self.appearance.len()
    }
}

impl<T: Ord + Clone + DetHash, L: Clone + DetHash> DetHash for Identification<T, L> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.appearance.len() as u32);
        for (kind, label) in &self.appearance {
            kind.det_hash(hasher);
            label.det_hash(hasher);
        }
        hasher.write_u32(self.identified.len() as u32);
        for kind in &self.identified {
            kind.det_hash(hasher);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[test]
    fn test_new_assigns_every_kind_a_label() {
        let mut rng = SplitMix64::new(1);
        let kinds = [1u32, 2, 3];
        let labels = [10u32, 20, 30];
        let id = Identification::new(&kinds, &labels, &mut rng);
        assert_eq!(id.total_count(), 3);
        for &k in &kinds {
            assert!(id.appearance(k).is_some());
        }
    }

    #[test]
    fn test_appearance_unknown_kind_is_none() {
        let mut rng = SplitMix64::new(1);
        let id = Identification::new(&[1u32], &[10u32], &mut rng);
        assert_eq!(id.appearance(999), None);
    }

    #[test]
    fn test_assignment_is_a_bijection_with_distinct_labels() {
        let mut rng = SplitMix64::new(7);
        let kinds: Vec<u32> = (0..10).collect();
        let labels: Vec<u32> = (100..110).collect();
        let id = Identification::new(&kinds, &labels, &mut rng);
        let mut assigned: Vec<u32> = kinds.iter().map(|&k| *id.appearance(k).unwrap()).collect();
        assigned.sort();
        assigned.dedup();
        assert_eq!(assigned.len(), 10, "every kind gets a distinct label");
    }

    #[test]
    fn test_deterministic_given_same_seed() {
        let kinds = [1u32, 2, 3, 4, 5];
        let labels = [10u32, 20, 30, 40, 50];
        let mut rng_a = SplitMix64::new(0xABCD);
        let a = Identification::new(&kinds, &labels, &mut rng_a);
        let mut rng_b = SplitMix64::new(0xABCD);
        let b = Identification::new(&kinds, &labels, &mut rng_b);
        for &k in &kinds {
            assert_eq!(a.appearance(k), b.appearance(k));
        }
    }

    #[test]
    fn test_input_kind_order_does_not_affect_mapping() {
        let labels = [10u32, 20, 30];
        let mut rng_a = SplitMix64::new(99);
        let a = Identification::new(&[1u32, 2, 3], &labels, &mut rng_a);
        let mut rng_b = SplitMix64::new(99);
        let b = Identification::new(&[3u32, 1, 2], &labels, &mut rng_b);
        for k in [1u32, 2, 3] {
            assert_eq!(
                a.appearance(k),
                b.appearance(k),
                "input order must not matter"
            );
        }
    }

    #[test]
    fn test_duplicate_kinds_are_deduped() {
        let mut rng = SplitMix64::new(1);
        let id = Identification::new(&[1u32, 1, 2, 2, 2], &[10u32, 20], &mut rng);
        assert_eq!(id.total_count(), 2);
    }

    #[test]
    #[should_panic(expected = "not enough labels")]
    fn test_panics_when_not_enough_labels() {
        let mut rng = SplitMix64::new(1);
        let _ = Identification::new(&[1u32, 2, 3], &[10u32], &mut rng);
    }

    #[test]
    fn test_identify_returns_true_first_time() {
        let mut rng = SplitMix64::new(1);
        let mut id = Identification::new(&[1u32], &[10u32], &mut rng);
        assert!(!id.is_identified(1));
        assert!(id.identify(1));
        assert!(id.is_identified(1));
    }

    #[test]
    fn test_identify_is_idempotent() {
        let mut rng = SplitMix64::new(1);
        let mut id = Identification::new(&[1u32], &[10u32], &mut rng);
        assert!(id.identify(1));
        assert!(!id.identify(1), "already identified, no-op");
        assert_eq!(id.identified_count(), 1);
    }

    #[test]
    fn test_identify_unknown_kind_is_noop() {
        let mut rng = SplitMix64::new(1);
        let mut id = Identification::new(&[1u32], &[10u32], &mut rng);
        assert!(!id.identify(999));
        assert_eq!(id.identified_count(), 0);
    }

    #[test]
    fn test_identify_does_not_change_appearance() {
        let mut rng = SplitMix64::new(1);
        let mut id = Identification::new(&[1u32, 2], &[10u32, 20], &mut rng);
        let before = *id.appearance(1).unwrap();
        id.identify(1);
        assert_eq!(
            id.appearance(1),
            Some(&before),
            "label is fixed at construction"
        );
    }

    #[test]
    fn test_identified_count_tracks_distinct_kinds() {
        let mut rng = SplitMix64::new(1);
        let mut id = Identification::new(&[1u32, 2, 3], &[10u32, 20, 30], &mut rng);
        id.identify(1);
        id.identify(2);
        id.identify(1); // repeat, must not double-count
        assert_eq!(id.identified_count(), 2);
    }

    #[test]
    fn test_total_count_reflects_kind_set_size() {
        let mut rng = SplitMix64::new(1);
        let id = Identification::new(&[1u32, 2, 3, 4], &[10u32, 20, 30, 40, 50], &mut rng);
        assert_eq!(
            id.total_count(),
            4,
            "extra labels beyond kind count are simply unused"
        );
    }

    #[test]
    fn test_det_hash_same_seed_same_hash() {
        let kinds = [1u32, 2, 3];
        let labels = [10u32, 20, 30];
        let mut rng_a = SplitMix64::new(55);
        let a = Identification::new(&kinds, &labels, &mut rng_a);
        let mut rng_b = SplitMix64::new(55);
        let b = Identification::new(&kinds, &labels, &mut rng_b);
        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_det_hash_changes_after_identify() {
        let mut rng = SplitMix64::new(1);
        let mut id = Identification::new(&[1u32, 2], &[10u32, 20], &mut rng);
        let before = hash_state(&id);
        id.identify(1);
        assert_ne!(hash_state(&id), before);
    }

    #[test]
    fn test_det_hash_stable_on_repeat_identify() {
        let mut rng = SplitMix64::new(1);
        let mut id = Identification::new(&[1u32], &[10u32], &mut rng);
        id.identify(1);
        let after_first = hash_state(&id);
        id.identify(1); // no-op
        assert_eq!(hash_state(&id), after_first);
    }

    #[test]
    fn test_det_hash_stable_on_noop_identify() {
        let mut rng = SplitMix64::new(1);
        let id_before_state = {
            let mut id = Identification::new(&[1u32], &[10u32], &mut rng);
            let before = hash_state(&id);
            id.identify(999); // unknown kind, no-op
            (id, before)
        };
        assert_eq!(hash_state(&id_before_state.0), id_before_state.1);
    }

    #[test]
    fn test_different_seeds_usually_scramble_differently() {
        // With a wide-enough label pool, two distinct seeds landing on the
        // identical permutation is astronomically unlikely (the same caveat
        // `rng::tests::test_shuffle_changes_order` documents: a real but
        // negligible false-failure probability with a fixed seed pair).
        let kinds: Vec<u32> = (0..8).collect();
        let labels: Vec<u32> = (100..108).collect();
        let mut rng_a = SplitMix64::new(1);
        let a = Identification::new(&kinds, &labels, &mut rng_a);
        let mut rng_b = SplitMix64::new(2);
        let b = Identification::new(&kinds, &labels, &mut rng_b);
        let differs = kinds.iter().any(|&k| a.appearance(k) != b.appearance(k));
        assert!(
            differs,
            "suspicious: two different seeds produced an identical mapping"
        );
    }
}
