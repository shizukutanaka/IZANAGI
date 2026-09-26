//! ARJ archive member headers.
//!
//! Each member starts with `60 EA` (u16 LE) plus a `u16` basic-header
//! size covering everything up to the first extended-header word: the
//! fixed 30-byte "first header" (version, host OS, flags, method, file
//! type, MS-DOS timestamp, packed/original sizes, CRC, filespec
//! position, access mode), the NUL-terminated filename and comment.
//! Zero or more extended headers follow, each prefixed by a `u16`
//! size with `0` terminating the chain; the packed data comes after.
//!
//! ```
//! use izanagi_kit::arj::{parse, Method};
//!
//! // minimal member: header-id, basic size 33 (30 fixed + "A\0" + "\0")
//! let mut d = vec![0x60, 0xEA, 33, 0];
//! d.extend_from_slice(&[30, 8, 1, 0, 0, 0, 1, 0]); // fixed part (8B)
//! d.extend_from_slice(&0u32.to_le_bytes());        // dos time
//! d.extend_from_slice(&4u32.to_le_bytes());        // packed
//! d.extend_from_slice(&4u32.to_le_bytes());        // original
//! d.extend_from_slice(&0x1234u32.to_le_bytes());   // crc
//! d.extend_from_slice(&1u16.to_le_bytes());        // filespec pos
//! d.extend_from_slice(&0u16.to_le_bytes());        // access
//! d.extend_from_slice(&[0, 0, b'A', 0, 0]);        // host, chapter, name, comment
//! d.extend_from_slice(&0u16.to_le_bytes());       // no ext headers
//! d.extend_from_slice(b"data");
//! let m = parse(&d, 0).unwrap();
//! assert_eq!(m.name, "A");
//! assert_eq!(m.method, Method::Stored);
//! assert_eq!(m.data(&d), Some(b"data".as_ref()));
//! ```

use std::string::String;

/// The two-byte archive/member signature (`0x60 0xEA` little-endian
/// words read as `0xEA60`).
pub const MAGIC: u16 = 0xEA60;
/// Fixed part of the basic header in bytes.
pub const FIXED: usize = 30;

fn u16le(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from(*d.get(at)?) | u16::from(*d.get(at + 1)?) << 8)
}

fn u32le(d: &[u8], at: usize) -> Option<u32> {
    Some(
        u32::from(*d.get(at)?)
            | u32::from(*d.get(at + 1)?) << 8
            | u32::from(*d.get(at + 2)?) << 16
            | u32::from(*d.get(at + 3)?) << 24,
    )
}

fn cstr(d: &[u8], at: usize, end: usize) -> Option<(String, usize)> {
    let nul = d.get(at..end)?.iter().position(|&b| b == 0)?;
    let s: String = d
        .get(at..at + nul)?
        .iter()
        .map(|&b| if b.is_ascii() { char::from(b) } else { '?' })
        .collect();
    Some((s, at + nul + 1))
}

/// Compression method byte.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Method {
    /// `0` — stored, no compression.
    Stored,
    /// `1` — compressed (most common).
    Compressed,
    /// `2`/`3`/`4` — compressed with `-je` options.
    CompressedFast,
    /// `5` — compressed fastest / `-jh`.
    CompressedFastest,
    /// `8` — archived directory entry.
    Directory,
    /// Any other method byte.
    Other(u8),
}

impl Method {
    fn of(v: u8) -> Self {
        match v {
            0 => Method::Stored,
            1 => Method::Compressed,
            2..=4 => Method::CompressedFast,
            5 => Method::CompressedFastest,
            8 => Method::Directory,
            v => Method::Other(v),
        }
    }

    /// True when the member payload is stored verbatim.
    pub fn is_stored(&self) -> bool {
        *self == Method::Stored
    }
}

