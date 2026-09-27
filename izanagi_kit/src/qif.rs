//! QIF (Quicken Interchange Format) file parsing.
//!
//! Sections open with `!Type:<name>`; each record is a run of
//! `X<value>` field lines (`D` date, `T` amount, `P` payee, `M`
//! memo, `N` check number, `L` category, `C` cleared, `A`
//! address…) terminated by `^`.
//!
//! ```
//! use izanagi_kit::qif;
//! let d = b"!Type:Bank\nD01/01/2024\nT-100.00\nPWalmart\n^\nD02/01/2024\nT2500.00\nPEmployer\n^\n";
//! let q = qif::parse(d).unwrap();
//! assert_eq!(q.sections.len(), 1);
//! assert_eq!(q.sections[0].records.len(), 2);
//! ```

use std::collections::BTreeMap;
use std::string::String;
use std::vec::Vec;

/// One `^`-terminated record: field letter → value(s).
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    /// Fields keyed by tag letter; repeated tags accumulate.
    pub fields: BTreeMap<u8, Vec<String>>,
}

/// A `!Type:` section.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    /// Section kind (`Bank`, `Cash`, `CCard`, `Invst`, ...).
    pub ty: String,
    /// Records in order.
    pub records: Vec<Record>,
}

/// A parsed QIF file.
#[derive(Clone, Debug, PartialEq)]
pub struct Qif {
    /// `!Option:` headers seen before/inside sections.
    pub options: Vec<String>,
    /// Sections in file order.
    pub sections: Vec<Section>,
}

/// Parses a QIF file: `!` directives open sections, `^` ends
/// records, single-letter tags carry values.
pub fn parse(d: &[u8]) -> Option<Qif> {
    let text = std::str::from_utf8(d).ok()?;
    let mut qif = Qif {
        options: Vec::new(),
        sections: Vec::new(),
    };
    let mut cur: Option<Record> = None;
    let mut saw_section = false;
    for raw in text.lines() {
        let line = raw.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        if let Some(bang) = line.strip_prefix('!') {
            let (k, v) = bang.split_once(':').unwrap_or((bang, ""));
            match k {
                "Type" => {
                    qif.sections.push(Section {
                        ty: v.trim().to_string(),
                        records: Vec::new(),
                    });
                    saw_section = true;
                }
                "Option" => qif.options.push(v.trim().to_string()),
                "Account" | "Clear" => {
                    qif.options.push(format!("{k}:{v}"));
                }
                _ => {
                    qif.options.push(bang.trim().to_string());
                }
            }
            continue;
        }
        if line == "^" {
            if let Some(rec) = cur.take() {
                if !saw_section {
                    // record without !Type — still legal in some exports
                    qif.sections.push(Section {
                        ty: String::new(),
                        records: Vec::new(),
                    });
                    saw_section = true;
                }
                qif.sections.last_mut()?.records.push(rec);
            }
            continue;
        }
        let tag = line.as_bytes()[0];
        if !tag.is_ascii_alphabetic() {
            return None;
        }
        let rec = cur.get_or_insert(Record {
            fields: BTreeMap::new(),
        });
        rec.fields
            .entry(tag)
            .or_default()
            .push(line.get(1..)?.trim().to_string());
    }
    if qif.sections.is_empty() {
        return None;
    }
    Some(qif)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"!Option:AutoSwitch\n!Account\nNChecking\n^\n!Clear:AutoSwitch\n!Option:AutoSwitch\n!Type:Bank\nD01/02' 4\nT-50.00\nPCoffee\nC*\n^\nD01/03' 4\nT100.00\nPRefund\n^\n";

    #[test]
    fn parses() {
        let q = parse(DOC).unwrap();
        assert!(q.options.iter().any(|o| o == "AutoSwitch"));
        let bank = q.sections.iter().find(|s| s.ty == "Bank").unwrap();
        assert_eq!(bank.records.len(), 2);
        assert_eq!(
            bank.records[0].fields.get(&b'T').map(|v| v[0].as_str()),
            Some("-50.00")
        );
        assert_eq!(
            bank.records[0].fields.get(&b'C').map(|v| v[0].as_str()),
            Some("*")
        );
    }

    #[test]
    fn rejects() {
        assert!(parse(b"hello\nworld").is_none());
        assert!(parse(b"").is_none());
    }
}
