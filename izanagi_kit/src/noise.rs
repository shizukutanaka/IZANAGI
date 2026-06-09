//! Deterministic integer noise functions for procedural generation.
//!
//! All functions are pure, float-free, and bit-identical across targets.
//!
//! ## Provided functions
//!
//! - `value_noise_1d(x, seed)` — 1-D value noise: smooth cubic-interpolated
//!   integer noise in `[0, 65535]`.
//! - `value_noise_2d(x, y, seed)` — 2-D value noise: bilinear-interpolated
//!   integer noise in `[0, 65535]`.
//! - `hash_1d(x, seed)` — fast integer hash of one coordinate + seed.
//!   Returns a `u32` in `[0, u32::MAX]`. Use as a raw random-looking value
//!   without smoothing (e.g. for scatter / jitter tables).
//! - `hash_2d(x, y, seed)` — 2-D version of the same.
//!
//! ## Design notes
//!
//! The hash mixes coordinates with a seed using the same integer operations as
//! SplitMix64 (xor-shift + multiply). Value noise interpolates between corner
//! hashes using a cubic Hermite ("smoothstep") polynomial computed in fixed-
//! point Q16.16 so no float is ever touched.
//!
//! The output range `[0, 65535]` is chosen so two values can be multiplied in
//! `u32` without overflow (fits Q16.0 integer arithmetic).

/// Fast integer hash of one coordinate + seed. Output in `[0, u32::MAX]`.
#[inline]
pub fn hash_1d(x: i32, seed: u64) -> u32 {
    let mut h = seed.wrapping_add(x as u64);
    h = h.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    h ^= h >> 30;
    h = h.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94d0_49bb_1331_11eb);
    h ^= h >> 31;
    (h >> 32) as u32
}

/// Fast integer hash of two coordinates + seed. Output in `[0, u32::MAX]`.
#[inline]
pub fn hash_2d(x: i32, y: i32, seed: u64) -> u32 {
    // Fold y into the seed so the two axes are independent.
    let seed2 = seed.wrapping_add(y as u64).wrapping_mul(0x6c62272e07bb0142);
    hash_1d(x, seed2)
}

/// Fast integer hash of three coordinates + seed. Output in `[0, u32::MAX]`.
///
/// Extends the `hash_1d`/`hash_2d` family to 3-D for voxel terrain, layered
/// world generation, and any system that uses a third axis (e.g. dungeon depth
/// or time slice) as part of the hash input.
#[inline]
pub fn hash_3d(x: i32, y: i32, z: i32, seed: u64) -> u32 {
    let seed2 = seed.wrapping_add(z as u64).wrapping_mul(0x9e3779b97f4a7c15);
    hash_2d(x, y, seed2)
}

/// Cubic Hermite smoothstep: `3t² - 2t³` in Q16.16 fixed-point.
/// Input `t` in `[0, 65536]` (Q16.16 where 65536 = 1.0).
/// Output in `[0, 65536]`.
#[inline]
fn smoothstep(t: u32) -> u32 {
    // t_norm in [0, 256] representing t as a fraction of 256.
    // We use 32-bit arithmetic throughout to stay in MSRV range.
    // 3t² - 2t³  where t ∈ [0, 1] encoded as t_q = t * 65536.
    // To avoid 64-bit overflow we work at reduced precision: scale t to [0,256].
    let t256 = t >> 8; // [0, 256], t scaled to 1/256 precision
    let t2 = t256 * t256; // t² * 65536, range [0, 65536]
    let t3 = (t2 * t256) >> 8; // t³ * 65536, range [0, 65536]
                               // 3t² - 2t³ ∈ [0, 1] for t ∈ [0, 1]; both operands already scaled by 65536.
    3 * t2 - 2 * t3
}