/// One archive member (file) header.
#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    /// Member filename.
    pub name: String,
    /// Compressor version byte.
    pub version: u8,
    /// Minimum ARJ version needed to extract.
    pub min_version: u8,
    /// Host OS id (`2` = MS-DOS, `3` = Unix, ...).
    pub host_os: u8,
    /// Flag byte (garbled/password, chapter, backup...).
    pub flags: u8,
    /// Compression method.
    pub method: Method,
    /// File type (`0` file, `1` drive label, `3` directory...).
    pub file_type: u8,
    /// MS-DOS packed timestamp.
    pub dos_time: u32,
    /// Packed (compressed) size.
    pub packed_size: u32,
    /// Original (uncompressed) size.
    pub original_size: u32,
    /// CRC32 of the original data.
    pub crc: u32,
    /// Byte offset of the filename inside the basic header.
    pub filespec_pos: u16,
    /// File access mode.
    pub access: u16,
    /// Absolute offset of this member header in the archive.
    pub at: usize,
    /// Absolute offset of the packed data.
    pub data_at: usize,
}

impl Member {
    /// The member's packed bytes inside archive `d`.
    pub fn data<'a>(&self, d: &'a [u8]) -> Option<&'a [u8]> {
        d.get(self.data_at..self.data_at + self.packed_size as usize)
    }

    /// Offset of the next member header (after `data` bytes), or `None`
    /// when the packed data would run past the buffer.
    pub fn next_at(&self, d: &[u8]) -> Option<usize> {
        let at = self.data_at.checked_add(self.packed_size as usize)?;
        if at > d.len() {
            return None;
        }
        Some(at)
    }
}

/// Parse the member header at `at`. Returns `None` on a bad magic,
/// a basic header shorter than `FIXED`, or an unterminated
/// name/comment/ext-header chain.
pub fn parse(d: &[u8], at: usize) -> Option<Member> {
    if u16le(d, at)? != MAGIC {
        return None;
    }
    let bsize = usize::from(u16le(d, at + 2)?);
    if bsize < FIXED {
        return None;
    }
    let h = at + 4;
    let hend = h + bsize;
    let fixed = usize::from(*d.get(h)?);
    if fixed < FIXED || fixed > bsize {
        return None;
    }
    let (name, p) = cstr(d, h + fixed, hend)?;
    let (_comment, _p) = cstr(d, p, hend)?;
    // extended headers: u16 sizes until 0, data starts after them
    let mut e = hend;
    loop {
        let n = usize::from(u16le(d, e)?);
        if n == 0 {
            break;
        }
        e = e.checked_add(n + 2)?;
    }
    Some(Member {
        name,
        version: *d.get(h + 1)?,
        min_version: *d.get(h + 2)?,
        host_os: *d.get(h + 3)?,
        flags: *d.get(h + 4)?,
        method: Method::of(*d.get(h + 5)?),
        file_type: *d.get(h + 6)?,
        dos_time: u32le(d, h + 8)?,
        packed_size: u32le(d, h + 12)?,
        original_size: u32le(d, h + 16)?,
        crc: u32le(d, h + 20)?,
        filespec_pos: u16le(d, h + 24)?,
        access: u16le(d, h + 26)?,
        at,
        data_at: e + 2,
    })
}

/// Iterator over consecutive member headers in an archive.
pub struct Entries<'a> {
    d: &'a [u8],
    at: usize,
}

impl<'a> Iterator for Entries<'a> {
    type Item = Member;

    fn next(&mut self) -> Option<Member> {
        let m = parse(self.d, self.at)?;
        self.at = m.next_at(self.d)?;
        Some(m)
    }
}

