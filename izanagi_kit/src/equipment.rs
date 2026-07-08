//! Equipment loadout — worn items per body slot, with aggregate stat bonus.
//!
//! [`Inventory`](crate::inventory::Inventory) *stores* items and
//! [`combat::StatsModifier`](crate::combat::StatsModifier) *describes* an item's
//! stat bonus, but nothing connected the two: there was no way to say "this
//! creature is wearing a sword + helmet + ring, here is the combined modifier to
//! fold into [`combat::Stats`](crate::combat::Stats)". [`Equipment`] is that
//! layer — a fixed set of named [`EquipSlot`]s, each holding at most one item,
//! with [`aggregate`](Equipment::aggregate) folding every worn item's modifier
//! into one via [`StatsModifier::combine`](crate::combat::StatsModifier::combine).
//!
//! ```
//! use izanagi_kit::equipment::{Equipment, EquipSlot};
//! use izanagi_kit::combat::{Stats, StatsModifier};
//!
//! // An item is anything; here a (name, modifier) pair.
//! let sword = ("sword", StatsModifier { attack: 4, ..Default::default() });
//! let helm = ("helm", StatsModifier { defense: 2, max_hp: 5, ..Default::default() });
//!
//! let mut gear: Equipment<(&str, StatsModifier)> = Equipment::new();
//! gear.equip(EquipSlot::MainHand, sword);
//! gear.equip(EquipSlot::Head, helm);
//!
//! // Fold all worn modifiers into the wearer's base stats.
//! let total = gear.aggregate(|item| item.1);
//! let effective = Stats::new(20, 3, 1).modified(&total);
//! assert_eq!(effective.attack, 7);   // 3 + 4
//! assert_eq!(effective.defense, 3);  // 1 + 2
//! assert_eq!(effective.max_hp, 25);  // 20 + 5
//! ```
//!
//! Determinism: slots have a fixed enumeration order ([`EquipSlot::ALL`]) and
//! the loadout is a fixed-length array — no `HashMap`, no allocation-order
//! dependence. [`Equipment`] implements
//! [`DetHash`] over occupancy + each worn item, so a
//! creature's gear folds into the replay checksum.
//!
//! ## Cursed items
//!
//! A cursed item is a NetHack-style mechanic nothing else in the kit
//! modeled: an equipped item the wearer cannot freely remove or swap out.
//! [`curse`](Equipment::curse) marks the item currently worn in a slot;
//! [`is_locked`](Equipment::is_locked) tells a caller whether normal
//! equip/unequip should be allowed there. [`equip`](Equipment::equip) and
//! [`unequip`](Equipment::unequip) remain **unconditional** (any change
//! their existing contract would break) — checking
//! [`is_locked`](Equipment::is_locked) first is the caller's job, exactly
//! the `can_X` / `X` pairing used by [`Wallet`](crate::wallet::Wallet)'s
//! `can_afford`/`withdraw` and [`Shop`](crate::shop::Shop)'s `can_buy`/`buy`.
//! A curse belongs to the *item currently in the slot*: both `equip` and
//! `unequip` clear the flag as part of changing what occupies the slot, so a
//! freshly-equipped item is never born cursed and an empty slot is never
//! reported as locked.

use crate::combat::StatsModifier;
use crate::world_hash::{DetHash, Fnv1a};

/// A body slot an item can be worn in. The set is fixed and ordered so that
/// iteration and hashing are deterministic across platforms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum EquipSlot {
    /// Primary weapon hand.
    MainHand = 0,
    /// Shield / second weapon / focus.
    OffHand = 1,
    /// Helmet / hat.
    Head = 2,
    /// Body armor / robe.
    Body = 3,
    /// Gloves / gauntlets.
    Hands = 4,
    /// Boots / greaves.
    Feet = 5,
    /// First ring.
    Ring1 = 6,
    /// Second ring.
    Ring2 = 7,
    /// Necklace / amulet.
    Amulet = 8,
}

impl EquipSlot {
    /// Every slot in canonical order. The length of this slice is
    /// [`Equipment::slot_count`].
    pub const ALL: [EquipSlot; 9] = [
        EquipSlot::MainHand,
        EquipSlot::OffHand,
        EquipSlot::Head,
        EquipSlot::Body,
        EquipSlot::Hands,
        EquipSlot::Feet,
        EquipSlot::Ring1,
        EquipSlot::Ring2,
        EquipSlot::Amulet,
    ];

