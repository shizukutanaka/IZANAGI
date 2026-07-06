//! Cyclical integer time — day/night cycle derived from a monotone tick counter.
//!
//! The kit had two time abstractions:
//! - *Monotone* time: [`timer::Cooldown`](crate::timer::Cooldown) counts down,
//!   [`progression::Progression`](crate::progression::Progression) counts up.
//! - *Scheduled* time: [`timer::TimerQueue`](crate::timer::TimerQueue) fires
//!   events at future tick offsets.
//!
//! Neither captured **cyclical** time — "what time of day is it after N turns?"
//! A [`Calendar`] holds a monotone tick counter and a fixed day length, and
//! derives the current phase of the cycle via modular arithmetic. Typical uses:
//!
//! - `ambient_fill(calendar.brightness())` — dim the dungeon at night.
//! - `faction.modify(...)` weighted by `calendar.is_in_phase(...)` — nocturnal
//!   hostility.
//! - `ability.use_ability(...)` gated on `calendar.time_of_day()` — sunrise
//!   spells only available at dawn.
//!
//! ```
//! use izanagi_kit::calendar::Calendar;
//!
//! // 24 ticks per day (one per turn, for illustration).
//! let mut cal = Calendar::new(24);
//! assert_eq!(cal.time_of_day(), 0);
//! assert_eq!(cal.day_number(), 0);
//!
//! cal.advance(30);   // into day 1
//! assert_eq!(cal.day_number(), 1);
//! assert_eq!(cal.time_of_day(), 6);   // 30 % 24
//! assert_eq!(cal.ticks_until_wrap(), 18); // 24 - 6
//!
//! // Is it within the "night" phase? (ticks 18–23 of each day)
//! assert!(!cal.is_in_phase(18, 24)); // currently tick 6 — daytime
//! ```
//!
//! Determinism: every value is a closed-form function of `u64 tick` and
//! `u32 ticks_per_day`, computed with wrapping/integer arithmetic and no float.
//! [`Calendar`] implements [`DetHash`](crate::world_hash::DetHash), folding the
//! current tick into the replay checksum.

use crate::world_hash::{DetHash, Fnv1a};

/// A cyclical integer clock: tracks a monotone tick counter and derives the
/// current position within a repeating day cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Calendar {
    tick: u64,
    ticks_per_day: u32,
}

impl Calendar {
    /// Create a `Calendar` starting at tick 0. `ticks_per_day` is clamped to
    /// at least 1 (a degenerate 1-tick day — every tick is the whole day).
    pub fn new(ticks_per_day: u32) -> Self {
        Calendar {
            tick: 0,
            ticks_per_day: ticks_per_day.max(1),
        }
    }

    /// Resume from a saved tick position.
    pub fn with_tick(ticks_per_day: u32, tick: u64) -> Self {
        Calendar {
            tick,
            ticks_per_day: ticks_per_day.max(1),
        }
    }

    /// The number of ticks in one full day cycle.
    #[inline]
    pub fn ticks_per_day(&self) -> u32 {
        self.ticks_per_day
    }

    /// Total elapsed ticks since the start (monotone).
    #[inline]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// The current position within today's cycle: `tick % ticks_per_day`,
    /// always in `[0, ticks_per_day)`.
    #[inline]
    pub fn time_of_day(&self) -> u32 {
        (self.tick % self.ticks_per_day as u64) as u32
    }

    /// The completed number of full days elapsed: `tick / ticks_per_day`.
    #[inline]
    pub fn day_number(&self) -> u64 {
        self.tick / self.ticks_per_day as u64
    }

    /// Ticks remaining in the current day: `ticks_per_day - time_of_day`.
    /// Always in `1..=ticks_per_day`.
    #[inline]
    pub fn ticks_until_wrap(&self) -> u32 {
        self.ticks_per_day - self.time_of_day()
    }

    /// The fraction of the current day elapsed as a value in `[0, 1000)` (per
    /// mille). Computed as `time_of_day * 1000 / ticks_per_day`. Useful for
    /// scaling ambient light or enemy activity without division in hot loops:
    /// divide your range by 1000 offline instead.
    pub fn fraction_per_mille(&self) -> u32 {
        (self.time_of_day() as u64 * 1000 / self.ticks_per_day as u64) as u32
    }

    /// `true` if the current `time_of_day` is in the half-open range
    /// `[start, end)`, **wrapping** if `end <= start` (e.g. a phase that spans
    /// midnight). Both `start` and `end` are taken modulo `ticks_per_day`.
    ///
    /// Examples:
    /// - `is_in_phase(6, 18)` — daytime (ticks 6–17)
    /// - `is_in_phase(18, 6)` — nighttime (ticks 18–23 **and** 0–5)
    /// - `is_in_phase(0, 0)` — always true (full-day phase)
    pub fn is_in_phase(&self, start: u32, end: u32) -> bool {
        let s = start % self.ticks_per_day;
        let e = end % self.ticks_per_day;
        let t = self.time_of_day();
        if s == e {
            true // degenerate: covers the whole day
        } else if s < e {
            t >= s && t < e
        } else {
            // Wrapping phase (spans midnight).
            t >= s || t < e
        }
    }

    /// Advance the tick counter by `n` ticks (saturating).
    /// Returns the number of full day cycles that were completed during this
    /// advance — useful for triggering "day changed" events.
    pub fn advance(&mut self, n: u64) -> u64 {
        let before = self.day_number();
        self.tick = self.tick.saturating_add(n);
        self.day_number() - before
    }

    /// Reset to tick 0 (start of a new game / time reset).
    pub fn reset(&mut self) {
        self.tick = 0;
    }

