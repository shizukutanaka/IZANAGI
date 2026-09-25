//! PCG32 — O'Neill's permuted congruential generator. One 64-bit
//! LCG state, one odd 64-bit increment selecting one of 2⁶³
//! independent streams, and an `xorshift → rotate` output
//! permutation — the reference PRNG family for "more bits of
//! quality per state bit than an LCG" without the 128-byte state of
//! xoshiro-class generators. Streams are independent: two `Pcg`s
//! with different `seq` never correlate — the deterministic cousin
//! of `SplitMix64`'s split().
//!
//! ```
//! use izanagi_kit::pcg::Pcg;
//!
//! let mut a = Pcg::new(42);
//! let mut c = Pcg::new(42);
//! assert_eq!(a.next(), c.next()); // same seed + stream → replay-identical
//! let mut b = Pcg::new_stream(42, 7);
//! assert_ne!(a.next(), b.next()); // different stream, same seed
//! ```

/// A PCG32 generator. `state` is the LCG register, `inc` the (always
/// odd) stream increment.
pub struct Pcg {
    state: u64,
    inc: u64,
}

impl Pcg {
    /// Default stream (`seq = 54` — the reference implementation's
    /// default sequence).
    pub fn new(seed: u64) -> Self {
        Self::new_stream(seed, 54)
    }

    /// Generator on stream `seq` — `2⁶³` independent streams per
    /// seed. The reference seeding dance: step once from 0, add the
    /// seed, step again, so every seed gets the same warmup.
    pub fn new_stream(seed: u64, seq: u64) -> Self {
        let mut g = Pcg {
            state: 0,
            inc: (seq << 1) | 1,
        };
        g.next();
        g.state = g.state.wrapping_add(seed);
        g.next();
        g
    }

    /// Next 32 bits. Output is taken from the *old* state before the
    /// LCG step (the reference ordering).
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> u32 {
        let old = self.state;
        self.state = old
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Next value in `[0, bound)` — multiply-shift (Lemire) mapping:
    /// `u32×bound → high 32 bits`, bias below 2⁻³² per draw — the
    /// uniform default everywhere in the kit.
    pub fn next_bounded(&mut self, bound: u32) -> u32 {
        ((self.next() as u64 * bound as u64) >> 32) as u32
    }

    /// Next `u64` from two draws.
    pub fn next_u64(&mut self) -> u64 {
        ((self.next() as u64) << 32) | self.next() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_ordering_is_pinned() {
        // Reference construction order (draw, add seed, draw) is what
        // the test pins: any reordering shows up as a different first
        // word. Values verified against an independent re-derivation.
        let mut g = Pcg::new_stream(42, 54);
        assert_eq!(g.next(), 0xa15c_02b7);
        assert_eq!(g.next(), 0x7b47_f409);
        assert_eq!(g.next(), 0xba1d_3330);
    }

    #[test]
    fn streams_are_independent() {
        // Same seed, different seq: streams must not just differ on
        // one draw — collect several.
        let (mut a, mut b) = (Pcg::new_stream(9, 1), Pcg::new_stream(9, 2));
        let same = (0..8).filter(|_| a.next() == b.next()).count();
        assert_eq!(same, 0);
    }

    #[test]
    fn bounded_is_uniform_and_in_range() {
        let mut g = Pcg::new(7);
        let mut seen = [0u32; 6];
        for _ in 0..60_000 {
            let v = g.next_bounded(6);
            assert!(v < 6);
            seen[v as usize] += 1;
        }
        for &c in &seen {
            assert!(c > 9_000 && c < 11_000, "{seen:?}");
        }
    }

    #[test]
    fn deterministic_twice() {
        let (mut a, mut b) = (Pcg::new(123), Pcg::new(123));
        for _ in 0..64 {
            assert_eq!(a.next(), b.next());
            assert_eq!(a.next_u64(), b.next_u64());
            assert_eq!(a.next_bounded(997), b.next_bounded(997));
        }
    }
}
