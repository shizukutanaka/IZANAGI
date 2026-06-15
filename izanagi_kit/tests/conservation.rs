//! Conservation / accounting test perspective.
//!
//! The other seven lenses are about *structure and determinism* — hashes, laws,
//! models, oracles, symmetry, ordering, API surface. None checks a *quantitative
//! conservation* invariant: that an operation neither creates nor destroys the
//! quantity it manipulates, and that the books balance. That is a distinct axis,
//! and it is where gameplay-correctness bugs hide: HP healed past its maximum, a
//! leaked or double-counted allocator slot, a duplicated or lost inventory item.
//!
//! The Socratic claim under test: every unit of HP, every entity slot, and every
//! item is exactly accounted for across any sequence of operations — what comes
//! out is precisely what went in, modulo the documented clamps.
//!
//! Deterministic via `SplitMix64`.

use izanagi_kit::{EntityAllocator, Inventory, SplitMix64, Stats};
use std::collections::BTreeMap;

const TRIALS: usize = 400;

/// The allocator's books always balance: `total_slots == count + free_count`,
/// and the live count equals the number of entities `live_entities` reports —
/// no slot is ever leaked, double-counted, or conjured.
#[test]
fn entity_allocator_slot_accounting_balances() {
    let mut rng = SplitMix64::new(0x_ACC0_07_01);
    let mut alloc = EntityAllocator::new();
    let mut pool: Vec<_> = Vec::new();

    for _ in 0..3000 {
        if pool.is_empty() || rng.below(2) == 0 {
            pool.push(alloc.allocate());
        } else {
            let e = pool[rng.below(pool.len() as u32) as usize];
            alloc.free(e);
        }

        // The fundamental accounting identity, after every operation.
        assert_eq!(
            alloc.total_slots(),
            alloc.count() + alloc.free_count(),
            "slot accounting broken: total != count + free"
        );
        // The live count is exactly what the live enumeration yields.
        assert_eq!(
            alloc.count(),
            alloc.live_entities().len(),
            "count disagrees with live_entities length"
        );
    }
}

/// HP is conserved within `[0, max_hp]`: `take_damage` removes exactly
/// `min(amount, hp)` and never drives HP negative; `heal` adds exactly
/// `min(amount, max_hp - hp)` and never overheals. Negative arguments are
/// no-ops. Values are bounded so the (documented) clamps, not i32 overflow, are
/// what is exercised.
#[test]
fn stats_hp_is_conserved_within_bounds() {
    let mut rng = SplitMix64::new(0x_4_9_02);
    for _ in 0..TRIALS {
        let max_hp = rng.range(1, 1000);
        let mut stats = Stats::new(rng.range(0, max_hp + 1), rng.range(0, 50), rng.range(0, 50));
        // (Stats::new sets max_hp = hp; reset max to our chosen ceiling.)
        stats.set_max_hp(max_hp);

        for _ in 0..40 {
            let before = stats.hp;
            // Mix damage, heal, and negative no-ops.
            let amount = rng.range(-20, 2000);
            if rng.below(2) == 0 {
                stats.take_damage(amount);
                let expected_loss = amount.max(0).min(before);
                assert_eq!(stats.hp, before - expected_loss, "take_damage conservation");
                assert!(stats.hp >= 0, "HP went negative");
            } else {
                stats.heal(amount);
                let expected_gain = amount.max(0).min(stats.max_hp - before);
                assert_eq!(stats.hp, before + expected_gain, "heal conservation");
                assert!(stats.hp <= stats.max_hp, "overheal past max_hp");
            }
            assert!((0..=stats.max_hp).contains(&stats.hp), "HP escaped [0, max_hp]");
        }
    }
}

/// Inventory items are conserved: the multiset of items currently held always
/// equals (items added − items removed), occupancy never exceeds capacity, and
/// `add` succeeds iff the inventory was not already full. Nothing is duplicated
/// or lost in a slot.
#[test]
fn inventory_items_are_conserved() {
    let mut rng = SplitMix64::new(0x_111_07_03);
    for _ in 0..TRIALS {
        let cap = rng.range(1, 12) as usize;
        let mut inv: Inventory<u32> = Inventory::new(cap);
        // Reference multiset of items currently present.
        let mut present: BTreeMap<u32, u32> = BTreeMap::new();
        let mut next_item: u32 = 1;

        for _ in 0..60 {
            if rng.below(2) == 0 {
                // add
                let was_full = inv.is_full();
                let item = next_item;
                next_item += 1;
                let slot = inv.add(item);
                if was_full {
                    assert!(slot.is_none(), "add succeeded on a full inventory");
                } else {
                    let s = slot.expect("add failed on a non-full inventory");
                    assert_eq!(inv.get(s), Some(&item), "added item not at returned slot");
                    *present.entry(item).or_insert(0) += 1;
                }
            } else {
                // remove a random slot index in range
                let slot = rng.below(cap as u32) as usize;
                if let Some(item) = inv.remove(slot) {
                    let c = present.get_mut(&item).expect("removed an item never present");
                    *c -= 1;
                    if *c == 0 {
                        present.remove(&item);
                    }
                }
            }

            // Capacity is never exceeded.
            assert!(inv.count_occupied() <= cap, "occupancy exceeds capacity");
            assert_eq!(inv.len(), inv.count_occupied(), "len disagrees with occupancy");
            // The held multiset equals the reference exactly — no dup, no loss.
            let mut held: BTreeMap<u32, u32> = BTreeMap::new();
            for (_, &item) in inv.iter() {
                *held.entry(item).or_insert(0) += 1;
            }
            assert_eq!(held, present, "inventory contents diverged from the accounting model");
        }
    }
}
