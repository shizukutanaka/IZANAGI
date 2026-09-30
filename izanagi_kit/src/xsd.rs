//! XML Schema Definition `.xsd` — a `<xs:schema>` document with
//! `element`/`complexType`/`simpleType`/`attribute`/`sequence`/`choice`
//! `all`/`enumeration`/`import`/`include` declarations and a
//! `targetNamespace`.
//!
//! ```
//! let d = b"<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\" \
//! targetNamespace=\"t\"><xs:element name=\"a\" type=\"xs:int\"/>\
//! <xs:complexType name=\"c\"><xs:sequence><xs:element name=\"i\"/></xs:sequence></xs:complexType></xs:schema>";
//! let x = izanagi_kit::xsd::parse(d).unwrap();
//! assert_eq!(x.elements, 2);
//! assert_eq!(x.complex_types, 1);
//! assert_eq!(x.sequences, 1);
//! assert!(x.target_ns.is_some());
//! assert!(izanagi_kit::xsd::detect(d));
//! ```

/// Census of an `.xsd` schema.
#[derive(Debug, Clone)]
pub struct Xsd {
    /// `<*:element` declarations.
    pub elements: usize,
    /// `<*:complexType` declarations.
    pub complex_types: usize,
    /// `<*:simpleType` declarations.
    pub simple_types: usize,
    /// `<*:attribute` declarations.
    pub attributes: usize,
    /// `<*:sequence` compositors.
    pub sequences: usize,
    /// `<*:choice` compositors.
    pub choices: usize,
    /// `<*:all` compositors.
    pub alls: usize,
    /// `<*:enumeration` facets.
    pub enumerations: usize,
    /// `<*:import` statements.
    pub imports: usize,
    /// `<*:include` statements.
    pub includes: usize,
    /// `targetNamespace="…"` value length.
    pub target_ns: Option<usize>,
    /// `elementFormDefault="qualified"` present.
    pub qualified: bool,
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
        let close = t[..a]
            .rfind('<')
            .is_some_and(|i| t[i + 1..].starts_with('/'));
        let bounded = !close
            && t[..a].chars().last().is_some_and(|c| c == '<' || c == ':')
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

fn attr<'a>(t: &'a str, k: &str) -> Option<&'a str> {
    let at = t.find(k)? + k.len();
    let r = t.get(at..)?.trim_start();
    let r = r.strip_prefix('=')?.trim_start();
    let q = r.chars().next()?;
    if q != '"' && q != '\'' {
        return None;
    }
    let e = r[1..].find(q)?;
    Some(&r[1..1 + e])
}

/// Detects an `.xsd`: a `schema` element bound to the XMLSchema namespace.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("XMLSchema") && (t.contains(":schema") || t.contains("<schema"))
}

/// Parses an `.xsd`; `None` on non-UTF-8 or missing markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Xsd> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    Some(Xsd {
        elements: count_tag(t, "element"),
        complex_types: count_tag(t, "complexType"),
        simple_types: count_tag(t, "simpleType"),
        attributes: count_tag(t, "attribute"),
        sequences: count_tag(t, "sequence"),
        choices: count_tag(t, "choice"),
        alls: count_tag(t, "all"),
        enumerations: count_tag(t, "enumeration"),
        imports: count_tag(t, "import"),
        includes: count_tag(t, "include"),
        target_ns: attr(t, "targetNamespace").map(str::len),
        qualified: t.contains("qualified"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\" targetNamespace=\"urn:t\" elementFormDefault=\"qualified\"><xs:element name=\"a\" type=\"xs:int\"/><xs:complexType name=\"c\"><xs:sequence><xs:element name=\"i\"/><xs:choice><xs:element name=\"j\"/></xs:choice></xs:sequence><xs:attribute name=\"k\"/></xs:complexType><xs:simpleType name=\"s\"><xs:restriction><xs:enumeration value=\"v\"/></xs:restriction></xs:simpleType><xs:import schemaLocation=\"o.xsd\"/><xs:include schemaLocation=\"p.xsd\"/></xs:schema>";

    #[test]
    fn parses() {
        let x = parse(D).unwrap();
        assert_eq!(x.elements, 3);
        assert_eq!(x.complex_types, 1);
        assert_eq!(x.simple_types, 1);
        assert_eq!(x.attributes, 1);
        assert_eq!(x.sequences, 1);
        assert_eq!(x.choices, 1);
        assert_eq!(x.alls, 0);
        assert_eq!(x.enumerations, 1);
        assert_eq!(x.imports, 1);
        assert_eq!(x.includes, 1);
        assert_eq!(x.target_ns, Some(5));
        assert!(x.qualified);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(
            b"<schema xmlns=\"http://www.w3.org/2001/XMLSchema\"/>"
        ));
        assert!(!detect(b"<xs:element/>"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<other/>").is_none());
    }
}
