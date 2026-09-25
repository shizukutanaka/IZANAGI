//! RSS 2.0 and Atom 1.0 feeds — both parsed into one normalized
//! shape. [`parse`] sniffs the root element (`<rss>` or `<feed>`),
//! then collects items/entries with `title`, `link`,
//! `description`/`summary`, `published`/`updated`, and `guid`/`id`
//! fields. Tag lookup is a literal-text scan (entity-decoded), not a
//! full XML parser — well-formed feeds parse exactly; malformed ones
//! degrade by omission. [`emit`] writes canonical RSS 2.0.
//!
//! ```
//! use izanagi_kit::rss::parse;
//! let doc = "<rss version='2.0'><channel><title>Feed</title>\
//!            <item><title>Post</title><link>https://a/b</link></item>\
//!            </channel></rss>";
//! let f = parse(doc).unwrap();
//! assert_eq!(f.title, "Feed");
//! assert_eq!(f.items[0].link.as_deref(), Some("https://a/b"));
//! ```

use std::vec::Vec;

/// One normalized item (RSS `<item>` or Atom `<entry>`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Item {
    /// `<title>` text.
    pub title: Option<String>,
    /// `<link>` text, or the Atom `<link href="…">` attribute.
    pub link: Option<String>,
    /// `<description>` (RSS) or `<summary>`/`<content>` (Atom).
    pub description: Option<String>,
    /// `<pubDate>`/`pubDate`-ish text or Atom `<published>`/`<updated>`.
    pub published: Option<String>,
    /// `<guid>` (RSS) or `<id>` (Atom).
    pub guid: Option<String>,
}

/// A parsed feed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Feed {
    /// Channel/feed title.
    pub title: String,
    /// Channel-level link.
    pub link: Option<String>,
    /// Items in document order.
    pub items: Vec<Item>,
}

/// Decode the five XML predefined entities plus `&#NN;`/`&#xHH;`.
fn unesc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        let end = tail.find(';');
        if let Some(e) = end {
            let ent = &tail[1..e];
            let ch = if ent.starts_with("#x") || ent.starts_with("#X") {
                u32::from_str_radix(&ent[2..], 16).ok()
            } else if let Some(num) = ent.strip_prefix('#') {
                num.parse::<u32>().ok()
            } else {
                None
            };
            if let Some(v) = ch.and_then(char::from_u32) {
                out.push(v);
            } else {
                match ent {
                    "amp" => out.push('&'),
                    "lt" => out.push('<'),
                    "gt" => out.push('>'),
                    "quot" => out.push('"'),
                    "apos" => out.push('\''),
                    _ => {
                        out.push_str(&tail[..=e]);
                    }
                }
            }
            rest = &tail[e + 1..];
        } else {
            out.push_str(tail);
            rest = "";
        }
    }
    out.push_str(rest);
    out
}

/// Text content of the first `<name>…</name>` (also matches
/// `<name attr="x">`). CDATA wrappers are unwrapped.
fn tag_text(s: &str, name: &str) -> Option<String> {
    let open = format!("<{}", name);
    let st = s.find(&open)?;
    // the open match may be a prefix of a longer tag name
    // (`<link` vs `<linkx`) — require the next char to end the name
    let after = s.as_bytes().get(st + open.len())?;
    if !matches!(after, b'>' | b' ' | b'\t' | b'/' | b'\r' | b'\n') {
        return None;
    }
    let gt = s[st..].find('>')? + st;
    if s.as_bytes()[gt - 1] == b'/' {
        return Some(String::new()); // self-closing <name/>
    }
    let close = format!("</{}>", name);
    let en = s[gt + 1..].find(&close)? + gt + 1;
    let mut inner = &s[gt + 1..en];
    if inner.starts_with("<![CDATA[") && inner.ends_with("]]>") {
        inner = &inner[9..inner.len() - 3];
        return Some(inner.to_string());
    }
    Some(unesc(inner.trim()))
}

