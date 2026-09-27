//! Apache Thrift TBinaryProtocol / TFramedTransport envelope
//! scanning (Thrift IDL spec + `TBinaryProtocol` wire layout).
//!
//! A strict binary message header is `0x80010000 | type` (i32 BE,
//! sign bit set), then a method-name `string` (i32 BE len + bytes),
//! then a 4-byte seqid. Unversioned messages start directly with a
//! name length. `TFramedTransport` wraps each message in a
//! big-endian u32 frame size.
//!
//! ```
//! use izanagi_kit::thrift::{parse, frame, name, Kind};
//! let mut d = vec![0x80, 0x01, 0x00, 0x01]; // strict, CALL
//! d.extend_from_slice(&[0, 0, 0, 4]);
//! d.extend_from_slice(b"ping");
//! d.extend_from_slice(&[0, 0, 0, 7]); // seqid 7
//! let t = parse(&d).unwrap();
//! assert_eq!(t.kind, Kind::Call);
//! assert_eq!(name(&d, &t), b"ping");
//! assert_eq!(t.seqid, 7);
//! assert_eq!(frame(&d[..4]), None); // no frame — that's the message
//! ```

/// Message type of a strict TBinaryProtocol header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `T_CALL` (1).
    Call,
    /// `T_REPLY` (2).
    Reply,
    /// `T_EXCEPTION` (3).
    Exception,
    /// `T_ONEWAY` (4).
    Oneway,
    /// Any other low byte.
    Other(u8),
}

/// A parsed strict or unversioned binary message header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Thrift {
    /// Message kind (`Other(0)` when unversioned — no kind byte
    /// exists there).
    pub kind: Kind,
    /// `true` when the strict `0x8001` version word was present.
    pub strict: bool,
    /// Offset of the method name bytes.
    pub name_at: usize,
    /// Method name length.
    pub name_len: usize,
    /// Sequence id.
    pub seqid: u32,
    /// Offset just past the header (start of the field list).
    pub fields_at: usize,
}

fn be32(d: &[u8], at: usize) -> Option<u32> {
    let b: [u8; 4] = d.get(at..at + 4)?.try_into().ok()?;
    Some(u32::from_le_bytes([b[3], b[2], b[1], b[0]]))
}

/// Decode a `Kind` from the low byte of a version|type word.
pub fn kind_of(lo: u8) -> Kind {
    match lo {
        1 => Kind::Call,
        2 => Kind::Reply,
        3 => Kind::Exception,
        4 => Kind::Oneway,
        n => Kind::Other(n),
    }
}

/// Parse a TBinaryProtocol message header (strict or unversioned).
/// `None` on malformed input or a name running past the buffer.
pub fn parse(d: &[u8]) -> Option<Thrift> {
    let w0 = be32(d, 0)?;
    if w0 & 0x8000_0000 != 0 {
        // strict: version mask is 0xFFFF0000, required value 0x80010000
        if w0 & 0xFFFF_0000 != 0x8001_0000 {
            return None;
        }
        let nl = be32(d, 4)? as usize;
        let name_at = 8usize;
        let name_end = name_at.checked_add(nl)?;
        if name_end > d.len() {
            return None;
        }
        Some(Thrift {
            kind: kind_of((w0 & 0xFF) as u8),
            strict: true,
            name_at,
            name_len: nl,
            seqid: be32(d, name_end)?,
            fields_at: name_end + 4,
        })
    } else {
        // unversioned: w0 IS the name length
        let nl = w0 as usize;
        if nl == 0 || nl > 0x0FFF_FFFF {
            return None;
        }
        let name_at = 4usize;
        let name_end = name_at.checked_add(nl)?;
        if name_end > d.len() {
            return None;
        }
        Some(Thrift {
            kind: Kind::Other(0),
            strict: false,
            name_at,
            name_len: nl,
            seqid: be32(d, name_end)?,
            fields_at: name_end + 4,
        })
    }
}

/// Read a `TFramedTransport` frame: the leading u32 BE byte size
/// and the offset of the framed message (a TBinaryProtocol one).
/// `None` when the frame would run past the buffer.
pub fn frame(d: &[u8]) -> Option<(u32, usize)> {
    let n = be32(d, 0)?;
    if 4 + n as usize > d.len() {
        return None;
    }
    Some((n, 4))
}

/// The method name bytes of a parsed header.
pub fn name<'a>(d: &'a [u8], t: &Thrift) -> &'a [u8] {
    &d[t.name_at..t.name_at + t.name_len]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strict_fixture() -> Vec<u8> {
        let mut d = vec![0x80, 0x01, 0x00, 0x01];
        d.extend_from_slice(&[0, 0, 0, 4]);
        d.extend_from_slice(b"ping");
        d.extend_from_slice(&[0, 0, 0, 7]);
        d.push(0x00); // STOP field
        d
    }

    #[test]
    fn strict_header() {
        let d = strict_fixture();
        let t = parse(&d).unwrap();
        assert_eq!(t.kind, Kind::Call);
        assert!(t.strict);
        assert_eq!(name(&d, &t), b"ping");
        assert_eq!(t.seqid, 7);
        assert_eq!(t.fields_at, 16);
        assert_eq!(d[t.fields_at], 0); // STOP
    }

    #[test]
    fn kind_table() {
        assert_eq!(kind_of(1), Kind::Call);
        assert_eq!(kind_of(2), Kind::Reply);
        assert_eq!(kind_of(3), Kind::Exception);
        assert_eq!(kind_of(4), Kind::Oneway);
        assert_eq!(kind_of(9), Kind::Other(9));
    }

    #[test]
    fn framed() {
        let body = strict_fixture();
        let mut d = vec![0, 0, 0, body.len() as u8];
        d.extend_from_slice(&body);
        let (n, at) = frame(&d).unwrap();
        assert_eq!(n as usize, body.len());
        assert!(parse(&d[at..]).is_some());
        let short = [0, 0, 0, 21, 0, 0];
        assert_eq!(frame(&short), None);
        assert!(frame(b"").is_none());
    }

    #[test]
    fn unversioned() {
        let mut d = vec![0, 0, 0, 4];
        d.extend_from_slice(b"pong");
        d.extend_from_slice(&[0, 0, 0, 1]);
        let t = parse(&d).unwrap();
        assert!(!t.strict);
        assert_eq!(t.kind, Kind::Other(0));
        assert_eq!(name(&d, &t), b"pong");
        assert_eq!(t.seqid, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"x").is_none());
        let bad = [0x80, 0x02, 0x00, 0x01, 0, 0, 0, 1, b'x', 0, 0, 0, 0];
        assert!(parse(&bad).is_none()); // version mask 0x8002 wrong
        let mut d = strict_fixture();
        d.truncate(10); // name cut short
        assert!(parse(&d).is_none());
        let empty_name = [0x80, 0x01, 0x00, 0x01, 0, 0, 0, 0, 0, 0, 0, 9];
        assert_eq!(parse(&empty_name).unwrap().name_len, 0);
    }
}