/// 1-D value noise: smooth cubic-interpolated noise in `[0, 65535]`.
///
/// `x` is the sample coordinate; `seed` differentiates noise layers.
pub fn value_noise_1d(x: i32, seed: u64) -> u32 {
    let x0 = x;
    let x1 = x.wrapping_add(1);
    // Fractional part: where in [x0, x1] are we? We use the low 16 bits of
    // the raw coordinate cast to u32 as the fractional part in Q16.16.
    // Since x is already an integer, the fractional part is 0 — value noise
    // at integer coordinates returns exactly the corner hash (no interpolation).
    // To get sub-integer samples, callers pass a fixed-point x (upper 16 bits
    // = integer part, lower 16 bits = fractional). We extract those here.
    let xi = x >> 16; // integer coordinate
    let frac = (x as u32) & 0xffff; // fractional part in Q16.16

    let v0 = hash_1d(xi, seed) >> 16; // [0, 65535]
    let v1 = hash_1d(xi.wrapping_add(1), seed) >> 16;
    let _ = (x0, x1); // used conceptually above

    // Smooth interpolation: v0 + (v1 - v0) * smoothstep(frac).
    let t = smoothstep(frac); // [0, 65536]
    let lerped = if v1 >= v0 {
        v0 + (((v1 - v0) * t) >> 16)
    } else {
        v0 - (((v0 - v1) * t) >> 16)
    };
    lerped.min(65535)
}

/// 2-D value noise: bilinear-interpolated noise in `[0, 65535]`.
///
/// `x` and `y` are fixed-point Q16.16 coordinates (upper 16 bits = integer,
/// lower 16 bits = fraction). `seed` differentiates noise layers.
pub fn value_noise_2d(x: i32, y: i32, seed: u64) -> u32 {
    let xi = x >> 16;
    let yi = y >> 16;
    let fx = (x as u32) & 0xffff;
    let fy = (y as u32) & 0xffff;

    let v00 = hash_2d(xi, yi, seed) >> 16;
    let v10 = hash_2d(xi.wrapping_add(1), yi, seed) >> 16;
    let v01 = hash_2d(xi, yi.wrapping_add(1), seed) >> 16;
    let v11 = hash_2d(xi.wrapping_add(1), yi.wrapping_add(1), seed) >> 16;

    let sx = smoothstep(fx);
    let sy = smoothstep(fy);

    // Bilinear interpolation:
    //   top    = lerp(v00, v10, sx)
    //   bottom = lerp(v01, v11, sx)
    //   result = lerp(top, bottom, sy)
    let top = lerp_u32(v00, v10, sx);
    let bottom = lerp_u32(v01, v11, sx);
    lerp_u32(top, bottom, sy).min(65535)
}

#[inline]
fn lerp_u32(a: u32, b: u32, t: u32) -> u32 {
    if b >= a {
        a + (((b - a) * t) >> 16)
    } else {
        a - (((a - b) * t) >> 16)
    }
}

/// Fractional Brownian motion in 2-D: sum `octaves` layers of [`value_noise_2d`],
/// each at double the frequency and half the amplitude of the previous, then
/// renormalise to `[0, 65535]`. This is the standard way to turn flat value
/// noise into natural-looking terrain (mountains, clouds, biome fields).
///
/// `x`/`y` are Q16.16 coordinates (as for [`value_noise_2d`]); each octave uses
/// a distinct seed derived from `seed`. `octaves == 0` returns `0`. Frequency
/// shifts and the amplitude taper are bounded so any octave count is panic-free
/// and deterministic — 1–6 octaves is the useful range.
pub fn fbm_2d(x: i32, y: i32, seed: u64, octaves: u32) -> u32 {
    let mut acc: u64 = 0;
    let mut amplitude: u64 = 65536;
    let mut total_amp: u64 = 0;
    for i in 0..octaves {
        let shift = i.min(30);
        let sx = x.wrapping_shl(shift);
        let sy = y.wrapping_shl(shift);
        let v = value_noise_2d(sx, sy, seed.wrapping_add(i as u64)) as u64;
        acc += v * amplitude;
        total_amp += amplitude * 65535;
        amplitude >>= 1;
        if amplitude == 0 {
            break; // further octaves contribute nothing
        }
    }
    if total_amp == 0 {
        0
    } else {
        ((acc * 65535) / total_amp).min(65535) as u32
    }
}

