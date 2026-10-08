//! ISO 20022 `camt.*` account-reporting messages — `camt.052` intraday report,
//! `camt.053` bank-to-customer statement, `camt.054` debit/credit notification.
//! All share a `<Document>` root with a `urn:iso:std:iso:20022:tech:xsd:camt.*`
//! namespace and a `BkToCstmr*` payload element containing `Stmt`/`Rpt`/
//! `Ntfctn` blocks of `Ntry` entries carrying `Ccy=`-tagged `Amt` amounts.
//!
//! ```
//! let d = b"<Document xmlns=\"urn:iso:std:iso:20022:tech:xsd:camt\x2e053\x2e001\x2e02\"><BkToCstmrStmt><GrpHdr/><Stmt><Id>S1</Id><Ntry><Amt Ccy=\"EUR\">10</Amt></Ntry><Ntry><Amt Ccy=\"EUR\">20</Amt></Ntry></Stmt></BkToCstmrStmt></Document>";
//! let c = izanagi_kit::camt::parse(d).unwrap();
//! assert_eq!(c.camt_code, "053");
//! assert_eq!(c.statements, 1);
//! assert_eq!(c.entries, 2);
//! assert_eq!(c.currency.as_deref(), Some("EUR"));
//! assert!(izanagi_kit::camt::detect(d));
//! ```

/// A censused ISO 20022 camt message.
#[derive(Debug)]
pub struct Camt {
    /// 3-digit message suffix from the namespace (`052`/`053`/`054`/`060`…).
    pub camt_code: String,
    /// `Stmt` + `Rpt` + `Ntfctn` block count.
    pub statements: u32,
    /// `Ntry` entry count.
    pub entries: u32,
    /// First `Ccy="XXX"` currency attribute.
    pub currency: Option<String>,
    /// First `<IBAN>` element text, when present.
    pub iban: Option<String>,
}

fn count(s: &str, pat: &str) -> u32 {
    u32::try_from(s.matches(pat).count()).unwrap_or(u32::MAX)
}

fn open_tag_count(s: &str, name: &str) -> u32 {
    count(s, &format!("<{name} "))
        + count(s, &format!("<{name}>"))
        + count(s, &format!("<{name}/>"))
}

fn text_of<'a>(s: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let i = s.find(&open)? + open.len();
    let j = s[i..].find(&format!("</{tag}>"))? + i;
    Some(&s[i..j])
}

fn currency(s: &str) -> Option<String> {
    let i = s.find("Ccy=\"")? + 5;
    let j = s[i..].find('"')? + i;
    Some(s[i..j].to_string())
}

fn ns_code(s: &str) -> Option<String> {
    let i = s.find("camt.")? + 5;
    if s[i..].len() >= 3 && s.as_bytes()[i..i + 3].iter().all(u8::is_ascii_digit) {
        Some(s[i..i + 3].to_string())
    } else {
        None
    }
}

/// `BkToCstmr*` payload element or `camt.` namespace.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.contains("BkToCstmr") || s.contains("camt.")
}

/// Parses the message; `None` without a camt marker.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Camt> {
    let s = core::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    Some(Camt {
        camt_code: ns_code(s).unwrap_or_default(),
        statements: open_tag_count(s, "Stmt")
            + open_tag_count(s, "Rpt")
            + open_tag_count(s, "Ntfctn"),
        entries: open_tag_count(s, "Ntry"),
        currency: currency(s),
        iban: text_of(s, "IBAN").map(str::to_string),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] = b"<Document xmlns=\"urn:iso:std:iso:20022:tech:xsd:camt\x2e053\x2e001\x2e02\"><BkToCstmrStmt><GrpHdr/><Stmt><Id>S1</Id><Ntry><Amt Ccy=\"EUR\">10</Amt></Ntry><Ntry><Amt Ccy=\"EUR\">20</Amt></Ntry></Stmt></BkToCstmrStmt></Document>";

    #[test]
    fn detect_works() {
        assert!(detect(FIXTURE));
        assert!(!detect(b"<document/>"));
    }

    #[test]
    fn parses() {
        let c = parse(FIXTURE).unwrap();
        assert_eq!(c.camt_code, "053");
        assert_eq!(c.statements, 1);
        assert_eq!(c.entries, 2);
        assert_eq!(c.currency.as_deref(), Some("EUR"));
        assert_eq!(c.iban, None);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"camt 053 report").is_none());
    }

    #[test]
    fn handles_multibyte_namespace_code() {
        let _ = parse("camt.\u{1d11e}".as_bytes());
    }
}
