//! Procedural item affixes — "Rusty Sword of Dragonslaying" generation.
//!
//! Base items become *magic* items by attaching a prefix and/or suffix, each
//! carrying a modifier payload (typically
//! [`combat::StatsModifier`](crate::combat::StatsModifier)). This module
//! supplies the affix records, the rolled item wrapper, and a deterministic
//! generator over weighted affix pools (G7 in `STRENGTHS_WEAKNESSES.md`).
//!
//! Determinism contract (replay-safe):
//! - [`AffixGenerator::roll`] draws in a **fixed order**: prefix coin →
//!   prefix table roll → suffix coin → suffix table roll.
//! - Degenerate chances (0 / ≥ 100) resolve without drawing, mirroring
//!   [`SplitMix64::coin`](crate::rng::SplitMix64::coin); an empty or all-zero
//!   affix pool rolls `None` without drawing, mirroring
//!   [`RandomTable::roll`](crate::random_table::RandomTable::roll).
//! - No float, no OS clock, no `HashMap` ordering.
//!
//! `DetHash` impls fold affix and item state so enchanted inventory items can
//! participate in the per-frame replay checksum.

use crate::random_table::RandomTable;
use crate::rng::SplitMix64;
use crate::world_hash::{DetHash, Fnv1a};

/// Where an affix attaches to the item name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AffixSlot {
    /// Before the base name: "**Rusty** Sword".
    Prefix,
    /// After the base name: "Sword **of Dragonslaying**".
    Suffix,
}

impl DetHash for AffixSlot {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u8(match self {
            AffixSlot::Prefix => 0,
            AffixSlot::Suffix => 1,
        });
    }
}

/// One affix: a display name fragment plus a modifier payload `M`
/// (e.g. [`combat::StatsModifier`](crate::combat::StatsModifier)).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Affix<M> {
    /// Name fragment: "Rusty" (prefix) or "of Dragonslaying" (suffix).
    pub name: String,
    /// Whether this attaches before or after the base name.
    pub slot: AffixSlot,
    /// The mechanical effect the affix grants.
    pub modifier: M,
}

impl<M> Affix<M> {
    /// Construct a prefix affix.
    pub fn prefix(name: impl Into<String>, modifier: M) -> Self {
        Affix {
            name: name.into(),
            slot: AffixSlot::Prefix,
            modifier,
        }
    }

    /// Construct a suffix affix.
    pub fn suffix(name: impl Into<String>, modifier: M) -> Self {
        Affix {
            name: name.into(),
            slot: AffixSlot::Suffix,
            modifier,
        }
    }
}

impl<M: DetHash> DetHash for Affix<M> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_str(&self.name);
        self.slot.det_hash(hasher);
        self.modifier.det_hash(hasher);
    }
}

/// A base item of type `T` with up to one prefix and one suffix attached.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AffixedItem<T, M> {
    /// The unmodified base item.
    pub base: T,
    /// Prefix affix, if rolled.
    pub prefix: Option<Affix<M>>,
    /// Suffix affix, if rolled.
    pub suffix: Option<Affix<M>>,
}

impl<T, M> AffixedItem<T, M> {
    /// Wrap a base item with no affixes (a plain, non-magical item).
    pub fn plain(base: T) -> Self {
        AffixedItem {
            base,
            prefix: None,
            suffix: None,
        }
    }

    /// `true` when at least one affix is attached.
    #[inline]
    pub fn is_magical(&self) -> bool {
        self.prefix.is_some() || self.suffix.is_some()
    }

    /// Number of attached affixes (0–2).
    #[inline]
    pub fn affix_count(&self) -> usize {
        self.prefix.is_some() as usize + self.suffix.is_some() as usize
    }

    /// Iterate attached affix modifiers, prefix first.
    pub fn modifiers(&self) -> impl Iterator<Item = &M> {
        self.prefix
            .iter()
            .chain(self.suffix.iter())
            .map(|a| &a.modifier)
    }

    /// Full display name: `"<prefix> <base_name> <suffix>"`, omitting absent
    /// parts. E.g. `"Rusty Sword of Dragonslaying"`, `"Sword of Flame"`,
    /// or just `"Sword"` for a plain item.
    pub fn display_name(&self, base_name: &str) -> String {
        let mut out = String::new();
        if let Some(p) = &self.prefix {
            out.push_str(&p.name);
            out.push(' ');
        }
        out.push_str(base_name);
        if let Some(s) = &self.suffix {
            out.push(' ');
            out.push_str(&s.name);
        }
        out
    }
}

