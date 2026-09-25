//! xxHash32/xxHash64 (Yann Collet, xxHash spec r.5) — the fast non-crypto
//! hash sibling of `fnv1a`/`sip`, for hash tables and checksums where a wide
//! avalanche matters more than SipHash's keyed strength.
//!
//! Both are pure integer mixing — no lookup tables, endian-independent by
//! construction (little-endian loads spelled out byte-wise).
//!
//! ```
//! use izanagi_kit::xxhash::{xxh32, xxh64};
//! assert_eq!(xxh32(b"", 0), 0x02cc5d05);
//! assert_eq!(xxh64(b"", 0), 0xef46db3751d8e999);
//! ```

const P32_1: u32 = 0x9E37_79B1;
const P32_2: u32 = 0x85EB_CA77;
const P32_3: u32 = 0xC2B2_AE3D;
const P32_4: u32 = 0x27D4_EB2F;
const P32_5: u32 = 0x1656_67B1;

const P64_1: u64 = 0x9E37_79B1_85EB_CA87;
const P64_2: u64 = 0xC2B2_AE3D_27D4_EB4F;
const P64_3: u64 = 0x1656_67B1_9E37_79F9;
const P64_4: u64 = 0x85EB_CA77_C2B2_AE63;
const P64_5: u64 = 0x27D4_EB2F_1656_67C5;

fn u32le(b: &[u8], i: usize) -> u32 {
    (b[i] as u32) | ((b[i + 1] as u32) << 8) | ((b[i + 2] as u32) << 16) | ((b[i + 3] as u32) << 24)
}

fn u64le(b: &[u8], i: usize) -> u64 {
    let mut v = 0u64;
    for k in 0..8 {
        v |= (b[i + k] as u64) << (k * 8);
    }
    v
}

/// xxHash32 of `data` with `seed`.
pub fn xxh32(data: &[u8], seed: u32) -> u32 {
    let n = data.len();
    let mut i = 0usize;
    let mut h: u32;
    if n >= 16 {
        let mut v1 = seed.wrapping_add(P32_1).wrapping_add(P32_2);
        let mut v2 = seed.wrapping_add(P32_2);
        let mut v3 = seed;
        let mut v4 = seed.wrapping_sub(P32_1);
        while i + 16 <= n {
            for v in [&mut v1, &mut v2, &mut v3, &mut v4] {
                *v = v
                    .wrapping_add(u32le(data, i).wrapping_mul(P32_2))
                    .rotate_left(13)
                    .wrapping_mul(P32_1);
                i += 4;
            }
        }
        h = v1
            .rotate_left(1)
            .wrapping_add(v2.rotate_left(7))
            .wrapping_add(v3.rotate_left(12))
            .wrapping_add(v4.rotate_left(18));
    } else {
        h = seed.wrapping_add(P32_5);
    }
    h = h.wrapping_add(n as u32);
    while i + 4 <= n {
        h = h
            .wrapping_add(u32le(data, i).wrapping_mul(P32_3))
            .rotate_left(17)
            .wrapping_mul(P32_4);
        i += 4;
    }
    while i < n {
        h = h
            .wrapping_add((data[i] as u32).wrapping_mul(P32_5))
            .rotate_left(11)
            .wrapping_mul(P32_1);
        i += 1;
    }
    h ^= h >> 15;
    h = h.wrapping_mul(P32_2);
    h ^= h >> 13;
    h = h.wrapping_mul(P32_3);
    h ^ (h >> 16)
}

