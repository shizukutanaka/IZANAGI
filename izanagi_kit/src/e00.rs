//! ArcInfo `.e00` export files — Esri's line-oriented coverage
//! interchange format. The first record is `EXP  <ver>` (or `EXP`),
//! followed by labelled sections (`ARC`, `CNT`, `LAB`, `PAL`, `SIN`,
//! `TOL`, `PRJ`, `IFO`, `GRD`, `TXT`, `RXP`, `RPL`, `MSP`, `LNK`,
//! `ELP`, `BLK`, `BND`, `XGN`, `LOG`) and optionally
//! `SINGLE PRECISION`/`DOUBLE PRECISION` precision records.
//!
//! `parse` requires the `EXP` prolog plus at least one section label
//! at column 0 and reports the precision + section census.
//!
//! ```
//! let f = b"EXP  0\nARC\n  1  1  1  2\nCNT\n 1\nPRJ\nSINGLE PRECISION\nEOS\n";
//! let e = izanagi_kit::e00::parse(f).unwrap();
//! assert_eq!(e.sections, 3);
//! assert!(e.single);
//! assert!(!e.double);
//! assert!(izanagi_kit::e00::parse(b"ARC\n").is_none());
//! ```

/// Section labels recognised at column 0.
const SECTIONS: &[&str] = &[
    "ARC", "CNT", "LAB", "PAL", "SIN", "TOL", "PRJ", "IFO", "GRD", "TXT", "RXP", "RPL", "MSP",
    "LNK", "ELP", "BLK", "BND", "XGN", "LOG",
];

/// Parsed E00 summary.
#[derive(Debug, Clone, PartialEq)]
pub struct E00 {
    /// `EXP` version text (may be empty for plain `EXP`).
    pub version: String,
    /// Number of recognised section-label lines.
    pub sections: usize,
    /// `SINGLE PRECISION` record present.
    pub single: bool,
    /// `DOUBLE PRECISION` record present.
    pub double: bool,
}

/// Parse an E00 file; `None` without `EXP` prolog + section labels.
pub fn parse(d: &[u8]) -> Option<E00> {
    let s = std::str::from_utf8(d).ok()?;
    let mut lines = s.lines();
    let first = lines.next()?.trim_end();
    if !first.starts_with("EXP") {
        return None;
    }
    let version = first[3..].trim().to_string();
    let mut sections = 0usize;
    let mut single = false;
    let mut double = false;
    for line in lines {
        if SECTIONS.contains(&line) {
            sections += 1;
        }
        if line.contains("SINGLE PRECISION") {
            single = true;
        }
        if line.contains("DOUBLE PRECISION") {
            double = true;
        }
    }
    if sections == 0 {
        return None;
    }
    Some(E00 {
        version,
        sections,
        single,
        double,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let f = b"EXP  0\nIFO\nLAB\nDOUBLE PRECISION\nEOS\n";
        let e = parse(f).unwrap();
        assert_eq!(e.version, "0");
        assert_eq!(e.sections, 2);
        assert!(e.double);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"EXP\n").is_none()); // no sections
        assert!(parse(b"ARC\nCNT\n").is_none()); // no EXP
    }
}
