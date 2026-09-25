//! Base58 — Bitcoin's alphabet (`123456789ABCDEFGHJKLMNPQRSTUVWXYZ`
//! `abcdefghijkmnopqrstuvwxyz`, no `0 O I l`) plus the Flickr variant
//! ordering, and the double-SHA-256 checked form (`encode_check` /
//! `decode_check`) used by legacy addresses.
//!
//! Encoding treats the input as one big-endian integer: leading zero
//! bytes become leading `1` digits. Decode reverses it exactly, so
//! `decode(encode(x)) == x` for every input.
//!
//! ```
//! use izanagi_kit::base58::{encode, decode};
//!
//! assert_eq!(encode(b"hello world"), "StV1DL6CwTryKyV");
//! assert_eq!(decode("StV1DL6CwTryKyV"), Some(b"hello world".to_vec()));
//! ```

use std::string::String;
use std::vec::Vec;

/// Bitcoin alphabet — the default.
pub const BTC: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
/// Flickr ordering (letters before digits switched around).
pub const FLICKR: &[u8; 58] = b"123456789abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ";

/// Encode `data` on the Bitcoin alphabet.
pub fn encode(data: &[u8]) -> String {
    encode_with(data, BTC)
}

/// Decode a Bitcoin-alphabet string. `None` on any out-of-alphabet
/// character.
pub fn decode(s: &str) -> Option<Vec<u8>> {
    decode_with(s, BTC)
}

/// Encode on an explicit 58-byte alphabet.
pub fn encode_with(data: &[u8], alphabet: &[u8; 58]) -> String {
    let zeros = data.iter().take_while(|&&b| b == 0).count();
    // The number never needs more than log(256)/log(58) ≈ 1.366 digits
    // per input byte.
    let mut digits = vec![0u8; data.len() * 2 + 1];
    let mut ndig = 0usize;
    for &b in &data[zeros..] {
        let mut carry = b as u32;
        for d in digits[..ndig].iter_mut() {
            carry += (*d as u32) << 8;
            *d = (carry % 58) as u8;
            carry /= 58;
        }
        while carry > 0 {
            digits[ndig] = (carry % 58) as u8;
            carry /= 58;
            ndig += 1;
        }
    }
    let mut s = String::with_capacity(zeros + ndig);
    for _ in 0..zeros {
        s.push(alphabet[0] as char);
    }
    for d in digits[..ndig].iter().rev() {
        s.push(alphabet[*d as usize] as char);
    }
    s
}

/// Decode on an explicit 58-byte alphabet.
pub fn decode_with(s: &str, alphabet: &[u8; 58]) -> Option<Vec<u8>> {
    let mut map = [255u8; 256];
    for (i, &c) in alphabet.iter().enumerate() {
        map[c as usize] = i as u8;
    }
    let bytes = s.as_bytes();
    let zeros = bytes.iter().take_while(|&&c| c == alphabet[0]).count();
    let mut out = vec![0u8; bytes.len() + 1];
    let mut nbyte = 0usize;
    for &c in &bytes[zeros..] {
        let mut carry = map[c as usize] as u32;
        if carry > 57 {
            return None;
        }
        for b in out[..nbyte].iter_mut() {
            carry += (*b as u32) * 58;
            *b = carry as u8;
            carry >>= 8;
        }
        while carry > 0 {
            out[nbyte] = carry as u8;
            carry >>= 8;
            nbyte += 1;
        }
    }
    let mut v = Vec::with_capacity(zeros + nbyte);
    v.extend(std::iter::repeat(0u8).take(zeros));
    v.extend(out[..nbyte].iter().rev().copied());
    Some(v)
}

/// Base58Check: `encode(payload || sha256(sha256(payload))[..4])`.
pub fn encode_check(payload: &[u8]) -> String {
    let mut buf = payload.to_vec();
    let h = crate::sha256::sha256(&crate::sha256::sha256(payload));
    buf.extend_from_slice(&h[..4]);
    encode(&buf)
}

/// Reverse of [`encode_check`]: verifies the 4-byte checksum and
/// returns the payload. `None` on bad checksum or too-short input.
pub fn decode_check(s: &str) -> Option<Vec<u8>> {
    let raw = decode(s)?;
    if raw.len() < 4 {
        return None;
    }
    let (payload, sum) = raw.split_at(raw.len() - 4);
    let h = crate::sha256::sha256(&crate::sha256::sha256(payload));
    if h[..4] != *sum {
        return None;
    }
    Some(payload.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn btc_vectors() {
        assert_eq!(encode(b""), "");
        assert_eq!(encode(&[0]), "1");
        assert_eq!(encode(&[0, 0]), "11");
        assert_eq!(encode(b"a"), "2g");
        assert_eq!(encode(b"bbb"), "a3gV");
        assert_eq!(encode(b"hello world"), "StV1DL6CwTryKyV");
        assert_eq!(encode(b"Hello World!"), "2NEpo7TZRRrLZSi2U");
    }

    #[test]
    fn decode_roundtrips() {
        for s in [
            "",
            "1",
            "2g",
            "a3gV",
            "StV1DL6CwTryKyV",
            "2NEpo7TZRRrLZSi2U",
            "111",
        ] {
            let back = decode(s).unwrap();
            assert_eq!(encode(&back), s);
        }
    }

    #[test]
    fn zero_prefix_preserved() {
        let data = [0u8, 0, 0, 1, 2, 3];
        assert_eq!(decode(&encode(&data)), Some(data.to_vec()));
    }

    #[test]
    fn bad_chars_rejected() {
        for s in ["0", "O", "I", "l", "abc!", " "] {
            assert!(decode(s).is_none(), "{s}");
        }
    }

    #[test]
    fn flickr_alphabet() {
        assert_eq!(encode_with(b"hello world", FLICKR), "rTu1dk6cWsRYjYu");
        assert_eq!(
            decode_with("rTu1dk6cWsRYjYu", FLICKR),
            Some(b"hello world".to_vec())
        );
    }

    #[test]
    fn check_roundtrip_and_tamper() {
        let payload = b"\x00\x01payload";
        let s = encode_check(payload);
        assert_eq!(decode_check(&s).as_deref(), Some(&payload[..]));
        // Flip a non-check character.
        let mut chars: Vec<u8> = s.bytes().collect();
        chars[1] = if chars[1] == b'1' { b'2' } else { b'1' };
        let tampered = String::from_utf8_lossy(&chars).into_owned();
        assert!(decode_check(&tampered).is_none() || decode_check(&tampered).unwrap() == payload);
    }
}
