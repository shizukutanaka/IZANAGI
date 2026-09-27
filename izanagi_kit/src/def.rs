//! DEF — Design Exchange Format, the ASCII placed-netlist companion
//! to LEF. Statements are `KEYWORD … ;` (`VERSION`, `DESIGN`,
//! `UNITS DISTANCE MICRONS`, `COMPONENTS`, `NETS`, …) and each
//! `COMPONENTS n ; … END COMPONENTS` block lists `- name macro …`.
//!
//! ```
//! use izanagi_kit::def::{parse, components};
//! let d = b"VERSION 5.8 ;\nDESIGN top ;\nCOMPONENTS 2 ;\n- u1 inv ;\n- u2 buf ;\nEND COMPONENTS\n";
//! let df = parse(d).unwrap();
//! assert_eq!(df.design.as_slice(), b"top");
//! assert_eq!(df.components_declared, Some(2));
//! assert_eq!(components(d).len(), 2);
//! ```

/// A parsed header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Def {
    /// `VERSION` token text.
    pub version: Vec<u8>,
    /// `DESIGN name` (empty when absent).
    pub design: Vec<u8>,
    /// Declared count in `COMPONENTS n ;`.
    pub components_declared: Option<u32>,
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

/// Parse `VERSION`/`DESIGN`/`COMPONENTS n`.
pub fn parse(d: &[u8]) -> Option<Def> {
    let mut version = None;
    let mut design = Vec::new();
    let mut comp = None;
    for l in lines(d) {
        let w = words(l);
        match w.first().copied() {
            Some(b"VERSION") => version = w.get(1).map(|v| v.to_vec()),
            Some(b"DESIGN") => design = w.get(1).map(|v| v.to_vec()).unwrap_or_default(),
            Some(b"COMPONENTS") => comp = w.get(1).and_then(|v| u32s(v)),
            _ => {}
        }
    }
    version.map(|v| Def {
        version: v,
        design,
        components_declared: comp,
    })
}

/// `(instance, macro)` pairs from every COMPONENTS block.
pub fn components(d: &[u8]) -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut out = Vec::new();
    let mut in_block = false;
    for l in lines(d) {
        let w = words(l);
        match w.first().copied() {
            Some(b"COMPONENTS") => in_block = true,
            Some(b"END") if in_block => in_block = false,
            Some(b"-") if in_block => {
                if let (Some(i), Some(m)) = (w.get(1), w.get(2)) {
                    out.push((i.to_vec(), m.to_vec()));
                }
            }
            _ => {}
        }
    }
    out
}

/// Net names from every `NETS` block (`- name …`).
pub fn nets(d: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut in_block = false;
    for l in lines(d) {
        let w = words(l);
        match w.first().copied() {
            Some(b"NETS") => in_block = true,
            Some(b"END") if in_block => in_block = false,
            Some(b"-") if in_block => {
                if let Some(n) = w.get(1) {
                    out.push(n.to_vec());
                }
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"VERSION 5.8 ;\nDESIGN top ;\nUNITS DISTANCE MICRONS 2000 ;\nCOMPONENTS 2 ;\n- u1 inv PLACED ( 0 0 ) N ;\n- u2 buf ;\nEND COMPONENTS\nNETS 1 ;\n- n1 ( u1 Y ) ( u2 A ) ;\nEND NETS\n";

    #[test]
    fn header() {
        let df = parse(DOC).unwrap();
        assert_eq!(df.version.as_slice(), b"5.8");
        assert_eq!(df.design.as_slice(), b"top");
        assert_eq!(df.components_declared, Some(2));
    }

    #[test]
    fn blocks() {
        assert_eq!(
            components(DOC),
            vec![
                (b"u1".to_vec(), b"inv".to_vec()),
                (b"u2".to_vec(), b"buf".to_vec())
            ]
        );
        assert_eq!(nets(DOC), vec![b"n1".to_vec()]);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"DESIGN d ;").is_none());
        // stray '-' outside a block is not a component
        assert!(components(b"- x y ;").is_empty());
    }
}
