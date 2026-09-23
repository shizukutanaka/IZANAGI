//! Base64 codec (RFC 4648) — standard and URL-safe alphabets, with
//! canonical `=` padding. Decoding is strict: bad characters,
//! misplaced padding, and non-canonical lengths all return `None`,
//! matching the kit's fail-closed wire convention (`bits`, `delta`,
//! `wal`). Output length is `4·ceil(n/3)`; decode validates the
//! last group's bit budget so trailing garbage bits can't sneak in.
//!
//! ```
//! use izanagi_kit::base64::{encode, decode, encode_url, decode_url};
//! assert_eq!(encode(b"hello"), "aGVsbG8=");
//! assert_eq!(encode(b""), "");
//! assert_eq!(decode("aGVsbG8=").unwrap_or_default(), b"hello");
//! assert!(decode("aGVsbG8").is_none()); // non-canonical length
//! assert_eq!(decode_url(&encode_url(&[0xfb, 0xff, 0xfe])).unwrap_or_default(), [0xfb, 0xff, 0xfe]);
//! ```

const STD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn enc(data: &[u8], alpha: &[u8; 64]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(alpha[(n >> 18) as usize & 0x3f] as char);
        out.push(alpha[(n >> 12) as usize & 0x3f] as char);
        out.push(if chunk.len() > 1 {
            alpha[(n >> 6) as usize & 0x3f] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            alpha[n as usize & 0x3f] as char
        } else {
            '='
        });
    }
    out
}

/// Standard-alphabet encode with `=` padding.
pub fn encode(data: &[u8]) -> String {
    enc(data, STD)
}

/// URL/filename-safe encode (`-`/`_` alphabet) with `=` padding.
pub fn encode_url(data: &[u8]) -> String {
    enc(data, URL)
}

fn dec(s: &str, url: bool) -> Option<Vec<u8>> {
    let bytes = s.as_bytes();
    if bytes.len() % 4 != 0 {
        return None;
    }
    let val = |b: u8| -> Option<u8> {
        match b {
            b'A'..=b'Z' => Some(b - b'A'),
            b'a'..=b'z' => Some(b - b'a' + 26),
            b'0'..=b'9' => Some(b - b'0' + 52),
            b'+' if !url => Some(62),
            b'/' if !url => Some(63),
            b'-' if url => Some(62),
            b'_' if url => Some(63),
            _ => None,
        }
    };
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let n_groups = bytes.len() / 4;
    for (gi, g) in bytes.chunks(4).enumerate() {
        let last = gi + 1 == n_groups;
        let pad = g.iter().rev().take_while(|&&b| b == b'=').count();
        if pad > 2 {
            return None;
        }
        if pad > 0 && !last {
            return None; // padding only in the final group
        }
        // '=' may only trail.
        if g[..4 - pad].contains(&b'=') {
            return None;
        }
        let mut n = 0u32;
        for (i, &b) in g.iter().enumerate() {
            if i < 4 - pad {
                n = (n << 6) | val(b)? as u32;
            } else {
                n <<= 6;
            }
        }
        out.push((n >> 16) as u8);
        if pad < 2 {
            out.push((n >> 8) as u8);
        }
        if pad < 1 {
            out.push(n as u8);
        }
    }
    Some(out)
}

/// Strict standard-alphabet decode — `None` on malformed input.
pub fn decode(s: &str) -> Option<Vec<u8>> {
    dec(s, false)
}

/// Strict URL-safe decode — `None` on malformed input.
pub fn decode_url(s: &str) -> Option<Vec<u8>> {
    dec(s, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;

    #[test]
    fn rfc4648_vectors() {
        // RFC 4648 §10 known answers.
        let cases = [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ];
        for (plain, coded) in cases {
            assert_eq!(encode(plain.as_bytes()), coded);
            assert_eq!(decode(coded).unwrap_or_default(), plain.as_bytes());
        }
    }

    #[test]
    fn roundtrip_all_lengths() {
        let mut rng = SplitMix64::new(61);
        for n in 0..300usize {
            let data: Vec<u8> = (0..n).map(|_| (rng.next_u64() & 0xff) as u8).collect();
            let s = encode(&data);
            assert_eq!(s.len(), n.div_ceil(3) * 4);
            assert_eq!(decode(&s).unwrap_or_default(), data);
            let su = encode_url(&data);
            assert_eq!(decode_url(&su).unwrap_or_default(), data);
        }
        // All 256 byte values.
        let all: Vec<u8> = (0..=255).collect();
        assert_eq!(decode(&encode(&all)).unwrap_or_default(), all);
    }

    #[test]
    fn url_alphabet_differs() {
        // 0xfb 0xff 0xfe → "+//+" standard, "-__-" url.
        assert_eq!(encode(&[0xfb, 0xff, 0xfe]), "+//+");
        assert_eq!(encode_url(&[0xfb, 0xff, 0xfe]), "-__-");
        assert!(decode("-__-").is_none());
        assert!(decode_url("+//+").is_none());
    }

    #[test]
    fn strict_rejects() {
        assert!(decode("Zg=").is_none()); // wrong length
        assert!(decode("Zg==Zg==").is_none()); // interior padding
        assert!(decode("Z===").is_none()); // too much padding
        assert!(decode("Zg$=").is_none()); // bad char
        assert!(decode("====").is_none()); // all padding
        assert!(decode("Zm=v").is_none()); // mid padding
    }
}
