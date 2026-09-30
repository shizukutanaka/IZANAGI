//! GTP tunnel header parser (3GPP TS 29.060 v1 / TS 29.274 v2).
//!
//! GTPv1 (GTP-U/GTP-C, UDP 2152/2123):
//! `[flags][type u8][length u16][teid u32]` where `flags` is
//! `version(3) PT E S PN` — version field `001`. When E/S/PN are set a
//! 4-byte extension follows (`seq u16, npdu u8, next_ext u8`).
//!
//! GTPv2 (GTPv2-C, UDP 2123): `[flags][type][length u24][teid?][seq u24][spare]`
//! — version field `010`, `T` bit says whether TEID is present.
//!
//! ```
//! let f = b"\x30\xff\x00\x04\x01\x02\x03\x04data";
//! let g = izanagi_kit::gtp::parse(f).unwrap();
//! assert_eq!(g.version, 1);
//! assert_eq!(g.message_type, 255);
//! assert_eq!(g.teid, Some(0x01020304));
//! ```

/// Parsed GTP header (v1 or v2).
#[derive(Debug, Clone, PartialEq)]
pub struct Gtp {
    /// Protocol version: `1` (29.060) or `2` (29.274).
    pub version: u8,
    /// Message type byte (255 = G-PDU for v1).
    pub message_type: u8,
    /// TEID tunnel endpoint id when present (v1 always; v2 iff T bit).
    pub teid: Option<u32>,
    /// Declared payload length field (bytes after the fixed header).
    pub payload_len: u32,
    /// Byte length of the parsed header (fixed + optional fields).
    pub header_len: usize,
    /// v1: sequence-number-present flag / v2: spare bits echoed raw.
    pub extension: bool,
}

/// Parse a GTP header; `None` for other versions or truncation.
pub fn parse(d: &[u8]) -> Option<Gtp> {
    if d.len() < 4 {
        return None;
    }
    let flags = d[0];
    match flags >> 5 {
        1 => parse_v1(d),
        2 => parse_v2(d),
        _ => None,
    }
}

fn parse_v1(d: &[u8]) -> Option<Gtp> {
    if d.len() < 8 {
        return None;
    }
    let flags = d[0];
    if flags & 0x10 == 0 {
        return None; // PT must be 1 for GTP
    }
    let ext = flags & 0x07 != 0; // E|S|PN
    let message_type = d[1];
    let payload_len = ((d[2] as u32) << 8) | d[3] as u32;
    let teid = ((d[4] as u32) << 24) | ((d[5] as u32) << 16) | ((d[6] as u32) << 8) | d[7] as u32;
    let mut header = 8usize;
    if ext {
        if d.len() < header + 4 {
            return None;
        }
        header += 4;
    }
    Some(Gtp {
        version: 1,
        message_type,
        teid: Some(teid),
        payload_len,
        header_len: header,
        extension: ext,
    })
}

fn parse_v2(d: &[u8]) -> Option<Gtp> {
    // v2: `flags type len:u24 [teid:u32] seq:u24 spare:u8`
    if d.len() < 8 {
        return None;
    }
    let flags = d[0];
    let has_teid = flags & 0x08 != 0;
    let message_type = d[1];
    let payload_len = ((d[2] as u32) << 16) | ((d[3] as u32) << 8) | d[4] as u32;
    let mut off = 5usize;
    let teid = if has_teid {
        if d.len() < off + 4 {
            return None;
        }
        let t = ((d[off] as u32) << 24)
            | ((d[off + 1] as u32) << 16)
            | ((d[off + 2] as u32) << 8)
            | d[off + 3] as u32;
        off += 4;
        Some(t)
    } else {
        None
    };
    if d.len() < off + 4 {
        return None;
    }
    off += 4; // seq u24 + spare
    Some(Gtp {
        version: 2,
        message_type,
        teid,
        payload_len,
        header_len: off,
        extension: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_gpdu() {
        let f = b"\x30\xff\x00\x04\xde\xad\xbe\xefdata";
        let g = parse(f).unwrap();
        assert_eq!(g.version, 1);
        assert_eq!(g.message_type, 255);
        assert_eq!(g.teid, Some(0xDEADBEEF));
        assert_eq!(g.payload_len, 4);
        assert_eq!(g.header_len, 8);
        assert!(!g.extension);
    }

    #[test]
    fn v1_with_extension() {
        let f = b"\x36\xff\x00\x08\x00\x00\x00\x01\x12\x34\x00\x00xx";
        let g = parse(f).unwrap();
        assert!(g.extension);
        assert_eq!(g.header_len, 12);
    }

    #[test]
    fn v2() {
        // flags: version 2 <<5 = 0x40, +T bit
        let f = b"\x48\x20\x00\x00\x00\xaa\xbb\xcc\xdd\x00\x00\x01\x00";
        let g = parse(f).unwrap();
        assert_eq!(g.version, 2);
        assert_eq!(g.message_type, 0x20); // Create Session Request
        assert_eq!(g.teid, Some(0xAABBCCDD));
        assert_eq!(g.header_len, 13);
        // without T
        let g = parse(b"\x40\x20\x00\x00\x00\x00\x00\x01\x00").unwrap();
        assert!(g.teid.is_none());
        assert_eq!(g.header_len, 9);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"\x00").is_none());
        assert!(parse(b"\x60\xff\x00\x04xxxxxxxx").is_none()); // version 3
        assert!(parse(b"\x20\xff\x00\x04xxxxxxxx").is_none()); // v1 PT=0
        assert!(parse(b"\x30\xff").is_none());
    }
}
