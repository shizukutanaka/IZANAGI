//! XML Signature (XMLDSIG / RFC 3275) — `<Signature xmlns="…/xmldsig#">`
//! with `<SignedInfo>`/`<SignatureValue>`/`<KeyInfo>`/`<Object>` blocks,
//! `<Reference URI>`s, Transform/DigestMethod algorithms.
//!
//! ```
//! let d = b"<Signature xmlns=\"http://www.w3.org/2000/09/xmldsig#\"><SignedInfo><Reference URI=\"#x\"/></SignedInfo><SignatureValue>v</SignatureValue></Signature>";
//! let s = izanagi_kit::dsig::parse(d).unwrap();
//! assert_eq!(s.references, 1);
//! assert!(s.signed_info);
//! assert!(izanagi_kit::dsig::detect(d));
//! ```

/// Census of an XML Signature document.
#[derive(Debug, Clone)]
pub struct Dsig {
    /// `<Signature` root found.
    pub signature: bool,
    /// `xmldsig#` namespace declared.
    pub ns: bool,
    /// `<SignedInfo` block.
    pub signed_info: bool,
    /// `<SignatureValue` present.
    pub signature_value: bool,
    /// `<KeyInfo` block.
    pub key_info: bool,
    /// `<Object` blocks.
    pub objects: usize,
    /// `<Reference` elements.
    pub references: usize,
    /// `URI=` attributes.
    pub uris: usize,
    /// `<Transform` elements.
    pub transforms: usize,
    /// `<DigestMethod` elements.
    pub digest_methods: usize,
    /// `Algorithm=` attributes.
    pub algorithms: usize,
    /// Canonicalization (`CanonicalizationMethod`/`c14n`).
    pub c14n: usize,
    /// `<X509` cert data present.
    pub x509: bool,
    /// `<Manifest` / `<SignatureProperties` extras.
    pub extras: usize,
}

fn open(t: &str, tag: &str) -> usize {
    let mut n = 0usize;
    let mut from = 0;
    let needle = ["<", tag].concat();
    while let Some(p) = t[from..].find(&needle) {
        let a = from + p;
        let after = t[a + needle.len()..]
            .chars()
            .next()
            .map_or(true, |c| !c.is_alphanumeric() && c != ':');
        if after && !t[..a].ends_with("</") {
            n += 1;
        }
        from = a + needle.len();
    }
    n
}

/// Detects XMLDSIG: `<Signature` + `xmldsig` namespace.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.windows(10).any(|w| w == b"<Signature") && b.windows(7).any(|w| w == b"xmldsig")
}

/// Parses an XMLDSIG document; `None` without signature + namespace.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Dsig> {
    if !detect(b) {
        return None;
    }
    let t = String::from_utf8_lossy(b);
    Some(Dsig {
        signature: true,
        ns: t.contains("xmldsig"),
        signed_info: t.contains("<SignedInfo"),
        signature_value: t.contains("<SignatureValue"),
        key_info: t.contains("<KeyInfo"),
        objects: open(&t, "Object"),
        references: open(&t, "Reference"),
        uris: t.matches("URI=").count(),
        transforms: open(&t, "Transform"),
        digest_methods: open(&t, "DigestMethod"),
        algorithms: t.matches("Algorithm=").count(),
        c14n: t.matches("CanonicalizationMethod").count() + t.matches("c14n").count(),
        x509: t.matches("<X509").count() > 0,
        extras: open(&t, "Manifest") + open(&t, "SignatureProperties"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"<Signature xmlns=\"http://www.w3.org/2000/09/xmldsig#\"><SignedInfo><CanonicalizationMethod Algorithm=\"http://c14n\"/><Reference URI=\"#r1\"><DigestMethod Algorithm=\"sha1\"/></Reference></SignedInfo><SignatureValue>v</SignatureValue><KeyInfo><X509Data/></KeyInfo><Object/></Signature>";

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert!(s.signed_info && s.signature_value && s.key_info && s.x509);
        assert_eq!(s.references, 1);
        assert_eq!(s.uris, 1);
        assert_eq!(s.digest_methods, 1);
        assert_eq!(s.objects, 1);
        assert!(s.c14n >= 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"<Signature xmlns=\"x\"/>"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
    }
}