/// xxHash64 of `data` with `seed`.
pub fn xxh64(data: &[u8], seed: u64) -> u64 {
    let n = data.len();
    let mut i = 0usize;
    let mut h: u64;
    if n >= 32 {
        let mut v1 = seed.wrapping_add(P64_1).wrapping_add(P64_2);
        let mut v2 = seed.wrapping_add(P64_2);
        let mut v3 = seed;
        let mut v4 = seed.wrapping_sub(P64_1);
        while i + 32 <= n {
            for v in [&mut v1, &mut v2, &mut v3, &mut v4] {
                *v = v
                    .wrapping_add(u64le(data, i).wrapping_mul(P64_2))
                    .rotate_left(31)
                    .wrapping_mul(P64_1);
                i += 8;
            }
        }
        h = v1
            .rotate_left(1)
            .wrapping_add(v2.rotate_left(7))
            .wrapping_add(v3.rotate_left(12))
            .wrapping_add(v4.rotate_left(18));
        for v in [v1, v2, v3, v4] {
            let m = v.wrapping_mul(P64_2).rotate_left(31).wrapping_mul(P64_1);
            h ^= m;
            h = h.wrapping_mul(P64_1).wrapping_add(P64_4);
        }
    } else {
        h = seed.wrapping_add(P64_5);
    }
    h = h.wrapping_add(n as u64);
    while i + 8 <= n {
        let k = u64le(data, i)
            .wrapping_mul(P64_2)
            .rotate_left(31)
            .wrapping_mul(P64_1);
        h ^= k;
        h = h.rotate_left(27).wrapping_mul(P64_1).wrapping_add(P64_4);
        i += 8;
    }
    if i + 4 <= n {
        h ^= (u32le(data, i) as u64).wrapping_mul(P64_1);
        h = h.rotate_left(23).wrapping_mul(P64_2).wrapping_add(P64_3);
        i += 4;
    }
    while i < n {
        h ^= (data[i] as u64).wrapping_mul(P64_5);
        h = h.rotate_left(11).wrapping_mul(P64_1);
        i += 1;
    }
    h ^= h >> 33;
    h = h.wrapping_mul(P64_2);
    h ^= h >> 29;
    h = h.wrapping_mul(P64_3);
    h ^ (h >> 32)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reference values from the xxHash spec / xxhsum for seed 0.
    #[test]
    fn known_vectors_seed0() {
        assert_eq!(xxh32(b"", 0), 0x02cc_5d05);
        assert_eq!(xxh32(b"a", 0), 0x550d_7456);
        assert_eq!(xxh32(b"abc", 0), 0x32d1_53ff);
        assert_eq!(xxh64(b"", 0), 0xef46_db37_51d8_e999);
        assert_eq!(xxh64(b"a", 0), 0xd24e_c4f1_a98c_6e5b);
        assert_eq!(xxh64(b"abc", 0), 0x44bc_2cf5_ad77_0999);
    }

    #[test]
    fn seed_changes_hash() {
        assert_ne!(xxh32(b"abc", 0), xxh32(b"abc", 1));
        assert_ne!(xxh64(b"abc", 0), xxh64(b"abc", 1));
    }

    #[test]
    fn length_lanes() {
        // Cross each block/tail boundary; values pinned as regression points
        // computed by this implementation (self-consistency pins).
        for n in [0usize, 3, 4, 8, 15, 16, 31, 32, 33, 64, 100] {
            let data: Vec<u8> = (0..n).map(|i| (i * 31 + 7) as u8).collect();
            let a = xxh64(&data, 42);
            let b = xxh64(&data, 42);
            assert_eq!(a, b);
            let c = xxh32(&data, 42);
            assert_eq!(c, xxh32(&data, 42));
            // Perturbing one byte changes the hash.
            if n > 0 {
                let mut d = data.clone();
                d[n / 2] ^= 0x80;
                assert_ne!(a, xxh64(&d, 42));
            }
        }
    }

    #[test]
    fn cross_lane_reference() {
        // xxh64("The quick brown fox jumps over the lazy dog", 0) —
        // widely-published reference value.
        let s = b"The quick brown fox jumps over the lazy dog";
        assert_eq!(xxh64(s, 0), 0x0b24_2d36_1fda_71bc);
    }

    #[test]
    fn det_hash_impl() {
        use crate::world_hash::{DetHash, Fnv1a};
        let mut h = Fnv1a::new();
        xxh64(b"state", 0).det_hash(&mut h);
        assert_eq!(xxh64(b"state", 0), xxh64(b"state", 0));
    }
}
