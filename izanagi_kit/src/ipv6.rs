//! IPv6 fixed header (RFC 8200).
//!
//! The header is always 40 bytes: version and traffic class plus a
//! 20-bit flow label in the first word, payload length, next-header
//! selector, hop limit, then the two 16-byte addresses. Extension
//! headers chain via the next-header field; `is_extension` lists
//! the registered extension types.
//!
//! ```
//! use izanagi_kit::ipv6::{parse, HEADER};
//!
//! let mut d = vec![0u8; HEADER];
//! d[0] = 0x60; // version 6
//! d[4] = 0; d[5] = 8;      // payload length
//! d[6] = 6; d[7] = 64;     // TCP, hop limit
//! for i in 0..16 { d[8 + i] = i as u8; d[24 + i] = 0xff - i as u8; }
//! let h = parse(&d).unwrap();
//! assert_eq!(h.src[0], 0);
//! assert_eq!(h.dst[15], 0xf0);
//! assert_eq!(h.payload_len, 8);
//! ```

/// Fixed header size in bytes.
pub const HEADER: usize = 40;

fn be16(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) << 8 | u16::from(*d.get(at + 1)?))
}

/// Registered next-header values that denote extension headers.
pub const EXTENSION_HEADERS: [u8; 6] = [0, 43, 44, 50, 51, 60];

/// A parsed IPv6 fixed header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ipv6 {
    /// Traffic class (8 bits).
    pub traffic_class: u8,
    /// Flow label (20 bits).
    pub flow_label: u32,
    /// Payload length in bytes (excludes this header).
    pub payload_len: u16,
    /// Next-header selector (extension header or L4 protocol).
    pub next_header: u8,
    /// Hop limit.
    pub hop_limit: u8,
    /// Source address.
    pub src: [u8; 16],
    /// Destination address.
    pub dst: [u8; 16],
}

impl Ipv6 {
    /// True when `next_header` names an extension header that must
    /// be walked before the upper-layer payload.
    pub fn is_extension(&self) -> bool {
        EXTENSION_HEADERS.contains(&self.next_header)
    }
}

/// Parse a fixed header. Returns `None` when fewer than 40 bytes
/// are present or the version nibble is not 6.
pub fn parse(d: &[u8]) -> Option<Ipv6> {
    if d.len() < HEADER {
        return None;
    }
    let b0 = *d.first()?;
    if b0 >> 4 != 6 {
        return None;
    }
    let b1 = d.get(1).copied()?;
    let mut src = [0u8; 16];
    let mut dst = [0u8; 16];
    src.copy_from_slice(d.get(8..24)?);
    dst.copy_from_slice(d.get(24..40)?);
    Some(Ipv6 {
        traffic_class: (b0 & 0x0f) << 4 | b1 >> 4,
        flow_label: u32::from(b1 & 0x0f) << 16
            | u32::from(d.get(2).copied()?) << 8
            | u32::from(d.get(3).copied()?),
        payload_len: be16(d, 4)?,
        next_header: d.get(6).copied()?,
        hop_limit: d.get(7).copied()?,
        src,
        dst,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; HEADER + 4];
        d[0] = 0x6f; // v6, tc high nibble f
        d[1] = 0x8a; // tc low 8, flow label high 4 = a
        d[2] = 0xbc;
        d[3] = 0xde; // flow label 0xabcde
        d[5] = 0x20; // payload 32
        d[6] = 17; // UDP
        d[7] = 3;
        d[8] = 0x20;
        d[9] = 0x01;
        d[39] = 0x01;
        d
    }

    #[test]
    fn fields_decode() {
        let d = fixture();
        let h = parse(&d).unwrap();
        assert_eq!(h.traffic_class, 0xf8);
        assert_eq!(h.flow_label, 0xabcde);
        assert_eq!(h.payload_len, 0x20);
        assert_eq!(h.next_header, 17);
        assert_eq!(h.hop_limit, 3);
        assert_eq!(&h.src[..2], &[0x20, 0x01]);
        assert_eq!(h.dst[15], 0x01);
        assert!(!h.is_extension());
    }

    #[test]
    fn extension_header_detected() {
        let mut d = fixture();
        d[6] = 0; // hop-by-hop options
        let h = parse(&d).unwrap();
        assert!(h.is_extension());
        d[6] = 59; // no next header -> not an extension header to walk
        let h2 = parse(&d).unwrap();
        assert!(!h2.is_extension());
    }

    #[test]
    fn rejects_short_and_wrong_version() {
        assert!(parse(&[0u8; 39]).is_none());
        let mut v4 = fixture();
        v4[0] = 0x45;
        assert!(parse(&v4).is_none());
    }
}
