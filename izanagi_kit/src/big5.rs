//! Big5 decoding (lead `0x81..=0xFE`, trail `0x40..=0x7E` or `0xA1..=0xFE` —
//! the `0x80..=0xA0` gap between the two trail blocks is invalid).
//!
//! The decoder reports the Big5 *point* — a linear index
//! `(lead - 0x81) * 157 + trail_index` — rather than a Unicode scalar, since a
//! full mapping table is out of scope.
//!
//! ```
//! use izanagi_kit::big5::{next, Next, point, count};
//! let d = b"A\xa4\x40";            // 'A' + Big5 0xA4 0x40 ("一")
//! assert_eq!(next(d, 0), Some((Next::Ascii(b'A'), 1)));
//! assert_eq!(next(d, 1), Some((Next::Big5 { point: 0x23 * 157 }, 2)));
//! assert_eq!(point(0xa4, 0x40), 0x23 * 157);
//! assert_eq!(count(b"\x81\x40\x81\x7e"), Some(2));
//! assert_eq!(next(b"\x81\x90", 0), Some((Next::Truncated, 0)));
//! ```

/// `true` for a Big5 lead byte (`0x81..=0xFE`).
pub fn is_lead(b: u8) -> bool {
    (0x81..=0xfe).contains(&b)
}

/// `true` for a Big5 trail byte (`0x40..=0x7E` or `0xA1..=0xFE`).
pub fn is_trail(b: u8) -> bool {
    (0x40..=0x7e).contains(&b) || (0xa1..=0xfe).contains(&b)
}

/// What [`next`] decoded at one position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// Single ASCII byte.
    Ascii(u8),
    /// Two-byte Big5 character, as its linear `point` index.
    Big5 {
        /// Linear Big5 point index: `(lead - 0x81) * 157 + trail_index`.
        point: u16,
    },
    /// Illegal byte (`0x80` only — every other non-ASCII byte is a lead).
    Invalid(u8),
    /// Lead with a missing or out-of-range trail; consumed 0.
    Truncated,
}

/// `(lead, trail)` -> Big5 linear point. Callers must validate the bytes.
pub fn point(lead: u8, trail: u8) -> u16 {
    let t = if trail <= 0x7e {
        trail - 0x40
    } else {
        trail - 0x62
    };
    (lead as u16 - 0x81) * 157 + t as u16
}

/// Decode the character at `d[i]`; `None` only when `i >= d.len()`.
pub fn next(d: &[u8], i: usize) -> Option<(Next, usize)> {
    let b = *d.get(i)?;
    if b <= 0x7f {
        return Some((Next::Ascii(b), 1));
    }
    if !is_lead(b) {
        return Some((Next::Invalid(b), 1));
    }
    match d.get(i + 1) {
        Some(&t) if is_trail(t) => Some((Next::Big5 { point: point(b, t) }, 2)),
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
        assert!(is_lead(0x81) && is_lead(0xfe) && !is_lead(0x80) && !is_lead(0xff));
        assert!(is_trail(0x40) && is_trail(0x7e) && is_trail(0xa1) && is_trail(0xfe));
        assert!(!is_trail(0x3f) && !is_trail(0x7f) && !is_trail(0xa0) && !is_trail(0xff));
    }

    #[test]
    fn point_formula() {
        assert_eq!(point(0x81, 0x40), 0);
        assert_eq!(point(0x81, 0xa1), 63);
        assert_eq!(point(0xfe, 0xfe), 0x7d * 157 + 156);
    }

    #[test]
    fn next_and_count() {
        assert_eq!(next(b"\x80", 0), Some((Next::Invalid(0x80), 1)));
        assert_eq!(next(b"\x81", 0), Some((Next::Truncated, 0)));
        assert_eq!(next(b"\x81\x90", 0), Some((Next::Truncated, 0)));
        assert_eq!(next(b"", 3), None);
        assert_eq!(count(b"a\x81\x40"), Some(2));
        assert_eq!(count(b"\x82"), None);
    }
}
