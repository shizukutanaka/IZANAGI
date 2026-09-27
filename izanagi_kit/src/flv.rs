//! Flash Video (`.flv`) tag stream walk.
//!
//! Header: `FLV` + version byte + flags byte (`bit0` video, `bit2`
//! audio) + BE32 header size (9) + `PreviousTagSize0` (0). Each tag:
//! type u8 (8 audio / 9 video / 18 script), 24-bit BE data size,
//! 24-bit BE timestamp + u8 extended byte, 24-bit stream id, data,
//! then a BE32 `PreviousTagSize`.
//!
//! ```
//! use izanagi_kit::flv::{parse, tags, TagKind};
//! let mut d = b"FLV\x01\x05".to_vec();      // audio + video
//! d.extend_from_slice(&[0, 0, 0, 9, 0, 0, 0, 0]); // offset, prev0
//! // tag 0: video, 4-byte payload at t=0
//! d.extend_from_slice(&[9, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0]);
//! d.extend_from_slice(b"ABCD");
//! d.extend_from_slice(&[0, 0, 0, 15]); // prev tag size = 11+4
//! let f = parse(&d).unwrap();
//! assert!(f.has_video());
//! let ts = tags(&d, &f);
//! assert_eq!(ts.len(), 1);
//! assert_eq!(ts[0].kind, TagKind::Video);
//! ```

/// Tag type byte for script-data (`onMetaData` etc.) tags.
pub const TAG_SCRIPT: u8 = 18;
/// Tag type byte for audio tags.
pub const TAG_AUDIO: u8 = 8;
/// Tag type byte for video tags.
pub const TAG_VIDEO: u8 = 9;

/// The kind of an FLV tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagKind {
    /// `8` — audio packet.
    Audio,
    /// `9` — video packet.
    Video,
    /// `18` — script data (metadata).
    Script,
    /// Any other tag type byte.
    Other(u8),
}

/// One tag header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tag {
    /// Classified tag type.
    pub kind: TagKind,
    /// Declared payload size in bytes.
    pub size: usize,
    /// 32-bit timestamp in milliseconds (lower 24 bits + extended byte).
    pub timestamp: u32,
    /// Stream id (usually 0).
    pub stream_id: u32,
    /// Offset of the tag payload.
    pub at: usize,
}

/// Parsed FLV header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flv {
    /// Format version byte (always 1 in practice).
    pub version: u8,
    /// Raw flags byte.
    pub flags: u8,
    /// Offset where the first tag begins (`header_size + 4`).
    pub tags_at: usize,
}

impl Flv {
    /// `bit2` — an audio track is present.
    pub fn has_audio(&self) -> bool {
        self.flags & 0x04 != 0
    }
    /// `bit0` — a video track is present.
    pub fn has_video(&self) -> bool {
        self.flags & 0x01 != 0
    }
}

/// Parse the 13-byte FLV header. `None` on bad magic, a header size
/// below 9, or a buffer shorter than `header_size + 4`.
pub fn parse(d: &[u8]) -> Option<Flv> {
    if d.get(0..3)? != b"FLV" {
        return None;
    }
    let version = *d.get(3)?;
    let flags = *d.get(4)?;
    let header_size = (*d.get(5)? as usize) << 24
        | (*d.get(6)? as usize) << 16
        | (*d.get(7)? as usize) << 8
        | *d.get(8)? as usize;
    if header_size < 9 {
        return None;
    }
    let tags_at = header_size.checked_add(4)?; // skip PreviousTagSize0
    if tags_at > d.len() {
        return None;
    }
    Some(Flv {
        version,
        flags,
        tags_at,
    })
}

fn be24(d: &[u8], o: usize) -> Option<u32> {
    Some((*d.get(o)? as u32) << 16 | (*d.get(o + 1)? as u32) << 8 | *d.get(o + 2)? as u32)
}

