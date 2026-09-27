//! Mobipocket / PalmDOC headers: 78-byte PDB record header (`name` at 0,
//! `num_records` at 76) followed by record 0 whose `compression` word and
//! `MOBI` type-marker identify the book format.
//!
//! ```
//! use izanagi_kit::mobi::parse;
//!
//! let mut d = vec![0u8; 78 + 32];
//! d[0..4].copy_from_slice(b"Book");
//! d[76] = 0; d[77] = 1;           // 1 record
//! d[78] = 0; d[79] = 1;           // compression = none
//! d[94..98].copy_from_slice(b"MOBI"); // MOBI marker at record0+16
//! let m = parse(&d).unwrap();
//! assert_eq!(m.name, "Book");
//! assert!(m.is_mobi());
//! ```

use std::string::String;

/// Record-0 offset right after the 78-byte PalmDOC header.
pub const HEADER: usize = 78;
/// `MOBI` marker offset inside record 0.
pub const MOBI_AT: usize = 16;
/// AZW3's extra `BOUNDARY` record marker.
pub const BOUNDARY: &[u8] = b"BOUNDARY";

fn be16(d: &[u8], at: usize) -> Option<u16> {
    let (a, b) = (d.get(at)?, d.get(at + 1)?);
    Some((u16::from(*a) << 8) | u16::from(*b))
}

fn name_field(d: &[u8]) -> String {
    let raw = d.get(0..32).unwrap_or(&[]);
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..end]).into_owned()
}

/// Parsed PalmDOC/MOBI header.
#[derive(Debug, Clone)]
pub struct Mobi {
    /// Database name (32B, NUL-trimmed).
    pub name: String,
    /// Compression scheme: 1 none, 2 PalmDOC, 17480 HUFF/CDIC.
    pub compression: u16,
    /// Record count.
    pub records: u16,
    /// `MOBI` marker present at record0+16.
    pub mobi: bool,
    /// Record-0 offset of the `BOUNDARY` marker if present (KF8/AZW3).
    pub boundary_at: Option<usize>,
}

/// Parse the PalmDOC header plus the MOBI marker inside record 0.
pub fn parse(d: &[u8]) -> Option<Mobi> {
    if d.len() < HEADER {
        return None;
    }
    let records = be16(d, 76)?;
    if records == 0 {
        return None;
    }
    let compression = be16(d, HEADER)?;
    let mobi = d
        .get(HEADER + MOBI_AT..HEADER + MOBI_AT + 4)
        .map(|s| s == b"MOBI")
        .unwrap_or(false);
    let boundary_at = windows_find(d, BOUNDARY);
    Some(Mobi {
        name: name_field(d),
        compression,
        records,
        mobi,
        boundary_at,
    })
}

/// First offset of `needle` in `d` (byte scan — KF8 boundaries sit in
/// ordinary records, so a whole-file scan is the honest check).
fn windows_find(d: &[u8], needle: &[u8]) -> Option<usize> {
    d.windows(needle.len()).position(|w| w == needle)
}

impl Mobi {
    /// True when record 0 carries the `MOBI` marker.
    pub fn is_mobi(&self) -> bool {
        self.mobi
    }
    /// True when a `BOUNDARY` section exists — the AZW3/KF8 twin-header
    /// layout.
    pub fn is_kf8(&self) -> bool {
        self.boundary_at.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(mobi: bool, boundary: bool) -> Vec<u8> {
        let mut d = vec![0u8; 200];
        d[0..4].copy_from_slice(b"MyBk");
        d[77] = 2; // records = 2
        d[79] = 2; // compression = PalmDOC
        if mobi {
            d[HEADER + MOBI_AT..HEADER + MOBI_AT + 4].copy_from_slice(b"MOBI");
        }
        if boundary {
            d[150..158].copy_from_slice(BOUNDARY);
        }
        d
    }

    #[test]
    fn fields() {
        let m = parse(&fixture(true, true)).unwrap();
        assert_eq!(m.name, "MyBk");
        assert_eq!(m.compression, 2);
        assert_eq!(m.records, 2);
        assert!(m.is_mobi());
        assert!(m.is_kf8());
        assert_eq!(m.boundary_at, Some(150));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"short").is_none());
        let mut d = fixture(false, false);
        d[77] = 0; // 0 records
        assert!(parse(&d).is_none());
    }
}
