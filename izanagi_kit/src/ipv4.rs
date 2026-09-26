//! IPv4 datagram header (RFC 791).
//!
//! The 20-byte base header carries version/IHL nibbles, DSCP+ECN,
//! total length, identification, flags and fragment offset, TTL,
//! protocol, a one's-complement header checksum and the source and
//! destination addresses. `checksum_ok` folds the header words and
//! verifies the stored checksum.
//!
//! ```
//! use izanagi_kit::ipv4::{parse, HEADER};
//!
//! let mut d = vec![0u8; HEADER + 4];
//! d[0] = 0x45; // version 4, IHL 5
//! let put = |d: &mut [u8], at: usize, v: u16| {
//!     d[at] = (v >> 8) as u8; d[at + 1] = v as u8;
//! };
//! put(&mut d, 2, (HEADER + 4) as u16); // total length
//! d[8] = 64; d[9] = 6;                // ttl, TCP
//! d[12..16].copy_from_slice(&[192, 0, 2, 1]);
//! d[16..20].copy_from_slice(&[198, 51, 100, 2]);
//! // RFC 1071 checksum over the header
//! let mut sum = 0u32;
//! for w in d[..HEADER].chunks_exact(2) {
//!     sum = sum.wrapping_add(u32::from(w[0]) << 8 | u32::from(w[1]));
//! }
//! sum = (sum >> 16) + (sum & 0xffff);
//! sum += sum >> 16;
//! put(&mut d, 10, !(sum as u16));
//! let h = parse(&d).unwrap();
//! assert_eq!(h.ttl, 64);
//! assert_eq!(h.protocol, 6);
//! assert!(h.checksum_ok(&d));
//! ```

/// Minimum (and usual) header size in bytes.
pub const HEADER: usize = 20;

fn be16(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) << 8 | u16::from(*d.get(at + 1)?))
}
/// A parsed IPv4 header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ipv4 {
    /// Header length in bytes (IHL * 4, >= 20).
    pub ihl: usize,
    /// Differentiated services code point (top 6 bits of @1).
    pub dscp: u8,
    /// Explicit congestion notification (low 2 bits of @1).
    pub ecn: u8,
    /// Total datagram length including header.
    pub total_len: u16,
    /// Identification for fragment reassembly.
    pub id: u16,
    /// Don't-fragment flag.
    pub dont_fragment: bool,
    /// More-fragments flag.
    pub more_fragments: bool,
    /// Fragment offset in 8-byte units.
    pub fragment_offset: u16,
    /// Time to live.
    pub ttl: u8,
    /// Payload protocol number (6 TCP, 17 UDP, 1 ICMP).
    pub protocol: u8,
    /// Stored header checksum.
    pub checksum: u16,
    /// Source address.
    pub src: [u8; 4],
    /// Destination address.
    pub dst: [u8; 4],
    /// Header options bytes (usually empty).
    pub options: Vec<u8>,
    /// Offset of the payload.
    pub payload_at: usize,
}

impl Ipv4 {
    /// Verify the one's-complement header checksum: summing all
    /// 16-bit header words (including the checksum field) and
    /// folding carries must yield all-ones.
    pub fn checksum_ok(&self, d: &[u8]) -> bool {
        let hdr = match d.get(..self.ihl) {
            Some(h) => h,
            None => return false,
        };
        let mut sum = 0u32;
        for w in hdr.chunks(2) {
            let word = if w.len() == 2 {
                u32::from(w[0]) << 8 | u32::from(w[1])
            } else {
                u32::from(w[0]) << 8
            };
            sum = sum.wrapping_add(word);
        }
        sum = (sum >> 16) + (sum & 0xffff);
        sum += sum >> 16;
        sum == 0xffff
    }
}