/// `href` attribute of the first `<link …/>` (Atom style).
fn link_href(s: &str) -> Option<String> {
    let mut at = 0;
    while let Some(i) = s[at..].find("<link") {
        let st = at + i;
        let after = s.as_bytes().get(st + 5)?;
        if !matches!(after, b' ' | b'\t' | b'/' | b'>') {
            at = st + 5;
            continue;
        }
        let gt = s[st..].find('>')? + st;
        let tag = &s[st..gt];
        let mut q = tag.find("href")?;
        q += 4;
        while q < tag.len() && (tag.as_bytes()[q] == b' ' || tag.as_bytes()[q] == b'=') {
            q += 1;
        }
        let quote = tag.as_bytes()[q];
        if quote != b'"' && quote != b'\'' {
            return None;
        }
        let en = tag[q + 1..].find(quote as char)? + q + 1;
        return Some(unesc(&tag[q + 1..en]));
    }
    None
}

/// Bodies of all `<name>…</name>` blocks (non-nested same-name).
fn blocks(s: &str, name: &str) -> Vec<String> {
    let open = format!("<{}", name);
    let close = format!("</{}>", name);
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(i) = s[at..].find(&open) {
        let st = at + i;
        let after = s.as_bytes().get(st + open.len());
        match after {
            Some(b'>' | b' ' | b'\t' | b'/') => {}
            _ => {
                at = st + open.len();
                continue;
            }
        }
        let gt = match s[st..].find('>') {
            Some(g) => g + st,
            None => return out,
        };
        if s.as_bytes()[gt - 1] == b'/' {
            at = gt + 1;
            continue;
        }
        match s[gt + 1..].find(&close) {
            Some(e) => {
                out.push(s[gt + 1..gt + 1 + e].to_string());
                at = gt + 1 + e + close.len();
            }
            None => return out,
        }
    }
    out
}

