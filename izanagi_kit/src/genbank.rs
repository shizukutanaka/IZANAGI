//! GenBank flat file — NCBI's fixed-column record: `LOCUS` header,
//! `DEFINITION`/`ACCESSION`/`VERSION` lines, a `FEATURES` table and an
//! `ORIGIN` sequence block closed by `//`.
//!
//! Coordinates and sequence are exposed as text/bytes; FEATURES are
//! reported as `(key, location)` pairs with qualifiers left inline.
//!
//! ```
//! use izanagi_kit::genbank::{parse, Feature};
//!
//! let d = b"LOCUS       TEST                 100 bp    DNA     circular\n\
//!           DEFINITION  a test record.\n\
//!           ACCESSION   AB000001\n\
//!           FEATURES             Location/Qualifiers\n\
//!           \x20    source          1..100\n\
//!           ORIGIN\n        1 acgt acgt\n//\n";
//! let g = parse(d).unwrap();
//! assert_eq!(g.locus.name, "TEST");
//! assert!(g.locus.circular);
//! assert_eq!(g.accession(), Some("AB000001"));
//! assert_eq!(g.sequence(), b"acgtacgt");
//! ```

use std::vec::Vec;

/// Parsed LOCUS line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locus<'a> {
    /// Locus name token.
    pub name: &'a str,
    /// Sequence length in bp (0 when absent).
    pub length: u64,
    /// `circular` appeared on the LOCUS line.
    pub circular: bool,
}

/// One FEATURES entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feature<'a> {
    /// Feature key (`source`, `CDS`, `gene`, …).
    pub key: &'a str,
    /// Location string (`1..100`, `complement(5..9)`, …).
    pub location: &'a str,
}

/// Parsed GenBank record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Genbank<'a> {
    /// Parsed LOCUS header.
    pub locus: Locus<'a>,
    d: &'a [u8],
}

fn text(b: &[u8]) -> &str {
    std::str::from_utf8(b).unwrap_or("")
}

fn line<'a>(d: &'a [u8], at: &mut usize) -> Option<&'a [u8]> {
    if *at >= d.len() {
        return None;
    }
    let start = *at;
    let end = d[start..]
        .iter()
        .position(|&c| c == b'\n')
        .map(|p| start + p)
        .unwrap_or(d.len());
    *at = end + usize::from(end < d.len());
    let mut l = &d[start..end];
    if l.last() == Some(&b'\r') {
        l = &l[..l.len() - 1];
    }
    Some(l)
}

/// Parse a GenBank record; requires a `LOCUS` first line.
pub fn parse(d: &[u8]) -> Option<Genbank<'_>> {
    let mut at = 0;
    let first = line(d, &mut at)?;
    if !first.starts_with(b"LOCUS") {
        return None;
    }
    let t = text(first);
    let mut it = t.split_whitespace();
    let _ = it.next(); // LOCUS
    let name = it.next().unwrap_or("");
    let length = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let circular = t.contains("circular");
    Some(Genbank {
        locus: Locus {
            name,
            length,
            circular,
        },
        d,
    })
}

impl<'a> Genbank<'a> {
    /// First `ACCESSION` value.
    pub fn accession(&self) -> Option<&'a str> {
        self.keyword_line(b"ACCESSION").next()
    }

    /// `DEFINITION` joined across continuation lines.
    pub fn definition(&self) -> Vec<&'a str> {
        self.keyword_line(b"DEFINITION").collect()
    }

    /// `FEATURES` table entries: `(key, location)` pairs.
    pub fn features(&self) -> Features<'a> {
        Features {
            d: self.d,
            at: 0,
            in_features: false,
        }
    }

    /// ORIGIN sequence letters (whitespace/digits stripped).
    pub fn sequence(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut at = 0;
        let mut in_origin = false;
        while let Some(l) = line(self.d, &mut at) {
            if in_origin {
                if l.starts_with(b"//") {
                    break;
                }
                out.extend(l.iter().filter(|c| c.is_ascii_alphabetic()).copied());
            } else if l.starts_with(b"ORIGIN") {
                in_origin = true;
            }
        }
        out
    }

    fn keyword_line<'b>(&self, kw: &'b [u8]) -> KeywordLines<'a, 'b> {
        KeywordLines {
            d: self.d,
            at: 0,
            kw,
            active: false,
        }
    }
}

/// Continuation-aware iterator over one top-level keyword's value.
pub struct KeywordLines<'a, 'b> {
    d: &'a [u8],
    at: usize,
    kw: &'b [u8],
    active: bool,
}

impl<'a> Iterator for KeywordLines<'a, '_> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        while let Some(l) = line(self.d, &mut self.at) {
            if l.starts_with(b"//") {
                return None;
            }
            let is_keyword = !l.is_empty() && l[0] != b' ';
            if is_keyword {
                self.active = l.starts_with(self.kw);
                if self.active {
                    return Some(text(l).get(self.kw.len()..).unwrap_or("").trim());
                }
            } else if self.active {
                return Some(text(l).trim());
            }
        }
        None
    }
}

/// FEATURES table iterator.
pub struct Features<'a> {
    d: &'a [u8],
    at: usize,
    in_features: bool,
}

impl<'a> Iterator for Features<'a> {
    type Item = Feature<'a>;
    fn next(&mut self) -> Option<Feature<'a>> {
        while let Some(l) = line(self.d, &mut self.at) {
            if !self.in_features {
                if l.starts_with(b"FEATURES") {
                    self.in_features = true;
                }
                continue;
            }
            if l.starts_with(b"ORIGIN") || l.starts_with(b"//") || (!l.is_empty() && l[0] != b' ') {
                return None;
            }
            let t = text(l);
            // feature rows have the key in columns 6..21, location from 21
            let row = t.get(5..).unwrap_or("");
            if row.trim_start().starts_with('/') {
                continue; // qualifier continuation
            }
            let mut it = row.split_whitespace();
            if let (Some(key), Some(loc)) = (it.next(), it.next()) {
                return Some(Feature { key, location: loc });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"LOCUS       TEST                  12 bp    DNA     circular\n\
        DEFINITION  first line\n            second line.\n\
        ACCESSION   AB000001\n\
        FEATURES             Location/Qualifiers\n\
        \x20    source          1..12\n\
        \x20    gene            3..9\n\
        \x20                    /gene=\"x\"\n\
        ORIGIN\n        1 acgtacgt acgt\n//\n";

    #[test]
    fn locus_and_fields() {
        let g = parse(D).unwrap();
        assert_eq!(g.locus.length, 12);
        assert!(g.locus.circular);
        assert_eq!(g.accession(), Some("AB000001"));
        assert_eq!(g.definition(), ["first line", "second line."]);
    }

    #[test]
    fn features_and_sequence() {
        let g = parse(D).unwrap();
        let f: Vec<_> = g.features().collect();
        assert_eq!(f.len(), 2);
        assert_eq!((f[0].key, f[0].location), ("source", "1..12"));
        assert_eq!(g.sequence(), b"acgtacgtacgt");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"not a locus\n").is_none());
        assert!(parse(b"").is_none());
        let g = parse(b"LOCUS       X                      5 bp    DNA\n//\n").unwrap();
        assert!(!g.locus.circular);
        assert_eq!(g.accession(), None);
        assert_eq!(g.features().count(), 0);
    }
}
