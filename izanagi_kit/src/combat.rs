//! Integer combat formula — deterministic damage and HP tracking.
//!
//! Roguelike combat can be as simple as `damage = attack - defense, min 1`.
//! This module provides the standard building blocks: stat blocks, hit rolls
//! via `SplitMix64`, and a damage pipeline. All arithmetic is integer; no
//! float anywhere in the calculation so results are bit-identical across
//! targets and safe to fold into the world hash.
//!
//! The formulas are intentionally simple and composable — callers assemble
//! higher-level systems (flanking, crits, resistances) from these primitives.

use crate::{
    rng::SplitMix64,
    world_hash::{DetHash, Fnv1a},
};

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

/// A minimal stat block for a combatant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stats {
    /// Current / max HP pair.
    pub hp: i32,
    pub max_hp: i32,
    /// Offensive power (raw damage before defense).
    pub attack: i32,
    /// Incoming damage reduction (subtracted from attack, minimum 1 damage).
    pub defense: i32,
}

impl Stats {
    pub fn new(hp: i32, attack: i32, defense: i32) -> Self {
        let hp = hp.max(0);
        Stats {
            hp,
            max_hp: hp,
            attack,
            defense,
        }
    }

    /// Whether this combatant is alive (hp > 0).
    #[inline]
    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }

    /// Whether this combatant is dead (`hp <= 0`). The explicit complement of
    /// `is_alive` — avoids the `!is_alive()` double-negation at sites where
    /// the positive "is dead?" condition is clearer: death triggers, loot drops,
    /// and "remove all dead entities this tick" cleanup passes.
    #[inline]
    pub fn is_dead(&self) -> bool {
        self.hp <= 0
    }

    /// Apply `amount` damage (clamped to the current HP; HP floor is 0).
    #[inline]
    pub fn take_damage(&mut self, amount: i32) {
        self.hp = (self.hp - amount.max(0)).max(0);
    }

    /// Heal by `amount` (clamped to `max_hp`; no overheal).
    #[inline]
    pub fn heal(&mut self, amount: i32) {
        self.hp = (self.hp + amount.max(0)).min(self.max_hp);
    }

    /// Restore HP to full (`max_hp`).
    #[inline]
    pub fn restore(&mut self) {
        self.hp = self.max_hp;
    }

    /// Set a new `max_hp` (clamped to ≥ 0) and clamp current HP to the new
    /// ceiling. Use this for level-ups or permanent HP changes.
    pub fn set_max_hp(&mut self, new_max: i32) {
        self.max_hp = new_max.max(0);
        self.hp = self.hp.min(self.max_hp);
    }

    /// Proportion of HP remaining as a fraction `(hp, max_hp)`.
    #[inline]
    pub fn hp_fraction(&self) -> (i32, i32) {
        (self.hp, self.max_hp.max(1))
    }

    /// HP remaining as an integer percentage in `[0, 100]`. Saturates: HP
    /// above `max_hp` returns 100, zero or negative HP returns 0. Useful for
    /// `BarWidget` fill values and health thresholds without float division.
    #[inline]
    pub fn hp_percent(&self) -> u32 {
        if self.max_hp <= 0 {
            return 0;
        }
        (self.hp.max(0) as u64 * 100 / self.max_hp as u64).min(100) as u32
    }

    /// Health deficit below the maximum: `max(0, max_hp − hp)`. Returns the
    /// amount of healing needed to reach full HP. `0` at (or above) full HP.
    /// Useful for healing AI ("how much do I need?") and damage-preview UI.
    #[inline]
    pub fn missing_hp(&self) -> i32 {
        (self.max_hp - self.hp).max(0)
    }

    /// Apply `amount` damage and return the **overkill** — the excess damage
    /// beyond the current HP (always ≥ 0). HP is clamped to 0 as usual.
    ///
    /// Useful for death events and chaining AoE damage to other targets.
    #[inline]
    pub fn take_overkill_damage(&mut self, amount: i32) -> i32 {
        let amount = amount.max(0);
        let overkill = (amount - self.hp).max(0);
        self.hp = (self.hp - amount).max(0);
        overkill
    }

    /// Returns `true` when this combatant's HP is below half of `max_hp`
    /// (the D&D "bloodied" condition). `false` when `max_hp == 0`.
    ///
    /// Useful for AI aggression triggers, visual damage indicators, and
    /// conditional abilities that activate when the target is weakened.
    #[inline]
    pub fn is_bloodied(&self) -> bool {
        self.max_hp > 0 && self.hp * 2 < self.max_hp
    }

    /// Returns `true` when HP is at or above `max_hp`.
    /// Complements `is_bloodied`; useful for suppressing heal prompts and
    /// disabling "restore" ability options when already at full health.
    #[inline]
    pub fn is_full_hp(&self) -> bool {
        self.hp >= self.max_hp
    }
}

