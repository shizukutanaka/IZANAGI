//! Sigma generic detection rule scanner (YAML form).
//!
//! A Sigma rule is a YAML document with top-level keys `title`,
//! `id`, `status`, `description`, `author`, `date`,
//! `logsource:` (product/service/category), `detection:`
//! (selection + condition), `level` and `tags`. This scanner reads
//! only top-level `key:` markers plus the well-known scalar values
//! — a full YAML parse is intentionally out of scope.
//!
//! ```
//! let d = b"title: Suspicious x\nid: 1f3e2d4c-0000-0000-8000-000000000000\nstatus: test\nlogsource:\n  product: windows\ndetection:\n  sel:\n    a: 1\n  condition: sel\nlevel: high\n";
//! let s = izanagi_kit::sigma::parse(d).unwrap();
//! assert_eq!(s.title.as_deref(), Some("Suspicious x"));
//! assert_eq!(s.level.as_deref(), Some("high"));
//! ```
//!
//! Reference: Sigma specification (sigmahq.io) — required `title` +
//! `detection` and the `logsource`/`level`/`tags` metadata keys.

/// Parsed Sigma rule fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Sigma {
    /// `title:` scalar.
    pub title: Option<String>,
    /// `id:` scalar (UUID form in practice).
    pub id: Option<String>,
    /// `status:` scalar (`experimental`, `test`, `stable`, …).
    pub status: Option<String>,
    /// `level:` scalar (`informational` … `critical`).
    pub level: Option<String>,
    /// `author:` scalar.
    pub author: Option<String>,
    /// `true` when a `logsource:` key exists.
    pub has_logsource: bool,
    /// `true` when a `detection:` key exists.
    pub has_detection: bool,
    /// `tags:` list item count (`- name` entries after `tags:`).
    pub tags: usize,
}

fn top_key<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().find_map(|l| {
        let l = l.trim_end();
        if l.starts_with(' ') || l.starts_with('-') || l.starts_with('#') {
            return None;
        }
        l.strip_prefix(key)?.strip_prefix(':')?;
        Some(l[key.len() + 1..].trim())
    })
}

/// Parse a Sigma YAML rule; `None` unless `detection:` plus at
/// least one of `title:`/`logsource:` is present.
pub fn parse(d: &[u8]) -> Option<Sigma> {
    let text = core::str::from_utf8(d).ok()?;
    let has_detection = top_key(text, "detection").is_some();
    let title = top_key(text, "title").map(|s| s.to_string());
    let has_logsource = top_key(text, "logsource").is_some();
    if !has_detection || (title.is_none() && !has_logsource) {
        return None;
    }
    // count list items directly under `tags:`
    let mut tags = 0usize;
    let mut in_tags = false;
    for l in text.lines() {
        if !l.starts_with(' ') && !l.starts_with('-') {
            in_tags = l.trim_end() == "tags:";
            continue;
        }
        if in_tags && l.trim_start().starts_with('-') {
            tags += 1;
        } else if in_tags && !l.trim().is_empty() {
            in_tags = false;
        }
    }
    Some(Sigma {
        title,
        id: top_key(text, "id").map(|s| s.to_string()),
        status: top_key(text, "status").map(|s| s.to_string()),
        level: top_key(text, "level").map(|s| s.to_string()),
        author: top_key(text, "author").map(|s| s.to_string()),
        has_logsource,
        has_detection,
        tags,
    })
}

/// `true` if the buffer looks like a Sigma rule.
pub fn detect(d: &[u8]) -> bool {
    parse(d).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &[u8] = b"title: Suspicious x\nid: 1f3e2d4c-0000-0000-8000-000000000000\nstatus: test\nauthor: j\nlogsource:\n  product: windows\ndetection:\n  sel:\n    a: 1\n  condition: sel\nlevel: high\ntags:\n  - attack.t1059\n  - attack.execution\n";

    #[test]
    fn parses() {
        let s = parse(DOC).unwrap();
        assert_eq!(s.title.as_deref(), Some("Suspicious x"));
        assert_eq!(s.status.as_deref(), Some("test"));
        assert_eq!(s.level.as_deref(), Some("high"));
        assert_eq!(s.author.as_deref(), Some("j"));
        assert!(s.has_logsource && s.has_detection);
        assert_eq!(s.tags, 2);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"detection:\n  x: 1\n").is_none()); // no title/logsource
        assert!(parse(b"title: x\n").is_none()); // no detection
        assert!(parse(&[0xff]).is_none());
    }

    #[test]
    fn detect_works() {
        assert!(detect(DOC));
        assert!(!detect(b"detection: nope"));
    }
}
