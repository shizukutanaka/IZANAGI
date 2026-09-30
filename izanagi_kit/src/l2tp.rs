//! L2TP (RFC 2661) — Layer 2 Tunnelling Protocol v2 header.
//!
//! `u16 flags|ver` then conditional fields: `[u16 len if L]` · `u16 tunnel`
//! · `u16 session` · `[u16 Ns | u16 Nr if S]` · `[u16 offset if O]`.
//! Version nibble must be 2.
//!
//! ```
//! // T+L+S bits set, ver 2, len 16, tunnel 7, session 9, Ns 1, Nr 0
//! let d = [0xC8, 0x02, 0, 16, 0, 7, 0, 9, 0, 1, 0, 0, 0, 0, 0, 0];
//! let l = izanagi_kit::l2tp::parse(&d).unwrap();
//! assert!(l.control);
//! assert_eq!((l.tunnel_id, l.session_id), (7, 9));
//! assert_eq!(l.ns, Some(1));
//! ```

/// Parsed L2TP header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct L2tp {
    /// `T` bit — control (1) vs data (0) message.
    pub control: bool,
    /// `L` bit — length field present.
    pub has_len: bool,
    /// `S` bit — Ns/Nr sequence fields present.
    pub has_seq: bool,
    /// `O` bit — offset field present.
    pub has_offset: bool,
    /// `P` bit — priority (data only).
    pub priority: bool,
    /// Declared message length (`None` when `L` is clear).
    pub len: Option<u16>,
    /// Tunnel ID.
    pub tunnel_id: u16,
    /// Session ID.
    pub session_id: u16,
    /// `Ns` send sequence (S only).
    pub ns: Option<u16>,
    /// `Nr` receive sequence (S only).
    pub nr: Option<u16>,
    /// Byte offset where the payload begins.
    pub payload_offset: usize,
}

fn be16(d: &[u8], o: usize) -> Option<u16> {
    Some(((*d.get(o)? as u16) << 8) | *d.get(o + 1)? as u16)
}

/// Parse an L2TP header; `None` on short input or version != 2.
pub fn parse(d: &[u8]) -> Option<L2tp> {
    let flags = be16(d, 0)?;
    if flags & 0x000F != 2 {
        return None;
    }
    let (control, has_len, has_seq, has_offset, priority) = (
        flags & 0x8000 != 0,
        flags & 0x4000 != 0,
        flags & 0x0800 != 0,
        flags & 0x0010 != 0,
        flags & 0x0200 != 0,
    );
    let mut pos = 2usize;
    let len = if has_len {
        let l = be16(d, pos)?;
        pos += 2;
        if (l as usize) > d.len() {
            return None;
        }
        Some(l)
    } else {
        None
    };
    let tunnel_id = be16(d, pos)?;
    let session_id = be16(d, pos + 2)?;
    pos += 4;
    let (mut ns, mut nr) = (None, None);
    if has_seq {
        ns = Some(be16(d, pos)?);
        nr = Some(be16(d, pos + 2)?);
        pos += 4;
    }
    if has_offset {
        pos += 2 + be16(d, pos)? as usize;
    }
    if pos > d.len() {
        return None;
    }
    Some(L2tp {
        control,
        has_len,
        has_seq,
        has_offset,
        priority,
        len,
        tunnel_id,
        session_id,
        ns,
        nr,
        payload_offset: pos,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_with_seq() {
        let d = [0xC8, 0x02, 0, 16, 0, 7, 0, 9, 0, 1, 0, 0, 0, 0, 0, 0];
        let l = parse(&d).unwrap();
        assert!(l.control && l.has_len && l.has_seq && !l.has_offset);
        assert_eq!(l.len, Some(16));
        assert_eq!(l.ns, Some(1));
        assert_eq!(l.nr, Some(0));
        assert_eq!(l.payload_offset, 12);
    }

    #[test]
    fn data_message() {
        // T=0, no optional fields → header is 6 bytes
        let d = [0x00, 0x02, 0, 7, 0, 9, b'p', b'p', b'p'];
        let l = parse(&d).unwrap();
        assert!(!l.control && !l.has_len && !l.has_seq);
        assert_eq!(l.ns, None);
        assert_eq!(l.payload_offset, 6);
        assert!(!l.priority);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0x80, 0x03]).is_none()); // version 3
        assert!(parse(&[0xC8, 0x02, 0, 99, 0, 7, 0, 9]).is_none()); // len > input
        assert!(parse(&[0x88, 0x02]).is_none()); // S set but truncated
    }
}
