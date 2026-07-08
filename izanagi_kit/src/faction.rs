//! Faction reputation — inter-faction standing and alignment queries.
//!
//! The kit could describe *what happens* in combat ([`combat`](crate::combat))
//! and *how* entities relate hierarchically ([`relations`](crate::relations)),
//! but had no notion of *why* two entities fight or cooperate — the faction
//! alignment layer every RPG roguelike needs. [`FactionMap<K>`] is that layer:
//! it tracks signed integer reputation between pairs of factions and exposes
//! `is_hostile`, `is_neutral`, and `is_friendly` threshold queries.
//!
//! ```
//! use izanagi_kit::faction::FactionMap;
//!
//! #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
//! enum F { Player, Goblin, Guard }
//!
//! let mut map: FactionMap<F> = FactionMap::new();
//! map.set(F::Player, F::Goblin, -80); // player and goblins are at war
//! map.set(F::Player, F::Guard,   60); // player is well-liked by guards
//!
//! assert!(map.is_hostile(F::Player, F::Goblin));
//! assert!(map.is_friendly(F::Player, F::Guard));
//! assert!(map.is_neutral(F::Goblin, F::Guard));   // no entry → 0 = neutral
//!
//! map.modify(F::Player, F::Goblin, 10);            // reputation improves
//! assert_eq!(map.get(F::Player, F::Goblin), -70);
//! ```
//!
//! ## Design choices
//!
//! Reputation is stored as `i32 ∈ [-100, 100]` (clamped on every write).
//! The sign is asymmetric by default — "A dislikes B" does not force "B
//! dislikes A" — because game designers frequently want one-sided hostility
//! (e.g. a neutral faction that the player has wronged). Symmetric operations
//! ([`FactionMap::set_symmetric`], [`FactionMap::modify_symmetric`]) are provided when bilateral
//! changes are wanted.
//!
//! Storage is a `BTreeMap<(K, K), i32>` (ordered, deterministic iteration).
//! Missing entries implicitly read as `0` (neutral). [`FactionMap`] implements
//! [`DetHash`], folding faction standings into the
//! replay checksum.
//!
//! ## Thresholds
//!
//! | Range | Alignment |
//! |-------|-----------|
//! | `< HOSTILE_THRESHOLD` (-25) | hostile |
//! | `≤ FRIENDLY_THRESHOLD` (25) | neutral |
//! | `> FRIENDLY_THRESHOLD` | friendly |

use crate::world_hash::{DetHash, Fnv1a};
use std::collections::BTreeMap;

/// Reputation values strictly below this threshold are considered hostile.
pub const HOSTILE_THRESHOLD: i32 = -25;
/// Reputation values strictly above this threshold are considered friendly.
pub const FRIENDLY_THRESHOLD: i32 = 25;
/// Minimum reputation value (most hostile).
pub const MIN_REP: i32 = -100;
/// Maximum reputation value (most friendly).
pub const MAX_REP: i32 = 100;

/// A table of signed integer reputation values between pairs of factions.
/// Missing pairs implicitly have a reputation of `0` (neutral).
#[derive(Clone, Debug, Default)]
pub struct FactionMap<K: Ord + Clone> {
    /// Ordered map: `(from, to) -> reputation`. The ordering is the product
    /// order on `K`, which must implement `Ord` so iteration is deterministic.
    standings: BTreeMap<(K, K), i32>,
}

impl<K: Ord + Clone> FactionMap<K> {
    /// Create an empty faction map — all pairs neutral.
    pub fn new() -> Self {
        FactionMap {
            standings: BTreeMap::new(),
        }
    }

    /// The reputation of `from` toward `to`, in `[MIN_REP, MAX_REP]`. Missing
    /// entries return `0` (neutral).
    pub fn get(&self, from: K, to: K) -> i32 {
        *self.standings.get(&(from, to)).unwrap_or(&0)
    }