/// Fractional Brownian motion in 1-D — the [`fbm_2d`] analogue over
/// [`value_noise_1d`]. Useful for height-lines, audio-style ramps, and 1-D
/// terrain silhouettes. Returns `[0, 65535]`; `octaves == 0` returns `0`.
pub fn fbm_1d(x: i32, seed: u64, octaves: u32) -> u32 {
    let mut acc: u64 = 0;
    let mut amplitude: u64 = 65536;
    let mut total_amp: u64 = 0;
    for i in 0..octaves {
        let shift = i.min(30);
        let sx = x.wrapping_shl(shift);
        let v = value_noise_1d(sx, seed.wrapping_add(i as u64)) as u64;
        acc += v * amplitude;
        total_amp += amplitude * 65535;
        amplitude >>= 1;
        if amplitude == 0 {
            break;
        }
    }
    if total_amp == 0 {
        0
    } else {
        ((acc * 65535) / total_amp).min(65535) as u32
    }
}

/// 2-D value noise that tiles seamlessly with period `(period_x, period_y)`.
///
/// `x` and `y` are Q16.16 fixed-point coordinates (upper 16 bits = integer).
/// Corner hashes wrap at the given integer period, so the noise is identical
/// at `(0, y)` and `(period_x, y)` (and equivalently for `y`). This is the
/// standard technique for tileable dungeon-level textures and seamless world-
/// map wrapping. Returns `[0, 65535]`.
///
/// A `period` of `0` is treated as `1` (constant noise) rather than panicking.
pub fn value_noise_2d_wrap(x: i32, y: i32, seed: u64, period_x: i32, period_y: i32) -> u32 {
    let px = period_x.max(1);
    let py = period_y.max(1);
    let xi = x >> 16;
    let yi = y >> 16;
    let fx = (x as u32) & 0xffff;
    let fy = (y as u32) & 0xffff;

    // Positive modulo so negative coordinates wrap cleanly.
    let xi0 = xi.rem_euclid(px);
    let xi1 = (xi0 + 1).rem_euclid(px);
    let yi0 = yi.rem_euclid(py);
    let yi1 = (yi0 + 1).rem_euclid(py);

    let v00 = hash_2d(xi0, yi0, seed) >> 16;
    let v10 = hash_2d(xi1, yi0, seed) >> 16;
    let v01 = hash_2d(xi0, yi1, seed) >> 16;
    let v11 = hash_2d(xi1, yi1, seed) >> 16;

    let sx = smoothstep(fx);
    let sy = smoothstep(fy);
    let top = lerp_u32(v00, v10, sx);
    let bottom = lerp_u32(v01, v11, sx);
    lerp_u32(top, bottom, sy).min(65535)
}

/// 1-D value noise that tiles seamlessly with integer period `period`.
///
/// `x` is a Q16.16 fixed-point coordinate (upper 16 bits = integer part).
/// Corner hashes wrap at `period` via `rem_euclid`, so the noise is identical
/// at `(0)` and `(period)`. A `period` of `0` is treated as `1`. Returns
/// `[0, 65535]`.
pub fn value_noise_1d_wrap(x: i32, seed: u64, period: i32) -> u32 {
    let p = period.max(1);
    let xi = x >> 16;
    let frac = (x as u32) & 0xffff;
    let xi0 = xi.rem_euclid(p);
    let xi1 = (xi0 + 1).rem_euclid(p);
    let v0 = hash_1d(xi0, seed) >> 16;
    let v1 = hash_1d(xi1, seed) >> 16;
    let t = smoothstep(frac);
    lerp_u32(v0, v1, t).min(65535)
}

