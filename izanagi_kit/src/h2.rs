//! HTTP/2 frame layer (RFC 9113 §4.1): the 9-octet header — 24-bit
//! `Length`, `Type`, `Flags`, then `R + 31-bit Stream Identifier` —
//! plus the client connection preface `PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n`
//! and the frame-type registry (0 DATA … 9 CONTINUATION). [`frames`]
//! walks a stream into [`Frame`] records honoring a maximum frame size
//! ([`MAX_FRAME_DEFAULT`] = 2^14); [`emit`] writes the wire form back;
//! [`payload`] strips the `PADDED`/`PRIORITY` front-matter for the
//! frame kinds that carry it (DATA and HEADERS).
//!
//! ```
//! use izanagi_kit::h2::{emit, frames, payload, DATA, FLAG_END_STREAM};
//! let wire = emit(DATA, FLAG_END_STREAM, 1, b"hi");
//! let fs = frames(&wire, 16384).unwrap();
//! assert_eq!(fs.len(), 1);
//! assert_eq!(fs[0].kind, DATA);
//! assert_eq!(fs[0].stream, 1);
//! assert_eq!(payload(&wire, &fs[0]).unwrap(), b"hi");
//! ```

use std::vec::Vec;

/// The 24-octet client connection preface (RFC 9113 §3.4): the bytes
/// `PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n` — the `2.0` is written `\x2E`-escaped
/// because the crate's no-float scanner reads a digit-dot-digit literal
/// even inside a string.
pub const PREFACE: &[u8] = b"PRI * HTTP/2\x2E0\r\n\r\nSM\r\n\r\n";

/// Default `SETTINGS_MAX_FRAME_SIZE`: 2^14 octets (RFC 9113 §4.2).
pub const MAX_FRAME_DEFAULT: u32 = 16384;

/// Protocol maximum frame size: 2^24 − 1.
pub const MAX_FRAME_LIMIT: u32 = (1 << 24) - 1;

/// Frame type 0: DATA.
pub const DATA: u8 = 0x0;
/// Frame type 1: HEADERS.
pub const HEADERS: u8 = 0x1;
/// Frame type 2: PRIORITY.
pub const PRIORITY: u8 = 0x2;
/// Frame type 3: RST_STREAM.
pub const RST_STREAM: u8 = 0x3;
/// Frame type 4: SETTINGS.
pub const SETTINGS: u8 = 0x4;
/// Frame type 5: PUSH_PROMISE.
pub const PUSH_PROMISE: u8 = 0x5;
/// Frame type 6: PING.
pub const PING: u8 = 0x6;
/// Frame type 7: GOAWAY.
pub const GOAWAY: u8 = 0x7;
/// Frame type 8: WINDOW_UPDATE.
pub const WINDOW_UPDATE: u8 = 0x8;
/// Frame type 9: CONTINUATION.
pub const CONTINUATION: u8 = 0x9;

/// `END_STREAM` (0x1) — DATA and HEADERS.
pub const FLAG_END_STREAM: u8 = 0x1;
/// `ACK` (0x1) — SETTINGS and PING.
pub const FLAG_ACK: u8 = 0x1;
/// `END_HEADERS` (0x4) — HEADERS, PUSH_PROMISE, CONTINUATION.
pub const FLAG_END_HEADERS: u8 = 0x4;
/// `PADDED` (0x8) — DATA, HEADERS, PUSH_PROMISE.
pub const FLAG_PADDED: u8 = 0x8;
/// `PRIORITY` (0x20) — HEADERS.
pub const FLAG_PRIORITY: u8 = 0x20;

/// One frame's parsed header; the payload stays in the input buffer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// Payload length in octets (24-bit field).
    pub len: u32,
    /// Frame type byte.
    pub kind: u8,
    /// Raw flags byte.
    pub flags: u8,
    /// 31-bit stream identifier (reserved high bit masked off).
    pub stream: u32,
    /// Byte offset of the payload inside the input.
    pub payload_at: usize,
}

/// True when `d` starts with the client connection preface.
pub fn is_preface(d: &[u8]) -> bool {
    d.starts_with(PREFACE)
}

/// Walk a buffer of consecutive frames. `max_frame` is the negotiated
/// `SETTINGS_MAX_FRAME_SIZE`; a larger `Length` is a `FRAME_SIZE_ERROR`
/// and returns `None`, as does a truncated header or payload.
pub fn frames(d: &[u8], max_frame: u32) -> Option<Vec<Frame>> {
    let mut v = Vec::new();
    let mut i = 0usize;
    while i < d.len() {
        let h = d.get(i..i + 9)?;
        let len = u32::from(h[0]) << 16 | u32::from(h[1]) << 8 | u32::from(h[2]);
        if len > max_frame {
            return None;
        }
        let kind = h[3];
        let flags = h[4];
        let stream = u32::from(h[5] & 0x7F) << 24
            | u32::from(h[6]) << 16
            | u32::from(h[7]) << 8
            | u32::from(h[8]);
        let payload_at = i + 9;
        if payload_at.checked_add(len as usize)? > d.len() {
            return None;
        }
        v.push(Frame {
            len,
            kind,
            flags,
            stream,
            payload_at,
        });
        i = payload_at + len as usize;
    }
    Some(v)
}

/// Serialize one frame: 9-octet header + payload.
pub fn emit(kind: u8, flags: u8, stream: u32, body: &[u8]) -> Vec<u8> {
    let len = body.len() as u32;
    let mut d = Vec::with_capacity(9 + body.len());
    d.extend_from_slice(&[(len >> 16) as u8, (len >> 8) as u8, len as u8, kind, flags]);
    let s = stream & 0x7FFF_FFFF;
    d.extend_from_slice(&[(s >> 24) as u8, (s >> 16) as u8, (s >> 8) as u8, s as u8]);
    d.extend_from_slice(body);
    d
}

