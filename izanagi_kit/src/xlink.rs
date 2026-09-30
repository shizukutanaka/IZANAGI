//! XLink — XML Linking Language attributes inside any document:
//! `xlink:href="…"`, `xlink:type="simple|extended|locator|arc|resource|
//! title"`, `xlink:role`/`arcrole`/`show`/`actuate`/`label`/`to`/`from`.
//!
//! ```
//! let d = b"<root xmlns:xlink=\"http://www.w3.org/1999/xlink\">\
//! <a xlink:href=\"u\" xlink:type=\"simple\"/><e xlink:type=\"extended\">\
//! <l xlink:type=\"locator\" xlink:href=\"o\" xlink:label=\"t\"/></e></root>";
//! let x = izanagi_kit::xlink::parse(d).unwrap();
//! assert_eq!(x.hrefs, 2);
//! assert_eq!(x.simple, 1);
//! assert_eq!(x.extended, 1);
//! assert_eq!(x.locators, 1);
//! assert!(izanagi_kit::xlink::detect(d));
//! ```

/// Census of XLink usage in a document.
#[derive(Debug, Clone)]
pub struct Xlink {
    /// `xlink:href=` occurrences.
    pub hrefs: usize,
    /// `xlink:type="simple"`.
    pub simple: usize,
    /// `xlink:type="extended"`.
    pub extended: usize,
    /// `xlink:type="locator"`.
    pub locators: usize,
    /// `xlink:type="arc"`.
    pub arcs: usize,
    /// `xlink:type="resource"`.
    pub resources: usize,
    /// `xlink:type="title"`.
    pub titles: usize,
    /// `xlink:role=` occurrences.
    pub roles: usize,
    /// `xlink:arcrole=` occurrences.
    pub arcroles: usize,
    /// `xlink:show=` occurrences.
    pub shows: usize,
    /// `xlink:actuate=` occurrences.
    pub actuates: usize,
    /// `xlink:label=` occurrences.
    pub labels: usize,
    /// `xlink:to=`/`xlink:from=` occurrences.
    pub to_froms: usize,
    /// `xmlns:xlink` namespace declarations.
    pub ns_decls: usize,
}

fn kv(t: &str, k: &str, v: &str) -> usize {
    let dq = format!("{k}=\"{v}\"");
    let sq = format!("{k}='{v}'");
    t.matches(&dq).count() + t.matches(&sq).count()
}

/// Detects XLink usage: the `xlink:` attribute prefix or namespace.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    t.contains("xlink:href") || t.contains("xlink:type") || t.contains("w3.org/1999/xlink")
}

/// Parses XLink usage; `None` on non-UTF-8 or no markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Xlink> {
    let t = std::str::from_utf8(b).ok()?;
    if !detect(b) {
        return None;
    }
    Some(Xlink {
        hrefs: t.matches("xlink:href=").count(),
        simple: kv(t, "xlink:type", "simple"),
        extended: kv(t, "xlink:type", "extended"),
        locators: kv(t, "xlink:type", "locator"),
        arcs: kv(t, "xlink:type", "arc"),
        resources: kv(t, "xlink:type", "resource"),
        titles: kv(t, "xlink:type", "title"),
        roles: t.matches("xlink:role=").count(),
        arcroles: t.matches("xlink:arcrole=").count(),
        shows: t.matches("xlink:show=").count(),
        actuates: t.matches("xlink:actuate=").count(),
        labels: t.matches("xlink:label=").count(),
        to_froms: t.matches("xlink:to=").count() + t.matches("xlink:from=").count(),
        ns_decls: t.matches("xmlns:xlink").count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"<root xmlns:xlink=\"http://www.w3.org/1999/xlink\"><a xlink:href=\"u\" xlink:type=\"simple\" xlink:show=\"replace\" xlink:actuate=\"onRequest\" xlink:role=\"r\"/><e xlink:type=\"extended\"><l xlink:type=\"locator\" xlink:href=\"o\" xlink:label=\"t\"/><c xlink:type=\"arc\" xlink:from=\"f\" xlink:to=\"g\" xlink:arcrole=\"ar\"/><r xlink:type=\"resource\"/><t xlink:type=\"title\"/></e></root>";

    #[test]
    fn parses() {
        let x = parse(D).unwrap();
        assert_eq!(x.hrefs, 2);
        assert_eq!(x.simple, 1);
        assert_eq!(x.extended, 1);
        assert_eq!(x.locators, 1);
        assert_eq!(x.arcs, 1);
        assert_eq!(x.resources, 1);
        assert_eq!(x.titles, 1);
        assert_eq!(x.roles, 1);
        assert_eq!(x.arcroles, 1);
        assert_eq!(x.shows, 1);
        assert_eq!(x.actuates, 1);
        assert_eq!(x.labels, 1);
        assert_eq!(x.to_froms, 2);
        assert_eq!(x.ns_decls, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(detect(b"<a xlink:href=\"x\"/>"));
        assert!(!detect(b"<a href=\"x\"/>"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<a/>").is_none());
    }
}
