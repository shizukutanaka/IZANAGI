//! Procedural encounter packs — "spawn 2–4 goblins, plus a shaman 30% of the
//! time" rollers for room population.
//!
//! [`RandomTable`](crate::random_table::RandomTable) picks *one* value per
//! roll; real encounter design wants a **group**: several slots, each with a
//! count range and an optional appearance chance, rolled together in a fixed
//! order. `EncounterPack<T>` packages that pattern (G4 in
//! `STRENGTHS_WEAKNESSES.md`).
//!
//! Determinism contract (replay-safe):
//! - Slots roll in **insertion order**, always.
//! - Per slot: one `coin` draw if `chance_percent` is in `1..=99` (degenerate
//!   chances of 0 or ≥ 100 resolve without drawing, mirroring
//!   [`SplitMix64::coin`](crate::rng::SplitMix64::coin)); then one count draw
//!   if `min < max` (a fixed count of `min == max` draws nothing, mirroring
//!   [`SplitMix64::range_u32`](crate::rng::SplitMix64::range_u32)).
//! - No float, no OS clock, no `HashMap` ordering.
//!
//! `DetHash` (gated on `T: DetHash`) folds the slot configuration in insertion
//! order so a pack definition can participate in a world/replay hash.

use crate::rng::SplitMix64;
use crate::world_hash::{DetHash, Fnv1a};

/// Uniform count in the inclusive range `[min, max]`, consuming exactly one RNG
/// draw (none when `min >= max`).
///
/// Avoids the `max - min + 1` overflow a naive `below(span + 1)` hits at the
/// full `u32` span (`min = 0, max = u32::MAX`): the inclusive span is computed
/// in `u64` and folded with the same wide-multiply `below` uses, so the result
/// is **identical** to `below(span + 1)` for every representable span — only the
/// otherwise-panicking full-range case is extended.
fn roll_count(rng: &mut SplitMix64, min: u32, max: u32) -> u32 {
    if min >= max {
        return min;
    }
    let span_inclusive = (max - min) as u64 + 1; // ∈ [2, 2^32], fits u64
    let pick = ((rng.next_u64() as u128).wrapping_mul(span_inclusive as u128) >> 64) as u32;
    min + pick
}

/// One slot in an [`EncounterPack`]: a value spawned `min..=max` times,
/// included with probability `chance_percent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncounterSlot<T> {
    /// What spawns (monster id, prefab name, …).
    pub value: T,
    /// Minimum count when the slot appears.
    pub min: u32,
    /// Maximum count (inclusive) when the slot appears. Clamped up to `min`
    /// at construction so `min <= max` always holds.
    pub max: u32,
    /// Probability the slot appears at all, as a percentage. `>= 100` means
    /// always; `0` means never (the slot is kept for cataloguing but rolls
    /// nothing, like a weight-0 `RandomTable` entry).
    pub chance_percent: u32,
}

/// A group encounter definition: an ordered list of [`EncounterSlot`]s rolled
/// together. Build with [`with_slot`](Self::with_slot) /
/// [`with_optional_slot`](Self::with_optional_slot), roll with
/// [`roll`](Self::roll):
///
/// ```
/// use izanagi_kit::encounter::EncounterPack;
/// use izanagi_kit::rng::SplitMix64;
///
/// let pack = EncounterPack::new()
///     .with_slot("goblin", 2, 4)               // always 2–4 goblins
///     .with_optional_slot("shaman", 1, 1, 30); // one shaman, 30% of the time
///
/// let mut rng = SplitMix64::new(11);
/// let spawns = pack.roll(&mut rng);
/// let goblins = spawns.iter().filter(|s| **s == "goblin").count();
/// assert!((2..=4).contains(&goblins));
/// ```
#[derive(Clone, Debug, Default)]
pub struct EncounterPack<T> {
    slots: Vec<EncounterSlot<T>>,
}

impl<T> EncounterPack<T> {
    /// Create an empty pack.
    pub fn new() -> Self {
        EncounterPack { slots: Vec::new() }
    }

    /// Builder: add a slot that always appears, spawning `min..=max` copies.
    /// `max` is clamped up to `min` so the range is never inverted.
    pub fn with_slot(mut self, value: T, min: u32, max: u32) -> Self {
        self.push_slot(value, min, max, 100);
        self
    }