/// Tileable 1-D FBM — like [`fbm_1d`] but each octave tiles at `period`
/// (doubled per octave so harmonics also tile). Completes the wrap family
/// alongside [`fbm_2d_wrap`]. Returns `[0, 65535]`; `octaves == 0` returns `0`.
pub fn fbm_1d_wrap(x: i32, seed: u64, octaves: u32, period: i32) -> u32 {
    let mut acc: u64 = 0;
    let mut amplitude: u64 = 65536;
    let mut total_amp: u64 = 0;
    for i in 0..octaves {
        let shift = i.min(15);
        let sx = x.wrapping_shl(shift);
        let px = (period << shift).max(1);
        let v = value_noise_1d_wrap(sx, seed.wrapping_add(i as u64), px) as u64;
        acc += v * amplitude;
        total_amp += amplitude * 65535;
        amplitude >>= 1;
        if amplitude == 0 {
            break;
        }
    }
    if total_amp == 0 {
        0
    } else {
        ((acc * 65535) / total_amp).min(65535) as u32
    }
}

/// Tileable 2-D FBM — like [`fbm_2d`] but each octave tiles at `period`
/// (halved per octave so harmonics also tile). Useful for seamlessly wrapping
/// level maps and world textures.
pub fn fbm_2d_wrap(x: i32, y: i32, seed: u64, octaves: u32, period: i32) -> u32 {
    let mut acc: u64 = 0;
    let mut amplitude: u64 = 65536;
    let mut total_amp: u64 = 0;
    for i in 0..octaves {
        let shift = i.min(15);
        let sx = x.wrapping_shl(shift);
        let sy = y.wrapping_shl(shift);
        let px = (period << shift).max(1);
        let v = value_noise_2d_wrap(sx, sy, seed.wrapping_add(i as u64), px, px) as u64;
        acc += v * amplitude;
        total_amp += amplitude * 65535;
        amplitude >>= 1;
        if amplitude == 0 {
            break;
        }
    }
    if total_amp == 0 {
        0
    } else {
        ((acc * 65535) / total_amp).min(65535) as u32
    }
}

/// Map a noise value from the standard `[0, 65535]` output range to the
/// caller-defined integer range `[lo, hi]`. Returns `lo` for `v == 0` and
/// `hi` for `v == 65535`. Returns `lo` immediately when `lo >= hi` (no-op /
/// degenerate range). Result is clamped to `[lo, hi]`.
///
/// Useful for converting raw noise to game values: elevation, temperature,
/// wealth, or any other integer attribute that spans a specific range.
#[inline]
pub fn normalize_noise(v: u32, lo: i32, hi: i32) -> i32 {
    if lo >= hi {
        return lo;
    }
    let range = (hi - lo) as i64;
    lo + (((v as i64) * range) / 65535).min(range) as i32
}

/// Map a raw hash `h` (full `u32` range, e.g. from [`hash_1d`] / [`hash_2d`])
/// into the half-open integer range `[lo, hi)` with an unbiased wide multiply.
/// Returns `lo` when `lo >= hi` (degenerate range). `h == 0` maps to `lo`;
/// `h == u32::MAX` maps to `hi - 1`.
///
/// Unlike [`normalize_noise`] (which scales the smooth `[0, 65535]` noise
/// range, inclusive), this consumes a full-width hash, so it suits per-cell
/// scatter tables: `hash_range(hash_2d(x, y, seed), 0, 6)` rolls a `0..6` value
/// independently at each cell. Deterministic and float-free.
#[inline]
pub fn hash_range(h: u32, lo: i32, hi: i32) -> i32 {
    if lo >= hi {
        return lo;
    }
    let span = (hi as i64 - lo as i64) as u64;
    (lo as i64 + ((h as u64 * span) >> 32) as i64) as i32
}

