//! Weighted random selection table — loot drops and spawn tables.
//!
//! `RandomTable<T>` holds `(weight, value)` entries and rolls one value with
//! probability proportional to its weight. This is the canonical roguelike
//! spawn/loot pattern (cf. the *Rust Roguelike Tutorial* `random_table.rs` and
//! `bracket-lib`): build a table whose weights scale with dungeon depth, then
//! roll it to decide what appears in each room.
//!
//! It is a typed convenience layer over [`SplitMix64::weighted_index`]: the
//! table *owns* the candidate values, so a roll yields the value directly. The
//! roll consumes exactly one draw from the supplied stream (or none for an
//! empty / all-zero table), keeping selection replay-deterministic.
//!
//! Entries with weight `0` are stored but never selected — handy for listing a
//! full item catalogue and gating availability purely through weights.
//!
//! `DetHash` (gated on `T: DetHash`) folds the weights and values in insertion
//! order, so a table's configuration can participate in a world/replay hash.

use crate::rng::SplitMix64;
use crate::world_hash::{DetHash, Fnv1a};

/// One weighted candidate in a [`RandomTable`].
#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry<T> {
    weight: u32,
    value: T,
}

/// A weighted table of candidate values of type `T`.
#[derive(Clone, Debug, Default)]
pub struct RandomTable<T> {
    entries: Vec<Entry<T>>,
    total_weight: u64,
}

impl<T> RandomTable<T> {
    /// Create an empty table.
    pub fn new() -> Self {
        RandomTable {
            entries: Vec::new(),
            total_weight: 0,
        }
    }

    /// Add a `(weight, value)` entry, consuming and returning `self` for
    /// builder-style chaining: `RandomTable::new().with(3, a).with(1, b)`.
    pub fn with(mut self, weight: u32, value: T) -> Self {
        self.push(weight, value);
        self
    }

    /// Add a `(weight, value)` entry in place. A `weight` of 0 stores the value
    /// but makes it unreachable by [`roll`](Self::roll).
    pub fn push(&mut self, weight: u32, value: T) {
        self.total_weight += weight as u64;
        self.entries.push(Entry { weight, value });
    }

    /// Roll a value with probability proportional to its weight, drawing once
    /// from `rng`. Returns `None` — **without drawing** — when the table is
    /// empty or every weight is 0, mirroring [`SplitMix64::weighted_index`].
    pub fn roll(&self, rng: &mut SplitMix64) -> Option<&T> {
        if self.total_weight == 0 {
            return None;
        }
        // Wide-multiply pick in `[0, total_weight)` — the u64 analogue of
        // `below`, identical to `weighted_index` so draw counts line up.
        let mut pick = ((rng.next_u64() as u128 * self.total_weight as u128) >> 64) as u64;
        for e in &self.entries {
            let w = e.weight as u64;
            if pick < w {
                return Some(&e.value);
            }
            pick -= w;
        }
        // Rounding can't normally reach here; fall back to the last real entry.
        self.entries
            .iter()
            .rev()
            .find(|e| e.weight > 0)
            .map(|e| &e.value)
    }

    /// Like [`roll`](Self::roll) but returns an owned (cloned) value — removes
    /// the borrow that `roll` carries so the result can be stored without keeping
    /// a reference to the table alive.
    pub fn roll_owned(&self, rng: &mut SplitMix64) -> Option<T>
    where
        T: Clone,
    {
        self.roll(rng).cloned()
    }

    /// Draw `n` independent samples **with replacement** and return them as a
    /// `Vec<T>` (cloned). Each draw consumes exactly one RNG call (same as
    /// `roll_owned`). Returns an empty `Vec` when the table is empty or
    /// `n == 0` — no draws are consumed for empty/zero cases.
    pub fn roll_n(&self, n: u32, rng: &mut SplitMix64) -> Vec<T>
    where
        T: Clone,
    {
        (0..n).filter_map(|_| self.roll_owned(rng)).collect()
    }

