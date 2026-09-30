//! WikiCreole markup scanner.
//!
//! WikiCreole 1.x: headings are `= text` … `====== text` (leading `=`
//! count = level), lists use `*`/`#` bullets, bold is `**…**`, italic is
//! `//…//`, links are `[[target|label]]`, images `{{src|alt}}`, and
//! horizontal rules are `----` on their own line.
//!
//! ```
//! let d = b"== Heading ==\n\nA **bold** and //em// word, [[Page|link]].\n\n----\n* item\n";
//! let c = izanagi_kit::creole::parse(d).unwrap();
//! assert_eq!(c.headings, 1);
//! assert_eq!(c.bold_spans, 1);
//! assert_eq!(c.links, 1);
//! assert_eq!(c.rules, 1);
//! assert_eq!(c.list_items, 1);
//! ```
//!
//! Reference: WikiCreole 1\x2e0 specification (wikicreole.org).

/// Parsed WikiCreole document statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct Creole {
    /// `=`-headed heading line count.
    pub headings: usize,
    /// `*`/`#` list-item line count.
    pub list_items: usize,
    /// `----` horizontal-rule line count.
    pub rules: usize,
    /// `**…**` bold-span count.
    pub bold_spans: usize,
    /// `//…//` italic-span count.
    pub italic_spans: usize,
    /// `[[…]]` link count.
    pub links: usize,
    /// `{{…}}` image/inclusion count.
    pub images: usize,
}

fn pairs(text: &str, open: &str, close: &str) -> usize {
    let mut n = 0;
    let mut rest = text;
    while let Some(i) = rest.find(open) {
        rest = &rest[i + open.len()..];
        match rest.find(close) {
            Some(j) => {
                n += 1;
                rest = &rest[j + close.len()..];
            }
            None => break,
        }
    }
    n
}

/// Parse a WikiCreole document; `None` when no Creole markup exists.
pub fn parse(d: &[u8]) -> Option<Creole> {
    let text = core::str::from_utf8(d).ok()?;
    let mut headings = 0;
    let mut list_items = 0;
    let mut rules = 0;
    for line in text.lines() {
        let l = line.trim_end();
        // headings: a leading run of 1..=6 '=' followed by a space
        let eq = l.bytes().take_while(|&b| b == b'=').count();
        if (1..=6).contains(&eq) && l[eq..].starts_with(' ') {
            headings += 1;
        }
        if l.starts_with('*') || l.starts_with('#') {
            list_items += 1;
        }
        if l.trim() == "----" || l.trim_start_matches('-').is_empty() && l.len() >= 4 {
            rules += 1;
        }
    }
    let bold_spans = pairs(text, "**", "**");
    let italic_spans = pairs(text, "//", "//");
    let links = pairs(text, "[[", "]]");
    let images = pairs(text, "{{", "}}");
    if headings + list_items + rules + bold_spans + italic_spans + links + images == 0 {
        return None;
    }
    Some(Creole {
        headings,
        list_items,
        rules,
        bold_spans,
        italic_spans,
        links,
        images,
    })
}

/// `true` if the buffer looks like a WikiCreole document.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] =
        b"== Heading ==\n\nA **bold** and //em// word, [[Page|link]] and {{img.png|alt}}.\n\n----\n* one\n# two\n";

    #[test]
    fn parses() {
        let c = parse(DOC).unwrap();
        assert_eq!(c.headings, 1);
        assert_eq!(c.bold_spans, 1);
        assert_eq!(c.italic_spans, 1);
        assert_eq!(c.links, 1);
        assert_eq!(c.images, 1);
        assert_eq!(c.rules, 1);
        assert_eq!(c.list_items, 2);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"plain text without markup\n").is_none());
        assert!(parse(&[0xff]).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"hello"));
    }
}
