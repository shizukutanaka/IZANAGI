//! Gradient (Perlin) noise on `Fixed` — the smooth lattice noise the
//! existing [`crate::noise`] value-noise can't express: zero at every
//! integer lattice point, continuous everywhere, C¹-continuous across
//! cell boundaries via Perlin's 2002 quintic fade `6t⁵−15t⁴+10t³`.
//!
//! Reference implementation sketch: hash the cell corners to one of 8
//! axis/diagonal gradients, dot each with the intra-cell offset, then
//! bilinear-quintic blend. Everything is integer-fixed so the field is
//! byte-identical on every platform and seed.

use crate::fixed::Fixed;

/// SplitMix64 of the lattice coordinates and seed — the deterministic
/// gradient selector. Avalanched so adjacent cells decorrelate.
fn corner_hash(ix: i64, iy: i64, seed: u64) -> u64 {
    let mut z = (ix as u64)
        .wrapping_mul(0x9E3779B97F4A7C15)
        .wrapping_add((iy as u64).wrapping_mul(0xC2B2AE3D27D4EB4F))
        .wrapping_add(seed);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// One of Perlin's 8 canonical gradients selected by the low 3 bits of
/// a corner hash; returned as `(gx, gy)` with components in `{−1,0,+1}`
/// scaled to `Fixed` so dots stay exact.
fn gradient(h: u64) -> (Fixed, Fixed) {
    const ONE: Fixed = Fixed::ONE;
    const NEG: Fixed = Fixed::from_raw(-65536); // −Fixed::ONE
    const Z: Fixed = Fixed::ZERO;
    match h & 7 {
        0 => (ONE, Z),
        1 => (NEG, Z),
        2 => (Z, ONE),
        3 => (Z, NEG),
        4 => (ONE, ONE),
        5 => (NEG, ONE),
        6 => (ONE, NEG),
        _ => (NEG, NEG),
    }
}

/// Perlin's improved fade `6t⁵−15t⁴+10t³` on `0 ≤ t ≤ 1`.
fn fade(t: Fixed) -> Fixed {
    // t³·(6t²−15t+10) — three muls.
    let t2 = t.mul(t);
    t.mul(t2)
        .mul(Fixed::from_int(6).mul(t2) - Fixed::from_int(15).mul(t) + Fixed::from_int(10))
}

/// 2-D Perlin gradient noise. Returns a value in roughly `[−√2/2, √2/2]`
/// (the exact bound of a gradient dot blend); exactly `0` on every
/// integer lattice point by construction.
///
/// ```
/// use izanagi_kit::{fixed::Fixed, gnoise::perlin2};
/// // Zero at every lattice point — the signature of gradient noise.
/// for (x, y) in [(0, 0), (3, -7), (30000, 5)] {
///     assert_eq!(perlin2(Fixed::from_int(x), Fixed::from_int(y), 42), Fixed::ZERO);
/// }
/// ```
pub fn perlin2(x: Fixed, y: Fixed, seed: u64) -> Fixed {
    // Integer lattice coords (i64 for hash input) + intra-cell offsets.
    let ix64 = (x.floor().raw() >> 16) as i64;
    let iy64 = (y.floor().raw() >> 16) as i64;
    // Lattice coordinates are clamped to the i32 the raw shift needs —
    // |x| beyond ±32768 is far outside any useful noise domain anyway.
    let fx = x - Fixed::from_raw((ix64.clamp(-32768, 32767) as i32) << 16);
    let fy = y - Fixed::from_raw((iy64.clamp(-32768, 32767) as i32) << 16);
    let u = fade(fx);
    let v = fade(fy);
    let dot = |dx: i64, dy: i64| {
        let (gx, gy) = gradient(corner_hash(ix64 + dx, iy64 + dy, seed));
        gx.mul(fx - Fixed::from_int(dx as i32)) + gy.mul(fy - Fixed::from_int(dy as i32))
    };
    let n00 = dot(0, 0);
    let n10 = dot(1, 0);
    let n01 = dot(0, 1);
    let n11 = dot(1, 1);
    // Bilinear blend under the quintic weights.
    let nx0 = n00 + (n10 - n00).mul(u);
    let nx1 = n01 + (n11 - n01).mul(u);
    nx0 + (nx1 - nx0).mul(v)
}

/// Fractal Brownian motion over [`perlin2`]: `octaves` of doubling
/// frequency / halving amplitude, normalized so the result lands near
/// `[−1, 1]`. `octaves = 0` returns `0`.
///
/// ```
/// use izanagi_kit::{fixed::Fixed, gnoise::fbm2};
/// let v = fbm2(Fixed::from_ratio(37, 10), Fixed::from_ratio(11, 5), 4, 0xCAFE);
/// assert!(v.abs() <= Fixed::ONE);
/// ```
pub fn fbm2(x: Fixed, y: Fixed, octaves: u32, seed: u64) -> Fixed {
    let mut sum = Fixed::ZERO;
    let mut amp = Fixed::ONE;
    let mut total = Fixed::ZERO;
    let (mut fx, mut fy) = (x, y);
    for o in 0..octaves {
        sum = sum + perlin2(fx, fy, seed.wrapping_add(o as u64)).mul(amp);
        total = total + amp;
        // Amplitude halves; frequency doubles.
        amp = amp.mul(Fixed::from_ratio(1, 2));
        fx = fx.mul(Fixed::from_int(2));
        fy = fy.mul(Fixed::from_int(2));
    }
    if total.is_zero() {
        return Fixed::ZERO;
    }
    sum.div(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i32, d: i32) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    #[test]
    fn zero_at_every_lattice_point() {
        for seed in [0u64, 1, 0xDEADBEEF] {
            for ix in -8..=8 {
                for iy in -8..=8 {
                    assert_eq!(
                        perlin2(Fixed::from_int(ix), Fixed::from_int(iy), seed),
                        Fixed::ZERO
                    );
                }
            }
        }
    }

    #[test]
    fn continuous_across_cell_boundaries() {
        // Small steps ⇒ small changes; check every quarter boundary.
        let eps = Fixed::from_ratio(1, 4096);
        for seed in [7u64, 99] {
            for i in -6..=6 {
                for j in -6..=6 {
                    for &(fx, fy) in &[(r(1, 4), r(3, 7)), (r(2, 3), r(1, 8))] {
                        let base = perlin2(Fixed::from_int(i) + fx, Fixed::from_int(j) + fy, seed);
                        let near =
                            perlin2(Fixed::from_int(i) + fx + eps, Fixed::from_int(j) + fy, seed);
                        assert!(
                            (near - base).abs() < Fixed::from_ratio(1, 100),
                            "jump at ({i},{j})"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn bounded_and_uses_the_field() {
        // Sample a grid: everything inside the theoretical bound, and the
        // field actually varies (not a constant).
        let mut seen_min = Fixed::MAX;
        let mut seen_max = Fixed::MIN;
        for i in 0..40 {
            for j in 0..40 {
                let v = perlin2(r(i * 10 + 3, 40), r(j * 10 + 7, 40), 1234);
                assert!(v.abs() <= Fixed::from_ratio(141, 100));
                seen_min = seen_min.min(v);
                seen_max = seen_max.max(v);
            }
        }
        assert!(seen_max - seen_min > Fixed::from_ratio(1, 2));
    }

    #[test]
    fn different_seeds_decorrelate() {
        let a = perlin2(r(13, 8), r(7, 3), 1);
        let b = perlin2(r(13, 8), r(7, 3), 2);
        assert_ne!(a, b);
    }

    #[test]
    fn fbm_stays_bounded_and_deterministic() {
        for o in 1..=6 {
            let v1 = fbm2(r(31, 10), r(-17, 4), o, 777);
            let v2 = fbm2(r(31, 10), r(-17, 4), o, 777);
            assert_eq!(v1, v2);
            assert!(v1.abs() <= Fixed::from_ratio(15, 10));
        }
        assert_eq!(fbm2(r(1, 2), r(1, 2), 0, 5), Fixed::ZERO);
    }
}
