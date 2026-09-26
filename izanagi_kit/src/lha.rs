//! LHA/LZH archive member headers (`-lh0-` … `-lh7-`, `-lz4-`, `-pm0-` …).
//!
//! Header levels 0, 1 and 2 are supported. A level-0/1 header is
//! `size u8 | checksum u8 | method[5] | packed u32 | original u32 |
//! msdos_time u32 | attr u8 | level u8 | name_len u8 | name | crc u16`
//! followed by extension blocks for level 1. A level-2 header is
//! `size u16 | method[5] | packed | original | unix_time | attr |
//! level | crc u16 | os u8 | extension blocks`, where `size` covers
//! the whole header including the size word itself. The archive is a
//! concatenation of `header + packed_data` members.
//!
//! ```
//! use izanagi_kit::lha::{parse, entries, Method};
//!
//! // One level-0 member holding 3 stored bytes of "abc".
//! let name = b"A.TXT";
//! let mut h = vec![0u8; 22 + name.len() + 2];
//! h[0] = (h.len() - 2) as u8;                  // header size
//! h[2..7].copy_from_slice(b"-lh0-");           // method: stored
//! h[7..11].copy_from_slice(&3u32.to_le_bytes());
//! h[11..15].copy_from_slice(&3u32.to_le_bytes());
//! h[20] = 0;                                   // level 0
//! h[21] = name.len() as u8;
//! h[22..22 + name.len()].copy_from_slice(name);
//! h[22 + name.len()] = 0x9A;                   // (fake) CRC
//! h[1] = h[2..].iter().fold(0u8, |a, &b| a.wrapping_add(b));
//! let mut d = h;
//! d.extend_from_slice(b"abc");
//! let e = parse(&d).unwrap();
//! assert_eq!(e.name, "A.TXT");
//! assert_eq!(e.method, Method::Lh0);
//! assert_eq!(e.data(&d), b"abc");
//! assert_eq!(entries(&d).count(), 1);
//! ```

use std::string::String;

/// Member header: the offset of the packed data for a level-0/1
/// member is `2 + header_size`; for level 2 it is `header_size`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// Header level: 0, 1 or 2.
    pub level: u8,
    /// Compression method (e.g. `-lh5-` → [`Method::Lh5`]).
    pub method: Method,
    /// Raw 5-byte method tag including the dashes.
    pub method_tag: [u8; 5],
    /// Member file name (extension block overrides inline name).
    pub name: String,
    /// Packed (compressed) byte count.
    pub packed_size: u32,
    /// Original byte count.
    pub original_size: u32,
    /// Timestamp: MS-DOS packed format for level 0/1, Unix seconds
    /// for level 2.
    pub timestamp: u32,
    /// CRC-16 stored in the header (not verified here).
    pub crc: u16,
    /// Offset of this member's header in the archive.
    pub at: usize,
    /// Offset of this member's packed data in the archive.
    pub data_at: usize,
}

impl Member {
    /// The member's packed data slice.
    pub fn data<'a>(&self, d: &'a [u8]) -> &'a [u8] {
        &d[self.data_at..self.data_at + self.packed_size as usize]
    }
}

/// Compression method tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// `-lh0-`: stored.
    Lh0,
    /// `-lh1-`: LZHUF 4KiB.
    Lh1,
    /// `-lh2-`.
    Lh2,
    /// `-lh3-`.
    Lh3,
    /// `-lh4-`.
    Lh4,
    /// `-lh5-`: LZHUF 8KiB.
    Lh5,
    /// `-lh6-`.
    Lh6,
    /// `-lh7-`.
    Lh7,
    /// `-lz4-`: stored.
    Lz4,
    /// `-lzs-`.
    Lzs,
    /// `-lz5-`.
    Lz5,
    /// `-pm0-`: stored (PMArc).
    Pm0,
    /// `-pm2-` (PMArc).
    Pm2,
    /// `-lhd-`: directory member.
    Lhd,
    /// Any other tag.
    Other,
}

impl Method {
    fn of(tag: &[u8]) -> Method {
        match tag {
            b"-lh0-" => Method::Lh0,
            b"-lh1-" => Method::Lh1,
            b"-lh2-" => Method::Lh2,
            b"-lh3-" => Method::Lh3,
            b"-lh4-" => Method::Lh4,
            b"-lh5-" => Method::Lh5,
            b"-lh6-" => Method::Lh6,
            b"-lh7-" => Method::Lh7,
            b"-lz4-" => Method::Lz4,
            b"-lzs-" => Method::Lzs,
            b"-lz5-" => Method::Lz5,
            b"-pm0-" => Method::Pm0,
            b"-pm2-" => Method::Pm2,
            b"-lhd-" => Method::Lhd,
            _ => Method::Other,
        }
    }

    /// True when the member data is stored uncompressed.
    pub fn is_stored(self) -> bool {
        matches!(self, Method::Lh0 | Method::Lz4 | Method::Pm0)
    }
}

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