/// Iterate the member headers of a whole archive.
pub fn entries(d: &[u8]) -> Entries<'_> {
    Entries { d, at: 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn member(name: &str, method: u8, data: &[u8]) -> Vec<u8> {
        let mut d = vec![0x60, 0xEA];
        let bsize = (FIXED + name.len() + 2) as u16;
        d.extend_from_slice(&bsize.to_le_bytes());
        d.extend_from_slice(&[FIXED as u8, 9, 1, 2, 0x20, method, 0, 0]);
        d.extend_from_slice(&0x8765_4321u32.to_le_bytes());
        d.extend_from_slice(&(data.len() as u32).to_le_bytes());
        d.extend_from_slice(&99u32.to_le_bytes());
        d.extend_from_slice(&0xBEEFu32.to_le_bytes());
        d.extend_from_slice(&0u16.to_le_bytes());
        d.extend_from_slice(&0o644u16.to_le_bytes());
        d.push(0); // host_data
        d.push(0); // chapter byte (fixed part reaches 30)
        d.extend_from_slice(name.as_bytes());
        d.push(0); // name NUL
        d.push(0); // comment NUL
        d.extend_from_slice(&0u16.to_le_bytes()); // no ext headers
        d.extend_from_slice(data);
        d
    }

    #[test]
    fn parse_reads_every_field() {
        let d = member("file.txt", 1, b"abcd");
        let m = parse(&d, 0).unwrap();
        assert_eq!(m.name, "file.txt");
        assert_eq!(m.version, 9);
        assert_eq!(m.min_version, 1);
        assert_eq!(m.host_os, 2);
        assert_eq!(m.flags, 0x20);
        assert_eq!(m.method, Method::Compressed);
        assert_eq!(m.file_type, 0);
        assert_eq!(m.dos_time, 0x8765_4321);
        assert_eq!(m.packed_size, 4);
        assert_eq!(m.original_size, 99);
        assert_eq!(m.crc, 0xBEEF);
        assert_eq!(m.filespec_pos, 0);
        assert_eq!(m.access, 0o644);
        assert_eq!(m.at, 0);
        assert_eq!(m.data(&d), Some(b"abcd".as_ref()));
        assert_eq!(m.next_at(&d), Some(d.len()));
    }

    #[test]
    fn entries_walks_the_chain() {
        let mut d = member("one", 0, b"aa");
        d.extend(member("two", 0, b"bb"));
        let names: Vec<_> = entries(&d).map(|m| m.name).collect();
        assert_eq!(names, ["one", "two"]);
    }

    #[test]
    fn ext_headers_skipped() {
        let mut d = member("x", 0, b"q");
        // splice an ext header: change 'no ext' u16 to len 6 + payload + 0
        let pos = d.len() - 2 - 1; // before terminating u16 + data
        d[pos] = 8;
        d[pos + 1] = 0;
        let mut d2 = d[..pos + 2].to_vec();
        d2.extend_from_slice(&[0; 8]);
        d2.extend_from_slice(&0u16.to_le_bytes());
        d2.push(b'q');
        let m = parse(&d2, 0).unwrap();
        assert_eq!(m.data(&d2), Some(b"q".as_ref()));
    }

    #[test]
    fn method_ids() {
        assert_eq!(Method::of(0), Method::Stored);
        assert!(Method::of(0).is_stored());
        assert_eq!(Method::of(2), Method::CompressedFast);
        assert_eq!(Method::of(3), Method::CompressedFast);
        assert_eq!(Method::of(4), Method::CompressedFast);
        assert_eq!(Method::of(5), Method::CompressedFastest);
        assert_eq!(Method::of(8), Method::Directory);
        assert_eq!(Method::of(7), Method::Other(7));
        assert!(!Method::of(1).is_stored());
    }

    #[test]
    fn rejects() {
        assert_eq!(parse(&[0u8; 4], 0), None);
        let mut d = member("x", 0, b"q");
        d[0] = 0;
        assert_eq!(parse(&d, 0), None);
        let d2 = member("x", 0, b"q");
        assert_eq!(parse(&d2[..2], 0), None);
        let mut d3 = member("x", 0, b"q");
        d3[2] = 10; // basic header too small
        assert_eq!(parse(&d3, 0), None);
    }

    #[test]
    fn constants() {
        assert_eq!(MAGIC, 0xEA60);
        assert_eq!(FIXED, 30);
    }
}
