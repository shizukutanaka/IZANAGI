//! gzip — RFC 1952's member wrapper. Byte 0 carries `1F 8B`, a
//! compression method (8 = deflate), an FLG byte (`FTEXT`=1,
//! `FHCRC`=2, `FEXTRA`=4, `FNAME`=8, `FCOMMENT`=16), then MTIME u32,
//! XFL and OS. The flag-selected fields follow in order — FEXTRA is a
//! u16 length, FNAME/FCOMMENT are NUL-terminated, FHCRC a u16 — and
//! the member ends with `{crc32, isize}` u32s.
//!
//! ```
//! use izanagi_kit::gzip::{parse, data_at, trailer_at};
//! let mut d = vec![0x1F, 0x8B, 8, 0x08]; // FNAME
//! d.extend_from_slice(&[0; 6]);            // MTIME/XFL/OS
//! d.extend_from_slice(b"name\0");
//! d.extend_from_slice(b"DD");              // deflate body
//! d.extend_from_slice(&0x1234u32.to_le_bytes()); // crc32
//! d.extend_from_slice(&2u32.to_le_bytes());      // isize
//! let g = parse(&d).unwrap();
//! assert_eq!(g.name, b"name");
//! assert_eq!(&d[data_at(&g, &d).unwrap()..trailer_at(&g, &d).unwrap()], b"DD");
//! ```

/// FLG bits.
pub const FTEXT: u8 = 1;
/// FLG bits.
pub const FHCRC: u8 = 2;
/// FLG bits.
pub const FEXTRA: u8 = 4;
/// FLG bits.
pub const FNAME: u8 = 8;
/// FLG bits.
pub const FCOMMENT: u8 = 16;

/// A parsed member head.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gzip {
    /// Compression method (8 = deflate).
    pub method: u8,
    /// Raw FLG byte.
    pub flags: u8,
    /// Modification time (0 = none recorded).
    pub mtime: u32,
    /// Extra flags byte.
    pub xfl: u8,
    /// OS byte (3 = unix, 255 = unknown).
    pub os: u8,
    /// `FNAME` payload when present.
    pub name: Vec<u8>,
    /// `FCOMMENT` payload when present.
    pub comment: Vec<u8>,
    /// `FEXTRA` length when present.
    pub extra_len: Option<u16>,
}

fn u16l(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?))
}

fn u32l(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?))
}

fn nul(d: &[u8], at: usize) -> Option<(Vec<u8>, usize)> {
    let rest = d.get(at..)?;
    let end = rest.iter().position(|&c| c == 0)?;
    Some((rest[..end].to_vec(), at + end + 1))
}

/// Parse the member head; `None` without `1F 8B`.
pub fn parse(d: &[u8]) -> Option<Gzip> {
    if d.get(..2)? != b"\x1F\x8B" {
        return None;
    }
    let method = *d.get(2)?;
    let flags = *d.get(3)?;
    let mtime = u32l(d, 4)?;
    let xfl = *d.get(8)?;
    let os = *d.get(9)?;
    let mut at = 10usize;
    let mut extra_len = None;
    let mut name = Vec::new();
    let mut comment = Vec::new();
    if flags & FEXTRA != 0 {
        let n = u16l(d, at)?;
        at = at.checked_add(2 + usize::from(n))?;
        extra_len = Some(n);
        d.get(..at)?; // must fit
    }
    if flags & FNAME != 0 {
        let (v, next) = nul(d, at)?;
        name = v;
        at = next;
    }
    if flags & FCOMMENT != 0 {
        let (v, next) = nul(d, at)?;
        comment = v;
        at = next;
    }
    if flags & FHCRC != 0 {
        at = at.checked_add(2)?;
    }
    d.get(..at)?; // whole header must be present
    Some(Gzip {
        method,
        flags,
        mtime,
        xfl,
        os,
        name,
        comment,
        extra_len,
    })
}

fn head_len(g: &Gzip, d: &[u8]) -> Option<usize> {
    // re-walk: data begins right after the last header field
    let mut at = 10usize;
    if g.flags & FEXTRA != 0 {
        at = at.checked_add(2 + usize::from(u16l(d, at)?))?;
    }
    if g.flags & FNAME != 0 {
        at = nul(d, at)?.1;
    }
    if g.flags & FCOMMENT != 0 {
        at = nul(d, at)?.1;
    }
    if g.flags & FHCRC != 0 {
        at = at.checked_add(2)?;
    }
    Some(at)
}

/// Byte offset where the deflate body begins.
pub fn data_at(g: &Gzip, d: &[u8]) -> Option<usize> {
    head_len(g, d)
}

/// Byte offset of the `{crc32, isize}` trailer — the last 8 bytes of
/// the member; `None` when the buffer is shorter than the header + 8.
pub fn trailer_at(g: &Gzip, d: &[u8]) -> Option<usize> {
    let head = head_len(g, d)?;
    d.len().checked_sub(8).filter(|&t| t >= head)
}

/// The recorded uncompressed size (`isize`) from the trailer.
pub fn isize(g: &Gzip, d: &[u8]) -> Option<u32> {
    u32l(d, trailer_at(g, d)? + 4)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0x1F, 0x8B, 8, FNAME | FCOMMENT | FHCRC];
        d.extend_from_slice(&123u32.to_le_bytes());
        d.extend_from_slice(&[2, 3]);
        d.extend_from_slice(b"file.txt\0");
        d.extend_from_slice(b"a comment\0");
        d.extend_from_slice(&0x00u16.to_le_bytes());
        d.extend_from_slice(b"XYZ");
        d.extend_from_slice(&0xBEEFu32.to_le_bytes());
        d.extend_from_slice(&3u32.to_le_bytes());
        d
    }

    #[test]
    fn parses_header() {
        let d = fixture();
        let g = parse(&d).unwrap();
        assert_eq!(g.method, 8);
        assert_eq!(g.mtime, 123);
        assert_eq!(g.os, 3);
        assert_eq!(g.name, b"file.txt");
        assert_eq!(g.comment, b"a comment");
        assert!(g.extra_len.is_none());
        let at = data_at(&g, &d).unwrap();
        assert_eq!(&d[at..trailer_at(&g, &d).unwrap()], b"XYZ");
        assert_eq!(isize(&g, &d), Some(3));
    }

    #[test]
    fn extra_field() {
        let mut d = vec![0x1F, 0x8B, 8, FEXTRA];
        d.extend_from_slice(&[0; 6]);
        d.extend_from_slice(&3u16.to_le_bytes());
        d.extend_from_slice(b"abc");
        d.extend_from_slice(&[0; 8]);
        let g = parse(&d).unwrap();
        assert_eq!(g.extra_len, Some(3));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"\x1F\x90").is_none());
        // unterminated FNAME
        let mut d = vec![0x1F, 0x8B, 8, FNAME];
        d.extend_from_slice(&[0; 6]);
        d.extend_from_slice(b"no nul here");
        assert!(parse(&d).is_none());
        // header + nothing: trailer can't exist
        let mut d2 = vec![0x1F, 0x8B, 8, 0];
        d2.extend_from_slice(&[0; 6]);
        assert!(parse(&d2).is_some());
        assert!(trailer_at(&parse(&d2).unwrap(), &d2).is_none());
    }
}
