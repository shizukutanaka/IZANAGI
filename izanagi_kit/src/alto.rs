//! ALTO (Analyzed Layout and Text Object, Library of Congress
//! standard for OCR output): `<alto` root with the `…/alto` or
//! `alto-schema` xmlns, `<Description>`/`<Styles>`/`<Layout>` and
//! `<Page>` containing `<PrintSpace>` → `<TextBlock>` → `<TextLine>`
//! → `<String>`/`<SP>`/`<HYP>` word geometry.
//!
//! ```
//! use izanagi_kit::alto::{detect, parse};
//!
//! let d = b"<alto xmlns=\"x/alto\"><Layout><Page><PrintSpace>\
//! <TextBlock><TextLine><String CONTENT=\"hi\"/><SP/></TextLine>\
//! </TextBlock></PrintSpace></Page></Layout></alto>";
//! assert!(detect(d));
//! let a = parse(d).unwrap();
//! assert_eq!(a.pages, 1);
//! assert_eq!(a.strings, 1);
//! ```

fn elem_count(s: &str, name: &str) -> u32 {
    let mut n = 0u32;
    let mut i = 0usize;
    let pat = format!("<{name}");
    while let Some(off) = s[i..].find(&pat) {
        let p = i + off + pat.len();
        match s[p..].chars().next() {
            None | Some(' ' | '>' | '/' | '\t' | '\n') => n += 1,
            _ => {}
        }
        i = p;
        if i >= s.len() {
            break;
        }
    }
    n
}

/// Parsed ALTO census.
#[derive(Debug, Clone, PartialEq)]
pub struct Alto {
    /// ALTO namespace URI seen (`alto` in an `xmlns`).
    pub has_alto_ns: bool,
    /// `<Page>` elements.
    pub pages: u32,
    /// `<PrintSpace>` elements.
    pub print_spaces: u32,
    /// `<TextBlock>` elements.
    pub text_blocks: u32,
    /// `<TextLine>` elements.
    pub text_lines: u32,
    /// `<String>` word elements.
    pub strings: u32,
    /// `<SP>` space elements.
    pub spaces: u32,
    /// `<HYP>` hyphen elements.
    pub hyphens: u32,
    /// `<Illustration>` elements.
    pub illustrations: u32,
    /// `<MeasurementUnit>` element seen.
    pub has_measurement_unit: bool,
    /// First `Page` `WIDTH`/`HEIGHT` attributes.
    pub page_size: Option<(u32, u32)>,
}

fn first_page_size(s: &str) -> Option<(u32, u32)> {
    let i = s.find("<Page")?;
    let end = s[i..].find('>')? + i;
    let tag = &s[i..end];
    let w = tag
        .find("WIDTH=\"")
        .and_then(|j| tag[j + 7..].find('"').map(|e| &tag[j + 7..j + 7 + e]))
        .and_then(|v| v.parse().ok())?;
    let h = tag
        .find("HEIGHT=\"")
        .and_then(|j| tag[j + 8..].find('"').map(|e| &tag[j + 8..j + 8 + e]))
        .and_then(|v| v.parse().ok())?;
    Some((w, h))
}

/// `true` on `<alto` + `alto`-namespace or `<Layout`.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    s.contains("<alto") && (s.contains("alto-ns") || s.contains("/alto") || s.contains("<Layout"))
}

/// Census; `None` without an `<alto` root.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Alto> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    Some(Alto {
        has_alto_ns: s.contains("xmlns=\"") && (s.contains("/alto") || s.contains("alto-schema")),
        pages: elem_count(s, "Page"),
        print_spaces: elem_count(s, "PrintSpace"),
        text_blocks: elem_count(s, "TextBlock"),
        text_lines: elem_count(s, "TextLine"),
        strings: elem_count(s, "String"),
        spaces: elem_count(s, "SP"),
        hyphens: elem_count(s, "HYP"),
        illustrations: elem_count(s, "Illustration"),
        has_measurement_unit: s.contains("MeasurementUnit"),
        page_size: first_page_size(s),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"<alto xmlns=\"x/alto-ns\"><Layout><Page WIDTH=\"800\" HEIGHT=\"600\">\
<PrintSpace><TextBlock><TextLine><String CONTENT=\"hi\"/><SP/><HYP/></TextLine>\
<Illustration/></TextBlock></PrintSpace></Page></Layout></alto>";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"<xml><alto>"));
    }

    #[test]
    fn parses() {
        let a = parse(D).unwrap();
        assert!(a.has_alto_ns);
        assert_eq!(a.pages, 1);
        assert_eq!(a.print_spaces, 1);
        assert_eq!(a.text_blocks, 1);
        assert_eq!(a.text_lines, 1);
        assert_eq!(a.strings, 1);
        assert_eq!(a.spaces, 1);
        assert_eq!(a.hyphens, 1);
        assert_eq!(a.illustrations, 1);
        assert_eq!(a.page_size, Some((800, 600)));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}