impl DetHash for Stats {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_i32(self.hp);
        hasher.write_i32(self.max_hp);
        hasher.write_i32(self.attack);
        hasher.write_i32(self.defense);
    }
}

// ---------------------------------------------------------------------------
// Stat modifiers
// ---------------------------------------------------------------------------

/// Additive stat modifier applied to a [`Stats`] snapshot — for equipment,
/// spells, and buffs that temporarily or permanently change a combatant's
/// capabilities. All fields are signed: positive = bonus, negative = penalty.
/// Apply with [`Stats::modified`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatsModifier {
    /// Bonus or penalty added to `attack`.
    pub attack: i32,
    /// Bonus or penalty added to `defense`.
    pub defense: i32,
    /// Bonus or penalty added to `max_hp`. A positive value increases the HP
    /// ceiling; a negative value reduces it (current HP is clamped to the new
    /// ceiling). `max_hp` is always clamped to `≥ 0` after application.
    pub max_hp: i32,
}

impl Stats {
    /// Return a new `Stats` with `modifier` applied additively.
    ///
    /// `max_hp` is clamped to `≥ 0`; current `hp` is clamped to the new
    /// `max_hp` (it can only decrease — the modifier does not heal). `attack`
    /// and `defense` are unbounded signed integers; callers should validate
    /// that gameplay invariants hold after application.
    pub fn modified(&self, modifier: &StatsModifier) -> Stats {
        let new_max_hp = (self.max_hp + modifier.max_hp).max(0);
        Stats {
            hp: self.hp.min(new_max_hp),
            max_hp: new_max_hp,
            attack: self.attack + modifier.attack,
            defense: self.defense + modifier.defense,
        }
    }
}

// ---------------------------------------------------------------------------
// Combat resolution
// ---------------------------------------------------------------------------

/// Compute raw damage dealt by `attacker` against `defender`.
/// Formula: `max(1, attacker.attack - defender.defense)`.
/// Always deals at least 1 damage (the standard roguelike minimum).
#[inline]
pub fn base_damage(attacker: &Stats, defender: &Stats) -> i32 {
    (attacker.attack - defender.defense).max(1)
}

/// Resolve one melee attack: compute damage and apply it to `defender`.
/// Returns the damage dealt.
pub fn melee_attack(attacker: &Stats, defender: &mut Stats) -> i32 {
    let dmg = base_damage(attacker, defender);
    defender.take_damage(dmg);
    dmg
}

/// Hit/miss roll. Returns `true` if the attacker hits.
/// Uses a single `rng` draw; `hit_chance` is in 1..=100 (percentage).
/// Values ≤0 always miss; values ≥100 always hit (no draw consumed for degenerate cases).
pub fn roll_to_hit(rng: &mut SplitMix64, hit_chance: i32) -> bool {
    if hit_chance <= 0 {
        return false;
    }
    if hit_chance >= 100 {
        return true;
    }
    rng.below(100) < hit_chance as u32
}

/// The result of a [`critical_strike`]: damage dealt and whether it was a crit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StrikeResult {
    /// Total damage dealt to the defender.
    pub damage: i32,
    /// Whether a critical hit occurred.
    pub critical: bool,
}

