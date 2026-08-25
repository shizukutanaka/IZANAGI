//! Xoshiro256++ — a longer-period, higher-quality deterministic PRNG.
//!
//! [`SplitMix64`] is the kit's default generator and
//! remains so: it is tiny, fast, and ideal for seeding and stream-splitting.
//! But its state is a single 64-bit word, so its period is 2⁶⁴ and it fails
//! some statistical tests on very long sequences (it is a pure additive
//! recurrence with an output mix). A game that draws billions of values —
//! large-scale procedural generation, long-running simulations, Monte-Carlo
//! balancing — benefits from a generator with a longer period and stronger
//! equidistribution. [`Xoshiro256pp`] is that alternative: the xoshiro256++
//! generator of Blackman & Vigna (2018, arXiv:1805.01407), with a 2²⁵⁶−1
//! period and excellent statistical quality.
//!
//! This is an **isolated, opt-in addition** — it does not replace
//! `SplitMix64` anywhere, is not wired into any existing draw path, and so
//! changes no pinned replay hash. Seed it idiomatically *from* SplitMix64
//! (the xoshiro authors' own recommended seeding procedure), keeping a single
//! 64-bit seed as the one source of entropy for a replay:
//!
//! ```
//! use izanagi_kit::rng_xoshiro::Xoshiro256pp;
//!
//! let mut rng = Xoshiro256pp::new(0xDEAD_BEEF);
//! let a = rng.next_u64();
//! let b = rng.next_u64();
//! assert_ne!(a, b); // overwhelmingly likely for distinct draws
//!
//! // Same seed, same sequence — the replay guarantee.
//! let mut rng2 = Xoshiro256pp::new(0xDEAD_BEEF);
//! assert_eq!(rng2.next_u64(), a);
//! ```
//!
//! For non-overlapping parallel streams, [`jump`](Xoshiro256pp::jump) advances
//! the state by 2¹²⁸ draws — the xoshiro equivalent of
//! [`SplitMix64::split`](crate::rng::SplitMix64::split), guaranteeing two
//! sub-streams cannot overlap within any realistic draw budget.

use crate::rng::SplitMix64;
use crate::world_hash::{DetHash, Fnv1a};

/// The xoshiro256++ generator. `Clone` snapshots the full 256-bit state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Xoshiro256pp {
    s: [u64; 4],
}

#[inline]
fn rotl(x: u64, k: u32) -> u64 {
    x.rotate_left(k)
}

impl Xoshiro256pp {
    /// Seed from a single 64-bit value using SplitMix64 to fill the 256-bit
    /// state — the seeding procedure the xoshiro authors recommend, so a
    /// whole-state seed still reduces to one replay-persisted `u64`.
    pub fn new(seed: u64) -> Self {
        let mut sm = SplitMix64::new(seed);
        Xoshiro256pp {
            s: [sm.next_u64(), sm.next_u64(), sm.next_u64(), sm.next_u64()],
        }
    }

    /// Construct from an explicit 256-bit state. The all-zero state is invalid
    /// for xoshiro (it is a fixed point that only ever emits zero), so it is
    /// remapped to a `new(0)` seeding instead of being accepted — keeping the
    /// constructor total and panic-free.
    pub fn from_state(state: [u64; 4]) -> Self {
        if state == [0, 0, 0, 0] {
            return Self::new(0);
        }
        Xoshiro256pp { s: state }
    }

    /// The current 256-bit state — fold into a world hash to detect stream
    /// divergence, or serialise for an exact save/restore.
    #[inline]
    pub fn state(&self) -> [u64; 4] {
        self.s
    }

    /// Advance and return the next 64-bit value (the `++` scrambler:
    /// `rotl(s0 + s3, 23) + s0`, then the xoshiro256 linear state update).
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let result = rotl(self.s[0].wrapping_add(self.s[3]), 23).wrapping_add(self.s[0]);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = rotl(self.s[3], 45);
        result
    }

    /// Upper 32 bits of the next 64-bit output (the high bits are the
    /// best-quality bits of a `++`-scrambled generator).
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Advance the state by 2¹²⁸ draws in one step (the published xoshiro jump
    /// polynomial). Two generators seeded identically, one then jumped, yield
    /// sequences that cannot overlap within 2¹²⁸ draws — the idiomatic way to
    /// hand each parallel actor / subsystem its own non-overlapping stream.
    pub fn jump(&mut self) {
        const JUMP: [u64; 4] = [
            0x180e_c6d3_3cfd_0aba,
            0xd5a6_1266_f0c9_392c,
            0xa958_2618_e03f_c9aa,
            0x39ab_dc45_29b1_661c,
        ];
        let mut s0 = 0u64;
        let mut s1 = 0u64;
        let mut s2 = 0u64;
        let mut s3 = 0u64;
        for &jump_word in JUMP.iter() {
            for b in 0..64 {
                if jump_word & (1u64 << b) != 0 {
                    s0 ^= self.s[0];
                    s1 ^= self.s[1];
                    s2 ^= self.s[2];
                    s3 ^= self.s[3];
                }
                self.next_u64();
            }
        }
        self.s = [s0, s1, s2, s3];
    }

    /// Return a clone of this generator jumped 2¹²⁸ draws ahead, leaving `self`
    /// untouched — a non-mutating [`jump`](Self::jump) for handing out a fresh
    /// independent stream while continuing to draw from the original.
    pub fn jumped(&self) -> Xoshiro256pp {
        let mut next = self.clone();
        next.jump();
        next
    }
}

