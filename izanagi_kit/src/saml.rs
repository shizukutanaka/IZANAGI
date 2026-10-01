//! SAML 2.0 — `<samlp:Response>`/`<saml:Assertion>` XML with
//! `Issuer`/`Subject`/`Conditions`/`AuthnStatement`/`Attribute`/
//! `NameID`/`Signature`/`EncryptedAssertion`/`Status`/`InResponseTo`.
//!
//! ```
//! let d = b"<samlp:Response xmlns:samlp=\"urn:oasis:names:tc:SAML:2.0:protocol\">\
//! <saml:Assertion xmlns:saml=\"urn:oasis:names:tc:SAML:2.0:assertion\">\
//! <saml:Issuer>i</saml:Issuer><saml:Subject><saml:NameID>n</saml:NameID></saml:Subject>\
//! <saml:AuthnStatement/><saml:AttributeStatement><saml:Attribute Name=\"a\"/></saml:AttributeStatement>\
//! </saml:Assertion></samlp:Response>";
//! let s = izanagi_kit::saml::parse(d).unwrap();
//! assert_eq!(s.assertions, 1);
//! assert_eq!(s.issuers, 1);
//! assert_eq!(s.attributes, 1);
//! assert!(izanagi_kit::saml::detect(d));
//! ```

/// Census of a SAML message/document.
#[derive(Debug, Clone)]
pub struct Saml {
    /// `samlp:Response` root.
    pub response: bool,
    /// `samlp:AuthnRequest` root.
    pub authn_request: bool,
    /// `<*:Assertion` elements.
    pub assertions: usize,
    /// `<*:Issuer` elements.
    pub issuers: usize,
    /// `<*:Subject` elements.
    pub subjects: usize,
    /// `<*:NameID` elements.
    pub name_ids: usize,
    /// `<*:Conditions` elements.
    pub conditions: usize,
    /// `<*:AuthnStatement` elements.
    pub authn_statements: usize,
    /// `<*:Attribute` elements (inside AttributeStatement too).
    pub attributes: usize,
    /// `<*:AttributeStatement` elements.
    pub attribute_statements: usize,
    /// `<ds:Signature>`/`<Signature` elements.
    pub signatures: usize,
    /// `<*:EncryptedAssertion`/`<*:EncryptedID`.
    pub encrypted: usize,
    /// `InResponseTo=` attribute occurrences.
    pub in_response_tos: usize,
    /// `NotBefore=`/`NotOnOrAfter=` occurrences.
    pub validity_bounds: usize,
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

/// Detects SAML: a `samlp:`/`saml:` namespaced element or the SAML URIs.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains(":saml")
        || t.contains("samlp:")
        || t.contains("names:tc:SAML:")
        || t.contains("<saml:")
}

/// Parses a SAML document; `None` on non-UTF-8 or missing markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Saml> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    Some(Saml {
        response: t.contains("Response"),
        authn_request: t.contains("AuthnRequest"),
        assertions: count_tag(t, "Assertion"),
        issuers: count_tag(t, "Issuer"),
        subjects: count_tag(t, "Subject"),
        name_ids: count_tag(t, "NameID"),
        conditions: count_tag(t, "Conditions"),
        authn_statements: count_tag(t, "AuthnStatement"),
        attributes: count_tag(t, "Attribute"),
        attribute_statements: count_tag(t, "AttributeStatement"),
        signatures: count_tag(t, "Signature"),
        encrypted: count_tag(t, "EncryptedAssertion") + count_tag(t, "EncryptedID"),
        in_response_tos: t.matches("InResponseTo=").count(),
        validity_bounds: t.matches("NotBefore=").count() + t.matches("NotOnOrAfter=").count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"<samlp:Response xmlns:samlp=\"urn:oasis:names:tc:SAML:2.0:protocol\" InResponseTo=\"r\"><samlp:Status/><saml:Assertion xmlns:saml=\"urn:oasis:names:tc:SAML:2.0:assertion\"><saml:Issuer>i</saml:Issuer><ds:Signature xmlns:ds=\"d\"/><saml:Subject><saml:NameID>n</saml:NameID></saml:Subject><saml:Conditions NotBefore=\"t\" NotOnOrAfter=\"u\"/><saml:AuthnStatement/><saml:AttributeStatement><saml:Attribute Name=\"a\"/><saml:Attribute Name=\"b\"/></saml:AttributeStatement></saml:Assertion><saml:EncryptedAssertion/></samlp:Response>";

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert!(s.response);
        assert!(!s.authn_request);
        assert_eq!(s.assertions, 1);
        assert_eq!(s.issuers, 1);
        assert_eq!(s.subjects, 1);
        assert_eq!(s.name_ids, 1);
        assert_eq!(s.conditions, 1);
        assert_eq!(s.authn_statements, 1);
        assert_eq!(s.attributes, 2);
        assert_eq!(s.attribute_statements, 1);
        assert_eq!(s.signatures, 1);
        assert_eq!(s.encrypted, 1);
        assert_eq!(s.in_response_tos, 1);
        assert_eq!(s.validity_bounds, 2);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"<saml:Assertion/>"));
        assert!(!detect(b"<r/>"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<other/>").is_none());
    }
}
