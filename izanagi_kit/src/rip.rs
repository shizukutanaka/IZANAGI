//! RIP — Routing Information Protocol (RFC 2453 v2): `command`
//! (1=request, 2=response) + `version` + `zero`, then 20-byte route
//! entries `{family, tag, addr, mask, nexthop, metric}`.
//!
//! ```
//! let mut d = vec![0u8; 24];
//! d[0] = 2; // response
//! d[1] = 2; // v2
//! // entry: family 2 (IPv4), tag 7, addr 192.168.0.0, metric 3
//! d[4..6].copy_from_slice(&[0, 2]);
//! d[6..8].copy_from_slice(&[0, 7]);
//! d[8..12].copy_from_slice(&[192, 168, 0, 0]);
//! d[20..24].copy_from_slice(&[0, 0, 0, 3]);
//! let r = izanagi_kit::rip::parse(&d).unwrap();
//! assert_eq!(r.entries.len(), 1);
//! assert_eq!(r.entries[0].metric, 3);
//! ```

use std::vec::Vec;

/// RIP command byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// `1` — request the routing table
    Request,
    /// `2` — response carrying route entries
    Response,
    /// Any other command byte (traceoff, poll, …).
    Other(u8),
}

/// One 20-byte RIP route entry.
#[derive(Clone, Debug)]
pub struct Route {
    /// Address-family identifier (2 = IPv4; `0xFFFF` = auth entry).
    pub family: u16,
    /// Route tag.
    pub tag: u16,
    /// IPv4 address bytes of the destination.
    pub addr: [u8; 4],
    /// Subnet mask.
    pub mask: [u8; 4],
    /// Next hop.
    pub nexthop: [u8; 4],
    /// Metric (1–16; 16 = unreachable).
    pub metric: u32,
}

/// A parsed RIP packet.
#[derive(Clone, Debug)]
pub struct Rip {
    /// `Request`/`Response`/`Other`.
    pub command: Command,
    /// RIP version (usually 2).
    pub version: u8,
    /// `must-be-zero` word at offset 2.
    pub must_be_zero: u16,
    /// Route entries (20 bytes each).
    pub entries: Vec<Route>,
}

/// Parse a RIP packet: ≥ 4 bytes; the trailing length must be a whole
/// number of 20-byte entries. `version` is reported, not enforced.
pub fn parse(d: &[u8]) -> Option<Rip> {
    if d.len() < 4 {
        return None;
    }
    let command = match *d.first()? {
        1 => Command::Request,
        2 => Command::Response,
        other => Command::Other(other),
    };
    let version = *d.get(1)?;
    let must_be_zero = (u16::from(*d.get(2)?) << 8) | u16::from(*d.get(3)?);
    let body = d.get(4..)?;
    if body.len() % 20 != 0 {
        return None;
    }
    let mut entries: Vec<Route> = Vec::new();
    let mut i = 0usize;
    while i < body.len() {
        let e = body.get(i..i + 20)?;
        entries.push(Route {
            family: (u16::from(e[0]) << 8) | u16::from(e[1]),
            tag: (u16::from(e[2]) << 8) | u16::from(e[3]),
            addr: [e[4], e[5], e[6], e[7]],
            mask: [e[8], e[9], e[10], e[11]],
            nexthop: [e[12], e[13], e[14], e[15]],
            metric: (u32::from(e[16]) << 24)
                | (u32::from(e[17]) << 16)
                | (u32::from(e[18]) << 8)
                | u32::from(e[19]),
        });
        i += 20;
    }
    Some(Rip {
        command,
        version,
        must_be_zero,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0u8; 44];
        d[0] = 2;
        d[1] = 2;
        d[4] = 0;
        d[5] = 2;
        d[6] = 0;
        d[7] = 7;
        d[8..12].copy_from_slice(&[10, 0, 0, 0]);
        d[12..16].copy_from_slice(&[255, 255, 0, 0]);
        d[20..24].copy_from_slice(&[0, 0, 0, 5]);
        // second entry: family 0xFFFF (auth)
        d[24] = 0xff;
        d[25] = 0xff;
        d
    }

    #[test]
    fn two_entries() {
        let r = parse(&fixture()).unwrap();
        assert_eq!(r.command, Command::Response);
        assert_eq!(r.version, 2);
        assert_eq!(r.entries.len(), 2);
        assert_eq!(r.entries[0].family, 2);
        assert_eq!(r.entries[0].mask, [255, 255, 0, 0]);
        assert_eq!(r.entries[0].metric, 5);
        assert_eq!(r.entries[1].family, 0xffff);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[2, 2, 0, 0, 1]).is_none()); // 1-byte entry tail
        let r = parse(&[3, 2, 0, 0]).unwrap(); // command 3 → Other(3)
        assert_eq!(r.command, Command::Other(3));
        assert!(r.entries.is_empty());
    }
}
