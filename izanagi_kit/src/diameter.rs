//! Diameter (RFC 6733) — the AAA successor to RADIUS.
//!
//! Header: `u8 version(=1) | u24 msg_len | u8 flags | u24 command_code |
//! u32 app_id | u32 hop_by_hop | u32 end_to_end | AVPs…`.
//! AVP: `u32 code | u8 flags | u24 len | data | pad to 4`.
//!
//! ```
//! let mut d = vec![1u8, 0, 0, 20];
//! d.extend_from_slice(&[0x80, 0, 1, 0x18]); // R flag + command 280 (DWR)
//! d.extend_from_slice(&[0, 0, 0, 0]); // app_id
//! d.extend_from_slice(&[0, 0, 0, 1]); // hop
//! d.extend_from_slice(&[0, 0, 0, 2]); // e2e
//! let m = izanagi_kit::diameter::parse(&d).unwrap();
//! assert!(m.request);
//! assert_eq!(m.command, 280);
//! ```

/// Parsed Diameter message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diameter {
    /// `R` flag — request when set, answer when clear.
    pub request: bool,
    /// `P` flag — proxiable.
    pub proxiable: bool,
    /// `E` flag — error.
    pub error: bool,
    /// `T` flag — potential retransmission.
    pub retransmit: bool,
    /// 24-bit command code.
    pub command: u32,
    /// Application ID.
    pub app_id: u32,
    /// Hop-by-hop identifier.
    pub hop_by_hop: u32,
    /// End-to-end identifier.
    pub end_to_end: u32,
    /// AVP codes in order.
    pub avps: Vec<u32>,
}

fn be32(d: &[u8], o: usize) -> Option<u32> {
    Some(
        ((*d.get(o)? as u32) << 24)
            | ((*d.get(o + 1)? as u32) << 16)
            | ((*d.get(o + 2)? as u32) << 8)
            | *d.get(o + 3)? as u32,
    )
}

fn be24(d: &[u8], o: usize) -> Option<u32> {
    Some(((*d.get(o)? as u32) << 16) | ((*d.get(o + 1)? as u32) << 8) | *d.get(o + 2)? as u32)
}

/// Parse a Diameter message; `None` on version != 1 or a bad length field.
pub fn parse(d: &[u8]) -> Option<Diameter> {
    if d.len() < 20 || d[0] != 1 {
        return None;
    }
    let msg_len = be24(d, 1)? as usize;
    if msg_len < 20 || msg_len > d.len() {
        return None;
    }
    let flags = d[4];
    let command = be24(d, 5)?;
    let app_id = be32(d, 8)?;
    let hop_by_hop = be32(d, 12)?;
    let end_to_end = be32(d, 16)?;

    let mut avps = Vec::new();
    let mut p = 20usize;
    while p + 8 <= msg_len {
        let code = be32(d, p)?;
        let alen = be24(d, p + 5)? as usize;
        if alen < 8 || p + alen > msg_len {
            break;
        }
        avps.push(code);
        p += (alen + 3) & !3; // 32-bit padding
    }
    Some(Diameter {
        request: flags & 0x80 != 0,
        proxiable: flags & 0x40 != 0,
        error: flags & 0x20 != 0,
        retransmit: flags & 0x10 != 0,
        command,
        app_id,
        hop_by_hop,
        end_to_end,
        avps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(flags: u8, cmd: u32, avps: &[u32]) -> Vec<u8> {
        let mut body = Vec::new();
        let b32 = |x: u32| [(x >> 24) as u8, (x >> 16) as u8, (x >> 8) as u8, x as u8];
        for &a in avps {
            body.extend_from_slice(&b32(a));
            body.push(0); // flags
            body.extend_from_slice(&[0, 0, 8]); // len = 8 (no data)
        }
        let len = 20 + body.len();
        let mut d = vec![1u8, (len >> 16) as u8, (len >> 8) as u8, len as u8];
        d.push(flags);
        d.extend_from_slice(&[(cmd >> 16) as u8, (cmd >> 8) as u8, cmd as u8]);
        d.extend_from_slice(&b32(0));
        d.extend_from_slice(&b32(7));
        d.extend_from_slice(&b32(9));
        d.extend_from_slice(&body);
        d
    }

    #[test]
    fn basic() {
        let m = parse(&msg(0x80, 257, &[263, 268])).unwrap(); // R, CER with AVPs
        assert!(m.request && !m.error && !m.retransmit);
        assert!(!m.proxiable);
        assert_eq!(m.command, 257); // Capabilities-Exchange
        assert_eq!((m.hop_by_hop, m.end_to_end), (7, 9));
        assert_eq!(m.avps, vec![263, 268]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&msg(0x80, 257, &[])[..10]).is_none()); // truncated
        let mut bad = msg(0x80, 257, &[]);
        bad[0] = 2; // version
        assert!(parse(&bad).is_none());
        let mut badlen = msg(0x80, 257, &[]);
        badlen[3] = 19; // msg_len < 20
        assert!(parse(&badlen).is_none());
    }
}
