//! Regenerating bounded resource pools — mana, stamina, hunger, charge.
//!
//! [`combat::Stats`](crate::combat::Stats) models HP as a current/max pair, and
//! [`ability`](crate::ability) checks an integer cost but explicitly leaves the
//! *resource pool itself* to the caller. Nothing in the kit captured the common
//! pattern shared by **mana, stamina, hunger, shield charge, and item
//! cooldown-energy**: a value bounded in `[0, max]` that is *spent* in lumps,
//! *regenerates* a fixed amount per tick, and answers "can I afford this?".
//! [`Pool`] is that primitive.
//!
//! ```
//! use izanagi_kit::pool::Pool;
//!
//! // 100 mana, regenerating 5 per tick, starting full.
//! let mut mana = Pool::with_regen(100, 5);
//! assert!(mana.is_full());
//!
//! // Cast a 30-mana spell: spend succeeds only if affordable.
//! assert!(mana.spend(30));
//! assert_eq!(mana.current(), 70);
//! assert!(!mana.spend(1000));        // unaffordable → no change
//! assert_eq!(mana.current(), 70);
//!
//! // Three turns pass: +5 per tick, clamped to max.
//! let gained = mana.tick(3);
//! assert_eq!(gained, 15);
//! assert_eq!(mana.current(), 85);
//! ```
//!
//! ## Design
//!
//! Everything is `i32` with saturating/clamping arithmetic — no float, no
//! overflow panic. `max` is held `>= 0` and `current` is always clamped to
//! `[0, max]`. The per-tick regeneration rate is **signed**: a positive rate
//! regenerates (stamina recovering), a negative rate decays (hunger draining,
//! poison ticking), and zero is inert. [`Pool`] implements
//! [`DetHash`], folding `current`, `max`, and the
//! regen rate into the replay checksum.

use crate::world_hash::{DetHash, Fnv1a};

/// A bounded, regenerating integer resource in `[0, max]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pool {
    current: i32,
    max: i32,
    regen_per_tick: i32,
}

impl Pool {
    /// Create a full pool with `max` capacity and no regeneration.
    /// `max` is clamped to `>= 0`.
    pub fn new(max: i32) -> Self {
        let max = max.max(0);
        Pool {
            current: max,
            max,
            regen_per_tick: 0,
        }
    }

    /// Create a full pool with `max` capacity and a per-tick regen rate
    /// (may be negative to model decay).
    pub fn with_regen(max: i32, regen_per_tick: i32) -> Self {
        Pool {
            regen_per_tick,
            ..Pool::new(max)
        }
    }

    /// Create a pool with an explicit starting `current` (clamped to
    /// `[0, max]`) and regen rate. Useful when resuming from a save.
    pub fn with_current(max: i32, current: i32, regen_per_tick: i32) -> Self {
        let max = max.max(0);
        Pool {
            current: current.clamp(0, max),
            max,
            regen_per_tick,
        }
    }

    /// The current amount in the pool.
    #[inline]
    pub fn current(&self) -> i32 {
        self.current
    }

    /// The maximum capacity.
    #[inline]
    pub fn max(&self) -> i32 {
        self.max
    }

    /// The per-tick regeneration rate (signed).
    #[inline]
    pub fn regen_per_tick(&self) -> i32 {
        self.regen_per_tick
    }

