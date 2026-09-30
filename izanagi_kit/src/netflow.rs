//! NetFlow export (Cisco NetFlow v5 RFC-style / v9 RFC 3954):
//! v5 header is 24 bytes — `version`, `count`, `sys_uptime`,
//! `unix_secs`, `unix_nsecs`, `flow_seq`, `engine_type`, `engine_id`,
//! `sampling` — followed by `count` × 48-byte flow records. v9 uses a
//! 20-byte header with `count` counting FlowSet records instead of
//! flows; only the header is validated.
//!
//! ```
//! let mut d = vec![0u8; 24 + 48];
//! d[0..2].copy_from_slice(&[0, 5]); // version 5
//! d[2..4].copy_from_slice(&[0, 1]); // count 1
//! d[24..28].copy_from_slice(&[192, 0, 2, 1]); // src addr
//! let n = izanagi_kit::netflow::parse(&d).unwrap();
//! assert_eq!(n.version, 5);
//! assert_eq!(n.flows[0].src, [192, 0, 2, 1]);
//! ```

use std::vec::Vec;

/// One NetFlow v5 flow record (48 bytes).
#[derive(Clone, Debug)]
pub struct Flow {
    /// Source IPv4 address.
    pub src: [u8; 4],
    /// Destination IPv4 address.
    pub dst: [u8; 4],
    /// Next-hop IPv4 address.
    pub next_hop: [u8; 4],
    /// Input interface index.
    pub input: u16,
    /// Output interface index.
    pub output: u16,
    /// Packet count.
    pub packets: u32,
    /// Octet count.
    pub octets: u32,
    /// TCP flags (cumulative OR).
    pub tcp_flags: u8,
    /// IP protocol number.
    pub proto: u8,
    /// IP ToS.
    pub tos: u8,
    /// Source port.
    pub src_port: u16,
    /// Destination port.
    pub dst_port: u16,
}

/// A parsed NetFlow datagram.
#[derive(Clone, Debug)]
pub struct Netflow {
    /// Export version (5 or 9 recognized).
    pub version: u16,
    /// Records/flows count field.
    pub count: u16,
    /// `sys_uptime` ms since device boot.
    pub sys_uptime: u32,
    /// `unix_secs` export time.
    pub unix_secs: u32,
    /// v5 flow records (empty for v9 — only the header is parsed).
    pub flows: Vec<Flow>,
    /// v9 `source_id` / v5 `engine_id` field.
    pub source_id: u32,
}

fn u16be(d: &[u8], o: usize) -> Option<u16> {
    Some((u16::from(*d.get(o)?) << 8) | u16::from(*d.get(o + 1)?))
}
fn u32be(d: &[u8], o: usize) -> Option<u32> {
    let d = d.get(o..o + 4)?;
    Some(
        (u32::from(d[0]) << 24)
            | (u32::from(d[1]) << 16)
            | (u32::from(d[2]) << 8)
            | u32::from(d[3]),
    )
}

/// Parse a NetFlow datagram; `None` unless version 5 (with all
/// `count` records present) or version 9 (header only).
pub fn parse(d: &[u8]) -> Option<Netflow> {
    let version = u16be(d, 0)?;
    match version {
        5 => {
            if d.len() < 24 {
                return None;
            }
            let count = u16be(d, 2)?;
            let need = 24 + usize::from(count) * 48;
            if d.len() < need {
                return None;
            }
            let mut flows = Vec::with_capacity(count.into());
            for i in 0..usize::from(count) {
                let o = 24 + i * 48;
                flows.push(Flow {
                    src: d[o..o + 4].try_into().ok()?,
                    dst: d[o + 4..o + 8].try_into().ok()?,
                    next_hop: d[o + 8..o + 12].try_into().ok()?,
                    input: u16be(d, o + 12)?,
                    output: u16be(d, o + 14)?,
                    packets: u32be(d, o + 16)?,
                    octets: u32be(d, o + 20)?,
                    tcp_flags: d[o + 37],
                    proto: d[o + 38],
                    tos: d[o + 39],
                    src_port: u16be(d, o + 32)?,
                    dst_port: u16be(d, o + 34)?,
                });
            }
            Some(Netflow {
                version,
                count,
                sys_uptime: u32be(d, 4)?,
                unix_secs: u32be(d, 8)?,
                flows,
                source_id: u32::from(d[19]), // engine_id
            })
        }
        9 => {
            if d.len() < 20 {
                return None;
            }
            Some(Netflow {
                version,
                count: u16be(d, 2)?,
                sys_uptime: u32be(d, 4)?,
                unix_secs: u32be(d, 8)?,
                flows: Vec::new(),
                source_id: u32be(d, 16)?,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v5() {
        let mut d = vec![0u8; 24 + 96];
        d[0..2].copy_from_slice(&[0, 5]);
        d[2..4].copy_from_slice(&[0, 2]);
        d[8..12].copy_from_slice(&[0, 0, 0, 99]);
        // first record
        d[24..28].copy_from_slice(&[10, 0, 0, 1]);
        d[28..32].copy_from_slice(&[10, 0, 0, 2]);
        d[24 + 16..24 + 20].copy_from_slice(&[0, 0, 0, 7]); // packets
        d[24 + 32..24 + 34].copy_from_slice(&[0x04, 0xd2]); // src port 1234
        d[24 + 34..24 + 36].copy_from_slice(&[0x00, 0x50]); // dst 80
        d[24 + 38] = 6; // proto TCP
        let n = parse(&d).unwrap();
        assert_eq!(n.flows.len(), 2);
        assert_eq!(n.flows[0].src, [10, 0, 0, 1]);
        assert_eq!(n.flows[0].packets, 7);
        assert_eq!(n.flows[0].proto, 6);
        assert_eq!(n.flows[0].dst_port, 80);
        assert_eq!(n.unix_secs, 99);
    }

    #[test]
    fn v9() {
        let mut d = vec![0u8; 20];
        d[0..2].copy_from_slice(&[0, 9]);
        d[2..4].copy_from_slice(&[0, 3]);
        d[16..20].copy_from_slice(&[0, 0, 0, 9]);
        let n = parse(&d).unwrap();
        assert_eq!(n.version, 9);
        assert_eq!(n.count, 3);
        assert_eq!(n.source_id, 9);
        assert!(n.flows.is_empty());
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        let mut d = vec![0u8; 24];
        d[0..2].copy_from_slice(&[0, 7]); // v7 unsupported
        assert!(parse(&d).is_none());
        d[0..2].copy_from_slice(&[0, 5]);
        d[2..4].copy_from_slice(&[0, 3]); // needs 24+144
        assert!(parse(&d).is_none());
        d[2..4].copy_from_slice(&[0, 1]); // v5 with only header, count=1 still short
        assert!(parse(&d).is_none());
    }
}
