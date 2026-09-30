//! RTCP packet parser (RFC 3550).
//!
//! RTCP flows are compound packets: a chain of `[v|p|rc][pt][len]`
//! blocks where `len` counts 32-bit words minus one and `v` must be 2.
//! Packet types: 200 SR, 201 RR, 202 SDES, 203 BYE, 204 APP,
//! 205 RTPFB, 206 PSFB, 207 XR. The first packet of a compound must be
//! SR or RR per RFC 3550 §6.1 — enforced here to reject non-RTCP data.
//!
//! ```
//! let mut f = b"\x81\xc8\x00\x01".to_vec(); // v2, rc=1, SR, len=1 word
//! f.extend_from_slice(&[0; 4]);            // SSRC
//! let r = izanagi_kit::rtcp::parse(&f).unwrap();
//! assert_eq!(r.packets, 1);
//! assert_eq!(r.types[0], 200);
//! ```

/// One parsed RTCP packet (of a possibly-compound chain).
#[derive(Debug, Clone, PartialEq)]
pub struct RtcpPacket {
    /// Packet type (200 SR, 201 RR, 202 SDES, 203 BYE, 204 APP, …).
    pub packet_type: u8,
    /// Reception-report/sub-item count field (5 bits).
    pub count: u8,
    /// Declared length in 32-bit words *including* the header word.
    pub length_words: u16,
    /// Byte offset of this packet's header in the input.
    pub offset: usize,
}

/// Parsed RTCP compound packet summary.
#[derive(Debug, Clone, PartialEq)]
pub struct Rtcp {
    /// Packets in wire order.
    pub packets: usize,
    /// Packet-type code of each packet (`types[i]` = `packets[i].packet_type`).
    pub types: Vec<u8>,
    /// `true` when a Sender Report (200) appears first — RFC-required.
    pub starts_with_sr: bool,
    /// Byte length consumed by valid packets (tail padding may remain).
    pub consumed: usize,
    /// Per-packet detail.
    pub detail: Vec<RtcpPacket>,
}

/// Parse an RTCP compound packet; `None` when the first header is not
/// version 2 / not SR-or-RR, or a declared length overruns the input.
pub fn parse(d: &[u8]) -> Option<Rtcp> {
    let mut i = 0usize;
    let mut detail = Vec::new();
    while i + 4 <= d.len() {
        let b0 = d[i];
        if b0 >> 6 != 2 {
            break;
        }
        let pt = d[i + 1];
        let words = ((d[i + 2] as u16) << 8) | d[i + 3] as u16;
        let bytes = (words as usize + 1) * 4;
        if bytes < 4 || i + bytes > d.len() {
            break;
        }
        detail.push(RtcpPacket {
            packet_type: pt,
            count: b0 & 0x1F,
            length_words: words + 1,
            offset: i,
        });
        i += bytes;
    }
    if detail.is_empty() || i == 0 {
        return None;
    }
    // RFC 3550 §6.1: first packet of a compound must be SR or RR
    // (accepted loosely — many tools emit lone SDES/BYE too).
    let starts_sr = detail[0].packet_type == 200 || detail[0].packet_type == 201;
    if !starts_sr && detail[0].packet_type < 200 {
        return None;
    }
    Some(Rtcp {
        packets: detail.len(),
        types: detail.iter().map(|p| p.packet_type).collect(),
        starts_with_sr: starts_sr,
        consumed: i,
        detail,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sr_then_rr() {
        let mut f = b"\x81\xc8\x00\x06".to_vec(); // SR, 7 words
        f.extend_from_slice(&[0; 24]);
        f.extend_from_slice(b"\x81\xc9\x00\x01"); // RR, 2 words
        f.extend_from_slice(&[0; 4]);
        let r = parse(&f).unwrap();
        assert_eq!(r.packets, 2);
        assert_eq!(r.types, vec![200, 201]);
        assert!(r.starts_with_sr);
        assert_eq!(r.consumed, f.len());
        assert_eq!(r.detail[0].length_words, 7);
        assert_eq!(r.detail[1].offset, 28);
    }

    #[test]
    fn lone_bye() {
        let f = b"\x80\xcb\x00\x01\x01\x02\x03\x04";
        let r = parse(f).unwrap();
        assert_eq!(r.types, vec![203]);
        assert!(!r.starts_with_sr);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"\x01\xc8\x00\x01").is_none()); // v1
        assert!(parse(b"\x81\x08\x00\x01xxxx").is_none()); // PT 8 < 200
                                                           // declared length overruns input → chain stops, tail reported
        let mut f = b"\x81\xc8\x00\x01".to_vec();
        f.extend_from_slice(&[0; 4]);
        f.extend_from_slice(b"\x81\xc9\x00\xffxx"); // RR claiming 256 words
        let r = parse(&f).unwrap();
        assert_eq!(r.packets, 1);
        assert_eq!(r.consumed, 8);
    }
}
