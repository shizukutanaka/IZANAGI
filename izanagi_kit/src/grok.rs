//! Parser for Grok pattern files (`grok-patterns`, logstash-patterns-core).
//!
//! Counts `NAME regex` pattern definitions, `%{REF}`/`%{REF:field}`/`%{REF:field:type}`
//! references inside patterns, alternation `|`/group `()` constructs, and comments.
//!
//! ```
//! let b = b"USERNAME [a-zA-Z]+\nUSER %{USERNAME}\nIPV4 (?:%{IPV4SEG}){3}\n";
//! assert!(izanagi_kit::grok::detect(b));
//! let c = izanagi_kit::grok::Grok::parse(b).unwrap();
//! assert_eq!(c.patterns, 3);
//! assert_eq!(c.references, 2);
//! ```

/// Parsed Grok patterns summary.
#[derive(Debug, Clone)]
pub struct Grok {
    /// `NAME regex` pattern definitions.
    pub patterns: usize,
    /// `%{…}` pattern references.
    pub references: usize,
    /// `%{NAME:field}` captures with a field name.
    pub field_captures: usize,
    /// `%{NAME:field:type}` typed captures.
    pub typed_captures: usize,
    /// Non-capturing `(?:`/`(?<`/`(?(` groups.
    pub groups: usize,
    /// `|` alternation tokens.
    pub alternations: usize,
    /// Built-in class shortcuts `\\w`/`\\d`/`\\s`/`\\b` used.
    pub class_tokens: usize,
    /// Quantifiers `*`/`+`/`{n}`/`{n,m}` in pattern bodies.
    pub quantifiers: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Returns `true` when the bytes look like a Grok patterns file.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let Ok(t) = std::str::from_utf8(b) else {
        return false;
    };
    let defs = t
        .lines()
        .filter(|l| {
            let tr = l.trim();
            let mut it = tr.split_whitespace();
            let name = it.next().unwrap_or("");
            let rest = it.next().unwrap_or("");
            name.len() >= 2
                && name
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
                && !rest.is_empty()
        })
        .count();
    defs >= 2 && (t.contains("%{") || t.contains("\\b") || t.contains("\\w"))
}

impl Grok {
    /// Parses a Grok patterns file, returning a summary or `None`.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        let t = std::str::from_utf8(b).ok()?;
        if !detect(b) {
            return None;
        }
        let mut c = Self {
            patterns: 0,
            references: 0,
            field_captures: 0,
            typed_captures: 0,
            groups: 0,
            alternations: 0,
            class_tokens: 0,
            quantifiers: 0,
            comments: 0,
        };
        for l in t.lines() {
            let tr = l.trim();
            if tr.is_empty() {
                continue;
            }
            if tr.starts_with('#') {
                c.comments += 1;
                continue;
            }
            let mut it = tr.splitn(2, ' ');
            let name = it.next().unwrap_or("");
            let body = it.next().unwrap_or("").trim_start();
            if name.len() < 2
                || !name
                    .chars()
                    .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
                || body.is_empty()
            {
                continue;
            }
            c.patterns += 1;
            for (i, _) in body.match_indices("%{") {
                let rest = &body[i..];
                if let Some(end) = rest.find('}') {
                    let inner = &rest[2..end];
                    c.references += 1;
                    let parts: Vec<&str> = inner.split(':').collect();
                    if parts.len() >= 2 {
                        c.field_captures += 1;
                    }
                    if parts.len() >= 3 {
                        c.typed_captures += 1;
                    }
                }
            }
            c.groups += body.matches("(?:").count()
                + body.matches("(?<").count()
                + body.matches("(?(").count();
            c.alternations += body.matches('|').count();
            c.class_tokens += body.matches("\\w").count()
                + body.matches("\\d").count()
                + body.matches("\\s").count()
                + body.matches("\\b").count();
            c.quantifiers += body
                .matches('{')
                .count()
                .saturating_sub(body.matches("%{").count() + body.matches("(?{").count())
                + body.matches('*').count()
                + body.matches('+').count()
                + body
                    .matches('?')
                    .count()
                    .saturating_sub(body.matches("(?:").count() + body.matches("(?<").count());
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATTERNS: &[u8] = b"# grok\nUSERNAME [a-zA-Z]+\nUSER %{USERNAME}\nIPV4SEG (?:25[0-5]|2[0-4]\\d|1\\d\\d|\\d{1,2})\nIPV4 %{IPV4SEG}(?:\\.%{IPV4SEG}){3}\nHOSTPORT %{IP:ip}:%{INT:port:int}\n";

    #[test]
    fn parses_grok() {
        let c = Grok::parse(PATTERNS).unwrap();
        assert_eq!(c.patterns, 5);
        assert_eq!(c.references, 5);
        assert_eq!(c.field_captures, 2);
        assert_eq!(c.typed_captures, 1);
        assert_eq!(c.groups, 2);
        assert_eq!(c.comments, 1);
        assert!(c.quantifiers > 0);
    }

    #[test]
    fn rejects_non_grok() {
        assert!(!detect(b"key = value\nother = thing"));
        assert!(Grok::parse(b"x").is_none());
    }
}
