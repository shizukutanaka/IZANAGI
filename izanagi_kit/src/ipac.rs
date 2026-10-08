//! IPAC Table (IRSA `*.tbl`) census.
//!
//! `\` directive lines, one or more `|`-delimited header rows
//! (names then `int`/`double`/`char` type row, optional `unit`/`null`
//! rows), then whitespace-separated data rows.
//!
//! ```
//! let s = b"\\title = demo\n| ra  | dec |\n| double | double |\n| deg | deg |\n 1 2\n 3 4\n";
//! assert!(izanagi_kit::ipac::detect(s));
//! let t = izanagi_kit::ipac::Ipac::parse(s).unwrap();
//! assert_eq!(t.columns, 2);
//! assert_eq!(t.types, 2);
//! assert_eq!(t.units, 2);
//! assert_eq!(t.data_rows, 2);
//! assert_eq!(t.directives, 1);
//! ```

use crate::textutil::strip_bom;
/// Parsed census of an IPAC table.
#[derive(Debug, Clone)]
pub struct Ipac {
    /// `\`-prefixed directive lines.
    pub directives: usize,
    /// `|`-delimited header rows.
    pub header_rows: usize,
    /// Cells in the first (column-name) header row.
    pub columns: usize,
    /// Cells in the type row (`int`/`double`/`char`/`long`/`float`/`date`).
    pub types: usize,
    /// Cells in the unit row.
    pub units: usize,
    /// Cells in the null row.
    pub nulls: usize,
    /// Non-header, non-directive data rows.
    pub data_rows: usize,
    /// Distinct column names in the first header row.
    pub names: usize,
    /// `char` typed columns.
    pub chars: usize,
    /// `double`/`float`/`real` typed columns.
    pub reals: usize,
    /// `int`/`long`/`i` typed columns.
    pub ints: usize,
}

const TYPES: &[&str] = &[
    "int", "i", "double", "d", "float", "f", "real", "char", "long", "l", "date", "short", "s",
];
const UNITS: &[&str] = &[
    "deg", "arcsec", "arcmin", "mag", "s", "sec", "jy", "mjy", "ujy", "hz", "khz", "mhz", "cm",
    "m", "km", "au", "pc", "kpc", "mpc", "d", "yr", "h", "min", "k", "nm", "um", "angstrom", "adu",
    "count", "counts", "dn", "e", "ph", "pix", "pixel", "sr", "w", "mw", "db", "pct", "%", "null",
];

fn cells(l: &str) -> Vec<&str> {
    l.split('|')
        .map(|c| c.trim())
        .filter(|c| !c.is_empty())
        .collect()
}

/// Reports whether `b` looks like an IPAC table.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = core::str::from_utf8(b) else {
        return false;
    };
    let t = strip_bom(t);
    let pipes = t
        .lines()
        .filter(|l| l.trim_start().starts_with('|'))
        .count();
    pipes >= 2
}

impl Ipac {
    /// Parses `b` as an IPAC table, returning `None` when the shape fails.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = core::str::from_utf8(b).ok()?;
        let t = strip_bom(t);
        if !detect(b) {
            return None;
        }
        let mut i = Ipac {
            directives: 0,
            header_rows: 0,
            columns: 0,
            types: 0,
            units: 0,
            nulls: 0,
            data_rows: 0,
            names: 0,
            chars: 0,
            reals: 0,
            ints: 0,
        };
        let mut saw_names = false;
        for l in t.lines() {
            let l = l.trim_end();
            if l.trim_start().starts_with('\\') {
                i.directives += 1;
                continue;
            }
            if l.trim_start().starts_with('|') {
                let c = cells(l);
                i.header_rows += 1;
                if !saw_names {
                    i.columns = c.len();
                    i.names = c.len();
                    saw_names = true;
                    continue;
                }
                let lc: Vec<String> = c.iter().map(|v| v.to_lowercase()).collect();
                if lc.iter().all(|v| TYPES.contains(&v.as_str())) {
                    i.types = c.len();
                    for v in &lc {
                        match v.as_str() {
                            "char" => i.chars += 1,
                            "double" | "d" | "float" | "f" | "real" => i.reals += 1,
                            "int" | "i" | "long" | "l" | "short" | "s" => i.ints += 1,
                            _ => {}
                        }
                    }
                } else if lc
                    .iter()
                    .all(|v| UNITS.contains(&v.as_str()) || v.is_empty())
                {
                    i.units = c.len();
                } else if lc.iter().all(|v| v == "null" || v == "nan" || v == "-") {
                    i.nulls = c.len();
                }
                continue;
            }
            if !l.trim().is_empty() {
                i.data_rows += 1;
            }
        }
        Some(i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: &[u8] =
        b"\\title = demo\n| ra  | dec |\n| double | double |\n| deg | deg |\n 1 2\n 3 4\n";

    #[test]
    fn parses_ipac() {
        assert!(detect(S));
        let t = Ipac::parse(S).unwrap();
        assert_eq!(t.columns, 2);
        assert_eq!(t.types, 2);
        assert_eq!(t.units, 2);
        assert_eq!(t.data_rows, 2);
        assert_eq!(t.directives, 1);
    }

    #[test]
    fn rejects_non_ipac() {
        assert!(!detect(b"a b x"));
        assert!(Ipac::parse(b"").is_none());
    }

    #[test]
    fn utf8_bom_is_tolerated() {
        assert_eq!(strip_bom("\u{feff}x"), "x");
        assert_eq!(strip_bom("x"), "x");
    }
}
