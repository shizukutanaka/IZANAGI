//! Key derivation on the existing `hmac`/`sha256` primitives —
//! **PBKDF2** (RFC 2898 §5.2: iterated HMAC for stretching passwords
//! into keys) and **HKDF** (RFC 5869: extract-then-expand for turning
//! high-entropy secrets into multiple independent keys). Both are
//! byte-deterministic by definition; determinism *is* the spec.
//!
//! ```
//! use izanagi_kit::kdf::{hkdf_expand, hkdf_extract, pbkdf2};
//!
//! // RFC 6070-style vector adapted to SHA-256.
//! let dk = pbkdf2(b"password", b"salt", 1, 32);
//! assert_eq!(
//!     dk[..8],
//!     [0x12, 0x0f, 0xb6, 0xcf, 0xfc, 0xf8, 0xb3, 0x2c]
//! );
//! let prk = hkdf_extract(b"salt", b"input keying material");
//! let okm = hkdf_expand(&prk, b"context", 32).unwrap();
//! assert_eq!(okm.len(), 32);
//! ```

use crate::hmac::hmac_sha256;

/// PBKDF2-HMAC-SHA256: `dkLen` bytes of derived key from `iters`
/// iterations. Each 32-byte block costs `iters` HMACs — `iters = 1`
/// is still correct (test vectors), production uses tens of
/// thousands. `out_len ≤ 32·(2³²−1)`; anything beyond is clamped by
/// the loop bound anyway.
pub fn pbkdf2(password: &[u8], salt: &[u8], iters: u32, out_len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(out_len);
    let mut block = 1u32;
    while out.len() < out_len {
        // U1 = HMAC(P, S ‖ INT32_BE(block)) — big-endian per spec,
        // spelled out so endianness never depends on the target.
        let mut input = Vec::with_capacity(salt.len() + 4);
        input.extend_from_slice(salt);
        input.extend_from_slice(&[
            (block >> 24) as u8,
            (block >> 16) as u8,
            (block >> 8) as u8,
            block as u8,
        ]);
        let mut u = hmac_sha256(password, &input);
        let mut t = u;
        for _ in 1..iters {
            u = hmac_sha256(password, &u);
            for (tb, ub) in t.iter_mut().zip(u.iter()) {
                *tb ^= ub;
            }
        }
        out.extend_from_slice(&t);
        block += 1;
    }
    out.truncate(out_len);
    out
}

/// HKDF-Extract (RFC 5869 §2.2): `PRK = HMAC-SHA256(salt, ikm)`.
/// Empty salt substitutes a 32-zero-byte salt per spec.
pub fn hkdf_extract(salt: &[u8], ikm: &[u8]) -> [u8; 32] {
    static ZERO_SALT: [u8; 32] = [0; 32];
    let key = if salt.is_empty() {
        &ZERO_SALT[..]
    } else {
        salt
    };
    hmac_sha256(key, ikm)
}

/// HKDF-Expand (RFC 5869 §2.3): `okm_len ≤ 255·32 = 8160` bytes
/// (`None` beyond — the spec forbids longer output).
/// `T(0) = ""`, `T(i) = HMAC(PRK, T(i−1) ‖ info ‖ byte(i))`.
pub fn hkdf_expand(prk: &[u8; 32], info: &[u8], okm_len: usize) -> Option<Vec<u8>> {
    if okm_len > 255 * 32 {
        return None;
    }
    let mut out = Vec::with_capacity(okm_len);
    let mut prev: Vec<u8> = Vec::new();
    let mut counter = 1u8;
    while out.len() < okm_len {
        let mut input = Vec::with_capacity(prev.len() + info.len() + 1);
        input.extend_from_slice(&prev);
        input.extend_from_slice(info);
        input.push(counter);
        prev = hmac_sha256(prk, &input).to_vec();
        out.extend_from_slice(&prev);
        counter = counter.wrapping_add(1);
    }
    out.truncate(okm_len);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn pbkdf2_matches_rfc_style_vectors() {
        // PBKDF2-HMAC-SHA256 published vectors.
        assert_eq!(
            pbkdf2(b"password", b"salt", 1, 32),
            hex("120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b")
        );
        assert_eq!(
            pbkdf2(b"password", b"salt", 2, 32),
            hex("ae4d0c95af6b46d32d0adff928f06dd02a303f8ef3c251dfd6e2d85a95474c43")
        );
        assert_eq!(
            pbkdf2(b"password", b"salt", 4096, 32),
            hex("c5e478d59288c841aa530db6845c4c8d962893a001ce4e11a4963873aa98134a")
        );
        // Longer output: second block is the U-chain at block index 2.
        let dk64 = pbkdf2(b"password", b"salt", 1, 64);
        assert_eq!(dk64.len(), 64);
        assert_eq!(dk64[..32], pbkdf2(b"password", b"salt", 1, 32)[..]);
        // Truncation works.
        assert_eq!(pbkdf2(b"password", b"salt", 1, 20).len(), 20);
        assert_eq!(pbkdf2(b"password", b"salt", 1, 0).len(), 0);
    }

    #[test]
    fn hkdf_matches_rfc5869_case1() {
        // RFC 5869 Test Case 1 (SHA-256).
        let ikm = [0x0bu8; 22];
        let salt = hex("000102030405060708090a0b0c");
        let info = hex("f0f1f2f3f4f5f6f7f8f9");
        let prk = hkdf_extract(&salt, &ikm);
        assert_eq!(
            prk.to_vec(),
            hex("077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5")
        );
        let okm = hkdf_expand(&prk, &info, 42).unwrap();
        assert_eq!(
            okm,
            hex("3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865")
        );
    }

    #[test]
    fn hkdf_edge_cases() {
        // Empty salt → zero-key extract; empty info legal.
        let prk = hkdf_extract(b"", b"ikm");
        let a = hkdf_expand(&prk, b"", 32).unwrap();
        assert_eq!(a.len(), 32);
        // Max and beyond.
        assert!(hkdf_expand(&prk, b"", 8160).is_some());
        assert!(hkdf_expand(&prk, b"", 8161).is_none());
        // Length 0 trivially fine.
        assert_eq!(hkdf_expand(&prk, b"x", 0).unwrap(), Vec::<u8>::new());
        // Different info → different keys.
        let b = hkdf_expand(&prk, b"other", 32).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn deterministic_twice() {
        assert_eq!(pbkdf2(b"p", b"s", 100, 48), pbkdf2(b"p", b"s", 100, 48));
        let prk = hkdf_extract(b"s", b"i");
        assert_eq!(hkdf_expand(&prk, b"n", 64), hkdf_expand(&prk, b"n", 64));
    }
}
