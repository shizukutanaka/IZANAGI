//! KMZ — a zipped KML document (Google Earth). The archive must
//! contain a root-level `.kml` file (conventionally `doc.kml`).
//!
//! ```
//! let mut zw = izanagi_kit::zip::ZipWriter::new();
//! zw.add_stored(
//!     "doc.kml",
//!     b"<kml><Document><name>Tour</name><Placemark/></Document></kml>",
//! );
//! zw.add_stored("files/icon.png", b"\x89PNG");
//! let k = izanagi_kit::kmz::parse(&zw.finish()).unwrap();
//! assert_eq!(k.kml_name.as_deref(), Some("Tour"));
//! assert_eq!(k.resource_files, 1);
//! ```

use std::string::String;
use std::vec::Vec;

/// A detected KMZ package.
#[derive(Clone, Debug)]
pub struct Kmz {
    /// Entry names in the archive.
    pub entries: Vec<String>,
    /// The root `.kml` member that qualified the archive.
    pub kml_member: String,
    /// `<name>` text of the first `<Document>` or `<kml>` element.
    pub kml_name: Option<String>,
    /// `files/` overlay resource count (images, models).
    pub resource_files: usize,
    /// `<Placemark>` occurrences inside the KML member.
    pub placemarks: usize,
}

fn count_sub(d: &[u8], needle: &[u8]) -> usize {
    let mut n = 0usize;
    let mut i = 0usize;
    while i + needle.len() <= d.len() {
        if d[i..].starts_with(needle) {
            n += 1;
            i += needle.len();
        } else {
            i += 1;
        }
    }
    n
}

fn element(src: &[u8], tag: &[u8]) -> Option<String> {
    let open = [&b"<"[..], tag, &b">"[..]].concat();
    let close = [&b"</"[..], tag, &b">"[..]].concat();
    let start = src
        .windows(open.len())
        .position(|w| w == open.as_slice())?
        .checked_add(open.len())?;
    let rel_end = src
        .get(start..)?
        .windows(close.len())
        .position(|w| w == close.as_slice())?;
    let body = src.get(start..start + rel_end)?;
    String::from(std::str::from_utf8(body).ok()?).into()
}

/// Parse a `.kmz`: a ZIP whose members include a root-level `*.kml`
/// (bare name, no `/`). Returns `None` otherwise.
pub fn parse(d: &[u8]) -> Option<Kmz> {
    let entries = crate::zip::list(d)?;
    let mut names: Vec<String> = Vec::new();
    let mut kml_member: Option<String> = None;
    let mut resource_files = 0usize;
    for e in &entries {
        let bare = e.name.trim_start_matches('/');
        if !bare.contains('/') && bare.ends_with(".kml") && kml_member.is_none() {
            kml_member = Some(String::from(bare));
        }
        if bare.starts_with("files/") && !bare.ends_with('/') {
            resource_files += 1;
        }
        names.push(e.name.clone());
    }
    let kml_member = kml_member?;
    let body = crate::zip::extract(d, &kml_member)?;
    Some(Kmz {
        entries: names,
        kml_member,
        kml_name: element(&body, b"name"),
        resource_files,
        placemarks: count_sub(&body, b"<Placemark"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut zw = crate::zip::ZipWriter::new();
        zw.add_stored(
            "doc.kml",
            b"<kml><Document><name>Drive</name><Placemark/><Placemark/></Document></kml>",
        );
        zw.add_stored("files/a.png", b"\x89PNG");
        zw.add_stored("files/b.png", b"\x89PNG");
        zw.add_stored("files/c.png", b"\x89PNG");
        zw.finish()
    }

    #[test]
    fn fields() {
        let k = parse(&fixture()).unwrap();
        assert_eq!(k.kml_member, "doc.kml");
        assert_eq!(k.kml_name.as_deref(), Some("Drive"));
        assert_eq!(k.placemarks, 2);
        assert_eq!(k.resource_files, 3);
    }

    #[test]
    fn nested_kml_also_qualifies() {
        // a subdirectory .kml does NOT qualify (spec: root doc.kml)
        let mut zw = crate::zip::ZipWriter::new();
        zw.add_stored("sub/tour.kml", b"<kml/>");
        assert!(parse(&zw.finish()).is_none());
        let mut zw2 = crate::zip::ZipWriter::new();
        zw2.add_stored("tour.kml", b"<kml><name>T</name></kml>");
        let k = parse(&zw2.finish()).unwrap();
        assert_eq!(k.kml_member, "tour.kml");
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut zw = crate::zip::ZipWriter::new();
        zw.add_stored("readme.txt", b"hi");
        assert!(parse(&zw.finish()).is_none());
    }
}
