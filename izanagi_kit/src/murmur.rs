//! MurmurHash3 — Austin Appleby's non-cryptographic hash, the x86_32
//! and x64_128 variants. The wide-avalanche sibling of
//! [`crate::world_hash`]/[`crate::xxhash`]/[`crate::siphash`]: fast on bulk
//! keys, no claims about adversarial inputs.
//!
//! Both variants return their hash little-endian-agnostic: bytes are
//! consumed as little-endian words so results are identical on every
//! host. Vectors are pinned against the reference implementation.
//!
//! ```
//! use izanagi_kit::murmur::{murmur3_32, murmur3_128};
//!
//! assert_eq!(murmur3_32(b"hello", 0), 0x248b_fa47);
//! assert_eq!(murmur3_128(b"", 0), (0, 0));
//! ```

/// 32-bit MurmurHash3 (x86_32 variant).
pub fn murmur3_32(data: &[u8], seed: u32) -> u32 {
    let n = data.len();
    let mut h = seed;
    for c in data.chunks_exact(4) {
        let mut k =
            (c[0] as u32) | ((c[1] as u32) << 8) | ((c[2] as u32) << 16) | ((c[3] as u32) << 24);
        k = k.wrapping_mul(0xcc9e_2d51);
        k = k.rotate_left(15);
        k = k.wrapping_mul(0x1b87_3593);
        h ^= k;
        h = h.rotate_left(13);
        h = h.wrapping_mul(5).wrapping_add(0xe654_6b64);
    }
    let tail = &data[n - n % 4..];
    let mut k = 0u32;
    for (i, &b) in tail.iter().enumerate() {
        k |= (b as u32) << (8 * i);
    }
    if !tail.is_empty() {
        k = k.wrapping_mul(0xcc9e_2d51);
        k = k.rotate_left(15);
        k = k.wrapping_mul(0x1b87_3593);
        h ^= k;
    }
    h ^= n as u32;
    fmix32(h)
}

fn fmix32(mut h: u32) -> u32 {
    h ^= h >> 16;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    h
}

fn fmix64(mut k: u64) -> u64 {
    k ^= k >> 33;
    k = k.wrapping_mul(0xff51_afd7_ed55_8ccd);
    k ^= k >> 33;
    k = k.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    k ^= k >> 33;
    k
}

/// 128-bit MurmurHash3 (x64_128 variant), returned as `(h1, h2)` —
/// `h1` is the low half. Matches `mmh3.hash128` byte order when the two
/// halves are concatenated little-endian.
pub fn murmur3_128(data: &[u8], seed: u32) -> (u64, u64) {
    let n = data.len();
    let mut h1 = seed as u64;
    let mut h2 = seed as u64;
    for c in data.chunks_exact(16) {
        let mut k1 = u64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]);
        let mut k2 = u64::from_le_bytes([c[8], c[9], c[10], c[11], c[12], c[13], c[14], c[15]]);
        k1 = k1.wrapping_mul(0x87c3_7b91_1142_53d5);
        k1 = k1.rotate_left(31);
        k1 = k1.wrapping_mul(0x4cf5_ad43_2745_937f);
        h1 ^= k1;
        h1 = h1.rotate_left(27);
        h1 = h1.wrapping_add(h2);
        h1 = h1.wrapping_mul(5).wrapping_add(0x52dc_e729);
        k2 = k2.wrapping_mul(0x4cf5_ad43_2745_937f);
        k2 = k2.rotate_left(33);
        k2 = k2.wrapping_mul(0x87c3_7b91_1142_53d5);
        h2 ^= k2;
        h2 = h2.rotate_left(31);
        h2 = h2.wrapping_add(h1);
        h2 = h2.wrapping_mul(5).wrapping_add(0x3849_5ab5);
    }
    let tail = &data[n - n % 16..];
    let mut k1 = 0u64;
    let mut k2 = 0u64;
    for (i, &b) in tail.iter().take(8).enumerate() {
        k1 |= (b as u64) << (8 * i);
    }
    for (i, &b) in tail.iter().skip(8).enumerate() {
        k2 |= (b as u64) << (8 * i);
    }
    if tail.len() > 8 {
        k2 = k2.wrapping_mul(0x4cf5_ad43_2745_937f);
        k2 = k2.rotate_left(33);
        k2 = k2.wrapping_mul(0x87c3_7b91_1142_53d5);
        h2 ^= k2;
    }
    if !tail.is_empty() {
        k1 = k1.wrapping_mul(0x87c3_7b91_1142_53d5);
        k1 = k1.rotate_left(31);
        k1 = k1.wrapping_mul(0x4cf5_ad43_2745_937f);
        h1 ^= k1;
    }
    h1 ^= n as u64;
    h2 ^= n as u64;
    h1 = h1.wrapping_add(h2);
    h2 = h2.wrapping_add(h1);
    h1 = fmix64(h1);
    h2 = fmix64(h2);
    h1 = h1.wrapping_add(h2);
    h2 = h2.wrapping_add(h1);
    (h1, h2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x86_32_vectors() {
        // Reference-implementation values.
        assert_eq!(murmur3_32(b"", 0), 0);
        assert_eq!(murmur3_32(b"a", 0), 0x3c25_69b2);
        assert_eq!(murmur3_32(b"abc", 0), 0xb3dd_93fa);
        assert_eq!(murmur3_32(b"abcd", 0), 0x43ed_676a);
        assert_eq!(murmur3_32(b"hello", 0), 0x248b_fa47);
        assert_eq!(murmur3_32(b"hello world", 0), 0x5e92_8f0f);
        assert_eq!(murmur3_32(b"hello", 1), 0xbb4a_bcad);
    }

    #[test]
    fn x64_128_vectors() {
        assert_eq!(murmur3_128(b"", 0), (0, 0));
        assert_eq!(
            murmur3_128(b"hello", 0),
            (0xcbd8_a7b3_41bd_9b02, 0x5b1e_906a_48ae_1d19)
        );
        assert_eq!(
            murmur3_128(b"abcd", 0),
            (0xb87b_b7d6_4656_cd4f, 0xf200_3e88_6073_e875)
        );
        assert_eq!(
            murmur3_128(b"hello", 1),
            (0xa78d_dff5_adae_8d10, 0x1289_00ef_2090_0135)
        );
    }

    #[test]
    fn tail_lengths_all_cover() {
        // Every tail length 0..=15 hits a distinct code path.
        let mut prev = (0u32, (0u64, 0u64));
        for len in 0usize..=16 {
            let d = vec![0xABu8; len];
            let h = (murmur3_32(&d, 7), murmur3_128(&d, 7));
            if len > 0 {
                assert_ne!(h, prev, "len {len} collided with len {}", len - 1);
            }
            prev = h;
        }
    }

    #[test]
    fn deterministic_twice() {
        assert_eq!(murmur3_32(b"izanagi", 42), murmur3_32(b"izanagi", 42));
        assert_eq!(murmur3_128(b"izanagi", 42), murmur3_128(b"izanagi", 42));
    }
}
