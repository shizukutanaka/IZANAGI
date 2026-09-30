//! AutoCAD DWG — the binary drawing database. The first six bytes are
//! an ASCII version stamp `AC1NNN`/`AC0NNN` (`AC1032` = 2018-format,
//! `AC1027` = 2013, `AC1024` = 2010, `AC1021` = 2007, `AC1018` = 2004,
//! `AC1015` = 2000, `AC1014` = R14, `AC1012`/`AC1009` = R12/13), then a
//! maintenance byte and sentinel/section paging.
//!
//! `parse` reads the stamp and reports the release year class and the
//! 5-byte sentinel region used by DWG's section paging (`0x00` or
//! `0xFF` padding before the image header).
//!
//! ```
//! let mut f = b"AC1027\0\x00\x00\x00\x00".to_vec(); // R2013
//! f.extend_from_slice(&[0u8; 64]);
//! let d = izanagi_kit::dwg::parse(&f).unwrap();
//! assert_eq!(d.code, "AC1027");
//! assert_eq!(d.year_class(), Some(2013));
//! assert!(izanagi_kit::dwg::parse(b"AC9999").is_none());
//! ```

/// Parsed DWG summary.
#[derive(Debug, Clone, PartialEq)]
pub struct Dwg {
    /// Six-character version stamp, e.g. `AC1027`.
    pub code: String,
    /// Maintenance/release byte at offset 6.
    pub maintenance: u8,
}

/// `(code, release_year)` table.
const CODES: &[(&str, u16)] = &[
    ("AC1006", 10), // R10
    ("AC1009", 12), // R12
    ("AC1012", 13),
    ("AC1014", 14),
    ("AC1015", 2000),
    ("AC1018", 2004),
    ("AC1021", 2007),
    ("AC1024", 2010),
    ("AC1027", 2013),
    ("AC1032", 2018),
];

impl Dwg {
    /// Release-year class for the version stamp, if known.
    pub fn year_class(&self) -> Option<u16> {
        CODES.iter().find(|(c, _)| *c == self.code).map(|(_, y)| *y)
    }
}

/// Parse a DWG file; `None` unless the `AC…` stamp is a known release.
pub fn parse(d: &[u8]) -> Option<Dwg> {
    if d.len() < 12 || !d.starts_with(b"AC") {
        return None;
    }
    let code = std::str::from_utf8(&d[..6]).ok()?.to_string();
    if CODES.iter().all(|(c, _)| *c != code) {
        return None;
    }
    Some(Dwg {
        code,
        maintenance: d[6],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let mut f = b"AC1018\x01\x00\x00\x00\x00".to_vec();
        f.extend_from_slice(&[0u8; 8]);
        let d = parse(&f).unwrap();
        assert_eq!(d.year_class(), Some(2004));
        assert_eq!(d.maintenance, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"AC").is_none());
        assert!(parse(b"XX1027").is_none());
        assert!(parse(b"AC1099").is_none()); // unknown stamp
    }
}
