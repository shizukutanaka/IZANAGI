//! ICMPv4 message header (RFC 792).
//!
//! Every message starts with type, code and a one's-complement
//! checksum, then a 4-byte "rest of header" field whose meaning
//! depends on the type: echo messages split it into identifier and
//! sequence, redirects carry a gateway address, and errors place
//! the original datagram's header+64 bits in the payload.
//!
//! ```
//! use izanagi_kit::icmp::{parse, Kind, HEADER};
//!
//! let mut d = vec![0u8; HEADER + 4];
//! d[0] = 8; // echo request
//! d[4] = 0x12; d[5] = 0x34; // identifier
//! d[6] = 0; d[7] = 1;       // sequence
//! let m = parse(&d).unwrap();
//! assert_eq!(m.kind, Kind::EchoRequest);
//! assert_eq!(m.identifier(), Some(0x1234));
//! assert_eq!(m.sequence(), Some(1));
//! ```

/// Minimum message header size in bytes.
pub const HEADER: usize = 8;

fn be16(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) << 8 | u16::from(*d.get(at + 1)?))
}

/// Well-known ICMPv4 type numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Type 0 — ping reply.
    EchoReply,
    /// Type 3 — unreachable (`code` narrows to host/port/fragmentation...).
    DestUnreachable,
    /// Type 4 — congestion control (deprecated by RFC 6633).
    SourceQuench,
    /// Type 5 — redirect; rest-of-header is a gateway address.
    Redirect,
    /// Type 8 — ping request.
    EchoRequest,
    /// Type 9 — router advertisement.
    RouterAdvertisement,
    /// Type 10 — router solicitation.
    RouterSolicitation,
    /// Type 11 — TTL expired or reassembly timeout.
    TimeExceeded,
    /// Type 12 — malformed header; rest-of-header is a byte pointer.
    ParameterProblem,
    /// Type 13 — timestamp request.
    Timestamp,
    /// Type 14 — timestamp reply.
    TimestampReply,
    /// Any other type number.
    Other(u8),
}

/// A parsed ICMPv4 message header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Icmp {
    /// Message type classifier.
    pub kind: Kind,
    /// Raw type byte.
    pub type_: u8,
    /// Sub-code qualifying the type.
    pub code: u8,
    /// Stored checksum.
    pub checksum: u16,
    /// The type-dependent 32-bit rest-of-header field.
    pub rest: u32,
    /// Offset of the message body.
    pub payload_at: usize,
}

impl Icmp {
    /// Echo/timestamp identifier (top half of `rest`) for
    /// echo-style types, else `None`.
    pub fn identifier(&self) -> Option<u16> {
        match self.kind {
            Kind::EchoReply | Kind::EchoRequest | Kind::Timestamp | Kind::TimestampReply => {
                Some((self.rest >> 16) as u16)
            }
            _ => None,
        }
    }
    /// Echo/timestamp sequence number (low half of `rest`) for
    /// echo-style types, else `None`.
    pub fn sequence(&self) -> Option<u16> {
        self.identifier().map(|_| (self.rest & 0xffff) as u16)
    }
    /// Gateway address for Redirect messages, else `None`.
    pub fn gateway(&self) -> Option<[u8; 4]> {
        if self.kind == Kind::Redirect {
            Some([
                (self.rest >> 24) as u8,
                (self.rest >> 16) as u8,
                (self.rest >> 8) as u8,
                self.rest as u8,
            ])
        } else {
            None
        }
    }
}

/// Parse a message header. Returns `None` when fewer than 8 bytes
/// are present.
pub fn parse(d: &[u8]) -> Option<Icmp> {
    if d.len() < HEADER {
        return None;
    }
    let type_ = d.first().copied()?;
    let kind = match type_ {
        0 => Kind::EchoReply,
        3 => Kind::DestUnreachable,
        4 => Kind::SourceQuench,
        5 => Kind::Redirect,
        8 => Kind::EchoRequest,
        9 => Kind::RouterAdvertisement,
        10 => Kind::RouterSolicitation,
        11 => Kind::TimeExceeded,
        12 => Kind::ParameterProblem,
        13 => Kind::Timestamp,
        14 => Kind::TimestampReply,
        t => Kind::Other(t),
    };
    Some(Icmp {
        kind,
        type_,
        code: d.get(1).copied()?,
        checksum: be16(d, 2)?,
        rest: u32::from(*d.get(4)?) << 24
            | u32::from(*d.get(5)?) << 16
            | u32::from(*d.get(6)?) << 8
            | u32::from(*d.get(7)?),
        payload_at: HEADER,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn msg(type_: u8, code: u8, rest: u32) -> Vec<u8> {
        let mut d = vec![0u8; 12];
        d[0] = type_;
        d[1] = code;
        d[4] = (rest >> 24) as u8;
        d[5] = (rest >> 16) as u8;
        d[6] = (rest >> 8) as u8;
        d[7] = rest as u8;
        d
    }

    #[test]
    fn echo_request_id_seq() {
        let d = msg(8, 0, 0xabcd_0007);
        let m = parse(&d).unwrap();
        assert_eq!(m.kind, Kind::EchoRequest);
        assert_eq!(m.identifier(), Some(0xabcd));
        assert_eq!(m.sequence(), Some(7));
        assert!(m.gateway().is_none());
    }

    #[test]
    fn redirect_gateway() {
        let d = msg(5, 1, 0x0a00_0001);
        let m = parse(&d).unwrap();
        assert_eq!(m.kind, Kind::Redirect);
        assert_eq!(m.gateway(), Some([10, 0, 0, 1]));
        assert!(m.identifier().is_none());
    }

    #[test]
    fn unreachable_and_unknown() {
        let d = msg(3, 4, 0);
        let m = parse(&d).unwrap();
        assert_eq!(m.kind, Kind::DestUnreachable);
        assert_eq!(m.code, 4);
        let u = parse(&msg(42, 0, 0)).unwrap();
        assert_eq!(u.kind, Kind::Other(42));
    }

    #[test]
    fn short_rejects() {
        assert!(parse(&[0u8; 7]).is_none());
    }
}
