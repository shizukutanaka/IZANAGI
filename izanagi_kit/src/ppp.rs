//! Point-to-Point Protocol (RFC 1661) frame parser.
//!
//! A PPP frame is `[FF 03]? protocol:u16be info* [fcs:u16be]?`. The
//! `FF 03` address/control pair is present in async-style framing and
//! omitted under ACCM/address-control-field compression. Per RFC 1661
//! the two-octet protocol field has a zero low bit in its first octet
//! and an odd final octet; the optional 16-bit FCS cannot be verified
//! without CRC-16 tables, so it is reported as a flag only.
//!
//! ```
//! let f = b"\xff\x03\x00\x21hello";
//! let p = izanagi_kit::ppp::parse(f).unwrap();
//! assert_eq!(p.protocol, 0x0021); // IPv4
//! assert!(p.has_address_control);
//! assert_eq!(p.info_len, 5);
//! ```

/// Parsed PPP frame header.
#[derive(Debug, Clone, PartialEq)]
pub struct Ppp {
    /// 16-bit protocol identifier (e.g. `0x0021` IPv4, `0xC021` LCP).
    pub protocol: u16,
    /// Whether the leading `FF 03` address/control field is present.
    pub has_address_control: bool,
    /// Byte length of the information field (everything between the
    /// protocol field and any trailing FCS).
    pub info_len: usize,
    /// Whether the frame ends with a plausible FCS field. Heuristic:
    /// the frame has at least 4 bytes of info.
    pub has_fcs: bool,
    /// Total header size in bytes (address/control + protocol).
    pub header_len: usize,
}

/// Parse a PPP frame; `None` when too short or the protocol field is
/// malformed (low bit set, or the address/control is a stray `FF` pair
/// not followed by a sane protocol).
pub fn parse(d: &[u8]) -> Option<Ppp> {
    let mut off = 0usize;
    let has_ac = d.len() >= 2 && d[0] == 0xFF && d[1] == 0x03;
    if has_ac {
        off = 2;
    }
    if d.len() < off + 2 {
        return None;
    }
    // RFC 1661: in a two-octet protocol field the first octet's LSB
    // must be 0 and the last octet's LSB must be 1 (odd protocol ids
    // compress to a single byte when negotiated).
    if d[off] & 1 == 1 {
        return None;
    }
    let protocol = ((d[off] as u16) << 8) | d[off + 1] as u16;
    if protocol & 1 == 0 {
        return None;
    }
    let info_len = d.len() - off - 2;
    // An FCS is heuristically assumed when the payload is long enough
    // that the sender would have appended one; purely informational.
    let has_fcs = info_len >= 4;
    Some(Ppp {
        protocol,
        has_address_control: has_ac,
        info_len,
        has_fcs,
        header_len: off + 2,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let f = b"\xff\x03\x00\x21ipv4-data";
        let p = parse(f).unwrap();
        assert_eq!(p.protocol, 0x21);
        assert!(p.has_address_control);
        assert_eq!(p.info_len, 9);
        assert!(p.has_fcs);
        assert_eq!(p.header_len, 4);
    }

    #[test]
    fn compressed_acf() {
        let f = b"\xc0\x23\x01\x00"; // PAP LCP auth proto
        let p = parse(f).unwrap();
        assert_eq!(p.protocol, 0xC023);
        assert!(!p.has_address_control);
        assert_eq!(p.info_len, 2);
        assert!(!p.has_fcs);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"\xff").is_none());
        assert!(parse(b"\xff\x03").is_none());
        // first-octet LSB must be 0, last-octet LSB must be 1
        assert!(parse(b"\x01\x21xx").is_none());
        assert!(parse(b"\x00\x22xx").is_none());
    }
}
