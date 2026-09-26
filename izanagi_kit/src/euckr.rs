//! EUC-KR decoding (KS X 1001, single 94x94 plane).
//!
//! ASCII `0x00..=0x7F`; any other byte must pair up: two bytes in
//! `0xA1..=0xFE` form a `ku`/`ten` pair. Unlike EUC-JP there are no `0x8E`/
//! `0x8F` shift bytes — `0x80..=0xA0` and `0xFF` are simply invalid.
//!
//! ```
//! use izanagi_kit::euckr::{next, Next, count};
//! let d = b"A\xb0\xa1";               // 'A' + EUC-KR 0xB0 0xA1 (ku 16, ten 1)
//! assert_eq!(next(d, 0), Some((Next::Ascii(b'A'), 1)));
//! assert_eq!(next(d, 1), Some((Next::Ksx1001 { ku: 16, ten: 1 }, 2)));
//! assert_eq!(count(b"\xa1\xa1\xfe\xfe"), Some(2));
//! assert_eq!(next(b"\x8e\xa1", 0), Some((Next::Invalid(0x8e), 1)));
//! ```

/// `true` for an EUC-KR code byte (`0xA1..=0xFE`).
pub fn is_code(b: u8) -> bool {
    (0xa1..=0xfe).contains(&b)
}

/// What [`next`] decoded at one position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// Single ASCII byte.
    Ascii(u8),
    /// Two-byte KS X 1001 character, as 1-based `ku`/`ten`.
    Ksx1001 {
        /// Row number (1..=94).
        ku: u8,
        /// Cell number (1..=94) within `ku`.
        ten: u8,
    },
    /// Illegal byte (`0x80..=0xA0` or `0xFF` — including the would-be
    /// shift bytes `0x8E`/`0x8F`, which EUC-KR does not use).
    Invalid(u8),
    /// First byte valid but its pair is missing or out of range; consumed 0.
    Truncated,
}

/// Decode the character at `d[i]`; `None` only when `i >= d.len()`.
pub fn next(d: &[u8], i: usize) -> Option<(Next, usize)> {
    let b = *d.get(i)?;
    if b <= 0x7f {
        return Some((Next::Ascii(b), 1));
    }
    if !is_code(b) {
        return Some((Next::Invalid(b), 1));
    }
    match d.get(i + 1) {
        Some(&c) if is_code(c) => Some((
            Next::Ksx1001 {
                ku: b - 0xa0,
                ten: c - 0xa0,
            },
            2,
        )),
        _ => Some((Next::Truncated, 0)),
    }
}

/// Walk the whole buffer; `Some(char_count)` if clean, else `None`.
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
        assert!(is_code(0xa1) && is_code(0xfe) && !is_code(0xa0) && !is_code(0xff));
    }

    #[test]
    fn next_paths() {
        assert_eq!(next(b"\xa1", 0), Some((Next::Truncated, 0)));
        assert_eq!(next(b"\xa1\x7f", 0), Some((Next::Truncated, 0)));
        assert_eq!(next(b"\xff", 0), Some((Next::Invalid(0xff), 1)));
        assert_eq!(next(b"\x8f\xa1\xa1", 0), Some((Next::Invalid(0x8f), 1)));
        assert_eq!(
            next(b"\xfe\xfe", 0),
            Some((Next::Ksx1001 { ku: 94, ten: 94 }, 2))
        );
        assert_eq!(next(b"", 0), None);
    }

    #[test]
    fn count_all() {
        assert_eq!(count(b"ab"), Some(2));
        assert_eq!(count(b"\xa1"), None);
        assert_eq!(count(b"\xa1\xa1z"), Some(2));
    }
}