/// Melee attack with a critical-hit chance.
///
/// `crit_chance` is in 1..=100 (percent). On a critical hit the base damage is
/// multiplied by `crit_multiplier` (typical value: `2` for double damage).
/// At `crit_chance <= 0` crits never occur; at `crit_chance >= 100` every
/// strike is critical. The multiplier is clamped to `≥ 1`.
///
/// Uses one [`SplitMix64`] draw for the crit roll (or none for degenerate
/// `crit_chance`), matching [`roll_to_hit`]'s RNG contract.
pub fn critical_strike(
    rng: &mut SplitMix64,
    attacker: &Stats,
    defender: &mut Stats,
    crit_chance: i32,
    crit_multiplier: i32,
) -> StrikeResult {
    let base = base_damage(attacker, defender);
    let critical = roll_to_hit(rng, crit_chance);
    let mult = if critical { crit_multiplier.max(1) } else { 1 };
    let damage = (base as i64 * mult as i64).min(i32::MAX as i64) as i32;
    defender.take_damage(damage);
    StrikeResult { damage, critical }
}

/// Roll base damage plus a random variance component.
///
/// Returns `base + rng.below(variance + 1) as i32`, floored at `0`.
/// `variance == 0` always returns `base.max(0)` without consuming an RNG draw.
/// Equivalent to rolling `1d(variance+1) - 1` and adding `base`.
///
/// Useful for giving attacks a natural spread (e.g. `roll_damage(rng, 5, 3)`
/// yields 5–8 damage) without spelling out the Dice formula at every call site.
pub fn roll_damage(rng: &mut SplitMix64, base: i32, variance: u32) -> i32 {
    let bonus = if variance > 0 {
        rng.below(variance + 1) as i32
    } else {
        0
    };
    (base + bonus).max(0)
}

/// Ranged attack with a hit roll. Returns `Some(damage)` on hit, `None` on miss.
pub fn ranged_attack(
    rng: &mut SplitMix64,
    attacker: &Stats,
    defender: &mut Stats,
    hit_chance: i32,
) -> Option<i32> {
    if roll_to_hit(rng, hit_chance) {
        Some(melee_attack(attacker, defender))
    } else {
        None
    }
}

/// Apply area-of-effect damage from `attacker` to every target in `targets`.
///
/// The attacker's effective attack is reduced by `falloff` for each successive
/// target: target `i` receives `max(1, attacker.attack − falloff · i)` raw
/// damage, then the target's own defense is subtracted (`max(1, raw −
/// target.defense)`). Every target always takes at least 1 damage.
///
/// Returns a `Vec<i32>` of the actual damage dealt to each target, in order.
/// Returns an empty `Vec` for an empty slice (no side effects, no RNG draw).
///
/// Typical use: `splash_attack(&mage, &mut targets, 3)` — a fireball centred
/// on the primary target, with each outer ring taking 3 less raw damage.
pub fn splash_attack(attacker: &Stats, targets: &mut [Stats], falloff: i32) -> Vec<i32> {
    targets
        .iter_mut()
        .enumerate()
        .map(|(i, target)| {
            let raw = (attacker.attack - falloff * i as i32).max(1);
            let dmg = (raw - target.defense).max(1);
            target.take_damage(dmg);
            dmg
        })
        .collect()
}

