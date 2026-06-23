//! Threat / aggro tables — per-combatant target selection.
//!
//! [`faction`](crate::faction) answers a *group-level* question — "are these
//! two factions hostile?" — but not the *encounter-level* one every tactical
//! roguelike needs: "given that I am hostile to the entire party, **which
//! member do I attack right now**?" [`influence`](crate::influence) answers a
//! *spatial* version ("where is danger on the grid?"), and
//! [`behavior`](crate::behavior) chooses *what action* to take, but neither
//! accumulates per-target hostility over time. That accumulation — built up by
//! dealing damage, healing allies, or taunting, and decaying when out of
//! combat — is the threat axis. [`ThreatTable<K>`] is that layer.
//!
//! ```
//! use izanagi_kit::threat::ThreatTable;
//!
//! #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
//! enum Hero { Warrior, Mage, Cleric }
//!
//! let mut t: ThreatTable<Hero> = ThreatTable::new();
//! t.add(Hero::Warrior, 50);   // warrior hits the monster
//! t.add(Hero::Mage, 80);      // mage nukes it harder
//! t.add(Hero::Cleric, 30);    // cleric heals (generates some threat)
//!
//! // The monster attacks whoever it hates most.
//! assert_eq!(t.top_target(), Some(&Hero::Mage));
//!
//! // The warrior taunts: forced to the top of the table.
//! t.taunt(Hero::Warrior, 1);
//! assert_eq!(t.top_target(), Some(&Hero::Warrior));
//!
//! // Out of line of sight for a few turns — threat cools off.
//! t.decay_all(40);
//! assert_eq!(t.threat_of(Hero::Cleric), 0); // dropped out entirely
//! ```
//!
//! ## Design
//!
//! Threat is a non-negative `i32` per source key, stored in a
//! `BTreeMap<K, i32>` for deterministic iteration and hashing. Entries that
//! reach `0` are pruned, so [`is_empty`](ThreatTable::is_empty) means "nobody
//! on my threat list" and [`top_target`](ThreatTable::top_target) returns
//! `None` only when there is genuinely no one to hate.
//!
//! **Tie-breaking is deterministic**: when two sources share the highest
//! threat, the one with the smallest key (`K: Ord`) wins. This makes target
//! selection replay-safe regardless of insertion order.
//!
//! [`ThreatTable`] implements [`DetHash`](crate::world_hash::DetHash), folding
//! the sorted `(key, threat)` pairs into the replay checksum.

use crate::world_hash::{DetHash, Fnv1a};
use std::collections::BTreeMap;

/// A per-combatant aggro table: how much each source key has threatened the
/// owner. Threat is a non-negative integer; zero-threat sources are pruned.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreatTable<K: Ord + Clone> {
    threats: BTreeMap<K, i32>,
}

impl<K: Ord + Clone> ThreatTable<K> {
    /// Create an empty threat table.
    pub fn new() -> Self {
        ThreatTable {
            threats: BTreeMap::new(),
        }
    }

    /// The current threat for `source`, or `0` if it is not on the table.
    pub fn threat_of(&self, source: K) -> i32 {
        self.threats.get(&source).copied().unwrap_or(0)
    }

    /// Set `source`'s threat to an exact value (clamped to `>= 0`).
    /// A value of `0` removes the entry.
    pub fn set(&mut self, source: K, value: i32) {
        let v = value.max(0);
        if v == 0 {
            self.threats.remove(&source);
        } else {
            self.threats.insert(source, v);
        }
    }

    /// Add `amount` to `source`'s threat (saturating, floored at `0`).
    /// Negative `amount` reduces threat. Pruned if the result is `0`.
    pub fn add(&mut self, source: K, amount: i32) {
        let next = self.threat_of(source.clone()).saturating_add(amount).max(0);
        self.set(source, next);
    }

    /// Reduce `source`'s threat by `amount` (floored at `0`, pruned if zero).
    /// Convenience for `add(source, -amount)` with a non-negative argument.
    pub fn reduce(&mut self, source: K, amount: i32) {
        self.add(source, -amount.max(0));
    }

    /// Remove `source` from the table entirely (e.g. it died or fled).
    /// Returns the threat it had, or `0` if it was not present.
    pub fn remove(&mut self, source: K) -> i32 {
        self.threats.remove(&source).unwrap_or(0)
    }

