//! Character progression — experience accumulation and integer level curves.
//!
//! The kit could describe a combatant's *instantaneous* state
//! ([`combat::Stats`](crate::combat::Stats)) and *temporary* modifiers
//! ([`combat::StatsModifier`](crate::combat::StatsModifier),
//! [`status`](crate::status)), but had no notion of **permanent growth over
//! time** — the experience-and-levels axis every RPG-flavoured roguelike needs.
//! [`Progression`] is that layer: a monotone, fully-integer mapping from
//! accumulated experience to character level, driven by a deterministic
//! [`LevelCurve`].
//!
//! The curve uses an *arithmetic* per-level cost: advancing from level `L` to
//! `L+1` costs `base + step·(L-1)` experience, so the cumulative experience to
//! *reach* level `L` is a closed-form integer sum (no float, no table lookup):
//!
//! ```text
//! xp_to_reach(1) = 0
//! xp_to_reach(L) = (L-1)·base + step·(L-1)·(L-2)/2     for L ≥ 1
//! ```
//!
//! ```
//! use izanagi_kit::progression::{LevelCurve, Progression};
//!
//! // Level 2 costs 100 xp; each subsequent level costs 50 more than the last.
//! let curve = LevelCurve::new(100, 50, 99);
//! let mut hero = Progression::new(curve);
//! assert_eq!(hero.level(), 1);
//!
//! let gained = hero.add_xp(100);   // reach the level-2 threshold exactly
//! assert_eq!(gained, 1);
//! assert_eq!(hero.level(), 2);
//! assert_eq!(hero.xp_into_level(), 0);
//! assert_eq!(hero.xp_to_next(), 150); // level 3 costs 100 + 50
//! ```
//!
//! Determinism: every quantity is a closed-form function of `u64` inputs
//! computed in widened `u128` to avoid overflow, with no float and no
//! allocation. [`Progression`] and [`LevelCurve`] implement
//! [`DetHash`](crate::world_hash::DetHash), folding a character's growth into
//! the replay checksum.

use crate::world_hash::{DetHash, Fnv1a};

/// A deterministic, integer experience curve with an arithmetic per-level cost.
///
/// Reaching level `L+1` from `L` costs `base + step·(L-1)` experience. A `step`
/// of zero gives a flat (linear) curve; a positive `step` makes each level
/// progressively more expensive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LevelCurve {
    base: u64,
    step: u64,
    max_level: u32,
}

impl LevelCurve {
    /// Create a curve. `base` is the cost of the first level-up (1→2), `step` is
    /// how much more each subsequent level-up costs, and `max_level` is the cap
    /// (clamped to at least 1).
    pub fn new(base: u64, step: u64, max_level: u32) -> Self {
        LevelCurve {
            base,
            step,
            max_level: max_level.max(1),
        }
    }

    /// The first level-up cost (1→2).
    #[inline]
    pub fn base(&self) -> u64 {
        self.base
    }

    /// The additional cost added to each successive level-up.
    #[inline]
    pub fn step(&self) -> u64 {
        self.step
    }

    /// The maximum attainable level (≥ 1).
    #[inline]
    pub fn max_level(&self) -> u32 {
        self.max_level
    }

    /// Cumulative experience required to *reach* `level` from the start.
    /// `xp_to_reach(1) == 0`; levels above `max_level` saturate at the cap's
    /// threshold; `level == 0` is treated as level 1 (returns 0).
    ///
    /// Closed form: `(L-1)·base + step·(L-1)·(L-2)/2`, computed in `u128` and
    /// saturated back to `u64`.
    pub fn xp_to_reach(&self, level: u32) -> u64 {
        let level = level.clamp(1, self.max_level);
        let n = (level - 1) as u128; // number of level-ups taken
        if n == 0 {
            return 0;
        }
        // sum_{k=0}^{n-1} (base + step·k) = n·base + step·n·(n-1)/2
        let linear = n * self.base as u128;
        let triangular = self.step as u128 * (n * (n - 1) / 2);
        (linear + triangular).min(u64::MAX as u128) as u64
    }

