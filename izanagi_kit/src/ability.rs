//! Unified ability / skill system (G9 in `STRENGTHS_WEAKNESSES.md`).
//!
//! Connects mana cost, cooldown, range, and effect payload into a single
//! query — `try_use` checks all conditions and starts the cooldown on success.
//! The caller manages the mana pool (any integer resource) so the system stays
//! agnostic to energy-source type (mana, stamina, action points, …).
//!
//! ## Design
//!
//! - **`Ability<E>`**: static definition — name, mana cost, cooldown, range,
//!   and a generic effect payload `E` (e.g. a `Spell` enum or `StatsModifier`).
//! - **`AbilitySet<K, E>`**: per-entity collection of abilities keyed by `K`
//!   (e.g. an enum or `u8` slot index). Tracks individual cooldowns per ability.
//! - **`AbilityResult<E>`**: outcome of `try_use` — either success (with the
//!   cloned effect and the mana cost to deduct) or a specific failure reason
//!   (cooldown, mana, range, key not found).
//!
//! ## Determinism contract
//!
//! - No float, no `unsafe`, no OS clock, no `HashMap` ordering.
//! - `tick(n)` advances all cooldown counters with saturating subtraction.
//! - `DetHash` folds every ability definition and current cooldown state so
//!   the full ability set participates in per-frame replay checksums.
//! - The hash is **registration-order-sensitive**: abilities are stored (and
//!   folded) in the order they were added, because the key `K` is generic and
//!   carries no `Ord` bound to canonicalize against (unlike the kit's
//!   `Entity`-keyed collections, which sort in `det_hash`). Two sets holding the
//!   same abilities added in different orders therefore hash differently — so
//!   register abilities in a fixed order on every peer, which deterministic
//!   construction code does for free.
//!
//! ## Example
//!
//! ```
//! use izanagi_kit::ability::{Ability, AbilityResult, AbilitySet};
//!
//! #[derive(Clone, Copy, Debug, PartialEq, Eq)]
//! enum Spell { Fireball, Heal }
//!
//! let mut set: AbilitySet<Spell, &str> = AbilitySet::new()
//!     .with(Spell::Fireball, Ability::new("Fireball", 30, 5, 3, "fire dmg"))
//!     .with(Spell::Heal,     Ability::new("Heal",      10, 2, 0, "restore hp"));
//!
//! // 100 mana, target 2 tiles away.
//! match set.try_use(&Spell::Fireball, 100, 2) {
//!     AbilityResult::Used { effect, mana_cost } => {
//!         // deduct mana, apply effect to target
//!         assert_eq!(effect, "fire dmg");
//!         assert_eq!(mana_cost, 30);
//!     }
//!     _ => panic!("should succeed"),
//! }
//! // Now on cooldown for 5 ticks.
//! assert!(!set.is_ready(&Spell::Fireball));
//! set.tick(5);
//! assert!(set.is_ready(&Spell::Fireball));
//! ```

use crate::world_hash::{DetHash, Fnv1a};

// ── Ability definition ────────────────────────────────────────────────────────

/// Static definition of one ability: its resource cost, cooldown, range, and
/// effect payload.
///
/// The type parameter `E` is the caller-defined effect type — anything from a
/// simple integer damage value to a rich `SpellEffect` enum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ability<E> {
    /// Display name (for UI / message log).
    pub name: String,
    /// Mana / energy cost. Checked against the caller-supplied current mana;
    /// the caller is responsible for deducting it after a `Used` result.
    pub mana_cost: u32,
    /// Ticks the ability is unavailable after use (`0` = no cooldown).
    pub cooldown_ticks: u32,
    /// Maximum grid-distance to target. `0` means self-only or melee (range
    /// check is skipped when `range == 0`).
    pub range: u32,
    /// The mechanical effect: damage formula, buff applied, projectile type, …
    pub effect: E,
}

