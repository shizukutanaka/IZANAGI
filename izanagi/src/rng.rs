//! Deterministic random numbers.
//!
//! xorshift64 — small, fast, good enough for games. Not for cryptography.
//! Same seed always produces the same sequence, which matters for replays
//! and networked games.

/// A seeded pseudo-random generator.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// Create from a seed. A seed of 0 becomes 1 (xorshift requires non-zero).
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    /// Seed from the current system time. Non-deterministic; use sparingly.
    pub fn from_entropy() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9e3779b97f4a7c15);
        Self::new(now)
    }

    /// Next u64.
    pub fn u64(&mut self) -> u64 {
        // xorshift64
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Next u32.
    pub fn u32(&mut self) -> u32 {
        self.u64() as u32
    }

    /// Uniform f32 in [0, 1).
    pub fn f32(&mut self) -> f32 {
        // 24-bit mantissa precision.
        (self.u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform f32 in `[lo, hi)`.
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.f32() * (hi - lo)
    }

    /// Uniform i32 in `[lo, hi)`.
    pub fn int_range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        let span = (hi - lo) as u32;
        lo + (self.u32() % span) as i32
    }

    /// Random element of a slice, or `None` if empty.
    pub fn choose<'a, T>(&mut self, s: &'a [T]) -> Option<&'a T> {
        if s.is_empty() {
            None
        } else {
            Some(&s[self.u32() as usize % s.len()])
        }
    }

    /// Coin flip — true with probability `p`.
    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }
}

impl Default for Rng {
    /// Deterministic default seed. Good for tests and reproducible demos.
    fn default() -> Self {
        Self::new(0xCAFEBABE_DEADBEEF)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.u64(), b.u64());
        }
    }

    #[test]
    fn zero_seed_handled() {
        let mut r = Rng::new(0);
        let _ = r.u64(); // must not hang / panic
    }

    #[test]
    fn f32_in_range() {
        let mut r = Rng::new(1);
        for _ in 0..10_000 {
            let v = r.f32();
            assert!((0.0..1.0).contains(&v));
        }
    }

    #[test]
    fn range_respects_bounds() {
        let mut r = Rng::new(1);
        for _ in 0..1000 {
            let v = r.range(-10.0, 10.0);
            assert!((-10.0..10.0).contains(&v));
        }
    }

    #[test]
    fn int_range_inclusive_low_exclusive_high() {
        let mut r = Rng::new(1);
        let mut seen_lo = false;
        let mut seen_hi_minus_1 = false;
        for _ in 0..2000 {
            let v = r.int_range(0, 5);
            assert!((0..5).contains(&v));
            if v == 0 {
                seen_lo = true;
            }
            if v == 4 {
                seen_hi_minus_1 = true;
            }
        }
        assert!(seen_lo && seen_hi_minus_1);
    }

    #[test]
    fn choose_from_empty_is_none() {
        let empty: &[i32] = &[];
        assert!(Rng::new(1).choose(empty).is_none());
    }

    #[test]
    fn chance_probability_converges() {
        let mut r = Rng::new(1);
        let mut hits = 0;
        for _ in 0..10_000 {
            if r.chance(0.3) {
                hits += 1;
            }
        }
        let rate = hits as f32 / 10_000.0;
        assert!((rate - 0.3).abs() < 0.05, "rate = {rate}");
    }
}
