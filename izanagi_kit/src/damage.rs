//! Typed damage and per-type resistance profiles.
//!
//! [`combat`](crate::combat) models damage as a single scalar reduced by a flat
//! `defense` or a percentage via [`combat::apply_resistance`](crate::combat::apply_resistance).
//! Real roguelikes also need **damage typing**: a fire bolt should be soaked by
//! fire resistance but not cold resistance, and a "vulnerable to lightning"
//! target should take *extra* lightning damage. This module supplies that layer
//! without touching the existing combat primitives.
//!
//! Everything here is integer-only and order-deterministic:
//! - [`DamageType`] is a small `#[repr(u8)]` enum with a fixed [`DamageType::ALL`]
//!   ordering, so iteration and hashing are stable across runs and platforms.
//! - [`ResistanceProfile`] stores one `i32` percentage per type in a fixed-size
//!   array (no `HashMap`, so no iteration-order non-determinism).
//! - [`ResistanceProfile`] implements [`DetHash`] so
//!   a creature's resistances fold into the per-frame replay checksum.
//!
//! Resistance semantics (matching [`combat::apply_resistance`](crate::combat::apply_resistance)
//! for the 0..=100 range, extended below and above it):
//! - `resist == 0`   → full damage.
//! - `resist == 100` → no damage (full immunity).
//! - `0 < resist < 100` → partial soak: `damage × (100 − resist) / 100`.
//! - `resist > 100`  → still zero damage (clamped; over-immunity is harmless).
//! - `resist < 0`    → **vulnerability**: takes *more* damage
//!   (`resist == −50` → 1.5× damage).
//! - [`DamageType::True`] always bypasses the profile and deals full damage.

use crate::world_hash::{DetHash, Fnv1a};

/// A category of damage. The discriminants are explicit and stable so the
/// type can be persisted in save files and folded into replay hashes.
///
/// [`DamageType::True`] is special: it ignores all resistances (used for
/// scripted/unavoidable damage such as falling, drowning, or boss "execute"
/// mechanics).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum DamageType {
    /// Melee/kinetic damage.
    Physical = 0,
    /// Fire elemental damage.
    Fire = 1,
    /// Cold elemental damage.
    Cold = 2,
    /// Lightning elemental damage.
    Lightning = 3,
    /// Poison/toxin damage.
    Poison = 4,
    /// Arcane/magic damage.
    Arcane = 5,
    /// Unresistable damage. [`ResistanceProfile::apply`] returns the input
    /// unchanged for this type regardless of the profile.
    True = 6,
}

impl DamageType {
    /// Every damage type in canonical (discriminant) order. Iterate this for
    /// deterministic enumeration — never rely on `HashMap` ordering.
    pub const ALL: [DamageType; 7] = [
        DamageType::Physical,
        DamageType::Fire,
        DamageType::Cold,
        DamageType::Lightning,
        DamageType::Poison,
        DamageType::Arcane,
        DamageType::True,
    ];

    /// The number of distinct damage types (including [`DamageType::True`]).
    pub const COUNT: usize = 7;

    /// The discriminant as a `usize` index into a per-type array.
    #[inline]
    pub fn index(self) -> usize {
        self as usize
    }

    /// Reconstruct a `DamageType` from its discriminant, or `None` if `idx`
    /// is out of range. Inverse of [`index`](Self::index).
    #[inline]
    pub fn from_index(idx: usize) -> Option<DamageType> {
        DamageType::ALL.get(idx).copied()
    }
}

impl DetHash for DamageType {
    #[inline]
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u8(*self as u8);
    }
}

/// A creature's resistance to each [`DamageType`], stored as a percentage in a
/// fixed-size array. Positive = soak, negative = vulnerability, `>= 100` =
/// immune. [`DamageType::True`]'s slot is stored but never consulted by
/// [`apply`](Self::apply).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResistanceProfile {
    resist: [i32; DamageType::COUNT],
}

impl Default for ResistanceProfile {
    fn default() -> Self {
        ResistanceProfile::new()
    }
}

impl ResistanceProfile {
    /// A profile with zero resistance to every type (full damage taken).
    #[inline]
    pub fn new() -> Self {
        ResistanceProfile {
            resist: [0; DamageType::COUNT],
        }
    }

    /// A profile with the same `percent` resistance to every type. `True`
    /// damage still bypasses it at [`apply`](Self::apply) time.
    #[inline]
    pub fn uniform(percent: i32) -> Self {
        ResistanceProfile {
            resist: [percent; DamageType::COUNT],
        }
    }

