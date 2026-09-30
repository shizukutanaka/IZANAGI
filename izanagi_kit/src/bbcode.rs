//! BBCode markup scanner.
//!
//! BBCode ( bulletin-board code) uses bracketed tags: `[b]bold[/b]`,
//! `[i]`, `[u]`, `[url]`, `[img]`, `[quote]`, `[code]`, `[list]`,
//! `[size=n]`, `[color=…]`. This scanner counts well-formed
//! `open`/`close` pairs for the common tag set.
//!
//! ```
//! let d = b"Hello [b]world[/b], see [url=http://x\x2ey]link[/url] and [quote]q[/quote].";
//! let b = izanagi_kit::bbcode::parse(d).unwrap();
//! assert_eq!(b.paired, 3);
//! assert_eq!(b.tag_names.len(), 3);
//! ```
//!
//! Reference: the BBCode tag conventions documented by
//! phpBB (`docs/BBcode.html`) and bbcode.org.

/// Parsed BBCode statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct Bbcode {
    /// Number of well-formed `[tag]…[/tag]` pairs.
    pub paired: usize,
    /// Number of unclosed `[tag]` opens (tags without a `[/tag]`).
    pub unclosed: usize,
    /// Distinct lowercase tag names seen (e.g. `b`, `url`, `quote`).
    pub tag_names: Vec<String>,
}

const KNOWN: &[&str] = &[
    "b", "i", "u", "s", "url", "img", "quote", "code", "list", "size", "color", "font", "center",
    "left", "right", "table", "tr", "td", "spoiler", "video", "email",
];

/// Parse BBCode text; `None` when no bracketed BBCode tag is present.
pub fn parse(d: &[u8]) -> Option<Bbcode> {
    let text = core::str::from_utf8(d).ok()?;
    let mut paired = 0usize;
    let mut unclosed = 0usize;
    let mut tag_names: Vec<String> = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        let after = &rest[open + 1..];
        let Some(close) = after.find(']') else {
            break;
        };
        let raw = &after[..close];
        rest = &after[close + 1..];
        if raw.is_empty() || raw.starts_with('/') {
            continue;
        }
        // Tag name is up to the first `=` or whitespace inside the bracket.
        let name = raw
            .split(['=', ' ', ';'])
            .next()
            .unwrap_or("")
            .to_lowercase();
        if !KNOWN.contains(&name.as_str()) {
            continue;
        }
        if !tag_names.contains(&name) {
            tag_names.push(name.clone());
        }
        let closing = ["[/", &name, "]"].concat();
        if rest.contains(&closing) {
            paired += 1;
        } else {
            unclosed += 1;
        }
    }
    if paired + unclosed == 0 {
        return None;
    }
    Some(Bbcode {
        paired,
        unclosed,
        tag_names,
    })
}

/// `true` if the buffer looks like BBCode text.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] =
        b"Hello [b]world[/b], see [url=http://x]link[/url] and [quote]q[/quote]. [img]pic[/img] [list]items[/list]";

    #[test]
    fn parses() {
        let b = parse(DOC).unwrap();
        assert_eq!(b.paired, 5);
        assert_eq!(b.unclosed, 0);
        assert_eq!(
            b.tag_names,
            ["b", "url", "quote", "img", "list"]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn unclosed_counted() {
        let b = parse(b"start [b]never closed").unwrap();
        assert_eq!(b.paired, 0);
        assert_eq!(b.unclosed, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"no [brackets] here").is_none()); // 'brackets' is not a known tag
        assert!(parse(&[0xff]).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"plain [x]"));
    }
}
