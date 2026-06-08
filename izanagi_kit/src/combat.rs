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

    /// Proportion of HP remaining as a fraction `(hp, max_hp)`.
    #[inline]
    pub fn hp_fraction(&self) -> (i32, i32) {
        (self.hp, self.max_hp.max(1))
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
}