    /// The slot's stable index in `0..9`, matching its position in
    /// [`EquipSlot::ALL`].
    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }

    /// Recover a slot from its [`index`](Self::index). Returns `None` if out of
    /// range — the inverse of `index` over `0..9`.
    #[inline]
    pub fn from_index(i: usize) -> Option<EquipSlot> {
        EquipSlot::ALL.get(i).copied()
    }
}

const SLOT_COUNT: usize = EquipSlot::ALL.len();

/// A loadout of worn items: at most one item per [`EquipSlot`].
#[derive(Clone, Debug)]
pub struct Equipment<T> {
    slots: [Option<T>; SLOT_COUNT],
    cursed: [bool; SLOT_COUNT],
}

impl<T> Default for Equipment<T> {
    fn default() -> Self {
        Equipment {
            slots: Default::default(),
            cursed: [false; SLOT_COUNT],
        }
    }
}

impl<T> Equipment<T> {
    /// Create an empty loadout — every slot vacant.
    pub fn new() -> Self {
        Equipment::default()
    }

    /// The number of slots (constant: 9).
    #[inline]
    pub fn slot_count(&self) -> usize {
        SLOT_COUNT
    }

    /// Borrow the item worn in `slot`, or `None` if the slot is empty.
    #[inline]
    pub fn get(&self, slot: EquipSlot) -> Option<&T> {
        self.slots[slot.index()].as_ref()
    }

    /// Mutably borrow the item worn in `slot`.
    #[inline]
    pub fn get_mut(&mut self, slot: EquipSlot) -> Option<&mut T> {
        self.slots[slot.index()].as_mut()
    }

    /// `true` if `slot` currently holds an item.
    #[inline]
    pub fn is_equipped(&self, slot: EquipSlot) -> bool {
        self.slots[slot.index()].is_some()
    }

    /// Put `item` in `slot`, returning whatever was previously worn there (the
    /// swapped-out item, or `None` if the slot was empty). Total worn-item count
    /// is conserved: one in, at most one out. Unconditional — bypasses any
    /// curse on the outgoing item (see the module docs); check
    /// [`is_locked`](Self::is_locked) first if the caller wants to respect
    /// curses. Clears the slot's curse flag: the incoming item is never
    /// born cursed.
    pub fn equip(&mut self, slot: EquipSlot, item: T) -> Option<T> {
        self.cursed[slot.index()] = false;
        self.slots[slot.index()].replace(item)
    }

    /// Remove and return the item worn in `slot`, leaving it empty.
    /// Unconditional — bypasses any curse (see the module docs); check
    /// [`is_locked`](Self::is_locked) first if the caller wants to respect
    /// curses. Clears the slot's curse flag along with the item.
    pub fn unequip(&mut self, slot: EquipSlot) -> Option<T> {
        self.cursed[slot.index()] = false;
        self.slots[slot.index()].take()
    }

    /// Curse the item currently worn in `slot`. Returns `true` if the slot
    /// was occupied (the curse takes effect); `false` for an empty slot
    /// (no-op — there is nothing to curse) or a slot already cursed
    /// (idempotent).
    pub fn curse(&mut self, slot: EquipSlot) -> bool {
        if !self.is_equipped(slot) || self.cursed[slot.index()] {
            return false;
        }
        self.cursed[slot.index()] = true;
        true
    }

    /// Lift the curse on `slot`, if any. Returns `true` if it had been
    /// cursed.
    pub fn uncurse(&mut self, slot: EquipSlot) -> bool {
        let was_cursed = self.cursed[slot.index()];
        self.cursed[slot.index()] = false;
        was_cursed
    }

    /// `true` if `slot` currently holds a cursed item. Always `false` for an
    /// empty slot.
    #[inline]
    pub fn is_cursed(&self, slot: EquipSlot) -> bool {
        self.cursed[slot.index()]
    }

    /// `true` if `slot` should refuse normal equip/unequip: occupied **and**
    /// cursed. An empty slot, or an occupied-but-uncursed slot, is never
    /// locked. This is a pure query — [`equip`](Self::equip) and
    /// [`unequip`](Self::unequip) do not consult it themselves.
    #[inline]
    pub fn is_locked(&self, slot: EquipSlot) -> bool {
        self.is_equipped(slot) && self.is_cursed(slot)
    }

    /// The number of occupied slots.
    pub fn occupied_count(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }

    /// The number of empty slots. Always `slot_count() - occupied_count()`.
    pub fn empty_count(&self) -> usize {
        SLOT_COUNT - self.occupied_count()
    }

