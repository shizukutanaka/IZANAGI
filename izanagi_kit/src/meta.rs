//! Meta-progression — the state that survives permadeath.
//!
//! [`progression`](crate::progression) tracks a *single character's* growth
//! within one life (XP → level), and [`savefile`](crate::savefile) can
//! persist whatever bytes you hand it — but nothing modeled the roguelike
//! genre's other axis: **what survives when the run ends**. Rogue Legacy's
//! inherited castle upgrades, Hades's mirror of night, Dead Cells's cell
//! currency, and even NetHack's high-score list all share the same shape —
//! a small set of **idempotent unlock flags** and **all-time best records**
//! that a new run never resets, layered *above* per-run state that always
//! does. [`MetaProgress`] is that layer.
//!
//! ```
//! use izanagi_kit::meta::MetaProgress;
//!
//! #[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
//! enum Feature { HardMode, MageClass }
//!
//! let mut meta: MetaProgress<Feature, &str> = MetaProgress::new();
//!
//! // Beat the game once — unlock hard mode for next time.
//! assert!(meta.unlock(Feature::HardMode), "newly unlocked");
//! assert!(!meta.unlock(Feature::HardMode), "already unlocked — no-op");
//! assert!(meta.is_unlocked(Feature::HardMode));
//!
//! // Track the deepest floor ever reached across all runs (higher is better).
//! assert!(meta.record_best("deepest_floor", 7), "first record set");
//! assert!(!meta.record_best("deepest_floor", 5), "5 does not beat 7");
//! assert!(meta.record_best("deepest_floor", 12), "12 beats 7");
//! assert_eq!(meta.best("deepest_floor"), Some(12));
//! ```
//!
//! ## Design
//!
//! - `unlocked: BTreeSet<K>` and `records: BTreeMap<R, i64>` — both canonical
//!   containers, so [`DetHash`] is
//!   insertion-order-independent by construction.
//! - [`record_best`](MetaProgress::record_best) treats "higher is better,"
//!   the same idempotent max-fold used elsewhere in the kit for
//!   arrival-order-safe aggregation (e.g. [`netinput`](crate::netinput)'s
//!   `last_known` tick tracking). For a "lower is better" record (fastest
//!   clear time, fewest turns), negate the value before recording — a call
//!   site keeping `-elapsed_ticks` sees the same "did this beat the max?"
//!   semantics without a second, easily-misused method.
//! - This module makes no lifecycle decisions: it does not know what "a run"
//!   is or when one starts or ends. The caller owns that — typically by
//!   keeping one `MetaProgress` instance alive for the whole session while
//!   discarding and rebuilding per-run state (character stats, inventory,
//!   dungeon) on every death. `MetaProgress` only guarantees that whatever it
//!   holds behaves correctly regardless of how many times it is queried or in
//!   what order unlocks/records arrive.

use crate::world_hash::{DetHash, Fnv1a};
use std::collections::{BTreeMap, BTreeSet};

/// Persistent unlock flags and all-time best records, keyed independently:
/// `K` names unlockable features, `R` names tracked records. Typically both
/// are small enums, but they need not be the same type.
#[derive(Clone, Debug, Default)]
pub struct MetaProgress<K: Ord + Clone, R: Ord + Clone> {
    unlocked: BTreeSet<K>,
    records: BTreeMap<R, i64>,
}

impl<K: Ord + Clone, R: Ord + Clone> MetaProgress<K, R> {
    /// An empty meta-progress record: nothing unlocked, no records set.
    pub fn new() -> Self {
        MetaProgress {
            unlocked: BTreeSet::new(),
            records: BTreeMap::new(),
        }
    }

    /// Unlock `feature` permanently. Returns `true` if it was not already
    /// unlocked (idempotent: unlocking an already-unlocked feature is a
    /// harmless no-op that returns `false`).
    pub fn unlock(&mut self, feature: K) -> bool {
        self.unlocked.insert(feature)
    }

    /// `true` if `feature` has been unlocked.
    pub fn is_unlocked(&self, feature: K) -> bool {
        self.unlocked.contains(&feature)
    }

    /// Revoke a previously unlocked feature (e.g. a scripted "hardcore reset"
    /// or a debug tool). Returns `true` if it had been unlocked.
    pub fn revoke(&mut self, feature: K) -> bool {
        self.unlocked.remove(&feature)
    }

