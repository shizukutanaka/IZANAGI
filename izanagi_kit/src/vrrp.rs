//! Minimal reader for VRRP advertisements (RFC 5798): 4-bit `version` +
//! 4-bit `type` (1 = Advertisement is the only defined type) in the
//! first byte, then `vrid`, `priority`, `count_ip`, `rsvd+max_adver_int`
//! (u16, high nibble reserved for v2 / centisecond interval for v3),
//! `checksum`, followed by `count_ip` IPv4 addresses.
//!
//! ```
//! use izanagi_kit::vrrp::parse;
//!
//! // v3 advert, vrid 10, priority 100, 1 address
//! let mut d = vec![0x31u8, 10, 100, 1, 0, 100, 0xDE, 0xAD];
//! d.extend_from_slice(&[192, 0, 2, 1]);
//! let p = parse(&d).unwrap();
//! assert_eq!(p.version, 3);
//! assert_eq!(p.vrid, 10);
//! assert_eq!(p.addrs, vec![[192, 0, 2, 1]]);
//! ```

/// A parsed VRRP advertisement.
#[derive(Debug)]
pub struct Vrrp {
    /// Version nibble (2 or 3).
    pub version: u8,
    /// Type nibble (only 1 = Advertisement is valid).
    pub kind: u8,
    /// Virtual Router ID.
    pub vrid: u8,
    /// Priority (255 = owner, 0 = release).
    pub priority: u8,
    /// Declared address count.
    pub count_ip: u8,
    /// `rsvd|max_adver_int` field — centiseconds for v3, seconds (v2 uses
    /// `adver_int` at the same offset) carried verbatim.
    pub max_adver_int: u16,
    /// Header checksum field (verification left to the caller).
    pub checksum: u16,
    /// IPv4 virtual addresses, `count_ip` of them.
    pub addrs: Vec<[u8; 4]>,
}

/// Parse a VRRP advertisement. `None` on version ∉ {2,3}, type ≠ 1,
/// `count_ip` mismatching the buffer, or short input.
pub fn parse(d: &[u8]) -> Option<Vrrp> {
    let first = *d.first()?;
    let version = first >> 4;
    let kind = first & 0x0F;
    if !matches!(version, 2 | 3) || kind != 1 {
        return None;
    }
    let count_ip = *d.get(3)?;
    let want = 8 + count_ip as usize * 4;
    if d.len() < want {
        return None;
    }
    let max_adver_int = {
        let hi = *d.get(4)? as u16;
        let lo = *d.get(5)? as u16;
        (hi << 8) | lo
    };
    let checksum = {
        let hi = *d.get(6)? as u16;
        let lo = *d.get(7)? as u16;
        (hi << 8) | lo
    };
    let mut addrs = Vec::with_capacity(count_ip as usize);
    for i in 0..count_ip as usize {
        let a = 8 + i * 4;
        addrs.push([d[a], d[a + 1], d[a + 2], d[a + 3]]);
    }
    Some(Vrrp {
        version,
        kind,
        vrid: d[1],
        priority: d[2],
        count_ip,
        max_adver_int,
        checksum,
        addrs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let mut d = vec![0x31u8, 10, 100, 2, 0, 100, 0x12, 0x34];
        d.extend_from_slice(&[10, 0, 0, 1, 10, 0, 0, 2]);
        let p = parse(&d).unwrap();
        assert_eq!(p.version, 3);
        assert_eq!(p.kind, 1);
        assert_eq!(p.vrid, 10);
        assert_eq!(p.priority, 100);
        assert_eq!(p.max_adver_int, 100);
        assert_eq!(p.checksum, 0x1234);
        assert_eq!(p.addrs.len(), 2);
        assert_eq!(p.addrs[1], [10, 0, 0, 2]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        // version 4 not defined
        assert!(parse(&[0x41, 1, 255, 0, 0, 0, 0, 0]).is_none());
        // type 2 doesn't exist
        assert!(parse(&[0x22, 1, 255, 0, 0, 0, 0, 0]).is_none());
        // count_ip=1 but no address bytes
        assert!(parse(&[0x31, 1, 255, 1, 0, 0, 0, 0]).is_none());
        // v2 is fine
        assert!(parse(&[0x21, 1, 255, 0, 0, 0, 0, 0]).is_some());
    }
}
