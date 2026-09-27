//! MQTT control-packet header parsing (3.1.1 / 5.0).
//!
//! Fixed header = `type:4 | flags:4` byte then the *remaining
//! length* varint (7-bit continuation, max 4 bytes). `CONNECT` is
//! decoded one step further: `Protocol Name` length-prefixed,
//! `Protocol Level` (4 = 3.1.1, 5 = 5.0), connect flags and
//! keep-alive seconds.
//!
//! ```
//! use izanagi_kit::mqtt;
//! let mut d = vec![0x10, 10]; // CONNECT, remaining len 10
//! d.extend_from_slice(&[0, 4]); // "MQTT" len
//! d.extend_from_slice(b"MQTT");
//! d.extend_from_slice(&[4, 0b0000_0010]); // level 4, clean flag
//! d.extend_from_slice(&[0, 60]); // keep alive
//! let m = mqtt::parse(&d).unwrap();
//! assert_eq!(m.ty, mqtt::Type::Connect);
//! ```

/// Packet types 1..15.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    /// 1 — CONNECT.
    Connect,
    /// 2 — CONNACK.
    Connack,
    /// 3 — PUBLISH.
    Publish,
    /// 8 — SUBSCRIBE.
    Subscribe,
    /// 9 — SUBACK.
    Suback,
    /// 12 — PINGREQ.
    Pingreq,
    /// 13 — PINGRESP.
    Pingresp,
    /// 14 — DISCONNECT.
    Disconnect,
    /// Other packet type.
    Other(u8),
}

/// A parsed MQTT packet header (+ CONNECT fields).
#[derive(Clone, Debug, PartialEq)]
pub struct Mqtt {
    /// Packet type nibble.
    pub ty: Type,
    /// Fixed-header flags nibble.
    pub flags: u8,
    /// Remaining-length value.
    pub remaining_len: u32,
    /// CONNECT: protocol level (4/5) or 0.
    pub protocol_level: u8,
    /// CONNECT: keep-alive seconds.
    pub keep_alive: u16,
    /// CONNECT: connect-flags byte.
    pub connect_flags: u8,
}

fn ty(t: u8) -> Type {
    match t {
        1 => Type::Connect,
        2 => Type::Connack,
        3 => Type::Publish,
        8 => Type::Subscribe,
        9 => Type::Suback,
        12 => Type::Pingreq,
        13 => Type::Pingresp,
        14 => Type::Disconnect,
        o => Type::Other(o),
    }
}

fn varint(d: &[u8], at: usize) -> Option<(u32, usize)> {
    let mut v = 0u32;
    let mut shift = 0u32;
    let mut i = at;
    loop {
        if i - at >= 4 {
            return None;
        }
        let b = *d.get(i)?;
        i += 1;
        v |= ((b & 0x7F) as u32) << shift;
        shift += 7;
        if b & 0x80 == 0 {
            return Some((v, i - at));
        }
    }
}

/// Parses one MQTT packet: type nibble ≠ 0, remaining-length varint
/// must fit the input exactly or be a prefix.
pub fn parse(d: &[u8]) -> Option<Mqtt> {
    let b0 = *d.first()?;
    let t = b0 >> 4;
    if t == 0 {
        return None;
    }
    let (remaining_len, vlen) = varint(d, 1)?;
    let head = 1 + vlen;
    if head.checked_add(remaining_len as usize)? > d.len() {
        return None;
    }
    let mut m = Mqtt {
        ty: ty(t),
        flags: b0 & 0x0F,
        remaining_len,
        protocol_level: 0,
        keep_alive: 0,
        connect_flags: 0,
    };
    if m.ty == Type::Connect {
        // variable header: u16 len + "MQTT", level, flags, keepalive
        let at = head;
        let plen = ((*d.get(at)? as usize) << 8) | *d.get(at + 1)? as usize;
        if plen != 4 || d.get(at + 2..at + 6)? != b"MQTT" {
            return None;
        }
        m.protocol_level = *d.get(at + 6)?;
        m.connect_flags = *d.get(at + 7)?;
        m.keep_alive = ((*d.get(at + 8)? as u16) << 8) | *d.get(at + 9)? as u16;
    }
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    fn connect() -> Vec<u8> {
        let mut d = vec![0x10, 12];
        d.extend_from_slice(&[0, 4]);
        d.extend_from_slice(b"MQTT");
        d.extend_from_slice(&[5, 0b10]); // level 5 + clean start
        d.extend_from_slice(&[0, 60]);
        d.extend_from_slice(&[0]); // payload start (client id len)
        d.extend_from_slice(&[0]);
        d
    }

    #[test]
    fn parses_connect() {
        let m = parse(&connect()).unwrap();
        assert_eq!(m.ty, Type::Connect);
        assert_eq!(m.protocol_level, 5);
        assert_eq!(m.keep_alive, 60);
        assert_eq!(m.remaining_len, 12);
    }

    #[test]
    fn varint_header() {
        // remaining length 321 = 0xC1 0x02 (payload padded to fit)
        let mut d = vec![0x30, 0xC1, 0x02];
        d.extend_from_slice(&vec![0u8; 321]);
        let m = parse(&d).unwrap();
        assert_eq!(m.ty, Type::Publish);
        assert_eq!(m.remaining_len, 321);
        // too-long varint
        assert!(parse(&[0x30, 0xFF, 0xFF, 0xFF, 0xFF, 0x01]).is_none());
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0x00, 0x00]).is_none()); // type 0
        let mut d = connect();
        d[7] = b'X'; // corrupt "MQTT"
        assert!(parse(&d).is_none());
    }
}
