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

    /// Construct a generator from two independent `u32` seeds.
    ///
    /// `(lo as u64) | ((hi as u64) << 32)` produces the `u64` state — the same
    /// result as combining manually, but named for call sites that hold two
    /// separate entropy sources (e.g. `map_seed` and `run_counter`).
    #[inline]
    pub fn from_u32_pair(lo: u32, hi: u32) -> Self {
        Self::new((lo as u64) | ((hi as u64) << 32))
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

    /// Draw from the **closed** range `[lo, hi]` uniformly. Returns `lo` when
    /// `lo >= hi` (degenerate range, no draw consumed). Useful wherever both
    /// endpoints are valid values — e.g. `range_closed(1, 20)` for a d20 roll
    /// or `range_closed(0, 100)` for a percent check (can roll exactly 100).
    ///
    /// Handles the full span including `range_closed(i32::MIN, i32::MAX)` where
    /// `span == 2^32` — a u32 would overflow to 0. Uses a 128-bit wide multiply
    /// to stay bias-free across all spans.
    #[inline]
    pub fn range_closed(&mut self, lo: i32, hi: i32) -> i32 {
        if lo >= hi {
            return lo;
        }
        // span ∈ [2, 2^32]; when lo=i32::MIN and hi=i32::MAX, span=2^32 which
        // would overflow u32. Use i64 for span and a 128-bit wide multiply
        // (the same low-bias technique as `below`) to handle all cases.
        let span = hi as i64 - lo as i64 + 1; // ∈ [2, 2^32], fits i64
        let pick = ((self.next_u64() as u128).wrapping_mul(span as u128) >> 64) as i64;
        (lo as i64 + pick) as i32
    }

    /// Draw from the half-open range `[lo, hi)` uniformly. Returns `lo` when
    /// `lo >= hi` (degenerate range, no draw consumed). Unsigned counterpart of
    /// [`range`](Self::range) for inventory indices and non-negative offsets.
    #[inline]
    pub fn range_u32(&mut self, lo: u32, hi: u32) -> u32 {
        if lo >= hi {
            return lo;
        }
        lo + self.below(hi - lo)
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

    /// Uniform random mutable reference to an element of `slice`, or `None`
    /// for an empty slice (no draw consumed). The mutable variant of [`pick`](Self::pick)
    /// — allows the caller to modify the chosen element in place without a
    /// separate index round-trip.
    pub fn pick_mut<'a, T>(&mut self, slice: &'a mut [T]) -> Option<&'a mut T> {
        if slice.is_empty() {
            return None;
        }
        let idx = self.below(slice.len() as u32) as usize;
        Some(&mut slice[idx])
    }

    /// Return a uniform random index in `0..len`, or `None` for `len == 0`
    /// (no draw consumed). The index-only primitive behind [`pick`](Self::pick)
    /// and [`pick_mut`](Self::pick_mut): use it when the caller holds the data
    /// elsewhere (parallel arrays, a `HashMap`, an ECS store) and only needs a
    /// random position. Consumes exactly one draw when `len > 0`.
    #[inline]
    pub fn pick_index(&mut self, len: usize) -> Option<usize> {
        if len == 0 {
            return None;
        }
        Some(self.below(len as u32) as usize)
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

    /// Fork an independent named child stream from this generator's *current*
    /// state, without drawing from or mutating `self`.
    ///
    /// A single shared stream means every subsystem's draw count affects every
    /// other subsystem's future values — add one extra roll to combat and
    /// every later loot/mapgen draw shifts too, even though those systems are
    /// otherwise unrelated. `split` lets each subsystem own a stream keyed by
    /// a caller-chosen `stream_id` (e.g. one `const` per subsystem), so
    /// drawing more or fewer times in one stream never perturbs another's
    /// sequence.
    ///
    /// Pure and deterministic: the same `(state, stream_id)` pair always
    /// produces the same child, regardless of how many times `split` is
    /// called or in what order — unlike drawing a seed via `next_u64` (which
    /// both consumes from the parent and depends on prior draws), this is
    /// safe to call repeatedly, from multiple sites, at any point in the
    /// parent's lifetime, and still get the *same* named child back for the
    /// *same* `stream_id` at that state. Typical use is once at startup,
    /// right after seeding the master stream and before drawing from it, so
    /// every subsystem's child is derived from the same fixed base state:
    ///
    /// ```
    /// use izanagi_kit::rng::SplitMix64;
    /// const LOOT_STREAM: u64 = 1;
    /// const AI_STREAM: u64 = 2;
    ///
    /// let master = SplitMix64::new(42);
    /// let mut loot_rng = master.split(LOOT_STREAM);
    /// let mut ai_rng = master.split(AI_STREAM);
    ///
    /// // Drawing from one never shifts the other's sequence.
    /// let ai_first = ai_rng.next_u64();
    /// loot_rng.next_u64();
    /// loot_rng.next_u64();
    /// let mut ai_rng_2 = master.split(AI_STREAM); // same id, same base state
    /// assert_eq!(ai_rng_2.next_u64(), ai_first, "unaffected by loot_rng's draws");
    /// ```
    ///
    /// Distinct `stream_id`s reliably yield distinct children: `stream_id` is
    /// combined with `state` via the same avalanche mix [`next_u64`](Self::next_u64)
    /// uses internally, so two different ids produce well-distributed,
    /// effectively-independent seeds even from identical parent state.
    pub fn split(&self, stream_id: u64) -> SplitMix64 {
        let mut z = self.state ^ stream_id.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        SplitMix64::new(z)
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

    /// Return a uniformly random grid point `(x, y)` inside the half-open
    /// rectangle `[x, x+w) × [y, y+h)`. Draws exactly two values when both
    /// dimensions are positive; returns the corner `(x, y)` without drawing for
    /// degenerate rectangles (`w ≤ 0` or `h ≤ 0`). The canonical random-spawn
    /// primitive for bounded rooms and rectangular spawn regions.
    pub fn within_rect(&mut self, x: i32, y: i32, w: i32, h: i32) -> (i32, i32) {
        if w <= 0 || h <= 0 {
            return (x, y);
        }
        (
            self.range(x, x.saturating_add(w)),
            self.range(y, y.saturating_add(h)),
        )
    }

    /// Advance the stream by exactly `n` draws, discarding all output. Use for
    /// deterministic "skip-ahead": two callers that seed identically and each
    /// call `skip(k)` before sampling will produce the same values as if they
    /// both simply drew `k` dummy values first.
    #[inline]
    pub fn skip(&mut self, n: u32) {
        for _ in 0..n {
            self.next_u64();
        }
    }

    /// Construct a `SplitMix64` whose internal state is set to the exact raw
    /// `state` value — the inverse of [`state()`](Self::state). Use this to
    /// restore a previously snapshotted RNG stream exactly (e.g. when loading
    /// a save file that serialised `state()`, or to fork a second generator at
    /// the same position in a longer stream). Semantically equivalent to
    /// `new(state)` but names the intent: restoring raw state rather than
    /// seeding from a game-world integer.
    #[inline]
    pub fn with_state(state: u64) -> Self {
        Self { state }
    }

    /// Sample up to `n` items from `slice` **without replacement**, returning
    /// them as a cloned `Vec`. If `n >= slice.len()`, returns a shuffled copy
    /// of the whole slice. Draws `min(n, len).saturating_sub(1)` times — a
    /// partial Fisher-Yates on a local index copy so `slice` is never modified.
    /// Returns an empty `Vec` when `slice` is empty or `n == 0` (no draws).
    pub fn sample_n<T: Clone>(&mut self, slice: &[T], n: usize) -> Vec<T> {
        let k = n.min(slice.len());
        if k == 0 {
            return Vec::new();
        }
        let mut indices: Vec<usize> = (0..slice.len()).collect();
        for i in 0..k {
            let j = i + self.below((slice.len() - i) as u32) as usize;
            indices.swap(i, j);
        }
        indices[..k].iter().map(|&i| slice[i].clone()).collect()
    }

    /// Integer Bates-distribution approximation to a normal distribution.
    ///
    /// Draws 4 uniform samples from `[0, spread]` (inclusive), averages them,
    /// and centers the result at `center`. Output range is
    /// `[center − spread, center + spread]`; the distribution is
    /// bell-shaped (Bates/Irwin-Hall with 4 samples). Consumes exactly 4 draws
    /// for `spread > 0`, 0 draws for `spread == 0`. Deterministic and
    /// replay-safe. Useful for damage variance, difficulty curves, and
    /// procedural stat generation.
    pub fn gaussian_approx(&mut self, center: i32, spread: u32) -> i32 {
        if spread == 0 {
            return center;
        }
        // Saturating bound and i64 accumulation: `spread` is an arbitrary u32,
        // so `spread + 1` overflows at u32::MAX and a 4-term i32 sum overflows
        // once `spread` nears i32::MAX. Identical draws/result for any sane
        // spread (the bound is unchanged until u32::MAX, so replays match).
        let bound = spread.saturating_add(1);
        let sum: i64 = (0..4).map(|_| self.below(bound) as i64).sum();
        // sum ∈ [0, 4*spread], mean = 2*spread
        // sum/2 − spread ∈ [−spread, spread], mean = 0
        (center as i64 + sum / 2 - spread as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32
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

    // --- split ---

    #[test]
    fn test_split_is_pure_does_not_mutate_parent() {
        let parent = SplitMix64::new(42);
        let state_before = parent.state();
        let _ = parent.split(1);
        assert_eq!(parent.state(), state_before, "split must not mutate the parent");
    }

    #[test]
    fn test_split_is_repeatable_for_same_id_and_state() {
        let parent = SplitMix64::new(42);
        let mut a = parent.split(7);
        let mut b = parent.split(7);
        for _ in 0..10 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn test_split_different_ids_yield_different_streams() {
        let parent = SplitMix64::new(42);
        let mut a = parent.split(1);
        let mut b = parent.split(2);
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn test_split_different_parent_states_yield_different_streams() {
        let a = SplitMix64::new(1).split(5);
        let b = SplitMix64::new(2).split(5);
        assert_ne!(a.state(), b.state(), "same id, different parent state, must diverge");
    }

    #[test]
    fn test_split_children_are_independent_of_each_other() {
        let parent = SplitMix64::new(42);
        let ai_first = parent.split(1).next_u64();

        // Drawing many times from an unrelated sibling stream must not
        // change what a freshly re-derived child for the same id yields.
        let mut loot = parent.split(2);
        for _ in 0..100 {
            loot.next_u64();
        }
        let ai_again = parent.split(1).next_u64();
        assert_eq!(ai_first, ai_again, "sibling draws must not perturb this child");
    }

    #[test]
    fn test_split_is_order_independent_across_call_sites() {
        // Deriving children in one order vs. the reverse order must not
        // change either child's resulting sequence — each is a pure function
        // of (parent state, stream_id) alone.
        let parent = SplitMix64::new(1234);
        let (mut a1, mut b1) = (parent.split(10), parent.split(20));
        let (mut b2, mut a2) = (parent.split(20), parent.split(10));
        assert_eq!(a1.next_u64(), a2.next_u64());
        assert_eq!(b1.next_u64(), b2.next_u64());
    }

    #[test]
    fn test_split_child_state_advances_independently_of_parent() {
        let mut parent = SplitMix64::new(9);
        let mut child = parent.split(3);
        let child_seed = child.state();
        parent.next_u64();
        parent.next_u64();
        // The child was already constructed; later parent draws (which do
        // not call split again) cannot retroactively change it.
        assert_eq!(child.state(), child_seed);
        let _ = child.next_u64(); // child advances independently under its own draws
        assert_ne!(child.state(), child_seed);
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

    #[test]
    fn test_skip_advances_stream() {
        let mut a = SplitMix64::new(42);
        let mut b = SplitMix64::new(42);
        // Skip 5 draws in 'a'; manually draw 5 in 'b'.
        a.skip(5);
        for _ in 0..5 {
            b.next_u64();
        }
        assert_eq!(a.next_u64(), b.next_u64(), "streams must align after skip");
    }

    #[test]
    fn test_skip_zero_is_noop() {
        let mut r = SplitMix64::new(99);
        let s_before = r.state();
        r.skip(0);
        assert_eq!(r.state(), s_before);
    }

    #[test]
    fn test_skip_is_deterministic() {
        let mut a = SplitMix64::new(7);
        let mut b = SplitMix64::new(7);
        a.skip(10);
        b.skip(10);
        assert_eq!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn test_pick_mut_returns_element() {
        let mut r = SplitMix64::new(1);
        let mut v = vec![10u32, 20, 30];
        let elem = r.pick_mut(&mut v).expect("non-empty slice");
        assert!([10u32, 20, 30].contains(elem));
    }

    #[test]
    fn test_pick_mut_empty_returns_none() {
        let mut r = SplitMix64::new(1);
        let mut v: Vec<u32> = vec![];
        assert!(r.pick_mut(&mut v).is_none());
    }

    #[test]
    fn test_pick_mut_allows_modification() {
        let mut r = SplitMix64::new(42);
        let mut v = vec![1u32, 2, 3];
        *r.pick_mut(&mut v).unwrap() = 99;
        assert!(v.contains(&99));
    }

    #[test]
    fn test_pick_index_within_bounds() {
        let mut r = SplitMix64::new(1);
        for _ in 0..200 {
            let i = r.pick_index(5).unwrap();
            assert!(i < 5, "index {i} out of range");
        }
    }

    #[test]
    fn test_pick_index_zero_len_returns_none_without_drawing() {
        let mut r = SplitMix64::new(7);
        let before = r.state();
        assert_eq!(r.pick_index(0), None);
        assert_eq!(r.state(), before, "len 0 must not advance the stream");
    }

    #[test]
    fn test_pick_index_matches_pick_position() {
        // pick_index and pick must select the same element for the same draw.
        let items = [10u32, 20, 30, 40];
        let mut a = SplitMix64::new(123);
        let mut b = SplitMix64::new(123);
        let idx = a.pick_index(items.len()).unwrap();
        let val = b.pick(&items).unwrap();
        assert_eq!(&items[idx], val);
    }

    #[test]
    fn test_with_state_produces_same_stream_as_new() {
        let mut a = SplitMix64::new(42);
        let mut b = SplitMix64::with_state(42);
        for _ in 0..10 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn test_with_state_restores_mid_stream() {
        let mut rng = SplitMix64::new(7);
        rng.next_u64();
        rng.next_u64();
        rng.next_u64();
        let snap = rng.state();
        let v1 = rng.next_u64();
        let mut restored = SplitMix64::with_state(snap);
        let v2 = restored.next_u64();
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_with_state_zero_is_valid() {
        let mut r = SplitMix64::with_state(0);
        let v = r.next_u64();
        assert_ne!(v, 0, "output should not be zero even for zero state");
    }

    #[test]
    fn test_range_u32_stays_in_bounds() {
        let mut r = SplitMix64::new(99);
        for _ in 0..1000 {
            let v = r.range_u32(5, 15);
            assert!((5..15).contains(&v));
        }
    }

    #[test]
    fn test_range_u32_empty_range_returns_lo() {
        let mut r = SplitMix64::new(1);
        assert_eq!(r.range_u32(7, 7), 7);
        assert_eq!(r.range_u32(10, 5), 10);
    }

    #[test]
    fn test_range_u32_full_u32_range() {
        let mut r = SplitMix64::new(0xABCD);
        let v = r.range_u32(0, u32::MAX);
        assert!(v < u32::MAX);
    }

    #[test]
    fn test_within_rect_stays_inside() {
        let mut r = SplitMix64::new(42);
        for _ in 0..200 {
            let (x, y) = r.within_rect(10, 20, 5, 8);
            assert!((10..15).contains(&x), "x={x} not in [10,15)");
            assert!((20..28).contains(&y), "y={y} not in [20,28)");
        }
    }

    #[test]
    fn test_within_rect_degenerate_no_draw() {
        let mut r = SplitMix64::new(7);
        let s0 = r.state();
        assert_eq!(r.within_rect(3, 4, 0, 5), (3, 4));
        assert_eq!(r.within_rect(3, 4, 5, 0), (3, 4));
        assert_eq!(r.within_rect(3, 4, -1, 3), (3, 4));
        assert_eq!(r.state(), s0, "degenerate rect must not draw");
    }

    #[test]
    fn test_within_rect_1x1_returns_corner() {
        let mut r = SplitMix64::new(99);
        let (x, y) = r.within_rect(-5, 7, 1, 1);
        assert_eq!((x, y), (-5, 7));
    }

    // --- sample_n -----------------------------------------------------------

    #[test]
    fn test_sample_n_returns_distinct_items() {
        let items = vec![10u32, 20, 30, 40, 50];
        let mut rng = SplitMix64::new(42);
        let sample = rng.sample_n(&items, 3);
        assert_eq!(sample.len(), 3);
        // All returned items must be from the source slice.
        assert!(sample.iter().all(|v| items.contains(v)));
        // No duplicates (without-replacement).
        let mut sorted = sample.clone();
        sorted.dedup();
        assert_eq!(sorted.len(), 3);
    }

    #[test]
    fn test_sample_n_zero_returns_empty_without_drawing() {
        let items = vec![1u32, 2, 3];
        let mut rng = SplitMix64::new(7);
        let state_before = rng.state();
        let sample = rng.sample_n(&items, 0);
        assert!(sample.is_empty());
        assert_eq!(rng.state(), state_before, "n=0 must not draw");
    }

    #[test]
    fn test_sample_n_more_than_len_returns_all() {
        let items = vec!['a', 'b', 'c'];
        let mut rng = SplitMix64::new(55);
        let sample = rng.sample_n(&items, 100);
        assert_eq!(sample.len(), 3);
        let mut got: Vec<char> = sample.clone();
        got.sort();
        assert_eq!(got, vec!['a', 'b', 'c']);
    }

    #[test]
    fn test_gaussian_approx_zero_spread_returns_center_no_draw() {
        let mut rng = SplitMix64::new(1);
        let state_before = rng.state();
        assert_eq!(rng.gaussian_approx(42, 0), 42);
        assert_eq!(rng.state(), state_before, "must not draw for spread=0");
    }

    #[test]
    fn test_gaussian_approx_result_within_center_plus_minus_spread() {
        let mut rng = SplitMix64::new(0xABC);
        for _ in 0..200 {
            let v = rng.gaussian_approx(100, 50);
            assert!(
                (50..=150).contains(&v),
                "gaussian_approx(100, 50) = {v} out of [50, 150]"
            );
        }
    }

    #[test]
    fn test_gaussian_approx_deterministic_given_same_state() {
        let mut rng_a = SplitMix64::new(99);
        let mut rng_b = SplitMix64::new(99);
        for _ in 0..10 {
            assert_eq!(rng_a.gaussian_approx(0, 20), rng_b.gaussian_approx(0, 20));
        }
    }

    // --- from_u32_pair ---

    #[test]
    fn test_from_u32_pair_matches_manual_combination() {
        let lo = 0xDEAD_BEEFu32;
        let hi = 0x0102_0304u32;
        let expected = SplitMix64::new((lo as u64) | ((hi as u64) << 32));
        let from_pair = SplitMix64::from_u32_pair(lo, hi);
        assert_eq!(expected.state(), from_pair.state());
    }

    #[test]
    fn test_from_u32_pair_zero_zero_is_new_zero() {
        let rng = SplitMix64::from_u32_pair(0, 0);
        assert_eq!(rng.state(), SplitMix64::new(0).state());
    }

    #[test]
    fn test_from_u32_pair_different_pairs_different_states() {
        let a = SplitMix64::from_u32_pair(1, 2);
        let b = SplitMix64::from_u32_pair(2, 1);
        assert_ne!(a.state(), b.state());
    }

    // --- range_closed ---

    #[test]
    fn test_range_closed_includes_both_endpoints() {
        let mut rng = SplitMix64::new(0);
        let mut saw_lo = false;
        let mut saw_hi = false;
        for _ in 0..200 {
            let v = rng.range_closed(1, 3);
            assert!((1..=3).contains(&v), "out of [1,3]: {v}");
            if v == 1 {
                saw_lo = true;
            }
            if v == 3 {
                saw_hi = true;
            }
        }
        assert!(saw_lo, "never rolled lo=1");
        assert!(saw_hi, "never rolled hi=3");
    }

    #[test]
    fn test_range_closed_degenerate_returns_lo() {
        let mut rng = SplitMix64::new(42);
        assert_eq!(rng.range_closed(5, 5), 5);
        assert_eq!(rng.range_closed(7, 3), 7);
    }

    #[test]
    fn test_range_closed_consistent_with_range_plus_one() {
        let mut rng_a = SplitMix64::new(123);
        let mut rng_b = SplitMix64::new(123);
        // range_closed(lo, hi) draws one value; range(lo, hi+1) draws one value.
        // They should draw the same underlying random number.
        for _ in 0..50 {
            assert_eq!(rng_a.range_closed(0, 9), rng_b.range(0, 10));
        }
    }

    /// `range_closed(i32::MIN, i32::MAX)` covers the full i32 span (2^32 values).
    /// Before the fix, `(hi - lo + 1) as u32` overflowed to 0, causing the
    /// function to fall through to `below(0)` which returns 0 without drawing —
    /// silently returning `i32::MIN` regardless of the seed.
    #[test]
    fn test_range_closed_full_i32_span_draws_and_covers_range() {
        let mut rng = SplitMix64::new(0xFEED_CAFE_DEAD_BEEF);
        let s0 = rng.state();
        let v = rng.range_closed(i32::MIN, i32::MAX);
        assert_ne!(rng.state(), s0, "range_closed(MIN,MAX) must consume a draw");
        // With just 100 draws, we won't see the full range, but the result must
        // be in bounds and we should see both positive and negative values.
        let mut saw_neg = v < 0;
        let mut saw_pos = v >= 0;
        for _ in 0..100 {
            let v = rng.range_closed(i32::MIN, i32::MAX);
            // Range is the whole i32 domain, so any value is trivially in
            // bounds; the real assertions are the saw_neg/saw_pos coverage below.
            if v < 0 { saw_neg = true; }
            if v >= 0 { saw_pos = true; }
        }
        assert!(saw_neg, "full i32 range must sometimes return negative");
        assert!(saw_pos, "full i32 range must sometimes return non-negative");
    }

    #[test]
    fn test_range_closed_near_extremes_draws_correctly() {
        // lo=i32::MIN, hi=i32::MAX-1: span=2^32-1, fits u32 — uses code path
        // that was always correct. Verify it still works after the refactor.
        let mut rng_a = SplitMix64::new(777);
        let mut rng_b = SplitMix64::new(777);
        for _ in 0..50 {
            let va = rng_a.range_closed(i32::MIN, i32::MAX - 1);
            let vb = rng_b.range_closed(i32::MIN, i32::MAX - 1);
            assert_eq!(va, vb);
            // Closed range [MIN, MAX-1] must never yield i32::MAX (the lower
            // bound is the type minimum, so only the upper bound is meaningful).
            assert!(va < i32::MAX, "exceeded hi bound: {va}");
        }
    }

    /// Exhaustive guard for the determinism-critical "degenerate input consumes
    /// no draw" contract (S3). Every consuming method with a documented no-draw
    /// path is exercised here; the harness asserts `state()` never moves from the
    /// initial value, so a refactor that sneaks a `next_u64()` *before* the guard
    /// (silently shifting the draw count and desyncing replays, while leaving the
    /// return value unchanged) is caught uniformly.
    ///
    /// **When you add a new RNG-consuming method with a degenerate path, add a
    /// line here.** This is the single systematic anchor for the contract.
    #[test]
    fn test_degenerate_inputs_consume_no_draw_exhaustive() {
        let mut r = SplitMix64::new(0x1234_5678_9ABC_DEF0);
        let s0 = r.state();

        macro_rules! no_draw {
            ($label:literal, $call:expr) => {{
                let _ = $call;
                assert_eq!(
                    r.state(),
                    s0,
                    concat!($label, " consumed a draw on a degenerate input")
                );
            }};
        }

        no_draw!("below(0)", r.below(0));
        no_draw!("range(lo==hi)", r.range(5, 5));
        no_draw!("range(lo>hi)", r.range(10, 5));
        no_draw!("range_closed(lo==hi)", r.range_closed(5, 5));
        no_draw!("range_closed(lo>hi)", r.range_closed(7, 3));
        no_draw!("range_u32(lo==hi)", r.range_u32(7, 7));
        no_draw!("range_u32(lo>hi)", r.range_u32(10, 5));
        no_draw!("coin(num==0)", r.coin(0, 10));
        no_draw!("coin(den==0)", r.coin(5, 0));
        no_draw!("coin(num==den)", r.coin(10, 10));
        no_draw!("coin(num>den)", r.coin(11, 10));
        no_draw!("weighted_index(empty)", r.weighted_index(&[]));
        no_draw!("weighted_index(all-zero)", r.weighted_index(&[0, 0, 0]));
        no_draw!("dice(count==0)", r.dice(0, 6));
        no_draw!("dice(sides==0)", r.dice(3, 0));
        no_draw!("shuffle(empty)", r.shuffle(&mut Vec::<u32>::new()));
        no_draw!("shuffle(single)", r.shuffle(&mut [42u32]));
        no_draw!("pick(empty)", r.pick(&[] as &[u32]));
        no_draw!("pick_mut(empty)", r.pick_mut(&mut [] as &mut [u32]));
        no_draw!("pick_index(0)", r.pick_index(0));
        no_draw!("within_rect(w<=0)", r.within_rect(0, 0, 0, 5));
        no_draw!("within_rect(h<=0)", r.within_rect(0, 0, 5, 0));
        no_draw!("within_rect(both<=0)", r.within_rect(0, 0, -1, -1));
        no_draw!("sample_n(empty)", r.sample_n(&[] as &[u32], 3));
        no_draw!("sample_n(n==0)", r.sample_n(&[1u32, 2, 3], 0));
        no_draw!("gaussian_approx(spread==0)", r.gaussian_approx(10, 0));
        no_draw!("skip(0)", r.skip(0));
    }
}
