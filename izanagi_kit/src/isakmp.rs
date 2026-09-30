//! ISAKMP / IKE (RFC 2408, RFC 7296) — VPN key-exchange header.
//!
//! 28-byte header: `u64 initiator SPI/cookie | u64 responder SPI |
//! u8 next_payload | u8 version (major<<4|minor) | u8 exchange |
//! u8 flags | u32 msg_id | u32 len` (big-endian).
//!
//! ```
//! let mut d = vec![0u8; 8];
//! d.extend_from_slice(&[0u8; 8]); // responder cookie
//! d.extend_from_slice(&[33, 0x10, 34, 0, 0, 0, 0, 0, 0, 0, 0, 28]);
//! let i = izanagi_kit::isakmp::parse(&d).unwrap();
//! assert_eq!((i.major, i.minor), (1, 0));
//! assert_eq!(i.exchange, 34); // IKE_SA_INIT in IKEv2 numbering
//! ```

/// Parsed ISAKMP/IKE header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Isakmp {
    /// Initiator SPI / cookie.
    pub init_cookie: u64,
    /// Responder SPI / cookie (0 on the first message).
    pub resp_cookie: u64,
    /// Next-payload code of the first payload (0 = none).
    pub next_payload: u8,
    /// IKE major version (1 = IKEv1 ISAKMP, 2 = IKEv2).
    pub major: u8,
    /// IKE minor version.
    pub minor: u8,
    /// Exchange type.
    pub exchange: u8,
    /// Flags: bit5 E (encrypted), bit4 C (commit), bit3 I (initiator, IKEv2+).
    pub flags: u8,
    /// Message ID.
    pub msg_id: u32,
    /// Declared total message length.
    pub len: u32,
}

fn be32(d: &[u8], o: usize) -> Option<u32> {
    Some(
        ((*d.get(o)? as u32) << 24)
            | ((*d.get(o + 1)? as u32) << 16)
            | ((*d.get(o + 2)? as u32) << 8)
            | *d.get(o + 3)? as u32,
    )
}

fn be64(d: &[u8], o: usize) -> Option<u64> {
    let mut v = 0u64;
    for i in 0..8 {
        v = (v << 8) | *d.get(o + i)? as u64;
    }
    Some(v)
}

/// Parse an IKE header; `None` on short input or implausible version/length.
pub fn parse(d: &[u8]) -> Option<Isakmp> {
    if d.len() < 28 {
        return None;
    }
    let init_cookie = be64(d, 0)?;
    let resp_cookie = be64(d, 8)?;
    let next_payload = d[16];
    let major = d[17] >> 4;
    let minor = d[17] & 0x0F;
    if !(major == 1 || major == 2) {
        return None;
    }
    let exchange = d[18];
    let flags = d[19];
    let msg_id = be32(d, 20)?;
    let len = be32(d, 24)?;
    if len < 28 || len as usize > d.len() {
        return None;
    }
    Some(Isakmp {
        init_cookie,
        resp_cookie,
        next_payload,
        major,
        minor,
        exchange,
        flags,
        msg_id,
        len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ike(major: u8, minor: u8, len: u32) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(&[0, 0, 0, 0, 0xAA, 0xBB, 0xCC, 0xDD]); // init cookie
        d.extend_from_slice(&[0u8; 8]); // resp cookie
        d.push(33); // next payload SA
        d.push((major << 4) | minor);
        d.push(34); // exchange IKE_SA_INIT
        d.push(0x08); // flags: initiator
        d.extend_from_slice(&[0; 4]); // msg_id
        d.extend_from_slice(&[
            (len >> 24) as u8,
            (len >> 16) as u8,
            (len >> 8) as u8,
            len as u8,
        ]);
        d.resize(len as usize, 0);
        d
    }

    #[test]
    fn ikev2() {
        let i = parse(&ike(2, 0, 28)).unwrap();
        assert_eq!((i.major, i.minor), (2, 0));
        assert_eq!(i.init_cookie, 0xAABBCCDD);
        assert_eq!(i.resp_cookie, 0);
        assert_eq!(i.next_payload, 33);
        assert_eq!(i.exchange, 34);
        assert_eq!(i.flags & 0x08, 0x08);
        assert_eq!(i.len, 28);
    }

    #[test]
    fn ikev1() {
        let i = parse(&ike(1, 0, 40)).unwrap();
        assert_eq!(i.major, 1);
        assert_eq!(i.msg_id, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(&[0u8; 27]).is_none());
        assert!(parse(&ike(3, 0, 28)).is_none()); // version 3 unknown
        assert!(parse(&ike(2, 0, 20)).is_none()); // len < header
        let mut d = ike(2, 0, 28);
        d.truncate(28);
        d[24..28].copy_from_slice(&[0, 0, 0, 100]);
        assert!(parse(&d).is_none()); // len > input
    }
}
