//! GBK decoding (lead `0x81..=0xFE`, trail `0x40..=0xFE` excluding `0x7F` —
//! the GBK extension lets trails dip into `0x40..=0x7E`, unlike pure GB 2312
//! which restricts them to `0xA1..=0xFE`).
//!
//! Reports the linear GBK *point* `(lead - 0x81) * 190 + trail_index`
//! instead of a Unicode scalar.
//!
//! ```
//! use izanagi_kit::gbk::{next, Next, point, count, is_gbx_trail};
//! let d = b"A\xb0\xa1";             // 'A' + GBK 0xB0 0xA1 ("啊")
//! assert_eq!(next(d, 0), Some((Next::Ascii(b'A'), 1)));
//! assert_eq!(next(d, 1), Some((Next::Gbk { point: 0x2f * 190 + 0x60 }, 2)));
//! assert!(is_gbx_trail(0x50));      // low trail — GBK-only, not GB 2312
//! assert_eq!(point(0x81, 0x40), 0);
//! assert_eq!(count(b"\x81\x40\x81\xfe"), Some(2));
//! ```

/// `true` for a GBK lead byte (`0x81..=0xFE`).
pub fn is_lead(b: u8) -> bool {
    (0x81..=0xfe).contains(&b)
}

/// `true` for a GBK trail byte: `0x40..=0xFE` with the single hole at `0x7F`.
pub fn is_trail(b: u8) -> bool {
    (0x40..=0xfe).contains(&b) && b != 0x7f
}

/// `true` when `trail` is in the GBK-only low range `0x40..=0x7E` — valid in
/// GBK but illegal in pure GB 2312.
pub fn is_gbx_trail(trail: u8) -> bool {
    (0x40..=0x7e).contains(&trail)
}

/// What [`next`] decoded at one position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// Single ASCII byte.
    Ascii(u8),
    /// Two-byte GBK character, as its linear `point` index.
    Gbk {
        /// Linear GBK point index: `(lead - 0x81) * 190 + trail_index`.
        point: u16,
    },
    /// Illegal byte (`0x80` or `0xFF`).
    Invalid(u8),
    /// Lead with missing/bad trail; consumed 0.
    Truncated,
}

/// `(lead, trail)` -> GBK linear point (callers must validate the pair).
pub fn point(lead: u8, trail: u8) -> u16 {
    let t = trail - if trail <= 0x7e { 0x40 } else { 0x41 };
    (lead as u16 - 0x81) * 190 + t as u16
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
        Some(&t) if is_trail(t) => Some((Next::Gbk { point: point(b, t) }, 2)),
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
        assert!(is_trail(0x40) && is_trail(0x7e) && is_trail(0xfe));
        assert!(!is_trail(0x3f) && !is_trail(0x7f) && !is_trail(0xff));
        assert!(is_gbx_trail(0x7e) && !is_gbx_trail(0xa1));
    }

    #[test]
    fn point_formula() {
        assert_eq!(point(0x81, 0x40), 0);
        assert_eq!(point(0x81, 0x80), 63);
        assert_eq!(point(0xb0, 0xa1), 0x2f * 190 + 0x60);
    }

    #[test]
    fn next_and_count() {
        assert_eq!(next(b"\xff", 0), Some((Next::Invalid(0xff), 1)));
        assert_eq!(next(b"\x81\x7f", 0), Some((Next::Truncated, 0)));
        assert_eq!(next(b"\x81", 0), Some((Next::Truncated, 0)));
        assert_eq!(next(b"", 0), None);
        assert_eq!(count(b"x\x81\x41"), Some(2)); // low trail OK in GBK
        assert_eq!(count(b"\x81\x7f"), None);
    }
}
