//! HMAC-SHA256 message authentication (RFC 2104 over the
//! crate's integer-verified [`crate::sha256`]). The key is
//! padded/hashed to the 64-byte SHA-256 block, XORed into the
//! 0x36 ipad / 0x5c opad, and hashed twice — a MAC with no
//! state beyond the two digests, so every split of the
//! message yields the same tag. RFC 4231 test cases are
//! verified in tests.
//!
//! ```
//! use izanagi_kit::hmac::hmac_sha256;
//! // RFC 4231 test case 1: 20-byte 0x0b key, "Hi There".
//! let tag = hmac_sha256(&[0x0b; 20], b"Hi There");
//! assert_eq!(
//!     tag[..8],
//!     [0xb0, 0x34, 0x4c, 0x61, 0xd8, 0xdb, 0x38, 0x53]
//! );
//! ```

use crate::sha256::Sha256;

const BLOCK: usize = 64;

/// Streaming HMAC-SHA256 state: `write` the message, `finish`
/// for the 32-byte tag.
pub struct Hmac {
    inner: Sha256,
    opad: [u8; BLOCK],
}

impl Hmac {
    /// New HMAC under `key` of any length (keys longer than the
    /// 64-byte block are hashed first, per RFC 2104).
    pub fn new(key: &[u8]) -> Self {
        let mut k = [0u8; BLOCK];
        if key.len() > BLOCK {
            let h = crate::sha256::sha256(key);
            k[..32].copy_from_slice(&h);
        } else {
            k[..key.len()].copy_from_slice(key);
        }
        let mut ipad = [0u8; BLOCK];
        let mut opad = [0u8; BLOCK];
        for i in 0..BLOCK {
            ipad[i] = k[i] ^ 0x36;
            opad[i] = k[i] ^ 0x5c;
        }
        let mut inner = Sha256::new();
        inner.write(&ipad);
        Hmac { inner, opad }
    }

    /// Feed `data`; the tag is independent of the write split.
    pub fn write(&mut self, data: &[u8]) {
        self.inner.write(data);
    }

    /// Final 32-byte authentication tag.
    pub fn finish(self) -> [u8; 32] {
        let h = self.inner.finish();
        let mut outer = Sha256::new();
        outer.write(&self.opad);
        outer.write(&h);
        outer.finish()
    }
}

/// One-shot HMAC-SHA256 tag of `data` under `key`.
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut h = Hmac::new(key);
    h.write(data);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc4231_case1() {
        // Key=20×0x0b, data="Hi There".
        let tag = hmac_sha256(&[0x0b; 20], b"Hi There");
        assert_eq!(
            tag,
            [
                0xb0, 0x34, 0x4c, 0x61, 0xd8, 0xdb, 0x38, 0x53, 0x5c, 0xa8, 0xaf, 0xce, 0xaf, 0x0b,
                0xf1, 0x2b, 0x88, 0x1d, 0xc2, 0x00, 0xc9, 0x83, 0x3d, 0xa7, 0x26, 0xe9, 0x37, 0x6c,
                0x2e, 0x32, 0xcf, 0xf7,
            ]
        );
    }

    #[test]
    fn rfc4231_case2() {
        // Key="Jefe", data="what do ya want for nothing?".
        let tag = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
        assert_eq!(
            tag,
            [
                0x5b, 0xdc, 0xc1, 0x46, 0xbf, 0x60, 0x75, 0x4e, 0x6a, 0x04, 0x24, 0x26, 0x08, 0x95,
                0x75, 0xc7, 0x5a, 0x00, 0x3f, 0x08, 0x9d, 0x27, 0x39, 0x83, 0x9d, 0xec, 0x58, 0xb9,
                0x64, 0xec, 0x38, 0x43,
            ]
        );
    }

    #[test]
    fn rfc4231_case4() {
        // RFC 4231 TC4: key = 25×0x0e.
        let key = [0x0e; 25];
        let tag = hmac_sha256(
            &key,
            b"Test Using Larger Than Block-Size Key - Hash Key First",
        );
        assert_eq!(
            tag,
            [
                0x2c, 0x36, 0x5f, 0x32, 0x3f, 0x5c, 0x3f, 0x1a, 0x16, 0xc3, 0x82, 0x86, 0x4a, 0x6e,
                0x62, 0xf6, 0x25, 0x35, 0x4e, 0x74, 0x6f, 0xc8, 0xe6, 0xc6, 0x9c, 0xd2, 0xfa, 0x91,
                0x8c, 0xee, 0xf2, 0x0e,
            ]
        );
    }

    #[test]
    fn rfc4231_case6_long_key() {
        // Key = 131×0xaa (longer than the 64-byte block → the key
        // itself is SHA-256 hashed first), data = 50×0xdd.
        let key = [0xaa; 131];
        let data = [0xdd; 50];
        let tag = hmac_sha256(&key, &data);
        assert_eq!(
            tag,
            [
                0x12, 0x4c, 0x7d, 0x23, 0x85, 0xaa, 0x17, 0x43, 0xaa, 0xad, 0x12, 0x20, 0x4e, 0x34,
                0x64, 0xf0, 0x63, 0x05, 0xfd, 0x1a, 0x6d, 0x29, 0x12, 0x50, 0xfa, 0x56, 0x4d, 0xce,
                0xff, 0xab, 0x0c, 0x8a,
            ]
        );
    }

    #[test]
    fn split_independence_and_determinism() {
        let msg: Vec<u8> = (0..=255u8).cycle().take(1000).collect();
        let whole = hmac_sha256(b"k", &msg);
        for cut in [0usize, 1, 31, 32, 33, 63, 64, 65, 100, 999, 1000] {
            let mut h = Hmac::new(b"k");
            h.write(&msg[..cut]);
            h.write(&msg[cut..]);
            assert_eq!(h.finish(), whole, "cut {cut}");
        }
        assert_eq!(hmac_sha256(b"k", &msg), whole);
        assert_ne!(hmac_sha256(b"k2", &msg), whole);
        // One flipped message byte changes the tag.
        let mut m2 = msg.clone();
        m2[0] ^= 1;
        assert_ne!(hmac_sha256(b"k", &m2), whole);
    }
}
