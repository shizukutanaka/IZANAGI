//! STUN message parsing (RFC 5389).
//!
//! 20-byte header: `type u16BE` (top 2 bits clear), `len u16BE`
//! (attribute bytes, multiple of 4), magic cookie `0x2112A442`,
//! 12-byte transaction id. Attributes are `type u16 | len u16`
//! TLVs padded to 4-byte boundaries.
//!
//! ```
//! use izanagi_kit::stun;
//! let mut d = vec![0u8; 28];
//! d[0..2].copy_from_slice(&0x0001u16.to_be_bytes()); // Binding Request
//! d[2..4].copy_from_slice(&8u16.to_be_bytes()); // len 8
//! d[4..8].copy_from_slice(&0x2112A442u32.to_be_bytes()); // cookie
//! d[20..22].copy_from_slice(&0x0006u16.to_be_bytes()); // USERNAME
//! d[22..24].copy_from_slice(&4u16.to_be_bytes());
//! d[24..28].copy_from_slice(b"user");
//! let s = stun::parse(&d).unwrap();
//! assert_eq!(s.method, 1);
//! assert_eq!(s.attrs.len(), 1);
//! ```

use std::vec::Vec;

/// RFC 5389 magic cookie.
pub const COOKIE: u32 = 0x2112_A442;

/// One attribute header.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Attr {
    /// Attribute type.
    pub ty: u16,
    /// Value length (unpadded).
    pub len: u16,
    /// Byte offset of the value.
    pub at: usize,
}

/// A parsed STUN message.
#[derive(Clone, Debug, PartialEq)]
pub struct Stun {
    /// Method bits of `type` (low 12 effective bits: M11..M0).
    pub method: u16,
    /// Class bits (`type` bits 4 and 8 → 0 req / 1 ind / 2 resp / 3 err).
    pub class: u8,
    /// `len` — attribute-section byte count.
    pub len: u16,
    /// Transaction id.
    pub transaction_id: [u8; 12],
    /// Attribute headers.
    pub attrs: Vec<Attr>,
}

fn u16be(d: &[u8], at: usize) -> Option<u16> {
    let s = d.get(at..at.checked_add(2)?)?;
    Some((s[0] as u16) << 8 | s[1] as u16)
}

fn u32be(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32) << 24 | (s[1] as u32) << 16 | (s[2] as u32) << 8 | s[3] as u32)
}

/// Parses a STUN message: top two `type` bits clear, cookie present,
/// declared length tiles the input with 4-byte-aligned attributes.
pub fn parse(d: &[u8]) -> Option<Stun> {
    if d.len() < 20 {
        return None;
    }
    let ty = u16be(d, 0)?;
    if ty & 0xC000 != 0 {
        return None;
    }
    let len = u16be(d, 2)?;
    if len % 4 != 0 {
        return None;
    }
    if u32be(d, 4)? != COOKIE {
        return None;
    }
    let end = 20usize.checked_add(len as usize)?;
    if end > d.len() {
        return None;
    }
    let mut transaction_id = [0u8; 12];
    transaction_id.copy_from_slice(d.get(8..20)?);
    let mut attrs = Vec::new();
    let mut at = 20usize;
    while at < end {
        let a_ty = u16be(d, at)?;
        let a_len = u16be(d, at + 2)?;
        let value_at = at + 4;
        if value_at.checked_add(a_len as usize)? > end {
            return None;
        }
        attrs.push(Attr {
            ty: a_ty,
            len: a_len,
            at: value_at,
        });
        // advance past value + padding to the 4-byte boundary
        at = value_at + ((a_len as usize + 3) & !3);
        if attrs.len() > 4096 {
            return None;
        }
    }
    Some(Stun {
        method: ((ty & 0x3E00) >> 2) | (ty & 0x00F0) >> 4 | (ty & 0x000F),
        class: (((ty >> 4) & 1) | ((ty >> 7) & 2)) as u8,
        len,
        transaction_id,
        attrs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 32];
        d[0..2].copy_from_slice(&0x0001u16.to_be_bytes());
        d[2..4].copy_from_slice(&12u16.to_be_bytes());
        d[4..8].copy_from_slice(&COOKIE.to_be_bytes());
        // attr: type 0x8022 (SOFTWARE), len 5 "hi!!!" + 3 pad
        d[20..22].copy_from_slice(&0x8022u16.to_be_bytes());
        d[22..24].copy_from_slice(&5u16.to_be_bytes());
        d[24..29].copy_from_slice(b"hi!!!");
        d
    }

    #[test]
    fn parses_message() {
        let s = parse(&fixture()).unwrap();
        assert_eq!(s.method, 1);
        assert_eq!(s.class, 0);
        assert_eq!(s.attrs.len(), 1);
        assert_eq!(s.attrs[0].ty, 0x8022);
        assert_eq!(s.attrs[0].len, 5);
        assert_eq!(s.attrs[0].at, 24);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 12]).is_none());
        let mut d = fixture();
        d[4] = 0x21; // cookie broken? 0x2112A442→ keep? set wrong
        d[7] = 0xFF;
        assert!(parse(&d).is_none());
        let mut d = fixture();
        d[0] = 0xC0; // top bits set
        assert!(parse(&d).is_none());
        let mut d = fixture();
        d[3] = 3; // len 0x0C → 3? sets len LSB to 3 → odd
        assert!(parse(&d).is_none());
    }
}