fn tag_ok(d: &[u8]) -> bool {
    matches!(d.get(2), Some(b'-')) && matches!(d.get(6), Some(b'-'))
}

/// Parse the first archive member, or `None` when the header is not
/// a valid level-0/1/2 LHA header.
pub fn parse(d: &[u8]) -> Option<Member> {
    member_at(d, 0).map(|(m, _)| m)
}

/// Walk the member chain: each member is followed by `packed_size`
/// bytes of data, then the next member's header.
pub fn entries(d: &[u8]) -> Entries<'_> {
    Entries { d, at: 0 }
}

/// Iterator over archive members; stops at the first invalid or
/// truncated header.
pub struct Entries<'a> {
    d: &'a [u8],
    at: usize,
}

impl<'a> Iterator for Entries<'a> {
    type Item = Member;
    fn next(&mut self) -> Option<Member> {
        let (m, next) = member_at(self.d, self.at)?;
        self.at = next;
        Some(m)
    }
}

/// Parse the member starting at `at`; returns the member plus the
/// offset just past its packed data.
fn member_at(d: &[u8], at: usize) -> Option<(Member, usize)> {
    // Level-0/1 headers carry a verifiable byte-sum checksum; try
    // them first, then fall back to the level-2 layout where the
    // level byte sits at offset 19.
    if let Some(m) = member_level01(d, at) {
        return Some(m);
    }
    member_level2(d, at)
}

fn member_level01(d: &[u8], at: usize) -> Option<(Member, usize)> {
    let h = d.get(at..)?;
    let level = *h.get(20)?;
    if level != 0 && level != 1 {
        return None;
    }
    let hsize = usize::from(h[0]);
    if hsize < 22 {
        return None;
    }
    let hdr = h.get(..hsize + 2)?;
    // Header checksum: bytes [2..2+hsize] folded to u8.
    let sum = hdr[2..].iter().fold(0u8, |a, &b| a.wrapping_add(b));
    if sum != hdr[1] || !tag_ok(hdr) {
        return None;
    }
    let mut tag = [0u8; 5];
    tag.copy_from_slice(&hdr[2..7]);
    let packed = u32le(hdr, 7)?;
    let original = u32le(hdr, 11)?;
    let timestamp = u32le(hdr, 15)?;
    let nlen = usize::from(*hdr.get(21)?);
    let name_at = 22;
    let name_raw = hdr.get(name_at..name_at + nlen)?;
    let mut name = String::from_utf8_lossy(name_raw).into_owned();
    let crc = u16le(hdr, name_at + nlen)?;
    if level == 1 {
        // Extended blocks live inside the header after the CRC:
        // u16 size (covering size+id+payload) | u8 id | payload,
        // terminated by a zero size word. Id 2 is the filename.
        let mut e = name_at + nlen + 2;
        while let Some(sz) = u16le(hdr, e) {
            if sz == 0 {
                break;
            }
            let sz = usize::from(sz);
            if sz < 3 || e + sz > hdr.len() {
                return None;
            }
            if *hdr.get(e + 2)? == 2 {
                if let Some(nb) = hdr.get(e + 3..e + sz) {
                    name = String::from_utf8_lossy(nb).into_owned();
                }
            }
            e += sz;
        }
    }
    let data_at = at + hsize + 2;
    let end = data_at.checked_add(packed as usize)?;
    if end > d.len() {
        return None;
    }
    Some((
        Member {
            level,
            method: Method::of(&tag),
            method_tag: tag,
            name,
            packed_size: packed,
            original_size: original,
            timestamp,
            crc,
            at,
            data_at,
        },
        end,
    ))
}

