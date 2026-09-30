//! OOXML `.vsdx` / `.vsdm` (Visio): a ZIP package that must contain
//! `[Content_Types].xml` and `visio/document.xml`.
//!
//! ```
//! let mut zw = izanagi_kit::zip::ZipWriter::new();
//! zw.add_stored("[Content_Types].xml", b"<Types/>");
//! zw.add_stored("visio/document.xml", b"<VisioDocument/>");
//! zw.add_stored("visio/pages/pages.xml", b"<Pages/>");
//! zw.add_stored("visio/pages/page1.xml", b"<PageContents/>");
//! zw.add_stored("visio/pages/page2.xml", b"<PageContents/>");
//! zw.add_stored("visio/masters/master1.xml", b"<MasterContents/>");
//! let v = izanagi_kit::vsdx::parse(&zw.finish()).unwrap();
//! assert_eq!(v.pages, 2);
//! assert_eq!(v.masters, 1);
//! ```

use std::string::String;
use std::vec::Vec;

/// A detected Visio OOXML package.
#[derive(Clone, Debug)]
pub struct Vsdx {
    /// Entry names in the archive.
    pub entries: Vec<String>,
    /// `visio/pages/pageN.xml` part count.
    pub pages: usize,
    /// `visio/masters/*.xml` part count.
    pub masters: usize,
    /// `visio/windows.xml` is present.
    pub has_windows: bool,
    /// `vbaProject.bin` marks the package macro-enabled (.vsdm).
    pub macro_enabled: bool,
    /// `<PreviewPicture` member is present.
    pub has_preview: bool,
}

/// Parse a `.vsdx`/`.vsdm`: ZIP members must include
/// `[Content_Types].xml` and `visio/document.xml`. Returns `None`
/// otherwise.
pub fn parse(d: &[u8]) -> Option<Vsdx> {
    let entries = crate::zip::list(d)?;
    let mut names: Vec<String> = Vec::new();
    let mut content_types = false;
    let mut document = false;
    let mut pages = 0usize;
    let mut masters = 0usize;
    let mut has_windows = false;
    let mut macro_enabled = false;
    let mut has_preview = false;
    for e in &entries {
        let bare = e.name.trim_start_matches('/');
        match bare {
            "[Content_Types].xml" => content_types = true,
            "visio/document.xml" => document = true,
            "visio/windows.xml" => has_windows = true,
            "docProps/thumbnail.emf" | "docProps/thumbnail.jpeg" => has_preview = true,
            "vbaProject.bin" | "visio/vbaProject.bin" => macro_enabled = true,
            _ => {}
        }
        if bare.starts_with("visio/pages/")
            && bare.ends_with(".xml")
            && bare != "visio/pages/pages.xml"
        {
            pages += 1;
        }
        if bare.starts_with("visio/masters/") && bare.ends_with(".xml") {
            masters += 1;
        }
        names.push(e.name.clone());
    }
    if !content_types || !document {
        return None;
    }
    Some(Vsdx {
        entries: names,
        pages,
        masters,
        has_windows,
        macro_enabled,
        has_preview,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_fields() {
        let mut zw = crate::zip::ZipWriter::new();
        zw.add_stored("[Content_Types].xml", b"<Types/>");
        zw.add_stored("visio/document.xml", b"<VisioDocument/>");
        zw.add_stored("visio/pages/pages.xml", b"<Pages/>");
        zw.add_stored("visio/pages/page1.xml", b"<PageContents/>");
        zw.add_stored("visio/windows.xml", b"<Windows/>");
        zw.add_stored("docProps/thumbnail.emf", b"EMF");
        let v = parse(&zw.finish()).unwrap();
        assert_eq!(v.pages, 1);
        assert!(v.has_windows && v.has_preview);
        assert!(!v.macro_enabled);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut zw = crate::zip::ZipWriter::new();
        zw.add_stored("[Content_Types].xml", b"<>");
        assert!(parse(&zw.finish()).is_none());
        let mut zw2 = crate::zip::ZipWriter::new();
        zw2.add_stored("[Content_Types].xml", b"<>");
        zw2.add_stored("visio/document.xml", b"<>");
        assert!(parse(&zw2.finish()).is_some());
    }
}