impl<T> AffixedItem<T, crate::combat::StatsModifier> {
    /// Sum all attached affix modifiers into one
    /// [`StatsModifier`](crate::combat::StatsModifier) (saturating per field).
    /// Returns the default (zero) modifier for a plain item. Compose with
    /// [`Stats::modified`](crate::combat::Stats::modified) to get the wielder's
    /// effective stats.
    pub fn combined_modifier(&self) -> crate::combat::StatsModifier {
        let mut total = crate::combat::StatsModifier::default();
        for m in self.modifiers() {
            total.attack = total.attack.saturating_add(m.attack);
            total.defense = total.defense.saturating_add(m.defense);
            total.max_hp = total.max_hp.saturating_add(m.max_hp);
        }
        total
    }
}

impl<T: DetHash, M: DetHash> DetHash for AffixedItem<T, M> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        self.base.det_hash(hasher);
        self.prefix.det_hash(hasher);
        self.suffix.det_hash(hasher);
    }
}

/// Deterministic affix roller: weighted prefix/suffix pools plus the chance
/// each slot is granted at all.
///
/// ```
/// use izanagi_kit::affix::{Affix, AffixGenerator};
/// use izanagi_kit::combat::StatsModifier;
/// use izanagi_kit::random_table::RandomTable;
/// use izanagi_kit::rng::SplitMix64;
///
/// let atk = |n| StatsModifier { attack: n, ..Default::default() };
/// let generator = AffixGenerator::new(
///     RandomTable::new()
///         .with(3, Affix::prefix("Rusty", atk(-1)))
///         .with(1, Affix::prefix("Gleaming", atk(2))),
///     RandomTable::new().with(1, Affix::suffix("of Dragonslaying", atk(5))),
///     50, // 50% prefix chance
///     20, // 20% suffix chance
/// );
///
/// let mut rng = SplitMix64::new(99);
/// let item = generator.roll("sword", &mut rng);
/// let name = item.display_name("Sword");
/// assert!(name.contains("Sword"));
/// ```
#[derive(Clone, Debug)]
pub struct AffixGenerator<M> {
    prefixes: RandomTable<Affix<M>>,
    suffixes: RandomTable<Affix<M>>,
    prefix_chance: u32,
    suffix_chance: u32,
}

impl<M: Clone> AffixGenerator<M> {
    /// Build a generator from weighted affix pools and per-slot grant chances
    /// (percentages; 0 = never, ≥ 100 = always).
    pub fn new(
        prefixes: RandomTable<Affix<M>>,
        suffixes: RandomTable<Affix<M>>,
        prefix_chance: u32,
        suffix_chance: u32,
    ) -> Self {
        AffixGenerator {
            prefixes,
            suffixes,
            prefix_chance,
            suffix_chance,
        }
    }