    /// The cost of the single level-up from `level` to `level+1`, i.e.
    /// `base + step·(level-1)`. Returns `0` at or beyond `max_level` (no further
    /// level-up is possible). `level == 0` is treated as level 1.
    pub fn cost_of_level_up(&self, level: u32) -> u64 {
        let level = level.max(1);
        if level >= self.max_level {
            return 0;
        }
        let k = (level - 1) as u128;
        (self.base as u128 + self.step as u128 * k).min(u64::MAX as u128) as u64
    }

    /// The largest level whose [`xp_to_reach`](Self::xp_to_reach) is `≤ total_xp`,
    /// clamped to `max_level`. The inverse of `xp_to_reach` over the reachable
    /// range: `level_at(xp_to_reach(L)) == L` for `1 ≤ L ≤ max_level`.
    pub fn level_at(&self, total_xp: u64) -> u32 {
        // Monotone in `total_xp`, so binary-search the threshold.
        let mut lo = 1u32;
        let mut hi = self.max_level;
        while lo < hi {
            let mid = lo + (hi - lo).div_ceil(2);
            if self.xp_to_reach(mid) <= total_xp {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        lo
    }
}

/// A character's accumulated experience and derived level, governed by a
/// [`LevelCurve`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progression {
    curve: LevelCurve,
    total_xp: u64,
    level: u32,
}

impl Progression {
    /// Start a fresh progression at level 1 with zero experience.
    pub fn new(curve: LevelCurve) -> Self {
        Progression {
            curve,
            total_xp: 0,
            level: 1,
        }
    }

    /// Start at a specific accumulated experience total (level is derived).
    pub fn with_xp(curve: LevelCurve, total_xp: u64) -> Self {
        let level = curve.level_at(total_xp);
        Progression {
            curve,
            total_xp,
            level,
        }
    }

    /// The governing curve.
    #[inline]
    pub fn curve(&self) -> &LevelCurve {
        &self.curve
    }

    /// Total experience accumulated over the character's lifetime.
    #[inline]
    pub fn total_xp(&self) -> u64 {
        self.total_xp
    }

    /// The current level (always in `1..=curve.max_level()`).
    #[inline]
    pub fn level(&self) -> u32 {
        self.level
    }

    /// `true` once the character has reached the curve's maximum level.
    #[inline]
    pub fn is_max_level(&self) -> bool {
        self.level >= self.curve.max_level()
    }

    /// Experience earned *within* the current level — `total_xp` minus the
    /// threshold of the current level. Always `< cost_of_current_level()` unless
    /// at max level.
    pub fn xp_into_level(&self) -> u64 {
        self.total_xp - self.curve.xp_to_reach(self.level)
    }

    /// Experience still required to reach the next level. Returns `0` at max
    /// level (no next level exists).
    pub fn xp_to_next(&self) -> u64 {
        if self.is_max_level() {
            return 0;
        }
        self.curve.xp_to_reach(self.level + 1) - self.total_xp
    }

    /// Add `amount` experience (saturating), recompute the level, and return the
    /// number of levels gained (`0` if none, possibly more than one for a large
    /// award). Experience is never lost; the level is a pure function of the new
    /// total.
    pub fn add_xp(&mut self, amount: u64) -> u32 {
        let before = self.level;
        self.total_xp = self.total_xp.saturating_add(amount);
        self.level = self.curve.level_at(self.total_xp);
        self.level - before
    }
}

impl DetHash for LevelCurve {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u64(self.base);
        hasher.write_u64(self.step);
        hasher.write_u32(self.max_level);
    }
}

impl DetHash for Progression {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        self.curve.det_hash(hasher);
        hasher.write_u64(self.total_xp);
        hasher.write_u32(self.level);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[test]
    fn test_new_starts_at_level_one() {
        let p = Progression::new(LevelCurve::new(100, 50, 99));
        assert_eq!(p.level(), 1);
        assert_eq!(p.total_xp(), 0);
        assert_eq!(p.xp_into_level(), 0);
        assert_eq!(p.xp_to_next(), 100);
    }

    #[test]
    fn test_xp_to_reach_closed_form() {
        let c = LevelCurve::new(100, 50, 99);
        assert_eq!(c.xp_to_reach(1), 0);
        assert_eq!(c.xp_to_reach(2), 100); // 100
        assert_eq!(c.xp_to_reach(3), 250); // 100 + 150
        assert_eq!(c.xp_to_reach(4), 450); // 100 + 150 + 200
    }