fn member_level2(d: &[u8], at: usize) -> Option<(Member, usize)> {
    let h = d.get(at..)?;
    let hsize = usize::from(u16le(h, 0)?);
    if hsize < 24 || h.get(..hsize).is_none() {
        return None;
    }
    let hdr = h.get(..hsize)?;
    if !tag_ok(hdr) || *hdr.get(20)? != 2 {
        return None;
    }
    let mut tag = [0u8; 5];
    tag.copy_from_slice(&hdr[2..7]);
    let packed = u32le(hdr, 7)?;
    let original = u32le(hdr, 11)?;
    let timestamp = u32le(hdr, 15)?;
    let crc = u16le(hdr, 21)?;
    let mut name = String::new();
    let mut e = 24usize;
    while let Some(sz) = u16le(hdr, e) {
        if sz == 0 {
            break;
        }
        let sz = usize::from(sz);
        if sz < 3 || e + sz > hsize {
            break;
        }
        if *hdr.get(e + 2)? == 2 {
            if let Some(nb) = hdr.get(e + 3..e + sz) {
                name = String::from_utf8_lossy(nb).into_owned();
            }
        }
        e += sz;
    }
    let data_at = at + hsize;
    let end = data_at.checked_add(packed as usize)?;
    if end > d.len() {
        return None;
    }
    Some((
        Member {
            level: 2,
            method: Method::of(&tag),
            method_tag: tag,
            name,
            packed_size: packed,
            original_size: original,
            timestamp,
            crc,
            at,
            data_at,
        },
        end,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    fn level0(name: &[u8], body: &[u8], method: &[u8; 5]) -> Vec<u8> {
        let mut h = vec![0u8; 22 + name.len() + 2];
        h[0] = (h.len() - 2) as u8;
        h[2..7].copy_from_slice(method);
        h[7..11].copy_from_slice(&(body.len() as u32).to_le_bytes());
        h[11..15].copy_from_slice(&(body.len() as u32).to_le_bytes());
        h[20] = 0;
        h[21] = name.len() as u8;
        h[22..22 + name.len()].copy_from_slice(name);
        h[22 + name.len()] = 0x9A;
        h[22 + name.len() + 1] = 0x27;
        h[1] = h[2..].iter().fold(0u8, |a, &b| a.wrapping_add(b));
        h.extend_from_slice(body);
        h
    }

    fn level2(name: &[u8], body: &[u8]) -> Vec<u8> {
        // header: size(2) method(5) packed(4) orig(4) ts(4) attr(1)
        // level(1) crc(2) os(1) ext-blocks... terminator
        let ext = 2 + 3 + name.len(); // terminator word + name block
        let hsize = 24 + ext;
        let mut h = vec![0u8; hsize];
        h[0..2].copy_from_slice(&(hsize as u16).to_le_bytes());
        h[2..7].copy_from_slice(b"-lh5-");
        h[7..11].copy_from_slice(&(body.len() as u32).to_le_bytes());
        h[11..15].copy_from_slice(&(body.len() as u32).to_le_bytes());
        h[15..19].copy_from_slice(&1_700_000_000u32.to_le_bytes());
        h[19] = 0x20;
        h[20] = 2;
        h[21..23].copy_from_slice(&0xBEEFu16.to_le_bytes());
        h[23] = b'M';
        let nb = (3 + name.len()) as u16;
        h[24..26].copy_from_slice(&nb.to_le_bytes());
        h[26] = 2;
        h[27..27 + name.len()].copy_from_slice(name);
        // terminator already zero
        h.extend_from_slice(body);
        h
    }

    #[test]
    fn level0_fields() {
        let d = level0(b"FOO.DAT", b"xy", b"-lh0-");
        let m = parse(&d).unwrap();
        assert_eq!(m.level, 0);
        assert_eq!(m.method, Method::Lh0);
        assert_eq!(&m.method_tag, b"-lh0-");
        assert_eq!(m.name, "FOO.DAT");
        assert_eq!(m.packed_size, 2);
        assert_eq!(m.original_size, 2);
        assert_eq!(m.crc, 0x279A);
        assert_eq!(m.at, 0);
        assert_eq!(m.data(&d), b"xy");
        assert!(m.method.is_stored());
    }

    #[test]
    fn bad_checksum_rejects() {
        let mut d = level0(b"FOO", b"xy", b"-lh0-");
        d[1] ^= 0xFF;
        assert_eq!(parse(&d), None);
    }

    #[test]
    fn level2_fields() {
        let d = level2(b"DOC.TXT", b"packed");
        let m = parse(&d).unwrap();
        assert_eq!(m.level, 2);
        assert_eq!(m.method, Method::Lh5);
        assert_eq!(m.name, "DOC.TXT");
        assert_eq!(m.timestamp, 1_700_000_000);
        assert_eq!(m.crc, 0xBEEF);
        assert_eq!(m.data(&d), b"packed");
        assert!(!m.method.is_stored());
    }

    #[test]
    fn chain_of_two_members() {
        let mut d = level0(b"ONE", b"1", b"-lh0-");
        d.extend_from_slice(&level2(b"TWO", b"22"));
        let all: Vec<_> = entries(&d).map(|m| (m.level, m.name)).collect();
        assert_eq!(all, [(0, String::from("ONE")), (2, String::from("TWO"))]);
    }

    #[test]
    fn truncation_rejects() {
        let mut d = level0(b"FOO", b"xyz", b"-lh0-");
        d.truncate(d.len() - 1); // short by one data byte
        assert_eq!(parse(&d), None);
        assert_eq!(parse(&[]), None);
        assert_eq!(parse(b"-lh0-"), None);
    }

    #[test]
    fn method_tags() {
        let d = level0(b"A", b"", b"-lzs-");
        assert_eq!(parse(&d).unwrap().method, Method::Lzs);
        let d = level0(b"A", b"", b"-pm0-");
        assert_eq!(parse(&d).unwrap().method, Method::Pm0);
        assert!(parse(&d).unwrap().method.is_stored());
        let d = level0(b"A", b"", b"-lhd-");
        assert_eq!(parse(&d).unwrap().method, Method::Lhd);
        let d = level0(b"A", b"", b"-xxx-");
        assert_eq!(parse(&d).unwrap().method, Method::Other);
    }
}
