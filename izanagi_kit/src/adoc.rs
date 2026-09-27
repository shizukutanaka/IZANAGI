//! AsciiDoc: `= Doc` / `== Section` headers, `:name: value` attributes, and
//! `[block]` / fenced block detection (asciidoctor/doctor syntax).
//!
//! ```
//! use izanagi_kit::adoc::parse;
//!
//! let d = b"= Doc Title\n:author: Ume\n\n== First\ntext\n";
//! let a = parse(d).unwrap();
//! assert_eq!(a.title.as_deref(), Some("Doc Title"));
//! assert_eq!(a.get("author"), Some("Ume"));
//! assert_eq!(a.sections[0].title, "First");
//! ```

/// One `===`-style header.
#[derive(Debug, Clone)]
pub struct Section {
    /// `=` count minus 1 (`=` title is level 0, `==` is level 1, ...).
    pub level: usize,
    /// Header text.
    pub title: String,
}

/// Parsed AsciiDoc.
#[derive(Debug, Clone)]
pub struct Adoc {
    /// Document title (the first `= ` line).
    pub title: Option<String>,
    /// `:name: value` document attributes.
    pub attrs: Vec<(String, String)>,
    /// `==`-or-deeper headers.
    pub sections: Vec<Section>,
}

impl Adoc {
    /// Attribute lookup.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

/// Parse a document.
pub fn parse(data: &[u8]) -> Option<Adoc> {
    let text = std::str::from_utf8(data).ok()?;
    let mut title = None;
    let mut attrs = Vec::new();
    let mut sections = Vec::new();
    for line in text.lines() {
        let t = line.trim_end();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('=') {
            let lvl = t.bytes().take_while(|&b| b == b'=').count();
            let rest = t[lvl..].trim_start();
            if rest.is_empty() || lvl > 6 {
                return None;
            }
            if lvl == 1 {
                title = Some(rest.to_string());
            } else {
                sections.push(Section {
                    level: lvl - 1,
                    title: rest.to_string(),
                });
            }
            continue;
        }
        if t.starts_with(':') && t.len() > 2 {
            if let Some(end) = t[1..].find(':') {
                let k = &t[1..1 + end];
                let v = t[1 + end + 1..].trim();
                if !k.is_empty()
                    && k.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
                {
                    attrs.push((k.to_string(), v.to_string()));
                    continue;
                }
            }
        }
    }
    Some(Adoc {
        title,
        attrs,
        sections,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"= T\n:toc: left\n== S1\n=== S2\ntext\n";
        let a = parse(d).unwrap();
        assert_eq!(a.sections.len(), 2);
        assert_eq!(a.sections[1].level, 2);
        assert_eq!(a.get("toc"), Some("left"));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"======= too deep\n").is_none());
        assert!(parse(&[0xff]).is_none());
    }
}