    /// Roll affixes for `base`. Fixed draw order: prefix coin → prefix table
    /// roll → suffix coin → suffix table roll, with the degenerate no-draw
    /// rules documented at module level. A granted slot whose pool is empty
    /// (or all-zero weight) stays `None`.
    pub fn roll<T>(&self, base: T, rng: &mut SplitMix64) -> AffixedItem<T, M> {
        let prefix = if rng.coin(self.prefix_chance, 100) {
            self.prefixes.roll_owned(rng)
        } else {
            None
        };
        let suffix = if rng.coin(self.suffix_chance, 100) {
            self.suffixes.roll_owned(rng)
        } else {
            None
        };
        AffixedItem {
            base,
            prefix,
            suffix,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{Stats, StatsModifier};
    use crate::world_hash::hash_state;

    fn atk(n: i32) -> StatsModifier {
        StatsModifier {
            attack: n,
            ..Default::default()
        }
    }

    fn def(n: i32) -> StatsModifier {
        StatsModifier {
            defense: n,
            ..Default::default()
        }
    }

    fn generator(prefix_chance: u32, suffix_chance: u32) -> AffixGenerator<StatsModifier> {
        AffixGenerator::new(
            RandomTable::new()
                .with(3, Affix::prefix("Rusty", atk(-1)))
                .with(1, Affix::prefix("Gleaming", atk(2))),
            RandomTable::new()
                .with(1, Affix::suffix("of Dragonslaying", atk(5)))
                .with(1, Affix::suffix("of Warding", def(3))),
            prefix_chance,
            suffix_chance,
        )
    }

    // --- display_name ---

    #[test]
    fn test_display_name_plain() {
        let item: AffixedItem<&str, StatsModifier> = AffixedItem::plain("sword");
        assert_eq!(item.display_name("Sword"), "Sword");
        assert!(!item.is_magical());
        assert_eq!(item.affix_count(), 0);
    }

    #[test]
    fn test_display_name_prefix_only() {
        let item = AffixedItem {
            base: "sword",
            prefix: Some(Affix::prefix("Rusty", atk(-1))),
            suffix: None,
        };
        assert_eq!(item.display_name("Sword"), "Rusty Sword");
        assert_eq!(item.affix_count(), 1);
    }

    #[test]
    fn test_display_name_both_affixes() {
        let item = AffixedItem {
            base: "sword",
            prefix: Some(Affix::prefix("Gleaming", atk(2))),
            suffix: Some(Affix::suffix("of Dragonslaying", atk(5))),
        };
        assert_eq!(
            item.display_name("Sword"),
            "Gleaming Sword of Dragonslaying"
        );
        assert!(item.is_magical());
        assert_eq!(item.affix_count(), 2);
    }

    // --- combined_modifier / combat integration ---

    #[test]
    fn test_combined_modifier_sums_affixes() {
        let item = AffixedItem {
            base: "sword",
            prefix: Some(Affix::prefix("Gleaming", atk(2))),
            suffix: Some(Affix::suffix("of Warding", def(3))),
        };
        let m = item.combined_modifier();
        assert_eq!(m.attack, 2);
        assert_eq!(m.defense, 3);
        assert_eq!(m.max_hp, 0);
    }

    #[test]
    fn test_combined_modifier_plain_is_default() {
        let item: AffixedItem<&str, StatsModifier> = AffixedItem::plain("sword");
        assert_eq!(item.combined_modifier(), StatsModifier::default());
    }

    #[test]
    fn test_combined_modifier_applies_to_stats() {
        let item = AffixedItem {
            base: "sword",
            prefix: Some(Affix::prefix("Rusty", atk(-1))),
            suffix: None,
        };
        let effective = Stats::new(20, 5, 3).modified(&item.combined_modifier());
        assert_eq!(effective.attack, 4, "rusty penalty applied");
    }

    // --- AffixGenerator::roll ---

    #[test]
    fn test_roll_always_chances_always_magical() {
        let g = generator(100, 100);
        let mut rng = SplitMix64::new(1);
        for _ in 0..20 {
            let item = g.roll("sword", &mut rng);
            assert!(item.prefix.is_some(), "100% prefix must appear");
            assert!(item.suffix.is_some(), "100% suffix must appear");
        }
    }

    #[test]
    fn test_roll_zero_chances_no_draws_plain_item() {
        let g = generator(0, 0);
        let mut rng = SplitMix64::new(2);
        let before = rng.state();
        let item = g.roll("sword", &mut rng);
        assert!(!item.is_magical());
        assert_eq!(rng.state(), before, "0% chances must not draw");
    }

    #[test]
    fn test_roll_deterministic_same_seed() {
        let g = generator(50, 50);
        let mut a = SplitMix64::new(42);
        let mut b = SplitMix64::new(42);
        for _ in 0..20 {
            assert_eq!(g.roll("sword", &mut a), g.roll("sword", &mut b));
        }
    }

    #[test]
    fn test_roll_partial_chance_varies() {
        let g = generator(50, 50);
        let mut rng = SplitMix64::new(3);
        let mut seen_magical = false;
        let mut seen_plain_slot = false;
        for _ in 0..100 {
            let item = g.roll("sword", &mut rng);
            if item.is_magical() {
                seen_magical = true;
            }
            if item.affix_count() < 2 {
                seen_plain_slot = true;
            }
        }
        assert!(seen_magical, "50% chances never granted an affix");
        assert!(seen_plain_slot, "50% chances always granted both affixes");
    }

    #[test]
    fn test_roll_empty_pool_granted_slot_stays_none() {
        // 100% grant chance but an empty prefix pool: no draw for the pool,
        // prefix stays None (coin also resolves without drawing at 100%).
        let g: AffixGenerator<StatsModifier> =
            AffixGenerator::new(RandomTable::new(), RandomTable::new(), 100, 100);
        let mut rng = SplitMix64::new(4);
        let before = rng.state();
        let item = g.roll("sword", &mut rng);
        assert!(!item.is_magical());
        assert_eq!(rng.state(), before, "empty pools must not draw");
    }

    // --- DetHash ---

    #[test]
    fn test_det_hash_same_item_same_hash() {
        let make = || AffixedItem {
            base: "sword".to_string(),
            prefix: Some(Affix::prefix("Rusty", 1i32)),
            suffix: None,
        };
        assert_eq!(hash_state(&make()), hash_state(&make()));
    }

    #[test]
    fn test_det_hash_differs_by_affix() {
        let plain: AffixedItem<String, i32> = AffixedItem::plain("sword".to_string());
        let magic = AffixedItem {
            base: "sword".to_string(),
            prefix: Some(Affix::prefix("Rusty", 1i32)),
            suffix: None,
        };
        assert_ne!(hash_state(&plain), hash_state(&magic));
    }

    #[test]
    fn test_det_hash_prefix_vs_suffix_slot_differs() {
        let p = Affix::prefix("Flame", 1i32);
        let s = Affix::suffix("Flame", 1i32);
        assert_ne!(hash_state(&p), hash_state(&s), "slot participates in hash");
    }
}
