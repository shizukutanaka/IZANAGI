//! txt2tags markup scanner.
//!
//! txt2tags documents have a three-area structure: an optional
//! configuration area of `%!key: value` directives, an optional header
//! area (three leading lines), and the body. Body markup includes
//! `= Heading =`/`== Heading ==` titles, `+ numbered`/`+` items,
//! `-` bullet items, `**bold**`, `//italic//`, `__underline__`,
//! `` ``monospace`` ``, `..` comments, and `| table |` rows.
//!
//! ```
//! let d = b"Title\nAuthor\n\n%!encoding: utf-8\n\n= Section =\n\nA **bold** line\n";
//! let t = izanagi_kit::txt2tags::parse(d).unwrap();
//! assert_eq!(t.directives, 1);
//! assert_eq!(t.headings, 1);
//! ```
//!
//! Reference: txt2tags user guide (txt2tags.org — the `%!` directive,
//! `=…=`/`-`/`+`/`:` body-markup set).

/// Parsed txt2tags document statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct Txt2tags {
    /// `%!name:` directive count.
    pub directives: usize,
    /// `= Title =`/`== Title ==`/`=== Title ===` heading count.
    pub headings: usize,
    /// `- item` bullet count.
    pub bullets: usize,
    /// `+ item` numbered-item count.
    pub numbered: usize,
    /// `| … |` table-row count.
    pub table_rows: usize,
    /// `**…**` bold-span count.
    pub bold_spans: usize,
    /// `//…//` italic-span count.
    pub italic_spans: usize,
}

fn pairs(text: &str, open: &str) -> usize {
    let mut n = 0;
    let mut rest = text;
    while let Some(i) = rest.find(open) {
        rest = &rest[i + open.len()..];
        match rest.find(open) {
            Some(j) => {
                n += 1;
                rest = &rest[j + open.len()..];
            }
            None => break,
        }
    }
    n
}

/// Parse a txt2tags document; `None` when no txt2tags marker exists.
pub fn parse(d: &[u8]) -> Option<Txt2tags> {
    let text = core::str::from_utf8(d).ok()?;
    let mut directives = 0;
    let mut headings = 0;
    let mut bullets = 0;
    let mut numbered = 0;
    let mut table_rows = 0;
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with("%!") && l.contains(':') {
            directives += 1;
            continue;
        }
        let eq = l.bytes().take_while(|&b| b == b'=').count();
        if (1..=3).contains(&eq)
            && l.len() > eq * 2
            && l.ends_with(&"=".repeat(eq))
            && l[eq..].starts_with(' ')
        {
            headings += 1;
            continue;
        }
        if l.starts_with("- ") {
            bullets += 1;
            continue;
        }
        if l.starts_with("+ ") {
            numbered += 1;
            continue;
        }
        if l.starts_with('|') && l.ends_with('|') && l.len() > 2 {
            table_rows += 1;
        }
    }
    if directives + headings + bullets + numbered + table_rows == 0 {
        return None;
    }
    Some(Txt2tags {
        directives,
        headings,
        bullets,
        numbered,
        table_rows,
        bold_spans: pairs(text, "**"),
        italic_spans: pairs(text, "//"),
    })
}

/// `true` if the buffer looks like a txt2tags document.
pub fn detect(d: &[u8]) -> bool {
    let Some(c) = parse(d) else {
        return false;
    };
    // `- ` bullets and `* ` lists alone match every markdown-ish file;
    // txt2tags needs a distinctive family: `%!` directives, `= title =`
    // headings, `|a|b|` table rows, `+ ` numbered items, or `**`/`//`
    // spans that markdown does not use
    c.directives >= 1
        || c.headings >= 1
        || c.table_rows >= 1
        || c.numbered >= 1
        || ((c.bold_spans + c.italic_spans) >= 1
            && (c.bullets + c.numbered + c.headings + c.table_rows) >= 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] =
        b"Title\nAuthor\n\n%!encoding: utf-8\n\n= Section =\n\nA **bold** and //it// line\n- one\n+ two\n|a|b|\n";

    #[test]
    fn parses() {
        let t = parse(DOC).unwrap();
        assert_eq!(t.directives, 1);
        assert_eq!(t.headings, 1);
        assert_eq!(t.bullets, 1);
        assert_eq!(t.numbered, 1);
        assert_eq!(t.table_rows, 1);
        assert_eq!(t.bold_spans, 1);
        assert_eq!(t.italic_spans, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"plain text\nno markers\n").is_none());
        assert!(parse(&[0xff]).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"hi"));
    }
}