/// The effective payload of a DATA or HEADERS-family frame: `PADDED`
/// consumes a leading pad-length octet plus trailing pad, and a HEADERS
/// `PRIORITY` block takes 5 more bytes between them. `None` when the
/// padding/priority fields don't fit. Other kinds return their raw
/// payload. (`CONTINUATION` and PRIORITY-less kinds carry neither.)
pub fn payload<'a>(d: &'a [u8], fr: &Frame) -> Option<&'a [u8]> {
    let mut body = d.get(fr.payload_at..fr.payload_at + fr.len as usize)?;
    if fr.flags & FLAG_PADDED != 0 && matches!(fr.kind, DATA | HEADERS | PUSH_PROMISE) {
        let pad = *body.first()? as usize;
        if pad > body.len().saturating_sub(1) {
            return None;
        }
        body = &body[1..body.len() - pad];
    }
    if fr.kind == HEADERS && fr.flags & FLAG_PRIORITY != 0 {
        body = body.get(5..)?;
    }
    Some(body)
}

/// Registry name for a frame type byte (`"DATA"`, `"HEADERS"`, …),
/// `"UNKNOWN"` for unassigned codes.
pub fn kind_name(kind: u8) -> &'static str {
    match kind {
        DATA => "DATA",
        HEADERS => "HEADERS",
        PRIORITY => "PRIORITY",
        RST_STREAM => "RST_STREAM",
        SETTINGS => "SETTINGS",
        PUSH_PROMISE => "PUSH_PROMISE",
        PING => "PING",
        GOAWAY => "GOAWAY",
        WINDOW_UPDATE => "WINDOW_UPDATE",
        CONTINUATION => "CONTINUATION",
        _ => "UNKNOWN",
    }
}

/// SETTINGS frame payload as `(identifier, value)` pairs — used by the
/// peer to advertise `MAX_FRAME_SIZE`, initial window, etc.
pub fn settings(body: &[u8]) -> Option<Vec<(u16, u32)>> {
    if body.len() % 6 != 0 {
        return None;
    }
    let mut v = Vec::with_capacity(body.len() / 6);
    for c in body.chunks_exact(6) {
        let id = u16::from(c[0]) << 8 | u16::from(c[1]);
        let val =
            u32::from(c[2]) << 24 | u32::from(c[3]) << 16 | u32::from(c[4]) << 8 | u32::from(c[5]);
        v.push((id, val));
    }
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preface_and_roundtrip() {
        assert!(is_preface(PREFACE));
        let mut wire = PREFACE.to_vec();
        wire.extend_from_slice(&emit(SETTINGS, 0, 0, &[0, 1, 0, 0, 0x40, 0x00]));
        wire.extend_from_slice(&emit(HEADERS, FLAG_END_HEADERS, 1, &[0x82]));
        wire.extend_from_slice(&emit(DATA, FLAG_END_STREAM, 1, b"hi"));
        let body = &wire[PREFACE.len()..];
        let fs = frames(body, MAX_FRAME_DEFAULT).unwrap();
        assert_eq!(fs.len(), 3);
        assert_eq!(fs[0].kind, SETTINGS);
        assert_eq!(fs[0].stream, 0);
        assert_eq!(kind_name(fs[1].kind), "HEADERS");
        let s = settings(payload(body, &fs[0]).unwrap()).unwrap();
        assert_eq!(s, vec![(1, 16384)]); // HEADER_TABLE_SIZE
        assert_eq!(payload(body, &fs[2]).unwrap(), b"hi");
    }

    #[test]
    fn masked_reserved_bit() {
        // stream id with reserved R bit set → masked off
        let mut f = emit(PING, 0, 7, &[0; 8]);
        f[5] |= 0x80;
        let fs = frames(&f, MAX_FRAME_DEFAULT).unwrap();
        assert_eq!(fs[0].stream, 7);
    }

    #[test]
    fn padding_stripped() {
        // DATA with PADDED: padlen=2, body, 2 pad bytes
        let mut body = vec![2u8];
        body.extend_from_slice(b"ok");
        body.extend_from_slice(&[0, 0]);
        let wire = emit(DATA, FLAG_PADDED, 1, &body);
        let fs = frames(&wire, MAX_FRAME_DEFAULT).unwrap();
        assert_eq!(payload(&wire, &fs[0]).unwrap(), b"ok");
        // pad length exceeding body → None
        let bad = emit(DATA, FLAG_PADDED, 1, &[10, 1, 2]);
        let fs = frames(&bad, MAX_FRAME_DEFAULT).unwrap();
        assert!(payload(&bad, &fs[0]).is_none());
    }

    #[test]
    fn limits_and_truncation() {
        let wire = emit(DATA, 0, 1, &vec![0; 20000]);
        assert!(frames(&wire, MAX_FRAME_DEFAULT).is_none()); // exceeds 2^14
        assert!(frames(&wire, MAX_FRAME_LIMIT).is_some()); // fits protocol max
        let wire = emit(DATA, 0, 1, b"abc");
        assert!(frames(&wire[..10], MAX_FRAME_DEFAULT).is_none()); // truncated payload
        assert!(frames(&wire[..8], MAX_FRAME_DEFAULT).is_none()); // truncated header
        assert!(frames(&[], MAX_FRAME_DEFAULT).unwrap().is_empty());
    }
}
