//! Schematron `.sch` — `<schema>` with `<phase>`/`<pattern>`/`<rule>`
//! `context=` selectors and `<assert>`/`<report>` tests plus
//! `<ns>` namespace bindings.
//!
//! ```
//! let d = b"<schema xmlns=\"http://purl.oclc.org/dsdl/schematron\">\
//! <ns uri=\"u\" prefix=\"p\"/><phase id=\"x\"><active pattern=\"a\"/></phase>\
//! <pattern id=\"a\"><rule context=\"c\"><assert test=\"t\">m</assert>\
//! <report test=\"u\"/></rule></pattern></schema>";
//! let s = izanagi_kit::sch::parse(d).unwrap();
//! assert_eq!(s.patterns, 1);
//! assert_eq!(s.rules, 1);
//! assert_eq!(s.asserts, 1);
//! assert_eq!(s.reports, 1);
//! assert!(izanagi_kit::sch::detect(d));
//! ```

/// Census of a Schematron schema.
#[derive(Debug, Clone)]
pub struct Sch {
    /// `<ns` namespace bindings.
    pub namespaces: usize,
    /// `<phase` declarations.
    pub phases: usize,
    /// `<active` phase references.
    pub actives: usize,
    /// `<pattern` blocks.
    pub patterns: usize,
    /// `<rule` blocks.
    pub rules: usize,
    /// `<assert` tests.
    pub asserts: usize,
    /// `<report` tests.
    pub reports: usize,
    /// `context="…"` attribute occurrences.
    pub contexts: usize,
    /// `<diagnostics`/`<diagnostic` occurrences.
    pub diagnostics: usize,
    /// `<let` variable bindings.
    pub lets: usize,
    /// `<extends` rule extensions.
    pub extends: usize,
    /// `<title`/`<p` documentation blocks.
    pub docs: usize,
}

const BOUND: &[char] = &[' ', '\t', '\n', '/', '>', ':'];

fn count_tag(t: &str, name: &str) -> usize {
    let mut n = 0;
    let mut from = 0;
    while let Some(slice) = t.get(from..) {
        let Some(p) = slice.find(name) else {
            break;
        };
        let a = from + p;
        let bounded = t[..a].chars().last().is_some_and(|c| c == '<')
            && t[a + name.len()..]
                .chars()
                .next()
                .is_some_and(|c| BOUND.contains(&c));
        if bounded {
            n += 1;
        }
        from = a + name.len();
    }
    n
}

/// Detects a `.sch`: the Schematron namespace or schema+rule+assert shape.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("schematron")
        || (t.contains("<pattern") && t.contains("<rule") && t.contains("context="))
}

/// Parses a `.sch`; `None` on non-UTF-8 or missing markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Sch> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    Some(Sch {
        namespaces: count_tag(t, "ns"),
        phases: count_tag(t, "phase"),
        actives: count_tag(t, "active"),
        patterns: count_tag(t, "pattern"),
        rules: count_tag(t, "rule"),
        asserts: count_tag(t, "assert"),
        reports: count_tag(t, "report"),
        contexts: t.matches("context=").count(),
        diagnostics: count_tag(t, "diagnostic") + count_tag(t, "diagnostics"),
        lets: count_tag(t, "let"),
        extends: count_tag(t, "extends"),
        docs: count_tag(t, "title") + count_tag(t, "p"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"<schema xmlns=\"http://purl.oclc.org/dsdl/schematron\"><title>T</title><ns uri=\"u\" prefix=\"p\"/><phase id=\"x\"><active pattern=\"a\"/></phase><pattern id=\"a\"><rule context=\"c\"><let name=\"v\" value=\"1\"/><assert test=\"t\" diagnostics=\"d\">m</assert><report test=\"u\"/></rule><rule context=\"e\"><extends rule=\"r\"/></rule></pattern><diagnostics><diagnostic id=\"d\">x</diagnostic></diagnostics></schema>";

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert_eq!(s.namespaces, 1);
        assert_eq!(s.phases, 1);
        assert_eq!(s.actives, 1);
        assert_eq!(s.patterns, 1);
        assert_eq!(s.rules, 2);
        assert_eq!(s.asserts, 1);
        assert_eq!(s.reports, 1);
        assert_eq!(s.contexts, 2);
        assert_eq!(s.diagnostics, 2);
        assert_eq!(s.lets, 1);
        assert_eq!(s.extends, 1);
        assert_eq!(s.docs, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"<pattern><rule context=\"x\"/></pattern>"));
        assert!(!detect(b"<schema/>"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<other/>").is_none());
    }
}