    /// Subtract a flat `amount` from **every** source's threat (saturating,
    /// floored at `0`). Sources that hit `0` are pruned. Models a per-turn
    /// cool-off when the owner is not actively being attacked.
    pub fn decay_all(&mut self, amount: i32) {
        let a = amount.max(0);
        if a == 0 {
            return;
        }
        self.threats.retain(|_, v| {
            *v = (*v).saturating_sub(a).max(0);
            *v > 0
        });
    }

    /// Scale every source's threat by `(1000 - per_mille) / 1000` using integer
    /// arithmetic (e.g. `per_mille = 100` decays by 10% each call). Sources
    /// that round down to `0` are pruned. `per_mille` is clamped to `0..=1000`.
    pub fn decay_all_permille(&mut self, per_mille: u32) {
        let keep = 1000u64.saturating_sub(per_mille.min(1000) as u64);
        if keep == 1000 {
            return;
        }
        self.threats.retain(|_, v| {
            *v = ((*v as i64 * keep as i64) / 1000) as i32;
            *v > 0
        });
    }

    /// The source with the highest threat. Ties are broken deterministically in
    /// favour of the smallest key. `None` if the table is empty.
    pub fn top_target(&self) -> Option<&K> {
        self.top_entry().map(|(k, _)| k)
    }

    /// The highest-threat `(source, threat)` pair, with the same deterministic
    /// tie-break as [`top_target`](ThreatTable::top_target). `None` if empty.
    pub fn top_entry(&self) -> Option<(&K, i32)> {
        // BTreeMap iterates in ascending key order; using strict `>` keeps the
        // first (smallest-key) maximum, making the tie-break deterministic.
        let mut best: Option<(&K, i32)> = None;
        for (k, &v) in &self.threats {
            match best {
                Some((_, bv)) if v > bv => best = Some((k, v)),
                None => best = Some((k, v)),
                _ => {}
            }
        }
        best
    }

    /// Force `source` to the top of the table by setting its threat to the
    /// current maximum plus `margin` (at least `margin`). Classic "taunt":
    /// guarantees `top_target()` returns `source` afterwards when `margin >= 1`.
    pub fn taunt(&mut self, source: K, margin: i32) {
        let max = self.top_entry().map(|(_, v)| v).unwrap_or(0);
        let m = margin.max(0);
        self.set(source, max.saturating_add(m).max(m));
    }

    /// Remove every source from the table (combat ended).
    pub fn clear(&mut self) {
        self.threats.clear();
    }

    /// The number of sources currently on the table.
    pub fn len(&self) -> usize {
        self.threats.len()
    }

    /// `true` if no source is on the table.
    pub fn is_empty(&self) -> bool {
        self.threats.is_empty()
    }

    /// Iterate over `(source, threat)` pairs in ascending key order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, i32)> {
        self.threats.iter().map(|(k, &v)| (k, v))
    }

    /// The total threat summed across all sources (saturating).
    pub fn total(&self) -> i64 {
        self.threats.values().map(|&v| v as i64).sum()
    }
}

