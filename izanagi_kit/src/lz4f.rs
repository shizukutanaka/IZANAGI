//! LZ4 frame — the container around the `lz4` block codec (the block
//! format lives in `crate::lz4`). A frame opens with the u32LE magic
//! `0x184D2204`, then FLG (bits 7–6 version `01`, bit 5 block
//! independence, bit 4 block checksum, bit 3 content size present,
//! bit 2 content checksum, bit 0 dict ID present) and BD (bits 6–4
//! pick the max block size: 4=64 KiB … 7=4 MiB), the optional u64
//! content size / u32 dict ID, and a header-checksum byte. Data
//! blocks then follow as `{u32 size}` — bit 31 set means the payload
//! is stored uncompressed — ending in a u32 `0` end marker.
//!
//! ```
//! use izanagi_kit::lz4f::{parse, blocks};
//! let mut d = 0x184D2204u32.to_le_bytes().to_vec();
//! d.push(0x60); // version 1, block-independence
//! d.push(0x70); // max block 4 MiB
//! d.push(0x00); // header checksum (unchecked here)
//! d.extend_from_slice(&5u32.to_le_bytes());
//! d.extend_from_slice(b"hello");
//! d.extend_from_slice(&0u32.to_le_bytes()); // end mark
//! let f = parse(&d).unwrap();
//! assert_eq!(f.block_max, 0x400000);
//! assert_eq!(blocks(&f, &d).next().unwrap().size, 5);
//! ```

/// Little-endian frame magic.
pub const MAGIC: u32 = 0x184D_2204;
/// Skippable-frame magic range (`0x184D2A50`..`0x184D2A5F`).
pub const SKIPPABLE_MIN: u32 = 0x184D_2A50;

/// A parsed frame header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lz4f {
    /// FLG version bits (must be 1).
    pub version: u8,
    /// Block-independence flag.
    pub block_independence: bool,
    /// Block-checksum flag.
    pub block_checksum: bool,
    /// Content-checksum flag.
    pub content_checksum: bool,
    /// Declared content size when the flag is set.
    pub content_size: Option<u64>,
    /// Dictionary ID when the flag is set.
    pub dict_id: Option<u32>,
    /// Maximum block size in bytes (64 KiB … 4 MiB).
    pub block_max: u32,
    /// Byte offset of the first block header.
    pub data_at: usize,
}

/// One data block's header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    /// Byte offset of the block's `u32` size word.
    pub at: usize,
    /// Declared payload size (low 31 bits).
    pub size: u32,
    /// True when stored uncompressed (bit 31).
    pub uncompressed: bool,
}

fn u32l(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn u64l(d: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(d.get(at..at + 8)?.try_into().ok()?))
}

/// Parse the frame header; `None` without the magic or version 1.
pub fn parse(d: &[u8]) -> Option<Lz4f> {
    if u32l(d, 0)? != MAGIC {
        return None;
    }
    let flg = *d.get(4)?;
    let bd = *d.get(5)?;
    let version = flg >> 6;
    if version != 1 {
        return None;
    }
    let block_max = match (bd >> 4) & 0x7 {
        4 => 64 * 1024,
        5 => 256 * 1024,
        6 => 1024 * 1024,
        7 => 4 * 1024 * 1024,
        _ => return None,
    };
    let mut at = 6usize;
    let content_size = if flg & 0x08 != 0 {
        let v = u64l(d, at)?;
        at += 8;
        Some(v)
    } else {
        None
    };
    let dict_id = if flg & 0x01 != 0 {
        let v = u32l(d, at)?;
        at += 4;
        Some(v)
    } else {
        None
    };
    at = at.checked_add(1)?; // header checksum byte
    d.get(..at)?;
    Some(Lz4f {
        version,
        block_independence: flg & 0x20 != 0,
        block_checksum: flg & 0x10 != 0,
        content_checksum: flg & 0x04 != 0,
        content_size,
        dict_id,
        block_max,
        data_at: at,
    })
}

/// Iterate block headers until the `0` end marker or EOF.
pub fn blocks<'d>(f: &'d Lz4f, d: &'d [u8]) -> impl Iterator<Item = Block> + 'd {
    let mut at = f.data_at;
    core::iter::from_fn(move || {
        let raw = u32l(d, at)?;
        if raw == 0 {
            return None; // end mark
        }
        let b = Block {
            at,
            size: raw & 0x7FFF_FFFF,
            uncompressed: raw & 0x8000_0000 != 0,
        };
        at = at
            .checked_add(4 + b.size as usize)?
            .checked_add(usize::from(f.block_checksum) * 4)?;
        Some(b)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = MAGIC.to_le_bytes().to_vec();
        d.push(0x60); // v1 + block-independence
        d.push(0x70); // 4 MiB max
        d.push(0); // hc
        d.extend_from_slice(&5u32.to_le_bytes());
        d.extend_from_slice(b"hello");
        d.extend_from_slice(&0x8000_0003u32.to_le_bytes()); // uncompressed block
        d.extend_from_slice(b"raw");
        d.extend_from_slice(&0u32.to_le_bytes()); // end mark
        d
    }

    #[test]
    fn parses_header() {
        let f = parse(&fixture()).unwrap();
        assert_eq!(f.version, 1);
        assert!(f.block_independence);
        assert_eq!(f.block_max, 4 * 1024 * 1024);
        assert!(f.content_size.is_none() && f.dict_id.is_none());
    }

    #[test]
    fn block_walk() {
        let d = fixture();
        let f = parse(&d).unwrap();
        let bs: Vec<_> = blocks(&f, &d).collect();
        assert_eq!(bs.len(), 2);
        assert_eq!(bs[0].size, 5);
        assert!(!bs[0].uncompressed);
        assert_eq!(bs[1].size, 3);
        assert!(bs[1].uncompressed);
    }

    #[test]
    fn with_content_size_and_dict() {
        let mut d = MAGIC.to_le_bytes().to_vec();
        d.push(0x49); // version 1 + content-size + dict-id flags
        d.push(0x40); // 64 KiB max
        d.extend_from_slice(&99u64.to_le_bytes());
        d.extend_from_slice(&0xABu32.to_le_bytes());
        d.push(0);
        let f = parse(&d).unwrap();
        assert_eq!(f.content_size, Some(99));
        assert_eq!(f.dict_id, Some(0xAB));
        assert_eq!(f.block_max, 64 * 1024);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut d = fixture();
        d[0] = 0;
        assert!(parse(&d).is_none());
        let mut d2 = fixture();
        d2[4] = 0x80; // version 2
        assert!(parse(&d2).is_none());
        let mut d3 = fixture();
        d3[5] = 0x10; // block max code 1 — invalid
        assert!(parse(&d3).is_none());
    }
}
