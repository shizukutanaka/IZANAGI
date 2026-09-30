//! Document Type Definition `.dtd` — `<!ELEMENT`/`<!ATTLIST`/
//! `<!ENTITY`/`<!NOTATION` declarations, content models like
//! `(#PCDATA)`/`EMPTY`/`ANY`, parameter entities `%name;`, and
//! `PUBLIC`/`SYSTEM` external IDs.
//!
//! ```
//! let d = b"<!ELEMENT a (b,c)>\n<!ATTLIST a k CDATA #IMPLIED>\n\
//! <!ENTITY e \"v\">\n<!NOTATION n SYSTEM \"s\">\n";
//! let x = izanagi_kit::dtd::parse(d).unwrap();
//! assert_eq!(x.elements, 1);
//! assert_eq!(x.attlists, 1);
//! assert_eq!(x.entities, 1);
//! assert_eq!(x.notations, 1);
//! assert!(izanagi_kit::dtd::detect(d));
//! ```

/// Census of a `.dtd`.
#[derive(Debug, Clone)]
pub struct Dtd {
    /// `<!ELEMENT` declarations.
    pub elements: usize,
    /// `<!ATTLIST` declarations.
    pub attlists: usize,
    /// `<!ENTITY` declarations (all).
    pub entities: usize,
    /// `<!ENTITY %` parameter entities.
    pub param_entities: usize,
    /// `<!NOTATION` declarations.
    pub notations: usize,
    /// `<!DOCTYPE` occurrences.
    pub doctypes: usize,
    /// `<!COMMENT`-style `<!--` comment starts.
    pub comments: usize,
    /// `PUBLIC` keywords.
    pub publics: usize,
    /// `SYSTEM` keywords.
    pub systems: usize,
    /// `EMPTY`/`ANY` content keywords.
    pub content_keywords: usize,
    /// `%name;` parameter-entity references.
    pub pe_refs: usize,
    /// `#IMPLIED`/`#REQUIRED`/`#FIXED`/`#PCDATA` keywords.
    pub attr_keywords: usize,
    /// Conditional `<![INCLUDE[`/`<![IGNORE[` sections.
    pub conditionals: usize,
}

fn count_kw(t: &str, kw: &str) -> usize {
    t.matches(kw).count()
}

/// Detects a `.dtd`: at least one `<!…` declaration keyword.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("<!ELEMENT")
        || t.contains("<!ATTLIST")
        || t.contains("<!ENTITY")
        || t.contains("<!NOTATION")
        || t.contains("<!DOCTYPE")
}

/// Parses a `.dtd`; `None` on non-UTF-8 or no declarations.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Dtd> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    let entities = count_kw(t, "<!ENTITY");
    let mut param_entities = 0;
    let mut pe_refs = 0;
    let mut conditionals = 0;
    for raw in t.lines() {
        let l = raw.trim();
        if l.starts_with("<!ENTITY %") {
            param_entities += 1;
        }
        if l.starts_with("<![") {
            conditionals += 1;
        }
    }
    // `%name;` references outside of `<!ENTITY %` declarations.
    let bytes = t.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let rest = &bytes[i + 1..];
            let name_len = rest
                .iter()
                .take_while(|c| c.is_ascii_alphanumeric() || **c == b'.' || **c == b'-')
                .count();
            if name_len > 0 && rest.get(name_len) == Some(&b';') {
                pe_refs += 1;
            }
            i += 1 + name_len.max(1);
        } else {
            i += 1;
        }
    }
    Some(Dtd {
        elements: count_kw(t, "<!ELEMENT"),
        attlists: count_kw(t, "<!ATTLIST"),
        entities,
        param_entities,
        notations: count_kw(t, "<!NOTATION"),
        doctypes: count_kw(t, "<!DOCTYPE"),
        comments: count_kw(t, "<!--"),
        publics: count_kw(t, "PUBLIC"),
        systems: count_kw(t, "SYSTEM"),
        content_keywords: count_kw(t, "EMPTY") + count_kw(t, "ANY"),
        pe_refs,
        attr_keywords: count_kw(t, "#IMPLIED")
            + count_kw(t, "#REQUIRED")
            + count_kw(t, "#FIXED")
            + count_kw(t, "#PCDATA"),
        conditionals,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"<!ELEMENT a (b,c)>\n<!ATTLIST a k CDATA #IMPLIED>\n<!ENTITY e \"v\">\n<!ENTITY % pe \"w\">\n%pe;\n<!NOTATION n SYSTEM \"s\">\n<!-- hi -->\n<![INCLUDE[<!ELEMENT i ANY>]]>\n<!DOCTYPE r PUBLIC \"-//x//y\" \"z\" [\n<!ELEMENT r EMPTY>\n]>\n";

    #[test]
    fn parses() {
        let x = parse(D).unwrap();
        assert_eq!(x.elements, 3);
        assert_eq!(x.attlists, 1);
        assert_eq!(x.entities, 2);
        assert_eq!(x.param_entities, 1);
        assert_eq!(x.notations, 1);
        assert_eq!(x.doctypes, 1);
        assert_eq!(x.comments, 1);
        assert_eq!(x.publics, 1);
        assert_eq!(x.systems, 1);
        assert!(x.content_keywords >= 2);
        assert_eq!(x.attr_keywords, 1);
        assert_eq!(x.conditionals, 1);
        assert!(x.pe_refs >= 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"<!ELEMENT a EMPTY>"));
        assert!(!detect(b"<a/>"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"plain").is_none());
    }
}
