//! SWIFT MT940 customer statement message parsing.
//!
//! An MT940 file is a sequence of `:TAG:`-prefixed fields:
//! `:20:` reference, `:25:` account, `:28C:` statement number,
//! `:60F:`/`:60M:` opening balance, repeated `:61:` statement lines
//! with optional `:86:` remittance info, `:62F:`/`:62M:` closing,
//! and `:64:`/`65:` available balances.
//!
//! ```
//! use izanagi_kit::mt940;
//! let d = b":20:REF123\n:25:ACCT1\n:60F:C240101USD1000,00\n:62F:C240102USD900,00\n";
//! let m = mt940::parse(d).unwrap();
//! assert_eq!(m.fields[0].tag, "20");
//! assert_eq!(m.fields[0].value, "REF123");
//! ```

use std::string::String;
use std::vec::Vec;

/// One `:TAG:` field; tag is 2-3 chars (`20`, `28C`, `60F`…).
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    /// Tag text without colons.
    pub tag: String,
    /// Field value (multi-line values joined with `\n`).
    pub value: String,
}

/// A parsed MT940 message.
#[derive(Clone, Debug, PartialEq)]
pub struct Mt940 {
    /// All fields in file order.
    pub fields: Vec<Field>,
    /// `:20:` transaction reference when present.
    pub reference: Option<String>,
    /// `:25:` account identification.
    pub account: Option<String>,
    /// `:61:` statement-line values.
    pub statement_lines: Vec<String>,
}

fn is_tag_at(d: &[u8], at: usize) -> Option<usize> {
    // `:XX:` or `:XXL:` — 2 digits then optional letter, closing colon
    if *d.get(at)? != b':' {
        return None;
    }
    let a = *d.get(at + 1)?;
    let b = *d.get(at + 2)?;
    if !(a.is_ascii_digit() && b.is_ascii_digit()) {
        return None;
    }
    let mut end = at + 3;
    if let Some(&c) = d.get(end) {
        if c.is_ascii_uppercase() {
            end += 1;
        }
    }
    if *d.get(end)? != b':' {
        return None;
    }
    Some(end + 1)
}

/// Parses an MT940 file: every byte must belong to a `:TAG:` field
/// (CR/LF between fields allowed); `:61:` values collected.
pub fn parse(d: &[u8]) -> Option<Mt940> {
    if !d.starts_with(b":") {
        return None;
    }
    let mut fields = Vec::new();
    let mut at = 0usize;
    while at < d.len() {
        if d[at] == b'\r' || d[at] == b'\n' {
            at += 1;
            continue;
        }
        let val_at = is_tag_at(d, at)?;
        let tag = String::from_utf8_lossy(&d[at + 1..val_at - 1]).into_owned();
        // value runs to the next `\r\n:` / `\n:` tag boundary or EOF
        let mut end = d.len();
        let mut scan = val_at;
        while scan < d.len() {
            if d[scan] == b':' {
                let back = scan == 0 || d[scan - 1] == b'\n' || d[scan - 1] == b'\r';
                if back && is_tag_at(d, scan).is_some() {
                    end = scan;
                    break;
                }
            }
            scan += 1;
        }
        let value = String::from_utf8_lossy(&d[val_at..end])
            .trim_end_matches(['\r', '\n'])
            .to_string();
        fields.push(Field { tag, value });
        at = end;
        if fields.len() > 65536 {
            return None;
        }
    }
    if fields.is_empty() {
        return None;
    }
    let get =
        |t: &str| -> Option<String> { fields.iter().find(|f| f.tag == t).map(|f| f.value.clone()) };
    let statement_lines = fields
        .iter()
        .filter(|f| f.tag == "61")
        .map(|f| f.value.clone())
        .collect();
    let reference = get("20");
    let account = get("25");
    Some(Mt940 {
        fields,
        reference,
        account,
        statement_lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b":20:STARTSUM\n:25:12345678\n:28C:00001\n:60F:C240101EUR0,00\n:61:2401010101D100,00NCHK//12345\n:86:Payment ref\n:62F:C240102EUR-100,00\n:64:C240102EUR-100,00\n";

    #[test]
    fn parses_fields() {
        let m = parse(DOC).unwrap();
        assert_eq!(m.reference.as_deref(), Some("STARTSUM"));
        assert_eq!(m.account.as_deref(), Some("12345678"));
        assert_eq!(m.fields.len(), 8);
        assert_eq!(m.statement_lines.len(), 1);
        assert!(m.statement_lines[0].starts_with("240101"));
        assert_eq!(
            m.fields.iter().find(|f| f.tag == "86").unwrap().value,
            "Payment ref"
        );
    }

    #[test]
    fn rejects() {
        assert!(parse(b"no colons").is_none());
        assert!(parse(b":XX:v\n").is_none());
        assert!(parse(b":2A:v\n").is_none());
    }
}
