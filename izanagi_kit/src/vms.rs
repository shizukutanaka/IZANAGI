//! Dreamcast VMU save file (`.vms`) directory-entry parsing.
//!
//! The file opens with the VMU directory entry: `file_type u8`
//! (`0x33` data / `0xCC` game), `copy_protected u8`,
//! `first_block u16LE`, `filename[12]` in `12345678.SAV` form,
//! an 8-byte BCD timestamp, `file_size u16LE` in 512-byte blocks
//! and `header_offset u16LE`.
//!
//! ```
//! use izanagi_kit::vms;
//! let mut d = vec![0u8; 32 + 512];
//! d[0] = 0x33; // data file
//! d[4..12].copy_from_slice(b"SAVE0001");
//! d[12..16].copy_from_slice(b".SAV");
//! d[24..26].copy_from_slice(&1u16.to_le_bytes()); // 1 block
//! let v = vms::parse(&d).unwrap();
//! assert_eq!(v.filename, "SAVE0001.SAV");
//! ```

use std::string::String;

/// Directory-entry size.
pub const DENTRY_LEN: usize = 32;
/// VMU block size.
pub const BLOCK_LEN: usize = 512;

/// `file_type`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileType {
    /// 0x33 — plain data file.
    Data,
    /// 0xCC — executable game file.
    Game,
    /// Unknown value.
    Other(u8),
}

/// A parsed `.vms`.
#[derive(Clone, Debug, PartialEq)]
pub struct Vms {
    /// `file_type`.
    pub file_type: FileType,
    /// `copy_protected` flag byte.
    pub copy_protected: u8,
    /// `first_block`.
    pub first_block: u16,
    /// `filename` (12 chars, space-trimmed).
    pub filename: String,
    /// `file_size` in 512-byte blocks.
    pub file_size_blocks: u16,
    /// `header_offset` in blocks.
    pub header_offset: u16,
    /// Data-region byte length implied by `file_size`.
    pub data_len: usize,
}

fn u16le(d: &[u8], at: usize) -> Option<u16> {
    let s = d.get(at..at.checked_add(2)?)?;
    Some((s[0] as u16) | (s[1] as u16) << 8)
}

/// Parses a `.vms` header: known file type, printable `NAME.SAV`
/// filename containing a `.`, and `file_size` blocks fitting inside
/// the input after the 32-byte entry.
pub fn parse(d: &[u8]) -> Option<Vms> {
    if d.len() < DENTRY_LEN + BLOCK_LEN {
        return None;
    }
    let ft = match d[0] {
        0x33 => FileType::Data,
        0xCC => FileType::Game,
        o => FileType::Other(o),
    };
    if matches!(ft, FileType::Other(_)) {
        return None;
    }
    let name_bytes = d.get(4..16)?;
    if !name_bytes.iter().all(|&b| (0x20..=0x7E).contains(&b)) {
        return None;
    }
    let filename: String = name_bytes
        .iter()
        .map(|&b| b as char)
        .collect::<String>()
        .trim_end()
        .to_string();
    if !filename.contains('.') {
        return None;
    }
    let file_size_blocks = u16le(d, 24)?;
    if file_size_blocks == 0 {
        return None;
    }
    let data_len = (file_size_blocks as usize).checked_mul(BLOCK_LEN)?;
    if DENTRY_LEN.checked_add(data_len)? > d.len() {
        return None;
    }
    Some(Vms {
        file_type: ft,
        copy_protected: d[1],
        first_block: u16le(d, 2)?,
        filename,
        file_size_blocks,
        header_offset: u16le(d, 26)?,
        data_len,
    })
}

/// True when the entry declares a game (`0xCC`).
pub fn is_game(d: &[u8]) -> bool {
    d.first() == Some(&0xCC)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    fn fixture(ty: u8, blocks: u16) -> Vec<u8> {
        let mut d = vec![0u8; 32 + 512 * blocks as usize];
        d[0] = ty;
        d[4..12].copy_from_slice(b"TESTSAVE");
        d[12..16].copy_from_slice(b".SAV");
        d[24..26].copy_from_slice(&blocks.to_le_bytes());
        d
    }

    #[test]
    fn parses_entry() {
        let v = parse(&fixture(0x33, 2)).unwrap();
        assert_eq!(v.file_type, FileType::Data);
        assert_eq!(v.filename, "TESTSAVE.SAV");
        assert_eq!(v.data_len, 1024);
        assert!(!is_game(&fixture(0x33, 1)));
        assert!(is_game(&fixture(0xCC, 1)));
    }

    #[test]
    fn rejects() {
        assert!(parse(&[0u8; 64]).is_none());
        assert!(parse(&fixture(0x00, 1)).is_none()); // bad type
        let mut d = fixture(0x33, 1);
        d[12] = b' '; // remove the dot
        d[13] = b' ';
        d[14] = b' ';
        d[15] = b' ';
        assert!(parse(&d).is_none());
    }
}
