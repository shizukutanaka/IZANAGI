//! dictzip `.dict.dz` — a gzip member with the `RA` random-access
//! extra-field subfield: `chunk_size u16le` + `chunk_count u16le`
//! (plus `u32le` offsets per chunk). Enables block seek into the
//! compressed dictionary body.
//!
//! ```
//! use izanagi_kit::dictzip::{detect, parse};
//!
//! // gzip hdr + FEXTRA with an "RA" subfield: len=4, size=8192, chunks=2
//! let d = b"\x1F\x8B\x08\x04\x00\x00\x00\x00\x00\xFF\
//! \x0A\x00RA\x04\x00\x00\x20\x02\x00\x00\x00\x00\x00";
//! assert!(detect(d));
//! let z = parse(d).unwrap();
//! assert_eq!(z.chunk_size, 8192);
//! assert_eq!(z.chunks, 2);
//! ```

/// Parsed dictzip `.dz` extra-field census.
#[derive(Debug, Clone, PartialEq)]
pub struct Dictzip {
    /// Gzip `FLG` byte.
    pub flags: u8,
    /// `XLEN` — total extra-field byte size.
    pub xlen: u16,
    /// `RA` chunk byte size (`u16le`).
    pub chunk_size: u16,
    /// `RA` chunk count (`u16le`).
    pub chunks: u16,
    /// Extra-field subfield count walked (including `RA`).
    pub subfields: u32,
    /// Other subfield IDs seen (e.g. `BC` for bgzf).
    pub other_ids: Vec<[u8; 2]>,
}

fn le16(b: &[u8], off: usize) -> Option<u16> {
    Some(u16::from(*b.get(off)?) | u16::from(*b.get(off + 1)?) << 8)
}

/// `true` on gzip magic + FEXTRA + an `RA` subfield.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    if b.len() < 12 || !b.starts_with(b"\x1F\x8B\x08") || b[3] & 4 == 0 {
        return false;
    }
    let xlen = usize::from(le16(b, 10).unwrap_or(0));
    if 12 + xlen > b.len() {
        return false;
    }
    let mut off = 12usize;
    let end = 12 + xlen;
    while off + 4 <= end {
        let id = &b[off..off + 2];
        let len = usize::from(le16(b, off + 2).unwrap_or(0));
        if id == b"RA" && len >= 4 && off + 4 + len <= end {
            return true;
        }
        if off + 4 + len > end {
            return false;
        }
        off += 4 + len;
    }
    false
}

/// Census; `None` without the `RA` subfield.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Dictzip> {
    if !detect(b) {
        return None;
    }
    let xlen = usize::from(le16(b, 10)?);
    let end = 12 + xlen;
    let mut off = 12usize;
    let mut z = Dictzip {
        flags: b[3],
        xlen: le16(b, 10)?,
        chunk_size: 0,
        chunks: 0,
        subfields: 0,
        other_ids: Vec::new(),
    };
    while off + 4 <= end {
        let len = usize::from(le16(b, off + 2)?);
        let id = [b[off], b[off + 1]];
        if off + 4 + len > end {
            return None;
        }
        z.subfields += 1;
        if &id == b"RA" && len >= 4 {
            z.chunk_size = le16(b, off + 4)?;
            z.chunks = le16(b, off + 6)?;
        } else if &id != b"RA" {
            z.other_ids.push(id);
        }
        off += 4 + len;
    }
    Some(z)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"\x1F\x8B\x08\x04\x00\x00\x00\x00\x00\xFF\
\x0A\x00RA\x04\x00\x00\x20\x02\x00\x00\x00\x00\x00";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        // gzip without FEXTRA
        assert!(!detect(b"\x1F\x8B\x08\x00\x00\x00\x00\x00\x00\xFFbody"));
        // FEXTRA without RA
        assert!(!detect(
            b"\x1F\x8B\x08\x04\x00\x00\x00\x00\x00\xFF\x04\x00BC\x00\x00"
        ));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn parses() {
        let z = parse(D).unwrap();
        assert_eq!(z.flags, 4);
        assert_eq!(z.xlen, 10);
        assert_eq!(z.chunk_size, 8192);
        assert_eq!(z.chunks, 2);
        assert_eq!(z.subfields, 1);
        assert!(z.other_ids.is_empty());
    }

    #[test]
    fn other_subfield_kept() {
        let d = b"\x1F\x8B\x08\x04\x00\x00\x00\x00\x00\xFF\
\x10\x00BC\x04\x00\x00\x00\x00\x00RA\x04\x00\x00\x20\x01\x00";
        let z = parse(d).unwrap();
        assert_eq!(z.other_ids, [*b"BC"]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}