impl<K: Ord + Clone + DetHash> DetHash for ThreatTable<K> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.threats.len() as u32);
        for (k, &v) in &self.threats {
            k.det_hash(hasher);
            hasher.write_i32(v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[test]
    fn test_new_is_empty() {
        let t: ThreatTable<u32> = ThreatTable::new();
        assert!(t.is_empty());
        assert_eq!(t.len(), 0);
        assert_eq!(t.top_target(), None);
        assert_eq!(t.threat_of(1), 0);
    }

    #[test]
    fn test_add_and_top_target() {
        let mut t = ThreatTable::new();
        t.add(1u32, 50);
        t.add(2, 80);
        t.add(3, 30);
        assert_eq!(t.top_target(), Some(&2));
        assert_eq!(t.threat_of(2), 80);
        assert_eq!(t.len(), 3);
    }

    #[test]
    fn test_add_accumulates() {
        let mut t = ThreatTable::new();
        t.add(1u32, 20);
        t.add(1, 35);
        assert_eq!(t.threat_of(1), 55);
    }

    #[test]
    fn test_add_negative_floors_at_zero_and_prunes() {
        let mut t = ThreatTable::new();
        t.add(1u32, 30);
        t.add(1, -50); // would go negative
        assert_eq!(t.threat_of(1), 0);
        assert!(t.is_empty(), "zero threat is pruned");
    }

    #[test]
    fn test_set_zero_removes() {
        let mut t = ThreatTable::new();
        t.set(1u32, 40);
        t.set(1, 0);
        assert!(t.is_empty());
    }

    #[test]
    fn test_set_clamps_negative() {
        let mut t = ThreatTable::new();
        t.set(1u32, -100);
        assert_eq!(t.threat_of(1), 0);
        assert!(t.is_empty());
    }

    #[test]
    fn test_reduce() {
        let mut t = ThreatTable::new();
        t.add(1u32, 100);
        t.reduce(1, 30);
        assert_eq!(t.threat_of(1), 70);
        t.reduce(1, 1000);
        assert_eq!(t.threat_of(1), 0);
    }

    #[test]
    fn test_remove_returns_prior() {
        let mut t = ThreatTable::new();
        t.add(7u32, 42);
        assert_eq!(t.remove(7), 42);
        assert_eq!(t.remove(7), 0, "second remove → 0");
    }

    #[test]
    fn test_decay_all_flat() {
        let mut t = ThreatTable::new();
        t.add(1u32, 50);
        t.add(2, 20);
        t.add(3, 100);
        t.decay_all(30);
        assert_eq!(t.threat_of(1), 20);
        assert_eq!(t.threat_of(2), 0); // 20 - 30 → pruned
        assert_eq!(t.threat_of(3), 70);
        assert_eq!(t.len(), 2);
    }

    #[test]
    fn test_decay_all_zero_is_noop() {
        let mut t = ThreatTable::new();
        t.add(1u32, 50);
        t.decay_all(0);
        assert_eq!(t.threat_of(1), 50);
    }

    #[test]
    fn test_decay_permille() {
        let mut t = ThreatTable::new();
        t.add(1u32, 1000);
        t.decay_all_permille(100); // -10%
        assert_eq!(t.threat_of(1), 900);
        t.decay_all_permille(1000); // -100% → everything pruned
        assert!(t.is_empty());
    }

    #[test]
    fn test_decay_permille_zero_is_noop() {
        let mut t = ThreatTable::new();
        t.add(1u32, 777);
        t.decay_all_permille(0);
        assert_eq!(t.threat_of(1), 777);
    }

    #[test]
    fn test_tie_break_smallest_key() {
        let mut t = ThreatTable::new();
        t.add(5u32, 100);
        t.add(2, 100);
        t.add(9, 100);
        // All equal → smallest key wins deterministically.
        assert_eq!(t.top_target(), Some(&2));
    }

    #[test]
    fn test_taunt_forces_top() {
        let mut t = ThreatTable::new();
        t.add(1u32, 50);
        t.add(2, 200);
        t.taunt(1, 1);
        assert_eq!(t.top_target(), Some(&1));
        assert_eq!(t.threat_of(1), 201, "max(200) + margin(1)");
    }

    #[test]
    fn test_taunt_on_empty_table() {
        let mut t = ThreatTable::new();
        t.taunt(1u32, 10);
        assert_eq!(t.threat_of(1), 10);
        assert_eq!(t.top_target(), Some(&1));
    }

    #[test]
    fn test_total() {
        let mut t = ThreatTable::new();
        t.add(1u32, 30);
        t.add(2, 70);
        assert_eq!(t.total(), 100);
    }

    #[test]
    fn test_clear() {
        let mut t = ThreatTable::new();
        t.add(1u32, 30);
        t.add(2, 70);
        t.clear();
        assert!(t.is_empty());
    }

    #[test]
    fn test_iter_is_sorted_by_key() {
        let mut t = ThreatTable::new();
        t.add(9u32, 1);
        t.add(2, 1);
        t.add(5, 1);
        let keys: Vec<u32> = t.iter().map(|(k, _)| *k).collect();
        assert_eq!(keys, vec![2, 5, 9], "iteration is in ascending key order");
    }

    #[test]
    fn test_det_hash_canonical_and_sensitive() {
        let mut a = ThreatTable::new();
        a.add(1u32, 50);
        a.add(2, 80);
        // Insertion order should not matter for the hash.
        let mut b = ThreatTable::new();
        b.add(2u32, 80);
        b.add(1, 50);
        assert_eq!(hash_state(&a), hash_state(&b), "order-independent hash");

        let mut c = ThreatTable::new();
        c.add(1u32, 50);
        c.add(2, 81); // one different threat value
        assert_ne!(hash_state(&a), hash_state(&c), "different threat → different hash");
    }
}
