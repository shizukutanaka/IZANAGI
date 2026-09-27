//! Minimal reader for MODS (Metadata Object Description Schema, Library of
//! Congress): `<mods>` records with `<titleInfo><title>`,
//! `<name type="personal"><namePart>` creators, `<identifier type="...">`,
//! and `<originInfo><dateIssued>`. Self-contained tag scanning.
//!
//! ```
//! use izanagi_kit::mods::parse;
//!
//! let m = parse(
//!     b"<mods><titleInfo><title>Catalog</title><subTitle>A</subTitle></titleInfo>\
//!        <name type=\"personal\"><namePart>Doe, J.</namePart></name>\
//!        <identifier type=\"isbn\">978-0</identifier>\
//!        <originInfo><dateIssued>2003</dateIssued></originInfo></mods>",
//! )
//! .unwrap();
//! assert_eq!(m.title.as_deref(), Some("Catalog"));
//! assert_eq!(m.names, vec!["Doe, J."]);
//! assert_eq!(m.identifier("isbn").unwrap(), "978-0");
//! assert_eq!(m.date_issued.as_deref(), Some("2003"));
//! ```

/// A parsed `<mods>` record.
#[derive(Debug, Default)]
pub struct Mods {
    /// `<titleInfo><title>` (subtitle is kept separate below).
    pub title: Option<String>,
    /// `<titleInfo><subTitle>`.
    pub subtitle: Option<String>,
    /// `<name><namePart>` texts (creators, in order).
    pub names: Vec<String>,
    /// `<identifier type="X">v</identifier>` pairs.
    pub identifiers: Vec<(String, String)>,
    /// `<originInfo><dateIssued>` verbatim (may be a range).
    pub date_issued: Option<String>,
}

impl Mods {
    /// First `<identifier>` of `type` (`isbn`, `issn`, `doi`, `lccn`, ...).
    pub fn identifier(&self, kind: &str) -> Option<&str> {
        self.identifiers
            .iter()
            .find(|(k, _)| k == kind)
            .map(|(_, v)| v.as_str())
    }
}

fn text_between<'a>(src: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let a = src.find(open)? + open.len();
    let b = src[a..].find(close)? + a;
    Some(&src[a..b])
}

fn attr(tag: &str, key: &str) -> Option<String> {
    let pat = format!("{key}=\"");
    let at = tag.find(&pat)? + pat.len();
    let end = tag[at..].find('"')? + at;
    Some(tag[at..end].to_string())
}

/// Scan `open`-prefixed elements with a `type` attribute; yields
/// `(type, inner_text)` for each `<open type="x">inner</close>`-shaped node.
fn typed_elements(src: &str, open: &str, close: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(a) = rest.find(open) {
        let tag_end = match rest[a..].find('>') {
            Some(b) => a + b + 1,
            None => break,
        };
        let kind = match attr(&rest[a..tag_end], "type") {
            Some(k) => k,
            None => {
                rest = &rest[tag_end..];
                continue;
            }
        };
        let close_at = match rest[tag_end..].find(close) {
            Some(c) => tag_end + c,
            None => break,
        };
        out.push((kind, rest[tag_end..close_at].trim().to_string()));
        rest = &rest[close_at + close.len()..];
    }
    out
}

/// Parse a `<mods>` record. `None` without a `<mods>` root element.
pub fn parse(data: &[u8]) -> Option<Mods> {
    let src = std::str::from_utf8(data).ok()?;
    // locate the root (accept `<mods>` or `<mods ...>`)
    let start = src.find("<mods")?;
    let mut m = Mods::default();
    if let Some(ti) = text_between(&src[start..], "<titleInfo>", "</titleInfo>") {
        m.title = text_between(ti, "<title>", "</title>")
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty());
        m.subtitle = text_between(ti, "<subTitle>", "</subTitle>")
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty());
    }
    let mut rest = &src[start..];
    while let Some(a) = rest.find("<name") {
        // skip `<namePart>`/`<nameTitle>` prefixes: `<name` + non-alpha
        let nb = rest.as_bytes().get(a + 5).copied();
        if matches!(nb, Some(b'P') | Some(b'T')) || nb == Some(b's') {
            rest = &rest[a + 5..];
            continue;
        }
        let body = match text_between(&rest[a..], "", "</name>") {
            Some(b) => b,
            None => break,
        };
        // `<name ...>` header is inside `body`; find namePart inside
        if let Some(np) = text_between(body, "<namePart>", "</namePart>") {
            m.names.push(np.trim().to_string());
        }
        rest = &rest[a + body.len() + "</name>".len()..];
    }
    m.identifiers = typed_elements(&src[start..], "<identifier", "</identifier>");
    if let Some(oi) = text_between(&src[start..], "<originInfo>", "</originInfo>") {
        m.date_issued = text_between(oi, "<dateIssued>", "</dateIssued>")
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty());
    }
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"<mods xmlns=\"http://www.loc.gov/mods/v3\">\
<titleInfo><title>Title</title><subTitle>Sub</subTitle></titleInfo>\
<name type=\"personal\"><namePart>Knuth, D.</namePart></name>\
<name type=\"personal\"><namePart>Knuth, E.</namePart></name>\
<identifier type=\"isbn\">0-201</identifier><identifier type=\"doi\">10/x</identifier>\
<originInfo><dateIssued>1968</dateIssued></originInfo></mods>";

    #[test]
    fn parses() {
        let m = parse(DOC).unwrap();
        assert_eq!(m.title.as_deref(), Some("Title"));
        assert_eq!(m.subtitle.as_deref(), Some("Sub"));
        assert_eq!(m.names, vec!["Knuth, D.", "Knuth, E."]);
        assert_eq!(m.identifier("isbn"), Some("0-201"));
        assert_eq!(m.identifier("doi"), Some("10/x"));
        assert_eq!(m.identifier("lccn"), None);
        assert_eq!(m.date_issued.as_deref(), Some("1968"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<notmods/>").is_none());
        // bare minimum
        let m = parse(b"<mods/>").unwrap();
        assert_eq!(m.title, None);
        assert!(m.names.is_empty());
    }
}