    /// `true` if the pool is at full capacity.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.current >= self.max
    }

    /// `true` if the pool is empty (`current == 0`).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.current <= 0
    }

    /// `true` if at least `amount` is available to spend.
    /// A non-positive `amount` is always affordable.
    #[inline]
    pub fn can_afford(&self, amount: i32) -> bool {
        self.current >= amount
    }

    /// Spend exactly `amount` **only if affordable**, all-or-nothing.
    /// Returns `true` and deducts on success; returns `false` and leaves the
    /// pool unchanged if `current < amount`. A non-positive `amount` is a
    /// no-op that returns `true`.
    pub fn spend(&mut self, amount: i32) -> bool {
        if amount <= 0 {
            return true;
        }
        if self.current >= amount {
            self.current -= amount;
            true
        } else {
            false
        }
    }

    /// Remove up to `amount` (floored at `0`), regardless of whether the full
    /// amount is available. Returns how much was actually removed. Negative
    /// `amount` is treated as `0`.
    pub fn drain(&mut self, amount: i32) -> i32 {
        let take = amount.max(0).min(self.current);
        self.current -= take;
        take
    }

    /// Add up to `amount` (capped at `max`). Returns how much was actually
    /// added. Negative `amount` is treated as `0`.
    pub fn restore(&mut self, amount: i32) -> i32 {
        let before = self.current;
        self.current = self.current.saturating_add(amount.max(0)).min(self.max);
        self.current - before
    }

    /// Set the current amount directly (clamped to `[0, max]`).
    pub fn set(&mut self, value: i32) {
        self.current = value.clamp(0, self.max);
    }

    /// Fill the pool to maximum.
    #[inline]
    pub fn fill(&mut self) {
        self.current = self.max;
    }

    /// Empty the pool to zero.
    #[inline]
    pub fn empty(&mut self) {
        self.current = 0;
    }

    /// Change the per-tick regeneration rate.
    #[inline]
    pub fn set_regen(&mut self, regen_per_tick: i32) {
        self.regen_per_tick = regen_per_tick;
    }

    /// Set a new maximum (clamped to `>= 0`) and clamp `current` to it.
    /// Models a permanent capacity change (level-up, curse).
    pub fn set_max(&mut self, new_max: i32) {
        self.max = new_max.max(0);
        self.current = self.current.min(self.max);
    }

    /// Advance `ticks` turns of regeneration: applies `regen_per_tick * ticks`,
    /// clamped to `[0, max]`. A positive rate regenerates, a negative rate
    /// decays. Returns the **signed** net change to `current` (positive =
    /// gained, negative = lost).
    pub fn tick(&mut self, ticks: u32) -> i32 {
        if self.regen_per_tick == 0 || ticks == 0 {
            return 0;
        }
        let before = self.current;
        let delta = (self.regen_per_tick as i64) * (ticks as i64);
        let next = (self.current as i64 + delta).clamp(0, self.max as i64);
        self.current = next as i32;
        self.current - before
    }

    /// The amount below maximum: `max(0, max - current)`. The amount of
    /// restoration needed to reach full.
    #[inline]
    pub fn deficit(&self) -> i32 {
        (self.max - self.current).max(0)
    }

    /// Fill level as an integer percentage in `[0, 100]`. Returns `0` when
    /// `max` is `0`. No float division — suitable for `BarWidget` fill values.
    pub fn percent(&self) -> u32 {
        if self.max <= 0 {
            return 0;
        }
        (self.current.max(0) as u64 * 100 / self.max as u64).min(100) as u32
    }

    /// Fill level in per mille `[0, 1000]` for finer-grained bars/gradients.
    pub fn per_mille(&self) -> u32 {
        if self.max <= 0 {
            return 0;
        }
        (self.current.max(0) as u64 * 1000 / self.max as u64).min(1000) as u32
    }
}

impl DetHash for Pool {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_i32(self.current);
        hasher.write_i32(self.max);
        hasher.write_i32(self.regen_per_tick);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[test]
    fn test_new_is_full_no_regen() {
        let p = Pool::new(100);
        assert_eq!(p.current(), 100);
        assert_eq!(p.max(), 100);
        assert_eq!(p.regen_per_tick(), 0);
        assert!(p.is_full());
        assert!(!p.is_empty());
    }

    #[test]
    fn test_new_clamps_negative_max() {
        let p = Pool::new(-50);
        assert_eq!(p.max(), 0);
        assert_eq!(p.current(), 0);
        assert!(p.is_empty());
        assert!(p.is_full(), "0/0 is both empty and full");
    }

    #[test]
    fn test_with_current_clamps() {
        let p = Pool::with_current(100, 250, 5);
        assert_eq!(p.current(), 100, "current clamped to max");
        let q = Pool::with_current(100, -10, 5);
        assert_eq!(q.current(), 0, "current clamped to 0");
    }

    #[test]
    fn test_spend_all_or_nothing() {
        let mut p = Pool::new(100);
        assert!(p.spend(30));
        assert_eq!(p.current(), 70);
        assert!(!p.spend(1000), "unaffordable spend fails");
        assert_eq!(p.current(), 70, "failed spend leaves pool unchanged");
    }

    #[test]
    fn test_spend_exact_and_to_zero() {
        let mut p = Pool::new(50);
        assert!(p.spend(50));
        assert_eq!(p.current(), 0);
        assert!(p.is_empty());
        assert!(!p.spend(1));
    }