impl DetHash for Xoshiro256pp {
    /// Folds the full 256-bit state so a divergence in this stream surfaces in
    /// the world hash, exactly like `SplitMix64`'s single-word fold.
    fn det_hash(&self, hasher: &mut Fnv1a) {
        for word in self.s {
            hasher.write_u64(word);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_hash::hash_state;

    #[test]
    fn test_next_matches_hand_computed_reference_vector() {
        // State [1,0,0,0]: result = rotl(s0+s3, 23) + s0 = rotl(1,23) + 1
        //                         = (1 << 23) + 1 = 8388608 + 1 = 8388609.
        // Pins the exact `++` scrambler math independently of the seeding path.
        let mut r = Xoshiro256pp::from_state([1, 0, 0, 0]);
        assert_eq!(r.next_u64(), 8_388_609);
    }

    #[test]
    fn test_state_transition_matches_hand_computed() {
        // From [1,0,0,0] the published update yields [1,1,1,0] (t = 0, so the
        // only changes are the xor-cascade; s3 rotl(0,45) stays 0). Verifying
        // the state — not just the output — pins the update permutation too.
        let mut r = Xoshiro256pp::from_state([1, 0, 0, 0]);
        let _ = r.next_u64();
        assert_eq!(r.state(), [1, 1, 1, 0]);
    }

    #[test]
    fn test_same_seed_identical_sequence() {
        let mut a = Xoshiro256pp::new(0x1234_5678);
        let mut b = Xoshiro256pp::new(0x1234_5678);
        for _ in 0..64 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn test_different_seed_diverges() {
        let mut a = Xoshiro256pp::new(1);
        let mut b = Xoshiro256pp::new(2);
        // Extremely unlikely to match on the very first draw.
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn test_new_seeds_state_from_splitmix() {
        // The seeding contract: state[i] is the i-th SplitMix64(seed) output.
        let mut sm = SplitMix64::new(99);
        let expected = [sm.next_u64(), sm.next_u64(), sm.next_u64(), sm.next_u64()];
        assert_eq!(Xoshiro256pp::new(99).state(), expected);
    }

    #[test]
    fn test_from_state_all_zero_is_remapped_not_stuck() {
        // All-zero would be a fixed point emitting only 0; from_state must
        // remap it so the generator still produces varied output.
        let mut r = Xoshiro256pp::from_state([0, 0, 0, 0]);
        let a = r.next_u64();
        let b = r.next_u64();
        assert!(
            a != 0 || b != 0,
            "all-zero state must be remapped, not accepted"
        );
        assert_eq!(Xoshiro256pp::from_state([0, 0, 0, 0]), Xoshiro256pp::new(0));
    }

    #[test]
    fn test_from_state_nonzero_is_preserved() {
        let st = [9u64, 8, 7, 6];
        assert_eq!(Xoshiro256pp::from_state(st).state(), st);
    }

    #[test]
    fn test_next_u32_is_high_bits() {
        let mut a = Xoshiro256pp::new(7);
        let mut b = Xoshiro256pp::new(7);
        let full = a.next_u64();
        assert_eq!(b.next_u32(), (full >> 32) as u32);
    }

    #[test]
    fn test_jump_produces_a_different_stream() {
        let mut base = Xoshiro256pp::new(42);
        let mut jumped = base.clone();
        jumped.jump();
        // The jumped stream's state differs, and its output stream differs from
        // continuing the base stream.
        assert_ne!(base.state(), jumped.state());
        assert_ne!(base.next_u64(), jumped.next_u64());
    }

    #[test]
    fn test_jump_is_deterministic() {
        let mut a = Xoshiro256pp::new(123);
        let mut b = Xoshiro256pp::new(123);
        a.jump();
        b.jump();
        assert_eq!(a.state(), b.state());
        assert_eq!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn test_jumped_does_not_mutate_self() {
        let base = Xoshiro256pp::new(555);
        let state_before = base.state();
        let j = base.jumped();
        assert_eq!(
            base.state(),
            state_before,
            "jumped() must not mutate the original"
        );
        // And it equals an explicit mutating jump.
        let mut expected = base.clone();
        expected.jump();
        assert_eq!(j.state(), expected.state());
    }

    #[test]
    fn test_det_hash_reflects_full_state() {
        let a = Xoshiro256pp::new(1);
        let b = Xoshiro256pp::new(1);
        assert_eq!(hash_state(&a), hash_state(&b));
        let mut c = Xoshiro256pp::new(1);
        c.next_u64();
        assert_ne!(
            hash_state(&a),
            hash_state(&c),
            "advancing must change the hash"
        );
    }

    #[test]
    fn test_no_short_cycle_over_a_window() {
        // Not a period proof, but catches gross implementation errors: 10k
        // consecutive draws from a fixed seed must not immediately repeat the
        // opening value in a tight cycle.
        let mut r = Xoshiro256pp::new(0xABCD_EF01);
        let first = r.next_u64();
        let mut repeats = 0;
        for _ in 0..10_000 {
            if r.next_u64() == first {
                repeats += 1;
            }
        }
        // A single coincidental match is statistically fine; a short cycle
        // would produce many. Expect ~0.
        assert!(
            repeats <= 1,
            "suspicious repetition of the opening value: {repeats}"
        );
    }
}
