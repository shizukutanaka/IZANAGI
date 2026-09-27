//! id Software RoQ video container parsing.
//!
//! A RoQ file is a chain of 8-byte chunk headers
//! `{id u16LE, size u32LE, arg u16LE}` + `size` payload bytes; the
//! first header is the magic chunk `{0x1084, 0xFFFFFFFF, 0}`. Chunk
//! `0x1001` carries `width u16, height u16, maxx u16, maxy u16` in
//! its payload. This module walks the chain and validates tiling.
//!
//! ```
//! use izanagi_kit::roq;
//! // magic chunk header {0x1084, 0xFFFFFFFF, 0}
//! let mut d = 0x1084u16.to_le_bytes().to_vec();
//! d.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
//! d.extend_from_slice(&0u16.to_le_bytes());
//! d.extend_from_slice(&0x1001u16.to_le_bytes()); // video info chunk
//! d.extend_from_slice(&8u32.to_le_bytes()); // size
//! d.extend_from_slice(&0u16.to_le_bytes()); // arg
//! d.extend_from_slice(&320u16.to_le_bytes()); // width
//! d.extend_from_slice(&200u16.to_le_bytes()); // height
//! d.extend_from_slice(&16u16.to_le_bytes()); // maxx
//! d.extend_from_slice(&16u16.to_le_bytes()); // maxy
//! let r = roq::parse(&d).unwrap();
//! assert_eq!(r.chunks.len(), 1);
//! ```

use std::vec::Vec;

/// RoQ magic word `0x1084` (little-endian at file start).
pub const MAGIC: u16 = 0x1084;

/// Chunk `id` classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `0x1001` video info.
    VideoInfo,
    /// `0x1002` quad codebook.
    Codebook,
    /// `0x1011`/`0x1012` video frame.
    VideoFrame,
    /// `0x1020`/`0x1021`/`0x1030` audio.
    Audio,
    /// Unknown id.
    Other(u16),
}

fn kind(id: u16) -> Kind {
    match id {
        0x1001 => Kind::VideoInfo,
        0x1002 => Kind::Codebook,
        0x1011 | 0x1012 => Kind::VideoFrame,
        0x1020 | 0x1021 | 0x1030 => Kind::Audio,
        o => Kind::Other(o),
    }
}

/// One RoQ chunk header.
#[derive(Clone, Debug, PartialEq)]
pub struct Chunk {
    /// Raw `id`.
    pub id: u16,
    /// Classified kind.
    pub kind: Kind,
    /// `size` — payload byte count.
    pub size: u32,
    /// `arg` — encoding/flags hint.
    pub arg: u16,
    /// Payload offset in the input.
    pub data_offset: usize,
}

/// A parsed RoQ stream.
#[derive(Clone, Debug, PartialEq)]
pub struct Roq {
    /// Chunk list (video-info chunk included).
    pub chunks: Vec<Chunk>,
    /// `width`/`height` from the `0x1001` payload when present.
    pub width: u16,
    /// Video height.
    pub height: u16,
}

fn u16le(d: &[u8], at: usize) -> Option<u16> {
    Some(*d.get(at)? as u16 | (*d.get(at + 1)? as u16) << 8)
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some(s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

/// Parses a RoQ file: magic word, then `{id,size,arg}` chunk headers
/// tiling the input. `size` 0xFFFFFFFF ends the stream.
pub fn parse(d: &[u8]) -> Option<Roq> {
    if d.len() < 8 {
        return None;
    }
    if u16le(d, 0)? != MAGIC {
        return None;
    }
    let mut at = 8usize;
    let mut chunks = Vec::new();
    let (mut width, mut height) = (0u16, 0u16);
    while at < d.len() {
        let id = u16le(d, at)?;
        let size = u32le(d, at + 2)?;
        let arg = u16le(d, at + 6)?;
        at += 8;
        if size == 0xFFFF_FFFF {
            chunks.push(Chunk {
                id,
                kind: kind(id),
                size,
                arg,
                data_offset: at,
            });
            break;
        }
        let data_offset = at;
        let end = at.checked_add(size as usize)?;
        if end > d.len() {
            return None;
        }
        if id == 0x1001 && size >= 8 {
            width = u16le(d, data_offset)?;
            height = u16le(d, data_offset + 2)?;
        }
        chunks.push(Chunk {
            id,
            kind: kind(id),
            size,
            arg,
            data_offset,
        });
        at = end;
    }
    Some(Roq {
        chunks,
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = Vec::new();
        // magic chunk header: id 0x1084, size 0xFFFFFFFF, arg 0
        d.extend_from_slice(&0x1084u16.to_le_bytes());
        d.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        d.extend_from_slice(&0u16.to_le_bytes());
        // video info chunk: id 0x1001, size 8, arg 0
        d.extend_from_slice(&0x1001u16.to_le_bytes());
        d.extend_from_slice(&8u32.to_le_bytes());
        d.extend_from_slice(&0u16.to_le_bytes());
        d.extend_from_slice(&320u16.to_le_bytes());
        d.extend_from_slice(&200u16.to_le_bytes());
        d.extend_from_slice(&16u16.to_le_bytes());
        d.extend_from_slice(&16u16.to_le_bytes());
        // video frame: id 0x1011, size 4, arg 0
        d.extend_from_slice(&0x1011u16.to_le_bytes());
        d.extend_from_slice(&4u32.to_le_bytes());
        d.extend_from_slice(&0u16.to_le_bytes());
        d.extend_from_slice(&[0xAA; 4]);
        d
    }

    #[test]
    fn parses_chunks() {
        let r = parse(&fixture()).unwrap();
        assert_eq!(r.chunks.len(), 2);
        assert_eq!(r.chunks[0].kind, Kind::VideoInfo);
        assert_eq!(r.width, 320);
        assert_eq!(r.height, 200);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 4]).is_none());
        let mut d = fixture();
        d[0] = 0x85; // wrong magic low byte
        assert!(parse(&d).is_none());
        let mut d = fixture();
        d[12..16].copy_from_slice(&9999u32.to_le_bytes()); // chunk size too big
        assert!(parse(&d).is_none());
    }
}
