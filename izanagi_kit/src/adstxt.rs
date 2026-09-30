//! ads.txt / app-ads.txt (IAB Tech Lab spec v1.x): lines are
//! `# comments`, `VARIABLE=VALUE` variable declarations
//! (`CONTACT=`, `SUBDOMAIN=`, `INVENTORYPARTNERDOMAIN=`, …), or
//! data records `domain, publisherId, DIRECT|RESELLER[, certId]`.
//!
//! ```
//! let s = "# ads.txt\ngreenadexchange.com, PUB-1, DIRECT, f208\
//!          \nCONTACT=adops@e\x2ex\n";
//! let d = izanagi_kit::adstxt::parse(s).unwrap();
//! assert_eq!(d.records.len(), 1);
//! assert_eq!(d.records[0].relation, izanagi_kit::adstxt::Relation::Direct);
//! ```

use std::string::{String, ToString};
use std::vec::Vec;

/// Publisher-account relationship declared by a record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Relation {
    /// `DIRECT` — the publisher owns the account.
    Direct,
    /// `RESELLER` — the publisher authorized a reseller.
    Reseller,
    /// Any other / unrecognized token (kept verbatim in `raw`).
    Other,
}

/// One ads.txt data record.
#[derive(Clone, Debug)]
pub struct Record {
    /// Advertising-system domain (field 1).
    pub domain: String,
    /// Publisher account id (field 2).
    pub publisher_id: String,
    /// `DIRECT` / `RESELLER` / other (field 3).
    pub relation: Relation,
    /// TAGID certification id (optional field 4).
    pub cert_id: Option<String>,
    /// The raw line, trimmed.
    pub raw: String,
}

/// A parsed ads.txt file.
#[derive(Clone, Debug)]
pub struct AdsTxt {
    /// Data records in order.
    pub records: Vec<Record>,
    /// `KEY=VALUE` variable declarations in order.
    pub variables: Vec<(String, String)>,
}

/// Parse ads.txt content; `None` when neither a valid record nor a
/// variable line exists (a pure comment file is not an ads.txt).
pub fn parse(s: &str) -> Option<AdsTxt> {
    let mut records = Vec::new();
    let mut variables = Vec::new();
    for line in s.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // A '#' may also start a trailing comment.
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.contains('=') && !line.contains(',') {
            if let Some((k, v)) = line.split_once('=') {
                let k = k.trim();
                if !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    variables.push((k.to_string(), v.trim().to_string()));
                    continue;
                }
            }
        }
        let parts: Vec<&str> = line.split(',').map(|p| p.trim()).collect();
        if parts.len() < 3 || parts[..3].iter().any(|p| p.is_empty()) {
            continue; // malformed record — skip, don't fail the file
        }
        let relation = match parts[2] {
            "DIRECT" | "direct" => Relation::Direct,
            "RESELLER" | "reseller" => Relation::Reseller,
            _ => Relation::Other,
        };
        records.push(Record {
            domain: parts[0].to_string(),
            publisher_id: parts[1].to_string(),
            relation,
            cert_id: parts
                .get(3)
                .filter(|c| !c.is_empty())
                .map(|c| (*c).to_string()),
            raw: line.to_string(),
        });
    }
    if records.is_empty() && variables.is_empty() {
        return None;
    }
    Some(AdsTxt { records, variables })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_vars() {
        let s = "greenadexchange.com, PUB-1, DIRECT, f208\n\
                 ssp.example.com, 42, RESELLER # trailing\n\
                 bad.example, x\n\
                 CONTACT=ops@e\x2ex\nSUBDOMAIN=www\n";
        let d = parse(s).unwrap();
        assert_eq!(d.records.len(), 2);
        assert_eq!(d.records[0].domain, "greenadexchange.com");
        assert_eq!(d.records[0].cert_id.as_deref(), Some("f208"));
        assert_eq!(d.records[1].relation, Relation::Reseller);
        assert!(d.records[1].cert_id.is_none());
        assert_eq!(d.variables.len(), 2);
    }

    #[test]
    fn comments_only_reject() {
        assert!(parse("# nothing\n\n").is_none());
        assert!(parse("").is_none());
        // malformed lines alone don't make a file
        assert!(parse("a, b\n").is_none());
    }
}