    #[test]
    fn test_spend_nonpositive_is_noop_true() {
        let mut p = Pool::new(10);
        assert!(p.spend(0));
        assert!(p.spend(-5));
        assert_eq!(p.current(), 10);
    }

    #[test]
    fn test_can_afford() {
        let p = Pool::with_current(100, 40, 0);
        assert!(p.can_afford(40));
        assert!(p.can_afford(0));
        assert!(!p.can_afford(41));
    }

    #[test]
    fn test_drain_returns_actual() {
        let mut p = Pool::with_current(100, 30, 0);
        assert_eq!(p.drain(50), 30, "drain capped at current");
        assert_eq!(p.current(), 0);
        assert_eq!(p.drain(10), 0, "drain on empty removes nothing");
    }

    #[test]
    fn test_restore_returns_actual_and_caps() {
        let mut p = Pool::with_current(100, 80, 0);
        assert_eq!(p.restore(50), 20, "restore capped at max");
        assert!(p.is_full());
        assert_eq!(p.restore(10), 0, "restore on full adds nothing");
    }

    #[test]
    fn test_set_clamps() {
        let mut p = Pool::new(100);
        p.set(150);
        assert_eq!(p.current(), 100);
        p.set(-10);
        assert_eq!(p.current(), 0);
        p.set(42);
        assert_eq!(p.current(), 42);
    }

    #[test]
    fn test_fill_and_empty() {
        let mut p = Pool::with_current(100, 50, 0);
        p.empty();
        assert!(p.is_empty());
        p.fill();
        assert!(p.is_full());
    }

    #[test]
    fn test_tick_regenerates_and_clamps() {
        let mut p = Pool::with_current(100, 70, 5);
        assert_eq!(p.tick(3), 15);
        assert_eq!(p.current(), 85);
        // Over-regen clamps to max and reports only the real gain.
        assert_eq!(p.tick(100), 15, "gain is clamped to the deficit");
        assert!(p.is_full());
    }

    #[test]
    fn test_tick_negative_regen_decays() {
        let mut p = Pool::with_current(100, 30, -10);
        assert_eq!(p.tick(2), -20, "negative regen decays");
        assert_eq!(p.current(), 10);
        // Decay floors at zero.
        assert_eq!(p.tick(5), -10);
        assert_eq!(p.current(), 0);
    }

    #[test]
    fn test_tick_zero_rate_or_zero_ticks_is_noop() {
        let mut p = Pool::with_current(100, 50, 0);
        assert_eq!(p.tick(10), 0);
        let mut q = Pool::with_current(100, 50, 7);
        assert_eq!(q.tick(0), 0);
        assert_eq!(q.current(), 50);
    }

    #[test]
    fn test_set_max_clamps_current() {
        let mut p = Pool::new(100);
        p.set_max(40);
        assert_eq!(p.max(), 40);
        assert_eq!(p.current(), 40, "current clamped down to new max");
        p.set_max(-5);
        assert_eq!(p.max(), 0);
        assert_eq!(p.current(), 0);
    }

    #[test]
    fn test_deficit() {
        let mut p = Pool::with_current(100, 30, 0);
        assert_eq!(p.deficit(), 70);
        p.fill();
        assert_eq!(p.deficit(), 0);
    }

    #[test]
    fn test_percent_and_per_mille() {
        let p = Pool::with_current(200, 50, 0);
        assert_eq!(p.percent(), 25);
        assert_eq!(p.per_mille(), 250);
        let z = Pool::new(0);
        assert_eq!(z.percent(), 0, "zero max → 0%");
        assert_eq!(z.per_mille(), 0);
    }

    #[test]
    fn test_det_hash_canonical_and_sensitive() {
        let a = Pool::with_current(100, 60, 5);
        let b = Pool::with_current(100, 60, 5);
        assert_eq!(hash_state(&a), hash_state(&b), "same state, same hash");
        let c = Pool::with_current(100, 61, 5);
        assert_ne!(
            hash_state(&a),
            hash_state(&c),
            "different current → different hash"
        );
        let d = Pool::with_current(100, 60, 6);
        assert_ne!(
            hash_state(&a),
            hash_state(&d),
            "different regen → different hash"
        );
    }
}