    /// Builder: add a slot that appears with probability `chance_percent`
    /// (0 = never, ≥ 100 = always), spawning `min..=max` copies when it does.
    pub fn with_optional_slot(mut self, value: T, min: u32, max: u32, chance_percent: u32) -> Self {
        self.push_slot(value, min, max, chance_percent);
        self
    }

    /// Add a slot in place. `max` is clamped up to `min`.
    pub fn push_slot(&mut self, value: T, min: u32, max: u32, chance_percent: u32) {
        self.slots.push(EncounterSlot {
            value,
            min,
            max: max.max(min),
            chance_percent,
        });
    }

    /// Number of slots (including never-appearing `chance_percent == 0` ones).
    #[inline]
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// `true` if no slots are defined.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Iterate the slot definitions in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = &EncounterSlot<T>> {
        self.slots.iter()
    }

    /// The smallest possible spawn count across a roll (every optional slot
    /// absent, every present slot at `min`). Slots with `chance_percent < 100`
    /// contribute 0; guaranteed slots contribute `min`.
    pub fn min_spawns(&self) -> u32 {
        self.slots
            .iter()
            .filter(|s| s.chance_percent >= 100)
            .fold(0u32, |acc, s| acc.saturating_add(s.min))
    }

    /// The largest possible spawn count across a roll (every slot with
    /// `chance_percent > 0` present at `max`).
    pub fn max_spawns(&self) -> u32 {
        self.slots
            .iter()
            .filter(|s| s.chance_percent > 0)
            .fold(0u32, |acc, s| acc.saturating_add(s.max))
    }

    /// Roll the pack: for each slot in insertion order, decide appearance
    /// (`coin(chance, 100)` — no draw for degenerate 0 / ≥ 100 chances), then
    /// roll a count in `min..=max` (no draw when `min == max`) and emit that
    /// many clones of the slot value.
    ///
    /// Returns the spawned values grouped by slot, in slot order. An empty
    /// pack returns an empty `Vec` without drawing.
    pub fn roll(&self, rng: &mut SplitMix64) -> Vec<T>
    where
        T: Clone,
    {
        let mut out = Vec::new();
        for slot in &self.slots {
            if !rng.coin(slot.chance_percent, 100) {
                continue;
            }
            let count = roll_count(rng, slot.min, slot.max);
            for _ in 0..count {
                out.push(slot.value.clone());
            }
        }
        out
    }

    /// Roll the pack and return `(value, count)` pairs instead of repeated
    /// clones — one entry per slot that appeared with a non-zero count, in
    /// slot order. Identical draw sequence to [`roll`](Self::roll).
    pub fn roll_counts(&self, rng: &mut SplitMix64) -> Vec<(T, u32)>
    where
        T: Clone,
    {
        let mut out = Vec::new();
        for slot in &self.slots {
            if !rng.coin(slot.chance_percent, 100) {
                continue;
            }
            let count = roll_count(rng, slot.min, slot.max);
            if count > 0 {
                out.push((slot.value.clone(), count));
            }
        }
        out
    }
}

