//! FictionBook (FB2) XML: `<FictionBook>` root carrying `<title-info>`
//! (`<book-title>`, nested `<author>` first/last names) and `<body>`
//! sections. Text extraction is unescaped-free and whitespace-trimmed.
//!
//! ```
//! use izanagi_kit::fb2::parse;
//!
//! let src = "<FictionBook><description><title-info>\
//!            <book-title>Novel</book-title>\
//!            <author><first-name>A</first-name><last-name>B</last-name></author>\
//!            </title-info></description><body/></FictionBook>";
//! let f = parse(src).unwrap();
//! assert_eq!(f.title.as_deref(), Some("Novel"));
//! assert_eq!(f.authors[0], "A B");
//! ```

use std::string::String;
use std::vec::Vec;

/// Parsed FB2 metadata (description only — body is left as offsets).
#[derive(Debug, Clone)]
pub struct Fb2 {
    /// `<book-title>` text.
    pub title: Option<String>,
    /// `"first last"` joined author names inside `<title-info>`.
    pub authors: Vec<String>,
    /// `<genre>` texts.
    pub genres: Vec<String>,
    /// `<lang>` text.
    pub lang: Option<String>,
}

/// Find `<name ..>text</name>` — returns the inner text (tags inside are
/// dropped) and the offset past `</name>`.
fn tag_text(src: &str, name: &str, from: usize) -> Option<(String, usize)> {
    let open = String::from("<");
    let pat = open + name;
    let i = src[from..].find(pat.as_str())? + from;
    let gt = src[i..].find('>')? + i;
    if src[..gt].ends_with('/') {
        // self-closing tag — empty text
        return Some((String::new(), gt + 1));
    }
    let close_pat = String::from("</") + name + ">";
    let j = src[gt + 1..].find(close_pat.as_str())? + gt + 1;
    let inner = strip_tags(&src[gt + 1..j]);
    Some((inner.trim().to_string(), j + close_pat.len()))
}

/// Remove nested markup from a text run (keeps text content only).
fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// Iterate `<name ..>...</name>` regions (inner slices).
fn tag_blocks(src: &str, name: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let open = String::from("<") + name;
    let close_pat = String::from("</") + name + ">";
    let mut at = 0usize;
    while let Some(i) = src[at..].find(open.as_str()).map(|p| p + at) {
        let Some(gt) = src[i..].find('>').map(|p| p + i) else {
            break;
        };
        if src[..gt].ends_with('/') {
            out.push((gt + 1, gt + 1));
            at = gt + 1;
            continue;
        }
        let Some(j) = src[gt + 1..].find(close_pat.as_str()).map(|p| p + gt + 1) else {
            break;
        };
        out.push((gt + 1, j));
        at = j + close_pat.len();
    }
    out
}

/// Parse `<title-info>`: title, authors, genres, lang.
pub fn parse(src: &str) -> Option<Fb2> {
    if !src.contains("<FictionBook") {
        return None;
    }
    let (title, _) = tag_text(src, "book-title", 0).map_or((None, 0), |(t, e)| (Some(t), e));
    let mut authors = Vec::new();
    for (a, b) in tag_blocks(src, "author") {
        let inner = &src[a..b];
        let first = tag_text(inner, "first-name", 0).map(|(t, _)| t);
        let last = tag_text(inner, "last-name", 0).map(|(t, _)| t);
        match (first, last) {
            (Some(f), Some(l)) => authors.push(format!("{f} {l}").trim().to_string()),
            (Some(f), None) | (None, Some(f)) => {
                let f = f.trim().to_string();
                if !f.is_empty() {
                    authors.push(f);
                }
            }
            (None, None) => {}
        }
    }
    let genres = tag_blocks(src, "genre")
        .into_iter()
        .map(|(a, b)| strip_tags(&src[a..b]).trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let lang = tag_text(src, "lang", 0).map(|(t, _)| t);
    Some(Fb2 {
        title,
        authors,
        genres,
        lang,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "<FictionBook><description><title-info>\
        <genre>fiction</genre><genre>sf</genre>\
        <author><first-name>Ann</first-name><last-name>Leckie</last-name></author>\
        <author><first-name>Solo</first-name></author>\
        <book-title>Ancillary Justice</book-title>\
        <lang>en</lang></title-info></description><body><section/></body></FictionBook>";

    #[test]
    fn fields() {
        let f = parse(DOC).unwrap();
        assert_eq!(f.title.as_deref(), Some("Ancillary Justice"));
        assert_eq!(f.authors, vec!["Ann Leckie", "Solo"]);
        assert_eq!(f.genres, vec!["fiction", "sf"]);
        assert_eq!(f.lang.as_deref(), Some("en"));
    }

    #[test]
    fn rejects() {
        assert!(parse("<html/>").is_none());
        let f = parse("<FictionBook/>").unwrap();
        assert!(f.title.is_none() && f.authors.is_empty());
    }
}
