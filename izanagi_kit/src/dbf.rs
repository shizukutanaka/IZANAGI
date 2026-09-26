//! dBASE III `.dbf` — the attribute table that completes the
//! shapefile trio. Fixed-width records: 32-byte header, 32-byte
//! field descriptors ending in `0x0D`, then flag-prefixed rows.
//! All fields are raw bytes; `C` is text, `N`/`F` are ASCII
//! numerics, `D` is `YYYYMMDD`, `L` is `T/F/?`.
//!
//! ```
//! use izanagi_kit::dbf;
//! let mut d = vec![0x03, 25, 9, 1];
//! d.extend_from_slice(&[1, 0, 0, 0]);        // 1 record
//! d.extend_from_slice(&[96, 0]);             // header len 96
//! d.extend_from_slice(&[6, 0]);              // record len 6
//! d.extend_from_slice(&[0; 20]);
//! let mut f = [0u8; 32];
//! f[..4].copy_from_slice(b"NAME");
//! f[11] = b'C';
//! f[16] = 5;
//! d.extend_from_slice(&f);
//! d.push(0x0D);                              // end of descriptors
//! while d.len() < 96 {
//!     d.push(0);                             // pad to header len
//! }
//! d.push(b' ');
//! d.extend_from_slice(b"PIKE ");
//! d.push(0x1A);
//! let t = dbf::parse(&d).unwrap();
//! assert_eq!(t.records.len(), 1);
//! assert_eq!(dbf::cell(&d, &t, 0, 0), b"PIKE ");
//! assert!(!dbf::deleted(&d, &t, 0));
//! ```

use std::vec::Vec;

fn rl16(d: &[u8], at: usize) -> Option<u32> {
    Some(*d.get(at)? as u32 | (*d.get(at + 1)? as u32) << 8)
}
fn rl32(d: &[u8], at: usize) -> Option<u32> {
    Some(
        *d.get(at)? as u32
            | (*d.get(at + 1)? as u32) << 8
            | (*d.get(at + 2)? as u32) << 16
            | (*d.get(at + 3)? as u32) << 24,
    )
}

/// One field descriptor.
#[derive(Clone, Debug)]
pub struct Field {
    /// 11-byte null-padded field name.
    pub name: [u8; 11],
    /// `C`/`N`/`D`/`F`/`L`/`M`/… type letter.
    pub ty: u8,
    /// Field width in bytes.
    pub len: u8,
    /// Decimal count (`N`/`F`).
    pub dec: u8,
    /// Byte offset of the field inside a record (after the flag).
    pub offset: usize,
}

/// A parsed `.dbf`.
#[derive(Clone, Debug)]
pub struct Dbf {
    /// Version byte (`0x03` dBASE III, `0x30` Visual FoxPro…).
    pub version: u8,
    /// Last-update `yy/mm/dd` (year since 1900).
    pub date: [u8; 3],
    /// Declared record count (bounded by file length on parse).
    pub count: u32,
    /// Byte offset where records begin.
    pub header_len: usize,
    /// Bytes per record including the delete flag.
    pub record_len: usize,
    /// Field descriptors.
    pub fields: Vec<Field>,
    /// File offsets of each record (flag byte first).
    pub records: Vec<usize>,
}

/// Type letter → short name.
pub fn field_type_name(ty: u8) -> &'static str {
    match ty {
        b'C' => "char",
        b'N' => "numeric",
        b'F' => "float-text",
        b'D' => "date",
        b'L' => "logical",
        b'M' => "memo",
        b'I' => "integer",
        b'B' => "double",
        b'G' => "general",
        b'Y' => "currency",
        _ => "unknown",
    }
}

/// Parses the header + descriptor chain and bounds-checks every
/// declared record against the file size.
pub fn parse(d: &[u8]) -> Option<Dbf> {
    let version = *d.first()?;
    if version == 0 || version == 0x02 {
        return None; // 0x02 = dBASE II layout — different shape
    }
    let count = rl32(d, 4)?;
    let header_len = rl16(d, 8)? as usize;
    let record_len = rl16(d, 10)? as usize;
    if header_len < 33 || record_len < 1 {
        return None;
    }
    // descriptor chain ends at a 0x0D byte; fields are 32B each
    let nfields = (header_len - 33) / 32;
    if 32 + nfields * 32 + 1 > header_len {
        return None;
    }
    let mut fields = Vec::with_capacity(nfields);
    let mut off = 1usize; // after the delete flag
    for i in 0..nfields {
        let at = 32 + i * 32;
        if *d.get(at)? == 0x0D {
            break;
        }
        let mut name = [0u8; 11];
        name.copy_from_slice(d.get(at..at + 11)?);
        let ty = *d.get(at + 11)?;
        let len = *d.get(at + 16)?;
        let dec = *d.get(at + 17)?;
        if len == 0 && !matches!(ty, b'0' | b'_') {
            // zero-length non-special field is corrupt
            return None;
        }
        fields.push(Field {
            name,
            ty,
            len,
            dec,
            offset: off,
        });
        off += len as usize;
    }
    if fields.is_empty() || off != record_len {
        return None; // declared record length must equal Σ fields + flag
    }
    // records: header_len + i*record_len; file may carry a 0x1A
    // trailer and memo-file tail — validate what fits.
    if d.len() < header_len {
        return None;
    }
    let avail = d.len() - header_len;
    let actual = (avail / record_len).min(count as usize);
    if actual < count as usize {
        return None; // truncated records
    }
    let mut records = Vec::with_capacity(actual);
    for i in 0..actual {
        records.push(header_len + i * record_len);
    }
    Some(Dbf {
        version,
        date: [d[1], d[2], d[3]],
        count,
        header_len,
        record_len,
        fields,
        records,
    })
}

