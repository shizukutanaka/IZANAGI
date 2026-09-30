//! PIM — Protocol Independent Multicast (RFC 4601): `version:4` +
//! `type:4` nibble header + reserved byte + RFC 1071 checksum, then
//! type-specific body. Version must be 2.
//!
//! ```
//! // PIMv2 Hello (type 0) — checksum deliberately zero (not verified)
//! let d = [0x20u8, 0x00, 0x00, 0x00];
//! let p = izanagi_kit::pim::parse(&d).unwrap();
//! assert_eq!(p.kind, izanagi_kit::pim::Kind::Hello);
//! assert_eq!(p.version, 2);
//! ```

/// PIMv2 message type (RFC 4601 §4.9.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `0` — Hello
    Hello,
    /// `1` — Register
    Register,
    /// `2` — Register-Stop
    RegisterStop,
    /// `3` — Join/Prune
    JoinPrune,
    /// `4` — Bootstrap
    Bootstrap,
    /// `5` — Assert
    Assert,
    /// `6` — Graft (PIM-DM)
    Graft,
    /// `7` — Graft-Ack
    GraftAck,
    /// `8` — Candidate-RP-Advertisement
    CandRpAdv,
    /// `9` — State Refresh
    StateRefresh,
    /// `10` — DF Election
    DfElection,
    /// `11`–`15` — reserved/other values.
    Other(u8),
}

impl Kind {
    /// Map the wire nibble to a `Kind`.
    pub fn from_u8(b: u8) -> Kind {
        match b {
            0 => Kind::Hello,
            1 => Kind::Register,
            2 => Kind::RegisterStop,
            3 => Kind::JoinPrune,
            4 => Kind::Bootstrap,
            5 => Kind::Assert,
            6 => Kind::Graft,
            7 => Kind::GraftAck,
            8 => Kind::CandRpAdv,
            9 => Kind::StateRefresh,
            10 => Kind::DfElection,
            other => Kind::Other(other),
        }
    }
}

/// A parsed PIM header.
#[derive(Clone, Debug)]
pub struct Pim {
    /// Must be 2.
    pub version: u8,
    /// Message type.
    pub kind: Kind,
    /// `reserved` byte at offset 1.
    pub reserved: u8,
    /// Declared checksum bytes.
    pub checksum: u16,
    /// Checksum over the packet evaluates to `0` (RFC 1071). Always
    /// computed — a `false` means the checksum was bad or zero.
    pub checksum_ok: bool,
}

fn rfc1071(d: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut i = 0usize;
    while i + 1 < d.len() {
        sum += (u32::from(d[i]) << 8) | u32::from(d[i + 1]);
        i += 2;
    }
    if i < d.len() {
        sum += u32::from(d[i]) << 8;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

/// Parse a PIM header: 4 bytes minimum, version nibble must be 2, and
/// the header checksum is verified when nonzero (a zero checksum field
/// is accepted without verification per common receivers).
pub fn parse(d: &[u8]) -> Option<Pim> {
    if d.len() < 4 {
        return None;
    }
    let first = *d.first()?;
    let version = first >> 4;
    if version != 2 {
        return None;
    }
    let kind = Kind::from_u8(first & 0x0f);
    let reserved = *d.get(1)?;
    let checksum = (u16::from(*d.get(2)?) << 8) | u16::from(*d.get(3)?);
    let checksum_ok = checksum == 0 || rfc1071(d) == 0;
    Some(Pim {
        version,
        kind,
        reserved,
        checksum,
        checksum_ok,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_ok() {
        let d = [0x20u8, 0, 0, 0];
        let p = parse(&d).unwrap();
        assert_eq!(p.kind, Kind::Hello);
        assert!(p.checksum_ok);
    }

    #[test]
    fn bad_checksum_flagged() {
        let d = [0x20u8, 0, 0xde, 0xad];
        let p = parse(&d).unwrap();
        assert!(!p.checksum_ok);
    }

    #[test]
    fn kinds() {
        assert_eq!(Kind::from_u8(5), Kind::Assert);
        assert_eq!(Kind::from_u8(15), Kind::Other(15));
        for (b, k) in [(0x23u8, Kind::JoinPrune), (0x28, Kind::CandRpAdv)] {
            let d = [b, 0, 0, 0];
            assert_eq!(parse(&d).unwrap().kind, k);
        }
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0x10u8, 0, 0, 0]).is_none()); // version 1
        assert!(parse(&[0x30u8, 0, 0, 0]).is_none()); // version 3
    }
}
