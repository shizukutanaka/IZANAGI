//! MD5 — RFC 1321 message digest. Not cryptographic (collisions are
//! practical — use `sha256`/`blake2s` for anything adversarial); the
//! kit needs it for *compatibility*: content systems, save formats,
//! and network protocols that name MD5 as their checksum must
//! reproduce it bit-for-bit, and a wrong constant or shift order is
//! invisible to casual testing — hence the RFC test-suite pin.
//!
//! ```
//! use izanagi_kit::md5::md5;
//!
//! let d = md5(b"abc");
//! assert_eq!(d[0], 0x90);
//! assert_eq!(d[15], 0x72);
//! ```

#[rustfmt::skip]
const K: [u32; 64] = [
    0xd76a_a478, 0xe8c7_b756, 0x2420_70db, 0xc1bd_ceee, 0xf57c_0faf, 0x4787_c62a, 0xa830_4613, 0xfd46_9501,
    0x6980_98d8, 0x8b44_f7af, 0xffff_5bb1, 0x895c_d7be, 0x6b90_1122, 0xfd98_7193, 0xa679_438e, 0x49b4_0821,
    0xf61e_2562, 0xc040_b340, 0x265e_5a51, 0xe9b6_c7aa, 0xd62f_105d, 0x0244_1453, 0xd8a1_e681, 0xe7d3_fbc8,
    0x21e1_cde6, 0xc337_07d6, 0xf4d5_0d87, 0x455a_14ed, 0xa9e3_e905, 0xfcef_a3f8, 0x676f_02d9, 0x8d2a_4c8a,
    0xfffa_3942, 0x8771_f681, 0x6d9d_6122, 0xfde5_380c, 0xa4be_ea44, 0x4bde_cfa9, 0xf6bb_4b60, 0xbebf_bc70,
    0x289b_7ec6, 0xeaa1_27fa, 0xd4ef_3085, 0x0488_1d05, 0xd9d4_d039, 0xe6db_99e5, 0x1fa2_7cf8, 0xc4ac_5665,
    0xf429_2244, 0x432a_ff97, 0xab94_23a7, 0xfc93_a039, 0x655b_59c3, 0x8f0c_cc92, 0xffef_f47d, 0x8584_5dd1,
    0x6fa8_7e4f, 0xfe2c_e6e0, 0xa301_4314, 0x4e08_11a1, 0xf753_7e82, 0xbd3a_f235, 0x2ad7_d2bb, 0xeb86_d391,
];

#[rustfmt::skip]
const S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
    5,  9, 14, 20, 5,  9, 14, 20, 5,  9, 14, 20, 5,  9, 14, 20,
    4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
    6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

/// MD5 digest of `msg` — 16 bytes, little-endian word order per
/// spec.
pub fn md5(msg: &[u8]) -> [u8; 16] {
    let mut padded = msg.to_vec();
    let bit_len = (msg.len() as u64).wrapping_mul(8);
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_le_bytes());

    let (mut a0, mut b0, mut c0, mut d0) = (
        0x6745_2301u32,
        0xefcd_ab89u32,
        0x98ba_dcfeu32,
        0x1032_5476u32,
    );
    for block in padded.chunks_exact(64) {
        let mut m = [0u32; 16];
        for (i, w) in m.iter_mut().enumerate() {
            *w = u32::from_le_bytes([
                block[4 * i],
                block[4 * i + 1],
                block[4 * i + 2],
                block[4 * i + 3],
            ]);
        }
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let tmp = d;
            d = c;
            c = b;
            b = b.wrapping_add(
                a.wrapping_add(f)
                    .wrapping_add(K[i])
                    .wrapping_add(m[g])
                    .rotate_left(S[i]),
            );
            a = tmp;
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }
    let mut out = [0u8; 16];
    out[..4].copy_from_slice(&a0.to_le_bytes());
    out[4..8].copy_from_slice(&b0.to_le_bytes());
    out[8..12].copy_from_slice(&c0.to_le_bytes());
    out[12..].copy_from_slice(&d0.to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(d: &[u8; 16]) -> String {
        d.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn rfc1321_test_suite() {
        assert_eq!(hex(&md5(b"")), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(hex(&md5(b"a")), "0cc175b9c0f1b6a831c399e269772661");
        assert_eq!(hex(&md5(b"abc")), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(
            hex(&md5(b"message digest")),
            "f96b697d7cb7938d525a2f31aaf161d0"
        );
        assert_eq!(
            hex(&md5(b"abcdefghijklmnopqrstuvwxyz")),
            "c3fcd3d76192e4007dfb496cca67e13b"
        );
        assert_eq!(
            hex(&md5(
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
            )),
            "d174ab98d277d9f5a5611c2c9f419d9f"
        );
        assert_eq!(
            hex(&md5(
                b"12345678901234567890123456789012345678901234567890123456789012345678901234567890"
            )),
            "57edf4a22be3c955ac49da2e2107b67a"
        );
    }

    #[test]
    fn padding_boundaries() {
        // 55 bytes → pad 1+7; 56 → pad 8+... second block; 64 → exact block.
        let m55: Vec<u8> = vec![b'x'; 55];
        let m56: Vec<u8> = vec![b'x'; 56];
        let m63: Vec<u8> = vec![b'x'; 63];
        let m64: Vec<u8> = vec![b'x'; 64];
        for m in [&m55, &m56, &m63, &m64] {
            assert_eq!(md5(m).len(), 16);
        }
        // All distinct (padding differences change the digest).
        let ds: Vec<_> = [&m55, &m56, &m63, &m64].iter().map(|m| md5(m)).collect();
        for i in 0..4 {
            for j in (i + 1)..4 {
                assert_ne!(ds[i], ds[j]);
            }
        }
        // Long input (>1 block).
        let long: Vec<u8> = (0..1000).map(|i| (i % 251) as u8).collect();
        assert_eq!(md5(&long), md5(&long));
    }

    #[test]
    fn deterministic_twice() {
        assert_eq!(md5(b"determinism"), md5(b"determinism"));
    }
}
