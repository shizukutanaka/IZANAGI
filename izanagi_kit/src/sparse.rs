//! Android sparse image (`.simg`) chunk-directory parsing.
//!
//! 28-byte LE file header: magic `0xED26FF3A`, `major`/`minor` (u16,
//! v1.0), `file_hdr_sz` (≥28), `chunk_hdr_sz` (≥12), `blk_sz`, then
//! `total_blks`/`total_chunks`. Each chunk: `{type u16, reserved u16,
//! chunk_sz u32, total_sz u32}` — RAW `0xCAC1`, FILL `0xCAC2`,
//! DONT_CARE `0xCAC3`, CRC32 `0xCAC4`.
//!
//! ```
//! use izanagi_kit::sparse;
//! let mut d = Vec::new();
//! d.extend_from_slice(&0xed26ff3au32.to_le_bytes()); // magic
//! d.extend_from_slice(&[1, 0, 0, 0]); // major, minor
//! d.extend_from_slice(&28u16.to_le_bytes()); // file_hdr_sz
//! d.extend_from_slice(&12u16.to_le_bytes()); // chunk_hdr_sz
//! d.extend_from_slice(&4096u32.to_le_bytes()); // blk_sz
//! d.extend_from_slice(&2u32.to_le_bytes()); // total_blks
//! d.extend_from_slice(&2u32.to_le_bytes()); // total_chunks
//! d.extend_from_slice(&0u32.to_le_bytes()); // checksum
//! // RAW chunk: 1 block, total = 12 + 4096
//! d.extend_from_slice(&0xCAC1u16.to_le_bytes());
//! d.extend_from_slice(&0u16.to_le_bytes());
//! d.extend_from_slice(&1u32.to_le_bytes());
//! d.extend_from_slice(&4108u32.to_le_bytes());
//! d.extend_from_slice(&vec![0u8; 4096]);
//! // DONT_CARE chunk: 1 block, header only
//! d.extend_from_slice(&0xCAC3u16.to_le_bytes());
//! d.extend_from_slice(&0u16.to_le_bytes());
//! d.extend_from_slice(&1u32.to_le_bytes());
//! d.extend_from_slice(&12u32.to_le_bytes());
//! let s = sparse::parse(&d).unwrap();
//! assert_eq!(s.chunks[1].kind, sparse::Kind::DontCare);
//! ```

use std::vec::Vec;

/// File magic (little-endian).
pub const MAGIC: u32 = 0xED26_FF3A;

/// Sparse chunk type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `0xCAC1` — literal data follows.
    Raw,
    /// `0xCAC2` — one 4-byte fill value repeats `chunk_sz` blocks.
    Fill,
    /// `0xCAC3` — hole.
    DontCare,
    /// `0xCAC4` — file checksum chunk.
    Crc32,
    /// Unrecognised chunk type.
    Other(u16),
}

impl Kind {
    fn from_u16(v: u16) -> Kind {
        match v {
            0xCAC1 => Kind::Raw,
            0xCAC2 => Kind::Fill,
            0xCAC3 => Kind::DontCare,
            0xCAC4 => Kind::Crc32,
            o => Kind::Other(o),
        }
    }
}

/// One chunk directory entry.
#[derive(Clone, Debug, PartialEq)]
pub struct Chunk {
    /// Chunk kind.
    pub kind: Kind,
    /// Blocks (of `blk_sz` bytes) the chunk expands to.
    pub blocks: u32,
    /// Total on-disk size including the 12-byte chunk header.
    pub total_size: u32,
    /// Byte offset of the chunk's data (after the 12-byte header).
    pub data_offset: usize,
}

/// A parsed sparse image header + chunk directory.
#[derive(Clone, Debug, PartialEq)]
pub struct Sparse {
    /// Header version (usually (1,0)).
    pub version: (u16, u16),
    /// `file_hdr_sz`.
    pub file_hdr_size: u16,
    /// `blk_sz` output block size.
    pub block_size: u32,
    /// `total_blks` output block count.
    pub total_blocks: u32,
    /// `total_chunks` from the header.
    pub declared_chunks: u32,
    /// Parsed chunks.
    pub chunks: Vec<Chunk>,
}

fn u16le(d: &[u8], at: usize) -> Option<u16> {
    Some(*d.get(at)? as u16 | (*d.get(at + 1)? as u16) << 8)
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some(s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24)
}

/// Parses a sparse image; chunk `total_sz` fields must tile the input.
pub fn parse(d: &[u8]) -> Option<Sparse> {
    if d.len() < 28 {
        return None;
    }
    if u32le(d, 0)? != MAGIC {
        return None;
    }
    let major = u16le(d, 4)?;
    let minor = u16le(d, 6)?;
    if major != 1 {
        return None;
    }
    let file_hdr = u16le(d, 8)? as usize;
    let chunk_hdr = u16le(d, 10)? as usize;
    if file_hdr < 28 || file_hdr > d.len() || chunk_hdr < 12 {
        return None;
    }
    let blk_sz = u32le(d, 12)?;
    let total_blks = u32le(d, 16)?;
    let total_chunks = u32le(d, 20)?;
    let mut chunks = Vec::new();
    let mut at = file_hdr;
    for _ in 0..total_chunks {
        let ty = u16le(d, at)?;
        let blocks = u32le(d, at + 4)?;
        let total = u32le(d, at + 8)? as usize;
        if total < chunk_hdr || at.checked_add(total)? > d.len() {
            return None;
        }
        chunks.push(Chunk {
            kind: Kind::from_u16(ty),
            blocks,
            total_size: total as u32,
            data_offset: at + chunk_hdr,
        });
        at += total;
    }
    if at != d.len() {
        return None;
    }
    Some(Sparse {
        version: (major, minor),
        file_hdr_size: file_hdr as u16,
        block_size: blk_sz,
        total_blocks: total_blks,
        declared_chunks: total_chunks,
        chunks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = Vec::new();
        d.extend_from_slice(&MAGIC.to_le_bytes());
        d.extend_from_slice(&[1, 0, 0, 0]);
        d.extend_from_slice(&28u16.to_le_bytes());
        d.extend_from_slice(&12u16.to_le_bytes());
        d.extend_from_slice(&4096u32.to_le_bytes());
        d.extend_from_slice(&1u32.to_le_bytes()); // total_blks
        d.extend_from_slice(&1u32.to_le_bytes()); // total_chunks
        d.extend_from_slice(&0u32.to_le_bytes());
        // FILL chunk: 1 block, total 16 (12 + 4B fill value)
        d.extend_from_slice(&0xCAC2u16.to_le_bytes());
        d.extend_from_slice(&0u16.to_le_bytes());
        d.extend_from_slice(&1u32.to_le_bytes());
        d.extend_from_slice(&16u32.to_le_bytes());
        d.extend_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        d
    }

    #[test]
    fn parses_chunks() {
        let s = parse(&fixture()).unwrap();
        assert_eq!(s.version, (1, 0));
        assert_eq!(s.block_size, 4096);
        assert_eq!(s.chunks.len(), 1);
        assert_eq!(s.chunks[0].kind, Kind::Fill);
        assert_eq!(s.chunks[0].data_offset, 40);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 20]).is_none());
        let mut d = fixture();
        d[0] = 0xFF;
        assert!(parse(&d).is_none());
        let mut d = fixture();
        d.pop(); // truncated chunk body
        assert!(parse(&d).is_none());
    }
}