    /// Set the reputation of `from` toward `to`, clamped to `[MIN_REP, MAX_REP]`.
    /// Storing `0` is allowed (explicit neutral). Returns the previous value.
    pub fn set(&mut self, from: K, to: K, value: i32) -> i32 {
        let clamped = value.clamp(MIN_REP, MAX_REP);
        self.standings.insert((from, to), clamped).unwrap_or(0)
    }

    /// Set reputation *bidirectionally*: `from→to` and `to→from` both become
    /// `value` (clamped). Returns `(old_from_to, old_to_from)`.
    pub fn set_symmetric(&mut self, a: K, b: K, value: i32) -> (i32, i32) {
        let fwd = self.set(a.clone(), b.clone(), value);
        let rev = self.set(b, a, value);
        (fwd, rev)
    }

    /// Add `delta` to the reputation of `from` toward `to` (saturating, then
    /// clamped to `[MIN_REP, MAX_REP]`). Returns the new value.
    pub fn modify(&mut self, from: K, to: K, delta: i32) -> i32 {
        let current = self.get(from.clone(), to.clone());
        let new_val = current.saturating_add(delta).clamp(MIN_REP, MAX_REP);
        self.set(from, to, new_val);
        new_val
    }

    /// Add `delta` bidirectionally. Returns `(new_from_to, new_to_from)`.
    pub fn modify_symmetric(&mut self, a: K, b: K, delta: i32) -> (i32, i32) {
        let fwd = self.modify(a.clone(), b.clone(), delta);
        let rev = self.modify(b, a, delta);
        (fwd, rev)
    }

    /// Remove a reputation entry, reverting that pair to implicit neutral (`0`).
    /// Returns the removed value, or `0` if the pair had no explicit entry.
    pub fn remove(&mut self, from: K, to: K) -> i32 {
        self.standings.remove(&(from, to)).unwrap_or(0)
    }

    /// `true` if the reputation of `from` toward `to` is below [`HOSTILE_THRESHOLD`].
    #[inline]
    pub fn is_hostile(&self, from: K, to: K) -> bool {
        self.get(from, to) < HOSTILE_THRESHOLD
    }

    /// `true` if the reputation is strictly above [`FRIENDLY_THRESHOLD`].
    #[inline]
    pub fn is_friendly(&self, from: K, to: K) -> bool {
        self.get(from, to) > FRIENDLY_THRESHOLD
    }

    /// `true` if the reputation is in `[HOSTILE_THRESHOLD, FRIENDLY_THRESHOLD]`.
    #[inline]
    pub fn is_neutral(&self, from: K, to: K) -> bool {
        let r = self.get(from, to);
        (HOSTILE_THRESHOLD..=FRIENDLY_THRESHOLD).contains(&r)
    }

    /// The number of explicitly stored entries (pairs that are not implicitly 0).
    pub fn entry_count(&self) -> usize {
        self.standings.len()
    }

    /// Iterate over `(from, to, reputation)` in canonical `BTreeMap` order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &K, i32)> {
        self.standings.iter().map(|((f, t), &r)| (f, t, r))
    }
}

