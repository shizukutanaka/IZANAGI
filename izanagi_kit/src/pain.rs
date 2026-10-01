//! ISO 20022 `pain.*` payment-initiation messages — `pain.001` customer credit
//! transfer initiation, `pain.002` payment status report, `pain.007` reversal,
//! `pain.008` direct debit initiation. `<Document>` + `Cstmr*` payload with
//! `PmtInf` payment-information blocks and `CdtTrfTxInf`/`DrctDbtTxInf`
//! transaction rows; `NbOfTxs` holds the declared transaction count.
//!
//! ```
//! let d = b"<Document xmlns=\"urn:iso:std:iso:20022:tech:xsd:pain\x2e001\x2e001\x2e03\"><CstmrCdtTrfInitn><GrpHdr><NbOfTxs>2</NbOfTxs></GrpHdr><PmtInf><CdtTrfTxInf/><CdtTrfTxInf/></PmtInf><PmtInf/></CstmrCdtTrfInitn></Document>";
//! let p = izanagi_kit::pain::parse(d).unwrap();
//! assert_eq!(p.pain_code, "001");
//! assert_eq!(p.payment_infos, 2);
//! assert_eq!(p.transactions, 2);
//! assert_eq!(p.declared_txs, Some(2));
//! assert!(izanagi_kit::pain::detect(d));
//! ```

/// A censused ISO 20022 pain message.
pub struct Pain {
    /// 3-digit message suffix from the namespace (`001`/`002`/`007`/`008`…).
    pub pain_code: String,
    /// `PmtInf` payment-information block count.
    pub payment_infos: u32,
    /// `CdtTrfTxInf` + `DrctDbtTxInf` transaction element count.
    pub transactions: u32,
    /// `<NbOfTxs>` declared count, when present and numeric.
    pub declared_txs: Option<u64>,
    /// `NbOfTxs` vs. counted transactions agree.
    pub counts_match: bool,
    /// First `Ccy="XXX"` currency attribute.
    pub currency: Option<String>,
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
    let i = s.find("pain.")? + 5;
    if s[i..].len() >= 3 && s.as_bytes()[i..i + 3].iter().all(u8::is_ascii_digit) {
        Some(s[i..i + 3].to_string())
    } else {
        None
    }
}

/// `Cstmr*Initn`/`Cstmr*StsRpt` payload element or `pain.` namespace.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.contains("CstmrCdtTrfInitn")
        || s.contains("CstmrPmtStsRpt")
        || s.contains("CstmrDrctDbtInitn")
        || s.contains("CstmrPmtCxl")
        || s.contains("pain.")
}

/// Parses the message; `None` without a pain marker.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Pain> {
    let s = core::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    let transactions = open_tag_count(s, "CdtTrfTxInf") + open_tag_count(s, "DrctDbtTxInf");
    let declared_txs = text_of(s, "NbOfTxs").and_then(|t| t.trim().parse().ok());
    Some(Pain {
        pain_code: ns_code(s).unwrap_or_default(),
        payment_infos: open_tag_count(s, "PmtInf"),
        transactions,
        declared_txs,
        counts_match: declared_txs.is_some_and(|d| d == u64::from(transactions)),
        currency: currency(s),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] = b"<Document xmlns=\"urn:iso:std:iso:20022:tech:xsd:pain\x2e001\x2e001\x2e03\"><CstmrCdtTrfInitn><GrpHdr><NbOfTxs>2</NbOfTxs></GrpHdr><PmtInf><CdtTrfTxInf/><CdtTrfTxInf/></PmtInf><PmtInf/></CstmrCdtTrfInitn></Document>";

    #[test]
    fn detect_works() {
        assert!(detect(FIXTURE));
        assert!(!detect(b"<document/>"));
    }

    #[test]
    fn parses() {
        let p = parse(FIXTURE).unwrap();
        assert_eq!(p.pain_code, "001");
        assert_eq!(p.payment_infos, 2);
        assert_eq!(p.transactions, 2);
        assert_eq!(p.declared_txs, Some(2));
        assert!(p.counts_match);
        assert_eq!(p.currency, None);
    }

    #[test]
    fn mismatch_flags() {
        let d = b"<CstmrCdtTrfInitn><NbOfTxs>9</NbOfTxs><PmtInf><CdtTrfTxInf/></PmtInf></CstmrCdtTrfInitn>";
        let p = parse(d).unwrap();
        assert!(!p.counts_match);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"pain 001").is_none());
    }

    #[test]
    fn handles_multibyte_namespace_code() {
        let _ = parse("pain.\u{1d11e}".as_bytes());
    }
}
