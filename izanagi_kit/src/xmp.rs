//! XMP — Adobe Extensible Metadata Platform packet:
//! `<?xpacket …?>` PI + `<x:xmpmeta>` root + `rdf:RDF`/`rdf:Description`
//! property blocks with `dc:`/`xmp:`/`photoshop:`/`tiff:`/`exif:` namespaces.
//!
//! ```
//! let d = b"<?xpacket begin=\"\\xef\\xbb\\xbf\"?><x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF><rdf:Description dc:format=\"image\"/></rdf:RDF></x:xmpmeta><?xpacket end=\"w\"?>";
//! let x = izanagi_kit::xmp::parse(d).unwrap();
//! assert_eq!(x.packets, 2);
//! assert_eq!(x.descriptions, 1);
//! assert!(izanagi_kit::xmp::detect(d));
//! ```

/// Census of an XMP packet.
#[derive(Debug, Clone)]
pub struct Xmp {
    /// `<?xpacket` instructions.
    pub packets: usize,
    /// `begin` attribute present.
    pub begin: bool,
    /// `end="w"|"r"` present.
    pub end_marker: bool,
    /// `<x:xmpmeta` root found.
    pub xmpmeta: bool,
    /// `<rdf:RDF` found.
    pub rdf: bool,
    /// `rdf:Description` open tags.
    pub descriptions: usize,
    /// `rdf:Seq` ordered arrays.
    pub seqs: usize,
    /// `rdf:Bag` unordered arrays.
    pub bags: usize,
    /// `rdf:Alt` alternates.
    pub alts: usize,
    /// `rdf:li` items.
    pub lis: usize,
    /// Namespace prefixes present (dc/xmp/photoshop/tiff/exif/xmpMM/xmpRights/stEvt).
    pub namespaces: usize,
    /// `xmlns:` declarations.
    pub xmlns: usize,
    /// `xmp:CreatorTool`/`Rating`/`Label` common props.
    pub xmp_props: usize,
    /// `dc:` Dublin Core props.
    pub dc_props: usize,
}

const NS: &[&str] = &[
    "dc:",
    "xmp:",
    "photoshop:",
    "tiff:",
    "exif:",
    "xmpMM:",
    "xmpRights:",
    "stEvt:",
];

/// Detects XMP: `<?xpacket` + `<x:xmpmeta` + `rdf`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.windows(9).any(|w| w == b"<?xpacket")
        && b.windows(10).any(|w| w == b"<x:xmpmeta")
        && b.windows(4).any(|w| w == b"rdf:")
}

/// Parses an XMP packet; `None` without the xpacket/xmpmeta pair.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Xmp> {
    if !detect(b) {
        return None;
    }
    let t = String::from_utf8_lossy(b);
    let open = |tag: &str| {
        let mut n = 0usize;
        let mut from = 0;
        while let Some(p) = t[from..].find(tag) {
            let a = from + p;
            let after = t[a + tag.len()..]
                .chars()
                .next()
                .map_or(true, |c| !c.is_alphanumeric() && c != '/' && c != ':');
            if after && !t[..a].ends_with("</") {
                n += 1;
            }
            from = a + tag.len();
        }
        n
    };
    let ns = NS.iter().filter(|p| t.contains(**p)).count();
    Some(Xmp {
        packets: t.matches("<?xpacket").count(),
        begin: t.matches("begin").count() > 0,
        end_marker: t.matches("end=").count() > 0,
        xmpmeta: true,
        rdf: t.matches("<rdf:RDF").count() > 0 || t.matches("rdf:RDF").count() > 0,
        descriptions: open("rdf:Description"),
        seqs: open("rdf:Seq"),
        bags: open("rdf:Bag"),
        alts: open("rdf:Alt"),
        lis: open("rdf:li"),
        namespaces: ns,
        xmlns: t.matches("xmlns").count(),
        xmp_props: t.matches("xmp:").count(),
        dc_props: t.matches("dc:").count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"<?xpacket begin=\"\xef\xbb\xbf\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?><x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF><rdf:Description dc:format=\"image/png\" xmp:CreatorTool=\"tool\"><dc:title><rdf:Alt><rdf:li>t</rdf:li></rdf:Alt></dc:title></rdf:Description></rdf:RDF></x:xmpmeta><?xpacket end=\"w\"?>";

    #[test]
    fn parses() {
        let x = parse(D).unwrap();
        assert_eq!(x.packets, 2);
        assert!(x.begin && x.end_marker);
        assert_eq!(x.descriptions, 1);
        assert_eq!(x.alts, 1);
        assert_eq!(x.lis, 1);
        assert!(x.namespaces >= 2);
        assert!(x.dc_props >= 2);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"<?xpacket?>"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
    }
}
