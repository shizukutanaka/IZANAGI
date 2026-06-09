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

    /// Uniform integer in `[lo, hi)`. An empty range (`lo >= hi`) returns `lo`
    /// **without** drawing, mirroring [`SplitMix64::below`] so the draw count
    /// stays a deterministic function of the arguments. Uses a single low-bias
    /// draw via `below`.
    #[inline]
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if lo >= hi {
            return lo;
        }
        // hi > lo, so the span is in 1..=2^32 and fits a u32.
        let span = (hi as i64 - lo as i64) as u32;
        (lo as i64 + self.below(span) as i64) as i32
    }

    /// Returns `true` with probability `num/den` using one low-bias draw.
    /// Degenerate odds are resolved without drawing (deterministic): `den == 0`
    /// or `num == 0` is always `false`, and `num >= den` is always `true`.
    #[inline]
    pub fn coin(&mut self, num: u32, den: u32) -> bool {
        if den == 0 || num == 0 {
            return false;
        }
        if num >= den {
            return true;
        }
        self.below(den) < num
    }

    /// Pick an index in `0..weights.len()` with probability proportional to its
    /// weight (a loot/spawn table). Zero-weight entries are never chosen. Returns
    /// `None` — without drawing — for an empty slice or all-zero weights. Uses a
    /// single low-bias draw; the weight sum is accumulated in `u64`, so many
    /// large `u32` weights cannot overflow.
    pub fn weighted_index(&mut self, weights: &[u32]) -> Option<usize> {
        let total: u64 = weights.iter().map(|&w| w as u64).sum();
        if total == 0 {
            return None;
        }
        // Wide-multiply pick in [0, total), the u64 analogue of `below`.
        let mut pick = ((self.next_u64() as u128 * total as u128) >> 64) as u64;
        for (i, &w) in weights.iter().enumerate() {
            let w = w as u64;
            if pick < w {
                return Some(i);
            }
            pick -= w;
        }
        // Rounding can't normally reach here; fall back to the last real entry.
        weights.iter().rposition(|&w| w > 0)
    }

    /// Roll `count` dice of `sides` faces and sum them (the tabletop `NdM`). Each
    /// die yields `1..=sides`. `sides == 0` returns 0; `count == 0` returns 0.
    /// Draws exactly `count` times (when `sides > 0`); the sum saturates rather
    /// than overflowing.
    pub fn dice(&mut self, count: u32, sides: u32) -> u32 {
        if sides == 0 {
            return 0;
        }
        let mut sum = 0u32;
        for _ in 0..count {
            sum = sum.saturating_add(self.below(sides) + 1);
        }
        sum
    }

    /// Shuffle `slice` in-place using Fisher-Yates. Draws `slice.len() - 1`
    /// times (or 0 for slices shorter than 2). Deterministic for a given seed
    /// and draw position — identical to any spec-compliant Fisher-Yates
    /// implementation using `below` for the index draw.
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        let n = slice.len();
        for i in (1..n).rev() {
            let j = self.below((i + 1) as u32) as usize;
            slice.swap(i, j);
        }
    }

    /// Return a random element from `slice`, or `None` if it is empty. Consumes
    /// one draw (via [`below`](Self::below)). Mirrors `rot.js getItem` and
    /// `bracket-random`'s implicit random-element API.
    #[inline]
    pub fn pick<'a, T>(&mut self, slice: &'a [T]) -> Option<&'a T> {
        if slice.is_empty() {
            return None;
        }
        Some(&slice[self.below(slice.len() as u32) as usize])
    }

    /// Advance the stream and return the upper 32 bits of the 64-bit output.
    /// Useful when the caller only needs a 32-bit integer and wants to avoid
    /// discarding bits in a subsequent narrow cast.
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Snapshot of internal state — fold into the world hash to detect RNG
    /// stream divergence between two runs.
    #[inline]
    pub fn state(&self) -> u64 {
        self.state
    }

    /// Reset the RNG stream to `seed`. Subsequent draws will be identical to
    /// `SplitMix64::new(seed)`. Useful for deterministic branching in replay
    /// scenarios and for re-rolling a sub-generation with a different seed
    /// without allocating a new instance.
    #[inline]
    pub fn reseed(&mut self, seed: u64) {
        self.state = seed;
    }

    /// Return `true` with 50% probability — equivalent to `coin(1, 2)` but
    /// without the extra arithmetic. Consumes one draw. Never draws for empty
    /// ranges (same contract as `coin`).
    #[inline]
    pub fn next_bool(&mut self) -> bool {
        self.coin(1, 2)
    }
}