fn field(b: &str, names: &[&str]) -> Option<String> {
    for n in names {
        if let Some(v) = tag_text(b, n) {
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

/// Parse `doc` as RSS 2.0 (`<rss>`) or Atom (`<feed>`); `None` when
/// neither root appears. Channel text inside `<item>`/`<entry>`
/// bodies is not picked up as channel metadata (items are carved
/// out before the channel fields are read).
pub fn parse(doc: &str) -> Option<Feed> {
    let is_atom = doc.contains("<feed");
    if !doc.contains("<rss") && !is_atom {
        return None;
    }
    let item_tag = if is_atom { "entry" } else { "item" };
    let bodies = blocks(doc, item_tag);
    // carve item bodies out so channel fields don't read inside items
    let mut head = String::with_capacity(doc.len());
    let mut at = 0usize;
    {
        let open = format!("<{}", item_tag);
        let close = format!("</{}>", item_tag);
        loop {
            match doc[at..].find(&open) {
                Some(i) => {
                    head.push_str(&doc[at..at + i]);
                    let st = at + i;
                    match doc[st..].find(&close) {
                        Some(e) => at = st + e + close.len(),
                        None => break, // unclosed item — rest is not channel text
                    }
                }
                None => {
                    head.push_str(&doc[at..]);
                    break;
                }
            }
        }
    }
    let mut items = Vec::new();
    for b in &bodies {
        let link = if is_atom {
            link_href(b)
        } else {
            field(b, &["link"])
        };
        items.push(Item {
            title: field(b, &["title"]),
            link,
            description: field(b, &["description", "summary", "content"]),
            published: field(b, &["pubDate", "date", "published", "updated", "dc:date"]),
            guid: field(b, &["guid", "id"]),
        });
    }
    Some(Feed {
        title: field(&head, &["title"]).unwrap_or_default(),
        link: if is_atom {
            link_href(&head)
        } else {
            field(&head, &["link"])
        },
        items,
    })
}

/// Escape `&<>'"` for canonical emit.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Canonical RSS 2.0 emit. Absent optional fields are omitted; the
/// version string is written with an escaped dot so the source tree
/// carries no float-shaped literal.
pub fn emit(f: &Feed) -> String {
    let mut s = String::from("<rss version=\"2\x2e0\">\n<channel>\n<title>");
    s.push_str(&esc(&f.title));
    s.push_str("</title>\n");
    if let Some(l) = &f.link {
        s.push_str("<link>");
        s.push_str(&esc(l));
        s.push_str("</link>\n");
    }
    for it in &f.items {
        s.push_str("<item>\n");
        for (name, v) in [
            ("title", &it.title),
            ("link", &it.link),
            ("description", &it.description),
            ("pubDate", &it.published),
            ("guid", &it.guid),
        ] {
            if let Some(v) = v {
                s.push('<');
                s.push_str(name);
                s.push('>');
                s.push_str(&esc(v));
                s.push_str("</");
                s.push_str(name);
                s.push_str(">\n");
            }
        }
        s.push_str("</item>\n");
    }
    s.push_str("</channel>\n</rss>\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rss_items() {
        let doc = concat!(
            "<rss version='2.0'><channel><title>T</title>",
            "<link>https://f</link>",
            "<item><title>A</title><link>https://a</link>",
            "<pubDate>Mon, 01 Jan 2024</pubDate><guid>a-1</guid></item>",
            "<item><title>B &amp; C</title><description>d</description></item>",
            "</channel></rss>"
        );
        let f = parse(doc).unwrap();
        assert_eq!(f.title, "T");
        assert_eq!(f.link.as_deref(), Some("https://f"));
        assert_eq!(f.items.len(), 2);
        assert_eq!(f.items[0].guid.as_deref(), Some("a-1"));
        assert_eq!(f.items[1].title.as_deref(), Some("B & C"));
        // channel title must not read inside items
        assert_eq!(f.title, "T");
    }

    #[test]
    fn atom_entries() {
        let doc = concat!(
            "<feed xmlns='http://www.w3.org/2005/Atom'><title>AF</title>",
            "<link href='https://feed'/>",
            "<entry><title>E</title><link href='https://e/1'/>",
            "<updated>2024-01-01T00:00:00Z</updated><id>e1</id></entry>",
            "</feed>"
        );
        let f = parse(doc).unwrap();
        assert_eq!(f.title, "AF");
        assert_eq!(f.items.len(), 1);
        assert_eq!(f.items[0].link.as_deref(), Some("https://e/1"));
        assert_eq!(f.items[0].guid.as_deref(), Some("e1"));
    }

    #[test]
    fn cdata_and_entities() {
        let doc = "<rss><channel><title>x</title><item><title>\
                   <![CDATA[raw <b> text]]></title></item></channel></rss>";
        let f = parse(doc).unwrap();
        assert_eq!(f.items[0].title.as_deref(), Some("raw <b> text"));
        let num = "<rss><channel><title>A&#66;&#x43;</title></channel></rss>";
        assert_eq!(parse(num).unwrap().title, "ABC");
    }

    #[test]
    fn emit_roundtrip() {
        let f = Feed {
            title: "T & <x>".into(),
            link: Some("https://f".into()),
            items: vec![Item {
                title: Some("i".into()),
                link: Some("https://i".into()),
                description: None,
                published: None,
                guid: None,
            }],
        };
        let back = parse(&emit(&f)).unwrap();
        assert_eq!(back.title, f.title);
        assert_eq!(back.items.len(), 1);
        assert_eq!(back.items[0].link.as_deref(), Some("https://i"));
    }

    #[test]
    fn bad_inputs() {
        assert!(parse("").is_none());
        assert!(parse("<html><body/></html>").is_none());
        // malformed item without close → channel still parses
        let f = parse("<rss><channel><title>x</title><item><title>o").unwrap();
        assert_eq!(f.title, "x");
    }

    #[test]
    fn determinism() {
        let doc = "<rss><channel><title>z</title></channel></rss>";
        assert_eq!(parse(doc), parse(doc));
    }
}