    /// The number of currently unlocked features.
    pub fn unlocked_count(&self) -> usize {
        self.unlocked.len()
    }

    /// Iterate unlocked features in ascending order.
    pub fn unlocked_iter(&self) -> impl Iterator<Item = &K> {
        self.unlocked.iter()
    }

    /// Record `value` for `stat` if it beats the current best (higher is
    /// better) or if no record exists yet. Returns `true` if this call set a
    /// new best. For "lower is better" stats (fastest clear, fewest deaths),
    /// record the negated value and negate again when reading it back.
    pub fn record_best(&mut self, stat: R, value: i64) -> bool {
        match self.records.get(&stat) {
            Some(&best) if value <= best => false,
            _ => {
                self.records.insert(stat, value);
                true
            }
        }
    }

    /// The current best recorded value for `stat`, or `None` if never set.
    pub fn best(&self, stat: R) -> Option<i64> {
        self.records.get(&stat).copied()
    }

    /// The number of distinct records held.
    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    /// Iterate `(stat, best value)` pairs in ascending stat order.
    pub fn records_iter(&self) -> impl Iterator<Item = (&R, i64)> {
        self.records.iter().map(|(r, &v)| (r, v))
    }
}

impl<K: Ord + Clone + DetHash, R: Ord + Clone + DetHash> DetHash for MetaProgress<K, R> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.unlocked.len() as u32);
        for feature in &self.unlocked {
            feature.det_hash(hasher);
        }
        hasher.write_u32(self.records.len() as u32);
        for (stat, &value) in &self.records {
            stat.det_hash(hasher);
            hasher.write_i64(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[test]
    fn test_new_is_empty() {
        let meta: MetaProgress<u32, u32> = MetaProgress::new();
        assert_eq!(meta.unlocked_count(), 0);
        assert_eq!(meta.record_count(), 0);
        assert!(!meta.is_unlocked(1));
        assert_eq!(meta.best(1), None);
    }

    #[test]
    fn test_unlock_returns_true_first_time() {
        let mut meta: MetaProgress<u32, u32> = MetaProgress::new();
        assert!(meta.unlock(1));
        assert!(meta.is_unlocked(1));
        assert_eq!(meta.unlocked_count(), 1);
    }

    #[test]
    fn test_unlock_is_idempotent() {
        let mut meta: MetaProgress<u32, u32> = MetaProgress::new();
        assert!(meta.unlock(1));
        assert!(!meta.unlock(1), "already unlocked, no-op");
        assert_eq!(meta.unlocked_count(), 1, "no duplicate entry");
    }

    #[test]
    fn test_multiple_features_independent() {
        let mut meta: MetaProgress<u32, u32> = MetaProgress::new();
        meta.unlock(1);
        meta.unlock(2);
        assert!(meta.is_unlocked(1));
        assert!(meta.is_unlocked(2));
        assert!(!meta.is_unlocked(3));
        assert_eq!(meta.unlocked_count(), 2);
    }

    #[test]
    fn test_revoke_removes_unlock() {
        let mut meta: MetaProgress<u32, u32> = MetaProgress::new();
        meta.unlock(1);
        assert!(meta.revoke(1));
        assert!(!meta.is_unlocked(1));
        assert!(!meta.revoke(1), "second revoke returns false");
    }

    #[test]
    fn test_unlocked_iter_is_sorted() {
        let mut meta: MetaProgress<u32, u32> = MetaProgress::new();
        meta.unlock(30);
        meta.unlock(10);
        meta.unlock(20);
        let features: Vec<u32> = meta.unlocked_iter().copied().collect();
        assert_eq!(features, vec![10, 20, 30]);
    }

    #[test]
    fn test_record_best_first_value_always_sets() {
        let mut meta: MetaProgress<u32, u32> = MetaProgress::new();
        assert!(meta.record_best(1, 5));
        assert_eq!(meta.best(1), Some(5));
    }

    #[test]
    fn test_record_best_rejects_worse_value() {
        let mut meta: MetaProgress<u32, u32> = MetaProgress::new();
        meta.record_best(1, 10);
        assert!(!meta.record_best(1, 5), "5 does not beat 10");
        assert_eq!(meta.best(1), Some(10), "unchanged");
    }

    #[test]
    fn test_record_best_accepts_better_value() {
        let mut meta: MetaProgress<u32, u32> = MetaProgress::new();
        meta.record_best(1, 10);
        assert!(meta.record_best(1, 15));
        assert_eq!(meta.best(1), Some(15));
    }

    #[test]
    fn test_record_best_equal_value_is_not_a_new_record() {
        let mut meta: MetaProgress<u32, u32> = MetaProgress::new();
        meta.record_best(1, 10);
        assert!(!meta.record_best(1, 10), "tying is not beating");
        assert_eq!(meta.best(1), Some(10));
    }

    #[test]
    fn test_negated_value_tracks_lower_is_better() {
        let mut meta: MetaProgress<u32, u32> = MetaProgress::new();
        // Track fastest clear (fewest ticks) by recording the negation.
        assert!(meta.record_best(1, -100)); // 100 ticks
        assert!(meta.record_best(1, -80), "80 ticks is faster: -80 > -100");
        assert!(!meta.record_best(1, -90), "90 ticks is slower: -90 < -80");
        assert_eq!(meta.best(1), Some(-80));
        assert_eq!(-meta.best(1).unwrap(), 80, "recover the real tick count");
    }

    #[test]
    fn test_multiple_stats_independent() {
        let mut meta: MetaProgress<u32, u32> = MetaProgress::new();
        meta.record_best(1, 10);
        meta.record_best(2, 20);
        assert_eq!(meta.best(1), Some(10));
        assert_eq!(meta.best(2), Some(20));
        assert_eq!(meta.record_count(), 2);
    }

    #[test]
    fn test_records_iter_is_sorted() {
        let mut meta: MetaProgress<u32, u32> = MetaProgress::new();
        meta.record_best(30, 1);
        meta.record_best(10, 2);
        meta.record_best(20, 3);
        let stats: Vec<u32> = meta.records_iter().map(|(s, _)| *s).collect();
        assert_eq!(stats, vec![10, 20, 30]);
    }

    #[test]
    fn test_unlocks_and_records_are_independently_keyed() {
        // K and R can differ; even when they're the same type, they occupy
        // separate namespaces (unlocking key 1 doesn't touch records at 1).
        let mut meta: MetaProgress<u32, u32> = MetaProgress::new();
        meta.unlock(1);
        assert_eq!(meta.best(1), None, "unlocking does not create a record");
        meta.record_best(1, 5);
        assert!(
            meta.is_unlocked(1),
            "recording does not affect unlock state"
        );
    }

    #[test]
    fn test_det_hash_order_independent_unlocks() {
        let mut a: MetaProgress<u32, u32> = MetaProgress::new();
        a.unlock(1);
        a.unlock(2);
        let mut b: MetaProgress<u32, u32> = MetaProgress::new();
        b.unlock(2);
        b.unlock(1);
        assert_eq!(
            hash_state(&a),
            hash_state(&b),
            "unlock order does not affect the hash"
        );
    }

    #[test]
    fn test_det_hash_order_independent_records() {
        let mut a: MetaProgress<u32, u32> = MetaProgress::new();
        a.record_best(1, 10);
        a.record_best(2, 20);
        let mut b: MetaProgress<u32, u32> = MetaProgress::new();
        b.record_best(2, 20);
        b.record_best(1, 10);
        assert_eq!(
            hash_state(&a),
            hash_state(&b),
            "record order does not affect the hash"
        );
    }

    #[test]
    fn test_det_hash_sensitive_to_unlock_set() {
        let mut a: MetaProgress<u32, u32> = MetaProgress::new();
        a.unlock(1);
        let mut b: MetaProgress<u32, u32> = MetaProgress::new();
        b.unlock(1);
        b.unlock(2);
        assert_ne!(
            hash_state(&a),
            hash_state(&b),
            "different unlock set → different hash"
        );
    }

    #[test]
    fn test_det_hash_sensitive_to_record_value() {
        let mut a: MetaProgress<u32, u32> = MetaProgress::new();
        a.record_best(1, 10);
        let mut b: MetaProgress<u32, u32> = MetaProgress::new();
        b.record_best(1, 11);
        assert_ne!(
            hash_state(&a),
            hash_state(&b),
            "different record value → different hash"
        );
    }

    #[test]
    fn test_det_hash_stable_when_record_attempt_fails() {
        let mut a: MetaProgress<u32, u32> = MetaProgress::new();
        a.record_best(1, 10);
        let before = hash_state(&a);
        a.record_best(1, 5); // fails, does not beat 10
        assert_eq!(
            hash_state(&a),
            before,
            "a rejected record leaves the hash unchanged"
        );
    }
}
