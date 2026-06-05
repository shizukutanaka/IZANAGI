//! SplitMix64 — a small, fast, fully deterministic PRNG.
//!
//! For replay and lockstep, randomness must be a pure function of (seed, draw
//! count). A single explicit stream advanced in a fixed order removes RNG as a
//! non-determinism source (arxiv/Bevy determinism audit). Never seed from wall
//! clock or thread-local state in code that must replay.

/// Deterministic 64-bit generator. `Clone` to snapshot/restore a stream.
#[derive(Clone, Debug)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Advances and returns the next 64-bit value.
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform-ish integer in `[0, bound)` via wide multiply. Deterministic for
    /// a given draw position.
    ///
    /// `bound == 0` denotes an empty range: it returns `0` **without** drawing,
    /// identically in debug and release. The old `debug_assert!`-only guard let
    /// release builds silently consume a draw and return `0`, which could desync
    /// a replay between profiles; this makes the behaviour explicit and uniform.
    #[inline]
    pub fn below(&mut self, bound: u32) -> u32 {
        if bound == 0 {
            return 0;
        }
        let product = (self.next_u64() as u128).wrapping_mul(bound as u128);
        (product >> 64) as u32
    }

    /// Snapshot of internal state — fold into the world hash to detect RNG
    /// stream divergence between two runs.
    #[inline]
    pub fn state(&self) -> u64 {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_seed_yields_identical_sequence() {
        let mut a = SplitMix64::new(0xDEAD_BEEF);
        let mut b = SplitMix64::new(0xDEAD_BEEF);
        let sa: Vec<u64> = (0..8).map(|_| a.next_u64()).collect();
        let sb: Vec<u64> = (0..8).map(|_| b.next_u64()).collect();
        assert_eq!(sa, sb);
    }

    #[test]
    fn test_different_seed_diverges() {
        let mut a = SplitMix64::new(1);
        let mut b = SplitMix64::new(2);
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn test_below_stays_within_bound() {
        let mut r = SplitMix64::new(42);
        for _ in 0..1000 {
            assert!(r.below(10) < 10);
        }
    }

    #[test]
    fn test_below_zero_returns_zero_without_drawing() {
        // Empty range is defined and side-effect-free: no panic, no draw consumed,
        // so the stream position is identical in debug and release builds.
        let mut r = SplitMix64::new(42);
        let before = r.state();
        assert_eq!(r.below(0), 0);
        assert_eq!(r.state(), before, "below(0) must not advance the stream");
    }

    #[test]
    fn test_known_first_draw_is_pinned() {
        // Pins the algorithm so an accidental constant change is caught.
        let mut r = SplitMix64::new(0);
        assert_eq!(r.next_u64(), 0xE220_A839_7B1D_CDAF);
    }
}
