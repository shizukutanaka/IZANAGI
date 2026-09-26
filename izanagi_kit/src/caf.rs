//! Core Audio Format (`.caf`) chunk walk.
//!
//! Header: `caff` + u16 version + u16 flags. Then chunks of
//! `{type:4, size:BE64, data}` — a negative size means "runs to
//! EOF" (used by `data`). The `desc` chunk holds the 32-byte
//! AudioStreamBasicDescription; the `data` chunk starts with a u32
//! edit count before the audio bytes. All fields are big-endian.
//!
//! ```
//! use izanagi_kit::caf::{parse, chunks, desc};
//! let mut d = b"caff\x00\x01\x00\x00".to_vec();
//! let mut desc_body = Vec::new();
//! desc_body.extend_from_slice(&[0x40, 0xBB, 0x80, 0, 0, 0, 0, 0]); // 44100.0 bits
//! desc_body.extend_from_slice(b"lpcm");
//! desc_body.extend_from_slice(&[0, 0, 0, 2, 0, 0, 0, 4, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 16]);
//! d.extend_from_slice(b"desc");
//! d.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 32]);
//! d.extend_from_slice(&desc_body);
//! let c = parse(&d).unwrap();
//! assert_eq!(c.version, 1);
//! let dsc = desc(&d, &c).unwrap();
//! assert_eq!(dsc.format_id, *b"lpcm");
//! assert_eq!(dsc.channels_per_frame, 2);
//! ```

fn be16(d: &[u8], o: usize) -> Option<u16> {
    Some((*d.get(o)? as u16) << 8 | *d.get(o + 1)? as u16)
}

fn be32(d: &[u8], o: usize) -> Option<u32> {
    Some(
        (*d.get(o)? as u32) << 24
            | (*d.get(o + 1)? as u32) << 16
            | (*d.get(o + 2)? as u32) << 8
            | *d.get(o + 3)? as u32,
    )
}

fn be64(d: &[u8], o: usize) -> Option<u64> {
    Some((be32(d, o)? as u64) << 32 | be32(d, o + 4)? as u64)
}

fn four(d: &[u8], o: usize) -> Option<[u8; 4]> {
    let mut v = [0u8; 4];
    v.copy_from_slice(d.get(o..o + 4)?);
    Some(v)
}

/// One CAF chunk header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
    /// Chunk type (`desc`, `data`, `chan`, `kuki`, …).
    pub kind: [u8; 4],
    /// Declared data size, or `None` when it runs to end of file.
    pub size: Option<usize>,
    /// Offset of the chunk data.
    pub at: usize,
}

/// Parsed CAF header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caf {
    /// File version (1).
    pub version: u16,
    /// Header flags.
    pub flags: u16,
}

/// The AudioStreamBasicDescription inside `desc` (float fields are
/// kept as raw big-endian bit patterns — no `f64` in the crate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Desc {
    /// `mSampleRate` as raw IEEE-754 bits.
    pub sample_rate_bits: u64,
    /// `mFormatID` (`lpcm`, `aac `, …).
    pub format_id: [u8; 4],
    /// `mFormatFlags`.
    pub format_flags: u32,
    /// `mBytesPerPacket`.
    pub bytes_per_packet: u32,
    /// `mFramesPerPacket`.
    pub frames_per_packet: u32,
    /// `mChannelsPerFrame`.
    pub channels_per_frame: u32,
    /// `mBitsPerChannel`.
    pub bits_per_channel: u32,
}

/// Parse the `caff` 8-byte header. `None` on bad magic.
pub fn parse(d: &[u8]) -> Option<Caf> {
    if four(d, 0)? != *b"caff" {
        return None;
    }
    Some(Caf {
        version: be16(d, 4)?,
        flags: be16(d, 6)?,
    })
}

/// Read the chunk header at `at`. A negative declared size maps to
/// `size: None` (runs to end of file).
pub fn chunk_at(d: &[u8], at: usize) -> Option<Chunk> {
    let kind = four(d, at)?;
    let raw = be64(d, at + 4)? as i64;
    let data = at.checked_add(12)?;
    let size = if raw < 0 {
        None
    } else {
        let n = raw as usize;
        if data.checked_add(n)? > d.len() {
            return None;
        }
        Some(n)
    };
    Some(Chunk {
        kind,
        size,
        at: data,
    })
}

