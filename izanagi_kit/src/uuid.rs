//! UUID generation and parsing (RFC 4122, variant 1 / version 4) — 128-bit
//! identifiers as the simpler, non-sortable sibling of [`crate::ulid`].
//!
//! [`uuid4`] draws 16 bytes from a [`SplitMix64`] and stamps the
//! version/variant bits; [`format()`] renders the canonical
//! `8-4-4-4-12` lowercase hex form; [`parse`] accepts both the hyphenated
//! canonical form and 32 bare hex digits, normalizing to lowercase on output.
//!
//! ```
//! use izanagi_kit::{rng::SplitMix64, uuid};
//! let mut rng = SplitMix64::new(1);
//! let u = uuid::uuid4(&mut rng);
//! let s = uuid::format(u);
//! assert_eq!(s.len(), 36);
//! assert_eq!(uuid::parse(&s), Some(u));
//! ```

use crate::rng::SplitMix64;

const HEX: &[u8; 16] = b"0123456789abcdef";

/// Draw a random version-4 UUID from `rng`.
pub fn uuid4(rng: &mut SplitMix64) -> [u8; 16] {
    let mut u = [0u8; 16];
    for w in u.chunks_exact_mut(8) {
        w.copy_from_slice(&rng.next_u64().to_le_bytes());
    }
    u[6] = (u[6] & 0x0f) | 0x40; // version 4
    u[8] = (u[8] & 0x3f) | 0x80; // variant 1 (10xx)
    u
}

/// Format `u` as `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` (lowercase).
pub fn format(u: [u8; 16]) -> String {
    let mut s = String::with_capacity(36);
    for (i, b) in u.iter().enumerate() {
        if i == 4 || i == 6 || i == 8 || i == 10 {
            s.push('-');
        }
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

fn hexval(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Parse `s` as a UUID. Accepts the canonical `8-4-4-4-12` hyphenated form
/// (any hex case) and 32 bare hex digits. Returns `None` on wrong length,
/// bad separators, or non-hex characters.
pub fn parse(s: &str) -> Option<[u8; 16]> {
    let b = s.as_bytes();
    if b.len() != 36 && b.len() != 32 {
        return None;
    }
    let mut u = [0u8; 16];
    let mut nibble = 0usize;
    let mut groups = [0usize; 4]; // hyphen positions observed
    for (i, &c) in b.iter().enumerate() {
        if c == b'-' {
            if b.len() != 36 || !(i == 8 || i == 13 || i == 18 || i == 23) {
                return None;
            }
            groups[i / 6] += 1;
            continue;
        }
        let hi = hexval(c)?;
        let lo = if nibble % 2 == 0 {
            // Next iteration supplies the low nibble; stash high nibble.
            u[nibble / 2] = hi << 4;
            nibble += 1;
            continue;
        } else {
            hi
        };
        u[nibble / 2] |= lo;
        nibble += 1;
    }
    if b.len() == 36 && groups.iter().sum::<usize>() != 4 {
        return None;
    }
    if nibble != 32 {
        return None;
    }
    Some(u)
}

/// The `Nil` UUID (all zeros).
pub const NIL: [u8; 16] = [0; 16];

/// The `Max` UUID (all ones).
pub const MAX: [u8; 16] = [0xff; 16];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_round_trips() {
        let u: [u8; 16] = core::array::from_fn(|i| (i * 17 + 3) as u8);
        let s = format(u);
        assert_eq!(s.len(), 36);
        assert_eq!(s.chars().nth(8), Some('-'));
        assert_eq!(parse(&s), Some(u));
    }

    #[test]
    fn uuid4_stamps_version_and_variant() {
        let mut rng = SplitMix64::new(42);
        for _ in 0..64 {
            let u = uuid4(&mut rng);
            assert_eq!(u[6] >> 4, 4);
            assert_eq!(u[8] >> 6, 2);
        }
    }

    #[test]
    fn deterministic_stream() {
        let mut a = SplitMix64::new(7);
        let mut b = SplitMix64::new(7);
        assert_eq!(uuid4(&mut a), uuid4(&mut b));
        let mut c = SplitMix64::new(8);
        assert_ne!(uuid4(&mut a), uuid4(&mut c));
    }

    #[test]
    fn parse_accepts_bare_and_upper() {
        let s = "00112233-4455-6677-8899-aabbccddeeff";
        let u = parse(s).unwrap();
        assert_eq!(u[0], 0x00);
        assert_eq!(u[15], 0xff);
        let bare = s.replace('-', "");
        assert_eq!(parse(&bare), Some(u));
        assert_eq!(parse(&s.to_uppercase()), Some(u));
        assert_eq!(
            format(parse("f81d4fae-7dec-11d0-a765-00a0c91e6bf6").unwrap()),
            "f81d4fae-7dec-11d0-a765-00a0c91e6bf6"
        );
    }

    #[test]
    fn parse_rejects() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("0000"), None);
        assert_eq!(parse("zz112233-4455-6677-8899-aabbccddeeff"), None);
        // Hyphen in the wrong position.
        assert_eq!(parse("001122334-455-6677-8899-aabbccddeeff"), None);
        // Missing a hyphen.
        assert_eq!(parse("00112233-44556677-8899-aabbccddeeff"), None);
        // Trailing junk.
        assert_eq!(parse("00112233-4455-6677-8899-aabbccddeefff"), None);
    }

    #[test]
    fn nil_and_max() {
        assert_eq!(format(NIL), "00000000-0000-0000-0000-000000000000");
        assert_eq!(format(MAX), "ffffffff-ffff-ffff-ffff-ffffffffffff");
        assert_eq!(parse(&format(NIL)), Some(NIL));
    }
}
