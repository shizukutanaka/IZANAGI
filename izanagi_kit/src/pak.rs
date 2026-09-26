//! Quake `PACK`/`PAK` asset package.
//!
//! The header is `PACK` + directory offset `u32` + directory length
//! `u32`. The directory is a table of 64-byte entries: a 56-byte
//! NUL-padded name, a `u32` file position, and a `u32` length — all
//! little-endian.
//!
//! ```
//! use izanagi_kit::pak::{parse, ENTRY_SIZE};
//!
//! let mut d = Vec::new();
//! d.extend_from_slice(b"PACK");
//! d.extend_from_slice(&12u32.to_le_bytes());  // dir at 12
//! d.extend_from_slice(&64u32.to_le_bytes());  // one entry
//! let mut e = vec![0u8; ENTRY_SIZE];
//! e[..9].copy_from_slice(b"progs.dat");
//! e[56..60].copy_from_slice(&1024u32.to_le_bytes());
//! e[60..64].copy_from_slice(&256u32.to_le_bytes());
//! d.extend_from_slice(&e);
//! let p = parse(&d).unwrap();
//! let en = p.entry(&d, 0).unwrap();
//! assert_eq!(en.name(), "progs.dat");
//! assert_eq!(en.filepos, 1024);
//! ```

use std::string::String;

/// Directory entry size in bytes.
pub const ENTRY_SIZE: usize = 64;
/// Name field width inside an entry.
pub const NAME_LEN: usize = 56;

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

/// One directory entry.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// Raw 56-byte name field.
    pub name_raw: [u8; NAME_LEN],
    /// File offset of the entry's data.
    pub filepos: u32,
    /// Data length in bytes.
    pub filelen: u32,
    /// Absolute offset of this entry inside the directory.
    pub at: usize,
}

impl Entry {
    /// Entry name as a string, trimmed at the first NUL.
    pub fn name(&self) -> String {
        let n = self
            .name_raw
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(NAME_LEN);
        self.name_raw[..n]
            .iter()
            .map(|&b| if b.is_ascii() { char::from(b) } else { '?' })
            .collect()
    }

    /// The entry's bytes inside package `d`.
    pub fn data<'a>(&self, d: &'a [u8]) -> Option<&'a [u8]> {
        let at = usize::try_from(self.filepos).ok()?;
        d.get(at..at + usize::try_from(self.filelen).ok()?)
    }
}

/// A parsed PACK header.
#[derive(Clone, Debug, PartialEq)]
pub struct Pak {
    /// Byte offset of the directory.
    pub dir_at: u32,
    /// Directory length in bytes.
    pub dir_len: u32,
    /// Number of directory entries (`dir_len / 64`).
    pub n_entries: usize,
}

impl Pak {
    /// Read directory entry `i`, or `None` when out of range/bounds.
    pub fn entry(&self, d: &[u8], i: usize) -> Option<Entry> {
        if i >= self.n_entries {
            return None;
        }
        let at = usize::try_from(self.dir_at).ok()? + i * ENTRY_SIZE;
        let mut name_raw = [0u8; NAME_LEN];
        name_raw.copy_from_slice(d.get(at..at + NAME_LEN)?);
        Some(Entry {
            name_raw,
            filepos: u32le(d, at + NAME_LEN)?,
            filelen: u32le(d, at + NAME_LEN + 4)?,
            at,
        })
    }

    /// Iterator over all directory entries.
    pub fn entries<'a>(&'a self, d: &'a [u8]) -> Entries<'a> {
        Entries { pak: self, d, i: 0 }
    }

    /// Find the first entry whose name equals `name`
    /// (case-insensitive, per Quake tool convention).
    pub fn find(&self, d: &[u8], name: &str) -> Option<Entry> {
        self.entries(d)
            .find(|e| e.name().eq_ignore_ascii_case(name))
    }
}

/// Iterator over a PACK's directory.
pub struct Entries<'a> {
    pak: &'a Pak,
    d: &'a [u8],
    i: usize,
}

impl<'a> Iterator for Entries<'a> {
    type Item = Entry;

    fn next(&mut self) -> Option<Entry> {
        let e = self.pak.entry(self.d, self.i)?;
        self.i += 1;
        Some(e)
    }
}

