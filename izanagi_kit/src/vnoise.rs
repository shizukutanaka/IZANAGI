//! Integer value noise and fBm — smooth pseudo-random fields in
//! [`Fixed`] Q16.16 with no floats and no permutation tables.
//!
//! [`Noise2`] maps every integer lattice point to a seeded hash value in
//! `[0, 1]`; `value` bilinearly interpolates the four corner hashes with
//! Perlin's smootherstep `u²·(3 − 2u)` weighting. `fbm` sums octaves at
//! doubling frequency / halving amplitude and renormalizes to `[0, 1]`.
//! Every result is a pure function of `(seed, coords, octave params)`.
//!
//! ```
//! use izanagi_kit::vnoise::Noise2;
//! use izanagi_kit::fixed::Fixed;
//! let n = Noise2::new(42);
//! let v = n.value(Fixed::from_int(3), Fixed::from_ratio(1, 2));
//! assert!(v >= Fixed::ZERO && v <= Fixed::ONE);
//! ```
//!
//! Reference: Perlin (1985); "Texturing and Modeling" ch. 11 (fBm).

use crate::fixed::Fixed;

/// Lift raw Q16.16 bits into a `Fixed` (the tuple field is private;
/// `from_ratio(r, 65536)` reproduces exactly `r` raw units).
fn raw(r: i32) -> Fixed {
    Fixed::from_ratio(r, 65536)
}

/// A seeded 2-D value-noise field.
pub struct Noise2 {
    seed: u64,
}

/// SplitMix64-style lattice hash → Q16.16 value in `[0, 1]`.
///
/// The whole 64-bit state is folded into the top 16 bits so every input
/// bit influences every output bit; negative lattice indices are folded
/// by bit-mixing, not by sign tricks.
fn lattice(seed: u64, ix: i64, iy: i64) -> Fixed {
    let mut z = seed
        .wrapping_add((ix as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15))
        .wrapping_add((iy as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f));
    z ^= z >> 30;
    z = z.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z ^= z >> 27;
    z = z.wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    raw((z >> 48) as i32)
}

/// Smootherstep weight `u²·(3 − 2u)` for `u ∈ [0, 1]` (Q16.16).
fn smootherstep(u: Fixed) -> Fixed {
    let three = Fixed::from_int(3);
    let two = Fixed::from_int(2);
    u.mul(u).mul(three - two.mul(u))
}

/// Bilinear lerp of four corners by already-smoothed `(u, v)`.
fn bilerp(c00: Fixed, c10: Fixed, c01: Fixed, c11: Fixed, u: Fixed, v: Fixed) -> Fixed {
    let bottom = c00 + (c10 - c00).mul(u);
    let top = c01 + (c11 - c01).mul(u);
    bottom + (top - bottom).mul(v)
}

impl Noise2 {
    /// Field with the given seed.
    pub const fn new(seed: u64) -> Noise2 {
        Noise2 { seed }
    }

    /// The raw lattice value at integer coordinates — what `value`
    /// returns exactly at lattice points.
    pub fn lattice(&self, ix: i64, iy: i64) -> Fixed {
        lattice(self.seed, ix, iy)
    }

    /// Smooth noise at `(x, y)`, in `[0, 1]`.
    ///
    /// Lattice coordinates use arithmetic-shift flooring, so negative
    /// coordinates map onto the correct cell on every platform.
    pub fn value(&self, x: Fixed, y: Fixed) -> Fixed {
        let ix = (x.raw() >> 16) as i64;
        let iy = (y.raw() >> 16) as i64;
        // `& 0xffff` on the two's-complement raw yields the fractional
        // part in [0, 1) even for negative x.
        let fx = raw(x.raw() & 0xffff);
        let fy = raw(y.raw() & 0xffff);
        let u = smootherstep(fx);
        let v = smootherstep(fy);
        bilerp(
            lattice(self.seed, ix, iy),
            lattice(self.seed, ix + 1, iy),
            lattice(self.seed, ix, iy + 1),
            lattice(self.seed, ix + 1, iy + 1),
            u,
            v,
        )
    }

