//! SNMP messages (RFC 1157 v1 / RFC 3416 v2c / RFC 3412 v3 framing):
//! BER `SEQUENCE` of `{ INTEGER version, OCTET STRING community,
//! PDU }` for v1/v2c; v3 carries an `INTEGER` msgID and headerData
//! sequence instead of a community. BER lengths are short-form or
//! `0x80+` long-form; indefinite lengths are rejected.
//!
//! ```
//! // SEQUENCE { INTEGER 1, OCTETSTRING "public", GetRequest … }
//! let d = [
//!     0x30u8, 0x0d, // seq len 13
//!     0x02, 0x01, 0x01, // INTEGER v2c
//!     0x04, 0x06, b'p', b'u', b'b', b'l', b'i', b'c',
//!     0xa0, 0x00, // GetRequest, empty
//! ];
//! let s = izanagi_kit::snmp::parse(&d).unwrap();
//! assert_eq!(s.version, izanagi_kit::snmp::Version::V2c);
//! assert_eq!(s.community.as_deref(), Some("public"));
//! assert_eq!(s.pdu, izanagi_kit::snmp::Pdu::GetRequest);
//! ```

use std::string::String;

/// SNMP version integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    /// `0` — SNMPv1
    V1,
    /// `1` — SNMPv2c
    V2c,
    /// `3` — SNMPv3
    V3,
    /// Other version number.
    Other(u64),
}

/// SNMPv1/v2c PDU tag (context-specific constructed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pdu {
    /// `0xA0` — GetRequest
    GetRequest,
    /// `0xA1` — GetNextRequest
    GetNextRequest,
    /// `0xA2` — GetResponse
    GetResponse,
    /// `0xA3` — SetRequest
    SetRequest,
    /// `0xA4` — v1 Trap
    Trap,
    /// `0xA5` — GetBulkRequest
    GetBulk,
    /// `0xA6` — InformRequest
    Inform,
    /// `0xA7` — SNMPv2-Trap
    TrapV2,
    /// `0xA8` — Report
    Report,
    /// Any other PDU tag.
    Other(u8),
}

impl Pdu {
    /// Map a context PDU tag byte to a `Pdu`.
    pub fn from_u8(t: u8) -> Pdu {
        match t {
            0xa0 => Pdu::GetRequest,
            0xa1 => Pdu::GetNextRequest,
            0xa2 => Pdu::GetResponse,
            0xa3 => Pdu::SetRequest,
            0xa4 => Pdu::Trap,
            0xa5 => Pdu::GetBulk,
            0xa6 => Pdu::Inform,
            0xa7 => Pdu::TrapV2,
            0xa8 => Pdu::Report,
            other => Pdu::Other(other),
        }
    }
}

/// A parsed SNMP message (top-level only; varbinds stay opaque).
#[derive(Clone, Debug)]
pub struct Snmp {
    /// Version.
    pub version: Version,
    /// Community string (v1/v2c); `None` for v3 (uses msgSecurity).
    pub community: Option<String>,
    /// PDU tag.
    pub pdu: Pdu,
    /// Byte length of the outer SEQUENCE content.
    pub seq_len: usize,
    /// PDU content region inside the buffer (`offset`, `len`).
    pub pdu_body: (usize, usize),
}

/// Minimal BER TLV reader — returns `(tag, content_start, content_len,
/// next_offset)`; rejects indefinite (`0x80`) lengths.
fn tlv(d: &[u8], off: usize) -> Option<(u8, usize, usize, usize)> {
    let tag = *d.get(off)?;
    let mut i = off + 1;
    let mut len = usize::from(*d.get(i)?);
    i += 1;
    if len & 0x80 != 0 {
        let n = len & 0x7f;
        if n == 0 || n > 8 || i + n > d.len() {
            return None;
        }
        let mut v = 0usize;
        for _ in 0..n {
            v = (v << 8) | usize::from(*d.get(i)?);
            i += 1;
        }
        len = v;
    }
    let end = i.checked_add(len)?;
    if end > d.len() {
        return None;
    }
    Some((tag, i, len, end))
}

/// Parse an SNMP message: outer SEQUENCE → INTEGER version → OCTET
/// STRING community (v1/v2c) or INTEGER/SEQUENCE (v3) → context PDU.
/// Returns `None` on malformed BER.
pub fn parse(d: &[u8]) -> Option<Snmp> {
    let (tag, start, len, end) = tlv(d, 0)?;
    if tag != 0x30 {
        return None;
    }
    let body = d.get(start..end)?;
    // version INTEGER
    let (t, vs, vl, off2) = tlv(body, 0)?;
    if t != 0x02 || vl > 8 || vl == 0 {
        return None;
    }
    let mut vnum = 0u64;
    for i in vs..vs + vl {
        vnum = (vnum << 8) | u64::from(*body.get(i)?);
    }
    let version = match vnum {
        0 => Version::V1,
        1 => Version::V2c,
        3 => Version::V3,
        other => Version::Other(other),
    };
    // next field: community (v1/v2c) or globalData seq (v3)
    let (t2, cs, cl, after2) = tlv(body, off2)?;
    let community = if t2 == 0x04 {
        Some(String::from(
            std::str::from_utf8(body.get(cs..cs + cl)?).ok()?,
        ))
    } else {
        None
    };
    // PDU
    let (pt, ps, pl, _) = tlv(body, after2)?;
    if pt & 0xc0 != 0x80 {
        return None; // context-specific class
    }
    Some(Snmp {
        version,
        community,
        pdu: Pdu::from_u8(pt),
        seq_len: len,
        pdu_body: (start + ps, pl),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v2c(comm: &str, pdu_tag: u8) -> Vec<u8> {
        let mut inner: Vec<u8> = Vec::new();
        inner.extend_from_slice(&[0x02, 0x01, 0x01]);
        inner.push(0x04);
        inner.push(comm.len() as u8);
        inner.extend_from_slice(comm.as_bytes());
        inner.extend_from_slice(&[pdu_tag, 0x00]);
        let mut d = vec![0x30u8, inner.len() as u8];
        d.extend_from_slice(&inner);
        d
    }

    #[test]
    fn get_request() {
        let s = parse(&v2c("public", 0xa0)).unwrap();
        assert_eq!(s.version, Version::V2c);
        assert_eq!(s.community.as_deref(), Some("public"));
        assert_eq!(s.pdu, Pdu::GetRequest);
    }

    #[test]
    fn v1_trap() {
        let mut inner: Vec<u8> = Vec::new();
        inner.extend_from_slice(&[0x02, 0x01, 0x00]);
        inner.extend_from_slice(&[0x04, 0x04, b'x', b'x', b'x', b'x']);
        inner.extend_from_slice(&[0xa4, 0x00]);
        let mut d = vec![0x30u8, inner.len() as u8];
        d.extend_from_slice(&inner);
        let s = parse(&d).unwrap();
        assert_eq!(s.version, Version::V1);
        assert_eq!(s.pdu, Pdu::Trap);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0x30u8]).is_none());
        // not a sequence
        assert!(parse(&[0x04u8, 0x01, 0x00]).is_none());
        // sequence containing non-INTEGER
        assert!(parse(&[0x30, 0x03, 0x04, 0x01, 0x00]).is_none());
        assert_eq!(Pdu::from_u8(0xa5), Pdu::GetBulk);
        assert_eq!(Pdu::from_u8(0xff), Pdu::Other(0xff));
    }
}
