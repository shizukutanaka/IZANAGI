//! MPS — the classic fixed-format linear/mixed-integer programming
//! interchange file: `NAME`, then sections `ROWS` (`N` objective,
//! `G`/`L`/`E` senses), `COLUMNS` (matrix entries), optional
//! `RHS`/`RANGES`/`BOUNDS` (`BV`/`UP`/`LO`/`FX`/`FR`/`MI`/`PL`),
//! `ENDATA`. Integer `MARKER` cards carry `INTORG`/`INTEND`.
//!
//! ```
//! use izanagi_kit::mps::{detect, parse};
//!
//! let d = b"NAME          TEST\nROWS\n N  COST\n L  LIM1\nCOLUMNS\n    X         COST      1   LIM1      1\n\
//! RHS\n    RHS       LIM1      4\nBOUNDS\n UP BND       X         10\nENDATA\n";
//! assert!(detect(d));
//! let m = parse(d).unwrap();
//! assert_eq!(m.objectives, 1);
//! assert_eq!(m.columns, 1);
//! ```

/// Parsed MPS census.
#[derive(Debug, Clone, PartialEq)]
pub struct Mps {
    /// `NAME` line text.
    pub name: Option<String>,
    /// `N` (objective) rows.
    pub objectives: u32,
    /// `G` (`>=`) constraint rows.
    pub g_rows: u32,
    /// `L` (`<=`) constraint rows.
    pub l_rows: u32,
    /// `E` (`=`) constraint rows.
    pub e_rows: u32,
    /// Distinct column names in `COLUMNS`.
    pub columns: u32,
    /// Matrix entries (row,value) pairs under `COLUMNS`.
    pub nonzeros: u32,
    /// `RHS` entries.
    pub rhs: u32,
    /// `RANGES` entries.
    pub ranges: u32,
    /// `BOUNDS` entries.
    pub bounds: u32,
    /// `INTORG`/`INTEND` markers seen.
    pub markers: u32,
    /// Free (`FR`), integer (`BV`), lower (`LO`), upper (`UP`), fixed (`FX`) bound kinds.
    pub bound_kinds: Vec<String>,
}

/// `true` on `NAME` … `ROWS`/`COLUMNS`/`ENDATA` cards.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.lines().any(|l| l.starts_with("NAME")) && s.contains("ROWS") && s.contains("ENDATA")
}

/// Census; `None` without the MPS skeleton.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Mps> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut m = Mps {
        name: None,
        objectives: 0,
        g_rows: 0,
        l_rows: 0,
        e_rows: 0,
        columns: 0,
        nonzeros: 0,
        rhs: 0,
        ranges: 0,
        bounds: 0,
        markers: 0,
        bound_kinds: Vec::new(),
    };
    let mut section = "";
    let mut cols = std::collections::BTreeSet::new();
    for line in s.lines() {
        let l = line.trim_end();
        if l.is_empty() || l.starts_with('*') {
            continue;
        }
        if section == "COLUMNS" && (l.contains("INTORG") || l.contains("INTEND")) {
            m.markers += 1;
            continue;
        }
        if !l.starts_with(' ') && !l.starts_with('\t') {
            let head = l.split_whitespace().next().unwrap_or("");
            match head {
                "NAME" => m.name = l.get(4..).map(|v| v.trim().to_string()),
                "ROWS" | "COLUMNS" | "RHS" | "RANGES" | "BOUNDS" | "ENDATA" => {
                    section = head;
                }
                _ => {}
            }
            continue;
        }
        let mut it = l.split_whitespace();
        match section {
            "ROWS" => match it.next().unwrap_or("") {
                "N" => m.objectives += 1,
                "G" => m.g_rows += 1,
                "L" => m.l_rows += 1,
                "E" => m.e_rows += 1,
                _ => {}
            },
            "COLUMNS" => {
                if let Some(col) = it.next() {
                    cols.insert(col.to_string());
                    m.nonzeros += it.by_ref().count() as u32 / 2;
                }
            }
            "RHS" => m.rhs += 1,
            "RANGES" => m.ranges += 1,
            "BOUNDS" => {
                if let Some(k) = it.next() {
                    m.bounds += 1;
                    let k = k.to_string();
                    if !m.bound_kinds.contains(&k) {
                        m.bound_kinds.push(k);
                    }
                }
            }
            _ => {}
        }
    }
    m.columns = u32::try_from(cols.len()).unwrap_or(u32::MAX);
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"NAME          TEST\nROWS\n N  COST\n L  LIM1\n E  EQ1\n G  G1\n\
COLUMNS\n    X         COST      1   LIM1      1\n    Y         COST      2   EQ1       1\n\
    MARKER                 'MARKER'                 'INTORG'\n    Z         G1        1\n\
    MARKER                 'MARKER'                 'INTEND'\nRHS\n    RHS       LIM1      4\n\
RANGES\n    RNG       LIM1      2\nBOUNDS\n UP BND       X         10\n BV BND       Z\nENDATA\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"NAME X\nENDATA\n"));
    }

    #[test]
    fn parses() {
        let m = parse(D).unwrap();
        assert_eq!(m.name.as_deref(), Some("TEST"));
        assert_eq!(m.objectives, 1);
        assert_eq!(m.l_rows, 1);
        assert_eq!(m.e_rows, 1);
        assert_eq!(m.g_rows, 1);
        assert_eq!(m.columns, 3);
        assert_eq!(m.nonzeros, 5);
        assert_eq!(m.rhs, 1);
        assert_eq!(m.ranges, 1);
        assert_eq!(m.bounds, 2);
        assert_eq!(m.markers, 2);
        assert_eq!(m.bound_kinds, vec!["UP".to_string(), "BV".to_string()]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}
