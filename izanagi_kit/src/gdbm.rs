//! GDBM database files — magic `0x13579ACE` (or the extended
//! `0x13579ACF`), then a file header with block size, the directory
//! offset/size, bucket geometry, and the next free block. Both byte
//! orders are detected from the magic.
//!
//! ```
//! use izanagi_kit::gdbm::parse;
//!
//! let mut f = vec![0u8; 640];
//! f[0..4].copy_from_slice(&0x13579ACEu32.to_le_bytes());
//! f[4..8].copy_from_slice(&4096u32.to_le_bytes());   // block_size
//! f[8..16].copy_from_slice(&96u64.to_le_bytes());    // dir offset
//! f[16..20].copy_from_slice(&512u32.to_le_bytes());  // dir size
//! let g = parse(&f).unwrap();
//! assert_eq!(g.block_size, 4096);
//! ```

use std::vec::Vec;

/// Classic magic.
pub const MAGIC: u32 = 0x1357_9ACE;
/// Extended magic (GDBM ≥1.9 layout variant).
pub const MAGIC_EXT: u32 = 0x1357_9ACF;

/// A parsed header.
#[derive(Clone, Debug)]
pub struct Gdbm {
    /// True when the extended magic was present.
    pub extended: bool,
    /// Header bytes were little-endian (`false` = big).
    pub le: bool,
    /// Data block size.
    pub block_size: u32,
    /// Byte offset of the hash directory.
    pub dir_offset: u64,
    /// Directory size in bytes.
    pub dir_size: u32,
    /// `log2` of dir entries.
    pub dir_bits: u32,
    /// Hash bucket size in bytes.
    pub bucket_size: u32,
    /// Entries per bucket.
    pub bucket_elems: u32,
    /// Next unallocated block offset.
    pub next_block: u64,
}

fn num(d: &[u8], o: usize, n: usize, le: bool) -> Option<u64> {
    let s = d.get(o..o + n)?;
    let mut v = 0u64;
    if le {
        for b in s.iter().rev() {
            v = (v << 8) | *b as u64;
        }
    } else {
        for b in s {
            v = (v << 8) | *b as u64;
        }
    }
    Some(v)
}

/// Parse a GDBM file header.
pub fn parse(d: &[u8]) -> Option<Gdbm> {
    if d.len() < 40 {
        return None;
    }
    let first = [*d.first()?, *d.get(1)?, *d.get(2)?, *d.get(3)?];
    // Detect endianness by folding the magic both ways manually.
    let le = u32::from_le_bytes(first);
    let be = ((first[0] as u32) << 24)
        | ((first[1] as u32) << 16)
        | ((first[2] as u32) << 8)
        | first[3] as u32;
    let (extended, little) = if le == MAGIC || be == MAGIC {
        (false, le == MAGIC)
    } else if le == MAGIC_EXT || be == MAGIC_EXT {
        (true, le == MAGIC_EXT)
    } else {
        return None;
    };
    let n = |o, w| num(d, o, w, little);
    let block_size = n(4, 4)? as u32;
    let dir_offset = n(8, 8)?;
    let dir_size = n(16, 4)? as u32;
    let dir_bits = n(20, 4)? as u32;
    let bucket_size = n(24, 4)? as u32;
    let bucket_elems = n(28, 4)? as u32;
    let next_block = n(32, 8)?;
    if block_size < 512 || !block_size.is_power_of_two() {
        return None;
    }
    if dir_offset
        .checked_add(dir_size as u64)
        .map_or(true, |end| end > d.len() as u64)
    {
        return None;
    }
    let _ = Vec::<u8>::new();
    Some(Gdbm {
        extended,
        le: little,
        block_size,
        dir_offset,
        dir_size,
        dir_bits,
        bucket_size,
        bucket_elems,
        next_block,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_le() {
        let mut f = vec![0u8; 64];
        f[0..4].copy_from_slice(&MAGIC.to_le_bytes());
        f[4..8].copy_from_slice(&4096u32.to_le_bytes());
        f[8..16].copy_from_slice(&32u64.to_le_bytes());
        f[16..20].copy_from_slice(&32u32.to_le_bytes());
        let g = parse(&f).unwrap();
        assert!(g.le && !g.extended);
        assert_eq!(g.dir_offset, 32);
    }

    #[test]
    fn parses_be_and_ext() {
        let mut f = vec![0u8; 128];
        f[0..4].copy_from_slice(&MAGIC_EXT.to_be_bytes());
        f[4..8].copy_from_slice(&1024u32.to_be_bytes());
        f[8..16].copy_from_slice(&64u64.to_be_bytes());
        f[16..20].copy_from_slice(&64u32.to_be_bytes());
        let g = parse(&f).unwrap();
        assert!(!g.le && g.extended);
        assert_eq!(g.dir_size, 64);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0u8; 64]).is_none());
        let mut f = vec![0u8; 64];
        f[0..4].copy_from_slice(&MAGIC.to_le_bytes());
        f[4..8].copy_from_slice(&4097u32.to_le_bytes()); // not power of two
        assert!(parse(&f).is_none());
    }
}
