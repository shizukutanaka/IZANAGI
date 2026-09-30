//! OpenXPS `.xps` / `.oxps`: a ZIP package holding a Fixed Document
//! Sequence (`FixedDocSeq.fdseq` or the `[0].piece` stream layout) and
//! one or more `.fpage` fixed pages.
//!
//! ```
//! let mut zw = izanagi_kit::zip::ZipWriter::new();
//! zw.add_stored("_rels/.rels", b"<Relationships/>");
//! zw.add_stored("FixedDocSeq.fdseq", b"<FixedDocumentSequence/>");
//! zw.add_stored("Documents/1/Pages/1.fpage", b"<FixedPage/>");
//! zw.add_stored("Documents/1/Pages/2.fpage", b"<FixedPage/>");
//! let x = izanagi_kit::xps::parse(&zw.finish()).unwrap();
//! assert_eq!(x.fixed_pages, 2);
//! assert!(x.has_fdseq);
//! ```

use std::string::String;
use std::vec::Vec;

/// A detected OpenXPS package.
#[derive(Clone, Debug)]
pub struct Xps {
    /// Entry names in the archive.
    pub entries: Vec<String>,
    /// `*.fpage` fixed-page part count.
    pub fixed_pages: usize,
    /// A `FixedDocSeq.fdseq` member is present.
    pub has_fdseq: bool,
    /// A `[0].piece` stream member is present (piecewise XPS).
    pub has_pieces: bool,
    /// Members ending `.xaml` (document structure parts).
    pub xaml_parts: usize,
    /// `Documents/N/DocumentStructure/…` structure resources count.
    pub structure_parts: usize,
}

/// Parse an `.xps`/`.oxps`: the archive must be a ZIP with at least
/// one `.fpage` and either `FixedDocSeq.fdseq` or `[0].piece`
/// members. Returns `None` otherwise.
pub fn parse(d: &[u8]) -> Option<Xps> {
    let entries = crate::zip::list(d)?;
    let mut names: Vec<String> = Vec::new();
    let mut fixed_pages = 0usize;
    let mut has_fdseq = false;
    let mut has_pieces = false;
    let mut xaml_parts = 0usize;
    let mut structure_parts = 0usize;
    for e in &entries {
        let bare = e.name.trim_start_matches('/');
        if bare == "FixedDocSeq.fdseq" {
            has_fdseq = true;
        }
        if bare.ends_with("].piece") || bare.ends_with("].pieces") {
            has_pieces = true;
        }
        if bare.ends_with(".fpage") {
            fixed_pages += 1;
        }
        if bare.ends_with(".xaml") {
            xaml_parts += 1;
        }
        if bare.contains("DocumentStructure") && !bare.ends_with('/') {
            structure_parts += 1;
        }
        names.push(e.name.clone());
    }
    if fixed_pages == 0 || (!has_fdseq && !has_pieces) {
        return None;
    }
    Some(Xps {
        entries: names,
        fixed_pages,
        has_fdseq,
        has_pieces,
        xaml_parts,
        structure_parts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_fields() {
        let mut zw = crate::zip::ZipWriter::new();
        zw.add_stored("_rels/.rels", b"<R/>");
        zw.add_stored("FixedDocSeq.fdseq", b"<F/>");
        zw.add_stored("Documents/1/Pages/1.fpage", b"<FixedPage/>");
        zw.add_stored("Resources/Fonts/x.odttf", b"OTTO");
        zw.add_stored("Documents/1/DocumentStructure/1.struct", b"DS");
        let x = parse(&zw.finish()).unwrap();
        assert_eq!(x.fixed_pages, 1);
        assert!(x.has_fdseq && !x.has_pieces);
        assert_eq!(x.structure_parts, 1);
    }

    #[test]
    fn piecewise_form() {
        let mut zw = crate::zip::ZipWriter::new();
        zw.add_stored("[0].piece", b"PK");
        zw.add_stored("Documents/1/Pages/1.fpage", b"<FixedPage/>");
        let x = parse(&zw.finish()).unwrap();
        assert!(x.has_pieces);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut zw = crate::zip::ZipWriter::new();
        zw.add_stored("[Content_Types].xml", b"<>");
        zw.add_stored("word/document.xml", b"<>");
        assert!(parse(&zw.finish()).is_none()); // docx, not xps
        let mut zw2 = crate::zip::ZipWriter::new();
        zw2.add_stored("Documents/1/Pages/1.fpage", b"<>");
        assert!(parse(&zw2.finish()).is_none()); // fpage but no fdseq/piece
        let mut zw3 = crate::zip::ZipWriter::new();
        zw3.add_stored("Documents/1/Pages/1.fpage", b"<>");
        zw3.add_stored("FixedDocSeq.fdseq", b"<F/>");
        assert!(parse(&zw3.finish()).is_some());
    }
}
