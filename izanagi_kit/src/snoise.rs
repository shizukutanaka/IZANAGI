//! Gustavson simplex noise — 2-D simplex-gradient noise on `Fixed`.
//!
//! Distinct from [`crate::gnoise`] (Perlin) and [`crate::simplex`] (the
//! *linear-programming* simplex method — same name, different field).
//! Simplex noise skews space into triangular simplices (3 corners in 2-D
//! instead of Perlin's 4), drops each corner's quartic-falloff
//! contribution `(½−r²)⁴·(g·r)`, and rescales by ≈70.
//!
//! Stefan Gustavson's constants: skew `F = (√3−1)/2`, unskew
//! `G = (3−√3)/6`, 8 gradients, kernel support `r² < ½`.
//!
//! ```
//! use izanagi_kit::{fixed::Fixed, snoise::simplex2};
//!
//! // Smooth deterministic field; reproducible across platforms.
//! let a = simplex2(Fixed::from_ratio(3, 10), Fixed::from_ratio(7, 10), 1);
//! let b = simplex2(Fixed::from_ratio(3, 10), Fixed::from_ratio(7, 10), 1);
//! assert_eq!(a, b);
//! ```

use crate::fixed::Fixed;

/// Skew factor F₂ = (√3−1)/2 ≈ 0.366025 → raw 23993.
const F2: i32 = 23993;
/// Unskew factor G₂ = (3−√3)/6 ≈ 0.211325 → raw 13841.
const G2: i32 = 13841;
/// Half in Q16.16.
const HALF: i32 = 32768;
/// Gustavson's output rescale ×70 (raw 4587520).
const SCALE: i32 = 70 * 65536;

