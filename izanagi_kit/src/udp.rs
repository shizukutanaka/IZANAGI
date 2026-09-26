//! UDP datagram header (RFC 768).
//!
//! Eight bytes: source port, destination port, length (header +
//! data, minimum 8) and a checksum that is mandatory for IPv6 and
//! may be zero for IPv4. `payload` slices the datagram body when
//! the declared length fits the buffer.
//!
//! ```
//! use izanagi_kit::udp::{parse, HEADER};
//!
//! let mut d = vec![0u8; HEADER + 3];
//! let put = |d: &mut [u8], at: usize, v: u16| {
//!     d[at] = (v >> 8) as u8; d[at + 1] = v as u8;
//! };
//! put(&mut d, 0, 53_000);
//! put(&mut d, 2, 8080);
//! put(&mut d, 4, 11); // header + 3 payload bytes
//! let u = parse(&d).unwrap();
//! assert_eq!(u.src_port, 53_000);
//! assert_eq!(u.dst_port, 8080);
//! assert_eq!(u.payload(&d).unwrap().len(), 3);
//! ```

/// Header size in bytes.
pub const HEADER: usize = 8;

fn be16(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) << 8 | u16::from(*d.get(at + 1)?))
}

/// A parsed UDP header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Udp {
    /// Source port (0 = none for some sender protocols).
    pub src_port: u16,
    /// Destination port.
    pub dst_port: u16,
    /// Datagram length including this header.
    pub length: u16,
    /// Header+payload checksum (0 = not computed, IPv4 only).
    pub checksum: u16,
}

impl Udp {
    /// Payload length declared by the header.
    pub fn data_len(&self) -> usize {
        usize::from(self.length) - HEADER
    }
    /// Payload slice, `None` when the buffer is shorter than the
    /// declared datagram length.
    pub fn payload<'a>(&self, d: &'a [u8]) -> Option<&'a [u8]> {
        d.get(HEADER..usize::from(self.length))
    }
}

/// Parse a UDP header. Returns `None` when fewer than 8 bytes are
/// present or `length` is below the header size.
pub fn parse(d: &[u8]) -> Option<Udp> {
    if d.len() < HEADER {
        return None;
    }
    let length = be16(d, 4)?;
    if length < HEADER as u16 {
        return None;
    }
    Some(Udp {
        src_port: be16(d, 0)?,
        dst_port: be16(d, 2)?,
        length,
        checksum: be16(d, 6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 16];
        d[0] = 0x12;
        d[1] = 0x34;
        d[2] = 0xab;
        d[3] = 0xcd;
        d[5] = 16;
        d[6] = 0xde;
        d[7] = 0xad;
        d
    }

    #[test]
    fn fields_and_payload() {
        let d = fixture();
        let u = parse(&d).unwrap();
        assert_eq!(u.src_port, 0x1234);
        assert_eq!(u.dst_port, 0xabcd);
        assert_eq!(u.length, 16);
        assert_eq!(u.checksum, 0xdead);
        assert_eq!(u.data_len(), 8);
        assert_eq!(u.payload(&d).unwrap().len(), 8);
    }

    #[test]
    fn declared_length_over_buffer_is_none_payload() {
        let mut d = fixture();
        d[5] = 40; // claims 40 bytes, buffer is 16
        let u = parse(&d).unwrap();
        assert_eq!(u.data_len(), 32);
        assert!(u.payload(&d).is_none());
    }

    #[test]
    fn rejects_short_and_tiny_length() {
        assert!(parse(&[0u8; 7]).is_none());
        let mut d = fixture();
        d[5] = 7;
        assert!(parse(&d).is_none());
        d[5] = 8;
        assert!(parse(&d).is_some());
    }
}