impl<E: Clone> Ability<E> {
    /// Construct an ability.
    pub fn new(
        name: impl Into<String>,
        mana_cost: u32,
        cooldown_ticks: u32,
        range: u32,
        effect: E,
    ) -> Self {
        Ability {
            name: name.into(),
            mana_cost,
            cooldown_ticks,
            range,
            effect,
        }
    }

    /// Whether this ability imposes a range limit (`range > 0`).
    #[inline]
    pub fn has_range_limit(&self) -> bool {
        self.range > 0
    }
}

impl<E: DetHash> DetHash for Ability<E> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        self.name.as_str().det_hash(hasher); // length-prefixed via DetHash for str
        hasher.write_u32(self.mana_cost);
        hasher.write_u32(self.cooldown_ticks);
        hasher.write_u32(self.range);
        self.effect.det_hash(hasher);
    }
}

// ── AbilityResult ─────────────────────────────────────────────────────────────

/// Result of an [`AbilitySet::try_use`] call.
///
/// `E` is the ability's effect payload type. On `Used`, the effect is **cloned**
/// out of the set so there is no shared borrow conflict with the cooldown write.
#[derive(Debug, PartialEq, Eq)]
pub enum AbilityResult<E> {
    /// Ability fired successfully. Deduct `mana_cost` from the entity's mana
    /// pool and apply `effect` to the target.
    Used {
        /// Cloned effect payload from the ability definition.
        effect: E,
        /// Mana cost to deduct from the caster's pool.
        mana_cost: u32,
    },
    /// Ability is still on cooldown.
    OnCooldown {
        /// Ticks until the ability is available again (always ≥ 1).
        ticks_remaining: u32,
    },
    /// The caster does not have enough mana.
    InsufficientMana {
        /// Mana the caster currently has.
        have: u32,
        /// Mana the ability costs.
        need: u32,
    },
    /// The target is farther than the ability's maximum range.
    OutOfRange {
        /// Distance to target (caller-supplied).
        distance: u32,
        /// Maximum range of the ability.
        max_range: u32,
    },
    /// No ability with the given key exists in the set.
    NotFound,
}

impl<E> AbilityResult<E> {
    /// `true` when the ability fired successfully.
    #[inline]
    pub fn is_used(&self) -> bool {
        matches!(self, AbilityResult::Used { .. })
    }
}

// ── AbilitySet ────────────────────────────────────────────────────────────────

struct AbilityEntry<K, E> {
    key: K,
    ability: Ability<E>,
    /// Current cooldown remaining in ticks (0 = ready).
    cooldown_remaining: u32,
}

/// Per-entity collection of abilities with individual cooldown tracking.
///
/// - `K`: the key type used to look up abilities (e.g. an action enum, `u8` slot).
/// - `E`: the effect payload type stored in each [`Ability`].
///
/// Build with [`new`](AbilitySet::new) and [`with`](AbilitySet::with).
pub struct AbilitySet<K, E> {
    entries: Vec<AbilityEntry<K, E>>,
}

impl<K: PartialEq, E: Clone> AbilitySet<K, E> {
    /// Create an empty ability set.
    pub fn new() -> Self {
        AbilitySet {
            entries: Vec::new(),
        }
    }

    /// Register an ability (builder style). If `key` already exists the
    /// existing entry is replaced and its cooldown is reset to 0.
    pub fn with(mut self, key: K, ability: Ability<E>) -> Self {
        if let Some(e) = self.entries.iter_mut().find(|e| e.key == key) {
            e.ability = ability;
            e.cooldown_remaining = 0;
        } else {
            self.entries.push(AbilityEntry {
                key,
                ability,
                cooldown_remaining: 0,
            });
        }
        self
    }

    /// Advance all cooldown counters by `n` ticks (saturating at zero).
    pub fn tick(&mut self, n: u32) {
        for e in &mut self.entries {
            e.cooldown_remaining = e.cooldown_remaining.saturating_sub(n);
        }
    }