    /// `true` if nothing is worn at all.
    pub fn is_empty(&self) -> bool {
        self.occupied_count() == 0
    }

    /// Remove every worn item, leaving all slots empty. Clears every curse
    /// flag too, maintaining the invariant that an empty slot is never
    /// reported as cursed.
    pub fn clear(&mut self) {
        for s in &mut self.slots {
            *s = None;
        }
        self.cursed = [false; SLOT_COUNT];
    }

    /// Iterate over `(slot, &item)` for every occupied slot, in canonical
    /// [`EquipSlot::ALL`] order.
    pub fn iter(&self) -> impl Iterator<Item = (EquipSlot, &T)> {
        EquipSlot::ALL
            .iter()
            .filter_map(move |&slot| self.get(slot).map(|item| (slot, item)))
    }

    /// Fold every worn item's [`StatsModifier`] into one, via
    /// [`StatsModifier::combine`]. `modifier_of` extracts an item's bonus.
    /// Empty slots contribute the identity (`Default`), so an empty loadout
    /// aggregates to [`StatsModifier::default`]. Slots are visited in canonical
    /// order, making the saturating sum fully deterministic.
    pub fn aggregate(&self, mut modifier_of: impl FnMut(&T) -> StatsModifier) -> StatsModifier {
        let mut total = StatsModifier::default();
        for slot in EquipSlot::ALL {
            if let Some(item) = self.get(slot) {
                total = total.combine(&modifier_of(item));
            }
        }
        total
    }
}

