//! Minimal reader for BGP-4 messages (RFC 4271): 16-byte `marker`
//! (0xFF×16 in unauthenticated sessions) + `len`(u16 BE, 19..=4096) +
//! `type`(1=OPEN, 2=UPDATE, 3=NOTIFICATION, 4=KEEPALIVE,
//! 5=ROUTE-REFRESH). The OPEN body is decoded one level deeper —
//! `version`, `my_as`, `hold_time`, `bgp_id`, `opt_len`.
//!
//! ```
//! use izanagi_kit::bgp::{parse, Kind};
//!
//! // KEEPALIVE: marker + len 19 + type 4
//! let d = [&[0xFF; 16][..], &[0, 19, 4]].concat();
//! let b = parse(&d).unwrap();
//! assert_eq!(b.kind, Kind::Keepalive);
//! ```

/// BGP message type codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `1` — OPEN
    Open,
    /// `2` — UPDATE
    Update,
    /// `3` — NOTIFICATION
    Notification,
    /// `4` — KEEPALIVE
    Keepalive,
    /// `5` — ROUTE-REFRESH (RFC 2918)
    RouteRefresh,
}

/// A parsed BGP message header (plus OPEN fields when applicable).
#[derive(Debug)]
pub struct Bgp {
    /// `marker` — 16 bytes, all 0xFF for unauthenticated sessions.
    pub marker: [u8; 16],
    /// `len` — total message length including the 19-byte header.
    pub len: u16,
    /// Message type.
    pub kind: Kind,
    /// OPEN body: `version` (usually 4).
    pub version: Option<u8>,
    /// OPEN body: sender's AS number.
    pub my_as: Option<u16>,
    /// OPEN body: hold time in seconds.
    pub hold_time: Option<u16>,
    /// OPEN body: BGP identifier (router ID as 4 raw bytes).
    pub bgp_id: Option<[u8; 4]>,
    /// OPEN body: optional-parameters byte length.
    pub opt_len: Option<u8>,
}

fn be16(d: &[u8], at: usize) -> Option<u16> {
    let hi = *d.get(at)? as u16;
    let lo = *d.get(at + 1)? as u16;
    Some((hi << 8) | lo)
}

/// Parse one BGP message. `None` on a bad marker, `len` out of range or
/// mismatching the buffer, or an unknown type.
pub fn parse(d: &[u8]) -> Option<Bgp> {
    if d.len() < 19 || d[..16].iter().any(|&b| b != 0xFF) {
        return None;
    }
    let len = be16(d, 16)?;
    if !(19..=4096).contains(&len) || len as usize != d.len() {
        return None;
    }
    let kind = match d[18] {
        1 => Kind::Open,
        2 => Kind::Update,
        3 => Kind::Notification,
        4 => Kind::Keepalive,
        5 => Kind::RouteRefresh,
        _ => return None,
    };
    let mut bgp = Bgp {
        marker: {
            let mut m = [0xFF; 16];
            m.copy_from_slice(&d[..16]);
            m
        },
        len,
        kind,
        version: None,
        my_as: None,
        hold_time: None,
        bgp_id: None,
        opt_len: None,
    };
    if kind == Kind::Open {
        if d.len() < 29 {
            return None;
        }
        bgp.version = Some(d[19]);
        bgp.my_as = be16(d, 20);
        bgp.hold_time = be16(d, 22);
        bgp.bgp_id = Some([d[24], d[25], d[26], d[27]]);
        bgp.opt_len = Some(d[28]);
        if 29 + d[28] as usize > d.len() {
            return None;
        }
    }
    Some(bgp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keepalive() {
        let d = [&[0xFF; 16][..], &[0, 19, 4]].concat();
        let b = parse(&d).unwrap();
        assert_eq!(b.kind, Kind::Keepalive);
        assert_eq!(b.len, 19);
    }

    #[test]
    fn open() {
        let mut d = vec![0xFF; 16];
        d.extend_from_slice(&[0, 29, 1]); // len 29, OPEN
        d.extend_from_slice(&[4, 0xFD, 0xE8, 0, 90, 10, 0, 0, 1, 0]); // v4, AS65000, hold 90, id 10.0.0.1, optlen 0
        let b = parse(&d).unwrap();
        assert_eq!(b.kind, Kind::Open);
        assert_eq!(b.version, Some(4));
        assert_eq!(b.my_as, Some(0xFDE8));
        assert_eq!(b.hold_time, Some(90));
        assert_eq!(b.bgp_id, Some([10, 0, 0, 1]));
        assert_eq!(b.opt_len, Some(0));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut d = vec![0xFF; 16];
        d.extend_from_slice(&[0, 19, 99]); // unknown type
        assert!(parse(&d).is_none());
        d[0] = 0; // bad marker
        assert!(parse(&d).is_none());
        d[0] = 0xFF;
        d[17] = 20; // len 20 but buffer 19
        d[18] = 4;
        assert!(parse(&d).is_none());
    }
}