/// Parse a PACK header. Returns `None` on a bad magic, a directory
/// length that is not a multiple of 64, or a directory that runs past
/// the buffer.
pub fn parse(d: &[u8]) -> Option<Pak> {
    if d.get(..4)? != b"PACK" {
        return None;
    }
    let dir_at = u32le(d, 4)?;
    let dir_len = u32le(d, 8)?;
    if dir_len as usize % ENTRY_SIZE != 0 {
        return None;
    }
    let at = usize::try_from(dir_at).ok()?;
    let len = usize::try_from(dir_len).ok()?;
    d.get(at..at + len)?;
    Some(Pak {
        dir_at,
        dir_len,
        n_entries: len / ENTRY_SIZE,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn image() -> Vec<u8> {
        let mut d = b"PACK".to_vec();
        d.extend_from_slice(&76u32.to_le_bytes()); // dir at 76
        d.extend_from_slice(&128u32.to_le_bytes()); // two entries
        d.extend_from_slice(b"hello\0world"); // 12 bytes of file data
        d.resize(76, 0);
        let mut e0 = vec![0u8; ENTRY_SIZE];
        e0[..8].copy_from_slice(b"a/b.txt\0");
        e0[56..60].copy_from_slice(&12u32.to_le_bytes());
        e0[60..64].copy_from_slice(&5u32.to_le_bytes());
        let mut e1 = vec![0u8; ENTRY_SIZE];
        e1[..10].copy_from_slice(b"MAPS/B.BSP");
        e1[56..60].copy_from_slice(&17u32.to_le_bytes());
        e1[60..64].copy_from_slice(&7u32.to_le_bytes());
        d.extend_from_slice(&e0);
        d.extend_from_slice(&e1);
        d
    }

    #[test]
    fn parse_reads_header() {
        let p = parse(&image()).unwrap();
        assert_eq!(p.dir_at, 76);
        assert_eq!(p.dir_len, 128);
        assert_eq!(p.n_entries, 2);
    }

    #[test]
    fn entries_and_data() {
        let d = image();
        let p = parse(&d).unwrap();
        let e0 = p.entry(&d, 0).unwrap();
        assert_eq!(e0.name(), "a/b.txt");
        assert_eq!(e0.data(&d), Some(b"hello".as_ref()));
        assert_eq!(p.entry(&d, 2), None);
        let names: Vec<_> = p.entries(&d).map(|e| e.name()).collect();
        assert_eq!(names, ["a/b.txt", "MAPS/B.BSP"]);
        let e1 = p.entry(&d, 1).unwrap();
        assert_eq!(&e1.data(&d).unwrap()[..7], &d[17..24]);
        assert_eq!(e1.data(&d).unwrap()[6], 0);
    }

    #[test]
    fn find_is_case_insensitive() {
        let d = image();
        let p = parse(&d).unwrap();
        assert_eq!(p.find(&d, "A/B.TXT").unwrap().filepos, 12);
        assert_eq!(p.find(&d, "maps/b.bsp").unwrap().filepos, 17);
        assert!(p.find(&d, "missing").is_none());
    }

    #[test]
    fn name_full_56_bytes() {
        let mut e = Entry {
            name_raw: [b'x'; NAME_LEN],
            filepos: 0,
            filelen: 0,
            at: 0,
        };
        assert_eq!(e.name().len(), 56);
        e.name_raw[0] = 0x80;
        assert!(e.name().starts_with('?'));
    }

    #[test]
    fn rejects() {
        assert_eq!(parse(&[]), None);
        assert_eq!(parse(&[0u8; 64]), None);
        let mut d = image();
        d[8..12].copy_from_slice(&65u32.to_le_bytes()); // not 64-aligned
        assert_eq!(parse(&d), None);
        let mut d2 = image();
        d2[4..8].copy_from_slice(&9999u32.to_le_bytes()); // dir out of bounds
        assert_eq!(parse(&d2), None);
    }

    #[test]
    fn constants() {
        assert_eq!(ENTRY_SIZE, 64);
        assert_eq!(NAME_LEN, 56);
    }
}
