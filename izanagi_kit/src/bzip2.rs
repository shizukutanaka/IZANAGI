//! bzip2 — the `BZh` stream. Bytes 0..4 carry `BZh` plus a digit
//! `1`..=`9` (block size ×100 KB). Each compressed block opens with
//! the six-byte magic `31 41 59 26 53 59` (π), a u32 block CRC, a
//! randomised flag and a u24 `origPtr`; the stream ends with the
//! magic `17 72 45 38 50 90` (√π) and a combined CRC.
//!
//! ```
//! use izanagi_kit::bzip2::{parse, block_at};
//! let mut d = b"BZh9".to_vec();
//! d.extend_from_slice(&[0x31, 0x41, 0x59, 0x26, 0x53, 0x59]);
//! d.extend_from_slice(&[0, 0, 0xCA, 0xFE]); // block crc, big-endian
//! let b = parse(&d).unwrap();
//! assert_eq!(b.level, 9);
//! assert_eq!(block_at(&d).unwrap(), 4);
//! ```

/// The six-byte compressed-block magic (π).
pub const BLOCK_MAGIC: [u8; 6] = [0x31, 0x41, 0x59, 0x26, 0x53, 0x59];
/// The six-byte end-of-stream magic (√π).
pub const EOS_MAGIC: [u8; 6] = [0x17, 0x72, 0x45, 0x38, 0x50, 0x90];

/// A parsed stream head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bz2 {
    /// Block size digit 1–9.
    pub level: u8,
}

/// A compressed-block header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    /// Byte offset of the block magic.
    pub at: usize,
    /// Block CRC (big-endian u32 in the stream).
    pub crc: u32,
    /// Randomised-block flag (obsolete, always 0 in practice).
    pub randomised: u8,
    /// Original pointer into the BWT output.
    pub orig_ptr: u32,
}

fn u24b(d: &[u8], at: usize) -> Option<u32> {
    let r = d.get(at..at + 3)?;
    Some((u32::from(r[0]) << 16) | (u32::from(r[1]) << 8) | u32::from(r[2]))
}

fn u32b(d: &[u8], at: usize) -> Option<u32> {
    let r = d.get(at..at + 4)?;
    Some(
        (u32::from(r[0]) << 24)
            | (u32::from(r[1]) << 16)
            | (u32::from(r[2]) << 8)
            | u32::from(r[3]),
    )
}

/// Parse the head; `None` without `BZh` + digit.
pub fn parse(d: &[u8]) -> Option<Bz2> {
    if d.get(..3)? != b"BZh" {
        return None;
    }
    let level = *d.get(3)?;
    if !(b'1'..=b'9').contains(&level) {
        return None;
    }
    Some(Bz2 {
        level: level - b'0',
    })
}

/// First `BLOCK_MAGIC` offset, if any.
pub fn block_at(d: &[u8]) -> Option<usize> {
    d.windows(BLOCK_MAGIC.len()).position(|w| w == BLOCK_MAGIC)
}

/// First `EOS_MAGIC` offset, if any.
pub fn end_at(d: &[u8]) -> Option<usize> {
    d.windows(EOS_MAGIC.len()).position(|w| w == EOS_MAGIC)
}

/// Parse the block header at `at` (the offset `block_at` found).
pub fn block(d: &[u8], at: usize) -> Option<Block> {
    if d.get(at..at + 6)? != BLOCK_MAGIC {
        return None;
    }
    Some(Block {
        at,
        crc: u32b(d, at + 6)?,
        randomised: *d.get(at + 10)?,
        orig_ptr: u24b(d, at + 11)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = b"BZh9".to_vec();
        d.extend_from_slice(&BLOCK_MAGIC);
        d.extend_from_slice(&[0xCA, 0xFE, 0xBA, 0xBE]); // crc, big-endian
        d.push(0); // randomised
        d.extend_from_slice(&[0x00, 0x01, 0x02]); // origPtr u24
        d.extend_from_slice(&[0; 8]);
        d.extend_from_slice(&EOS_MAGIC);
        d.extend_from_slice(&[0; 4]); // combined crc
        d
    }

    #[test]
    fn parses_head_and_block() {
        let d = fixture();
        assert_eq!(parse(&d).unwrap().level, 9);
        let at = block_at(&d).unwrap();
        assert_eq!(at, 4);
        let b = block(&d, at).unwrap();
        assert_eq!(b.crc, 0xCAFEBABE);
        assert_eq!(b.randomised, 0);
        assert_eq!(b.orig_ptr, 0x102);
        assert_eq!(end_at(&d), Some(d.len() - 10));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"BZh0").is_none());
        assert!(parse(b"BZhA").is_none());
        let mut d = fixture();
        d[4] = 0; // smash block magic
        assert!(block(&d, 4).is_none());
        assert!(block_at(&d).is_none());
        assert!(end_at(&d).is_some());
    }
}