    /// Fractal Brownian motion: `octaves` octaves at `lacunarity ×`
    /// frequency and `gain ×` amplitude per octave, normalized so the
    /// result stays in `[0, 1]` for `gain ≤ 1`.
    ///
    /// Frequencies and amplitudes scale by fixed-point `mul`, so
    /// `lacunarity = 2` and `gain = 1/2` give the classic spectrum.
    pub fn fbm(&self, x: Fixed, y: Fixed, octaves: usize, lacunarity: Fixed, gain: Fixed) -> Fixed {
        let mut amp = Fixed::ONE;
        let mut freq = Fixed::ONE;
        let mut sum = Fixed::ZERO;
        let mut norm = Fixed::ZERO;
        for _ in 0..octaves {
            let v = self.value(x.mul(freq), y.mul(freq));
            sum = sum + v.mul(amp);
            norm = norm + amp;
            amp = amp.mul(gain);
            freq = freq.mul(lacunarity);
        }
        if norm == Fixed::ZERO {
            return Fixed::ZERO;
        }
        sum.checked_div(norm).unwrap_or(Fixed::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn lattice_exactness() {
        // At integer coords, value == lattice hash exactly.
        let n = Noise2::new(7);
        for ix in -5..5 {
            for iy in -5..5 {
                let v = n.value(Fixed::from_int(ix), Fixed::from_int(iy));
                assert_eq!(v, n.lattice(ix as i64, iy as i64));
            }
        }
    }

    #[test]
    fn bounded_unit_range() {
        let n = Noise2::new(0xdeed);
        let mut rng = SplitMix64::new(0x1234_5678_9abc_def0);
        for _ in 0..4000 {
            let x = raw((rng.next_u64() % 0x14_0000) as i32 - 0xa_0000);
            let y = raw((rng.next_u64() % 0x14_0000) as i32 - 0xa_0000);
            let v = n.value(x, y);
            assert!(v >= Fixed::ZERO && v <= Fixed::ONE);
            let f = n.fbm(x, y, 4, Fixed::from_int(2), Fixed::from_ratio(1, 2));
            assert!(f >= Fixed::ZERO && f <= Fixed::ONE);
        }
    }

    #[test]
    fn smootherstep_matches_definition() {
        // Cross-check `value` against the transposed bilinear oracle:
        // interpolate along y first, then x. Algebraically identical,
        // different rounding path — agree within a few Q16 quanta.
        let n = Noise2::new(99);
        let mut rng = SplitMix64::new(0xaaaa_bbbb_cccc_dddd);
        for _ in 0..500 {
            let xr = (rng.next_u64() % 0xa_0000) as i32;
            let yr = (rng.next_u64() % 0xa_0000) as i32;
            let x = raw(xr - 0x5_0000);
            let y = raw(yr - 0x5_0000);
            let ix = (x.raw() >> 16) as i64;
            let iy = (y.raw() >> 16) as i64;
            let fx = raw(x.raw() & 0xffff);
            let fy = raw(y.raw() & 0xffff);
            let three = Fixed::from_int(3);
            let u = fx.mul(fx).mul(three - Fixed::from_int(2).mul(fx));
            let v = fy.mul(fy).mul(three - Fixed::from_int(2).mul(fy));
            let left = n.lattice(ix, iy) + (n.lattice(ix, iy + 1) - n.lattice(ix, iy)).mul(v);
            let right =
                n.lattice(ix + 1, iy) + (n.lattice(ix + 1, iy + 1) - n.lattice(ix + 1, iy)).mul(v);
            let want = left + (right - left).mul(u);
            assert!((n.value(x, y) - want).abs().raw() <= 4);
        }
    }

    #[test]
    fn fbm_endpoints() {
        // Single octave ≡ value(); zero octaves ≡ 0.
        let n = Noise2::new(3);
        let x = Fixed::from_ratio(3, 4);
        let y = Fixed::from_ratio(5, 6);
        assert_eq!(
            n.fbm(x, y, 1, Fixed::from_int(2), Fixed::from_ratio(1, 2)),
            n.value(x, y)
        );
        assert_eq!(n.fbm(x, y, 0, Fixed::from_int(2), Fixed::ONE), Fixed::ZERO);
    }

    #[test]
    fn seeds_decorrelate() {
        // Different seeds give different fields almost everywhere.
        let (a, b) = (Noise2::new(1), Noise2::new(2));
        let mut diff = 0;
        for ix in 0..16 {
            for iy in 0..16 {
                if a.lattice(ix, iy) != b.lattice(ix, iy) {
                    diff += 1;
                }
            }
        }
        assert!(diff > 240);
    }
}
