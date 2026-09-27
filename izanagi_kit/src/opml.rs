//! OPML outlines — `<opml version="…"><head><title/>…</head><body>`
//! plus nested `<outline>` elements with `text`/`type`/`xmlUrl`.
//!
//! ```
//! use izanagi_kit::opml::parse;
//!
//! let x = br#"<opml version="1.0"><head><title>Subs</title></head>
//!   <body><outline text="Tech"><outline text="Blog" type="rss" xmlUrl="http://x/feed"/></outline>
//!   </body></opml>"#;
//! let o = parse(x).unwrap();
//! assert_eq!(o.title.as_deref(), Some("Subs"));
//! assert_eq!(o.outlines[0].depth, 0);
//! ```

use std::string::String;
use std::vec::Vec;

/// One `<outline>` element.
#[derive(Clone, Debug)]
pub struct Outline {
    /// `text` (or `title`) attribute.
    pub text: String,
    /// `type` attribute (`rss`, `link`, …).
    pub kind: Option<String>,
    /// `xmlUrl` when present.
    pub xml_url: Option<String>,
    /// `url` when present.
    pub url: Option<String>,
    /// Nesting depth under `<body>`.
    pub depth: usize,
}

/// Parsed OPML document.
#[derive(Clone, Debug)]
pub struct Opml {
    /// `<title>` text in `<head>`.
    pub title: Option<String>,
    /// All outlines in document order.
    pub outlines: Vec<Outline>,
}

fn attr(tag: &[u8], name: &[u8]) -> Option<String> {
    let mut pat = Vec::new();
    pat.extend_from_slice(name);
    pat.push(b'=');
    pat.push(b'"');
    let at = tag
        .windows(pat.len())
        .position(|w| w == pat.as_slice())
        .or_else(|| {
            let mut p2 = Vec::new();
            p2.extend_from_slice(name);
            p2.push(b'=');
            p2.push(b'\'');
            tag.windows(p2.len()).position(|w| w == p2.as_slice())
        })?;
    let q = tag[at + name.len() + 1];
    let v0 = at + name.len() + 2;
    let e = tag[v0..].iter().position(|c| *c == q)? + v0;
    Some(String::from_utf8_lossy(&tag[v0..e]).into_owned())
}

/// Parse an OPML document.
pub fn parse(d: &[u8]) -> Option<Opml> {
    if !d.windows(5).any(|w| w == b"<opml") {
        return None;
    }
    let mut title = None;
    if let Some(t0) = d.windows(7).position(|w| w == b"<title>") {
        if let Some(e) = d[t0..].windows(8).position(|w| w == b"</title>") {
            title = Some(
                String::from_utf8_lossy(&d[t0 + 7..t0 + e])
                    .trim()
                    .to_string(),
            );
        }
    }
    let mut outlines = Vec::new();
    let mut depth = 0usize;
    let mut i = 0usize;
    while i + 8 < d.len() {
        if d[i..].starts_with(b"<outline") {
            let gt = d[i..].iter().position(|c| *c == b'>')? + i;
            let tag = &d[i..gt + 1];
            let self_close = gt > i && d[gt - 1] == b'/';
            outlines.push(Outline {
                text: attr(tag, b"text")
                    .or_else(|| attr(tag, b"title"))
                    .unwrap_or_default(),
                kind: attr(tag, b"type"),
                xml_url: attr(tag, b"xmlUrl"),
                url: attr(tag, b"url"),
                depth,
            });
            if !self_close {
                depth += 1;
            }
            i = gt + 1;
        } else if d[i..].starts_with(b"</outline") {
            depth = depth.saturating_sub(1);
            i += 9;
        } else {
            i += 1;
        }
    }
    if outlines.is_empty() && title.is_none() {
        return None;
    }
    Some(Opml { title, outlines })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested() {
        let x = br#"<opml version="1.0"><head><title>S</title></head><body>
            <outline text="A"><outline text="B" type="rss" xmlUrl="u"/><outline text="C"/></outline>
            <outline text="D"/></body></opml>"#;
        let o = parse(x).unwrap();
        assert_eq!(o.outlines.len(), 4);
        assert_eq!(o.outlines[0].depth, 0);
        assert_eq!(o.outlines[1].depth, 1);
        assert_eq!(o.outlines[1].xml_url.as_deref(), Some("u"));
        assert_eq!(o.outlines[3].depth, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<html/>").is_none());
        assert!(parse(b"<opml><body/></opml>").is_none());
    }
}
