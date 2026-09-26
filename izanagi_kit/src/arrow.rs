//! Apache Arrow IPC file and stream envelopes (Arrow columnar
//! format).
//!
//! File layout: `ARROW1\0\0` magic, message stream, flatbuffer
//! `Footer`, i32 LE footer length, `ARROW1\0\0` again. Stream
//! messages: `0xFFFFFFFF` continuation + i32 metadata length (v1.0+),
//! or a legacy bare i32 length (`0` ends the stream), then the
//! flatbuffer `Message` padded to 8 bytes, then the body. Only the
//! envelope is walked — message bodies are flatbuffers.
//!
//! ```
//! use izanagi_kit::arrow::{parse, messages, MAGIC};
//! let mut d = MAGIC.to_vec();            // 8 bytes "ARROW1\0\0"
//! d.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]); // continuation
//! d.extend_from_slice(&24i32.to_le_bytes());     // meta len
//! d.extend_from_slice(&[0xAA; 24]);              // flatbuffer
//! d.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]); // continuation
//! d.extend_from_slice(&0i32.to_le_bytes());      // EOS
//! d.extend_from_slice(&[0xBB; 16]);              // footer
//! d.extend_from_slice(&16i32.to_le_bytes());     // footer len
//! d.extend_from_slice(&MAGIC);
//! let a = parse(&d).unwrap();
//! assert_eq!(a.footer_len, 16);
//! let ms = messages(&d, &a);
//! assert_eq!(ms.len(), 1);
//! assert_eq!(ms[0].meta_len, 24);
//! ```

fn le32(d: &[u8], o: usize) -> Option<u32> {
    Some(
        (*d.get(o)? as u32)
            | (*d.get(o + 1)? as u32) << 8
            | (*d.get(o + 2)? as u32) << 16
            | (*d.get(o + 3)? as u32) << 24,
    )
}

/// 8-byte `ARROW1\0\0` file magic at both ends.
pub const MAGIC: [u8; 8] = *b"ARROW1\0\0";
/// Message-stream continuation token.
pub const CONTINUATION: u32 = 0xFFFF_FFFF;

/// Parsed Arrow IPC file envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arrow {
    /// Offset of the flatbuffer `Footer`.
    pub footer_at: usize,
    /// Footer length in bytes.
    pub footer_len: u32,
    /// Offset where the message stream begins (always 8).
    pub messages_at: usize,
}

/// One message in the stream: `meta_len` bytes of flatbuffer at
/// `meta_at`; the body follows aligned to 8 bytes (its length lives
/// inside the flatbuffer — not decoded here).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Message {
    /// Metadata flatbuffer offset.
    pub meta_at: usize,
    /// Metadata flatbuffer length.
    pub meta_len: usize,
    /// Offset just past this message's aligned metadata (body start).
    pub body_at: usize,
    /// `true` when the message used the `0xFFFFFFFF` continuation
    /// prefix (the modern form).
    pub continued: bool,
}

/// Parse `ARROW1\0\0` at both ends and the i32 footer length at
/// `len - 10`. `None` on missing magic or footer overrun.
pub fn parse(d: &[u8]) -> Option<Arrow> {
    if d.len() < 8 + 4 + 8 {
        return None;
    }
    if d.get(0..8)? != MAGIC {
        return None;
    }
    if d.get(d.len() - 8..)? != MAGIC {
        return None;
    }
    let footer_len = le32(d, d.len() - 8 - 4)? as usize;
    let footer_at = (d.len() - 12).checked_sub(footer_len)?;
    if footer_at < 8 {
        return None;
    }
    Some(Arrow {
        footer_at,
        footer_len: footer_len as u32,
        messages_at: 8,
    })
}

/// Read the message header at `at`: returns `(continued, meta_len)`.
/// A bare or continued length of `0` is the end-of-stream marker.
fn message_head(d: &[u8], at: usize) -> Option<(bool, usize)> {
    let v = le32(d, at)?;
    if v == CONTINUATION {
        let l = le32(d, at + 4)?;
        return Some((true, l as usize));
    }
    Some((false, v as usize))
}

/// Walk the message stream between `a.messages_at` and `a.footer_at`.
/// Metadata is 8-byte aligned. Bodies are not decoded, so the walk is
/// exact for body-less messages (schema/EOS markers); once a message
/// carries a body the next header lands mid-body and the walk stops at
/// the first unparseable length.
pub fn messages(d: &[u8], a: &Arrow) -> Vec<Message> {
    let mut out = Vec::new();
    let mut at = a.messages_at;
    let end = a.footer_at;
    while at + 4 <= end {
        let (continued, meta_len) = match message_head(d, at) {
            Some(v) => v,
            None => break,
        };
        let meta_at = at + if continued { 8 } else { 4 };
        if meta_len == 0 {
            break;
        }
        let meta_end = match meta_at.checked_add(meta_len) {
            Some(e) if e <= end => e,
            _ => break,
        };
        // body starts at the next 8-byte boundary
        let body_at = (meta_end + 7) & !7;
        out.push(Message {
            meta_at,
            meta_len,
            body_at,
            continued,
        });
        at = body_at;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = MAGIC.to_vec();
        // msg 1: continued, meta 24B
        d.extend_from_slice(&CONTINUATION.to_le_bytes());
        d.extend_from_slice(&24i32.to_le_bytes());
        d.extend_from_slice(&[0xAA; 24]);
        // msg 2: legacy bare len 8
        d.extend_from_slice(&8i32.to_le_bytes());
        d.extend_from_slice(&[0xCC; 8]);
        // EOS via continuation + 0
        d.extend_from_slice(&CONTINUATION.to_le_bytes());
        d.extend_from_slice(&0i32.to_le_bytes());
        d.extend_from_slice(&[0xBB; 16]); // footer
        d.extend_from_slice(&16i32.to_le_bytes());
        d.extend_from_slice(&MAGIC);
        d
    }

    #[test]
    fn envelope_and_messages() {
        let d = fixture();
        let a = parse(&d).unwrap();
        assert_eq!(a.footer_len, 16);
        let ms = messages(&d, &a);
        assert_eq!(ms.len(), 2);
        assert!(ms[0].continued);
        assert_eq!(ms[0].meta_at, 16);
        assert_eq!(ms[0].meta_len, 24);
        assert_eq!(ms[0].body_at, 40);
        assert!(!ms[1].continued);
        assert_eq!(ms[1].meta_at, 44);
        assert_eq!(ms[1].meta_len, 8);
        assert_eq!(ms[1].body_at, 56); // 52 aligned to 8
    }

    #[test]
    fn rejects() {
        assert!(parse(b"ARROW1\0\0").is_none()); // too short
        let mut d = fixture();
        d[0] = b'X';
        assert!(parse(&d).is_none());
        let mut d2 = fixture();
        let n = d2.len();
        d2[n - 3] = b'Z'; // corrupt trailing '1' of the magic
        assert!(parse(&d2).is_none());
        let mut d3 = fixture();
        let n3 = d3.len();
        d3[n3 - 9] = 0xFF; // huge footer_len
        let p = parse(&d3);
        assert!(p.is_none() || p.unwrap().footer_at >= 8);
    }
}
