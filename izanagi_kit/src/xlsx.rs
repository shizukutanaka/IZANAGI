//! OOXML `.xlsx` / `.xlsm` (SpreadsheetML): a ZIP package that must
//! contain `[Content_Types].xml` and `xl/workbook.xml`.
//!
//! ```
//! let mut zw = izanagi_kit::zip::ZipWriter::new();
//! zw.add_stored("[Content_Types].xml", b"<Types/>");
//! zw.add_stored(
//!     "xl/workbook.xml",
//!     b"<workbook><sheets><sheet name=\"Sh1\" sheetId=\"1\"/><sheet name=\"Data\"/></sheets></workbook>",
//! );
//! zw.add_stored("xl/worksheets/sheet1.xml", b"<worksheet/>");
//! zw.add_stored("xl/worksheets/sheet2.xml", b"<worksheet/>");
//! zw.add_stored("xl/sharedStrings.xml", b"<sst/>");
//! let x = izanagi_kit::xlsx::parse(&zw.finish()).unwrap();
//! assert_eq!(x.sheets, vec!["Sh1".to_string(), "Data".to_string()]);
//! assert!(x.has_shared_strings);
//! ```

use std::string::String;
use std::vec::Vec;

/// A detected SpreadsheetML package.
#[derive(Clone, Debug)]
pub struct Xlsx {
    /// Entry names in the archive.
    pub entries: Vec<String>,
    /// `<sheet name="…">` names in workbook order.
    pub sheets: Vec<String>,
    /// `xl/worksheets/*.xml` part count.
    pub sheet_parts: usize,
    /// `xl/sharedStrings.xml` is present.
    pub has_shared_strings: bool,
    /// `xl/styles.xml` is present.
    pub has_styles: bool,
    /// `xl/calcChain.xml` is present (formula dependency cache).
    pub has_calc_chain: bool,
    /// `vbaProject.bin` marks the package macro-enabled (.xlsm).
    pub macro_enabled: bool,
}

fn sheet_names(wb: &[u8]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let needle = b"<sheet";
    let key = b"name=\"";
    let mut i = 0usize;
    while i + needle.len() <= wb.len() {
        if !wb[i..].starts_with(needle) {
            i += 1;
            continue;
        }
        // scan the tag for `name="…"`
        let mut j = i + needle.len();
        let mut name: Option<String> = None;
        while j < wb.len() && wb[j] != b'>' {
            if wb[j..].starts_with(key) {
                let vs = j + key.len();
                let mut e = vs;
                while e < wb.len() && wb[e] != b'"' {
                    e += 1;
                }
                if let Some(raw) = wb.get(vs..e) {
                    if let Ok(t) = std::str::from_utf8(raw) {
                        name = Some(String::from(t));
                    }
                }
                j = e;
            } else {
                j += 1;
            }
        }
        if let Some(n) = name {
            out.push(n);
        }
        i = j;
    }
    out
}

/// Parse a `.xlsx`/`.xlsm`: ZIP members must include
/// `[Content_Types].xml` and `xl/workbook.xml`. Returns `None`
/// otherwise.
pub fn parse(d: &[u8]) -> Option<Xlsx> {
    let entries = crate::zip::list(d)?;
    let mut names: Vec<String> = Vec::new();
    let mut content_types = false;
    let mut workbook = false;
    let mut sheet_parts = 0usize;
    let mut has_shared_strings = false;
    let mut has_styles = false;
    let mut has_calc_chain = false;
    let mut macro_enabled = false;
    for e in &entries {
        let bare = e.name.trim_start_matches('/');
        match bare {
            "[Content_Types].xml" => content_types = true,
            "xl/workbook.xml" => workbook = true,
            "xl/sharedStrings.xml" => has_shared_strings = true,
            "xl/styles.xml" => has_styles = true,
            "xl/calcChain.xml" => has_calc_chain = true,
            "vbaProject.bin" | "xl/vbaProject.bin" => macro_enabled = true,
            _ => {}
        }
        if bare.starts_with("xl/worksheets/") && bare.ends_with(".xml") && !bare.ends_with('/') {
            sheet_parts += 1;
        }
        names.push(e.name.clone());
    }
    if !content_types || !workbook {
        return None;
    }
    let wb = crate::zip::extract(d, "xl/workbook.xml")?;
    Some(Xlsx {
        entries: names,
        sheets: sheet_names(&wb),
        sheet_parts,
        has_shared_strings,
        has_styles,
        has_calc_chain,
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
            "xl/workbook.xml",
            b"<workbook><sheets><sheet name=\"A\"/><sheet name=\"B\"/></sheets></workbook>",
        );
        zw.add_stored("xl/worksheets/sheet1.xml", b"<worksheet/>");
        zw.add_stored("xl/worksheets/sheet2.xml", b"<worksheet/>");
        zw.add_stored("xl/worksheets/_rels/sheet1.xml.rels", b"<R/>");
        zw.add_stored("xl/styles.xml", b"<styleSheet/>");
        zw.add_stored("xl/calcChain.xml", b"<calcChain/>");
        zw.finish()
    }

    #[test]
    fn workbook_fields() {
        let x = parse(&fixture()).unwrap();
        assert_eq!(x.sheets, vec![String::from("A"), String::from("B")]);
        assert_eq!(x.sheet_parts, 2); // rels under _rels/ end in .rels
        assert!(x.has_styles && x.has_calc_chain);
        assert!(!x.has_shared_strings && !x.macro_enabled);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        let mut zw = crate::zip::ZipWriter::new();
        zw.add_stored("[Content_Types].xml", b"<>");
        zw.add_stored("word/document.xml", b"<>");
        assert!(parse(&zw.finish()).is_none()); // docx, not xlsx
        let mut zw2 = crate::zip::ZipWriter::new();
        zw2.add_stored("[Content_Types].xml", b"<>");
        zw2.add_stored("xl/workbook.xml", b"<workbook/>");
        assert!(parse(&zw2.finish()).is_some());
    }
}
