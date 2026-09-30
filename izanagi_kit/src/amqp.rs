//! AMQP 0-9-1 wire frames (OASIS): an optional protocol header
//! `AMQP \x00 \x00 \x09 \x01`, then frames of
//! `[type u8][channel u16][size u32][payload][0xCE]` — type 1 =
//! method, 2 = content header, 3 = body, 4 = heartbeat. Method frames
//! start with `class_id u16` + `method_id u16`.
//!
//! ```
//! let mut d = b"AMQP\x00\x00\x09\x01".to_vec();
//! d.extend_from_slice(&[1, 0, 0, 0, 0, 0, 4]); // method frame ch0 len4
//! d.extend_from_slice(&[0, 10, 0, 10]); // class 10 method 10
//! d.push(0xCE);
//! let a = izanagi_kit::amqp::parse(&d).unwrap();
//! assert!(a.header_seen);
//! assert_eq!(a.frames[0].kind, izanagi_kit::amqp::Kind::Method);
//! assert_eq!(a.frames[0].class_id, Some(10));
//! ```

use std::vec::Vec;

/// Frame type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Type 1 — RPC method (class_id/method_id lead the payload).
    Method,
    /// Type 2 — content header.
    Header,
    /// Type 3 — content body.
    Body,
    /// Type 4 — heartbeat.
    Heartbeat,
    /// Any other reserved type byte.
    Other(u8),
}

/// One AMQP frame.
#[derive(Clone, Debug)]
pub struct Frame {
    /// Frame classification.
    pub kind: Kind,
    /// Channel number.
    pub channel: u16,
    /// Payload byte offset inside the input.
    pub payload_offset: usize,
    /// Payload length.
    pub payload_len: usize,
    /// `class_id` for method frames.
    pub class_id: Option<u16>,
    /// `method_id` for method frames.
    pub method_id: Option<u16>,
}

/// A parsed AMQP stream.
#[derive(Clone, Debug)]
pub struct Amqp {
    /// Whether the `AMQP\x00\x00\x09\x01` protocol header was seen.
    pub header_seen: bool,
    /// Frames in order.
    pub frames: Vec<Frame>,
}

/// Parse an AMQP stream; `None` on a truncated frame or missing
/// `0xCE` frame-end marker.
pub fn parse(d: &[u8]) -> Option<Amqp> {
    let mut off = 0usize;
    let header_seen = d.get(..8) == Some(b"AMQP\x00\x00\x09\x01");
    if header_seen {
        off = 8;
    }
    if off == d.len() {
        return None;
    }
    let mut frames = Vec::new();
    while off < d.len() {
        if d.len() - off < 8 {
            return None;
        }
        let ty = d[off];
        let channel = (u16::from(d[off + 1]) << 8) | u16::from(d[off + 2]);
        let size = ((u32::from(d[off + 3]) << 24)
            | (u32::from(d[off + 4]) << 16)
            | (u32::from(d[off + 5]) << 8)
            | u32::from(d[off + 6])) as usize;
        let payload_off = off + 7;
        if size > d.len() - payload_off - 1 {
            return None;
        }
        if d[payload_off + size] != 0xCE {
            return None;
        }
        let (class_id, method_id) = if ty == 1 {
            if size < 4 {
                return None;
            }
            let p = payload_off;
            (
                Some((u16::from(d[p]) << 8) | u16::from(d[p + 1])),
                Some((u16::from(d[p + 2]) << 8) | u16::from(d[p + 3])),
            )
        } else {
            (None, None)
        };
        frames.push(Frame {
            kind: match ty {
                1 => Kind::Method,
                2 => Kind::Header,
                3 => Kind::Body,
                4 => Kind::Heartbeat,
                o => Kind::Other(o),
            },
            channel,
            payload_offset: payload_off,
            payload_len: size,
            class_id,
            method_id,
        });
        off = payload_off + size + 1;
    }
    Some(Amqp {
        header_seen,
        frames,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ty: u8, ch: u16, payload: &[u8]) -> Vec<u8> {
        let mut v = vec![
            ty,
            (ch >> 8) as u8,
            ch as u8,
            (payload.len() >> 24) as u8,
            (payload.len() >> 16) as u8,
            (payload.len() >> 8) as u8,
            payload.len() as u8,
        ];
        v.extend_from_slice(payload);
        v.push(0xCE);
        v
    }

    #[test]
    fn frames() {
        let mut d = Vec::new();
        d.extend_from_slice(&frame(1, 1, &[0, 60, 0, 40, 0])); // basic.publish
        d.extend_from_slice(&frame(4, 0, &[])); // heartbeat
        let a = parse(&d).unwrap();
        assert!(!a.header_seen);
        assert_eq!(a.frames.len(), 2);
        assert_eq!(a.frames[0].channel, 1);
        assert_eq!(a.frames[0].class_id, Some(60));
        assert_eq!(a.frames[0].method_id, Some(40));
        assert_eq!(a.frames[1].kind, Kind::Heartbeat);
        assert_eq!(a.frames[1].payload_len, 0);
    }

    #[test]
    fn header_then_frames() {
        let mut d = b"AMQP\x00\x00\x09\x01".to_vec();
        d.extend_from_slice(&frame(3, 9, b"body"));
        let a = parse(&d).unwrap();
        assert!(a.header_seen);
        assert_eq!(a.frames[0].kind, Kind::Body);
        assert_eq!(a.frames[0].channel, 9);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        // no frame-end
        let mut d = frame(1, 0, &[0, 60, 0, 40]);
        *d.last_mut().unwrap() = 0x00;
        assert!(parse(&d).is_none());
        // truncated size
        let mut d = frame(1, 0, &[0, 60, 0, 40]);
        d[6] = 200;
        assert!(parse(&d).is_none());
        // method frame too short for class/method
        let d = frame(1, 0, &[0, 60]);
        assert!(parse(&d).is_none());
        // bare protocol header alone is not a stream
        assert!(parse(b"AMQP\x00\x00\x09\x01").is_none());
    }
}