impl<K: Ord + Clone + DetHash> DetHash for FactionMap<K> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.standings.len() as u32);
        for ((from, to), &rep) in &self.standings {
            from.det_hash(hasher);
            to.det_hash(hasher);
            hasher.write_i32(rep);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    type FM = FactionMap<u32>;

    #[test]
    fn test_missing_pair_is_neutral_zero() {
        let fm = FM::new();
        assert_eq!(fm.get(1, 2), 0);
        assert!(fm.is_neutral(1, 2));
        assert!(!fm.is_hostile(1, 2));
        assert!(!fm.is_friendly(1, 2));
    }

    #[test]
    fn test_set_clamped_and_returned() {
        let mut fm = FM::new();
        let prev = fm.set(1, 2, 200); // clamps to 100
        assert_eq!(prev, 0, "previous was neutral");
        assert_eq!(fm.get(1, 2), MAX_REP);
        let prev2 = fm.set(1, 2, -999); // clamps to -100
        assert_eq!(prev2, MAX_REP);
        assert_eq!(fm.get(1, 2), MIN_REP);
    }

    #[test]
    fn test_set_asymmetry() {
        let mut fm = FM::new();
        fm.set(1, 2, -80);
        assert_eq!(fm.get(1, 2), -80);
        assert_eq!(fm.get(2, 1), 0, "reverse direction is not affected");
    }

    #[test]
    fn test_set_symmetric_bidirectional() {
        let mut fm = FM::new();
        fm.set_symmetric(1, 2, -60);
        assert_eq!(fm.get(1, 2), -60);
        assert_eq!(fm.get(2, 1), -60);
    }

    #[test]
    fn test_modify_saturating_clamped() {
        let mut fm = FM::new();
        fm.set(1, 2, 90);
        let new_val = fm.modify(1, 2, 50); // would reach 140, clamped to 100
        assert_eq!(new_val, MAX_REP);
        fm.set(1, 2, -90);
        let new_val = fm.modify(1, 2, -50); // would reach -140, clamped to -100
        assert_eq!(new_val, MIN_REP);
    }

    #[test]
    fn test_modify_symmetric() {
        let mut fm = FM::new();
        fm.set_symmetric(1, 2, 0);
        let (fwd, rev) = fm.modify_symmetric(1, 2, 30);
        assert_eq!(fwd, 30);
        assert_eq!(rev, 30);
    }

    #[test]
    fn test_remove_reverts_to_zero() {
        let mut fm = FM::new();
        fm.set(1, 2, 50);
        assert_eq!(fm.remove(1, 2), 50);
        assert_eq!(fm.get(1, 2), 0);
        assert_eq!(fm.remove(1, 2), 0, "second remove returns 0");
    }

    #[test]
    fn test_threshold_queries() {
        let mut fm = FM::new();
        fm.set(1, 2, HOSTILE_THRESHOLD - 1); // just below hostile
        assert!(fm.is_hostile(1, 2));
        fm.set(1, 2, HOSTILE_THRESHOLD); // at boundary: neutral
        assert!(!fm.is_hostile(1, 2));
        assert!(fm.is_neutral(1, 2));
        fm.set(1, 2, FRIENDLY_THRESHOLD + 1); // just above friendly
        assert!(fm.is_friendly(1, 2));
        fm.set(1, 2, FRIENDLY_THRESHOLD); // at boundary: neutral
        assert!(!fm.is_friendly(1, 2));
        assert!(fm.is_neutral(1, 2));
    }

    #[test]
    fn test_iter_is_ordered() {
        let mut fm = FM::new();
        fm.set(3, 1, 10);
        fm.set(1, 2, 20);
        fm.set(2, 3, -30);
        let keys: Vec<(u32, u32)> = fm.iter().map(|(&f, &t, _)| (f, t)).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "iter must be in BTreeMap canonical order");
    }

    #[test]
    fn test_entry_count() {
        let mut fm = FM::new();
        assert_eq!(fm.entry_count(), 0);
        fm.set(1, 2, 50);
        assert_eq!(fm.entry_count(), 1);
        fm.set_symmetric(3, 4, -60);
        assert_eq!(fm.entry_count(), 3);
        fm.remove(1, 2);
        assert_eq!(fm.entry_count(), 2);
    }

    #[test]
    fn test_det_hash_canonical_and_sensitive() {
        let mut a = FM::new();
        a.set(1, 2, 50);
        let mut b = FM::new();
        b.set(1, 2, 50);
        assert_eq!(hash_state(&a), hash_state(&b), "same state, same hash");
        b.set(1, 2, 51);
        assert_ne!(
            hash_state(&a),
            hash_state(&b),
            "different value, different hash"
        );
        let mut c = FM::new();
        c.set(2, 1, 50); // reversed direction
        assert_ne!(hash_state(&a), hash_state(&c), "direction matters in hash");
    }
}
