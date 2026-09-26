//! UTF-16 decoding with BOM detection and surrogate-pair handling.
//!
//! [`endianness`] inspects a leading BOM (`FE FF` BE / `FF FE` LE; absent →
//! big-endian, the spec default). [`next`] decodes one scalar at a byte
//! offset: a BMP unit, a valid surrogate pair → one `u32` scalar, or a lone
//! surrogate as `Unpaired`. [`encode`] goes the other way.
//!
//! ```
//! use izanagi_kit::utf16::{endianness, next, encode, Endianness, Next, count};
//! let d = [0xFE, 0xFF, 0x30, 0x42, 0xD8, 0x40, 0xDF, 0x0E]; // BE BOM + 'あ' + U+2030E
//! assert_eq!(endianness(&d), (Endianness::Big, 2));
//! assert_eq!(next(&d, 2, Endianness::Big), Some((Next::Scalar(0x3042), 2)));
//! assert_eq!(next(&d, 4, Endianness::Big), Some((Next::Scalar(0x2030E), 4)));
//! assert_eq!(encode(0x2030E), (0xD840, Some(0xDF0E)));
//! assert_eq!(encode(0x3042), (0x3042, None));
//! ```

/// Byte order of a UTF-16 stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Endianness {
    /// Big-endian (`FE FF` BOM, or the no-BOM default).
    Big,
    /// Little-endian (`FF FE` BOM).
    Little,
}

/// Inspect a leading BOM. Returns `(byte_order, bom_len)`; a missing BOM
/// defaults to big-endian per the Unicode spec, and `FF FE` wins over
/// UTF-32's `FF FE 00 00` prefix (callers handling UTF-32 must check first).
pub fn endianness(d: &[u8]) -> (Endianness, usize) {
    if d.starts_with(&[0xff, 0xfe]) {
        (Endianness::Little, 2)
    } else if d.starts_with(&[0xfe, 0xff]) {
        (Endianness::Big, 2)
    } else {
        (Endianness::Big, 0)
    }
}

/// `true` for a high surrogate `0xD800..=0xDBFF`.
pub fn is_high_surrogate(u: u16) -> bool {
    (0xd800..=0xdbff).contains(&u)
}

/// `true` for a low surrogate `0xDC00..=0xDFFF`.
pub fn is_low_surrogate(u: u16) -> bool {
    (0xdc00..=0xdfff).contains(&u)
}

/// Combine a validated `(high, low)` surrogate pair into its scalar.
pub fn pair(hi: u16, lo: u16) -> u32 {
    0x1_0000 + (((hi as u32 - 0xd800) << 10) | (lo as u32 - 0xdc00))
}

/// Read one `u16` code unit at byte offset `i` in the given order.
pub fn unit(d: &[u8], i: usize, e: Endianness) -> Option<u16> {
    let (a, b) = (*d.get(i)?, *d.get(i + 1)?);
    Some(match e {
        Endianness::Big => ((a as u16) << 8) | b as u16,
        Endianness::Little => ((b as u16) << 8) | a as u16,
    })
}

/// What [`next`] decoded at one byte offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// A Unicode scalar value (BMP unit or combined surrogate pair).
    Scalar(u32),
    /// A surrogate unit without its partner.
    Unpaired(u16),
    /// Odd trailing byte — one byte left over at end of input; consumed 0.
    Truncated,
}

/// Decode the scalar starting at byte offset `i`. `None` only when
/// `i >= d.len()`.
pub fn next(d: &[u8], i: usize, e: Endianness) -> Option<(Next, usize)> {
    if i >= d.len() {
        return None;
    }
    if d.len() - i < 2 {
        return Some((Next::Truncated, 0));
    }
    let u = unit(d, i, e)?;
    if is_high_surrogate(u) {
        return match unit(d, i + 2, e) {
            Some(lo) if is_low_surrogate(lo) => Some((Next::Scalar(pair(u, lo)), 4)),
            _ => Some((Next::Unpaired(u), 2)),
        };
    }
    if is_low_surrogate(u) {
        return Some((Next::Unpaired(u), 2));
    }
    Some((Next::Scalar(u as u32), 2))
}

/// Encode a scalar `<= 0x10FFFF` as `(first, maybe_second)` code units;
/// returns `(0xFFFD, None)` for surrogate-range or out-of-range inputs.
pub fn encode(s: u32) -> (u16, Option<u16>) {
    if s > 0x10_ffff || (0xd800..=0xdfff).contains(&s) {
        return (0xfffd, None);
    }
    if s < 0x1_0000 {
        return (s as u16, None);
    }
    let v = s - 0x1_0000;
    (0xd800 + (v >> 10) as u16, Some(0xdc00 + (v as u16 & 0x3ff)))
}

/// Count scalars in a buffer already past any BOM; `None` on the first
/// `Unpaired`/`Truncated`.
pub fn count(d: &[u8], e: Endianness) -> Option<usize> {
    let (mut i, mut n) = (0usize, 0usize);
    while i < d.len() {
        match next(d, i, e)? {
            (Next::Unpaired(_) | Next::Truncated, _) => return None,
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
    fn bom_detect() {
        assert_eq!(endianness(&[0xff, 0xfe, 0x41]), (Endianness::Little, 2));
        assert_eq!(endianness(&[0xfe, 0xff]), (Endianness::Big, 2));
        assert_eq!(endianness(&[0x41]), (Endianness::Big, 0));
        assert_eq!(endianness(&[]), (Endianness::Big, 0));
    }

    #[test]
    fn units_and_pairs() {
        let le = [0x42, 0x30, 0x34, 0xd8, 0x1e, 0xdd];
        assert_eq!(unit(&le, 0, Endianness::Little), Some(0x3042));
        assert_eq!(
            next(&le, 2, Endianness::Little),
            Some((Next::Scalar(0x1d11e), 4))
        );
        assert_eq!(
            next(&le, 0, Endianness::Big),
            Some((Next::Scalar(0x4230), 2))
        );
        // Lone high and low surrogates report Unpaired.
        let lone = [0xd8, 0x00, 0x00, 0x41, 0xdc, 0x00];
        assert_eq!(
            next(&lone, 0, Endianness::Big),
            Some((Next::Unpaired(0xd800), 2))
        );
        assert_eq!(
            next(&lone, 4, Endianness::Big),
            Some((Next::Unpaired(0xdc00), 2))
        );
        assert_eq!(
            next(&[0x41], 0, Endianness::Big),
            Some((Next::Truncated, 0))
        );
        assert_eq!(next(&[], 0, Endianness::Big), None);
        assert!(is_high_surrogate(0xd800) && is_low_surrogate(0xdc00));
        assert_eq!(pair(0xd800, 0xdc00), 0x10000);
        assert_eq!(pair(0xdbff, 0xdfff), 0x10ffff);
    }

    #[test]
    fn encode_roundtrip() {
        assert_eq!(encode(0x41), (0x41, None));
        assert_eq!(encode(0x10000), (0xd800, Some(0xdc00)));
        assert_eq!(encode(0x10ffff), (0xdbff, Some(0xdfff)));
        assert_eq!(encode(0xd800), (0xfffd, None));
        assert_eq!(encode(0x110000), (0xfffd, None));
    }

    #[test]
    fn count_all() {
        assert_eq!(count(&[0x00, 0x41], Endianness::Big), Some(1));
        assert_eq!(count(&[0xd8, 0x00], Endianness::Big), None);
        assert_eq!(count(&[0x00], Endianness::Big), None);
    }
}
