//! Minimal reader for LLDP frames (IEEE 802.1AB): a TLV chain where each
//! header is `type:7 | len:9` — 7-bit type in the high bits, 9-bit length
//! in the low bits. Type 0 is the end-of-frame marker. Common types:
//! 1 Chassis ID, 2 Port ID, 3 TTL, 4 Port Description, 5 System Name,
//! 6 System Description, 8 Management Address.
//!
//! ```
//! use izanagi_kit::lldp::parse;
//!
//! // Chassis ID TLV (type 1, len 4) + end TLV
//! let d = [1u8 << 1, 4, 0xAA, 0xBB, 0xCC, 0xDD, 0, 0];
//! let f = parse(&d).unwrap();
//! assert_eq!(f.tlvs.len(), 1);
//! assert_eq!(f.tlvs[0].kind, 1);
//! ```

/// One TLV header.
#[derive(Debug)]
pub struct Tlv {
    /// 7-bit TLV type.
    pub kind: u8,
    /// Value byte length.
    pub len: u16,
    /// Byte offset of the value.
    pub value_at: usize,
}

/// A parsed LLDP frame.
#[derive(Debug)]
pub struct Lldp {
    /// TLV headers in wire order (the type-0 end TLV is not included).
    pub tlvs: Vec<Tlv>,
}

/// Parse an LLDP TLV chain. `None` on a truncated TLV header or value;
/// a missing end-of-frame marker is tolerated (input end implies it).
pub fn parse(d: &[u8]) -> Option<Lldp> {
    if d.is_empty() {
        return None;
    }
    let mut tlvs = Vec::new();
    let mut at = 0;
    while at < d.len() {
        let hi = *d.get(at)? as u16;
        let lo = *d.get(at + 1)? as u16;
        let head = (hi << 8) | lo;
        let kind = (head >> 9) as u8;
        let len = (head & 0x1FF) as usize;
        at += 2;
        if kind == 0 {
            break; // end marker
        }
        if at + len > d.len() {
            return None;
        }
        tlvs.push(Tlv {
            kind,
            len: len as u16,
            value_at: at,
        });
        at += len;
    }
    Some(Lldp { tlvs })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        // chassis(1,len 4) + port id(2,len 2) + ttl(3,len 2) + end
        let d = [
            1u8 << 1,
            4,
            0xAA,
            0xBB,
            0xCC,
            0xDD,
            2 << 1,
            2,
            b'e',
            b'1',
            3 << 1,
            2,
            0,
            120,
            0,
            0,
        ];
        let f = parse(&d).unwrap();
        assert_eq!(f.tlvs.len(), 3);
        assert_eq!(f.tlvs[0].kind, 1);
        assert_eq!(f.tlvs[1].kind, 2);
        assert_eq!(f.tlvs[2].len, 2);
        assert_eq!(f.tlvs[2].value_at, 12);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0x02]).is_none()); // truncated TLV header
                                           // TLV declares len 4 but only 1 value byte remains
        assert!(parse(&[2, 4, 0xAA]).is_none());
    }
}