impl<T: DetHash> DetHash for Equipment<T> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        for slot in EquipSlot::ALL {
            match self.get(slot) {
                Some(item) => {
                    hasher.write_u8(1);
                    item.det_hash(hasher);
                    hasher.write_bool(self.is_cursed(slot));
                }
                None => hasher.write_u8(0),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::Stats;
    use crate::world_hash::hash_state;

    fn m(attack: i32, defense: i32, max_hp: i32) -> StatsModifier {
        StatsModifier {
            attack,
            defense,
            max_hp,
        }
    }

    #[test]
    fn test_new_is_empty() {
        let gear: Equipment<(&str, StatsModifier)> = Equipment::new();
        assert!(gear.is_empty());
        assert_eq!(gear.occupied_count(), 0);
        assert_eq!(gear.empty_count(), 9);
        assert_eq!(gear.slot_count(), 9);
    }

    #[test]
    fn test_equip_into_empty_returns_none() {
        let mut gear = Equipment::new();
        assert_eq!(gear.equip(EquipSlot::MainHand, "sword"), None);
        assert!(gear.is_equipped(EquipSlot::MainHand));
        assert_eq!(gear.get(EquipSlot::MainHand), Some(&"sword"));
    }

    #[test]
    fn test_equip_into_occupied_swaps() {
        let mut gear = Equipment::new();
        gear.equip(EquipSlot::MainHand, "dagger");
        let old = gear.equip(EquipSlot::MainHand, "sword");
        assert_eq!(old, Some("dagger"), "occupant must be returned");
        assert_eq!(gear.get(EquipSlot::MainHand), Some(&"sword"));
        assert_eq!(gear.occupied_count(), 1, "swap keeps count at one");
    }

    #[test]
    fn test_unequip_round_trip() {
        let mut gear = Equipment::new();
        gear.equip(EquipSlot::Head, "helm");
        assert_eq!(gear.unequip(EquipSlot::Head), Some("helm"));
        assert!(!gear.is_equipped(EquipSlot::Head));
        assert_eq!(
            gear.unequip(EquipSlot::Head),
            None,
            "second unequip is empty"
        );
    }

    #[test]
    fn test_occupied_plus_empty_is_slot_count() {
        let mut gear = Equipment::new();
        gear.equip(EquipSlot::MainHand, 1u32);
        gear.equip(EquipSlot::Ring1, 2);
        gear.equip(EquipSlot::Amulet, 3);
        assert_eq!(gear.occupied_count(), 3);
        assert_eq!(
            gear.occupied_count() + gear.empty_count(),
            gear.slot_count()
        );
    }

    #[test]
    fn test_aggregate_empty_is_default() {
        let gear: Equipment<StatsModifier> = Equipment::new();
        assert_eq!(gear.aggregate(|&item| item), StatsModifier::default());
    }

    #[test]
    fn test_aggregate_sums_worn_modifiers() {
        let mut gear = Equipment::new();
        gear.equip(EquipSlot::MainHand, m(4, 0, 0));
        gear.equip(EquipSlot::Head, m(0, 2, 5));
        gear.equip(EquipSlot::Body, m(-1, 3, 10));
        let total = gear.aggregate(|&item| item);
        assert_eq!(total, m(3, 5, 15));
    }

    #[test]
    fn test_aggregate_folds_into_stats() {
        let mut gear = Equipment::new();
        gear.equip(EquipSlot::MainHand, m(4, 0, 0));
        gear.equip(EquipSlot::Head, m(0, 2, 5));
        let effective = Stats::new(20, 3, 1).modified(&gear.aggregate(|&item| item));
        assert_eq!(effective.attack, 7);
        assert_eq!(effective.defense, 3);
        assert_eq!(effective.max_hp, 25);
    }

    #[test]
    fn test_iter_is_canonical_order() {
        let mut gear = Equipment::new();
        gear.equip(EquipSlot::Amulet, "amulet");
        gear.equip(EquipSlot::MainHand, "sword");
        gear.equip(EquipSlot::Feet, "boots");
        let order: Vec<EquipSlot> = gear.iter().map(|(s, _)| s).collect();
        assert_eq!(
            order,
            vec![EquipSlot::MainHand, EquipSlot::Feet, EquipSlot::Amulet]
        );
    }

    #[test]
    fn test_clear_empties_everything() {
        let mut gear = Equipment::new();
        gear.equip(EquipSlot::MainHand, 1u32);
        gear.equip(EquipSlot::Ring2, 2);
        gear.clear();
        assert!(gear.is_empty());
    }

    #[test]
    fn test_slot_index_round_trip() {
        for (i, &slot) in EquipSlot::ALL.iter().enumerate() {
            assert_eq!(slot.index(), i);
            assert_eq!(EquipSlot::from_index(i), Some(slot));
        }
        assert_eq!(EquipSlot::from_index(9), None);
    }

    #[test]
    fn test_get_mut_edits_in_place() {
        let mut gear = Equipment::new();
        gear.equip(EquipSlot::MainHand, 10u32);
        *gear.get_mut(EquipSlot::MainHand).unwrap() += 5;
        assert_eq!(gear.get(EquipSlot::MainHand), Some(&15));
        assert!(gear.get_mut(EquipSlot::OffHand).is_none());
    }

    #[test]
    fn test_det_hash_canonical_and_sensitive() {
        let mut a = Equipment::new();
        a.equip(EquipSlot::MainHand, 7u32);
        let mut b = Equipment::new();
        b.equip(EquipSlot::MainHand, 7u32);
        assert_eq!(hash_state(&a), hash_state(&b), "same loadout, same hash");

        let mut c = a.clone();
        c.equip(EquipSlot::Head, 1u32);
        assert_ne!(
            hash_state(&a),
            hash_state(&c),
            "extra item must change hash"
        );

        // Slot matters: same item in a different slot hashes differently.
        let mut d = Equipment::new();
        d.equip(EquipSlot::OffHand, 7u32);
        assert_ne!(hash_state(&a), hash_state(&d), "slot placement must matter");
    }

    // --- curses -----------------------------------------------------------

    #[test]
    fn test_curse_on_occupied_slot_returns_true() {
        let mut gear = Equipment::new();
        gear.equip(EquipSlot::MainHand, "sword");
        assert!(gear.curse(EquipSlot::MainHand));
        assert!(gear.is_cursed(EquipSlot::MainHand));
    }

    #[test]
    fn test_curse_on_empty_slot_is_noop() {
        let mut gear: Equipment<&str> = Equipment::new();
        assert!(!gear.curse(EquipSlot::MainHand), "nothing to curse");
        assert!(!gear.is_cursed(EquipSlot::MainHand));
    }

    #[test]
    fn test_curse_is_idempotent() {
        let mut gear = Equipment::new();
        gear.equip(EquipSlot::MainHand, "sword");
        assert!(gear.curse(EquipSlot::MainHand));
        assert!(!gear.curse(EquipSlot::MainHand), "already cursed, no-op");
    }

    #[test]
    fn test_is_locked_requires_both_occupied_and_cursed() {
        let mut gear = Equipment::new();
        assert!(
            !gear.is_locked(EquipSlot::MainHand),
            "empty is never locked"
        );
        gear.equip(EquipSlot::MainHand, "sword");
        assert!(
            !gear.is_locked(EquipSlot::MainHand),
            "occupied but uncursed"
        );
        gear.curse(EquipSlot::MainHand);
        assert!(gear.is_locked(EquipSlot::MainHand), "occupied and cursed");
    }

    #[test]
    fn test_uncurse_lifts_the_curse() {
        let mut gear = Equipment::new();
        gear.equip(EquipSlot::MainHand, "sword");
        gear.curse(EquipSlot::MainHand);
        assert!(gear.uncurse(EquipSlot::MainHand), "was cursed");
        assert!(!gear.is_cursed(EquipSlot::MainHand));
        assert!(!gear.is_locked(EquipSlot::MainHand));
    }

    #[test]
    fn test_uncurse_on_not_cursed_returns_false() {
        let mut gear = Equipment::new();
        gear.equip(EquipSlot::MainHand, "sword");
        assert!(!gear.uncurse(EquipSlot::MainHand), "was never cursed");
    }

    #[test]
    fn test_unequip_still_removes_a_cursed_item_unconditionally() {
        // unequip's existing contract is unconditional; curses are a
        // caller-enforced convention via is_locked, not an engine gate.
        let mut gear = Equipment::new();
        gear.equip(EquipSlot::MainHand, "sword");
        gear.curse(EquipSlot::MainHand);
        assert_eq!(gear.unequip(EquipSlot::MainHand), Some("sword"));
        assert!(!gear.is_equipped(EquipSlot::MainHand));
    }

    #[test]
    fn test_unequip_clears_the_curse_flag() {
        let mut gear = Equipment::new();
        gear.equip(EquipSlot::MainHand, "sword");
        gear.curse(EquipSlot::MainHand);
        gear.unequip(EquipSlot::MainHand);
        assert!(
            !gear.is_cursed(EquipSlot::MainHand),
            "empty slot is never cursed"
        );
        // Equipping a fresh item afterward must start uncursed.
        gear.equip(EquipSlot::MainHand, "dagger");
        assert!(!gear.is_cursed(EquipSlot::MainHand));
    }

    #[test]
    fn test_equip_swap_clears_the_outgoing_curse() {
        let mut gear = Equipment::new();
        gear.equip(EquipSlot::MainHand, "cursed dagger");
        gear.curse(EquipSlot::MainHand);
        let old = gear.equip(EquipSlot::MainHand, "sword");
        assert_eq!(
            old,
            Some("cursed dagger"),
            "outgoing item is still returned"
        );
        assert!(
            !gear.is_cursed(EquipSlot::MainHand),
            "incoming item is never born cursed"
        );
    }

    #[test]
    fn test_clear_resets_all_curse_flags() {
        let mut gear = Equipment::new();
        gear.equip(EquipSlot::MainHand, "sword");
        gear.curse(EquipSlot::MainHand);
        gear.equip(EquipSlot::Head, "helm");
        gear.curse(EquipSlot::Head);
        gear.clear();
        assert!(!gear.is_cursed(EquipSlot::MainHand));
        assert!(!gear.is_cursed(EquipSlot::Head));
    }

    #[test]
    fn test_curses_are_independent_per_slot() {
        let mut gear = Equipment::new();
        gear.equip(EquipSlot::MainHand, "sword");
        gear.equip(EquipSlot::Head, "helm");
        gear.curse(EquipSlot::MainHand);
        assert!(gear.is_cursed(EquipSlot::MainHand));
        assert!(
            !gear.is_cursed(EquipSlot::Head),
            "curse does not leak to other slots"
        );
    }

    #[test]
    fn test_det_hash_sensitive_to_curse_state() {
        let mut a = Equipment::new();
        a.equip(EquipSlot::MainHand, 7u32);
        let mut b = a.clone();
        assert_eq!(
            hash_state(&a),
            hash_state(&b),
            "identical clones hash equal"
        );

        b.curse(EquipSlot::MainHand);
        assert_ne!(
            hash_state(&a),
            hash_state(&b),
            "cursed vs uncursed must hash differently"
        );
    }

    #[test]
    fn test_det_hash_unaffected_by_curse_on_empty_slot() {
        // curse() on an empty slot is a documented no-op; the hash must
        // reflect that (no phantom state leaks in for an unoccupied slot).
        let a: Equipment<u32> = Equipment::new();
        let mut b: Equipment<u32> = Equipment::new();
        b.curse(EquipSlot::MainHand); // no-op, slot is empty
        assert_eq!(hash_state(&a), hash_state(&b));
    }
}