    /// Builder: set `ty`'s resistance to `percent`, returning `self`.
    /// Chain calls to construct a profile in one expression.
    #[inline]
    pub fn with(mut self, ty: DamageType, percent: i32) -> Self {
        self.resist[ty.index()] = percent;
        self
    }

    /// Resistance percentage to `ty`. Defaults to `0` for a fresh profile.
    #[inline]
    pub fn get(&self, ty: DamageType) -> i32 {
        self.resist[ty.index()]
    }

    /// Set `ty`'s resistance to `percent` in place.
    #[inline]
    pub fn set(&mut self, ty: DamageType, percent: i32) {
        self.resist[ty.index()] = percent;
    }

    /// Add `delta` to `ty`'s resistance (saturating). Useful for temporary
    /// buffs ("+25% fire resist for 3 turns") layered over a base profile.
    #[inline]
    pub fn add(&mut self, ty: DamageType, delta: i32) {
        let slot = &mut self.resist[ty.index()];
        *slot = slot.saturating_add(delta);
    }

    /// `true` if `ty` is fully resisted (resistance `>= 100`). [`DamageType::True`]
    /// is never immune through this check — it bypasses resistance at apply time
    /// instead, so a `True` slot of 100 here is reported as immune but ignored by
    /// [`apply`](Self::apply).
    #[inline]
    pub fn is_immune(&self, ty: DamageType) -> bool {
        self.get(ty) >= 100
    }

    /// `true` if `ty`'s resistance is negative (the creature takes extra damage).
    #[inline]
    pub fn is_vulnerable(&self, ty: DamageType) -> bool {
        self.get(ty) < 0
    }

    /// Apply this profile to `damage` of type `ty`, returning the post-resistance
    /// amount (always `>= 0`).
    ///
    /// - [`DamageType::True`] returns `damage.max(0)` unchanged.
    /// - Otherwise: `max(0, damage × (100 − resist) / 100)`, where `resist` is
    ///   clamped at the top to 100 (so over-immunity never produces *negative*
    ///   healing) but negative `resist` is honoured to amplify damage.
    ///
    /// The intermediate product uses `i64`, so large damage values and large
    /// vulnerabilities never overflow before the final clamp back to `i32`.
    pub fn apply(&self, damage: i32, ty: DamageType) -> i32 {
        if ty == DamageType::True {
            return damage.max(0);
        }
        let dmg = damage.max(0) as i64;
        // Clamp resistance at 100 (full soak); allow negatives for vulnerability.
        let resist = (self.get(ty) as i64).min(100);
        let factor = 100 - resist; // 0 when immune; >100 when vulnerable.
        let result = dmg * factor / 100;
        result.clamp(0, i32::MAX as i64) as i32
    }
}

impl DetHash for ResistanceProfile {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        for &r in &self.resist {
            hasher.write_i32(r);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    // --- DamageType ---

    #[test]
    fn test_all_is_in_discriminant_order() {
        for (i, ty) in DamageType::ALL.iter().enumerate() {
            assert_eq!(ty.index(), i, "ALL[{i}] index mismatch");
        }
    }

    #[test]
    fn test_from_index_roundtrip() {
        for ty in DamageType::ALL {
            assert_eq!(DamageType::from_index(ty.index()), Some(ty));
        }
        assert_eq!(DamageType::from_index(DamageType::COUNT), None);
    }

    #[test]
    fn test_damage_type_det_hash_distinct() {
        let fire = hash_state(&DamageType::Fire);
        let cold = hash_state(&DamageType::Cold);
        assert_ne!(fire, cold);
    }

    // --- ResistanceProfile basics ---

    #[test]
    fn test_new_is_zero_resist_full_damage() {
        let p = ResistanceProfile::new();
        assert_eq!(p.apply(50, DamageType::Fire), 50);
        assert_eq!(p.get(DamageType::Fire), 0);
    }

    #[test]
    fn test_uniform_applies_to_all_types() {
        let p = ResistanceProfile::uniform(25);
        for ty in DamageType::ALL {
            if ty == DamageType::True {
                continue; // True bypasses
            }
            assert_eq!(p.get(ty), 25, "type {ty:?}");
        }
    }

    #[test]
    fn test_with_builder_sets_single_type() {
        let p = ResistanceProfile::new()
            .with(DamageType::Fire, 50)
            .with(DamageType::Cold, -50);
        assert_eq!(p.get(DamageType::Fire), 50);
        assert_eq!(p.get(DamageType::Cold), -50);
        assert_eq!(p.get(DamageType::Poison), 0);
    }

