//! EUC-JP decoding (JIS X 0201 + JIS X 0208 + halfwidth katakana +
//! JIS X 0212 plane 2 via the `0x8F` SS3 shift).
//!
//! Byte classes: ASCII `0x00..=0x7F`; `0x8E` (SS2) followed by
//! `0xA1..=0xDF` is a halfwidth katakana; two bytes `0xA1..=0xFE` each are a
//! JIS X 0208 `ku`/`ten` pair; `0x8F` (SS3) plus two `0xA1..=0xFE` bytes is a
//! JIS X 0212 character. `0x80..=0x8D`, `0x90..=0x9F` and `0xFF` are invalid.
//!
//! ```
//! use izanagi_kit::eucjp::{next, Next, count};
//! let d = b"A\x8E\xB1\xA4\xA2";      // 'A', katakana, JIS X 0208 ku4/ten2
//! assert_eq!(next(d, 0), Some((Next::Ascii(b'A'), 1)));
//! assert_eq!(next(d, 1), Some((Next::Katakana(0xB1), 2)));
//! assert_eq!(next(d, 3), Some((Next::JisX0208 { ku: 4, ten: 2 }, 2)));
//! assert_eq!(count(b"\x8F\xA4\xA2"), Some(1));  // JIS X 0212 plane
//! ```

/// `true` for `0xA1..=0xFE` — a valid code byte in any EUC-JP multibyte run.
pub fn is_code(b: u8) -> bool {
    (0xa1..=0xfe).contains(&b)
}

/// `true` for a halfwidth-katakana code byte after `0x8E` (`0xA1..=0xDF`).
pub fn is_kana_code(b: u8) -> bool {
    (0xa1..=0xdf).contains(&b)
}

/// What [`next`] decoded at one position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// Single ASCII byte.
    Ascii(u8),
    /// Halfwidth katakana — raw `0x8E`-shifted code byte (`0xA1..=0xDF`).
    Katakana(u8),
    /// Two-byte JIS X 0208, as 1-based `ku`/`ten`.
    JisX0208 {
        /// Row number (1..=94).
        ku: u8,
        /// Cell number (1..=94) within `ku`.
        ten: u8,
    },
    /// `0x8F`-shifted JIS X 0212, as 1-based `ku`/`ten`.
    JisX0212 {
        /// Row number (1..=94) in the JIS X 0212 plane.
        ku: u8,
        /// Cell number (1..=94) within `ku`.
        ten: u8,
    },
    /// Illegal byte.
    Invalid(u8),
    /// Multibyte lead whose following bytes are missing or out of range.
    Truncated,
}

/// Decode the character starting at `d[i]`; `None` only when `i >= d.len()`.
pub fn next(d: &[u8], i: usize) -> Option<(Next, usize)> {
    let b = *d.get(i)?;
    if b <= 0x7f {
        return Some((Next::Ascii(b), 1));
    }
    if b == 0x8e {
        return match d.get(i + 1) {
            Some(&c) if is_kana_code(c) => Some((Next::Katakana(c), 2)),
            _ => Some((Next::Truncated, 0)),
        };
    }
    if b == 0x8f {
        return match (d.get(i + 1), d.get(i + 2)) {
            (Some(&c1), Some(&c2)) if is_code(c1) && is_code(c2) => Some((
                Next::JisX0212 {
                    ku: c1 - 0xa0,
                    ten: c2 - 0xa0,
                },
                3,
            )),
            _ => Some((Next::Truncated, 0)),
        };
    }
    if is_code(b) {
        return match d.get(i + 1) {
            Some(&c) if is_code(c) => Some((
                Next::JisX0208 {
                    ku: b - 0xa0,
                    ten: c - 0xa0,
                },
                2,
            )),
            _ => Some((Next::Truncated, 0)),
        };
    }
    Some((Next::Invalid(b), 1))
}

/// Walk the whole buffer; `Some(char_count)` if clean, `None` at the first
/// `Invalid`/`Truncated`.
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
    fn classes() {
        assert!(is_code(0xa1) && is_code(0xfe) && !is_code(0xa0) && !is_code(0xff));
        assert!(is_kana_code(0xdf) && !is_kana_code(0xe0));
    }

    #[test]
    fn next_paths() {
        assert_eq!(next(b"\x8e\xff", 0), Some((Next::Truncated, 0)));
        assert_eq!(next(b"\x8f\xa4", 0), Some((Next::Truncated, 0)));
        assert_eq!(next(b"\x8d", 0), Some((Next::Invalid(0x8d), 1)));
        assert_eq!(next(b"\x90", 0), Some((Next::Invalid(0x90), 1)));
        assert_eq!(next(b"\xa4", 0), Some((Next::Truncated, 0)));
        assert_eq!(
            next(b"\xa4\xa2", 0),
            Some((Next::JisX0208 { ku: 4, ten: 2 }, 2))
        );
        assert_eq!(
            next(b"\x8f\xd5\xb0", 0),
            Some((Next::JisX0212 { ku: 53, ten: 16 }, 3))
        );
    }

    #[test]
    fn count_all() {
        assert_eq!(count(b"hello"), Some(5));
        assert_eq!(count(b"\x8e"), None);
        assert_eq!(count(b"\x8f\xa4\xa2a"), Some(2));
    }
}
