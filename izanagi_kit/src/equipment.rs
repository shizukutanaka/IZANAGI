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
//! [`DetHash`](crate::world_hash::DetHash) over occupancy + each worn item, so a
//! creature's gear folds into the replay checksum.

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
}

impl<T> Default for Equipment<T> {
    fn default() -> Self {
        Equipment {
            slots: Default::default(),
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
    /// is conserved: one in, at most one out.
    pub fn equip(&mut self, slot: EquipSlot, item: T) -> Option<T> {
        self.slots[slot.index()].replace(item)
    }

    /// Remove and return the item worn in `slot`, leaving it empty.
    pub fn unequip(&mut self, slot: EquipSlot) -> Option<T> {
        self.slots[slot.index()].take()
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

    /// Remove every worn item, leaving all slots empty.
    pub fn clear(&mut self) {
        for s in &mut self.slots {
            *s = None;
        }
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
        assert_eq!(gear.unequip(EquipSlot::Head), None, "second unequip is empty");
    }

    #[test]
    fn test_occupied_plus_empty_is_slot_count() {
        let mut gear = Equipment::new();
        gear.equip(EquipSlot::MainHand, 1u32);
        gear.equip(EquipSlot::Ring1, 2);
        gear.equip(EquipSlot::Amulet, 3);
        assert_eq!(gear.occupied_count(), 3);
        assert_eq!(gear.occupied_count() + gear.empty_count(), gear.slot_count());
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
        assert_eq!(order, vec![EquipSlot::MainHand, EquipSlot::Feet, EquipSlot::Amulet]);
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
        assert_ne!(hash_state(&a), hash_state(&c), "extra item must change hash");

        // Slot matters: same item in a different slot hashes differently.
        let mut d = Equipment::new();
        d.equip(EquipSlot::OffHand, 7u32);
        assert_ne!(hash_state(&a), hash_state(&d), "slot placement must matter");
    }
}
