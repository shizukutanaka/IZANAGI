//! TACACS+ (RFC 8907) — Cisco's AAA protocol header.
//!
//! 12-byte header: `u8 major_minor | u8 type | u8 seq | u8 flags |
//! u32 session_id | u32 len` (big-endian). Major nibble is 0xC
//! (0xC0 = v12.0, 0xC1 = v12.1).
//!
//! ```
//! let d = [0xC0, 1, 1, 0, 0, 0, 0, 7, 0, 0, 0, 10];
//! let t = izanagi_kit::tacacs::parse(&d).unwrap();
//! assert_eq!((t.major, t.minor), (12, 0));
//! assert_eq!(t.kind, izanagi_kit::tacacs::Kind::Authentication);
//! assert_eq!(t.session_id, 7);
//! ```

/// TACACS+ service type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// 0x01 — authentication.
    Authentication,
    /// 0x02 — authorization.
    Authorization,
    /// 0x03 — accounting.
    Accounting,
}

/// Parsed TACACS+ header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tacacs {
    /// Major version (12).
    pub major: u8,
    /// Minor version (0 default, 1 v12.1).
    pub minor: u8,
    /// Service type.
    pub kind: Kind,
    /// Sequence number.
    pub seq: u8,
    /// Flags: bit0 = unencrypted body, bit2 = single-connection.
    pub flags: u8,
    /// Session identifier.
    pub session_id: u32,
    /// Declared body length (may exceed the input on a header-only read).
    pub body_len: u32,
    /// True when bit0 is clear — the body is obfuscated with the MD5 pad.
    pub encrypted: bool,
}

fn be32(d: &[u8], o: usize) -> Option<u32> {
    Some(
        ((*d.get(o)? as u32) << 24)
            | ((*d.get(o + 1)? as u32) << 16)
            | ((*d.get(o + 2)? as u32) << 8)
            | *d.get(o + 3)? as u32,
    )
}

/// Parse a TACACS+ header; `None` on short input, bad version, or bad type.
pub fn parse(d: &[u8]) -> Option<Tacacs> {
    if d.len() < 12 {
        return None;
    }
    let (major, minor) = (d[0] >> 4, d[0] & 0x0F);
    if major != 0x0C || minor > 1 {
        return None;
    }
    let kind = match d[1] {
        1 => Kind::Authentication,
        2 => Kind::Authorization,
        3 => Kind::Accounting,
        _ => return None,
    };
    let seq = d[2];
    let flags = d[3];
    let session_id = be32(d, 4)?;
    let body_len = be32(d, 8)?;
    if seq == 0 {
        return None;
    }
    Some(Tacacs {
        major,
        minor,
        kind,
        seq,
        flags,
        session_id,
        body_len,
        encrypted: flags & 1 == 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let t = parse(&[0xC1, 2, 3, 0, 0xDE, 0xAD, 0xBE, 0xEF, 0, 0, 0, 40]).unwrap();
        assert_eq!((t.major, t.minor), (12, 1));
        assert_eq!(t.kind, Kind::Authorization);
        assert_eq!(t.seq, 3);
        assert_eq!(t.session_id, 0xDEADBEEF);
        assert_eq!(t.body_len, 40);
        assert!(t.encrypted);
    }

    #[test]
    fn cleartext_flag() {
        let t = parse(&[0xC0, 3, 1, 1, 0, 0, 0, 1, 0, 0, 0, 0]).unwrap();
        assert_eq!(t.kind, Kind::Accounting);
        assert!(!t.encrypted); // bit0 set → unencrypted
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0xC0; 11]).is_none()); // short
        assert!(parse(&[0xD0, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0]).is_none()); // major != 12
        assert!(parse(&[0xC2, 1, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0]).is_none()); // minor 2
        assert!(parse(&[0xC0, 9, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0]).is_none()); // bad type
        assert!(parse(&[0xC0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0]).is_none()); // seq 0
    }
}