    /// Try to use the ability identified by `key`.
    ///
    /// Checks are applied in order: **key found → cooldown → mana → range**.
    ///
    /// - `current_mana`: the caster's current mana. On `Used`, the caller
    ///   deducts `mana_cost` from their own pool; this set does not manage mana.
    /// - `distance`: grid distance to target. Ignored when `ability.range == 0`.
    ///
    /// On success: the cooldown timer starts and the effect is **cloned** into
    /// the returned `Used` variant. On failure: nothing changes.
    pub fn try_use(&mut self, key: &K, current_mana: u32, distance: u32) -> AbilityResult<E> {
        let idx = match self.entries.iter().position(|e| &e.key == key) {
            None => return AbilityResult::NotFound,
            Some(i) => i,
        };

        // All reads happen before any mutation.
        let cd = self.entries[idx].cooldown_remaining;
        if cd > 0 {
            return AbilityResult::OnCooldown {
                ticks_remaining: cd,
            };
        }
        let mana_cost = self.entries[idx].ability.mana_cost;
        if current_mana < mana_cost {
            return AbilityResult::InsufficientMana {
                have: current_mana,
                need: mana_cost,
            };
        }
        let range = self.entries[idx].ability.range;
        if range > 0 && distance > range {
            return AbilityResult::OutOfRange {
                distance,
                max_range: range,
            };
        }

        // Clone effect before mutating cooldown_remaining (avoids borrow conflict).
        let effect = self.entries[idx].ability.effect.clone();
        let cooldown_ticks = self.entries[idx].ability.cooldown_ticks;
        self.entries[idx].cooldown_remaining = cooldown_ticks;

        AbilityResult::Used { effect, mana_cost }
    }

    // ── Query helpers ──────────────────────────────────────────────────────

    /// `true` when the ability with `key` has no cooldown remaining.
    /// Returns `false` for unknown keys.
    pub fn is_ready(&self, key: &K) -> bool {
        self.entries
            .iter()
            .find(|e| &e.key == key)
            .is_some_and(|e| e.cooldown_remaining == 0)
    }

    /// Ticks remaining on the cooldown for `key`, or `0` if unknown or ready.
    pub fn cooldown_remaining(&self, key: &K) -> u32 {
        self.entries
            .iter()
            .find(|e| &e.key == key)
            .map_or(0, |e| e.cooldown_remaining)
    }

    /// Immutable reference to the ability definition for `key`, if present.
    pub fn get(&self, key: &K) -> Option<&Ability<E>> {
        self.entries
            .iter()
            .find(|e| &e.key == key)
            .map(|e| &e.ability)
    }

