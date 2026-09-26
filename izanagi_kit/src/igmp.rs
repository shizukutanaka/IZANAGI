//! Minimal reader for IGMP messages (RFC 1112/2236/3376): `type` +
//! `max_resp` + `checksum` + `group_addr`. Types: 0x11 Membership
//! Query (v3 carries extra fields: `s_flag`+`qrv`, `qqic`, source
//! count + source list), 0x12 v1 report, 0x16 v2 report, 0x17 v2
//! leave, 0x22 v3 report (`resv`, `num_groups` + group records).
//!
//! The RFC 1071 checksum is verified over the received bytes (a correct
//! packet sums to 0).
//!
//! ```
//! use izanagi_kit::igmp::{parse, Kind};
//!
//! // v2 Membership Report for 239.1.2.3 with zeroed checksum (skipped)
//! let d = [0x16u8, 0, 0, 0, 239, 1, 2, 3];
//! let m = parse(&d).unwrap();
//! assert_eq!(m.kind, Kind::V2Report);
//! assert_eq!(m.group, [239, 1, 2, 3]);
//! ```

/// IGMP message type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `0x11` — Membership Query (v1/v2 8 bytes, v3 longer)
    Query,
    /// `0x12` — v1 Membership Report
    V1Report,
    /// `0x16` — v2 Membership Report
    V2Report,
    /// `0x17` — v2 Leave Group
    Leave,
    /// `0x22` — v3 Membership Report
    V3Report,
}

/// A parsed IGMP message.
#[derive(Debug)]
pub struct Igmp {
    /// Message type.
    pub kind: Kind,
    /// `max_resp` / `max_resp_code` — deciseconds (v2+) or exponential
    /// code (v3, high bit set = floating form).
    pub max_resp: u8,
    /// Declared checksum field.
    pub checksum: u16,
    /// `true` when the RFC 1071 checksum over the whole message is
    /// correct (zero result). `checksum == 0` is reported as `None`-
    /// equivalent: a sender that doesn't checksum puts 0, accepted.
    pub checksum_ok: bool,
    /// `group_addr` (query/report) or 0.0.0.0 for v3 reports.
    pub group: [u8; 4],
    /// v3 query only: `s_flag`.
    pub s_flag: Option<bool>,
    /// v3 query only: `qrv` (querier robustness variable, 3 bits).
    pub qrv: Option<u8>,
    /// v3 query only: `qqic`.
    pub qqic: Option<u8>,
    /// v3 query only: source addresses.
    pub sources: Vec<[u8; 4]>,
    /// v3 report only: `num_groups`.
    pub num_groups: Option<u16>,
}

fn be16(d: &[u8], at: usize) -> Option<u16> {
    let hi = *d.get(at)? as u16;
    let lo = *d.get(at + 1)? as u16;
    Some((hi << 8) | lo)
}

/// RFC 1071 ones-complement sum; 0 means verified.
fn cksum(d: &[u8]) -> u16 {
    let mut acc: u32 = 0;
    let mut at = 0;
    while at + 1 < d.len() {
        acc += (d[at] as u32) << 8 | d[at + 1] as u32;
        acc = (acc & 0xFFFF) + (acc >> 16);
        at += 2;
    }
    if at < d.len() {
        acc += (d[at] as u32) << 8;
        acc = (acc & 0xFFFF) + (acc >> 16);
    }
    acc = (acc & 0xFFFF) + (acc >> 16);
    !acc as u16
}

/// Parse one IGMP message. `None` on short input or unknown type.
/// `checksum_ok` reports the RFC 1071 fold over the wire bytes; a zero
/// checksum field is treated as "not present" (v1 senders put 0).
pub fn parse(d: &[u8]) -> Option<Igmp> {
    if d.len() < 8 {
        return None;
    }
    let kind = match d[0] {
        0x11 => Kind::Query,
        0x12 => Kind::V1Report,
        0x16 => Kind::V2Report,
        0x17 => Kind::Leave,
        0x22 => Kind::V3Report,
        _ => return None,
    };
    let checksum = be16(d, 2)?;
    let mut m = Igmp {
        kind,
        max_resp: d[1],
        checksum,
        checksum_ok: checksum == 0 || cksum(d) == 0,
        group: [d[4], d[5], d[6], d[7]],
        s_flag: None,
        qrv: None,
        qqic: None,
        sources: Vec::new(),
        num_groups: None,
    };
    match kind {
        Kind::Query if d.len() >= 12 => {
            // v3 query: s/qrv/qqic + num_sources + sources
            m.s_flag = Some(d[8] & 0x08 != 0);
            m.qrv = Some(d[8] & 0x07);
            m.qqic = Some(d[9]);
            let num = be16(d, 10)? as usize;
            if d.len() < 12 + num * 4 {
                return None;
            }
            for i in 0..num {
                let a = 12 + i * 4;
                m.sources.push([d[a], d[a + 1], d[a + 2], d[a + 3]]);
            }
        }
        Kind::V3Report => {
            // resv(1) resv(1) num_groups(2) then records — we only count
            m.num_groups = be16(d, 6);
        }
        _ => {}
    }
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_checksum(mut d: Vec<u8>) -> Vec<u8> {
        // compute RFC 1071 checksum over the packet with cksum field = 0
        d[2] = 0;
        d[3] = 0;
        let c = cksum(&d);
        d[2] = (c >> 8) as u8;
        d[3] = (c & 0xFF) as u8;
        d
    }

    #[test]
    fn v2_report() {
        let d = with_checksum(vec![0x16, 0, 0, 0, 239, 1, 2, 3]);
        let m = parse(&d).unwrap();
        assert_eq!(m.kind, Kind::V2Report);
        assert_eq!(m.group, [239, 1, 2, 3]);
        assert!(m.checksum_ok);
    }

    #[test]
    fn zero_checksum_accepted() {
        let d = [0x16u8, 0, 0, 0, 239, 1, 2, 3];
        let m = parse(&d).unwrap();
        assert!(m.checksum_ok); // 0 = not checksummed
    }

    #[test]
    fn bad_checksum() {
        let mut d = vec![0x16u8, 0, 0xFF, 0xFF, 239, 1, 2, 3];
        d = with_checksum(d);
        d[7] ^= 0xFF; // corrupt payload
        let m = parse(&d).unwrap();
        assert!(!m.checksum_ok);
    }

    #[test]
    fn v3_query() {
        let mut d = vec![0x11u8, 10, 0, 0, 239, 1, 2, 3, 0x09, 60, 0, 1, 192, 0, 2, 1];
        d = with_checksum(d);
        let m = parse(&d).unwrap();
        assert_eq!(m.kind, Kind::Query);
        assert_eq!(m.s_flag, Some(true));
        assert_eq!(m.qrv, Some(1));
        assert_eq!(m.qqic, Some(60));
        assert_eq!(m.sources, vec![[192, 0, 2, 1]]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0x99u8; 8]).is_none()); // unknown type
        assert!(parse(&[0x16u8; 7]).is_none()); // short
    }
}
