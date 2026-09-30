//! SOCKS (RFC 1928) — v5 greeting / connect-request / reply, plus a v4
//! request sanity check.
//!
//! ```
//! // v5 connect to 192.0.2.1:80
//! let d = [5, 1, 0, 1, 192, 0, 2, 1, 0, 80];
//! let s = izanagi_kit::socks::parse(&d).unwrap();
//! assert_eq!(s.kind, izanagi_kit::socks::Kind::Message);
//! assert_eq!(s.command, Some(1));
//! ```

/// Which SOCKS5 datagram shape this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `05 nmethods methods…` — client greeting.
    Greeting,
    /// `05 cmd 00 atyp addr port` — request (client) or reply (server);
    /// indistinguishable on the wire.
    Message,
    /// `04 cmd port ip` — SOCKS4/4a request.
    V4,
}

/// Parsed SOCKS message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Socks {
    /// Message shape.
    pub kind: Kind,
    /// SOCKS version (4 or 5).
    pub version: u8,
    /// v5 greeting: method bytes offered.
    pub methods: Vec<u8>,
    /// v5 request `CMD` / reply `REP` field.
    pub command: Option<u8>,
    /// Address type (`1` IPv4, `3` domain, `4` IPv6) when present.
    pub atyp: Option<u8>,
    /// Byte length of the on-wire address.
    pub addr_len: usize,
    /// Destination port (request/reply only).
    pub port: Option<u16>,
}

fn be16(d: &[u8], o: usize) -> Option<u16> {
    Some(((*d.get(o)? as u16) << 8) | *d.get(o + 1)? as u16)
}

fn atyp_len(d: &[u8], at: usize, atyp: u8) -> Option<usize> {
    match atyp {
        1 => Some(4),
        4 => Some(16),
        3 => Some(1 + *d.get(at)? as usize),
        _ => None,
    }
}

/// Parse one SOCKS message; `None` on unknown version or truncation.
pub fn parse(d: &[u8]) -> Option<Socks> {
    match *d.first()? {
        4 => {
            // V4 request: 04 cmd port ip userid\0 [domain\0]
            let cmd = *d.get(1)?;
            let port = be16(d, 2)?;
            let _ip = d.get(4..8)?;
            let nul = d[8..].iter().position(|&c| c == 0)?;
            if nul == 0 {
                return None;
            }
            Some(Socks {
                kind: Kind::V4,
                version: 4,
                methods: Vec::new(),
                command: Some(cmd),
                atyp: None,
                addr_len: 4,
                port: Some(port),
            })
        }
        5 => {
            let n = *d.get(1)? as usize;
            if d.len() >= 2 + n && (d.len() == 2 + n || d.get(2) != Some(&0)) {
                return Some(Socks {
                    kind: Kind::Greeting,
                    version: 5,
                    methods: d[2..2 + n].to_vec(),
                    command: None,
                    atyp: None,
                    addr_len: 0,
                    port: None,
                });
            }
            // request/reply: VER CMD RSV ATYP DST.ADDR DST.PORT
            if d.len() < 4 || d[2] != 0 {
                return None;
            }
            let atyp = d[3];
            let alen = atyp_len(d, 4, atyp)?;
            let port = be16(d, 4 + alen)?;
            if 4 + alen + 2 > d.len() {
                return None;
            }
            Some(Socks {
                kind: Kind::Message,
                version: 5,
                methods: Vec::new(),
                command: Some(d[1]),
                atyp: Some(atyp),
                addr_len: alen,
                port: Some(port),
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greeting() {
        let s = parse(&[5, 2, 0, 2]).unwrap();
        assert_eq!(s.kind, Kind::Greeting);
        assert_eq!(s.methods, vec![0, 2]);
        assert_eq!(s.command, None);
    }

    #[test]
    fn connect_ipv4() {
        let s = parse(&[5, 1, 0, 1, 192, 0, 2, 1, 0x1F, 0x90]).unwrap();
        assert_eq!(s.kind, Kind::Message);
        assert_eq!(s.command, Some(1));
        assert_eq!(s.atyp, Some(1));
        assert_eq!(s.addr_len, 4);
        assert_eq!(s.port, Some(8080));
    }

    #[test]
    fn connect_domain() {
        // domain "x.io" (len 4), port 443
        let s = parse(&[5, 3, 0, 3, 4, b'x', b'.', b'i', b'o', 1, 0xBB]).unwrap();
        assert_eq!(s.addr_len, 5);
        assert_eq!(s.port, Some(443));
    }

    #[test]
    fn v4_request() {
        let s = parse(&[4, 1, 0, 80, 127, 0, 0, 1, b'u', 0]).unwrap();
        assert_eq!(s.kind, Kind::V4);
        assert_eq!(s.port, Some(80));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[6, 1, 0]).is_none()); // bad version
        assert!(parse(&[5, 1, 0, 9]).is_none()); // bad atyp
        assert!(parse(&[5, 1, 0, 1, 1, 2, 3]).is_none()); // truncated addr
        assert!(parse(&[4, 1, 0, 80, 1, 2, 3, 4]).is_none()); // v4 no NUL
    }
}