    /// A convenience brightness value in `[0, 255]` that peaks at midday
    /// (`time_of_day == ticks_per_day / 2`) and is `min_brightness` at
    /// midnight. Useful as an argument to
    /// [`LightMap::ambient_fill`](crate::lightmap::LightMap::ambient_fill).
    ///
    /// Uses the integer cosine approximation:
    /// `brightness = min + (max - min) * (1000 - |fraction_per_mille*2 - 1000|) / 1000`
    pub fn ambient_brightness(&self, min_brightness: u8, max_brightness: u8) -> u8 {
        if max_brightness <= min_brightness {
            return min_brightness;
        }
        let f = self.fraction_per_mille(); // 0..1000
                                           // Triangle wave peaking at f=500 (midday).
        let peak_dist = if f <= 500 { f } else { 1000 - f };
        // peak_dist in 0..=500; scale to [min, max].
        let range = (max_brightness - min_brightness) as u32;
        let contrib = range * peak_dist / 500;
        min_brightness + contrib as u8
    }
}

impl DetHash for Calendar {
    fn det_hash(&self, hasher: &mut Fnv1a) {
        hasher.write_u64(self.tick);
        hasher.write_u32(self.ticks_per_day);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[test]
    fn test_new_starts_at_zero() {
        let c = Calendar::new(24);
        assert_eq!(c.tick(), 0);
        assert_eq!(c.time_of_day(), 0);
        assert_eq!(c.day_number(), 0);
        assert_eq!(c.ticks_until_wrap(), 24);
    }

    #[test]
    fn test_modular_decomposition() {
        let mut c = Calendar::new(24);
        c.advance(30);
        assert_eq!(c.day_number(), 1);
        assert_eq!(c.time_of_day(), 6);
        // Invariant: day * ticks_per_day + time_of_day == tick.
        assert_eq!(
            c.day_number() * c.ticks_per_day() as u64 + c.time_of_day() as u64,
            c.tick()
        );
    }

    #[test]
    fn test_ticks_until_wrap() {
        let c = Calendar::with_tick(24, 30);
        assert_eq!(c.time_of_day(), 6);
        assert_eq!(c.ticks_until_wrap(), 18);
        assert_eq!(c.ticks_until_wrap() as u64 + c.time_of_day() as u64, 24);
    }

    #[test]
    fn test_advance_returns_days_completed() {
        let mut c = Calendar::new(24);
        let days = c.advance(49); // 49 / 24 = 2 full days
        assert_eq!(days, 2);
        let days2 = c.advance(1); // still on day 2
        assert_eq!(days2, 0);
    }

    #[test]
    fn test_is_in_phase_non_wrapping() {
        let c = Calendar::with_tick(24, 10); // time_of_day = 10
        assert!(c.is_in_phase(6, 18), "10 is in [6,18)");
        assert!(!c.is_in_phase(0, 6), "10 is not in [0,6)");
        assert!(!c.is_in_phase(18, 24), "10 is not in [18,24)");
    }

    #[test]
    fn test_is_in_phase_wrapping() {
        let c = Calendar::with_tick(24, 2); // time_of_day = 2 (early morning)
        assert!(c.is_in_phase(18, 6), "2 is in night phase [18,24)+[0,6)");
        let c2 = Calendar::with_tick(24, 20); // nighttime
        assert!(c2.is_in_phase(18, 6), "20 is in night phase");
        let c3 = Calendar::with_tick(24, 10); // daytime
        assert!(!c3.is_in_phase(18, 6), "10 is not in night phase");
    }

    #[test]
    fn test_is_in_phase_degenerate_equal() {
        let c = Calendar::with_tick(24, 5);
        assert!(c.is_in_phase(5, 5), "start==end covers entire day");
        assert!(c.is_in_phase(0, 0));
    }

    #[test]
    fn test_fraction_per_mille_range() {
        for tick in 0u64..48 {
            let c = Calendar::with_tick(24, tick);
            let f = c.fraction_per_mille();
            assert!(f < 1000, "fraction must be < 1000 (got {f} at tick {tick})");
        }
    }

    #[test]
    fn test_fraction_per_mille_at_boundaries() {
        let c0 = Calendar::with_tick(24, 0);
        assert_eq!(c0.fraction_per_mille(), 0);
        let c12 = Calendar::with_tick(24, 12);
        assert_eq!(c12.fraction_per_mille(), 500);
    }

    #[test]
    fn test_ambient_brightness_peaks_at_midday() {
        let c_night = Calendar::with_tick(24, 0); // midnight
        let c_day = Calendar::with_tick(24, 12); // noon
        let night_b = c_night.ambient_brightness(20, 200);
        let noon_b = c_day.ambient_brightness(20, 200);
        assert_eq!(night_b, 20, "midnight is at minimum brightness");
        assert_eq!(noon_b, 200, "noon is at maximum brightness");
    }

    #[test]
    fn test_reset_goes_to_zero() {
        let mut c = Calendar::with_tick(24, 100);
        c.reset();
        assert_eq!(c.tick(), 0);
        assert_eq!(c.day_number(), 0);
        assert_eq!(c.time_of_day(), 0);
    }

    #[test]
    fn test_ticks_per_day_minimum_one() {
        let c = Calendar::new(0);
        assert_eq!(c.ticks_per_day(), 1);
    }

    #[test]
    fn test_det_hash_canonical_and_sensitive() {
        let a = Calendar::with_tick(24, 50);
        let b = Calendar::with_tick(24, 50);
        assert_eq!(hash_state(&a), hash_state(&b), "same state, same hash");
        let c = Calendar::with_tick(24, 51);
        assert_ne!(
            hash_state(&a),
            hash_state(&c),
            "different tick, different hash"
        );
        let d = Calendar::with_tick(48, 50);
        assert_ne!(
            hash_state(&a),
            hash_state(&d),
            "different period, different hash"
        );
    }
}
