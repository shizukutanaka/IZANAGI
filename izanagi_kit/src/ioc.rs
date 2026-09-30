//! OpenIOC 1.1 XML indicator file scanner (Mandiant/OpenIOC).
//!
//! An `.ioc` file is XML rooted at `<ioc>` carrying an `id` and a
//! `<definition>` tree of `<Indicator operator="AND|OR">` /
//! `<IndicatorItem …>` context+value pairs.
//!
//! ```
//! let d = br#"<?xml version="1\x2e0"?>
//! <ioc id="aaaa" xmlns:xsi="x" xmlns:xsd="y" xmlns="openioc\x2eorg/schemas/ioc">
//! <definition><Indicator operator="OR">
//! <IndicatorItem id="i1" condition="is"><Context document="FileItem" search="FileItem/Md5sum" type="mir"/><Content type="md5">deadbeef</Content></IndicatorItem>
//! </Indicator></definition></ioc>"#;
//! let i = izanagi_kit::ioc::parse(d).unwrap();
//! assert_eq!(i.indicators, 1);
//! assert_eq!(i.items, 1);
//! ```
//!
//! Reference: OpenIOC 1\x2e1 schema (openioc.org `ioc`/`definition`/
//! `Indicator`/`IndicatorItem` element set, `Context`/`Content`
//! children of `IndicatorItem`).

/// Parsed `.ioc` statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct Ioc {
    /// `id` attribute of the root `<ioc>` element.
    pub id: Option<String>,
    /// `<Indicator ` group count.
    pub indicators: usize,
    /// `<IndicatorItem` leaf count.
    pub items: usize,
    /// Distinct `Context document="…"` values seen.
    pub contexts: Vec<String>,
    /// `condition="…"` values seen (deduplicated).
    pub conditions: Vec<String>,
}

fn attr<'a>(attrs: &'a str, key: &str) -> Option<&'a str> {
    let pos = attrs.find(key)?;
    let rest = attrs[pos + key.len()..].strip_prefix('=')?;
    let q = rest.chars().next()?;
    if q != '"' && q != '\'' {
        return None;
    }
    let end = rest[1..].find(q)?;
    Some(&rest[1..end + 1])
}

fn push_unique(v: &mut Vec<String>, s: &str) {
    let s = s.to_string();
    if !v.contains(&s) {
        v.push(s);
    }
}

/// Parse an OpenIOC file; `None` unless a `<ioc` root exists.
pub fn parse(d: &[u8]) -> Option<Ioc> {
    let text = core::str::from_utf8(d).ok()?;
    let root_pos = text.find("<ioc")?;
    // `<ioc` must be followed by whitespace or `>` — not `<ioccer`.
    match text.as_bytes().get(root_pos + 4) {
        Some(c) if c.is_ascii_whitespace() || *c == b'>' => {}
        _ => return None,
    }
    let tag_end = text[root_pos..].find('>')? + root_pos;
    let tag = &text[root_pos..tag_end];
    let id = attr(tag, "id").map(|s| s.to_string());
    let mut indicators = 0usize;
    let mut items = 0usize;
    let mut contexts = Vec::new();
    let mut conditions = Vec::new();
    let mut rest = &text[tag_end..];
    while let Some(i) = rest.find('<') {
        let after = &rest[i + 1..];
        if let Some(r) = after.strip_prefix("Indicator ") {
            indicators += 1;
            rest = r;
            continue;
        }
        if let Some(r) = after.strip_prefix("IndicatorItem") {
            items += 1;
            let end = r.find('>').unwrap_or(r.len());
            let a = &r[..end];
            if let Some(c) = attr(a, "condition") {
                push_unique(&mut conditions, c);
            }
            rest = &r[end..];
            continue;
        }
        if let Some(r) = after.strip_prefix("Context ") {
            let end = r.find('>').unwrap_or(r.len());
            let a = &r[..end];
            if let Some(c) = attr(a, "document") {
                push_unique(&mut contexts, c);
            }
            rest = &r[end..];
            continue;
        }
        rest = after;
    }
    Some(Ioc {
        id,
        indicators,
        items,
        contexts,
        conditions,
    })
}

/// `true` if the buffer looks like an OpenIOC file.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = br#"<?xml version="1\x2e0"?><ioc id="aaaa" xmlns="openioc\x2eorg/schemas/ioc"><definition><Indicator operator="OR"><IndicatorItem id="i1" condition="is"><Context document="FileItem" search="FileItem/Md5sum" type="mir"/><Content type="md5">x</Content></IndicatorItem><IndicatorItem id="i2" condition="contains"><Context document="FileItem" search="FileItem/FileName" type="mir"/><Content type="string">y</Content></IndicatorItem></Indicator></definition></ioc>"#;

    #[test]
    fn parses() {
        let i = parse(DOC).unwrap();
        assert_eq!(i.id.as_deref(), Some("aaaa"));
        assert_eq!(i.indicators, 1);
        assert_eq!(i.items, 2);
        assert_eq!(i.contexts, ["FileItem"]);
        assert_eq!(i.conditions.len(), 2);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<xml/>").is_none());
        assert!(parse(&[0xff]).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"<ioccer/>"));
    }
}
