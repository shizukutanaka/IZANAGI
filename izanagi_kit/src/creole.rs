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
    // Italic `//…//` is inline emphasis: the pair must sit on one line,
    // have non-empty content (`////` is nothing), and not be part of a
    // `scheme://` URL. Two `//` on different lines — e.g. two `//`-style
    // comments — used to pair across the whole file.
    let mut italic_spans = 0;
    for l in text.lines() {
        let mut rest = l;
        while let Some(i) = rest.find("//") {
            if i > 0 && rest.as_bytes()[i - 1] == b':' {
                rest = &rest[i + 2..];
                continue;
            }
            rest = &rest[i + 2..];
            match rest.find("//") {
                Some(j) if j > 0 => {
                    italic_spans += 1;
                    rest = &rest[j + 2..];
                }
                _ => break,
            }
        }
    }
    // A `[[…]]` spanning a whole trimmed line is a TOML array-of-tables
    // header, not an inline link.
    let mut links = 0;
    for l in text.lines() {
        let t = l.trim();
        if t.starts_with("[[") && t.ends_with("]]") && !t[2..t.len() - 2].contains("[[") {
            continue;
        }
        links += pairs(l, "[[", "]]");
    }
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

/// `true` if the buffer looks like a WikiCreole document. Requires at
/// least two different markup families: a lone `[[x]]` (TOML arrays),
/// a `#`-led line (config comments) or `**…**` alone are all shared
/// with other formats and do not qualify.
pub fn detect(d: &[u8]) -> bool {
    let Some(c) = parse(d) else {
        return false;
    };
    (c.headings > 0) as usize
        + (c.list_items > 0) as usize
        + (c.rules > 0) as usize
        + (c.bold_spans > 0) as usize
        + (c.italic_spans > 0) as usize
        + (c.links > 0) as usize
        + (c.images > 0) as usize
        >= 2
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
        // `//` on separate lines is not an italic pair
        assert!(!detect(b"# cfg\n// first comment\n// second comment\n"));
        // a URL scheme is not italic markup
        let c = parse(b"# cfg\nsee https://a//b\n").unwrap();
        assert_eq!(c.italic_spans, 0);
        // `[[x]]` alone on a line is a TOML array-of-tables, not a link
        let c = parse(b"# pkg\n[[package]]\nname = \"a\"\n[[package]]\nname = \"b\"\n").unwrap();
        assert_eq!(c.links, 0);
    }
}
