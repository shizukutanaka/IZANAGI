//! Minimal reader for STP BPDUs (IEEE 802.1D / 802.1w): `protocol_id`
//! (always 0) + `version` (0 STP, 2 RSTP, 3 MSTP) + `bpdu_type`
//! (0x00 config, 0x80 TCN, 0x02 RST/MST config). A Configuration BPDU
//! then carries `flags`, `root_id`(8), `root_path_cost`(4),
//! `bridge_id`(8), `port_id`(2), `message_age`, `max_age`,
//! `hello_time`, `forward_delay` — all u16/u32 big-endian; times are
//! 1/256 s units kept as integers.
//!
//! ```
//! use izanagi_kit::stp::{parse, Kind};
//!
//! // TCN BPDU: proto 0, version 0, type 0x80
//! let b = parse(&[0, 0, 0, 0x80]).unwrap();
//! assert_eq!(b.kind, Kind::Tcn);
//! ```

/// BPDU type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `0x00` — Configuration BPDU (STP)
    Config,
    /// `0x80` — Topology Change Notification
    Tcn,
    /// `0x02` — RSTP/MSTP Configuration BPDU
    Rst,
}

/// A parsed BPDU header (+ config fields when the type has them).
#[derive(Debug)]
pub struct Stp {
    /// Protocol identifier (0 = spanning tree).
    pub protocol_id: u16,
    /// Protocol version (0 STP / 2 RSTP / 3 MSTP).
    pub version: u8,
    /// BPDU type.
    pub kind: Kind,
    /// Config/RST: `flags` byte (TC ack/TC bits).
    pub flags: Option<u8>,
    /// Config/RST: 8-byte root bridge ID verbatim.
    pub root_id: Option<[u8; 8]>,
    /// Config/RST: root path cost.
    pub root_path_cost: Option<u32>,
    /// Config/RST: sender bridge ID verbatim.
    pub bridge_id: Option<[u8; 8]>,
    /// Config/RST: port ID.
    pub port_id: Option<u16>,
    /// Config/RST: message age (1/256 s units).
    pub message_age: Option<u16>,
    /// Config/RST: max age (1/256 s units).
    pub max_age: Option<u16>,
    /// Config/RST: hello time.
    pub hello_time: Option<u16>,
    /// Config/RST: forward delay.
    pub forward_delay: Option<u16>,
}

fn be16(d: &[u8], at: usize) -> Option<u16> {
    let hi = *d.get(at)? as u16;
    let lo = *d.get(at + 1)? as u16;
    Some((hi << 8) | lo)
}

fn be32(d: &[u8], at: usize) -> Option<u32> {
    let b0 = *d.get(at)? as u32;
    let b1 = *d.get(at + 1)? as u32;
    let b2 = *d.get(at + 2)? as u32;
    let b3 = *d.get(at + 3)? as u32;
    Some((b0 << 24) | (b1 << 16) | (b2 << 8) | b3)
}

/// Parse one BPDU. `None` on a non-zero `protocol_id`, unknown type, or
/// a truncated body.
pub fn parse(d: &[u8]) -> Option<Stp> {
    let protocol_id = be16(d, 0)?;
    if protocol_id != 0 {
        return None;
    }
    let version = *d.get(2)?;
    let kind = match *d.get(3)? {
        0x00 => Kind::Config,
        0x80 => Kind::Tcn,
        0x02 => Kind::Rst,
        _ => return None,
    };
    let mut stp = Stp {
        protocol_id,
        version,
        kind,
        flags: None,
        root_id: None,
        root_path_cost: None,
        bridge_id: None,
        port_id: None,
        message_age: None,
        max_age: None,
        hello_time: None,
        forward_delay: None,
    };
    if matches!(kind, Kind::Config | Kind::Rst) {
        if d.len() < 35 {
            return None;
        }
        let mut root_id = [0u8; 8];
        root_id.copy_from_slice(&d[5..13]);
        let mut bridge_id = [0u8; 8];
        bridge_id.copy_from_slice(&d[17..25]);
        stp.flags = Some(d[4]);
        stp.root_id = Some(root_id);
        stp.root_path_cost = be32(d, 13);
        stp.bridge_id = Some(bridge_id);
        stp.port_id = be16(d, 25);
        stp.message_age = be16(d, 27);
        stp.max_age = be16(d, 29);
        stp.hello_time = be16(d, 31);
        stp.forward_delay = be16(d, 33);
    }
    Some(stp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tcn() {
        let b = parse(&[0, 0, 0, 0x80]).unwrap();
        assert_eq!(b.kind, Kind::Tcn);
        assert_eq!(b.version, 0);
        assert_eq!(b.flags, None);
    }

    #[test]
    fn config() {
        let mut d = vec![0u8, 0, 0, 0x00, 0x80]; // proto, ver, type, flags=TC
        d.extend_from_slice(&[0x80; 8]); // root_id
        d.extend_from_slice(&[0, 0, 0, 4]); // root_path_cost 4
        d.extend_from_slice(&[0x80; 8]); // bridge_id
        d.extend_from_slice(&[0x80, 0x01]); // port_id
        d.extend_from_slice(&[0, 0, 0x14, 0, 0, 0x80, 0x0F, 0x00]); // ages/hello/fwd
        assert_eq!(d.len(), 35);
        let b = parse(&d).unwrap();
        assert_eq!(b.kind, Kind::Config);
        assert_eq!(b.flags, Some(0x80));
        assert_eq!(b.root_path_cost, Some(4));
        assert_eq!(b.port_id, Some(0x8001));
        assert_eq!(b.max_age, Some(0x1400));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0, 1, 0, 0x80]).is_none()); // proto != 0
        assert!(parse(&[0, 0, 0, 0x42]).is_none()); // unknown type
        assert!(parse(&[0, 0, 0, 0x00, 0x80]).is_none()); // config truncated
    }
}