impl crate::world_hash::DetHash for SplitMix64 {
    /// Folds the stream position so RNG divergence shows up in the world hash.
    #[inline]
    fn det_hash(&self, hasher: &mut crate::world_hash::Fnv1a) {
        hasher.write_u64(self.state);
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

    #[test]
    fn test_range_stays_within_bounds() {
        let mut r = SplitMix64::new(7);
        for _ in 0..1000 {
            let v = r.range(-5, 5);
            assert!((-5..5).contains(&v));
        }
    }

    #[test]
    fn test_range_empty_returns_lo_without_drawing() {
        let mut r = SplitMix64::new(7);
        let before = r.state();
        assert_eq!(r.range(3, 3), 3);
        assert_eq!(r.range(9, 2), 9);
        assert_eq!(r.state(), before, "empty range must not advance the stream");
        // A unit range is always its single value.
        assert_eq!(r.range(4, 5), 4);
    }

    #[test]
    fn test_coin_degenerate_odds_do_not_draw() {
        let mut r = SplitMix64::new(7);
        let before = r.state();
        assert!(!r.coin(0, 10));
        assert!(!r.coin(5, 0));
        assert!(r.coin(10, 10));
        assert!(r.coin(11, 10));
        assert_eq!(
            r.state(),
            before,
            "degenerate odds must not advance the stream"
        );
    }

    #[test]
    fn test_coin_is_deterministic_and_plausible() {
        let mut a = SplitMix64::new(123);
        let mut b = SplitMix64::new(123);
        let heads = (0..1000).filter(|_| a.coin(1, 2)).count();
        let heads2 = (0..1000).filter(|_| b.coin(1, 2)).count();
        assert_eq!(heads, heads2, "same seed → same coin sequence");
        // Fair coin over 1000 trials should land well away from the extremes.
        assert!((300..700).contains(&heads), "implausible fairness: {heads}");
    }

    #[test]
    fn test_weighted_index_empty_and_all_zero_return_none() {
        let mut r = SplitMix64::new(42);
        assert_eq!(r.weighted_index(&[]), None);
        let before = r.state();
        assert_eq!(r.weighted_index(&[0, 0, 0]), None);
        assert_eq!(
            r.state(),
            before,
            "all-zero weights must not advance the stream"
        );
    }

    #[test]
    fn test_weighted_index_single_nonzero_always_chosen() {
        let mut r = SplitMix64::new(99);
        for _ in 0..20 {
            assert_eq!(r.weighted_index(&[0, 0, 7, 0]), Some(2));
        }
    }

    #[test]
    fn test_weighted_index_proportional_distribution() {
        let mut r = SplitMix64::new(0xABCD);
        let mut counts = [0u32; 3];
        let weights = [1u32, 2, 7]; // 10%, 20%, 70%
        for _ in 0..1000 {
            counts[r.weighted_index(&weights).unwrap()] += 1;
        }
        // Allow generous slack (±15%) — correctness, not statistics.
        assert!(
            (50..200).contains(&counts[0]),
            "weight-1 bucket: {}",
            counts[0]
        );
        assert!(
            (100..350).contains(&counts[1]),
            "weight-2 bucket: {}",
            counts[1]
        );
        assert!(
            (550..850).contains(&counts[2]),
            "weight-7 bucket: {}",
            counts[2]
        );
    }

    #[test]
    fn test_weighted_index_is_deterministic() {
        let run = || {
            let mut r = SplitMix64::new(7);
            (0..50)
                .map(|_| r.weighted_index(&[3, 1, 2]).unwrap())
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn test_dice_sides_zero_returns_zero() {
        let mut r = SplitMix64::new(1);
        let before = r.state();
        assert_eq!(r.dice(5, 0), 0);
        assert_eq!(r.state(), before, "sides=0 must not draw");
    }

    #[test]
    fn test_dice_count_zero_returns_zero() {
        let mut r = SplitMix64::new(1);
        let before = r.state();
        assert_eq!(r.dice(0, 6), 0);
        assert_eq!(r.state(), before, "count=0 must not draw");
    }

    #[test]
    fn test_dice_sum_within_bounds() {
        let mut r = SplitMix64::new(77);
        for _ in 0..200 {
            let v = r.dice(3, 6); // 3d6: [3, 18]
            assert!((3..=18).contains(&v), "3d6 out of range: {v}");
        }
    }

    #[test]
    fn test_shuffle_is_deterministic() {
        let mut a = SplitMix64::new(0xF00D);
        let mut b = SplitMix64::new(0xF00D);
        let mut va: Vec<u32> = (0..10).collect();
        let mut vb: Vec<u32> = (0..10).collect();
        a.shuffle(&mut va);
        b.shuffle(&mut vb);
        assert_eq!(va, vb);
    }

    #[test]
    fn test_shuffle_is_permutation() {
        let mut r = SplitMix64::new(42);
        let mut v: Vec<u32> = (0..8).collect();
        r.shuffle(&mut v);
        let mut sorted = v.clone();
        sorted.sort();
        assert_eq!(sorted, (0..8).collect::<Vec<_>>(), "all elements survive");
    }

    #[test]
    fn test_shuffle_empty_and_single_no_panic() {
        let mut r = SplitMix64::new(1);
        let mut empty: Vec<u32> = Vec::new();
        r.shuffle(&mut empty); // no-op
        let mut single = vec![42u32];
        r.shuffle(&mut single); // no draw consumed
        assert_eq!(single, [42]);
    }

    #[test]
    fn test_shuffle_changes_order() {
        // With a 10-element list and a fresh seed, the shuffle should differ
        // from the identity (may rarely fail if seed happens to produce identity,
        // but that probability is 1/10! ≈ 2.8e-7 and the seed is fixed).
        let mut r = SplitMix64::new(12345);
        let orig: Vec<u32> = (0..10).collect();
        let mut v = orig.clone();
        r.shuffle(&mut v);
        assert_ne!(
            v, orig,
            "shuffle did not change order — suspicious with this seed"
        );
    }

    #[test]
    fn test_dice_is_deterministic() {
        let run = || {
            let mut r = SplitMix64::new(55);
            (0..50).map(|_| r.dice(2, 8)).collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn test_pick_returns_element_from_slice() {
        let mut r = SplitMix64::new(1);
        let items = [10u32, 20, 30, 40, 50];
        for _ in 0..200 {
            let v = r.pick(&items).unwrap();
            assert!(items.contains(v));
        }
    }

    #[test]
    fn test_pick_empty_returns_none() {
        let mut r = SplitMix64::new(7);
        let empty: &[u32] = &[];
        assert_eq!(r.pick(empty), None);
        // State must not advance on empty pick.
        let state_before = r.state();
        r.pick(empty);
        assert_eq!(r.state(), state_before);
    }

    #[test]
    fn test_pick_single_element_always_returns_it() {
        let mut r = SplitMix64::new(99);
        let single = [42u32];
        for _ in 0..50 {
            assert_eq!(r.pick(&single), Some(&42));
        }
    }

    #[test]
    fn test_next_u32_produces_varied_values() {
        let mut r = SplitMix64::new(0xABCD);
        let vals: Vec<u32> = (0..16).map(|_| r.next_u32()).collect();
        // Not all the same — genuine variation expected from a good PRNG.
        assert!(vals.windows(2).any(|w| w[0] != w[1]));
    }

    #[test]
    fn test_next_u32_is_deterministic() {
        let run = |seed: u64| {
            let mut r = SplitMix64::new(seed);
            (0..16).map(|_| r.next_u32()).collect::<Vec<_>>()
        };
        assert_eq!(run(5), run(5));
        assert_ne!(run(5), run(6));
    }

    #[test]
    fn test_reseed_resets_stream() {
        let mut a = SplitMix64::new(42);
        let mut b = SplitMix64::new(42);
        // Advance a, then reseed back to 42.
        a.next_u64();
        a.next_u64();
        a.reseed(42);
        // Both should now produce the same sequence.
        assert_eq!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn test_reseed_to_different_seed_diverges() {
        let mut r = SplitMix64::new(1);
        let v1 = r.next_u64();
        r.reseed(99);
        let v2 = r.next_u64();
        assert_ne!(v1, v2);
    }

    #[test]
    fn test_next_bool_is_bool() {
        let mut r = SplitMix64::new(0);
        for _ in 0..20 {
            let _ = r.next_bool(); // must not panic
        }
    }

    #[test]
    fn test_next_bool_roughly_half_true() {
        let mut r = SplitMix64::new(12345);
        let trues = (0..1000).filter(|_| r.next_bool()).count();
        // With 1000 draws expect ~500 trues; allow generous margin.
        assert!((400..=600).contains(&trues), "trues={trues}");
    }
}
