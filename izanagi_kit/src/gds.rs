//! GDSII stream — the mask-layout interchange format every physical
//! design tool emits. Records are `{reclen: u16 BE (incl. 4-byte
//! header), tag: u8, dtype: u8, payload}`; the first record is
//! `HEADER` (tag `0x00`) carrying an i16 version.
//!
//! ```
//! use izanagi_kit::gds::{parse, records, tag_name, i16s};
//! let mut d = vec![0, 6, 0, 2]; d.extend_from_slice(&[0, 1]);  // HEADER, i16 v1
//! d.extend_from_slice(&[0, 4, 0x13, 0]);                       // ENDLIB
//! let g = parse(&d).unwrap();
//! assert_eq!(g.version, 1);
//! assert_eq!(records(&d).len(), 2);
//! assert_eq!(tag_name(0x13), "ENDLIB");
//! ```
//!
//! Data types (payload `dtype`): 0 none, 1 bit array, 2 i16, 3 i32,
//! 4/5 four/eight-byte reals (kept as raw bytes — floats are out of
//! scope for a deterministic kit), 6 ASCII string.

/// A parsed stream head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gds {
    /// HEADER record's version (conventionally 3..=7).
    pub version: i16,
    /// Byte offset of the first record after HEADER.
    pub next: usize,
}

/// One record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record {
    /// Record tag byte (`0x00` HEADER … `0x3F` PLEX).
    pub tag: u8,
    /// Payload data-type nibble code.
    pub dtype: u8,
    /// Absolute offset of the payload.
    pub at: usize,
    /// Payload byte length (`reclen - 4`).
    pub len: usize,
}

fn u16b(d: &[u8], at: usize) -> Option<u16> {
    let b = d.get(at..at + 2)?;
    Some((b[0] as u16) << 8 | b[1] as u16)
}

/// Parse the mandatory leading HEADER record.
pub fn parse(d: &[u8]) -> Option<Gds> {
    let len = u16b(d, 0)? as usize;
    if len < 4 || len > d.len() || d[2] != 0x00 {
        return None;
    }
    Some(Gds {
        version: i16s(d, 4).unwrap_or(0),
        next: len,
    })
}

/// Walk every record, stopping at the first malformed length.
pub fn records(d: &[u8]) -> Vec<Record> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(&tag) = d.get(at + 2) {
        let Some(len) = u16b(d, at).map(|n| n as usize) else {
            break;
        };
        if len < 4 || at + len > d.len() {
            break;
        }
        out.push(Record {
            tag,
            dtype: d[at + 3],
            at: at + 4,
            len: len - 4,
        });
        at += len;
    }
    out
}

/// i16 payload helper (dtype 2 / first two bytes of any payload).
pub fn i16s(d: &[u8], at: usize) -> Option<i16> {
    Some(u16b(d, at)? as i16)
}

/// i32 payload helper (dtype 3).
pub fn i32s(d: &[u8], at: usize) -> Option<i32> {
    let b = d.get(at..at + 4)?;
    Some(((b[0] as i32) << 24) | ((b[1] as i32) << 16) | ((b[2] as i32) << 8) | (b[3] as i32))
}

/// ASCII payload (dtype 6) as a byte slice.
pub fn strval<'a>(d: &'a [u8], r: &Record) -> Option<&'a [u8]> {
    if r.dtype == 6 {
        d.get(r.at..r.at + r.len)
    } else {
        None
    }
}

/// Canonical tag names for the structural records.
pub fn tag_name(tag: u8) -> &'static str {
    match tag {
        0x00 => "HEADER",
        0x01 => "BGNLIB",
        0x02 => "LIBNAME",
        0x03 => "UNITS",
        0x04 => "ENDLIBS",
        0x05 => "BGNSTR",
        0x06 => "STRNAME",
        0x07 => "ENDSTR",
        0x08 => "BOUNDARY",
        0x09 => "PATH",
        0x0A => "SREF",
        0x0B => "AREF",
        0x0C => "TEXT",
        0x0D => "LAYER",
        0x0E => "DATATYPE",
        0x0F => "WIDTH",
        0x10 => "XY",
        0x11 => "ENDEL",
        0x12 => "SNAME",
        0x13 => "ENDLIB",
        0x15 => "NODE",
        0x16 => "TEXTTYPE",
        0x17 => "PRESENTATION",
        0x19 => "STRING",
        0x1A => "STRTYPE",
        0x1B => "PATHTYPE",
        0x1C => "GENERATIONS",
        0x1D => "ATTRTABLE",
        0x21 => "NODETYPE",
        0x22 => "PROPATTR",
        0x23 => "PROPVALUE",
        0x25 => "BOX",
        0x26 => "BOXTYPE",
        0x2A => "BGNEXTN",
        0x2B => "ENDEXTN",
        0x2C => "TEXTNODE",
        0x31 => "PLEX",
        0x3F => "ELFLAGS",
        _ => "unknown",
    }
}

/// Payload data-type names.
pub fn dtype_name(dtype: u8) -> &'static str {
    match dtype {
        0 => "none",
        1 => "bitarray",
        2 => "i16",
        3 => "i32",
        4 => "real4",
        5 => "real8",
        6 => "string",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut d = vec![0, 6, 0, 2, 0, 3]; // HEADER v3
        d.extend_from_slice(&[0, 8, 0x02, 6]); // LIBNAME "LIB\0"
        d.extend_from_slice(b"LIB\0");
        d.extend_from_slice(&[0, 4, 0x13, 0]); // ENDLIB
        d
    }

    #[test]
    fn header_and_walk() {
        let d = fixture();
        let g = parse(&d).unwrap();
        assert_eq!(g.version, 3);
        let rs = records(&d);
        assert_eq!(rs.len(), 3);
        assert_eq!(tag_name(rs[1].tag), "LIBNAME");
        assert_eq!(strval(&d, &rs[1]), Some(&b"LIB\0"[..]));
        assert_eq!(dtype_name(rs[1].dtype), "string");
    }

    #[test]
    fn helpers() {
        let d = fixture();
        assert_eq!(i16s(&d, 4), Some(3));
        assert_eq!(i32s(&[0xFF, 0xFF, 0xFF, 0xFF], 0), Some(-1));
        assert_eq!(tag_name(0x7F), "unknown");
        assert_eq!(dtype_name(9), "unknown");
        assert_eq!(dtype_name(0), "none");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut d = fixture();
        d[2] = 1; // first record not HEADER
        assert!(parse(&d).is_none());
        // truncated record ends the walk
        let mut d2 = fixture();
        d2[6] = 9; // reclen 2304 runs past EOF
        assert_eq!(records(&d2).len(), 1);
    }
}
