//! ABBYY Lingvo `.dsl` — annotated dictionary source: `#NAME "…"`,
//! `#INDEX_LANGUAGE`, `#CONTENTS_LANGUAGE`, `#CHARSET`, `#INCLUDE`
//! directives; headwords flush-left, definition bodies indented.
//!
//! ```
//! use izanagi_kit::dsl::{detect, parse};
//!
//! let d = b"#NAME \"Demo\"\n#INDEX_LANGUAGE \"English\"\n#CONTENTS_LANGUAGE \"Russian\"\n\
//! word\n\tdefinition one\n\tdefinition two\nother\n\tdef\n";
//! assert!(detect(d));
//! let l = parse(d).unwrap();
//! assert_eq!(l.name.as_deref(), Some("Demo"));
//! assert_eq!(l.words, 2);
//! ```

/// Parsed Lingvo DSL census.
#[derive(Debug, Clone, PartialEq)]
pub struct Dsl {
    /// `#NAME` quoted value.
    pub name: Option<String>,
    /// `#INDEX_LANGUAGE` quoted value.
    pub index_language: Option<String>,
    /// `#CONTENTS_LANGUAGE` quoted value.
    pub contents_language: Option<String>,
    /// `#INCLUDE` lines (embedded sub-dictionaries).
    pub includes: u32,
    /// `#ABBREVIATE` lines in an abbreviation prologue.
    pub abbreviations: u32,
    /// Flush-left headword lines.
    pub words: u32,
    /// Indented body lines.
    pub body_lines: u32,
    /// `[s]sound.wav[/s]` media references.
    pub media_refs: u32,
    /// `[ref]…[/ref]` / `<<…>>` cross-references.
    pub refs: u32,
}

fn directive<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    for line in s.lines() {
        if let Some(rest) = line.trim_start().strip_prefix(key) {
            let v = rest.trim();
            return Some(v.trim_matches('"'));
        }
    }
    None
}

/// `true` on a `#NAME`/`#INDEX_LANGUAGE` directive header + body lines.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let s = match core::str::from_utf8(b) {
        Ok(s) => s,
        Err(_) => return false,
    };
    if !(s.contains("#NAME") || s.contains("#INDEX_LANGUAGE")) {
        return false;
    }
    let mut directive = false;
    let mut body = false;
    for line in s.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            directive = true;
        } else {
            body = true;
        }
    }
    directive && body
}

/// Census; `None` without DSL directives.
#[must_use]
pub fn parse(b: &[u8]) -> Option<Dsl> {
    if !detect(b) {
        return None;
    }
    let s = core::str::from_utf8(b).ok()?;
    let mut l = Dsl {
        name: directive(s, "#NAME").map(str::to_string),
        index_language: directive(s, "#INDEX_LANGUAGE").map(str::to_string),
        contents_language: directive(s, "#CONTENTS_LANGUAGE").map(str::to_string),
        includes: 0,
        abbreviations: 0,
        words: 0,
        body_lines: 0,
        media_refs: 0,
        refs: 0,
    };
    for line in s.lines() {
        let t = line.trim_end();
        if t.starts_with('#') {
            if t.starts_with("#INCLUDE") {
                l.includes += 1;
            } else if t.starts_with("#ABBREVIATE") {
                l.abbreviations += 1;
            }
            continue;
        }
        if t.is_empty() {
            continue;
        }
        if t.starts_with(' ') || t.starts_with('\t') {
            l.body_lines += 1;
            l.media_refs += t.matches("[s]").count() as u32;
            l.refs += t.matches("[ref]").count() as u32;
            l.refs += t.matches("<<").count() as u32;
        } else {
            l.words += 1;
        }
    }
    Some(l)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: &[u8] =
        b"#NAME \"Demo\"\n#INDEX_LANGUAGE \"English\"\n#CONTENTS_LANGUAGE \"Russian\"\n\
#INCLUDE \"abbr.dsl\"\nword\n\tdefinition one [s]a.wav[/s]\n\tdef two <<ref>>\nother\n\tdef\n";

    #[test]
    fn detect_works() {
        assert!(detect(D));
        assert!(!detect(b"#NAME \"x\""));
        assert!(!detect(b"plain"));
    }

    #[test]
    fn parses() {
        let l = parse(D).unwrap();
        assert_eq!(l.name.as_deref(), Some("Demo"));
        assert_eq!(l.index_language.as_deref(), Some("English"));
        assert_eq!(l.contents_language.as_deref(), Some("Russian"));
        assert_eq!(l.includes, 1);
        assert_eq!(l.words, 2);
        assert_eq!(l.body_lines, 3);
        assert_eq!(l.media_refs, 1);
        assert_eq!(l.refs, 1);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"plain").is_none());
    }
}
