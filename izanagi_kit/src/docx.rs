//! OOXML `.docx` / `.docm` (WordprocessingML): a ZIP package that must
//! contain `[Content_Types].xml` and `word/document.xml`.
//!
//! ```
//! let mut zw = izanagi_kit::zip::ZipWriter::new();
//! zw.add_stored("[Content_Types].xml", b"<Types/>");
//! zw.add_stored("_rels/.rels", b"<Relationships/>");
//! zw.add_stored(
//!     "word/document.xml",
//!     b"<w:document><w:body><w:p><w:r><w:t>hi</w:t></w:r></w:p><w:p/></w:body></w:document>",
//! );
//! zw.add_stored("word/styles.xml", b"<w:styles/>");
//! zw.add_stored("word/media/img1.png", b"\x89PNG");
//! let d = izanagi_kit::docx::parse(&zw.finish()).unwrap();
//! assert_eq!(d.paragraphs, 2);
//! assert!(d.has_styles);
//! assert!(!d.macro_enabled);
//! ```

use std::string::String;
use std::vec::Vec;

/// A detected WordprocessingML package.
#[derive(Clone, Debug)]
pub struct Docx {
    /// Entry names in the archive.
    pub entries: Vec<String>,
    /// `<w:p` paragraph start count inside `word/document.xml`.
    pub paragraphs: usize,
    /// `word/styles.xml` is present.
    pub has_styles: bool,
    /// `docProps/core.xml` is present.
    pub has_core_props: bool,
    /// `word/vbaProject.bin` marks the package macro-enabled (.docm).
    pub macro_enabled: bool,
    /// Files stored under `word/media/`.
    pub media_files: usize,
    /// `<dc:title>` text from `docProps/core.xml` when present.
    pub title: Option<String>,
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

/// Parse a `.docx`/`.docm`: ZIP members must include
/// `[Content_Types].xml` and `word/document.xml`. Returns `None`
/// otherwise.
pub fn parse(d: &[u8]) -> Option<Docx> {
    let entries = crate::zip::list(d)?;
    let mut names: Vec<String> = Vec::new();
    let mut content_types = false;
    let mut document = false;
    let mut has_styles = false;
    let mut has_core_props = false;
    let mut macro_enabled = false;
    let mut media_files = 0usize;
    for e in &entries {
        let bare = e.name.trim_start_matches('/');
        match bare {
            "[Content_Types].xml" => content_types = true,
            "word/document.xml" => document = true,
            "word/styles.xml" => has_styles = true,
            "docProps/core.xml" => has_core_props = true,
            "word/vbaProject.bin" => macro_enabled = true,
            _ => {}
        }
        if bare.starts_with("word/media/") && !bare.ends_with('/') {
            media_files += 1;
        }
        names.push(e.name.clone());
    }
    if !content_types || !document {
        return None;
    }
    let body = crate::zip::extract(d, "word/document.xml")?;
    let title = if has_core_props {
        crate::zip::extract(d, "docProps/core.xml").and_then(|core| element(&core, b"dc:title"))
    } else {
        None
    };
    Some(Docx {
        entries: names,
        paragraphs: count_sub(&body, b"<w:p"),
        has_styles,
        has_core_props,
        macro_enabled,
        media_files,
        title,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut zw = crate::zip::ZipWriter::new();
        zw.add_stored("[Content_Types].xml", b"<Types/>");
        zw.add_stored(
            "word/document.xml",
            b"<w:document><w:body><w:p/><w:p/><w:p/></w:body></w:document>",
        );
        zw.add_stored("word/styles.xml", b"<w:styles/>");
        zw.add_stored("word/media/a.png", b"\x89PNG");
        zw.add_stored("word/media/b.png", b"\x89PNG");
        zw.add_stored("docProps/core.xml", b"<cp><dc:title>T</dc:title></cp>");
        zw.finish()
    }

    #[test]
    fn package_fields() {
        let d = parse(&fixture()).unwrap();
        assert_eq!(d.paragraphs, 3);
        assert!(d.has_styles && d.has_core_props);
        assert_eq!(d.media_files, 2);
        assert_eq!(d.title.as_deref(), Some("T"));
        assert!(!d.macro_enabled);
    }

    #[test]
    fn macro_flag() {
        let mut zw = crate::zip::ZipWriter::new();
        zw.add_stored("[Content_Types].xml", b"<Types/>");
        zw.add_stored("word/document.xml", b"<w:document/>");
        zw.add_stored("word/vbaProject.bin", b"\xd0\xcf");
        let d = parse(&zw.finish()).unwrap();
        assert!(d.macro_enabled);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut zw = crate::zip::ZipWriter::new();
        zw.add_stored("[Content_Types].xml", b"<>");
        assert!(parse(&zw.finish()).is_none()); // no document.xml
        let mut zw2 = crate::zip::ZipWriter::new();
        zw2.add_stored("[Content_Types].xml", b"<>");
        zw2.add_stored("word/document.xml", b"<>");
        assert!(parse(&zw2.finish()).is_some());
    }
}
