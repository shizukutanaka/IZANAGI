//! IP-XACT — IEEE 1685 XML component/design descriptions.
//!
//! Root element lives in the `ipxact` (1685-2009/2014/2022) or
//! legacy `spirit` (1685-1999) namespace and is one of
//! `component`, `design`, `generator`, `busDefinition`,
//! `abstractionDefinition`, `catalog`. The VLNV quad is carried by
//! `<…:vendor>`/`<…:library>`/`<…:name>`/`<…:version>`.
//!
//! ```
//! let d = b"<ipxact:component xmlns:ipxact='http://www.accellera.org/XMLSchema/IPXACT/1685-2014'>\
//! <ipxact:vendor>acme</ipxact:vendor><ipxact:library>lib</ipxact:library>\
//! <ipxact:name>uart</ipxact:name><ipxact:version>1\x2e0</ipxact:version>\
//! <ipxact:model/><ipxact:busInterfaces/></ipxact:component>";
//! let f = izanagi_kit::ipxact::parse(d).unwrap();
//! assert_eq!(f.root, "component");
//! assert_eq!(f.name.as_deref(), Some("uart"));
//! ```
//!
//! Reference: IEEE 1685 IP-XACT (Accellera schema set); kactus2 /
//! ipyxact implementations. Integer-only.

/// Parsed IP-XACT root + VLNV.
#[derive(Debug, Clone, PartialEq)]
pub struct Ipxact {
    /// Root element name: `component`, `design`, `generator`,
    /// `busDefinition`, `abstractionDefinition` or `catalog`.
    pub root: String,
    /// Namespace prefix actually used (`ipxact` or `spirit`).
    pub namespace: String,
    /// VLNV vendor.
    pub vendor: Option<String>,
    /// VLNV library.
    pub library: Option<String>,
    /// VLNV name.
    pub name: Option<String>,
    /// VLNV version.
    pub version: Option<String>,
    /// `<*:busInterface>` elements.
    pub bus_interfaces: u32,
    /// `<*:memoryMap>` elements.
    pub memory_maps: u32,
    /// `<*:port>` elements.
    pub ports: u32,
    /// `<*:parameter>` + `<*:modelParameter>` elements.
    pub parameters: u32,
    /// `<*:file>` elements inside `fileSets`.
    pub files: u32,
}

fn text_of(s: &str, ns: &str, tag: &str) -> Option<String> {
    let open = ["<", ns, ":", tag].concat();
    let close = ["</", ns, ":", tag, ">"].concat();
    let st = s.find(&open)? + open.len();
    // the open tag may carry attributes; find its '>' first
    let st = s[st..].find('>')? + st + 1;
    let e = s[st..].find(&close)? + st;
    Some(s[st..e].trim().to_string())
}

fn count_elem(s: &str, ns: &str, tag: &str) -> u32 {
    let pat = ["<", ns, ":", tag].concat();
    let mut n = 0;
    let mut i = 0;
    while let Some(p) = s[i..].find(&pat) {
        let j = i + p + pat.len();
        match s[j..].chars().next() {
            Some(c) if c.is_ascii_alphanumeric() || c == '_' || c == '-' => {}
            _ => n += 1,
        }
        i = j;
    }
    n
}

const ROOTS: &[&str] = &[
    "component",
    "design",
    "generator",
    "busDefinition",
    "abstractionDefinition",
    "catalog",
];

/// Parse the root element and VLNV. `None` when no
/// `ipxact:`/`spirit:` root element is present.
pub fn parse(d: &[u8]) -> Option<Ipxact> {
    let s = core::str::from_utf8(d).ok()?;
    for ns in ["ipxact", "spirit"] {
        for root in ROOTS {
            let pat = ["<", ns, ":", root].concat();
            if let Some(p) = s.find(&pat) {
                let j = p + pat.len();
                let c = s[j..].chars().next();
                if let Some(c) = c {
                    if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                        continue;
                    }
                }
                return Some(Ipxact {
                    root: root.to_string(),
                    namespace: ns.to_string(),
                    vendor: text_of(s, ns, "vendor"),
                    library: text_of(s, ns, "library"),
                    name: text_of(s, ns, "name"),
                    version: text_of(s, ns, "version"),
                    bus_interfaces: count_elem(s, ns, "busInterface"),
                    memory_maps: count_elem(s, ns, "memoryMap"),
                    ports: count_elem(s, ns, "port"),
                    parameters: count_elem(s, ns, "parameter")
                        + count_elem(s, ns, "modelParameter"),
                    files: count_elem(s, ns, "file"),
                });
            }
        }
    }
    None
}

/// `true` when an `ipxact:`/`spirit:` root element appears.
pub fn detect(d: &[u8]) -> bool {
    d.windows(8).any(|w| w == b"<ipxact:") || d.windows(8).any(|w| w == b"<spirit:")
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] =
        b"<ipxact:component xmlns:ipxact='http://www.accellera.org/XMLSchema/IPXACT/1685-2014'>\
        <ipxact:vendor>acme</ipxact:vendor><ipxact:library>lib</ipxact:library>\
        <ipxact:name>uart</ipxact:name><ipxact:version>1\x2e0</ipxact:version>\
        <ipxact:busInterfaces><ipxact:busInterface/></ipxact:busInterfaces>\
        <ipxact:memoryMaps><ipxact:memoryMap/></ipxact:memoryMaps>\
        <ipxact:model><ipxact:ports><ipxact:port/></ipxact:ports>\
        <ipxact:modelParameters><ipxact:modelParameter/></ipxact:modelParameters></ipxact:model>\
        <ipxact:fileSets><ipxact:fileSet><ipxact:file><ipxact:name>a.v</ipxact:name></ipxact:file>\
        </ipxact:fileSet></ipxact:fileSets></ipxact:component>";

    #[test]
    fn parses() {
        let f = parse(DOC).unwrap();
        assert_eq!(f.root, "component");
        assert_eq!(f.namespace, "ipxact");
        assert_eq!(f.vendor.as_deref(), Some("acme"));
        assert_eq!(f.library.as_deref(), Some("lib"));
        assert_eq!(f.name.as_deref(), Some("uart"));
        assert_eq!(f.version.as_deref(), Some("1\x2e0"));
        assert_eq!(f.bus_interfaces, 1);
        assert_eq!(f.memory_maps, 1);
        assert_eq!(f.ports, 1);
        assert_eq!(f.parameters, 1);
        assert_eq!(f.files, 1);
    }

    #[test]
    fn spirit_and_other_roots() {
        let d = b"<spirit:design xmlns:spirit='x'><spirit:vendor>v</spirit:vendor></spirit:design>";
        let f = parse(d).unwrap();
        assert_eq!(f.root, "design");
        assert_eq!(f.namespace, "spirit");
        let c = parse(b"<ipxact:catalog xmlns:ipxact='x'></ipxact:catalog>").unwrap();
        assert_eq!(c.root, "catalog");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"<component/>").is_none()); // no namespace
        assert!(parse(b"<html/>").is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"<x:component/>"));
    }
}
