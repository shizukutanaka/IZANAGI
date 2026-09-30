//! hOCR — OCR output embedded in (X)HTML: `class="ocr_page"` /
//! `ocr_line` / `ocrx_word` (and `ocr_header`/`ocr_par`/`ocr_carea`)
//! on elements whose `title` carries `bbox x0 y0 x1 y1`.
//!
//! ```
//! use izanagi_kit::hocr::{detect, parse};
//!
//! let d = b"<html><body>\
//! <div class='ocr_page' title='bbox 0 0 800 600'>\
//! <span class='ocr_line' title='bbox 0 0 100 20'>\
//! <span class='ocrx_word' title='bbox 0 0 50 20'>hi</span>\
//! </span></div></body></html>";
//! assert!(detect(d));
//! let h = parse(d).unwrap();
//! assert_eq!(h.pages, 1);
//! assert_eq!(h.words, 1);
//! ```

fn count(s: &str, pat: &str) -> u32 {
    s.matches(pat).count() as u32
}

/// Parsed hOCR census.
#[derive(Debug, Clone, PartialEq)]
pub struct Hocr {
    /// `<ocr_document>`/`<html>` wrap seen.
    pub has_html_root: bool,
    /// `ocr_page` elements.
    pub pages: u32,
    /// `ocr_carea` (column area) elements.
    pub careas: u32,
    /// `ocr_par` elements.
    pub paragraphs: u32,
    /// `ocr_line`/`ocr_textfloat` line elements.
    pub lines: u32,
    /// `ocrx_word` elements.
    pub words: u32,
    /// `bbox` occurrences in `title` attributes.
    pub bbox_count: u32,
    /// `x_wconf` occurrences (word confidence).
    pub confidences: u32,
    /// `ocr_header`/`ocr_footer`/`ocr_photo`/`ocr_table` etc combined.
    pub other_blocks: u32,
}

/// `true` on `ocr_`/`ocrx_` classes plus `bbox` geometry.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    (s.contains("ocr_page") || s.contains("ocrx_word")) && s.contains("bbox")
}

/// Census; `None` without hOCR markers.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Hocr> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    Some(Hocr {
        has_html_root: s.contains("<html") || s.contains("<ocr_document"),
        pages: count(s, "ocr_page"),
        careas: count(s, "ocr_carea"),
        paragraphs: count(s, "ocr_par"),
        lines: count(s, "ocr_line") + count(s, "ocr_textfloat"),
        words: count(s, "ocrx_word"),
        bbox_count: count(s, "bbox"),
        confidences: count(s, "x_wconf"),
        other_blocks: count(s, "ocr_header")
            + count(s, "ocr_footer")
            + count(s, "ocr_photo")
            + count(s, "ocr_table"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"<html><body>\
<div class='ocr_page' title='bbox 0 0 800 600'>\
<div class='ocr_carea' title='bbox 0 0 400 600'>\
<p class='ocr_par' title='bbox 0 0 400 100'>\
<span class='ocr_line' title='bbox 0 0 100 20'>\
<span class='ocrx_word' title='bbox 0 0 50 20 x_wconf 95'>hi</span>\
</span></p></div></div></body></html>";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"<html><body>plain</body></html>"));
    }

    #[test]
    fn parses() {
        let h = parse(D).unwrap();
        assert!(h.has_html_root);
        assert_eq!(h.pages, 1);
        assert_eq!(h.careas, 1);
        assert_eq!(h.paragraphs, 1);
        assert_eq!(h.lines, 1);
        assert_eq!(h.words, 1);
        assert_eq!(h.bbox_count, 5);
        assert_eq!(h.confidences, 1);
        assert_eq!(h.other_blocks, 0);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"<span class=ocr_page>no geometry</span>").is_none());
    }
}