impl<T: DetHash> DetHash for EncounterPack<T> {
    /// Folds slot definitions in insertion order, length-prefixed.
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.slots.len() as u32);
        for s in &self.slots {
            s.value.det_hash(hasher);
            hasher.write_u32(s.min);
            hasher.write_u32(s.max);
            hasher.write_u32(s.chance_percent);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    fn pack() -> EncounterPack<&'static str> {
        EncounterPack::new()
            .with_slot("goblin", 2, 4)
            .with_optional_slot("shaman", 1, 1, 30)
    }

    #[test]
    fn test_roll_guaranteed_slot_within_range() {
        let p = pack();
        let mut rng = SplitMix64::new(1);
        for _ in 0..50 {
            let spawns = p.roll(&mut rng);
            let goblins = spawns.iter().filter(|s| **s == "goblin").count();
            assert!((2..=4).contains(&goblins), "goblins={goblins}");
        }
    }

    #[test]
    fn test_roll_optional_slot_sometimes_absent_sometimes_present() {
        let p = pack();
        let mut rng = SplitMix64::new(2);
        let mut seen_present = false;
        let mut seen_absent = false;
        for _ in 0..200 {
            let spawns = p.roll(&mut rng);
            let shamans = spawns.iter().filter(|s| **s == "shaman").count();
            assert!(shamans <= 1);
            if shamans == 1 {
                seen_present = true;
            } else {
                seen_absent = true;
            }
        }
        assert!(seen_present, "30% slot never appeared in 200 rolls");
        assert!(seen_absent, "30% slot appeared every time in 200 rolls");
    }

    #[test]
    fn test_roll_deterministic_same_seed() {
        let p = pack();
        let mut a = SplitMix64::new(42);
        let mut b = SplitMix64::new(42);
        for _ in 0..20 {
            assert_eq!(p.roll(&mut a), p.roll(&mut b));
        }
    }

    #[test]
    fn test_roll_empty_pack_no_draws() {
        let p: EncounterPack<u32> = EncounterPack::new();
        let mut rng = SplitMix64::new(3);
        let before = rng.state();
        assert!(p.roll(&mut rng).is_empty());
        assert_eq!(rng.state(), before, "empty pack must not draw");
    }

    #[test]
    fn test_roll_fixed_count_slot_draws_nothing_for_count() {
        // chance >= 100 (no coin draw) and min == max (no count draw):
        // the whole roll must consume zero draws.
        let p = EncounterPack::new().with_slot('g', 3, 3);
        let mut rng = SplitMix64::new(4);
        let before = rng.state();
        assert_eq!(p.roll(&mut rng), vec!['g', 'g', 'g']);
        assert_eq!(rng.state(), before, "degenerate slot must not draw");
    }

    #[test]
    fn test_zero_chance_slot_never_appears() {
        let p = EncounterPack::new().with_optional_slot("ghost", 1, 3, 0);
        let mut rng = SplitMix64::new(5);
        let before = rng.state();
        for _ in 0..20 {
            assert!(p.roll(&mut rng).is_empty());
        }
        assert_eq!(rng.state(), before, "0% chance resolves without drawing");
    }

    #[test]
    fn test_inverted_range_clamped_at_construction() {
        let p = EncounterPack::new().with_slot('x', 5, 2); // max < min
        assert_eq!(p.iter().next().unwrap().max, 5, "max clamped up to min");
        let mut rng = SplitMix64::new(6);
        assert_eq!(p.roll(&mut rng).len(), 5);
    }

    #[test]
    fn test_min_max_spawns_bounds() {
        let p = pack(); // goblin 2..=4 guaranteed, shaman 0..=1 at 30%
        assert_eq!(p.min_spawns(), 2);
        assert_eq!(p.max_spawns(), 5);
        let mut rng = SplitMix64::new(7);
        for _ in 0..100 {
            let n = p.roll(&mut rng).len() as u32;
            assert!(n >= p.min_spawns() && n <= p.max_spawns(), "n={n}");
        }
    }

    #[test]
    fn test_roll_counts_matches_roll() {
        let p = pack();
        let mut a = SplitMix64::new(8);
        let mut b = SplitMix64::new(8);
        for _ in 0..30 {
            let flat = p.roll(&mut a);
            let grouped = p.roll_counts(&mut b);
            let total: u32 = grouped.iter().map(|(_, c)| c).sum();
            assert_eq!(flat.len() as u32, total, "identical draw sequence");
        }
    }

    fn hash_pack() -> EncounterPack<String> {
        EncounterPack::new()
            .with_slot("goblin".to_string(), 2, 4)
            .with_optional_slot("shaman".to_string(), 1, 1, 30)
    }

    #[test]
    fn test_det_hash_same_config_same_hash() {
        assert_eq!(hash_state(&hash_pack()), hash_state(&hash_pack()));
    }

    #[test]
    fn test_det_hash_differs_on_config_change() {
        let a = EncounterPack::new().with_slot("goblin".to_string(), 2, 4);
        let b = EncounterPack::new().with_slot("goblin".to_string(), 2, 5);
        assert_ne!(hash_state(&a), hash_state(&b));
    }
}
