//! Space-filling-curve codes — Morton (z-order) and Hilbert — for
//! deterministic spatial indexing and locality-preserving ordering.
//!
//! A space-filling curve maps `(x, y)` to one integer so that points close
//! on the curve are close in space. Two uses in a deterministic sim:
//!
//! * **Spatial sort**: `spatial_sort(points)` orders a point set so nearby
//!   points sit near each other — cache-friendly sweeps over scattered
//!   sites (pairs with [`crate::mapgen::poisson_disc`]) and a stable,
//!   content-defined ordering for world hashing.
//! * **Spatial keys**: Morton codes are the index quadtree cells are
//!   addressed by; `hilbert` trades a little decode speed for stronger
//!   locality (consecutive curve indices are always adjacent cells).
//!
//! All integer, all deterministic. References: Morton (1966); Skilling,
//! *Programming the Hilbert curve* (2004); the redblobgames and Wikipedia
//! expositions.
//!
//! ```
//! use izanagi_kit::zorder::{morton_encode, morton_decode, spatial_sort};
//! let code = morton_encode(5, 9);
//! assert_eq!(morton_decode(code), (5, 9));
//! let mut pts = vec![(9, 9), (0, 0), (9, 8), (0, 1)];
//! spatial_sort(&mut pts); // nearby points cluster together
//! ```

/// Interleave the 16 low bits of `x` and `y` into a 32-bit Morton code.
/// Bit pattern: `y15 x15 y14 x14 … y0 x0` — y occupies the odd bit
/// positions. Coordinates are masked to 16 bits; higher bits are dropped
/// (document your range or use [`morton_encode64`] for the full width).
#[inline]
pub fn morton_encode(x: u32, y: u32) -> u32 {
    part1by1(x) | (part1by1(y) << 1)
}

/// Interleave the full 32 bits of `x` and `y` into a 64-bit Morton code.
#[inline]
pub fn morton_encode64(x: u32, y: u32) -> u64 {
    part1by1_64(x) | (part1by1_64(y) << 1)
}

/// Recover `(x, y)` from a [`morton_encode`] code (16-bit coordinates).
#[inline]
pub fn morton_decode(code: u32) -> (u32, u32) {
    (compact1by1(code), compact1by1(code >> 1))
}

/// Recover `(x, y)` from a [`morton_encode64`] code (32-bit coordinates).
#[inline]
pub fn morton_decode64(code: u64) -> (u32, u32) {
    (compact1by1_64(code), compact1by1_64(code >> 1))
}

/// Interleave the low 10 bits of `x`, `y`, `z` into a 30-bit Morton code
/// (`z` odd-positioned, `y` middle, `x` lowest).
#[inline]
pub fn morton_encode3(x: u32, y: u32, z: u32) -> u32 {
    part1by2(x) | (part1by2(y) << 1) | (part1by2(z) << 2)
}

/// Recover `(x, y, z)` from a [`morton_encode3`] code.
#[inline]
pub fn morton_decode3(code: u32) -> (u32, u32, u32) {
    (
        compact1by2(code),
        compact1by2(code >> 1),
        compact1by2(code >> 2),
    )
}

// Spreading/compacting helpers — the classic bit-dance masks.

fn part1by1(x: u32) -> u32 {
    let mut v = x & 0xFFFF;
    v = (v | (v << 8)) & 0x00FF_00FF;
    v = (v | (v << 4)) & 0x0F0F_0F0F;
    v = (v | (v << 2)) & 0x3333_3333;
    v = (v | (v << 1)) & 0x5555_5555;
    v
}

fn compact1by1(x: u32) -> u32 {
    let mut v = x & 0x5555_5555;
    v = (v | (v >> 1)) & 0x3333_3333;
    v = (v | (v >> 2)) & 0x0F0F_0F0F;
    v = (v | (v >> 4)) & 0x00FF_00FF;
    v = (v | (v >> 8)) & 0x0000_FFFF;
    v
}

fn part1by1_64(x: u32) -> u64 {
    let mut v = x as u64;
    v = (v | (v << 16)) & 0x0000_FFFF_0000_FFFF;
    v = (v | (v << 8)) & 0x00FF_00FF_00FF_00FF;
    v = (v | (v << 4)) & 0x0F0F_0F0F_0F0F_0F0F;
    v = (v | (v << 2)) & 0x3333_3333_3333_3333;
    v = (v | (v << 1)) & 0x5555_5555_5555_5555;
    v
}

fn compact1by1_64(x: u64) -> u32 {
    let mut v = x & 0x5555_5555_5555_5555;
    v = (v | (v >> 1)) & 0x3333_3333_3333_3333;
    v = (v | (v >> 2)) & 0x0F0F_0F0F_0F0F_0F0F;
    v = (v | (v >> 4)) & 0x00FF_00FF_00FF_00FF;
    v = (v | (v >> 8)) & 0x0000_FFFF_0000_FFFF;
    ((v | (v >> 16)) & 0xFFFF_FFFF) as u32
}

fn part1by2(x: u32) -> u32 {
    let mut v = x & 0x3FF;
    v = (v | (v << 16)) & 0x0300_00FF;
    v = (v | (v << 8)) & 0x0300_F00F;
    v = (v | (v << 4)) & 0x030C_30C3;
    v = (v | (v << 2)) & 0x0924_9249;
    v
}

fn compact1by2(x: u32) -> u32 {
    let mut v = x & 0x0924_9249;
    v = (v ^ (v >> 2)) & 0x030C_30C3;
    v = (v ^ (v >> 4)) & 0x0300_F00F;
    v = (v ^ (v >> 8)) & 0x0300_00FF;
    v = (v ^ (v >> 16)) & 0x3FF;
    v
}