/// `true` if record `i` is flagged deleted (`*`).
pub fn deleted(d: &[u8], dbf: &Dbf, i: usize) -> bool {
    dbf.records
        .get(i)
        .and_then(|&r| d.get(r))
        .map(|&b| b == b'*')
        .unwrap_or(false)
}

/// Record `i`'s field `f` as its raw fixed-width bytes.
pub fn cell<'a>(d: &'a [u8], dbf: &Dbf, i: usize, f: usize) -> &'a [u8] {
    let Some(&r) = dbf.records.get(i) else {
        return &[];
    };
    let Some(fl) = dbf.fields.get(f) else {
        return &[];
    };
    let at = r + fl.offset;
    d.get(at..at + fl.len as usize).unwrap_or(&[])
}

/// Field `f`'s name as a trimmed `&str` (names are ASCII, may be
/// null-padded).
pub fn field_name(dbf: &Dbf, f: usize) -> Option<&str> {
    let f = dbf.fields.get(f)?;
    let end = f.name.iter().position(|&b| b == 0).unwrap_or(f.name.len());
    std::str::from_utf8(&f.name[..end]).ok()
}

/// Field index by (case-insensitive, trimmed) name.
pub fn field_index(dbf: &Dbf, name: &str) -> Option<usize> {
    (0..dbf.fields.len()).find(|&i| {
        field_name(dbf, i)
            .map(|n| n.eq_ignore_ascii_case(name.trim()))
            .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        // 2 fields: NAME C(5), AGE N(3). 2 records, second deleted.
        let mut d = vec![0x03, 25, 9, 1];
        d.extend_from_slice(&[2, 0, 0, 0]);
        d.extend_from_slice(&[129, 0]); // header = 32+32*2+1+32 pad = 129
        d.extend_from_slice(&[9, 0]); // record = 1+5+3 = 9
        d.extend_from_slice(&[0; 20]);
        let mut f = [0u8; 32];
        f[..4].copy_from_slice(b"NAME");
        f[11] = b'C';
        f[16] = 5;
        d.extend_from_slice(&f);
        f = [0u8; 32];
        f[..3].copy_from_slice(b"AGE");
        f[11] = b'N';
        f[16] = 3;
        d.extend_from_slice(&f);
        d.push(0x0D);
        while d.len() < 129 {
            d.push(0);
        }
        d.push(b' ');
        d.extend_from_slice(b"PIKE ");
        d.extend_from_slice(b" 42");
        d.push(b'*');
        d.extend_from_slice(b"JONES");
        d.extend_from_slice(b" 7 ");
        d.push(0x1A);
        d
    }

    #[test]
    fn parses_and_reads_cells() {
        let d = fixture();
        let t = parse(&d).unwrap();
        assert_eq!(t.fields.len(), 2);
        assert_eq!(t.records.len(), 2);
        assert_eq!(field_name(&t, 0), Some("NAME"));
        assert_eq!(field_index(&t, " age "), Some(1));
        assert_eq!(cell(&d, &t, 0, 0), b"PIKE ");
        assert_eq!(cell(&d, &t, 0, 1), b" 42");
        assert!(!deleted(&d, &t, 0));
        assert!(deleted(&d, &t, 1));
        assert_eq!(field_type_name(b'C'), "char");
        assert_eq!(field_type_name(b'Z'), "unknown");
    }

    #[test]
    fn malformed_degrades() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0x02; 100]).is_none()); // dBASE II
        let mut bad = fixture();
        bad[8] = 0; // header len → 0
        assert!(parse(&bad).is_none());
        let mut short = fixture();
        short.truncate(short.len() - 5); // cut inside last record
        assert!(parse(&short).is_none());
        let mut zero_len = fixture();
        zero_len[32 + 16] = 0; // NAME len 0
        assert!(parse(&zero_len).is_none());
    }
}
