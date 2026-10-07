//! MediaWiki wikitext scanner.
//!
//! Wikitext markers: `== Heading ==` … `====== Heading ======` (level =
//! number of `=` on each side), `'''bold'''`/`''italic''`, `[[internal
//! link]]`, `[http://… external]`, `{{template}}`, `* `/`# ` list items,
//! `: ` indents, `----` rules, and `[[Category:…]]`/`[[File:…]]`.
//!
//! ```
//! let d = b"== Section ==\n\nA '''bold''' word, [[Page|link]] and {{T|x}}.\n\n----\n* item\n";
//! let w = izanagi_kit::mediawiki::parse(d).unwrap();
//! assert_eq!(w.headings, 1);
//! assert_eq!(w.bold_spans, 1);
//! assert_eq!(w.internal_links, 1);
//! assert_eq!(w.templates, 1);
//! ```
//!
//! Reference: MediaWiki markup spec (mediawiki.org/wiki/Wikitext —
//! heading, list, link, template conventions).

/// Parsed wikitext statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct Mediawiki {
    /// `== … ==` heading count.
    pub headings: usize,
    /// `'''…'''` bold count.
    pub bold_spans: usize,
    /// `''…''` italic count.
    pub italic_spans: usize,
    /// `[[…]]` internal links (including `Category:`/`File:` links).
    pub internal_links: usize,
    /// `{{…}}` template transclusions.
    pub templates: usize,
    /// `[http… ]` external links.
    pub external_links: usize,
    /// `*`/`#`/`:`/`*`-led list-item lines.
    pub list_items: usize,
    /// `----` rule count.
    pub rules: usize,
    /// `[[Category:…]]` category links.
    pub categories: usize,
    /// `[[File:…]]`/`[[Image:…]]` media links.
    pub media: usize,
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

/// Parse wikitext; `None` when no wikitext marker exists.
pub fn parse(d: &[u8]) -> Option<Mediawiki> {
    let text = core::str::from_utf8(d).ok()?;
    let mut headings = 0;
    let mut list_items = 0;
    let mut rules = 0;
    for line in text.lines() {
        let l = line.trim();
        let lead = l.bytes().take_while(|&b| b == b'=').count();
        let trail = l.bytes().rev().take_while(|&b| b == b'=').count();
        if (2..=6).contains(&lead)
            && lead == trail
            && l.len() > lead * 2
            && l[lead..].starts_with(' ')
        {
            headings += 1;
            continue;
        }
        if l.starts_with('*') || l.starts_with(':') {
            list_items += 1;
            continue;
        }
        if l.len() >= 4 && l.bytes().all(|b| b == b'-') {
            rules += 1;
        }
    }
    let internal_links = pairs(text, "[[", "]]");
    let categories = text.matches("[[Category:").count();
    let media = text.matches("[[File:").count() + text.matches("[[Image:").count();
    let external_links = {
        let mut n = 0;
        let mut rest = text;
        while let Some(i) = rest.find("[http") {
            rest = &rest[i + 1..];
            if rest.find(']').is_some() {
                n += 1;
            } else {
                break;
            }
        }
        n
    };
    // Bold spans must not leak into the italic count: strip `'''…'''`
    // regions before scanning for `''…''`.
    let no_bold = text.replace("'''", "");
    let mw = Mediawiki {
        headings,
        bold_spans: pairs(text, "'''", "'''"),
        italic_spans: pairs(&no_bold, "''", "''"),
        internal_links,
        templates: pairs(text, "{{", "}}"),
        external_links,
        list_items,
        rules,
        categories,
        media,
    };
    // Require a MediaWiki-specific marker (`== H ==`, `[[..]]`, `[http..]`,
    // `[[Category:`/`[[File:`) — bare `{{ }}`/`''` spans, `*` bullets, and
    // `----` rules show up in Helm/Python/markdown too.
    if mw.headings + mw.internal_links + mw.external_links + categories + media == 0 {
        return None;
    }
    Some(mw)
}

/// `true` if the buffer looks like wikitext.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"== Section ==\n\nA '''bold''' and ''it'' word, [[Page|link]] and {{T|x}}.\nSee [http://e\x2ex/ ext].\n\n----\n* item\n[[Category:C]]\n";

    #[test]
    fn parses() {
        let w = parse(DOC).unwrap();
        assert_eq!(w.headings, 1);
        assert_eq!(w.bold_spans, 1);
        assert_eq!(w.italic_spans, 1);
        assert_eq!(w.internal_links, 2);
        assert_eq!(w.templates, 1);
        assert_eq!(w.external_links, 1);
        assert_eq!(w.list_items, 1);
        assert_eq!(w.rules, 1);
        assert_eq!(w.categories, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"plain text\nno markup\n").is_none());
        assert!(parse(&[0xff]).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"hi"));
        assert!(!detect(b"# comment\n* bullet\n{{jinja}}\n"));
    }
}
