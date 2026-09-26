//! TCP segment header (RFC 793/3168).
//!
//! The base header is 20 bytes: ports, sequence and acknowledgment
//! numbers, a 4-bit data offset (in 32-bit words) that also covers
//! options, the nine control flags (NS through FIN), window,
//! checksum and urgent pointer. `FLAG_*` constants index into the
//! combined 9-bit `flags` field.
//!
//! ```
//! use izanagi_kit::tcp::{parse, FLAG_ACK, FLAG_SYN, HEADER};
//!
//! let mut d = vec![0u8; HEADER];
//! let put = |d: &mut [u8], at: usize, v: u16| {
//!     d[at] = (v >> 8) as u8; d[at + 1] = v as u8;
//! };
//! put(&mut d, 0, 49_152); put(&mut d, 2, 80);
//! d[12] = 0x50; // data offset 5 -> 20 bytes
//! d[13] = 0x12; // SYN+ACK
//! put(&mut d, 14, 65_535);
//! let t = parse(&d).unwrap();
//! assert_eq!(t.dst_port, 80);
//! assert!(t.has(FLAG_SYN) && t.has(FLAG_ACK));
//! ```

/// Minimum header size in bytes.
pub const HEADER: usize = 20;
/// Flag: nonce concealment protection (RFC 3540).
pub const FLAG_NS: u16 = 0x100;
/// Flag: congestion window reduced.
pub const FLAG_CWR: u16 = 0x80;
/// Flag: ECN echo.
pub const FLAG_ECE: u16 = 0x40;
/// Flag: urgent pointer field valid.
pub const FLAG_URG: u16 = 0x20;
/// Flag: acknowledgment field valid.
pub const FLAG_ACK: u16 = 0x10;
/// Flag: push buffered data.
pub const FLAG_PSH: u16 = 0x08;
/// Flag: reset the connection.
pub const FLAG_RST: u16 = 0x04;
/// Flag: synchronize sequence numbers.
pub const FLAG_SYN: u16 = 0x02;
/// Flag: finish sending.
pub const FLAG_FIN: u16 = 0x01;

fn be16(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) << 8 | u16::from(*d.get(at + 1)?))
}
fn be32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?) << 24
            | u32::from(*d.get(at + 1)?) << 16
            | u32::from(*d.get(at + 2)?) << 8
            | u32::from(*d.get(at + 3)?),
    )
}

/// A parsed TCP header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tcp {
    /// Source port.
    pub src_port: u16,
    /// Destination port.
    pub dst_port: u16,
    /// Sequence number.
    pub seq: u32,
    /// Acknowledgment number.
    pub ack: u32,
    /// Header size in bytes (data offset * 4, >= 20).
    pub header_len: usize,
    /// Nine control flags (NS..FIN).
    pub flags: u16,
    /// Receive window.
    pub window: u16,
    /// Header+payload checksum.
    pub checksum: u16,
    /// Urgent pointer.
    pub urgent: u16,
    /// Options bytes between the fixed header and payload.
    pub options: Vec<u8>,
    /// Offset of the payload.
    pub payload_at: usize,
}

impl Tcp {
    /// True when `flag` (a `FLAG_*` mask) is set.
    pub fn has(&self, flag: u16) -> bool {
        self.flags & flag != 0
    }
}

/// Parse a TCP header. Returns `None` when truncated, the data
/// offset is below 20 bytes, or the segment is shorter than the
/// declared header.
pub fn parse(d: &[u8]) -> Option<Tcp> {
    if d.len() < HEADER {
        return None;
    }
    let b12 = d.get(12).copied()?;
    let header_len = usize::from(b12 >> 4) * 4;
    if header_len < HEADER || d.len() < header_len {
        return None;
    }
    Some(Tcp {
        src_port: be16(d, 0)?,
        dst_port: be16(d, 2)?,
        seq: be32(d, 4)?,
        ack: be32(d, 8)?,
        header_len,
        flags: u16::from(b12 & 1) << 8 | u16::from(d.get(13).copied()?),
        window: be16(d, 14)?,
        checksum: be16(d, 16)?,
        urgent: be16(d, 18)?,
        options: d.get(HEADER..header_len)?.to_vec(),
        payload_at: header_len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 24];
        d[0] = 0xc0;
        d[1] = 0x00;
        d[2] = 0x00;
        d[3] = 0x50; // dst 80
        d[4] = 0xde;
        d[5] = 0xad;
        d[6] = 0xbe;
        d[7] = 0xef; // seq
        d[8] = 0x00;
        d[9] = 0x00;
        d[10] = 0x10;
        d[11] = 0x00; // ack 4096
        d[12] = 0x50;
        d[13] = 0x12; // SYN+ACK
        d[14] = 0xff;
        d[15] = 0xff;
        d
    }

    #[test]
    fn fields_and_flags() {
        let d = fixture();
        let t = parse(&d).unwrap();
        assert_eq!(t.src_port, 0xc000);
        assert_eq!(t.dst_port, 80);
        assert_eq!(t.seq, 0xdead_beef);
        assert_eq!(t.ack, 0x0000_1000);
        assert_eq!(t.header_len, 20);
        assert!(t.has(FLAG_SYN) && t.has(FLAG_ACK));
        assert!(!t.has(FLAG_FIN) && !t.has(FLAG_RST));
        assert_eq!(t.window, 0xffff);
        assert_eq!(t.payload_at, 20);
    }

    #[test]
    fn options_and_ns_flag() {
        let mut d = fixture();
        d[12] = 0x61; // offset 6 (24B), NS set
        d.splice(20..20, [0x02, 0x04, 0x05, 0xb4]); // MSS option
        let t = parse(&d).unwrap();
        assert_eq!(t.header_len, 24);
        assert_eq!(t.options, vec![0x02, 0x04, 0x05, 0xb4]);
        assert!(t.has(FLAG_NS));
        assert_eq!(t.payload_at, 24);
    }

    #[test]
    fn rejects_short_and_small_offset() {
        assert!(parse(&[0u8; 19]).is_none());
        let mut d = fixture();
        d[12] = 0x40; // offset 4 -> 16 bytes < HEADER
        assert!(parse(&d).is_none());
    }
}
