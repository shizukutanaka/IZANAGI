//! OAI-PMH 2.0 (Open Archives Initiative Protocol for Metadata
//! Harvesting): `<OAI-PMH>` root with `<responseDate>` + `<request
//! verb="…">` + `<ListRecords>`/`<GetRecord>`/`<Identify>` payloads
//! and `<record>`/`<header>`/`identifier`/`datestamp` members, or an
//! `<error code="…">` reply.
//!
//! ```
//! use izanagi_kit::oai::{detect, parse};
//!
//! let d = b"<OAI-PMH><responseDate>t</responseDate>\
//! <request verb=\"ListRecords\"/>\
//! <ListRecords><record><header><identifier>a</identifier></header></record></ListRecords>\
//! </OAI-PMH>";
//! assert!(detect(d));
//! let o = parse(d).unwrap();
//! assert_eq!(o.verb.as_deref(), Some("ListRecords"));
//! assert_eq!(o.records, 1);
//! ```

fn elem_count(s: &str, name: &str) -> u32 {
    let mut n = 0u32;
    let mut i = 0usize;
    while let Some(off) = s[i..].find(&format!("<{name}")) {
        let p = i + off + name.len() + 1;
        match s[p..].chars().next() {
            None | Some(' ' | '>' | '/' | '\t' | '\n') => n += 1,
            _ => {}
        }
        i = p;
        if i >= s.len() {
            break;
        }
    }
    n
}

fn attr(s: &str, name: &str) -> Option<String> {
    let i = s.find(&format!("{name}=\""))? + name.len() + 2;
    s[i..].find('"').map(|e| s[i..i + e].to_string())
}

/// Parsed OAI-PMH response census.
#[derive(Debug, Clone, PartialEq)]
pub struct Oai {
    /// `verb` attribute of `<request>`.
    pub verb: Option<String>,
    /// `<request>` request URI text, when non-empty.
    pub request_uri: Option<String>,
    /// `<responseDate>` value.
    pub response_date: Option<String>,
    /// `<record>` elements.
    pub records: u32,
    /// `<header>` record headers.
    pub headers: u32,
    /// `<identifier>` elements.
    pub identifiers: u32,
    /// `<metadata>` blocks.
    pub metadata_blocks: u32,
    /// `<setSpec>` set memberships.
    pub set_specs: u32,
    /// `<resumptionToken>` seen.
    pub has_resumption_token: bool,
    /// `<error code="…">` code, when present.
    pub error_code: Option<String>,
    /// `<about>` containers.
    pub abouts: u32,
}

fn tag_text(s: &str, tag: &str) -> Option<String> {
    let i = s.find(&format!("<{tag}>"))? + tag.len() + 2;
    s[i..].find('<').map(|e| s[i..i + e].to_string())
}

/// `true` on `<OAI-PMH` + `responseDate`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.contains("<OAI-PMH") && s.contains("responseDate")
}

/// Census; `None` without the root marker.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Oai> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    Some(Oai {
        verb: attr(s, "verb"),
        request_uri: s
            .find("<request")
            .and_then(|i| s[i..].find('>').map(|e| i + e + 1))
            .and_then(|start| {
                s[start..].find('<').and_then(|e| {
                    let t = s[start..start + e].trim();
                    if t.is_empty() {
                        None
                    } else {
                        Some(t.to_string())
                    }
                })
            }),
        response_date: tag_text(s, "responseDate"),
        records: elem_count(s, "record"),
        headers: elem_count(s, "header"),
        identifiers: elem_count(s, "identifier"),
        metadata_blocks: elem_count(s, "metadata"),
        set_specs: elem_count(s, "setSpec"),
        has_resumption_token: s.contains("resumptionToken"),
        error_code: s.find("<error ").and_then(|i| attr(&s[i..], "code")),
        abouts: elem_count(s, "about"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"<OAI-PMH><responseDate>2024-01-01T00:00:00Z</responseDate>\
<request verb=\"ListRecords\">https://example.org/oai</request>\
<ListRecords><record><header><identifier>oai:a</identifier><datestamp>d</datestamp>\
<setSpec>s</setSpec></header><metadata/></record><resumptionToken>t</resumptionToken></ListRecords>\
</OAI-PMH>";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"<OAI-PMH>"));
    }

    #[test]
    fn parses() {
        let o = parse(D).unwrap();
        assert_eq!(o.verb.as_deref(), Some("ListRecords"));
        assert_eq!(o.request_uri.as_deref(), Some("https://example.org/oai"));
        assert_eq!(o.response_date.as_deref(), Some("2024-01-01T00:00:00Z"));
        assert_eq!(o.records, 1);
        assert_eq!(o.headers, 1);
        assert_eq!(o.identifiers, 1);
        assert_eq!(o.metadata_blocks, 1);
        assert_eq!(o.set_specs, 1);
        assert!(o.has_resumption_token);
        assert_eq!(o.error_code, None);
        assert_eq!(o.abouts, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}