/// Reduce `damage` by a percentage resistance, clamping `resist_percent`
/// to `[0, 100]`. The remaining damage is always ≥ 0. Negative `damage`
/// returns `0`. Formula: `max(0, damage × (100 − resist_percent) / 100)`.
///
/// The standard "armour/resistance" primitive for systems where defensive
/// stats are expressed as a flat percentage rather than an additive defense
/// score — complements [`base_damage`] which uses the subtraction model.
pub fn apply_resistance(damage: i32, resist_percent: u32) -> i32 {
    let resist = resist_percent.min(100) as i64;
    let remaining = damage.max(0) as i64 * (100 - resist) / 100;
    remaining as i32
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    fn attacker() -> Stats {
        Stats::new(20, 8, 2)
    }

    fn defender() -> Stats {
        Stats::new(30, 5, 3)
    }

    #[test]
    fn test_base_damage_is_attack_minus_defense() {
        // attacker.attack=8, defender.defense=3 → 5
        assert_eq!(base_damage(&attacker(), &defender()), 5);
    }

    #[test]
    fn test_base_damage_minimum_one() {
        let weak = Stats::new(10, 1, 0);
        let armored = Stats::new(10, 1, 100);
        assert_eq!(base_damage(&weak, &armored), 1);
    }

    #[test]
    fn test_melee_attack_reduces_defender_hp() {
        let att = attacker();
        let mut def = defender();
        let dmg = melee_attack(&att, &mut def);
        assert_eq!(dmg, 5);
        assert_eq!(def.hp, 25);
    }

    #[test]
    fn test_take_damage_floor_at_zero() {
        let mut s = Stats::new(5, 1, 0);
        s.take_damage(100);
        assert_eq!(s.hp, 0);
        assert!(!s.is_alive());
    }

    #[test]
    fn test_take_damage_negative_is_noop() {
        let mut s = Stats::new(10, 1, 0);
        s.take_damage(-5);
        assert_eq!(s.hp, 10);
    }

    #[test]
    fn test_heal_caps_at_max_hp() {
        let mut s = Stats::new(10, 1, 0);
        s.take_damage(7);
        s.heal(100);
        assert_eq!(s.hp, s.max_hp);
    }

    #[test]
    fn test_heal_negative_is_noop() {
        let mut s = Stats::new(10, 1, 0);
        s.heal(-5);
        assert_eq!(s.hp, 10);
    }

    #[test]
    fn test_is_alive_true_when_hp_positive() {
        assert!(Stats::new(1, 1, 0).is_alive());
    }

    #[test]
    fn test_is_alive_false_at_zero_hp() {
        let mut s = Stats::new(5, 1, 0);
        s.take_damage(5);
        assert!(!s.is_alive());
    }

    #[test]
    fn test_is_dead_at_zero_hp() {
        let mut s = Stats::new(5, 1, 0);
        s.take_damage(5);
        assert!(s.is_dead());
    }

    #[test]
    fn test_is_dead_false_when_alive() {
        let s = Stats::new(10, 2, 1);
        assert!(!s.is_dead());
    }

    #[test]
    fn test_is_dead_complements_is_alive() {
        let s = Stats::new(3, 1, 0);
        assert_ne!(s.is_dead(), s.is_alive());
    }

    #[test]
    fn test_hp_fraction() {
        let mut s = Stats::new(10, 1, 0);
        s.take_damage(3);
        let (hp, max) = s.hp_fraction();
        assert_eq!(hp, 7);
        assert_eq!(max, 10);
    }

    #[test]
    fn test_roll_to_hit_always_miss() {
        let mut r = SplitMix64::new(42);
        let before = r.state();
        assert!(!roll_to_hit(&mut r, 0));
        assert!(!roll_to_hit(&mut r, -5));
        assert_eq!(r.state(), before, "degenerate roll must not draw");
    }

    #[test]
    fn test_roll_to_hit_always_hit() {
        let mut r = SplitMix64::new(42);
        let before = r.state();
        assert!(roll_to_hit(&mut r, 100));
        assert!(roll_to_hit(&mut r, 200));
        assert_eq!(r.state(), before, "certain hit must not draw");
    }

    #[test]
    fn test_roll_to_hit_is_deterministic() {
        let mut a = SplitMix64::new(99);
        let mut b = SplitMix64::new(99);
        let ra: Vec<bool> = (0..50).map(|_| roll_to_hit(&mut a, 70)).collect();
        let rb: Vec<bool> = (0..50).map(|_| roll_to_hit(&mut b, 70)).collect();
        assert_eq!(ra, rb);
    }

    #[test]
    fn test_ranged_attack_hit_deals_damage() {
        let att = attacker();
        let mut def = defender();
        let mut r = SplitMix64::new(0);
        // With 100% hit chance the ranged attack always lands.
        let result = ranged_attack(&mut r, &att, &mut def, 100);
        assert!(result.is_some());
        assert_eq!(def.hp, 25);
    }

    #[test]
    fn test_ranged_attack_miss_leaves_hp_unchanged() {
        let att = attacker();
        let mut def = defender();
        let mut r = SplitMix64::new(0);
        let result = ranged_attack(&mut r, &att, &mut def, 0);
        assert_eq!(result, None);
        assert_eq!(def.hp, 30);
    }

    #[test]
    fn test_det_hash_changes_on_hp_loss() {
        let s1 = defender();
        let mut s2 = defender();
        s2.take_damage(5);
        assert_ne!(hash_state(&s1), hash_state(&s2));
    }

    #[test]
    fn test_det_hash_same_stats_same_hash() {
        let s1 = defender();
        let s2 = defender();
        assert_eq!(hash_state(&s1), hash_state(&s2));
    }

    #[test]
    fn test_restore_fills_hp_to_max() {
        let mut s = Stats::new(20, 5, 2);
        s.take_damage(15);
        assert_eq!(s.hp, 5);
        s.restore();
        assert_eq!(s.hp, 20);
        assert_eq!(s.hp, s.max_hp);
    }

    #[test]
    fn test_set_max_hp_clamps_current_hp() {
        let mut s = Stats::new(20, 5, 2);
        s.set_max_hp(10);
        assert_eq!(s.max_hp, 10);
        assert_eq!(s.hp, 10); // clamped from 20
    }

    #[test]
    fn test_set_max_hp_increase_does_not_auto_heal() {
        let mut s = Stats::new(10, 5, 2);
        s.take_damage(5); // hp = 5
        s.set_max_hp(30); // max raised but hp stays at 5
        assert_eq!(s.hp, 5);
        assert_eq!(s.max_hp, 30);
    }

    #[test]
    fn test_critical_strike_certain_crit_doubles_damage() {
        let att = attacker();
        let mut def = defender();
        let mut rng = SplitMix64::new(0);
        let result = critical_strike(&mut rng, &att, &mut def, 100, 2);
        assert!(result.critical);
        // base_damage = 5; doubled = 10
        assert_eq!(result.damage, 10);
        assert_eq!(def.hp, 20); // 30 - 10
    }

    #[test]
    fn test_critical_strike_no_crit_at_zero_chance() {
        let att = attacker();
        let mut def = defender();
        let mut rng = SplitMix64::new(0);
        let before = rng.state();
        let result = critical_strike(&mut rng, &att, &mut def, 0, 2);
        assert!(!result.critical);
        assert_eq!(result.damage, 5); // base damage only
                                      // crit_chance=0 must not draw from rng (same contract as roll_to_hit)
        assert_eq!(rng.state(), before);
    }

    #[test]
    fn test_critical_strike_is_deterministic() {
        let mut rng_a = SplitMix64::new(42);
        let mut rng_b = SplitMix64::new(42);
        let results: Vec<StrikeResult> = (0..20)
            .map(|_| {
                let att = attacker();
                let mut def = defender();
                critical_strike(&mut rng_a, &att, &mut def, 25, 2)
            })
            .collect();
        let results2: Vec<StrikeResult> = (0..20)
            .map(|_| {
                let att = attacker();
                let mut def = defender();
                critical_strike(&mut rng_b, &att, &mut def, 25, 2)
            })
            .collect();
        assert_eq!(results, results2);
    }

    #[test]
    fn test_hp_percent_full_hp() {
        let s = Stats::new(100, 5, 2);
        assert_eq!(s.hp_percent(), 100);
    }

    #[test]
    fn test_hp_percent_half() {
        let mut s = Stats::new(100, 5, 2);
        s.take_damage(50);
        assert_eq!(s.hp_percent(), 50);
    }

    #[test]
    fn test_hp_percent_zero_hp() {
        let mut s = Stats::new(10, 5, 2);
        s.take_damage(10);
        assert_eq!(s.hp_percent(), 0);
    }

    #[test]
    fn test_hp_percent_zero_max_hp_returns_zero() {
        let s = Stats::new(0, 5, 2);
        assert_eq!(s.hp_percent(), 0);
    }

    #[test]
    fn test_take_overkill_damage_exact_kill() {
        let mut s = Stats::new(10, 5, 2);
        let overkill = s.take_overkill_damage(10);
        assert_eq!(overkill, 0);
        assert_eq!(s.hp, 0);
    }

    #[test]
    fn test_take_overkill_damage_excess() {
        let mut s = Stats::new(5, 5, 2);
        let overkill = s.take_overkill_damage(8);
        assert_eq!(overkill, 3);
        assert_eq!(s.hp, 0);
    }

    #[test]
    fn test_take_overkill_damage_no_kill() {
        let mut s = Stats::new(20, 5, 2);
        let overkill = s.take_overkill_damage(6);
        assert_eq!(overkill, 0);
        assert_eq!(s.hp, 14);
    }

    #[test]
    fn test_take_overkill_damage_negative_amount_is_noop() {
        let mut s = Stats::new(10, 5, 2);
        let overkill = s.take_overkill_damage(-5);
        assert_eq!(overkill, 0);
        assert_eq!(s.hp, 10);
    }

    #[test]
    fn test_roll_damage_zero_variance_returns_base() {
        let mut rng = SplitMix64::new(99);
        let state = rng.state();
        let dmg = roll_damage(&mut rng, 7, 0);
        assert_eq!(dmg, 7);
        assert_eq!(
            rng.state(),
            state,
            "zero variance must not consume an RNG draw"
        );
    }

    #[test]
    fn test_roll_damage_with_variance_in_range() {
        let mut rng = SplitMix64::new(1);
        for _ in 0..50 {
            let dmg = roll_damage(&mut rng, 5, 3);
            assert!((5..=8).contains(&dmg), "expected 5..=8 but got {dmg}");
        }
    }

    #[test]
    fn test_roll_damage_negative_base_floored_at_zero() {
        let mut rng = SplitMix64::new(0);
        let dmg = roll_damage(&mut rng, -10, 0);
        assert_eq!(dmg, 0);
    }

    #[test]
    fn test_missing_hp_full_health_is_zero() {
        let s = Stats::new(30, 5, 2);
        assert_eq!(s.missing_hp(), 0);
    }

    #[test]
    fn test_missing_hp_after_damage() {
        let mut s = Stats::new(30, 5, 2);
        s.take_damage(12);
        assert_eq!(s.missing_hp(), 12);
    }

    #[test]
    fn test_missing_hp_never_negative() {
        // Lower max below current hp; deficit clamps at 0.
        let mut s = Stats::new(30, 5, 2);
        s.hp = 40; // contrived overheal
        assert_eq!(s.missing_hp(), 0);
    }

    #[test]
    fn test_apply_resistance_zero_resist_unchanged() {
        assert_eq!(apply_resistance(50, 0), 50);
    }

    #[test]
    fn test_apply_resistance_full_resist_is_zero() {
        assert_eq!(apply_resistance(50, 100), 0);
    }

    #[test]
    fn test_apply_resistance_partial_and_edge_cases() {
        assert_eq!(apply_resistance(50, 50), 25);
        assert_eq!(apply_resistance(100, 25), 75);
        // Negative damage always returns 0.
        assert_eq!(apply_resistance(-10, 50), 0);
        // Over-100 percent clamps to 100.
        assert_eq!(apply_resistance(50, 150), 0);
    }

    #[test]
    fn test_is_bloodied_below_half_hp() {
        let mut s = Stats::new(10, 5, 0);
        s.take_damage(6); // hp = 4, max = 10: 4*2=8 < 10 → bloodied
        assert!(s.is_bloodied());
    }

    #[test]
    fn test_is_bloodied_at_half_hp_is_false() {
        let mut s = Stats::new(10, 5, 0);
        s.take_damage(5); // hp = 5, 5*2 = 10 which is NOT < 10 → not bloodied
        assert!(!s.is_bloodied());
    }

    #[test]
    fn test_is_bloodied_zero_max_hp_is_false() {
        let s = Stats {
            hp: 0,
            max_hp: 0,
            attack: 0,
            defense: 0,
        };
        assert!(!s.is_bloodied());
    }

    #[test]
    fn test_is_full_hp_at_max() {
        let s = Stats::new(10, 5, 2);
        assert!(s.is_full_hp());
    }

    #[test]
    fn test_is_full_hp_false_when_damaged() {
        let mut s = Stats::new(10, 5, 2);
        s.take_damage(1);
        assert!(!s.is_full_hp());
    }

    #[test]
    fn test_is_full_hp_true_when_overfull() {
        let s = Stats {
            hp: 15,
            max_hp: 10,
            attack: 0,
            defense: 0,
        };
        assert!(s.is_full_hp());
    }

    // --- StatsModifier / modified -------------------------------------------

    #[test]
    fn test_modified_applies_attack_bonus() {
        let base = Stats::new(20, 5, 2);
        let m = StatsModifier {
            attack: 3,
            defense: 0,
            max_hp: 0,
        };
        let s = base.modified(&m);
        assert_eq!(s.attack, 8);
        assert_eq!(s.defense, 2);
        assert_eq!(s.max_hp, 20);
        assert_eq!(s.hp, 20);
    }

    #[test]
    fn test_modified_max_hp_reduction_clamps_hp() {
        let mut base = Stats::new(20, 5, 2);
        base.take_damage(5); // hp = 15
        let m = StatsModifier {
            attack: 0,
            defense: 0,
            max_hp: -10, // new max_hp = 10
        };
        let s = base.modified(&m);
        assert_eq!(s.max_hp, 10);
        assert_eq!(s.hp, 10); // clamped from 15
    }

    #[test]
    fn test_modified_max_hp_floor_at_zero() {
        let base = Stats::new(10, 5, 2);
        let m = StatsModifier {
            attack: 0,
            defense: 0,
            max_hp: -999,
        };
        let s = base.modified(&m);
        assert_eq!(s.max_hp, 0);
        assert_eq!(s.hp, 0);
    }

    // --- splash_attack -------------------------------------------------------

    #[test]
    fn test_splash_attack_all_targets_take_damage() {
        let att = Stats::new(20, 10, 0); // attack=10
        let mut targets = vec![
            Stats::new(30, 1, 2), // defense=2, takes max(1, 10-0-2)=8
            Stats::new(30, 1, 2), // takes max(1, 10-3-2)=5
            Stats::new(30, 1, 2), // takes max(1, 10-6-2)=2
        ];
        let dmgs = splash_attack(&att, &mut targets, 3);
        assert_eq!(dmgs, vec![8, 5, 2]);
        assert_eq!(targets[0].hp, 22);
        assert_eq!(targets[1].hp, 25);
        assert_eq!(targets[2].hp, 28);
    }

    #[test]
    fn test_splash_attack_minimum_one_damage() {
        let att = Stats::new(20, 1, 0); // attack=1
        let mut targets = vec![
            Stats::new(30, 1, 0), // raw=1 → dmg=1
            Stats::new(30, 1, 0), // raw=max(1,1-5)=1 → dmg=1
        ];
        let dmgs = splash_attack(&att, &mut targets, 5);
        assert_eq!(dmgs, vec![1, 1]);
        assert!(targets.iter().all(|t| t.hp == 29));
    }

    #[test]
    fn test_splash_attack_empty_targets_is_noop() {
        let att = Stats::new(20, 10, 0);
        let dmgs = splash_attack(&att, &mut [], 3);
        assert!(dmgs.is_empty());
    }
}
