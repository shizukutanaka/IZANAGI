//! RELAX NG `.rng` (XML syntax) — a `<grammar>` in the
//! `relaxng.org/ns/structure` namespace with `start`/`element`/`define`
//! `ref`/`attribute`/`choice`/`optional`/`zeroOrMore`/`oneOrMore`/`value`
//! `data`/`text`/`empty`/`list`/`interleave`/`mixed`/`group`/`notAllowed`
//! patterns.
//!
//! ```
//! let d = b"<grammar xmlns=\"http://relaxng.org/ns/structure/1.0\"><start>\
//! <element name=\"a\"><attribute name=\"k\"/></element></start>\
//! <define name=\"r\"><element name=\"b\"><empty/></element></define></grammar>";
//! let r = izanagi_kit::relaxng::parse(d).unwrap();
//! assert_eq!(r.starts, 1);
//! assert_eq!(r.elements, 2);
//! assert_eq!(r.defines, 1);
//! assert_eq!(r.attributes, 1);
//! assert!(izanagi_kit::relaxng::detect(d));
//! ```

/// Census of a RELAX NG (XML syntax) grammar.
#[derive(Debug, Clone)]
pub struct Relaxng {
    /// `<grammar` root elements.
    pub grammars: usize,
    /// `<start` blocks.
    pub starts: usize,
    /// `<define` blocks.
    pub defines: usize,
    /// `<element` patterns.
    pub elements: usize,
    /// `<attribute` patterns.
    pub attributes: usize,
    /// `<ref` references.
    pub refs: usize,
    /// `<externalRef` references.
    pub external_refs: usize,
    /// `<choice` combinators.
    pub choices: usize,
    /// `<optional` markers.
    pub optionals: usize,
    /// `<zeroOrMore` repetitions.
    pub zero_or_more: usize,
    /// `<oneOrMore` repetitions.
    pub one_or_more: usize,
    /// `<value` literals.
    pub values: usize,
    /// `<data` typed patterns.
    pub datas: usize,
    /// `<text` patterns.
    pub texts: usize,
    /// `<empty` patterns.
    pub empties: usize,
    /// `<group` combinators.
    pub groups: usize,
    /// `<interleave` combinators.
    pub interleaves: usize,
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

/// Detects an `.rng`: the RELAX NG structure namespace.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("relaxng.org/ns/structure") && t.contains("<grammar")
}

/// Parses an `.rng`; `None` on non-UTF-8 or missing markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Relaxng> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    Some(Relaxng {
        grammars: count_tag(t, "grammar"),
        starts: count_tag(t, "start"),
        defines: count_tag(t, "define"),
        elements: count_tag(t, "element"),
        attributes: count_tag(t, "attribute"),
        refs: count_tag(t, "ref"),
        external_refs: count_tag(t, "externalRef"),
        choices: count_tag(t, "choice"),
        optionals: count_tag(t, "optional"),
        zero_or_more: count_tag(t, "zeroOrMore"),
        one_or_more: count_tag(t, "oneOrMore"),
        values: count_tag(t, "value"),
        datas: count_tag(t, "data"),
        texts: count_tag(t, "text"),
        empties: count_tag(t, "empty"),
        groups: count_tag(t, "group"),
        interleaves: count_tag(t, "interleave"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"<grammar xmlns=\"http://relaxng.org/ns/structure/1.0\"><start><element name=\"a\"><attribute name=\"k\"/><optional><element name=\"o\"><empty/></element></optional><zeroOrMore><element name=\"z\"><text/></element></zeroOrMore><oneOrMore><element name=\"m\"><data type=\"int\"/></element></oneOrMore><choice><value>v</value></choice><interleave><element name=\"i\"/></interleave><group><element name=\"g\"/></group></element></start><define name=\"r\"><element name=\"b\"><ref name=\"x\"/></element></define><define name=\"x\"><externalRef href=\"e.rng\"/></define></grammar>";

    #[test]
    fn parses() {
        let r = parse(D).unwrap();
        assert_eq!(r.grammars, 1);
        assert_eq!(r.starts, 1);
        assert_eq!(r.defines, 2);
        assert_eq!(r.elements, 7);
        assert_eq!(r.attributes, 1);
        assert_eq!(r.refs, 1);
        assert_eq!(r.external_refs, 1);
        assert_eq!(r.choices, 1);
        assert_eq!(r.optionals, 1);
        assert_eq!(r.zero_or_more, 1);
        assert_eq!(r.one_or_more, 1);
        assert_eq!(r.values, 1);
        assert_eq!(r.datas, 1);
        assert_eq!(r.texts, 1);
        assert_eq!(r.empties, 1);
        assert_eq!(r.groups, 1);
        assert_eq!(r.interleaves, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"<grammar/>"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<other/>").is_none());
    }
}