    /// Number of abilities in the set.
    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` when no abilities are registered.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate `(key, ability)` pairs in registration order.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &Ability<E>)> {
        self.entries.iter().map(|e| (&e.key, &e.ability))
    }
}

impl<K: PartialEq, E: Clone> Default for AbilitySet<K, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: DetHash, E: Clone + DetHash> DetHash for AbilitySet<K, E> {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u32(self.entries.len() as u32);
        for e in &self.entries {
            e.key.det_hash(hasher);
            e.ability.det_hash(hasher);
            hasher.write_u32(e.cooldown_remaining);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Spell {
        Fireball,
        Heal,
        Teleport,
    }

    impl DetHash for Spell {
        fn det_hash(&self, h: &mut Fnv1a) {
            h.write_u8(*self as u8);
        }
    }

    fn fireball() -> Ability<i32> {
        Ability::new("Fireball", 30, 5, 3, 20)
    }
    fn heal() -> Ability<i32> {
        Ability::new("Heal", 10, 2, 0, -15)
    }

    fn set() -> AbilitySet<Spell, i32> {
        AbilitySet::new()
            .with(Spell::Fireball, fireball())
            .with(Spell::Heal, heal())
    }

    // ── try_use: success ───────────────────────────────────────────────────

    #[test]
    fn test_try_use_success_returns_used_with_effect_and_cost() {
        let mut s = set();
        match s.try_use(&Spell::Fireball, 100, 2) {
            AbilityResult::Used { effect, mana_cost } => {
                assert_eq!(effect, 20);
                assert_eq!(mana_cost, 30);
            }
            r => panic!("expected Used, got {:?}", r),
        }
    }

    #[test]
    fn test_try_use_starts_cooldown_on_success() {
        let mut s = set();
        s.try_use(&Spell::Fireball, 100, 1);
        assert_eq!(s.cooldown_remaining(&Spell::Fireball), 5);
        assert!(!s.is_ready(&Spell::Fireball));
    }

    #[test]
    fn test_try_use_zero_cost_zero_cooldown_always_succeeds() {
        let mut s = AbilitySet::new().with(Spell::Teleport, Ability::new("Blink", 0, 0, 0, 1i32));
        for _ in 0..5 {
            assert!(s.try_use(&Spell::Teleport, 0, 0).is_used());
        }
    }

    // ── try_use: cooldown ─────────────────────────────────────────────────

    #[test]
    fn test_try_use_on_cooldown_returns_ticks_remaining() {
        let mut s = set();
        s.try_use(&Spell::Fireball, 100, 1);
        match s.try_use(&Spell::Fireball, 100, 1) {
            AbilityResult::OnCooldown { ticks_remaining } => {
                assert_eq!(ticks_remaining, 5);
            }
            r => panic!("expected OnCooldown, got {:?}", r),
        }
    }

    #[test]
    fn test_tick_decrements_cooldown() {
        let mut s = set();
        s.try_use(&Spell::Fireball, 100, 1);
        s.tick(3);
        assert_eq!(s.cooldown_remaining(&Spell::Fireball), 2);
    }

    #[test]
    fn test_tick_past_cooldown_makes_ability_ready() {
        let mut s = set();
        s.try_use(&Spell::Fireball, 100, 1);
        s.tick(5);
        assert!(s.is_ready(&Spell::Fireball));
    }

    #[test]
    fn test_tick_saturates_at_zero() {
        let mut s = set();
        s.try_use(&Spell::Heal, 100, 0);
        s.tick(100);
        assert_eq!(s.cooldown_remaining(&Spell::Heal), 0);
    }

    // ── try_use: insufficient mana ────────────────────────────────────────

    #[test]
    fn test_try_use_insufficient_mana_returns_error() {
        let mut s = set();
        match s.try_use(&Spell::Fireball, 10, 1) {
            AbilityResult::InsufficientMana { have, need } => {
                assert_eq!(have, 10);
                assert_eq!(need, 30);
            }
            r => panic!("expected InsufficientMana, got {:?}", r),
        }
    }

    #[test]
    fn test_try_use_insufficient_mana_does_not_start_cooldown() {
        let mut s = set();
        s.try_use(&Spell::Fireball, 5, 1);
        assert_eq!(s.cooldown_remaining(&Spell::Fireball), 0);
    }

    #[test]
    fn test_try_use_exact_mana_succeeds() {
        let mut s = set();
        assert!(s.try_use(&Spell::Fireball, 30, 2).is_used());
    }

    // ── try_use: out of range ─────────────────────────────────────────────

    #[test]
    fn test_try_use_out_of_range_returns_error() {
        let mut s = set();
        match s.try_use(&Spell::Fireball, 100, 4) {
            AbilityResult::OutOfRange {
                distance,
                max_range,
            } => {
                assert_eq!(distance, 4);
                assert_eq!(max_range, 3);
            }
            r => panic!("expected OutOfRange, got {:?}", r),
        }
    }

    #[test]
    fn test_try_use_zero_range_ignores_distance() {
        let mut s = set();
        // Heal has range 0 — distance is irrelevant.
        assert!(s.try_use(&Spell::Heal, 100, 999).is_used());
    }

    #[test]
    fn test_try_use_out_of_range_no_cooldown() {
        let mut s = set();
        s.try_use(&Spell::Fireball, 100, 10);
        assert_eq!(s.cooldown_remaining(&Spell::Fireball), 0);
    }

    // ── try_use: not found ────────────────────────────────────────────────

    #[test]
    fn test_try_use_unknown_key_returns_not_found() {
        let mut s = set();
        assert!(matches!(
            s.try_use(&Spell::Teleport, 100, 0),
            AbilityResult::NotFound
        ));
    }

    // ── query helpers ─────────────────────────────────────────────────────

    #[test]
    fn test_is_ready_true_for_fresh_ability() {
        let s = set();
        assert!(s.is_ready(&Spell::Fireball));
    }

    #[test]
    fn test_is_ready_false_for_unknown_key() {
        let s = set();
        assert!(!s.is_ready(&Spell::Teleport));
    }

    #[test]
    fn test_get_returns_ability_definition() {
        let s = set();
        let ab = s.get(&Spell::Fireball).unwrap();
        assert_eq!(ab.mana_cost, 30);
        assert_eq!(ab.cooldown_ticks, 5);
    }

    #[test]
    fn test_len_and_is_empty() {
        let s: AbilitySet<Spell, i32> = AbilitySet::new();
        assert!(s.is_empty());
        let s = s.with(Spell::Fireball, fireball());
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_with_replaces_existing_key() {
        let s = AbilitySet::new()
            .with(Spell::Fireball, fireball())
            .with(Spell::Fireball, Ability::new("Fireball+", 50, 10, 5, 40i32));
        assert_eq!(s.len(), 1);
        assert_eq!(s.get(&Spell::Fireball).unwrap().mana_cost, 50);
    }

    #[test]
    fn test_iter_returns_all_abilities() {
        let s = set();
        let keys: Vec<&Spell> = s.iter().map(|(k, _)| k).collect();
        assert!(keys.contains(&&Spell::Fireball));
        assert!(keys.contains(&&Spell::Heal));
    }

    // ── DetHash ───────────────────────────────────────────────────────────

    #[test]
    fn test_det_hash_same_set_same_hash() {
        assert_eq!(hash_state(&set()), hash_state(&set()));
    }

    #[test]
    fn test_det_hash_differs_after_use() {
        let s1 = set();
        let mut s2 = set();
        s2.try_use(&Spell::Fireball, 100, 1);
        assert_ne!(hash_state(&s1), hash_state(&s2));
    }

    #[test]
    fn test_det_hash_differs_by_ability_set() {
        let s1: AbilitySet<Spell, i32> = AbilitySet::new().with(Spell::Fireball, fireball());
        let s2: AbilitySet<Spell, i32> = AbilitySet::new().with(Spell::Heal, heal());
        assert_ne!(hash_state(&s1), hash_state(&s2));
    }

    // ── Roguelike integration scenario ────────────────────────────────────

    #[test]
    fn test_mage_combat_sequence() {
        // Mage: 50 mana, casts Fireball → runs low → waits for cooldown →
        // not enough mana → regens → casts again.
        let mut mana = 50u32;
        let mut s =
            AbilitySet::new().with(Spell::Fireball, Ability::new("Fireball", 30, 3, 5, 25i32));

        // First cast succeeds.
        match s.try_use(&Spell::Fireball, mana, 4) {
            AbilityResult::Used { mana_cost, .. } => mana -= mana_cost,
            _ => panic!("first cast must succeed"),
        }
        assert_eq!(mana, 20);

        // On cooldown after 2 of 3 ticks.
        s.tick(2);
        assert!(matches!(
            s.try_use(&Spell::Fireball, mana, 4),
            AbilityResult::OnCooldown { .. }
        ));

        // Cooldown expires.
        s.tick(1);
        assert!(s.is_ready(&Spell::Fireball));

        // Not enough mana.
        assert!(matches!(
            s.try_use(&Spell::Fireball, mana, 4),
            AbilityResult::InsufficientMana { have: 20, need: 30 }
        ));

        // Regen and cast.
        mana += 10;
        match s.try_use(&Spell::Fireball, mana, 4) {
            AbilityResult::Used { mana_cost, .. } => mana -= mana_cost,
            _ => panic!("cast after regen must succeed"),
        }
        assert_eq!(mana, 0);
    }

    #[test]
    fn test_has_range_limit_respects_zero() {
        assert!(fireball().has_range_limit()); // range 3
        assert!(!heal().has_range_limit()); // range 0
    }
}