/// Hilbert curve index of the `bits`-resolution cell `(x, y)` — the
/// position `(x, y)` occupies on the Hilbert curve of a `2^bits × 2^bits`
/// grid. `bits ≤ 16` (coordinates are taken mod `2^bits`). Consecutive
/// indices are always 4-adjacent cells — strictly better locality than
/// Morton.
///
/// Skilling's `xy→d` mapping (the standard iterative formulation): the
/// quadrant transform must rotate against the *full* grid mask `n-1`,
/// not the level size `s-1` — that asymmetry is the classic off-by-one
/// bug in writeups of this algorithm.
pub fn hilbert_encode(x: u32, y: u32, bits: u32) -> u64 {
    let bits = bits.min(16);
    if bits == 0 {
        return 0;
    }
    let n_mask = (1u32 << bits) - 1;
    let (mut x, mut y) = (x & n_mask, y & n_mask);
    let mut d: u64 = 0;
    let mut s = 1u32 << (bits - 1);
    while s > 0 {
        let rx = u32::from(x & s > 0);
        let ry = u32::from(y & s > 0);
        d += u64::from(s) * u64::from(s) * u64::from((3 * rx) ^ ry);
        // Rotate/flip the quadrant (rot with n = full grid side).
        if ry == 0 {
            if rx == 1 {
                x = n_mask - x;
                y = n_mask - y;
            }
            std::mem::swap(&mut x, &mut y);
        }
        s >>= 1;
    }
    d
}

/// Inverse of [`hilbert_encode`]: the cell at index `d` on the
/// `2^bits × 2^bits` Hilbert curve.
pub fn hilbert_decode(d: u64, bits: u32) -> (u32, u32) {
    let bits = bits.min(16);
    let (mut x, mut y) = (0u32, 0u32);
    let mut s = 1u32;
    let mut t = d;
    for _ in 0..bits {
        // rx, ry are the bit pair of t (rx is the high bit).
        let rx = ((t >> 1) & 1) as u32;
        let ry = ((t ^ u64::from(rx)) & 1) as u32;
        // Inverse transform — here the quadrant size *is* s.
        if ry == 0 {
            if rx == 1 {
                x = (s - 1) - x;
                y = (s - 1) - y;
            }
            std::mem::swap(&mut x, &mut y);
        }
        x += s * rx;
        y += s * ry;
        t >>= 2;
        s <<= 1;
    }
    (x, y)
}

/// Sort `points` into Morton (z-order) spatial order — a stable,
/// content-defined ordering that clusters nearby points. Signed
/// coordinates are biased by the sign bit so the full `i32` range works.
pub fn spatial_sort(points: &mut [(i32, i32)]) {
    points
        .sort_by_key(|&(x, y)| morton_encode64((x as u32) ^ 0x8000_0000, (y as u32) ^ 0x8000_0000));
}

/// Morton order of `(x, y)` as a single comparison key — biased for signed
/// input, so usable in ordered maps (`BTreeMap<u64, _>`) without sorting.
#[inline]
pub fn morton_key(x: i32, y: i32) -> u64 {
    morton_encode64((x as u32) ^ 0x8000_0000, (y as u32) ^ 0x8000_0000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn morton_round_trips() {
        for x in 0..64u32 {
            for y in 0..64u32 {
                assert_eq!(morton_decode(morton_encode(x, y)), (x, y));
            }
        }
        let mut rng = SplitMix64::new(0x20DE);
        for _ in 0..2000 {
            let (x, y) = (rng.next_u32(), rng.next_u32());
            assert_eq!(morton_decode64(morton_encode64(x, y)), (x, y));
        }
    }

    #[test]
    fn morton3_round_trips() {
        for x in 0..16u32 {
            for y in 0..16u32 {
                for z in 0..16u32 {
                    assert_eq!(morton_decode3(morton_encode3(x, y, z)), (x, y, z));
                }
            }
        }
    }

    #[test]
    fn hilbert_round_trips_and_walks_adjacently() {
        for bits in 1..=8u32 {
            let n = 1u64 << (2 * bits);
            let mut prev = hilbert_decode(0, bits);
            for d in 1..n {
                let (x, y) = hilbert_decode(d, bits);
                assert_eq!(hilbert_encode(x, y, bits), d, "decode→encode at {d}");
                let step = (x as i32 - prev.0 as i32).abs() + (y as i32 - prev.1 as i32).abs();
                assert_eq!(step, 1, "Hilbert steps must be 4-adjacent at {d}");
                prev = (x, y);
            }
        }
    }

    #[test]
    fn morton_clusters_better_than_row_order() {
        // Locality property: consecutive points in Morton order are closer
        // on average than consecutive points in lexicographic (row) order,
        // on a cloud scattered over a square. (A degenerate thin band is
        // already ordered by x — that's not where Morton pays off.)
        let mut rng = SplitMix64::new(77);
        let mut pts: Vec<(i32, i32)> = (0..400)
            .map(|_| (rng.range(-500, 500), rng.range(-500, 500)))
            .collect();
        let mut morton_pts = pts.clone();
        spatial_sort(&mut morton_pts);
        pts.sort();
        let step = |p: &[(i32, i32)]| -> i64 {
            p.windows(2)
                .map(|w| ((w[1].0 - w[0].0) as i64).abs() + ((w[1].1 - w[0].1) as i64).abs())
                .sum()
        };
        assert!(step(&morton_pts) < step(&pts), "Morton should cluster");
    }

    #[test]
    fn morton_key_orders_signed_coords() {
        // Biased key keeps sign order on each axis' high bits.
        assert!(morton_key(-1, 0) < morton_key(0, 0));
        assert!(morton_key(0, -1) < morton_key(0, 0));
        assert_eq!(morton_key(3, 4), morton_key(3, 4));
    }
}