/// Corner hash — SplitMix64 avalanche of (i, j, seed); identical
/// construction to [`crate::gnoise`]'s so the two fields can be seeded
/// consistently.
fn corner_hash(i: i64, j: i64, seed: u64) -> u64 {
    let mut z = (i as u64)
        .wrapping_mul(0x9E3779B97F4A7C15)
        .wrapping_add((j as u64).wrapping_mul(0xC2B2AE3D27D4EB4F))
        .wrapping_add(seed);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// One of Gustavson's 8 gradients (axis or diagonal unit dirs).
fn gradient(h: u64) -> (i32, i32) {
    const ONE: i32 = 65536;
    match h & 7 {
        0 => (ONE, 0),
        1 => (-ONE, 0),
        2 => (0, ONE),
        3 => (0, -ONE),
        4 => (ONE, ONE),
        5 => (-ONE, ONE),
        6 => (ONE, -ONE),
        _ => (-ONE, -ONE),
    }
}

/// `(½ − dx² − dy²)⁴ · (g · d)` in raw Q16.16, or 0 outside the kernel.
/// i64 intermediates keep the fourth power inside range.
#[inline]
fn contribution(dx: i32, dy: i32, gx: i32, gy: i32) -> i32 {
    let t = (HALF as i64) - ((dx as i64 * dx as i64 + dy as i64 * dy as i64) >> 16);
    if t <= 0 {
        return 0;
    }
    let t2 = (t * t) >> 16;
    let t4 = (t2 * t2) >> 16;
    let dot = ((gx as i64) * (dx as i64) + (gy as i64) * (dy as i64)) >> 16;
    ((t4 * dot) >> 16) as i32
}

/// 2-D simplex noise at `(x, y)` for `seed`. Range ≈ `[−1, 1]`.
pub fn simplex2(x: Fixed, y: Fixed, seed: u64) -> Fixed {
    let xr = x.raw() as i64;
    let yr = y.raw() as i64;
    // Skew into simplex lattice space.
    let s = ((xr + yr) * F2 as i64) >> 16;
    let i = (xr + s) >> 16;
    let j = (yr + s) >> 16;
    // Unskew the cell origin back to input space. (i+j) is a count, so
    // t comes out already raw-scaled — no >>16.
    let t = (i + j) * G2 as i64;
    let x0 = (xr - (i << 16) + t) as i32;
    let y0 = (yr - (j << 16) + t) as i32;
    // Second corner of the simplex: (i+1,j) when x0>y0 else (i,j+1).
    let (i1, j1) = if x0 > y0 { (1i64, 0i64) } else { (0, 1) };
    let x1 = (x0 as i64 - (i1 << 16) + G2 as i64) as i32;
    let y1 = (y0 as i64 - (j1 << 16) + G2 as i64) as i32;
    let x2 = (x0 as i64 - 65536 + 2 * G2 as i64) as i32;
    let y2 = (y0 as i64 - 65536 + 2 * G2 as i64) as i32;
    let (g0x, g0y) = gradient(corner_hash(i, j, seed));
    let (g1x, g1y) = gradient(corner_hash(i + i1, j + j1, seed));
    let (g2x, g2y) = gradient(corner_hash(i + 1, j + 1, seed));
    let n = contribution(x0, y0, g0x, g0y)
        + contribution(x1, y1, g1x, g1y)
        + contribution(x2, y2, g2x, g2y);
    Fixed::from_raw(
        ((n as i64 * SCALE as i64) >> 16).clamp(i32::MIN as i64, i32::MAX as i64) as i32,
    )
}

/// Fractal Brownian motion over [`simplex2`]: `octaves` layers, each at
/// 2× frequency and half amplitude, normalized to roughly `[−1, 1]`.
pub fn fbm2(x: Fixed, y: Fixed, octaves: u32, seed: u64) -> Fixed {
    let mut sum: i64 = 0;
    let mut amp: i64 = 32768; // 0.5 in raw Q16.16
    let mut total: i64 = 0;
    let mut fx = x.raw() as i64;
    let mut fy = y.raw() as i64;
    for o in 0..octaves.max(1) {
        let v = simplex2(
            Fixed::from_raw(fx.clamp(i32::MIN as i64, i32::MAX as i64) as i32),
            Fixed::from_raw(fy.clamp(i32::MIN as i64, i32::MAX as i64) as i32),
            seed.wrapping_add(o as u64),
        );
        sum += v.raw() as i64 * amp;
        total += amp;
        amp >>= 1;
        fx *= 2;
        fy *= 2;
    }
    if total == 0 {
        return Fixed::ZERO;
    }
    // sum is in raw² (v.raw·amp), total in raw — the quotient is already
    // raw-scaled.
    Fixed::from_raw((sum / total).clamp(i32::MIN as i64, i32::MAX as i64) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fr(n: i32, d: i32) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    #[test]
    fn deterministic_and_bounded() {
        for xi in -20..20 {
            for yi in -20..20 {
                let x = fr(xi, 7);
                let y = fr(yi, 11);
                let v = simplex2(x, y, 42);
                assert_eq!(v, simplex2(x, y, 42));
                assert!(v.raw().abs() <= 65536 * 2, "{xi},{yi}: {}", v.raw());
            }
        }
    }

    #[test]
    fn corner_determinism_varies_with_seed() {
        let x = fr(1, 3);
        let y = fr(2, 5);
        assert_ne!(simplex2(x, y, 1), simplex2(x, y, 2));
    }

    #[test]
    fn continuity_fine_steps() {
        // Neighbouring samples 1/64 apart differ by a small amount —
        // the quartic kernel is C¹ across simplex edges.
        let mut prev = simplex2(Fixed::ZERO, fr(3, 10), 7);
        let mut maxd = 0i64;
        for k in 1..256 {
            let v = simplex2(fr(k, 64), fr(3, 10), 7);
            let d = (v.raw() as i64 - prev.raw() as i64).abs();
            maxd = maxd.max(d);
            prev = v;
        }
        // |∇n| ≲ 6 for 2-D simplex scaled by 70 → 1/64 step ≤ ~0.1 ≈ 6.5k raw.
        assert!(maxd < 8192, "max step {maxd} raw");
    }

    #[test]
    fn fbm_normalized_and_seed_stable() {
        for k in 0..64 {
            let v = fbm2(fr(k, 8), fr(k, 9), 4, 99);
            assert!(v.raw().abs() <= 131072 + 8, "{k}: {}", v.raw());
        }
        let a = fbm2(fr(1, 2), fr(1, 4), 3, 7);
        assert_eq!(a, fbm2(fr(1, 2), fr(1, 4), 3, 7));
        assert_ne!(a, fbm2(fr(1, 2), fr(1, 4), 3, 8));
    }

    #[test]
    fn simplex_partition_consistency() {
        // The two simplex halves tile without seam: probing a dense line
        // crossing many cells never produces a discontinuity spike.
        let mut prev = simplex2(fr(1, 10), fr(1, 10), 3);
        for k in 1..512 {
            let v = simplex2(fr(1 + k, 10), fr(1, 10), 3);
            assert!((v.raw() - prev.raw()).abs() < 40_000, "k={k}");
            prev = v;
        }
    }
}