    /// Sum of all entry weights.
    #[inline]
    pub fn total_weight(&self) -> u64 {
        self.total_weight
    }

    /// The highest single-entry weight in the table, or `0` if the table is
    /// empty or all weights are 0. Useful for "is this table uniform?" checks.
    pub fn max_weight(&self) -> u32 {
        self.entries.iter().map(|e| e.weight).max().unwrap_or(0)
    }

    /// Average weight across all entries: `total_weight / len` (integer
    /// division). Returns `0` for an empty table. Useful for "is the table
    /// roughly uniform?" distribution checks and balance heuristics without
    /// iterating manually.
    pub fn average_weight(&self) -> u32 {
        if self.entries.is_empty() {
            0
        } else {
            (self.total_weight / self.entries.len() as u64) as u32
        }
    }

    /// Number of entries (including any with weight 0).
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True if no entries have been added.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Remove all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_weight = 0;
    }

    /// Iterate `(weight, &value)` in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (u32, &T)> {
        self.entries.iter().map(|e| (e.weight, &e.value))
    }

    /// Update the weight of the entry at `idx`. Silently ignores out-of-range
    /// indices. Adjusts `total_weight` to stay consistent with the new value.
    /// Setting a weight to `0` makes the entry permanently unselectable without
    /// removing it from the table — useful for "sold out" items in a shop.
    pub fn set_weight(&mut self, idx: usize, weight: u32) {
        if let Some(entry) = self.entries.get_mut(idx) {
            self.total_weight = self.total_weight - entry.weight as u64 + weight as u64;
            entry.weight = weight;
        }
    }

    /// Like [`roll`](Self::roll) but returns the **entry index** instead of the
    /// value, so the caller can post-process, remove, or count the rolled slot.
    /// Draws exactly once from `rng`; returns `None` without drawing when all
    /// weights are 0 or the table is empty.
    pub fn weighted_idx(&self, rng: &mut SplitMix64) -> Option<usize> {
        if self.total_weight == 0 {
            return None;
        }
        let mut pick = ((rng.next_u64() as u128 * self.total_weight as u128) >> 64) as u64;
        for (i, e) in self.entries.iter().enumerate() {
            let w = e.weight as u64;
            if pick < w {
                return Some(i);
            }
            pick -= w;
        }
        // Rounding fallback: last non-zero-weight entry.
        self.entries
            .iter()
            .enumerate()
            .rev()
            .find(|(_, e)| e.weight > 0)
            .map(|(i, _)| i)
    }
}

