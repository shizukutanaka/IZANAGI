//! Gaussian formatted checkpoint (`*.fchk`) — a title line, a
//! `task method basis` line, then labeled fields: scalar
//! `Name<43 cols> T             value` or array
//! `Name<43 cols> T   N=         count` followed by packed values.
//!
//! Values are kept as raw tokens (integer vs. real is reported via the
//! field's type letter) so the parser stays float-free.
//!
//! ```
//! use izanagi_kit::fchk::parse;
//!
//! let f = parse(b"title\nSP        RB3LYP         STO-3G\nNumber of atoms                            I              2\nCharge                                     I              0\nAtomic numbers                             I   N=           2\n   8  1\n").unwrap();
//! assert_eq!(f.method, "RB3LYP");
//! assert_eq!(f.fields[0].name, "Number of atoms");
//! assert_eq!(f.fields[2].values.len(), 2);
//! ```

use std::string::String;
use std::vec::Vec;

/// One labeled field.
#[derive(Clone, Debug)]
pub struct Field {
    /// Field name (columns 0..43 trimmed).
    pub name: String,
    /// Type letter: `I` integer, `R` real, `C` char, `L` logical, `A` string array.
    pub kind: u8,
    /// Array length when the field was introduced by `N=`; `None` = scalar.
    pub n: Option<usize>,
    /// Raw value tokens (1 for scalars, `n` for arrays when complete).
    pub values: Vec<String>,
}

/// A parsed `.fchk` file.
#[derive(Clone, Debug)]
pub struct Fchk {
    /// Title line.
    pub title: String,
    /// Task keyword (e.g. `SP`, `Freq`, `Opt`).
    pub task: String,
    /// Method (`RB3LYP`, …).
    pub method: String,
    /// Basis set label.
    pub basis: String,
    /// Labeled fields in file order.
    pub fields: Vec<Field>,
}

impl Fchk {
    /// Look up a scalar field's integer value.
    pub fn int(&self, name: &str) -> Option<i64> {
        self.fields
            .iter()
            .find(|f| f.name == name && f.kind == b'I' && f.n.is_none())
            .and_then(|f| f.values.first()?.parse().ok())
    }
}

/// Parse a formatted checkpoint file.
pub fn parse(d: &[u8]) -> Option<Fchk> {
    let text = std::str::from_utf8(d).ok()?;
    let mut lines = text.split('\n');
    let title = lines.next()?.trim_end_matches('\r').to_string();
    let head = lines.next()?.trim_end_matches('\r').to_string();
    let mut hw = head.split_whitespace();
    let task = hw.next()?.to_string();
    let method = hw.next()?.to_string();
    let basis = hw.next()?.to_string();

    let mut fields: Vec<Field> = Vec::new();
    let mut pending: Option<usize> = None; // array field index awaiting N values
    for raw in lines {
        let l = raw.trim_end_matches('\r');
        if l.trim().is_empty() {
            continue;
        }
        if let Some(idx) = pending {
            // array continuation: pure whitespace-separated tokens
            let need = fields[idx].n.unwrap_or(0);
            let have = fields[idx].values.len();
            if have < need && !l[..l.len().min(43)].contains('=') {
                let is_labelish = l.len() > 43 && {
                    let t = &l[43..];
                    let t = t.trim_start();
                    t.starts_with('I') || t.starts_with('R') || t.starts_with('C')
                };
                if !is_labelish {
                    for tok in l.split_whitespace() {
                        fields[idx].values.push(tok.to_string());
                    }
                    if fields[idx].values.len() >= need {
                        pending = None;
                    }
                    continue;
                }
            }
            if fields[idx].values.len() < need {
                return None; // truncated array
            }
            pending = None;
        }
        if l.len() < 44 {
            return None;
        }
        let name = l.get(..43)?.trim().to_string();
        let rest = l.get(43..)?.trim_start();
        let kind = *rest.as_bytes().first()?;
        if !matches!(kind, b'I' | b'R' | b'C' | b'L' | b'A') {
            return None;
        }
        let rest = rest[1..].trim_start();
        if let Some(r) = rest.strip_prefix("N=") {
            let n: usize = r.trim().parse().ok()?;
            fields.push(Field {
                name,
                kind,
                n: Some(n),
                values: Vec::new(),
            });
            if n > 0 {
                pending = Some(fields.len() - 1);
            }
            continue;
        }
        let v = rest.split_whitespace().next()?.to_string();
        fields.push(Field {
            name,
            kind,
            n: None,
            values: vec![v],
        });
    }
    if pending.is_some() {
        return None;
    }
    if fields.is_empty() {
        return None;
    }
    Some(Fchk {
        title,
        task,
        method,
        basis,
        fields,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"water freq\nFreq      RB3LYP         6-31G(d)\nNumber of atoms                            I              3\nCharge                                     I              0\nAtomic numbers                             I   N=           3\n           8           1           1\nDipole Moment                              R   N=           3\n  0.00000000E+00  0.00000000E+00 -7.50000000E-01\n";

    #[test]
    fn header_and_fields() {
        let f = parse(DOC).unwrap();
        assert_eq!(f.task, "Freq");
        assert_eq!(f.method, "RB3LYP");
        assert_eq!(f.basis, "6-31G(d)");
        assert_eq!(f.int("Number of atoms"), Some(3));
        assert_eq!(f.int("Charge"), Some(0));
        let an = &f.fields[2];
        assert_eq!(an.n, Some(3));
        assert_eq!(an.values, vec!["8", "1", "1"]);
        assert_eq!(f.fields[3].values.len(), 3);
    }

    #[test]
    fn rejects_truncated_array() {
        assert!(parse(b"t\nSP RHF STO-3G\nA                                          I   N=           5\n 1 2\n").is_none());
        assert!(parse(b"t\nshort\nx\n").is_none());
        assert!(parse(b"t\nSP RHF STO-3G\n").is_none());
    }
}