/// Read the tag header at `at`; `None` on truncation.
pub fn tag_at(d: &[u8], at: usize) -> Option<Tag> {
    let ty = *d.get(at)?;
    let size = be24(d, at + 1)? as usize;
    let ts = be24(d, at + 4)? | (*d.get(at + 7)? as u32) << 24;
    let stream_id = be24(d, at + 8)?;
    let payload = at.checked_add(11)?;
    if payload.checked_add(size)? > d.len() {
        return None;
    }
    Some(Tag {
        kind: match ty {
            TAG_AUDIO => TagKind::Audio,
            TAG_VIDEO => TagKind::Video,
            TAG_SCRIPT => TagKind::Script,
            other => TagKind::Other(other),
        },
        size,
        timestamp: ts,
        stream_id,
        at: payload,
    })
}

/// Walk every tag until `tag_at` fails or the buffer ends. The
/// `PreviousTagSize` words are skipped (their values are unchecked).
pub fn tags(d: &[u8], f: &Flv) -> Vec<Tag> {
    let mut out = Vec::new();
    let mut at = f.tags_at;
    while at + 11 <= d.len() {
        match tag_at(d, at) {
            Some(t) => {
                out.push(t);
                at = t.at + t.size + 4; // skip PreviousTagSize
            }
            None => break,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_tag(d: &mut Vec<u8>, ty: u8, payload: &[u8], ts: u32) {
        let n = payload.len();
        d.push(ty);
        d.extend_from_slice(&[(n >> 16) as u8, (n >> 8) as u8, n as u8]);
        d.extend_from_slice(&[
            (ts >> 16) as u8,
            (ts >> 8) as u8,
            ts as u8,
            (ts >> 24) as u8,
        ]);
        d.extend_from_slice(&[0, 0, 0]);
        d.extend_from_slice(payload);
        let total = (11 + n) as u32;
        d.extend_from_slice(&[
            (total >> 24) as u8,
            (total >> 16) as u8,
            (total >> 8) as u8,
            total as u8,
        ]);
    }

    fn fixture() -> Vec<u8> {
        let mut d = b"FLV\x01\x05".to_vec();
        d.extend_from_slice(&[0, 0, 0, 9, 0, 0, 0, 0]);
        push_tag(&mut d, 9, b"ABCD", 0);
        push_tag(&mut d, 8, b"EF", 16_777_215); // low ts
        push_tag(&mut d, 18, b"onMetaData", 0x01_00_00_00); // ext byte
        d
    }

    #[test]
    fn header_and_tags() {
        let d = fixture();
        let f = parse(&d).unwrap();
        assert_eq!(f.version, 1);
        assert!(f.has_audio() && f.has_video());
        assert_eq!(f.tags_at, 13);
        let ts = tags(&d, &f);
        assert_eq!(ts.len(), 3);
        let first = tag_at(&d, f.tags_at).unwrap();
        assert_eq!(first, ts[0]);
        assert_eq!(ts[0].kind, TagKind::Video);
        assert_eq!(ts[0].at, 24);
        assert_eq!(&d[ts[0].at..ts[0].at + 4], b"ABCD");
        assert_eq!(ts[1].kind, TagKind::Audio);
        assert_eq!(ts[1].timestamp, 0x00FF_FFFF);
        assert_eq!(ts[2].kind, TagKind::Script);
        assert_eq!(ts[2].timestamp, 0x0100_0000);
        assert_eq!(ts[2].stream_id, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"FLX\x01\x05\0\0\0\x09\0\0\0\0").is_none());
        // header size below 9
        assert!(parse(b"FLV\x01\x05\0\0\0\x08\0\0\0\0").is_none());
        // buffer shorter than declared header
        assert!(parse(b"FLV\x01\x05\0\0\x01\0").is_none());
        let mut d = fixture();
        d.truncate(20); // inside tag 0 header
        let f = parse(&d).unwrap();
        assert!(tags(&d, &f).is_empty());
    }
}
