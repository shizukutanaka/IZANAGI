//! ChaCha20-Poly1305 **AEAD** — RFC 8439 §2.8: the kit's existing
//! `ChaCha20` stream and `poly1305` MAC composed into authenticated
//! encryption. Block counter 0 produces the one-time Poly1305 key;
//! encryption runs from counter 1; the tag covers
//! `aad ‖ pad16 ‖ ct ‖ pad16 ‖ len64(aad) ‖ len64(ct)`.
//!
//! Tamper-evident replay records are the kit use-case: `seal` output
//! is a pure function of `(key, nonce, aad, plaintext)`, so two
//! runs of the same sim produce byte-identical sealed blobs, and any
//! flipped bit fails `open` (`None`), never yields garbled plaintext.
//!
//! ```
//! use izanagi_kit::aead::{open, seal};
//!
//! let (key, nonce) = ([7u32; 8], [9u32; 3]);
//! let blob = seal(&key, &nonce, b"hdr", b"secret state");
//! assert_eq!(open(&key, &nonce, b"hdr", &blob), Some(b"secret state".to_vec()));
//! let mut bad = blob.clone();
//! bad[0] ^= 1;
//! assert_eq!(open(&key, &nonce, b"hdr", &bad), None); // tag mismatch
//! ```

use crate::chacha::ChaCha20;
use crate::poly1305::poly1305;

/// Seal `plaintext` under (`key`,`nonce`,`aad`) → `ct ‖ tag` (16-byte
/// trailer).
pub fn seal(key: &[u32; 8], nonce: &[u32; 3], aad: &[u8], plaintext: &[u8]) -> Vec<u8> {
    let c = ChaCha20::new(key, nonce);
    let mut ct = plaintext.to_vec();
    // Encrypt from block counter 1, 64 bytes per block.
    for (i, chunk) in ct.chunks_mut(64).enumerate() {
        let ks = c.block(1 + i as u32);
        for (b, k) in chunk.iter_mut().zip(ks.iter()) {
            *b ^= k;
        }
    }
    let tag = tag_of(&c, aad, &ct);
    ct.extend_from_slice(&tag);
    ct
}

/// Open a `seal`ed blob — `Some(plaintext)` only when the tag
/// verifies; wrong key/nonce/aad or any flipped bit → `None`.
pub fn open(key: &[u32; 8], nonce: &[u32; 3], aad: &[u8], blob: &[u8]) -> Option<Vec<u8>> {
    if blob.len() < 16 {
        return None;
    }
    let (ct, tag) = blob.split_at(blob.len() - 16);
    let c = ChaCha20::new(key, nonce);
    let want = tag_of(&c, aad, ct);
    // Bitwise-accumulate compare: same answer as `==` but no
    // position information leaks through early exit.
    let mut diff = 0u8;
    for (a, b) in tag.iter().zip(want.iter()) {
        diff |= a ^ b;
    }
    if diff != 0 {
        return None;
    }
    let mut pt = ct.to_vec();
    for (i, chunk) in pt.chunks_mut(64).enumerate() {
        let ks = c.block(1 + i as u32);
        for (b, k) in chunk.iter_mut().zip(ks.iter()) {
            *b ^= k;
        }
    }
    Some(pt)
}

fn pad16(out: &mut Vec<u8>, len: usize) {
    let rem = len % 16;
    if rem != 0 {
        out.resize(out.len() + (16 - rem), 0);
    }
}

fn tag_of(c: &ChaCha20, aad: &[u8], ct: &[u8]) -> [u8; 16] {
    let mut poly_key = [0u8; 32];
    poly_key.copy_from_slice(&c.block(0)[..32]);
    let mut mac_input = Vec::with_capacity(aad.len() + ct.len() + 32);
    mac_input.extend_from_slice(aad);
    pad16(&mut mac_input, aad.len());
    mac_input.extend_from_slice(ct);
    pad16(&mut mac_input, ct.len());
    mac_input.extend_from_slice(&(aad.len() as u64).to_le_bytes());
    mac_input.extend_from_slice(&(ct.len() as u64).to_le_bytes());
    poly1305(&mac_input, &poly_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc8439_aead_vector() {
        // RFC 8439 A.5 — the canonical seal vector.
        let key = [
            0x83828180, 0x87868584, 0x8b8a8988, 0x8f8e8d8c, 0x93929190, 0x97969594, 0x9b9a9998,
            0x9f9e9d9c,
        ];
        let nonce = [0x0000_0007, 0x4342_4140, 0x4746_4544];
        let aad = [
            0x50, 0x51, 0x52, 0x53, 0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7,
        ];
        let pt = b"Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.";
        let blob = seal(&key, &nonce, &aad, pt);
        let tag = &blob[blob.len() - 16..];
        assert_eq!(
            tag,
            &[
                0x1a, 0xe1, 0x0b, 0x59, 0x4f, 0x09, 0xe2, 0x6a, 0x7e, 0x90, 0x2e, 0xcb, 0xd0, 0x60,
                0x06, 0x91
            ]
        );
        assert_eq!(open(&key, &nonce, &aad, &blob).unwrap(), pt.to_vec());
        // First four ciphertext bytes per RFC.
        assert_eq!(&blob[..4], &[0xd3, 0x1a, 0x8d, 0x34]);
    }

    #[test]
    fn tamper_detection() {
        let (key, nonce, aad) = ([1u32; 8], [2u32; 3], b"ctx".as_ref());
        let blob = seal(&key, &nonce, aad, b"payload");
        for pos in 0..blob.len() {
            let mut bad = blob.clone();
            bad[pos] ^= 1;
            assert!(open(&key, &nonce, aad, &bad).is_none(), "pos {pos}");
        }
        // Wrong key / nonce / aad all fail.
        assert!(open(&[9u32; 8], &nonce, aad, &blob).is_none());
        assert!(open(&key, &[9u32; 3], aad, &blob).is_none());
        assert!(open(&key, &nonce, b"other", &blob).is_none());
        // Truncated / empty.
        assert!(open(&key, &nonce, aad, &blob[..10]).is_none());
        assert!(open(&key, &nonce, aad, &[]).is_none());
    }

    #[test]
    fn edge_lengths() {
        let (key, nonce) = ([3u32; 8], [4u32; 3]);
        // Empty plaintext → tag-only blob.
        let blob = seal(&key, &nonce, b"", b"");
        assert_eq!(blob.len(), 16);
        assert_eq!(open(&key, &nonce, b"", &blob), Some(Vec::new()));
        // Multi-block (spans 64-byte boundary).
        let pt: Vec<u8> = (0..300).map(|i| i as u8).collect();
        let blob = seal(&key, &nonce, b"", &pt);
        assert_eq!(open(&key, &nonce, b"", &blob), Some(pt));
    }

    #[test]
    fn deterministic_twice() {
        let f = || seal(&[5u32; 8], &[6u32; 3], b"a", b"p");
        assert_eq!(f(), f());
    }
}
