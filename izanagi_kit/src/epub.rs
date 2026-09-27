//! EPUB (Open Container Format): a ZIP whose first member is a stored
//! `mimetype` file containing `application/epub+zip`; `META-INF/container.xml`
//! then points at the OPF package document via `full-path`.
//!
//! ```
//! use izanagi_kit::epub::parse;
//!
//! let mut w = izanagi_kit::zip::ZipWriter::new();
//! w.add_stored("mimetype", b"application/epub+zip");
//! w.add_stored("META-INF/container.xml", b"<?xml?><container><rootfiles><rootfile full-path=\"OEBPS/content.opf\"/></rootfiles></container>");
//! let z = w.finish();
//! let e = parse(&z).unwrap();
//! assert_eq!(e.rootfile.as_deref(), Some("OEBPS/content.opf"));
//! ```

use std::string::String;

/// `mimetype` member content that must come first and uncompressed.
pub const MIMETYPE: &str = "application/epub+zip";
/// Well-known member naming the package document.
pub const CONTAINER: &str = "META-INF/container.xml";

/// Parsed container document.
#[derive(Debug, Clone)]
pub struct Epub {
    /// OPF package path from `container.xml` (`rootfile@full-path`).
    pub rootfile: Option<String>,
    /// All member names (via `crate::zip`).
    pub names: Vec<String>,
}

/// Parse an EPUB ZIP: verify `mimetype` is member 0, stored (method 0),
/// and holds the exact mimetype string; then read `container.xml`.
pub fn parse(d: &[u8]) -> Option<Epub> {
    let entries = crate::zip::list(d)?;
    let m = entries.first()?;
    if m.name != "mimetype" || m.method != 0 {
        return None;
    }
    let body = crate::zip::extract(d, "mimetype")?;
    if body != MIMETYPE.as_bytes() {
        return None;
    }
    let container = crate::zip::extract(d, CONTAINER).unwrap_or_default();
    let rootfile = attr(container.as_slice(), "full-path");
    Some(Epub {
        rootfile,
        names: entries.iter().map(|e| e.name.clone()).collect(),
    })
}

/// Extract `name="value"` from XML-ish bytes (`'` quotes also accepted).
fn attr(d: &[u8], name: &str) -> Option<String> {
    let text = std::str::from_utf8(d).ok()?;
    let pat = String::from(name);
    let mut rest = text;
    while let Some(i) = rest.find(pat.as_str()) {
        rest = &rest[i + pat.len()..];
        let rest = rest.trim_start();
        if !rest.starts_with('=') {
            continue;
        }
        let rest = rest[1..].trim_start();
        let q = rest.chars().next()?;
        if q != '"' && q != '\'' {
            return None;
        }
        let end = rest[1..].find(q)? + 1;
        return Some(String::from(&rest[1..end]));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn epub_zip() -> Vec<u8> {
        let mut w = crate::zip::ZipWriter::new();
        w.add_stored("mimetype", MIMETYPE.as_bytes());
        w.add_stored(
            "META-INF/container.xml",
            b"<rootfile full-path='OPS/main.opf'/>",
        );
        w.finish()
    }

    #[test]
    fn parses() {
        let e = parse(&epub_zip()).unwrap();
        assert_eq!(e.rootfile.as_deref(), Some("OPS/main.opf"));
        assert_eq!(e.names.len(), 2);
    }

    #[test]
    fn rejects_wrong_mimetype() {
        let mut w = crate::zip::ZipWriter::new();
        w.add_stored("mimetype", b"text/plain");
        assert!(parse(&w.finish()).is_none());
        assert!(parse(b"not zip").is_none());
        // mimetype must be the FIRST member
        let mut w = crate::zip::ZipWriter::new();
        w.add_stored("x", b"y");
        w.add_stored("mimetype", MIMETYPE.as_bytes());
        assert!(parse(&w.finish()).is_none());
    }
}