/// Parse an IPv4 header. Returns `None` when truncated, the version
/// nibble is not 4, the IHL is below 20 bytes, or the datagram is
/// shorter than the header length.
pub fn parse(d: &[u8]) -> Option<Ipv4> {
    if d.len() < HEADER {
        return None;
    }
    let v0 = *d.first()?;
    if v0 >> 4 != 4 {
        return None;
    }
    let ihl = usize::from(v0 & 0x0f) * 4;
    if ihl < HEADER || d.len() < ihl {
        return None;
    }
    let flags_frag = be16(d, 6)?;
    let mut src = [0u8; 4];
    let mut dst = [0u8; 4];
    src.copy_from_slice(d.get(12..16)?);
    dst.copy_from_slice(d.get(16..20)?);
    Some(Ipv4 {
        ihl,
        dscp: d.get(1).copied()? >> 2,
        ecn: d.get(1).copied()? & 3,
        total_len: be16(d, 2)?,
        id: be16(d, 4)?,
        dont_fragment: flags_frag & 0x4000 != 0,
        more_fragments: flags_frag & 0x2000 != 0,
        fragment_offset: flags_frag & 0x1fff,
        ttl: d.get(8).copied()?,
        protocol: d.get(9).copied()?,
        checksum: be16(d, 10)?,
        src,
        dst,
        options: d.get(HEADER..ihl)?.to_vec(),
        payload_at: ihl,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    /// RFC 791 example header with options (the classic fixture).
    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; HEADER + 8];
        d[0] = 0x45;
        d[1] = 0x00;
        d[2] = 0;
        d[3] = HEADER as u8 + 8;
        d[4] = 0x1c;
        d[5] = 0x46;
        d[6] = 0x40; // DF set
        d[7] = 0x00;
        d[8] = 64;
        d[9] = 17; // UDP
        d[12..16].copy_from_slice(&[10, 0, 0, 1]);
        d[16..20].copy_from_slice(&[10, 0, 0, 2]);
        // compute checksum
        let mut sum = 0u32;
        for w in d[..HEADER].chunks_exact(2) {
            sum += u32::from(w[0]) << 8 | u32::from(w[1]);
        }
        sum = (sum >> 16) + (sum & 0xffff);
        sum += sum >> 16;
        let cks = !(sum as u16);
        d[10] = (cks >> 8) as u8;
        d[11] = cks as u8;
        d
    }

    #[test]
    fn fields_and_checksum() {
        let d = fixture();
        let h = parse(&d).unwrap();
        assert_eq!(h.ihl, 20);
        assert_eq!(h.total_len, 28);
        assert_eq!(h.id, 0x1c46);
        assert!(h.dont_fragment);
        assert!(!h.more_fragments);
        assert_eq!(h.fragment_offset, 0);
        assert_eq!(h.ttl, 64);
        assert_eq!(h.protocol, 17);
        assert_eq!(h.src, [10, 0, 0, 1]);
        assert_eq!(h.dst, [10, 0, 0, 2]);
        assert!(h.options.is_empty());
        assert!(h.checksum_ok(&d));
    }

    #[test]
    fn tampered_header_fails_checksum() {
        let mut d = fixture();
        d[9] = 6;
        let h = parse(&d).unwrap();
        assert!(!h.checksum_ok(&d));
    }

    #[test]
    fn options_extend_ihl() {
        let mut d = fixture();
        d[0] = 0x46; // IHL 6 -> 24 bytes
        d[3] = 24 + 8;
        d.splice(20..20, [1, 1, 0, 0]); // 4 option bytes
        let h = parse(&d).unwrap();
        assert_eq!(h.ihl, 24);
        assert_eq!(h.options, vec![1, 1, 0, 0]);
        assert_eq!(h.payload_at, 24);
    }

    #[test]
    fn rejects_bad_inputs() {
        assert!(parse(&[0u8; 19]).is_none());
        let mut v6 = fixture();
        v6[0] = 0x65;
        assert!(parse(&v6).is_none());
        let mut tiny_ihl = fixture();
        tiny_ihl[0] = 0x41; // IHL 1 -> 4 bytes
        assert!(parse(&tiny_ihl).is_none());
        assert!(parse(&fixture()[..19]).is_none());
    }
}
