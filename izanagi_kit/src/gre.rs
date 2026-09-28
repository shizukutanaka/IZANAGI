//! GRE tunnel header (RFC 2784, extended by RFC 2890):
//! `flags+version`(16b) + `protocol_type`(16b), then optional
//! checksum+offset (`C`), key (`K`), and sequence (`S`) fields in that
//! fixed order.
//!
//! ```
//! // GRE carrying IPv4, key present, no checksum
//! let d = [
//!     0x20u8, 0x00, // K flag set, version 0
//!     0x08, 0x00, // ethertype IPv4
//!     0x00, 0x00, 0x00, 0x2a, // key = 42
//!     0x45, 0x00, 0x00, 0x14, // IPv4 payload starts here
//! ];
//! let g = izanagi_kit::gre::parse(&d).unwrap();
//! assert_eq!(g.protocol, 0x0800);
//! assert_eq!(g.key, Some(42));
//! assert_eq!(g.payload_offset, 8);
//! ```

/// A parsed GRE header.
#[derive(Clone, Debug)]
pub struct Gre {
    /// Flag bits + version from the first 16 bits (`C R K S s …`).
    pub flags: u16,
    /// Low 3 bits of the first word — must be 0 (PPTP uses 1 for
    /// its enhanced variant, which this does not claim).
    pub version: u8,
    /// EtherType of the payload (`0x0800` IPv4, `0x86DD` IPv6,
    /// `0x6558` transparent Ethernet bridging).
    pub protocol: u16,
    /// Checksum present (`C` bit); covers the checksum+offset pair.
    pub has_checksum: bool,
    /// Key field when `K` bit set.
    pub key: Option<u32>,
    /// Sequence number when `S` bit set.
    pub sequence: Option<u32>,
    /// Byte offset at which the payload begins.
    pub payload_offset: usize,
}

/// Parse a GRE header: requires 4 bytes, `version == 0`, and enough
/// room for each flag-enabled optional field (checksum, key, seq).
/// Packets with the routing-present bit (`R`, 0x4000) are rejected:
/// their variable-length source-route entries precede the payload and
/// we cannot compute a trustworthy `payload_offset` for them (RFC 2784
/// deprecates routing anyway). The PPTP-only `A` (ack, 0x0080) bit is
/// likewise rejected.
pub fn parse(d: &[u8]) -> Option<Gre> {
    if d.len() < 4 {
        return None;
    }
    let flags = (u16::from(*d.first()?) << 8) | u16::from(*d.get(1)?);
    let version = (flags & 7) as u8;
    if version != 0 {
        return None; // version 0 = RFC 2784 GRE; 1 is PPTP-enhanced
    }
    if flags & 0x4080 != 0 {
        return None; // R (routing) / A (ack) extensions unsupported
    }
    let protocol = (u16::from(*d.get(2)?) << 8) | u16::from(*d.get(3)?);
    let mut off = 4usize;
    let has_checksum = flags & 0x8000 != 0;
    let has_key = flags & 0x2000 != 0;
    let has_seq = flags & 0x1000 != 0;
    if has_checksum {
        off = off.checked_add(4)?;
    }
    let key = if has_key {
        let k = (u32::from(*d.get(off)?) << 24)
            | (u32::from(*d.get(off + 1)?) << 16)
            | (u32::from(*d.get(off + 2)?) << 8)
            | u32::from(*d.get(off + 3)?);
        off += 4;
        Some(k)
    } else {
        None
    };
    let sequence = if has_seq {
        let s = (u32::from(*d.get(off)?) << 24)
            | (u32::from(*d.get(off + 1)?) << 16)
            | (u32::from(*d.get(off + 2)?) << 8)
            | u32::from(*d.get(off + 3)?);
        off += 4;
        Some(s)
    } else {
        None
    };
    if off > d.len() {
        return None;
    }
    Some(Gre {
        flags,
        version,
        protocol,
        has_checksum,
        key,
        sequence,
        payload_offset: off,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_ipv4() {
        let d = [0, 0, 0x08, 0x00, 0x45];
        let g = parse(&d).unwrap();
        assert_eq!(g.protocol, 0x0800);
        assert_eq!(g.payload_offset, 4);
        assert_eq!(g.key, None);
        assert!(!g.has_checksum);
    }

    #[test]
    fn full_options() {
        let mut d = vec![0xB0u8, 0x00, 0x86, 0xDD]; // C+K+S, IPv6
        d.extend_from_slice(&[0x12, 0x34, 0, 0]); // checksum + offset
        d.extend_from_slice(&[0, 0, 0, 7]); // key
        d.extend_from_slice(&[0, 0, 0, 9]); // seq
        d.extend_from_slice(&[0x60]); // payload
        let g = parse(&d).unwrap();
        assert!(g.has_checksum);
        assert_eq!(g.key, Some(7));
        assert_eq!(g.sequence, Some(9));
        assert_eq!(g.payload_offset, 16);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0, 1, 0, 0]).is_none()); // version 1 = PPTP, not GRE
        assert!(parse(&[0x40, 0, 0x08, 0]).is_none()); // R flag
        assert!(parse(&[0, 0x80, 0x08, 0]).is_none()); // A flag
        let mut d = vec![0x20, 0x00, 0x08, 0x00];
        assert!(parse(&d).is_none()); // K flag but no room for key
        d.extend_from_slice(&[0, 0, 0, 1]);
        assert!(parse(&d).is_some());
    }
}
