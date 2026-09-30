//! Atom (RFC 4287) — `<feed xmlns="http://www.w3.org/2005/Atom">` with
//! `<entry>` items, `<title>`/`<link>`/`<id>`/`<updated>` per feed and
//! entry, `<content>`/`summary`/`author` and `type=`/`rel=`/`href` attrs.
//!
//! ```
//! let d = b"<feed xmlns=\"http://www.w3.org/2005/Atom\"><title>t</title><entry><id>x</id></entry></feed>";
//! let a = izanagi_kit::atom::parse(d).unwrap();
//! assert_eq!(a.entries, 1);
//! assert_eq!(a.titles, 1);
//! assert!(izanagi_kit::atom::detect(d));
//! ```

/// Census of an Atom document.
#[derive(Debug, Clone)]
pub struct Atom {
    /// Root is `<feed`.
    pub feed: bool,
    /// Root is `<entry` (standalone entry document).
    pub standalone_entry: bool,
    /// Atom namespace declared.
    pub ns: bool,
    /// `<entry` count.
    pub entries: usize,
    /// `<title` count.
    pub titles: usize,
    /// `<id` count.
    pub ids: usize,
    /// `<updated` count.
    pub updated: usize,
    /// `<link` elements.
    pub links: usize,
    /// `rel="self"|"alternate"|"edit"|"enclosure"` link rels.
    pub rels: usize,
    /// `<author` blocks.
    pub authors: usize,
    /// `<content` blocks.
    pub contents: usize,
    /// `<summary` blocks.
    pub summaries: usize,
    /// `<category` tags.
    pub categories: usize,
    /// `<published` timestamps.
    pub published: usize,
    /// `<generator` / `<icon` / `<logo` / `<subtitle` feed misc.
    pub misc: usize,
}

fn count_open(t: &str, tag: &str) -> usize {
    let mut n = 0usize;
    let mut from = 0;
    let needle = ["<", tag].concat();
    while let Some(p) = t[from..].find(&needle) {
        let a = from + p;
        let after = t[a + needle.len()..]
            .chars()
            .next()
            .map_or(true, |c| !c.is_alphanumeric() && c != ':');
        if after && !t[..a].ends_with("</") {
            n += 1;
        }
        from = a + needle.len();
    }
    n
}

/// Detects Atom: `<feed` or `<entry` root + `2005/Atom` namespace.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    b.windows(5).any(|w| w == b"<feed") && b.windows(9).any(|w| w == b"2005/Atom")
}

/// Parses an Atom document; `None` without feed/entry + namespace.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Atom> {
    if !detect(b) {
        return None;
    }
    let t = String::from_utf8_lossy(b);
    let rels = ["self", "alternate", "edit", "enclosure", "related"]
        .iter()
        .map(|r| t.matches(&["rel=\"", r, "\""].concat()).count())
        .sum();
    Some(Atom {
        feed: t.contains("<feed"),
        standalone_entry: !t.contains("<feed") && t.contains("<entry"),
        ns: t.contains("2005/Atom"),
        entries: count_open(&t, "entry"),
        titles: count_open(&t, "title"),
        ids: count_open(&t, "id"),
        updated: count_open(&t, "updated"),
        links: count_open(&t, "link"),
        rels,
        authors: count_open(&t, "author"),
        contents: count_open(&t, "content"),
        summaries: count_open(&t, "summary"),
        categories: count_open(&t, "category"),
        published: count_open(&t, "published"),
        misc: count_open(&t, "generator")
            + count_open(&t, "icon")
            + count_open(&t, "logo")
            + count_open(&t, "subtitle"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] = b"<feed xmlns=\"http://www.w3.org/2005/Atom\"><title>t</title><id>i</id><updated>2024-01-01T00:00:00Z</updated><link rel=\"self\" href=\"u\"/><author><name>n</name></author><entry><title>e</title><id>x</id><content>c</content><category term=\"k\"/></entry></feed>";

    #[test]
    fn parses() {
        let a = parse(D).unwrap();
        assert!(a.feed && a.ns);
        assert_eq!(a.entries, 1);
        assert_eq!(a.titles, 2);
        assert_eq!(a.links, 1);
        assert_eq!(a.rels, 1);
        assert_eq!(a.authors, 1);
        assert_eq!(a.contents, 1);
        assert_eq!(a.categories, 1);
    }

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"<feed xmlns=\"http://www.w3.org/1999/rss\"/>"));
        assert!(!detect(b""));
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
    }
}