/// Ridge (absolute-value) 2-D fractional Brownian motion.
///
/// Applies the ridge transform `|2v − 65535|` to each FBM layer *before*
/// accumulation, producing sharp ridgelines and valley-folds instead of
/// smooth rolling terrain. The output is in `[0, 65535]`; `octaves == 0`
/// returns 0. Deterministic and float-free.
///
/// Use this for dramatic mountain chains, clifffaces, and lightning-shaped
/// damage radii where regular FBM would be too gentle.
pub fn ridge_noise_2d(x: i32, y: i32, seed: u64, octaves: u32) -> u32 {
    let mut acc: u64 = 0;
    let mut amplitude: u64 = 65536;
    let mut total_amp: u64 = 0;
    for i in 0..octaves {
        let shift = i.min(30);
        let sx = x.wrapping_shl(shift);
        let sy = y.wrapping_shl(shift);
        let raw = value_noise_2d(sx, sy, seed.wrapping_add(i as u64)) as u64;
        // Ridge transform: absolute distance from midpoint → sharpens peaks.
        let ridged = if raw < 32768 { raw } else { 65535 - raw };
        acc += ridged * amplitude;
        total_amp += 32768 * amplitude; // midpoint is the max of the ridged signal
        amplitude >>= 1;
        if amplitude == 0 {
            break;
        }
    }
    if total_amp == 0 {
        0
    } else {
        ((acc * 65535) / total_amp).min(65535) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- hash_1d / hash_2d ---

    #[test]
    fn test_hash_1d_deterministic() {
        assert_eq!(hash_1d(42, 0), hash_1d(42, 0));
    }

    #[test]
    fn test_hash_1d_different_inputs_differ() {
        assert_ne!(hash_1d(0, 0), hash_1d(1, 0));
        assert_ne!(hash_1d(0, 0), hash_1d(0, 1));
    }

    #[test]
    fn test_hash_2d_deterministic() {
        assert_eq!(hash_2d(3, 7, 99), hash_2d(3, 7, 99));
    }

    #[test]
    fn test_hash_2d_x_y_not_symmetric() {
        // hash_2d(x,y) should differ from hash_2d(y,x) in general.
        assert_ne!(hash_2d(1, 2, 0), hash_2d(2, 1, 0));
    }

    #[test]
    fn test_hash_2d_different_seeds_differ() {
        assert_ne!(hash_2d(5, 5, 0), hash_2d(5, 5, 1));
    }

    // --- smoothstep ---

    #[test]
    fn test_smoothstep_at_zero() {
        assert_eq!(smoothstep(0), 0);
    }

    #[test]
    fn test_smoothstep_at_one() {
        // t=65536 → step = 3*65536^2/256^2 - 2*65536^3/(256^3*256)
        // We just check it's at the maximum (65536/3 * 3 = 65536)
        assert_eq!(smoothstep(65536), 65536);
    }

    #[test]
    fn test_smoothstep_midpoint_is_half() {
        let mid = smoothstep(32768); // 0.5 in Q16.16
                                     // smoothstep(0.5) = 3*(0.5)^2 - 2*(0.5)^3 = 0.75 - 0.25 = 0.5
                                     // Result should be ~32768 ± small rounding error.
        assert!((32000..=33536).contains(&mid), "mid={mid}");
    }

    // --- value_noise_1d ---

    #[test]
    fn test_value_noise_1d_in_range() {
        for x in -10..10 {
            let v = value_noise_1d(x << 16, 42);
            assert!(v <= 65535, "v={v} for x={x}");
        }
    }

    #[test]
    fn test_value_noise_1d_deterministic() {
        let a = value_noise_1d(3 << 16, 7);
        let b = value_noise_1d(3 << 16, 7);
        assert_eq!(a, b);
    }

    #[test]
    fn test_value_noise_1d_different_seeds_differ() {
        let a = value_noise_1d(5 << 16, 0);
        let b = value_noise_1d(5 << 16, 1);
        assert_ne!(a, b);
    }

    #[test]
    fn test_value_noise_1d_integer_coords_match_hash() {
        // At integer coordinates (frac=0), noise equals the corner hash.
        let x = 7i32;
        let seed = 123u64;
        let noise = value_noise_1d(x << 16, seed);
        let expected = hash_1d(x, seed) >> 16;
        assert_eq!(noise, expected);
    }

    // --- value_noise_2d ---

    #[test]
    fn test_value_noise_2d_in_range() {
        for x in -5..5 {
            for y in -5..5 {
                let v = value_noise_2d(x << 16, y << 16, 0);
                assert!(v <= 65535, "v={v} at ({x},{y})");
            }
        }
    }

    #[test]
    fn test_value_noise_2d_deterministic() {
        let a = value_noise_2d(2 << 16, 3 << 16, 55);
        let b = value_noise_2d(2 << 16, 3 << 16, 55);
        assert_eq!(a, b);
    }

    #[test]
    fn test_value_noise_2d_different_positions_vary() {
        let a = value_noise_2d(0, 0, 0);
        let b = value_noise_2d(1 << 16, 0, 0);
        let c = value_noise_2d(0, 1 << 16, 0);
        // Different positions should in general differ (may rarely collide).
        assert!(a != b || b != c, "all three samples identical — suspicious");
    }

    #[test]
    fn test_value_noise_2d_integer_coords_match_hash() {
        let x = 4i32;
        let y = 9i32;
        let seed = 777u64;
        let noise = value_noise_2d(x << 16, y << 16, seed);
        let expected = hash_2d(x, y, seed) >> 16;
        assert_eq!(noise, expected);
    }

    // --- fractional Brownian motion ---

    #[test]
    fn test_fbm_2d_in_range() {
        let seed = 0xF00D;
        for y in 0..40i32 {
            for x in 0..40i32 {
                let v = fbm_2d(x << 13, y << 13, seed, 4);
                assert!(v <= 65535, "fbm_2d out of range: {v}");
            }
        }
    }

    #[test]
    fn test_fbm_zero_octaves_is_zero() {
        assert_eq!(fbm_2d(123 << 16, 45 << 16, 9, 0), 0);
        assert_eq!(fbm_1d(123 << 16, 9, 0), 0);
    }

    #[test]
    fn test_fbm_one_octave_matches_value_noise() {
        // With a single octave, FBM is just the base value noise (same seed
        // offset 0), renormalised by the identity 65535/65535.
        let (x, y, seed) = (3 << 16 | 0x4000, 7 << 16 | 0x8000, 42u64);
        assert_eq!(fbm_2d(x, y, seed, 1), value_noise_2d(x, y, seed));
        assert_eq!(fbm_1d(x, seed, 1), value_noise_1d(x, seed));
    }

    #[test]
    fn test_fbm_2d_deterministic() {
        let a = fbm_2d(11 << 14, 5 << 14, 1, 5);
        let b = fbm_2d(11 << 14, 5 << 14, 1, 5);
        assert_eq!(a, b);
    }

    #[test]
    fn test_fbm_many_octaves_no_panic() {
        // Large octave counts must not panic (shift saturation + amplitude break).
        let v = fbm_2d(1 << 16, 1 << 16, 7, 40);
        assert!(v <= 65535);
    }

    #[test]
    fn test_fbm_1d_in_range() {
        for x in 0..200i32 {
            let v = fbm_1d(x << 12, 3, 4);
            assert!(v <= 65535);
        }
    }

    // ── tileable noise ───────────────────────────────────────────────────────

    #[test]
    fn test_value_noise_2d_wrap_in_range() {
        for x in 0..20i32 {
            for y in 0..20i32 {
                let v = value_noise_2d_wrap(x << 14, y << 14, 42, 8, 8);
                assert!(v <= 65535, "out of range at ({x},{y}): {v}");
            }
        }
    }

    #[test]
    fn test_value_noise_2d_wrap_tiles_x() {
        // Wrapping: noise at x=0 should equal noise at x=period.
        let period = 8i32;
        for y in 0..8i32 {
            let frac = 0i32; // integer coordinates
            let v0 = value_noise_2d_wrap(0, y << 16, 99, period, period);
            let vp = value_noise_2d_wrap(period << 16, y << 16, 99, period, period);
            assert_eq!(
                v0, vp,
                "wrap failed at y={y}: noise(0,y)={v0} != noise(period,y)={vp}",
            );
            let _ = frac;
        }
    }

    #[test]
    fn test_value_noise_2d_wrap_tiles_y() {
        let period = 6i32;
        for x in 0..6i32 {
            let v0 = value_noise_2d_wrap(x << 16, 0, 77, period, period);
            let vp = value_noise_2d_wrap(x << 16, period << 16, 77, period, period);
            assert_eq!(v0, vp, "y-wrap failed at x={x}");
        }
    }

    #[test]
    fn test_value_noise_2d_wrap_deterministic() {
        let v1 = value_noise_2d_wrap(3 << 16, 4 << 16, 5, 8, 8);
        let v2 = value_noise_2d_wrap(3 << 16, 4 << 16, 5, 8, 8);
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_fbm_2d_wrap_in_range() {
        for i in 0..20i32 {
            let v = fbm_2d_wrap(i << 14, i << 13, 13, 3, 16);
            assert!(v <= 65535);
        }
    }

    #[test]
    fn test_fbm_2d_wrap_deterministic() {
        let a = fbm_2d_wrap(1 << 16, 2 << 16, 42, 4, 8);
        let b = fbm_2d_wrap(1 << 16, 2 << 16, 42, 4, 8);
        assert_eq!(a, b);
    }

    // ── 1-D tileable noise ───────────────────────────────────────────────────

    #[test]
    fn test_value_noise_1d_wrap_in_range() {
        for x in 0..20i32 {
            let v = value_noise_1d_wrap(x << 14, 7, 8);
            assert!(v <= 65535, "out of range at {x}: {v}");
        }
    }

    #[test]
    fn test_value_noise_1d_wrap_tiles() {
        let period = 8i32;
        let v0 = value_noise_1d_wrap(0, 42, period);
        let vp = value_noise_1d_wrap(period << 16, 42, period);
        assert_eq!(v0, vp, "wrap: noise(0) != noise(period)");
    }

    #[test]
    fn test_value_noise_1d_wrap_deterministic() {
        let a = value_noise_1d_wrap(3 << 16, 5, 8);
        let b = value_noise_1d_wrap(3 << 16, 5, 8);
        assert_eq!(a, b);
    }

    #[test]
    fn test_fbm_1d_wrap_in_range() {
        for x in 0..30i32 {
            let v = fbm_1d_wrap(x << 13, 11, 4, 16);
            assert!(v <= 65535, "fbm_1d_wrap out of range at {x}: {v}");
        }
    }

    #[test]
    fn test_fbm_1d_wrap_zero_octaves_is_zero() {
        assert_eq!(fbm_1d_wrap(1 << 16, 99, 0, 8), 0);
    }

    #[test]
    fn test_fbm_1d_wrap_deterministic() {
        let a = fbm_1d_wrap(5 << 14, 17, 3, 8);
        let b = fbm_1d_wrap(5 << 14, 17, 3, 8);
        assert_eq!(a, b);
    }

    #[test]
    fn test_fbm_1d_wrap_one_octave_matches_value_noise_wrap() {
        let (x, seed, period) = (3 << 16 | 0x4000, 42u64, 8i32);
        assert_eq!(
            fbm_1d_wrap(x, seed, 1, period),
            value_noise_1d_wrap(x, seed, period)
        );
    }

    // --- normalize_noise ---

    #[test]
    fn test_normalize_noise_min_maps_to_lo() {
        assert_eq!(normalize_noise(0, -100, 100), -100);
    }

    #[test]
    fn test_normalize_noise_max_maps_to_hi() {
        assert_eq!(normalize_noise(65535, -100, 100), 100);
    }

    #[test]
    fn test_normalize_noise_midpoint() {
        // 32767/65535 ≈ 0.4999923...; maps to approximately 0 in [-100, 100]
        let v = normalize_noise(32767, -100, 100);
        assert!((-1..=0).contains(&v), "midpoint should be near 0, got {v}");
    }

    #[test]
    fn test_normalize_noise_degenerate_range_returns_lo() {
        assert_eq!(normalize_noise(32000, 5, 5), 5);
        assert_eq!(normalize_noise(32000, 10, 5), 10); // lo > hi
    }

    #[test]
    fn test_normalize_noise_positive_range() {
        // Any v in [0, 65535] should map to [0, 255].
        assert_eq!(normalize_noise(0, 0, 255), 0);
        assert_eq!(normalize_noise(65535, 0, 255), 255);
        let mid = normalize_noise(32768, 0, 255);
        assert!((127..=128).contains(&mid));
    }

    #[test]
    fn test_ridge_noise_2d_deterministic() {
        assert_eq!(ridge_noise_2d(5, 10, 42, 3), ridge_noise_2d(5, 10, 42, 3));
    }

    #[test]
    fn test_ridge_noise_2d_in_range() {
        for y in 0..8i32 {
            for x in 0..8i32 {
                let v = ridge_noise_2d(x, y, 0, 4);
                assert!(v <= 65535, "ridge_noise_2d out of range at ({x},{y}): {v}");
            }
        }
    }

    #[test]
    fn test_ridge_noise_2d_zero_octaves_is_zero() {
        assert_eq!(ridge_noise_2d(1, 2, 99, 0), 0);
    }

    #[test]
    fn test_ridge_noise_2d_differs_from_fbm() {
        // Ridge and standard FBM should differ for the same inputs.
        let ridge = ridge_noise_2d(3, 7, 1234, 3);
        let standard = fbm_2d(3, 7, 1234, 3);
        assert_ne!(ridge, standard, "ridge and fbm should differ");
    }

    // --- hash_range ---

    #[test]
    fn test_hash_range_within_bounds() {
        for x in 0..200i32 {
            let v = hash_range(hash_1d(x, 42), 0, 6);
            assert!((0..6).contains(&v), "v={v} out of [0,6)");
        }
    }

    #[test]
    fn test_hash_range_endpoints() {
        // h=0 → lo; h=u32::MAX → hi-1.
        assert_eq!(hash_range(0, 10, 20), 10);
        assert_eq!(hash_range(u32::MAX, 10, 20), 19);
    }

    #[test]
    fn test_hash_range_degenerate_returns_lo() {
        assert_eq!(hash_range(12345, 5, 5), 5);
        assert_eq!(hash_range(12345, 10, 3), 10); // lo > hi
    }

    #[test]
    fn test_hash_range_deterministic() {
        let h = hash_2d(3, 4, 99);
        assert_eq!(hash_range(h, -50, 50), hash_range(h, -50, 50));
    }

    #[test]
    fn test_hash_3d_deterministic() {
        assert_eq!(hash_3d(1, 2, 3, 77), hash_3d(1, 2, 3, 77));
    }

    #[test]
    fn test_hash_3d_z_changes_output() {
        let a = hash_3d(1, 2, 0, 77);
        let b = hash_3d(1, 2, 1, 77);
        assert_ne!(a, b, "different z must produce different hash");
    }

    #[test]
    fn test_hash_3d_differs_from_hash_2d() {
        // hash_3d and hash_2d for the same (x, y) should differ (different seed path).
        let h2 = hash_2d(5, 7, 42);
        let h3 = hash_3d(5, 7, 0, 42);
        assert_ne!(h2, h3);
    }
}
