//! LEF — Library Exchange Format, the ASCII physical-abstract
//! description fed to place & route tools. Statements are
//! `KEYWORD value* ;` terminated by `;`, with block statements
//! (`MACRO name … END name`, `PIN name … END name`).
//!
//! ```
//! use izanagi_kit::lef::{parse, macros, pins};
//! let d = b"VERSION 5.8 ;\nUNITS\n  DATABASE MICRONS 1000 ;\nEND UNITS\nMACRO inv\nPIN A\nEND A\nEND inv\n";
//! let l = parse(d).unwrap();
//! assert_eq!(l.version.as_slice(), b"5.8");
//! assert_eq!(l.dbu, Some(1000));
//! assert_eq!(macros(d).len(), 1);
//! assert_eq!(pins(d).len(), 1);
//! ```

/// A parsed header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lef {
    /// `VERSION` token text.
    pub version: Vec<u8>,
    /// `UNITS DATABASE MICRONS n` — database units per micron.
    pub dbu: Option<u32>,
}

fn words(line: &[u8]) -> Vec<&[u8]> {
    line.split(|&b| b == b' ' || b == b'\t')
        .filter(|w| !w.is_empty() && *w != b";")
        .collect()
}

fn lines(d: &[u8]) -> impl Iterator<Item = &[u8]> {
    d.split(|&b| b == b'\n')
        .map(|l| l.strip_suffix(b"\r").unwrap_or(l))
        .map(|l| {
            let mut e = l.len();
            while e > 0 && (l[e - 1] == b' ' || l[e - 1] == b'\t') {
                e -= 1;
            }
            &l[..e]
        })
}

fn u32s(a: &[u8]) -> Option<u32> {
    if a.is_empty() || a.iter().any(|b| !b.is_ascii_digit()) {
        return None;
    }
    let mut n = 0u32;
    for &b in a {
        n = n.checked_mul(10)?.checked_add((b - b'0') as u32)?;
    }
    Some(n)
}

/// Parse `VERSION` and `UNITS DATABASE MICRONS`.
pub fn parse(d: &[u8]) -> Option<Lef> {
    let mut version = None;
    let mut dbu = None;
    for l in lines(d) {
        let w = words(l);
        match w.first().copied() {
            Some(b"VERSION") => version = w.get(1).map(|v| v.to_vec()),
            Some(b"DATABASE") if w.get(1).copied() == Some(b"MICRONS") => {
                dbu = w.get(2).and_then(|v| u32s(v));
            }
            _ => {}
        }
    }
    version.map(|v| Lef { version: v, dbu })
}

/// `MACRO name` block headers, in order.
pub fn macros(d: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut in_macro = false;
    for l in lines(d) {
        let w = words(l);
        match w.first().copied() {
            Some(b"MACRO") => {
                in_macro = true;
                if let Some(n) = w.get(1) {
                    out.push(n.to_vec());
                }
            }
            Some(b"END") if in_macro => in_macro = false,
            _ => {}
        }
    }
    out
}

/// `PIN name` entries (each terminated by its own `END`).
pub fn pins(d: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for l in lines(d) {
        let w = words(l);
        if w.first().copied() == Some(b"PIN") {
            if let Some(n) = w.get(1) {
                out.push(n.to_vec());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"VERSION 5.8 ;\nUNITS\n  DATABASE MICRONS 1000 ;\nEND UNITS\nMACRO inv_a\n  PIN A ;\n  PIN Y ;\nEND inv_a\n";

    #[test]
    fn header() {
        let l = parse(DOC).unwrap();
        assert_eq!(l.version.as_slice(), b"5.8");
        assert_eq!(l.dbu, Some(1000));
    }

    #[test]
    fn blocks() {
        assert_eq!(macros(DOC), vec![b"inv_a".to_vec()]);
        assert_eq!(pins(DOC), vec![b"A".to_vec(), b"Y".to_vec()]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"UNITS DATABASE MICRONS 1 ;").is_none());
        assert!(macros(b"").is_empty());
    }
}
