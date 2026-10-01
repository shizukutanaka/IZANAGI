//! FpML (Financial products Markup Language) trade-document census — `<FpML>`
//! root with `version=`/`type=` attributes; `trade`/`party`/`product` element
//! counts cover the trade-capture shape (`TradeConfirmation`,
//! `DataDocument`, `RequestTradeConfirmation`, …).
//!
//! ```
//! let d = b"<?xml version=\"1\x2e0\"?>\n<FpML xmlns=\"http://www\x2efpml\x2eorg/FpML-5/confirmation\" version=\"5-10\" type=\"TradeConfirmation\">\n<trade><product><interestRateSwap/></product></trade>\n<party id=\"p1\"><partyId>AAA</partyId></party>\n<party id=\"p2\"><partyId>BBB</partyId></party>\n</FpML>";
//! let f = izanagi_kit::fpml::parse(d).unwrap();
//! assert_eq!(f.version, "5-10");
//! assert_eq!(f.doc_type, "TradeConfirmation");
//! assert_eq!(f.trades, 1);
//! assert_eq!(f.parties, 2);
//! assert_eq!(f.products, 1);
//! assert!(izanagi_kit::fpml::detect(d));
//! ```

/// A censused FpML document.
pub struct Fpml {
    /// `version` attribute (`"5-10"` style).
    pub version: String,
    /// `type` attribute (`TradeConfirmation`, `DataDocument`, …).
    pub doc_type: String,
    /// `<trade>` element count.
    pub trades: u32,
    /// `<party>` element count (`<partyId>` is excluded via boundary check).
    pub parties: u32,
    /// `<product>` element count.
    pub products: u32,
    /// `<calculation>` / `<novation>` / `<portfolio>` element count.
    pub other_blocks: u32,
}

fn count(s: &str, pat: &str) -> u32 {
    u32::try_from(s.matches(pat).count()).unwrap_or(u32::MAX)
}

fn open_tag_count(s: &str, name: &str) -> u32 {
    // `<party ` or `<party>` but not `<partyId>`
    count(s, &format!("<{name} "))
        + count(s, &format!("<{name}>"))
        + count(s, &format!("<{name}/>"))
}

fn attr<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    let i = s.find(&format!("{name}="))? + name.len() + 1;
    let rest = &s[i..];
    let q = rest.as_bytes().first().copied()?;
    if q != b'"' && q != b'\'' {
        return None;
    }
    let rest = &rest[1..];
    let j = rest.find(q as char)?;
    Some(&rest[..j])
}

/// `<FpML` root element.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.contains("<FpML")
}

/// Parses the document; `None` without an `<FpML` root.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Fpml> {
    let s = core::str::from_utf8(b).ok()?;
    let root = s.find("<FpML")?;
    let head = s.get(root..root + 512).unwrap_or(&s[root..]);
    Some(Fpml {
        version: attr(head, "version").unwrap_or("").to_string(),
        doc_type: attr(head, "type").unwrap_or("").to_string(),
        trades: open_tag_count(s, "trade"),
        parties: open_tag_count(s, "party"),
        products: open_tag_count(s, "product"),
        other_blocks: open_tag_count(s, "calculation")
            + open_tag_count(s, "novation")
            + open_tag_count(s, "portfolio"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] = b"<FpML xmlns=\"http://www\x2efpml\x2eorg/FpML-5/confirmation\" version=\"5-10\" type=\"TradeConfirmation\"><trade><product><interestRateSwap/></product></trade><party id=\"p1\"/><party id=\"p2\"/></FpML>";

    #[test]
    fn detect_works() {
        assert!(detect(FIXTURE));
        assert!(!detect(b"<fpml/>"));
        assert!(!detect(b"trade"));
    }

    #[test]
    fn parses() {
        let f = parse(FIXTURE).unwrap();
        assert_eq!(f.version, "5-10");
        assert_eq!(f.doc_type, "TradeConfirmation");
        assert_eq!(f.trades, 1);
        assert_eq!(f.parties, 2);
        assert_eq!(f.products, 1);
        assert_eq!(f.other_blocks, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<xml/>").is_none());
    }
}