/// Walk chunks until `chunk_at` fails or the buffer ends.
/// `Chunk { size: None, .. }` (data-to-EOF) terminates the walk.
pub fn chunks(d: &[u8], _c: &Caf) -> Vec<Chunk> {
    let mut out = Vec::new();
    let mut at = 8;
    while at + 12 <= d.len() {
        match chunk_at(d, at) {
            Some(c) => {
                let next = match c.size {
                    Some(n) => c.at + n,
                    None => d.len(),
                };
                out.push(c);
                at = next;
            }
            None => break,
        }
    }
    out
}

/// Decode the first `desc` chunk's 32-byte ASBD. `None` when there is
/// no `desc` chunk or it is shorter than 32 bytes.
pub fn desc(d: &[u8], c: &Caf) -> Option<Desc> {
    for ch in chunks(d, c) {
        if ch.kind == *b"desc" && ch.size.unwrap_or(0) >= 32 {
            let b = ch.at;
            return Some(Desc {
                sample_rate_bits: be64(d, b)?,
                format_id: four(d, b + 8)?,
                format_flags: be32(d, b + 12)?,
                bytes_per_packet: be32(d, b + 16)?,
                frames_per_packet: be32(d, b + 20)?,
                channels_per_frame: be32(d, b + 24)?,
                bits_per_channel: be32(d, b + 28)?,
            });
        }
    }
    None
}

/// Offset of the audio payload inside the first `data` chunk
/// (skipping its u32 edit count).
pub fn audio_at(d: &[u8], c: &Caf) -> Option<usize> {
    for ch in chunks(d, c) {
        if ch.kind == *b"data" {
            return ch.at.checked_add(4);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push_chunk(d: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
        d.extend_from_slice(kind);
        let n = body.len() as u64;
        for i in 0..8 {
            d.push((n >> ((7 - i) * 8)) as u8);
        }
        d.extend_from_slice(body);
    }

    fn fixture() -> Vec<u8> {
        let mut d = b"caff\x00\x01\x00\x00".to_vec();
        let mut desc_body = Vec::new();
        desc_body.extend_from_slice(&[0x40, 0xE5, 0x88, 0x80, 0, 0, 0, 0]); // 48000.0
        desc_body.extend_from_slice(b"lpcm");
        desc_body.extend_from_slice(&[0, 0, 0, 2, 0, 0, 0, 4, 0, 0, 0, 1]);
        desc_body.extend_from_slice(&[0, 0, 0, 2, 0, 0, 0, 16]);
        push_chunk(&mut d, b"desc", &desc_body);
        // data chunk with -1 size (runs to EOF)
        d.extend_from_slice(b"data");
        d.extend_from_slice(&[0xFF; 8]);
        d.extend_from_slice(&[0, 0, 0, 0]); // edit count
        d.extend_from_slice(b"AUDIODATA");
        d
    }

    #[test]
    fn header_chunks_desc() {
        let d = fixture();
        let c = parse(&d).unwrap();
        assert_eq!(c.version, 1);
        assert_eq!(c.flags, 0);
        let cs = chunks(&d, &c);
        assert_eq!(cs.len(), 2);
        let first = chunk_at(&d, 8).unwrap(); // chunks start after the 8-byte header
        assert_eq!(first, cs[0]);
        assert_eq!(cs[0].kind, *b"desc");
        assert_eq!(cs[0].size, Some(32));
        assert_eq!(cs[1].kind, *b"data");
        assert_eq!(cs[1].size, None); // -1 = to EOF
        let dc = desc(&d, &c).unwrap();
        assert_eq!(dc.format_id, *b"lpcm");
        assert_eq!(dc.sample_rate_bits, 0x40E5_8880_0000_0000);
        assert_eq!(dc.bytes_per_packet, 4);
        assert_eq!(dc.channels_per_frame, 2);
        assert_eq!(dc.bits_per_channel, 16);
        assert_eq!(&d[audio_at(&d, &c).unwrap()..], b"AUDIODATA");
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(b"caaf\x00\x01\x00\x00").is_none());
        // truncated chunk stops the walk
        let mut t = fixture();
        t.truncate(8 + 12 + 10);
        let c = parse(&t).unwrap();
        assert!(chunks(&t, &c).is_empty());
        assert!(desc(&t, &c).is_none());
        assert!(audio_at(&t, &c).is_none());
    }
}
