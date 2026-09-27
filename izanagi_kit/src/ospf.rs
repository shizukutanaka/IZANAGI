//! OSPF packet header (RFC 2328 v2 for IPv4, RFC 5340 v3 for IPv6):
//! 16 bytes of `version`, `type` (1=Hello, 2=DD, 3=LSR, 4=LSU,
//! 5=LSAck), `len`, `router_id`, `area_id`, `checksum`, `autype`,
//! `auth`. LSU packets (`type` 4) carry an LSA count word at offset
//! 24.
//!
//! ```
//! let mut d = vec![0u8; 24];
//! d[0] = 2; // OSPFv2
//! d[1] = 1; // Hello
//! d[2..4].copy_from_slice(&[0, 24]);
//! d[4..8].copy_from_slice(&[192, 0, 2, 1]); // router id
//! d[8..12].copy_from_slice(&[0, 0, 0, 0]); // area 0
//! let o = izanagi_kit::ospf::parse(&d).unwrap();
//! assert_eq!(o.kind, izanagi_kit::ospf::Kind::Hello);
//! assert_eq!(o.router_id, [192, 0, 2, 1]);
//! ```

/// OSPF packet type (RFC 2328 §A.3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `1` — Hello (discover/maintain neighbors)
    Hello,
    /// `2` — Database Description
    DatabaseDescription,
    /// `3` — Link State Request
    LsRequest,
    /// `4` — Link State Update
    LsUpdate,
    /// `5` — Link State Acknowledgement
    LsAck,
}

impl Kind {
    /// Map the wire type byte to a `Kind`; `None` if not 1..=5.
    pub fn from_u8(b: u8) -> Option<Kind> {
        Some(match b {
            1 => Kind::Hello,
            2 => Kind::DatabaseDescription,
            3 => Kind::LsRequest,
            4 => Kind::LsUpdate,
            5 => Kind::LsAck,
            _ => return None,
        })
    }
}

/// A parsed OSPF header.
#[derive(Clone, Debug)]
pub struct Ospf {
    /// `2` (OSPFv2) or `3` (OSPFv3).
    pub version: u8,
    /// Packet type.
    pub kind: Kind,
    /// Declared packet length; must equal `d.len()`.
    pub len: u16,
    /// Advertising router ID (displayed dotted-quad for v2).
    pub router_id: [u8; 4],
    /// Area ID.
    pub area_id: [u8; 4],
    /// Auth type: 0 none, 1 simple, 2 cryptographic.
    pub autype: u16,
    /// Number of LSAs an LSU declares (0 for other types).
    pub lsa_count: u32,
}

/// Parse an OSPF packet: header 16 bytes (v2) or v3, declared `len`
/// must fit the buffer, `autype` ≤ 2 for v2 (v3 uses `instance_id`).
/// For LSU (`type` 4) the `lsa_count` word is read at offset 24.
pub fn parse(d: &[u8]) -> Option<Ospf> {
    if d.len() < 16 {
        return None;
    }
    let version = *d.first()?;
    if version != 2 && version != 3 {
        return None;
    }
    let kind = Kind::from_u8(*d.get(1)?)?;
    let len = (u16::from(*d.get(2)?) << 8) | u16::from(*d.get(3)?);
    if usize::from(len) > d.len() || len < 16 {
        return None;
    }
    let router_id = [*d.get(4)?, *d.get(5)?, *d.get(6)?, *d.get(7)?];
    let area_id = [*d.get(8)?, *d.get(9)?, *d.get(10)?, *d.get(11)?];
    let autype = (u16::from(*d.get(14)?) << 8) | u16::from(*d.get(15)?);
    if version == 2 && autype > 2 {
        return None;
    }
    let lsa_count = if kind == Kind::LsUpdate && d.len() >= 24 {
        (u32::from(*d.get(20)?) << 24)
            | (u32::from(*d.get(21)?) << 16)
            | (u32::from(*d.get(22)?) << 8)
            | u32::from(*d.get(23)?)
    } else {
        0
    };
    Some(Ospf {
        version,
        kind,
        len,
        router_id,
        area_id,
        autype,
        lsa_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr(ty: u8, len: u16) -> Vec<u8> {
        let mut d = vec![0u8; usize::from(len).max(16)];
        d[0] = 2;
        d[1] = ty;
        d[2] = (len >> 8) as u8;
        d[3] = len as u8;
        d
    }

    #[test]
    fn hello_fields() {
        let mut d = hdr(1, 44);
        d[4..8].copy_from_slice(&[10, 0, 0, 1]);
        d[8..12].copy_from_slice(&[0, 0, 0, 0]);
        let o = parse(&d).unwrap();
        assert_eq!(o.kind, Kind::Hello);
        assert_eq!(o.router_id, [10, 0, 0, 1]);
        assert_eq!(o.len, 44);
    }

    #[test]
    fn lsu_count() {
        let mut d = hdr(4, 28);
        d[20] = 0;
        d[21] = 0;
        d[22] = 0;
        d[23] = 5;
        let o = parse(&d).unwrap();
        assert_eq!(o.kind, Kind::LsUpdate);
        assert_eq!(o.lsa_count, 5);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[4u8; 16]).is_none()); // version 4
        let mut bad = hdr(9, 16);
        assert!(parse(&bad).is_none()); // type 9 unknown
        bad[1] = 1;
        bad[2] = 0;
        bad[3] = 99; // len 99 > buffer 16
        assert!(parse(&bad).is_none());
        assert_eq!(Kind::from_u8(0), None);
        assert_eq!(Kind::from_u8(5), Some(Kind::LsAck));
    }
}