    #[test]
    fn test_set_and_add_in_place() {
        let mut p = ResistanceProfile::new();
        p.set(DamageType::Poison, 30);
        assert_eq!(p.get(DamageType::Poison), 30);
        p.add(DamageType::Poison, 25);
        assert_eq!(p.get(DamageType::Poison), 55);
    }

    #[test]
    fn test_add_saturates() {
        let mut p = ResistanceProfile::new();
        p.set(DamageType::Arcane, i32::MAX);
        p.add(DamageType::Arcane, 100); // would overflow without saturation
        assert_eq!(p.get(DamageType::Arcane), i32::MAX);
    }

    // --- apply semantics ---

    #[test]
    fn test_apply_partial_resistance() {
        let p = ResistanceProfile::new().with(DamageType::Fire, 50);
        // 100 fire * (100-50)/100 = 50
        assert_eq!(p.apply(100, DamageType::Fire), 50);
    }

    #[test]
    fn test_apply_full_immunity_is_zero() {
        let p = ResistanceProfile::new().with(DamageType::Cold, 100);
        assert_eq!(p.apply(999, DamageType::Cold), 0);
        assert!(p.is_immune(DamageType::Cold));
    }

    #[test]
    fn test_apply_over_immunity_clamps_to_zero() {
        let p = ResistanceProfile::new().with(DamageType::Cold, 150);
        // resist clamped to 100 → factor 0, never negative damage
        assert_eq!(p.apply(80, DamageType::Cold), 0);
    }

    #[test]
    fn test_apply_vulnerability_amplifies() {
        let p = ResistanceProfile::new().with(DamageType::Lightning, -50);
        // 40 * (100-(-50))/100 = 40 * 150/100 = 60
        assert_eq!(p.apply(40, DamageType::Lightning), 60);
        assert!(p.is_vulnerable(DamageType::Lightning));
    }

    #[test]
    fn test_apply_true_damage_bypasses_resistance() {
        let p = ResistanceProfile::uniform(100); // immune to everything
        assert_eq!(p.apply(75, DamageType::True), 75, "True ignores resistance");
        assert_eq!(p.apply(75, DamageType::Fire), 0, "Fire still soaked");
    }

    #[test]
    fn test_apply_negative_damage_is_zero() {
        let p = ResistanceProfile::new();
        assert_eq!(p.apply(-10, DamageType::Fire), 0);
        assert_eq!(p.apply(-10, DamageType::True), 0);
    }

    #[test]
    fn test_apply_large_vulnerability_no_overflow() {
        let p = ResistanceProfile::new().with(DamageType::Fire, -1000);
        // i64 intermediate: i32::MAX * 1100/100 would overflow i32 mid-calc;
        // result clamps to i32::MAX.
        assert_eq!(p.apply(i32::MAX, DamageType::Fire), i32::MAX);
    }

    // --- DetHash / determinism ---

    #[test]
    fn test_profile_det_hash_same_inputs_same_hash() {
        let a = ResistanceProfile::new().with(DamageType::Fire, 25);
        let b = ResistanceProfile::new().with(DamageType::Fire, 25);
        assert_eq!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_profile_det_hash_differs_on_change() {
        let a = ResistanceProfile::new().with(DamageType::Fire, 25);
        let b = ResistanceProfile::new().with(DamageType::Fire, 26);
        assert_ne!(hash_state(&a), hash_state(&b));
    }

    #[test]
    fn test_profile_det_hash_type_specific() {
        // Same magnitude on different types must hash differently.
        let fire = ResistanceProfile::new().with(DamageType::Fire, 40);
        let cold = ResistanceProfile::new().with(DamageType::Cold, 40);
        assert_ne!(hash_state(&fire), hash_state(&cold));
    }

    #[test]
    fn test_apply_matches_combat_apply_resistance_for_0_100() {
        // For resist in [0,100], damage typing must agree with the existing
        // flat-percentage primitive so the two systems compose predictably.
        for resist in [0i32, 10, 25, 50, 75, 100] {
            let p = ResistanceProfile::new().with(DamageType::Fire, resist);
            let typed = p.apply(80, DamageType::Fire);
            let flat = crate::combat::apply_resistance(80, resist as u32);
            assert_eq!(typed, flat, "mismatch at resist={resist}");
        }
    }
}
