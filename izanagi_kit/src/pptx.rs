//! OOXML `.pptx` / `.pptm` (PresentationML): a ZIP package that must
//! contain `[Content_Types].xml` and `ppt/presentation.xml`.
//!
//! ```
//! let mut zw = izanagi_kit::zip::ZipWriter::new();
//! zw.add_stored("[Content_Types].xml", b"<Types/>");
//! zw.add_stored("ppt/presentation.xml", b"<p:presentation><p:sldSz cx=\"9144000\" cy=\"6858000\"/></p:presentation>");
//! zw.add_stored("ppt/slides/slide1.xml", b"<p:sld/>");
//! zw.add_stored("ppt/slides/slide2.xml", b"<p:sld/>");
//! zw.add_stored("ppt/slideMasters/slideMaster1.xml", b"<p:sldMaster/>");
//! zw.add_stored("ppt/media/logo.png", b"\x89PNG");
//! let p = izanagi_kit::pptx::parse(&zw.finish()).unwrap();
//! assert_eq!(p.slides, 2);
//! assert_eq!(p.masters, 1);
//! assert_eq!(p.slide_size, Some((9144000, 6858000)));
//! ```

use std::string::String;
use std::vec::Vec;

/// A detected PresentationML package.
#[derive(Clone, Debug)]
pub struct Pptx {
    /// Entry names in the archive.
    pub entries: Vec<String>,
    /// `ppt/slides/slideN.xml` part count.
    pub slides: usize,
    /// `ppt/slideMasters/*.xml` part count.
    pub masters: usize,
    /// `ppt/notesSlides/*.xml` part count.
    pub notes_slides: usize,
    /// Files stored under `ppt/media/`.
    pub media_files: usize,
    /// `<p:sldSz cx cy>` extents in EMUs, when present.
    pub slide_size: Option<(u64, u64)>,
    /// `vbaProject.bin` marks the package macro-enabled (.pptm).
    pub macro_enabled: bool,
}

fn u64_attr(tag: &[u8], key: &[u8]) -> Option<u64> {
    let mut k: Vec<u8> = Vec::new();
    k.extend_from_slice(key);
    k.extend_from_slice(b"=\"");
    let at = tag.windows(k.len()).position(|w| w == k.as_slice())? + k.len();
    let mut v = 0u64;
    let mut i = at;
    let mut any = false;
    while i < tag.len() && tag[i].is_ascii_digit() {
        v = v.checked_mul(10)?.checked_add(u64::from(tag[i] - b'0'))?;
        i += 1;
        any = true;
    }
    if any {
        Some(v)
    } else {
        None
    }
}

/// Parse a `.pptx`/`.pptm`: ZIP members must include
/// `[Content_Types].xml` and `ppt/presentation.xml`. Returns `None`
/// otherwise.
pub fn parse(d: &[u8]) -> Option<Pptx> {
    let entries = crate::zip::list(d)?;
    let mut names: Vec<String> = Vec::new();
    let mut content_types = false;
    let mut presentation = false;
    let mut slides = 0usize;
    let mut masters = 0usize;
    let mut notes_slides = 0usize;
    let mut media_files = 0usize;
    let mut macro_enabled = false;
    for e in &entries {
        let bare = e.name.trim_start_matches('/');
        match bare {
            "[Content_Types].xml" => content_types = true,
            "ppt/presentation.xml" => presentation = true,
            "vbaProject.bin" | "ppt/vbaProject.bin" => macro_enabled = true,
            _ => {}
        }
        if bare.starts_with("ppt/slides/") && bare.ends_with(".xml") {
            slides += 1;
        } else if bare.starts_with("ppt/slideMasters/") && bare.ends_with(".xml") {
            masters += 1;
        } else if bare.starts_with("ppt/notesSlides/") && bare.ends_with(".xml") {
            notes_slides += 1;
        }
        if bare.starts_with("ppt/media/") && !bare.ends_with('/') {
            media_files += 1;
        }
        names.push(e.name.clone());
    }
    if !content_types || !presentation {
        return None;
    }
    let doc = crate::zip::extract(d, "ppt/presentation.xml")?;
    let slide_size = doc.windows(8).position(|w| w == b"<p:sldSz").and_then(|i| {
        let end = doc[i..]
            .iter()
            .position(|&b| b == b'>')
            .map(|e| i + e + 1)
            .unwrap_or(doc.len());
        Some((
            u64_attr(&doc[i..end], b"cx")?,
            u64_attr(&doc[i..end], b"cy")?,
        ))
    });
    Some(Pptx {
        entries: names,
        slides,
        masters,
        notes_slides,
        media_files,
        slide_size,
        macro_enabled,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        let mut zw = crate::zip::ZipWriter::new();
        zw.add_stored("[Content_Types].xml", b"<Types/>");
        zw.add_stored(
            "ppt/presentation.xml",
            b"<p:presentation><p:sldSz cx=\"12192000\" cy=\"6858000\"/></p:presentation>",
        );
        zw.add_stored("ppt/slides/slide1.xml", b"<p:sld/>");
        zw.add_stored("ppt/slides/slide2.xml", b"<p:sld/>");
        zw.add_stored("ppt/slides/slide3.xml", b"<p:sld/>");
        zw.add_stored("ppt/slideMasters/slideMaster1.xml", b"<p:m/>");
        zw.add_stored("ppt/notesSlides/notesSlide1.xml", b"<p:notes/>");
        zw.add_stored("ppt/media/a.png", b"\x89PNG");
        zw.finish()
    }

    #[test]
    fn package_fields() {
        let p = parse(&fixture()).unwrap();
        assert_eq!(p.slides, 3);
        assert_eq!(p.masters, 1);
        assert_eq!(p.notes_slides, 1);
        assert_eq!(p.media_files, 1);
        assert_eq!(p.slide_size, Some((12192000, 6858000)));
        assert!(!p.macro_enabled);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut zw = crate::zip::ZipWriter::new();
        zw.add_stored("[Content_Types].xml", b"<>");
        zw.add_stored("word/document.xml", b"<>");
        assert!(parse(&zw.finish()).is_none());
        let mut zw2 = crate::zip::ZipWriter::new();
        zw2.add_stored("[Content_Types].xml", b"<>");
        zw2.add_stored("ppt/presentation.xml", b"<p:presentation/>");
        let z = zw2.finish();
        assert!(parse(&z).is_some());
        assert_eq!(parse(&z).unwrap().slide_size, None);
    }
}
