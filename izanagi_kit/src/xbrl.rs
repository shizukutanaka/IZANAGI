//! XBRL instance document census — XBRL 2.1 instances declare their namespace
//! (`xbrl.org/2003/instance`), carry `context`/`unit` resources with `id=`,
//! `schemaRef` links back to the taxonomy, and facts tagged with
//! `contextRef=`/`unitRef=`/`decimals=` attributes. Namespace-prefix free:
//! counts `context id=` / `unit id=` patterns regardless of prefix.
//!
//! ```
//! let d = b"<?xml version=\"1\x2e0\"?>\n<xbrli:xbrl xmlns:xbrli=\"http://www\x2exbrl\x2eorg/2003/instance\">\n<xbrli:context id=\"c1\"><xbrli:entity/><xbrli:period/></xbrli:context>\n<xbrli:unit id=\"u1\"><xbrli:measure>iso4217:EUR</xbrli:measure></xbrli:unit>\n<xbrli:schemaRef xlink:href=\"t\x2exsd\"/>\n<x:revenue contextRef=\"c1\" unitRef=\"u1\" decimals=\"0\">1000</x:revenue>\n<x:cost contextRef=\"c1\" unitRef=\"u1\" decimals=\"0\">300</x:cost>\n</xbrli:xbrl>";
//! let x = izanagi_kit::xbrl::parse(d).unwrap();
//! assert_eq!(x.contexts, 1);
//! assert_eq!(x.units, 1);
//! assert_eq!(x.facts, 2);
//! assert_eq!(x.schema_refs, 1);
//! assert_eq!(x.decimals_seen, 2);
//! assert!(izanagi_kit::xbrl::detect(d));
//! ```

/// A censused XBRL instance document.
pub struct Xbrl {
    /// `context` resources with an `id` attribute.
    pub contexts: u32,
    /// `unit` resources with an `id` attribute.
    pub units: u32,
    /// Facts: elements carrying a `contextRef=` attribute.
    pub facts: u32,
    /// `schemaRef` links to taxonomy schemas.
    pub schema_refs: u32,
    /// `unitRef=` attribute occurrences.
    pub unit_refs: u32,
    /// `decimals=` attribute occurrences.
    pub decimals_seen: u32,
    /// `footnoteLink` / `roleRef` / `arcroleRef` linkbase-reference count.
    pub link_refs: u32,
}

fn count(s: &str, pat: &str) -> u32 {
    u32::try_from(s.matches(pat).count()).unwrap_or(u32::MAX)
}

/// `xbrl` root element or the 2003 instance namespace.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.contains("<xbrl") || s.contains(":xbrl") || s.contains("xbrl.org/2003/instance")
}

/// Parses the document; `None` without an XBRL namespace marker.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Xbrl> {
    let s = core::str::from_utf8(b).ok()?;
    if !s.contains("xbrl.org/") && !s.contains("<xbrl") && !s.contains(":xbrl") {
        return None;
    }
    Some(Xbrl {
        contexts: count(s, "context id="),
        units: count(s, "unit id="),
        facts: count(s, "contextRef="),
        schema_refs: count(s, "schemaRef"),
        unit_refs: count(s, "unitRef="),
        decimals_seen: count(s, "decimals="),
        link_refs: count(s, "footnoteLink") + count(s, "roleRef") + count(s, "arcroleRef"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] = b"<?xml version=\"1\x2e0\"?>\n<xbrli:xbrl xmlns:xbrli=\"http://www\x2exbrl\x2eorg/2003/instance\">\n<xbrli:context id=\"c1\"/>\n<xbrli:unit id=\"u1\"/>\n<xbrli:schemaRef xlink:href=\"t\x2exsd\"/>\n<x:r contextRef=\"c1\" unitRef=\"u1\" decimals=\"0\">1</x:r>\n</xbrli:xbrl>";

    #[test]
    fn detect_works() {
        assert!(detect(FIXTURE));
        assert!(detect(b"<xbrl xmlns=\"urn:x\"/>"));
        assert!(!detect(b"<html/>"));
    }

    #[test]
    fn parses() {
        let x = parse(FIXTURE).unwrap();
        assert_eq!(x.contexts, 1);
        assert_eq!(x.units, 1);
        assert_eq!(x.facts, 1);
        assert_eq!(x.schema_refs, 1);
        assert_eq!(x.unit_refs, 1);
        assert_eq!(x.decimals_seen, 1);
        assert_eq!(x.link_refs, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<doc/>").is_none());
    }
}
