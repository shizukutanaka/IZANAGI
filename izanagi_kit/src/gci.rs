//! GameCube save file (`.gci`) parsing.
//!
//! A `.gci` is a 64-byte memory-card directory entry followed by
//! `block_count` × 8 KiB data blocks. The dentry holds
//! `gamecode[4]`, `makercode[2]`, `banner_flags u8`,
//! `filename[32]` ASCII, `last_modified u32BE`, `icon_addr`,
//! `icon_fmt u16`, `icon_speed u16`, `permissions`,
//! `copy_count`, `first_block u16BE`, `block_count u16BE` and
//! `comment_addr u32BE`.
//!
//! ```
//! use izanagi_kit::gci;
//! let mut d = vec![0u8; 0x40 + 8192];
//! d[0..4].copy_from_slice(b"GALE");
//! d[4..6].copy_from_slice(b"01");
//! d[8..12].copy_from_slice(b"SAVE");
//! d[0x36..0x38].copy_from_slice(&5u16.to_be_bytes()); // first block
//! d[0x38..0x3A].copy_from_slice(&1u16.to_be_bytes()); // block count
//! let g = gci::parse(&d).unwrap();
//! assert_eq!(g.gamecode, "GALE");
//! assert_eq!(g.block_count, 1);
//! ```

use std::string::String;

/// Directory-entry size; data starts right after it.
pub const DENTRY_LEN: usize = 0x40;
/// One memory card block.
pub const BLOCK_LEN: usize = 8192;

/// A parsed `.gci` header.
#[derive(Clone, Debug, PartialEq)]
pub struct Gci {
    /// 4-char `gamecode` (e.g. `GALE` for Super Smash Bros Melee).
    pub gamecode: String,
    /// 2-char `makercode`.
    pub makercode: String,
    /// `banner_flags`.
    pub banner_flags: u8,
    /// ASCII `filename` (32 bytes, NUL-trimmed).
    pub filename: String,
    /// `last_modified` seconds since 2000-01-01.
    pub last_modified: u32,
    /// `permissions`.
    pub permissions: u8,
    /// `copy_count`.
    pub copy_count: u8,
    /// `first_block` on the card.
    pub first_block: u16,
    /// `block_count` claimed.
    pub block_count: u16,
    /// `comment_addr`.
    pub comment_addr: u32,
    /// Byte length implied by the header (`0x40 + blocks*8192`).
    pub data_len: usize,
}

fn u16be(d: &[u8], at: usize) -> Option<u16> {
    let s = d.get(at..at.checked_add(2)?)?;
    Some((s[0] as u16) << 8 | s[1] as u16)
}

fn u32be(d: &[u8], at: usize) -> Option<u32> {
    let s = d.get(at..at.checked_add(4)?)?;
    Some((s[0] as u32) << 24 | (s[1] as u32) << 16 | (s[2] as u32) << 8 | s[3] as u32)
}

fn ascii(d: &[u8]) -> String {
    let end = d.iter().position(|&b| b == 0).unwrap_or(d.len());
    String::from_utf8_lossy(&d[..end]).into_owned()
}

/// Parses a `.gci`: printable gamecode/makercode, non-empty filename,
/// `block_count` within the card limit and the file long enough to
/// contain the declared blocks.
pub fn parse(d: &[u8]) -> Option<Gci> {
    if d.len() < DENTRY_LEN + BLOCK_LEN {
        return None;
    }
    let gamecode = ascii(d.get(..4)?);
    let makercode = ascii(d.get(4..6)?);
    if !d.get(..6)?.iter().all(|&b| b.is_ascii_alphanumeric()) {
        return None;
    }
    let filename = ascii(d.get(8..40)?);
    if filename.is_empty() {
        return None;
    }
    let block_count = u16be(d, 0x38)?;
    if block_count == 0 || block_count > 127 {
        return None;
    }
    let data_len = DENTRY_LEN.checked_add((block_count as usize).checked_mul(BLOCK_LEN)?)?;
    if data_len > d.len() {
        return None;
    }
    Some(Gci {
        gamecode,
        makercode,
        banner_flags: d[7],
        filename,
        last_modified: u32be(d, 0x28)?,
        permissions: d[0x34],
        copy_count: d[0x35],
        first_block: u16be(d, 0x36)?,
        block_count,
        comment_addr: u32be(d, 0x3C)?,
        data_len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    fn fixture(blocks: u16) -> Vec<u8> {
        let mut d = vec![0u8; 0x40 + 8192 * blocks as usize];
        d[0..4].copy_from_slice(b"GALE");
        d[4..6].copy_from_slice(b"01");
        d[8..12].copy_from_slice(b"DATA");
        d[0x36..0x38].copy_from_slice(&5u16.to_be_bytes());
        d[0x38..0x3A].copy_from_slice(&blocks.to_be_bytes());
        d
    }

    #[test]
    fn parses_header() {
        let g = parse(&fixture(2)).unwrap();
        assert_eq!(g.makercode, "01");
        assert_eq!(g.filename, "DATA");
        assert_eq!(g.first_block, 5);
        assert_eq!(g.data_len, 0x40 + 2 * 8192);
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 100]).is_none()); // too short
        let mut d = fixture(4);
        d.truncate(0x40 + 8192); // declared 4 blocks, only 1 present
        assert!(parse(&d).is_none());
        let mut d = fixture(1);
        d[0..4].copy_from_slice(&[0, 0, 0, 0]); // bad gamecode
        assert!(parse(&d).is_none());
    }
}