impl<T: DetHash> DetHash for RandomTable<T> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.entries.len() as u32);
        for e in &self.entries {
            hasher.write_u32(e.weight);
            e.value.det_hash(hasher);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_table_rolls_none_without_drawing() {
        let table: RandomTable<&str> = RandomTable::new();
        let mut rng = SplitMix64::new(1);
        let before = rng.state();
        assert_eq!(table.roll(&mut rng), None);
        assert_eq!(rng.state(), before, "empty roll must not draw");
    }

    #[test]
    fn test_all_zero_weights_roll_none_without_drawing() {
        let table = RandomTable::new().with(0, "a").with(0, "b");
        let mut rng = SplitMix64::new(2);
        let before = rng.state();
        assert_eq!(table.roll(&mut rng), None);
        assert_eq!(rng.state(), before);
    }

    #[test]
    fn test_single_entry_always_rolls_itself() {
        let table = RandomTable::new().with(5, "only");
        let mut rng = SplitMix64::new(3);
        for _ in 0..20 {
            assert_eq!(table.roll(&mut rng), Some(&"only"));
        }
    }

    #[test]
    fn test_zero_weight_entry_never_selected() {
        let table = RandomTable::new().with(10, "common").with(0, "never");
        let mut rng = SplitMix64::new(4);
        for _ in 0..100 {
            assert_eq!(table.roll(&mut rng), Some(&"common"));
        }
    }

    #[test]
    fn test_roll_owned_returns_cloned_value() {
        let table = RandomTable::new().with(1, "sword").with(1, "shield");
        let mut rng = SplitMix64::new(0xABCD);
        // roll_owned must return Some(_) and the value must be owned (no borrow).
        let result: Option<&str> = table.roll_owned(&mut rng);
        assert!(result.is_some());
    }

    #[test]
    fn test_roll_owned_empty_is_none() {
        let table: RandomTable<u32> = RandomTable::new();
        let mut rng = SplitMix64::new(1);
        assert_eq!(table.roll_owned(&mut rng), None);
    }

    #[test]
    fn test_weights_track_total() {
        let mut table: RandomTable<u8> = RandomTable::new();
        table.push(3, 1);
        table.push(7, 2);
        assert_eq!(table.total_weight(), 10);
        assert_eq!(table.len(), 2);
        table.clear();
        assert_eq!(table.total_weight(), 0);
        assert!(table.is_empty());
    }

    #[test]
    fn test_distribution_is_proportional() {
        // 1:3 weighting should land near a 25%/75% split over many rolls.
        let table = RandomTable::new().with(1, 'a').with(3, 'b');
        let mut rng = SplitMix64::new(0xC0FFEE);
        let mut a = 0u32;
        let mut b = 0u32;
        for _ in 0..10_000 {
            match table.roll(&mut rng) {
                Some('a') => a += 1,
                Some('b') => b += 1,
                _ => unreachable!(),
            }
        }
        // a ≈ 2500, b ≈ 7500; allow generous slack but assert the ordering and
        // rough proportion so a bias bug would be caught.
        assert!(a > 2000 && a < 3000, "a={a}");
        assert!(b > 7000 && b < 8000, "b={b}");
        assert_eq!(a + b, 10_000);
    }

    #[test]
    fn test_roll_is_deterministic_for_seed() {
        let table = RandomTable::new()
            .with(2, "sword")
            .with(5, "potion")
            .with(1, "amulet");
        let seq = |seed: u64| {
            let mut rng = SplitMix64::new(seed);
            (0..16)
                .map(|_| *table.roll(&mut rng).unwrap())
                .collect::<Vec<_>>()
        };
        assert_eq!(seq(42), seq(42), "same seed must replay identically");
    }

    #[test]
    fn test_roll_n_returns_correct_count() {
        let table = RandomTable::new().with(1, 'a').with(1, 'b').with(1, 'c');
        let mut rng = SplitMix64::new(77);
        let results = table.roll_n(5, &mut rng);
        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|&c| matches!(c, 'a' | 'b' | 'c')));
    }

    #[test]
    fn test_roll_n_zero_returns_empty() {
        let table = RandomTable::new().with(1, 1u32);
        let mut rng = SplitMix64::new(1);
        let state_before = rng.state();
        let r = table.roll_n(0, &mut rng);
        assert!(r.is_empty());
        assert_eq!(rng.state(), state_before, "zero draws must not advance RNG");
    }

    #[test]
    fn test_roll_n_empty_table_returns_empty() {
        let table: RandomTable<u32> = RandomTable::new();
        let mut rng = SplitMix64::new(1);
        let state_before = rng.state();
        let r = table.roll_n(5, &mut rng);
        assert!(r.is_empty());
        assert_eq!(
            rng.state(),
            state_before,
            "empty table must not advance RNG"
        );
    }

    #[test]
    fn test_roll_n_is_deterministic() {
        let table = RandomTable::new().with(3, 10u32).with(7, 20u32);
        let run = |seed: u64| {
            let mut rng = SplitMix64::new(seed);
            table.roll_n(10, &mut rng)
        };
        assert_eq!(run(42), run(42));
        assert_ne!(run(42), run(99));
    }

    #[test]
    fn test_det_hash_reflects_config() {
        use crate::world_hash::hash_state;
        let a = RandomTable::new().with(1, 7u32).with(2, 9u32);
        let b = RandomTable::new().with(1, 7u32).with(2, 9u32);
        let c = RandomTable::new().with(2, 7u32).with(2, 9u32); // different weight
        assert_eq!(hash_state(&a), hash_state(&b));
        assert_ne!(hash_state(&a), hash_state(&c));
    }

    #[test]
    fn test_weighted_idx_empty_returns_none() {
        let t: RandomTable<u32> = RandomTable::new();
        let mut rng = SplitMix64::new(1);
        assert_eq!(t.weighted_idx(&mut rng), None);
    }

    #[test]
    fn test_weighted_idx_in_bounds() {
        let t = RandomTable::new().with(1, "a").with(2, "b").with(3, "c");
        let mut rng = SplitMix64::new(42);
        for _ in 0..20 {
            let idx = t.weighted_idx(&mut rng).unwrap();
            assert!(idx < 3, "idx {idx} out of range");
        }
    }

    #[test]
    fn test_weighted_idx_consistent_with_roll() {
        // Same RNG state → weighted_idx and roll should pick same entry.
        let t = RandomTable::new()
            .with(1, 10u32)
            .with(4, 20u32)
            .with(2, 30u32);
        let seed = 77u64;
        let mut rng_a = SplitMix64::new(seed);
        let mut rng_b = SplitMix64::new(seed);
        let idx = t.weighted_idx(&mut rng_a).unwrap();
        let val = t.roll(&mut rng_b).unwrap();
        // The entry at `idx` must be the same as what `roll` picked.
        let (_, &entry_val) = t.iter().nth(idx).unwrap();
        assert_eq!(entry_val, *val);
    }

    #[test]
    fn test_set_weight_updates_total_weight() {
        let mut t = RandomTable::new().with(3u32, 10u32).with(2u32, 20u32);
        assert_eq!(t.total_weight(), 5);
        t.set_weight(0, 7);
        assert_eq!(t.total_weight(), 9); // 7 + 2
    }

    #[test]
    fn test_set_weight_out_of_bounds_is_noop() {
        let mut t = RandomTable::new().with(5u32, 42u32);
        t.set_weight(99, 1); // out of range — must not panic
        assert_eq!(t.total_weight(), 5);
    }

    #[test]
    fn test_set_weight_to_zero_excludes_entry() {
        let mut t = RandomTable::new().with(1u32, 10u32).with(10u32, 20u32);
        t.set_weight(0, 0); // entry 0 now has weight 0
                            // With only entry 1 having non-zero weight, every roll must return 20.
        let mut rng = SplitMix64::new(99);
        for _ in 0..10 {
            assert_eq!(*t.roll(&mut rng).unwrap(), 20);
        }
    }

    #[test]
    fn test_max_weight_empty_table_is_zero() {
        let t: RandomTable<u32> = RandomTable::new();
        assert_eq!(t.max_weight(), 0);
    }

    #[test]
    fn test_max_weight_returns_highest() {
        let t = RandomTable::new()
            .with(3u32, 'a')
            .with(10u32, 'b')
            .with(1u32, 'c');
        assert_eq!(t.max_weight(), 10);
    }

    #[test]
    fn test_max_weight_all_zero_weights_is_zero() {
        let t = RandomTable::new().with(0u32, 1u32).with(0u32, 2u32);
        assert_eq!(t.max_weight(), 0);
    }

    #[test]
    fn test_average_weight_empty_is_zero() {
        let t: RandomTable<u32> = RandomTable::new();
        assert_eq!(t.average_weight(), 0);
    }

    #[test]
    fn test_average_weight_uniform_table() {
        let t = RandomTable::new()
            .with(10u32, 'a')
            .with(10u32, 'b')
            .with(10u32, 'c');
        assert_eq!(t.average_weight(), 10);
    }

    #[test]
    fn test_average_weight_mixed_table() {
        let t = RandomTable::new().with(2u32, 'a').with(6u32, 'b');
        // total = 8, len = 2, avg = 4
        assert_eq!(t.average_weight(), 4);
    }
}
