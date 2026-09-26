//! Shift_JIS / JIS X 0208 decoding (WHATWG trail ranges — lead `0x81..=0x9F`,
//! `0xE0..=0xFC`, trail `0x40..=0x7E` or `0x80..=0xFC`).
//!
//! [`next`] classifies the byte at `i`: ASCII (`0x00..=0x7F`), halfwidth
//! katakana (`0xA1..=0xDF`), a two-byte JIS X 0208 character returned as a
//! `ku`/`ten` (row/cell, 1-based) pair, or an invalid byte (`0x80`, `0xA0`,
//! `0xFD..=0xFF`, or a truncated/bad trail).
//!
//! ```
//! use izanagi_kit::sjis::{next, Next, to_kuten, count};
//! let d = b"A\x83e";  // 'A', then SJIS 0x83 0x65 -> ku 5, ten 38
//! assert_eq!(next(d, 0), Some((Next::Ascii(b'A'), 1)));
//! assert_eq!(next(d, 1), Some((Next::JisX0208 { ku: 5, ten: 38 }, 2)));
//! assert_eq!(to_kuten(0x83, 0x65), (5, 38));
//! assert_eq!(count(b"A\x83e\xB1"), Some(3));     // + halfwidth katakana
//! assert_eq!(next(b"\x81", 0), Some((Next::Truncated, 0)));
//! ```

/// `true` for a Shift_JIS lead byte (`0x81..=0x9F`, `0xE0..=0xFC`).
pub fn is_lead(b: u8) -> bool {
    (0x81..=0x9f).contains(&b) || (0xe0..=0xfc).contains(&b)
}

/// `true` for a valid trail byte (`0x40..=0x7E`, `0x80..=0xFC` — note the
/// excluded `0x7F`).
pub fn is_trail(b: u8) -> bool {
    (0x40..=0x7e).contains(&b) || (0x80..=0xfc).contains(&b)
}

/// `true` for a halfwidth-katakana single byte (`0xA1..=0xDF`).
pub fn is_katakana(b: u8) -> bool {
    (0xa1..=0xdf).contains(&b)
}

/// What [`next`] decoded at one position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// Single ASCII byte (`0x00..=0x7F`).
    Ascii(u8),
    /// Halfwidth katakana — the raw `0xA1..=0xDF` byte (add `0xFEC0` for
    /// the Unicode scalar).
    Katakana(u8),
    /// Two-byte JIS X 0208 character, as 1-based `ku`/`ten` (row/cell).
    JisX0208 {
        /// Row number (1..=94) in the JIS X 0208 plane.
        ku: u8,
        /// Cell number (1..=94) within `ku`.
        ten: u8,
    },
    /// Byte that is illegal in Shift_JIS (`0x80`, `0xA0`, `0xFD..=0xFF`).
    Invalid(u8),
    /// Lead byte whose trail is missing or out of range — advances 0.
    Truncated,
}

/// SJIS `(lead, trail)` -> 1-based JIS X 0208 `(ku, ten)`.
/// Callers must have validated `is_lead`/`is_trail`.
pub fn to_kuten(lead: u8, trail: u8) -> (u8, u8) {
    let hi = if lead <= 0x9f {
        lead - 0x81
    } else {
        lead - 0xc1
    };
    let lo = if trail <= 0x7e {
        trail - 0x40
    } else {
        trail - 0x41
    };
    let p = u16::from(hi) * 188 + u16::from(lo);
    ((p / 94 + 1) as u8, (p % 94 + 1) as u8)
}

/// Decode the character starting at `d[i]`; `Some((what, consumed))`, `None`
/// only when `i >= d.len()`.
pub fn next(d: &[u8], i: usize) -> Option<(Next, usize)> {
    let b = *d.get(i)?;
    if b <= 0x7f {
        return Some((Next::Ascii(b), 1));
    }
    if is_katakana(b) {
        return Some((Next::Katakana(b), 1));
    }
    if !is_lead(b) {
        return Some((Next::Invalid(b), 1));
    }
    match d.get(i + 1) {
        Some(&t) if is_trail(t) => {
            let (ku, ten) = to_kuten(b, t);
            Some((Next::JisX0208 { ku, ten }, 2))
        }
        _ => Some((Next::Truncated, 0)),
    }
}

/// Walk the whole buffer; `Some(char_count)` if every byte decodes cleanly,
/// `None` on the first `Invalid`/`Truncated`.
pub fn count(d: &[u8]) -> Option<usize> {
    let (mut i, mut n) = (0usize, 0usize);
    while i < d.len() {
        match next(d, i)? {
            (Next::Invalid(_) | Next::Truncated, _) => return None,
            (_, w) => {
                i += w;
                n += 1;
            }
        }
    }
    Some(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges() {
        assert!(is_lead(0x81) && is_lead(0x9f) && is_lead(0xe0) && is_lead(0xfc));
        assert!(!is_lead(0x80) && !is_lead(0xa0) && !is_lead(0xfd));
        assert!(is_trail(0x40) && is_trail(0x7e) && is_trail(0x80) && is_trail(0xfc));
        assert!(!is_trail(0x3f) && !is_trail(0x7f) && !is_trail(0xfd));
        assert!(is_katakana(0xa1) && is_katakana(0xdf) && !is_katakana(0xe0));
    }

    #[test]
    fn kuten_edge_rows() {
        assert_eq!(to_kuten(0x81, 0x40), (1, 1)); // first character
        assert_eq!(to_kuten(0x81, 0x9f), (2, 1)); // trail >= 0x9f flips row
        assert_eq!(to_kuten(0xea, 0xa4), (84, 6)); // 0xEA high block (IBM ext.)
        assert_eq!(to_kuten(0x9f, 0xfc), (62, 94));
    }

    #[test]
    fn next_classes() {
        assert_eq!(next(b"\x5c", 0), Some((Next::Ascii(0x5c), 1)));
        assert_eq!(next(b"\xfd", 0), Some((Next::Invalid(0xfd), 1)));
        assert_eq!(next(b"\x81\x7f", 0), Some((Next::Truncated, 0)));
        assert_eq!(next(b"", 0), None);
        match next(b"\x88\x9f", 0) {
            Some((Next::JisX0208 { ku, ten }, 2)) => assert_eq!((ku, ten), (16, 1)),
            _ => panic!("double byte expected"),
        }
    }

    #[test]
    fn count_checks_whole_buffer() {
        assert_eq!(count(b"abc"), Some(3));
        assert_eq!(count(b"\x8f"), None);
        assert_eq!(count(b"a\xff"), None);
        assert_eq!(count(b""), Some(0));
    }
}
