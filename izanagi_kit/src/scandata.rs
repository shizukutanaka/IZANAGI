//! Internet Archive `scandata.xml` — the page-level capture log
//! written by book scanners: `<book>` (or `<scandata>`) root,
//! `<bookId>` + `<pageData>` with one `<page leafNum="…">` per leaf
//! carrying `<pageType>`/`<addToAccessFormats>`/`<handSide>` etc.
//!
//! ```
//! use izanagi_kit::scandata::{detect, parse};
//!
//! let d = b"<scandata><bookId>bk</bookId><pageData>\
//! <page leafNum=\"0\"><pageType>Cover</pageType></page>\
//! <page leafNum=\"1\"><pageType>Normal</pageType></page>\
//! </pageData></scandata>";
//! assert!(detect(d));
//! let s = parse(d).unwrap();
//! assert_eq!(s.pages, 2);
//! assert_eq!(s.leaf_nums, vec![0, 1]);
//! ```

/// Parsed IA scandata census.
#[derive(Debug, Clone, PartialEq)]
pub struct Scandata {
    /// `<bookId>` value.
    pub book_id: Option<String>,
    /// `<page leafNum>` entries.
    pub pages: u32,
    /// `leafNum` values in document order.
    pub leaf_nums: Vec<u32>,
    /// `<pageType>` values in order (`Cover`, `Title`, `Normal`, …).
    pub page_types: Vec<String>,
    /// Pages flagged `<addToAccessFormats>true</addToAccessFormats>`.
    pub accessible: u32,
    /// `<broken>`/`<delete>`-marked pages.
    pub marked: u32,
    /// `<scandata>`-rooted (newer) vs `<book>`-rooted (legacy).
    pub scandata_root: bool,
}

fn tag_text(s: &str, tag: &str) -> Option<String> {
    let i = s.find(&format!("<{tag}>"))? + tag.len() + 2;
    s[i..].find('<').map(|e| s[i..i + e].to_string())
}

/// `true` on `<scandata`/`<book` + `<pageData`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    (s.contains("<scandata") || s.contains("<book"))
        && (s.contains("<pageData") || s.contains("leafNum"))
}

/// Census; `None` without a scandata root.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Scandata> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut leaf_nums = Vec::new();
    let mut page_types = Vec::new();
    let mut accessible = 0u32;
    let mut marked = 0u32;
    for (i, _) in s.match_indices("<page ") {
        let tag_end = s[i..].find('>').map(|e| i + e);
        if let Some(te) = tag_end {
            if let Some(ln) = s[i..te].find("leafNum=\"") {
                let v = &s[i + ln + 9..te];
                if let Some(e) = v.find('"') {
                    if let Ok(n) = v[..e].parse() {
                        leaf_nums.push(n);
                    }
                }
            }
        }
        // scan forward to the matching `</page>` for per-page fields
        if let Some(pe) = s[i..].find("</page>") {
            let body = &s[i..i + pe];
            if let Some(t) = tag_text(body, "pageType") {
                page_types.push(t);
            }
            if body.contains("addToAccessFormats>true") || body.contains("<addToAccessFormats>Yes")
            {
                accessible += 1;
            }
            if body.contains("<broken") || body.contains("<delete") {
                marked += 1;
            }
        }
    }
    Some(Scandata {
        book_id: tag_text(s, "bookId"),
        pages: u32::try_from(leaf_nums.len()).unwrap_or(u32::MAX),
        leaf_nums,
        page_types,
        accessible,
        marked,
        scandata_root: s.contains("<scandata"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"<scandata><bookId>bk</bookId><pageData>\
<page leafNum=\"0\"><pageType>Cover</pageType><addToAccessFormats>true</addToAccessFormats></page>\
<page leafNum=\"1\"><pageType>Normal</pageType></page>\
<page leafNum=\"2\"><pageType>Delete</pageType><delete>true</delete></page>\
</pageData></scandata>";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"<book>no pagedata</book>"));
    }

    #[test]
    fn parses() {
        let s = parse(D).unwrap();
        assert_eq!(s.book_id.as_deref(), Some("bk"));
        assert_eq!(s.pages, 3);
        assert_eq!(s.leaf_nums, vec![0, 1, 2]);
        assert_eq!(s.page_types, vec!["Cover", "Normal", "Delete"]);
        assert_eq!(s.accessible, 1);
        assert_eq!(s.marked, 1);
        assert!(s.scandata_root);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}
