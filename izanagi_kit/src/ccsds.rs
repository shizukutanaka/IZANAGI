//! CCSDS Space Packet Protocol (CCSDS 133.0-B) primary header parsing.
//!
//! Primary header (6 bytes, BE): `version(3)=0 | type(1) | sec_hdr(1) |
//! apid(11) | seq_flags(2) | seq_count(14) | len(16, = bytes after header - 1)`.
//!
//! ```
//! use izanagi_kit::ccsds::{parse, Kind};
//!
//! // version=0, TM, sec-hdr, apid=42, seq=first segment|count 7, len-1=9
//! let p = [0x08u8, 0x2A, 0xC0, 0x07, 0x00, 0x09, 0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
//! let s = parse(&p).unwrap();
//! assert_eq!(s.kind, Kind::Tm);
//! assert_eq!(s.apid, 42);
//! assert_eq!(s.seq_count, 7);
//! assert_eq!(s.data_len, 10);
//! ```

/// Packet type bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// 0 — telemetry.
    Tm,
    /// 1 — telecommand.
    Tc,
}

/// Sequence-flag values (2-bit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeqFlag {
    /// 0 — continuation segment.
    Continuation,
    /// 1 — first segment.
    First,
    /// 2 — last segment.
    Last,
    /// 3 — unsegmented.
    Unsegmented,
}

/// A parsed CCSDS packet header.
#[derive(Debug)]
pub struct Ccsds {
    /// TM/TC bit.
    pub kind: Kind,
    /// Secondary-header flag.
    pub sec_hdr: bool,
    /// Application process ID (11 bits).
    pub apid: u16,
    /// Sequence-flag field.
    pub seq_flag: SeqFlag,
    /// Packet sequence count (14 bits).
    pub seq_count: u16,
    /// Payload byte length (`len` field + 1).
    pub data_len: usize,
    /// Offset of the payload.
    pub data_at: usize,
}

/// Parses a CCSDS primary header; the payload must be fully present.
pub fn parse(d: &[u8]) -> Option<Ccsds> {
    if d.len() < 6 {
        return None;
    }
    let w0 = ((d[0] as u16) << 8) | d[1] as u16;
    let w1 = ((d[2] as u16) << 8) | d[3] as u16;
    let len = (((d[4] as u16) << 8) | d[5] as u16) as usize + 1;
    if w0 >> 13 != 0 {
        return None;
    }
    if 6 + len > d.len() {
        return None;
    }
    Some(Ccsds {
        kind: if w0 & 0x1000 != 0 { Kind::Tc } else { Kind::Tm },
        sec_hdr: w0 & 0x0800 != 0,
        apid: w0 & 0x07FF,
        seq_flag: match w1 >> 14 {
            0 => SeqFlag::Continuation,
            1 => SeqFlag::First,
            2 => SeqFlag::Last,
            _ => SeqFlag::Unsegmented,
        },
        seq_count: w1 & 0x3FFF,
        data_len: len,
        data_at: 6,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tm_packet() {
        let p = [
            0x08, 0x2A, 0x40, 0x00, 0x00, 0x02, 9, 9, 9, 0, 0, 0, 0, 0, 0,
        ];
        let s = parse(&p).unwrap();
        assert_eq!(s.kind, Kind::Tm);
        assert!(s.sec_hdr);
        assert_eq!(s.apid, 42);
        assert_eq!(s.seq_flag, SeqFlag::First);
        assert_eq!(s.seq_count, 0);
        assert_eq!(s.data_len, 3);
        assert_eq!(p[s.data_at], 9);
    }

    #[test]
    fn tc_packet() {
        // version 0, type 1(TC), no sec-hdr, apid 100
        let p = [0x10u8, 0x64, 0xC0, 0x01, 0x00, 0x00, 0xFF];
        let s = parse(&p).unwrap();
        assert_eq!(s.kind, Kind::Tc);
        assert!(!s.sec_hdr);
        assert_eq!(s.apid, 100);
        assert_eq!(s.data_len, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0; 5]).is_none());
        // version nonzero
        assert!(parse(&[0x20, 0, 0xC0, 0, 0, 0, 0]).is_none());
        // truncated payload
        assert!(parse(&[0x08, 0x2A, 0xC0, 0, 0, 9, 0]).is_none());
    }
}