    #[test]
    fn test_cost_of_level_up() {
        let c = LevelCurve::new(100, 50, 5);
        assert_eq!(c.cost_of_level_up(1), 100);
        assert_eq!(c.cost_of_level_up(2), 150);
        assert_eq!(c.cost_of_level_up(3), 200);
        assert_eq!(c.cost_of_level_up(5), 0, "no level-up past the cap");
        assert_eq!(c.cost_of_level_up(99), 0);
    }

    #[test]
    fn test_level_at_inverts_threshold() {
        let c = LevelCurve::new(100, 50, 50);
        for l in 1..=50u32 {
            assert_eq!(c.level_at(c.xp_to_reach(l)), l, "round-trip at level {l}");
        }
    }

    #[test]
    fn test_level_at_boundary() {
        let c = LevelCurve::new(100, 50, 50);
        for l in 2..=50u32 {
            let t = c.xp_to_reach(l);
            assert_eq!(c.level_at(t - 1), l - 1, "just below threshold {l}");
            assert_eq!(c.level_at(t), l, "at threshold {l}");
        }
    }

    #[test]
    fn test_add_xp_single_level() {
        let mut p = Progression::new(LevelCurve::new(100, 50, 99));
        assert_eq!(p.add_xp(100), 1);
        assert_eq!(p.level(), 2);
        assert_eq!(p.xp_into_level(), 0);
        assert_eq!(p.xp_to_next(), 150);
    }

    #[test]
    fn test_add_xp_multiple_levels_at_once() {
        let mut p = Progression::new(LevelCurve::new(100, 50, 99));
        // 0 -> 450 spans levels 2 (100), 3 (250), 4 (450): +3 levels.
        let gained = p.add_xp(450);
        assert_eq!(gained, 3);
        assert_eq!(p.level(), 4);
        assert_eq!(p.xp_into_level(), 0);
    }

    #[test]
    fn test_add_xp_partial_no_levelup() {
        let mut p = Progression::new(LevelCurve::new(100, 50, 99));
        assert_eq!(p.add_xp(60), 0);
        assert_eq!(p.level(), 1);
        assert_eq!(p.xp_into_level(), 60);
        assert_eq!(p.xp_to_next(), 40);
    }

    #[test]
    fn test_level_cap_clamps() {
        let mut p = Progression::new(LevelCurve::new(100, 0, 3));
        assert_eq!(p.add_xp(1_000_000), 2, "capped at level 3 (gained 2)");
        assert_eq!(p.level(), 3);
        assert!(p.is_max_level());
        assert_eq!(p.xp_to_next(), 0, "no next level at cap");
    }

    #[test]
    fn test_flat_curve() {
        let c = LevelCurve::new(50, 0, 10);
        assert_eq!(c.xp_to_reach(2), 50);
        assert_eq!(c.xp_to_reach(3), 100);
        assert_eq!(c.level_at(125), 3);
    }

    #[test]
    fn test_with_xp_derives_level() {
        let p = Progression::with_xp(LevelCurve::new(100, 50, 99), 300);
        // 300 is between threshold(3)=250 and threshold(4)=450.
        assert_eq!(p.level(), 3);
        assert_eq!(p.xp_into_level(), 50);
    }

    #[test]
    fn test_max_level_one_curve() {
        let mut p = Progression::new(LevelCurve::new(100, 50, 1));
        assert!(p.is_max_level());
        assert_eq!(p.add_xp(999), 0, "no levels above the cap of 1");
        assert_eq!(p.level(), 1);
        assert_eq!(p.xp_to_next(), 0);
    }

    #[test]
    fn test_add_xp_saturates() {
        let mut p = Progression::with_xp(LevelCurve::new(1, 0, u32::MAX), u64::MAX - 5);
        p.add_xp(1000); // would overflow u64
        assert_eq!(p.total_xp(), u64::MAX, "xp must saturate, not wrap");
    }

    #[test]
    fn test_det_hash_canonical_and_sensitive() {
        let curve = LevelCurve::new(100, 50, 99);
        let a = Progression::with_xp(curve, 300);
        let b = Progression::with_xp(curve, 300);
        assert_eq!(hash_state(&a), hash_state(&b));
        let c = Progression::with_xp(curve, 301);
        assert_ne!(hash_state(&a), hash_state(&c), "xp change must alter hash");
    }
}
