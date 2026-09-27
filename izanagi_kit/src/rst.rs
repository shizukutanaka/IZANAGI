//! reStructuredText: section titles underlined by punctuation, `.. name::`
//! directives, and `:field:` lists (docutils reST).
//!
//! ```
//! use izanagi_kit::rst::parse;
//!
//! let d = b"Title\n=====\n\n.. note:: hi\n\nSub\n---\n";
//! let r = parse(d).unwrap();
//! assert_eq!(r.sections[0].title, "Title");
//! assert_eq!(r.sections[0].level, 0);
//! assert_eq!(r.sections[1].level, 1);
//! assert_eq!(r.directives[0].name, "note");
//! ```

/// One underlined section heading.
#[derive(Debug, Clone)]
pub struct Section {
    /// Heading text.
    pub title: String,
    /// Nesting level: the order in which distinct underline characters first
    /// appear (`=`→0, `-`→1, ...), per docutils convention.
    pub level: usize,
}

/// One `.. name:: arg` directive.
#[derive(Debug, Clone)]
pub struct Directive {
    /// Directive name lowercased.
    pub name: String,
    /// Everything after `::` trimmed.
    pub arg: String,
}

/// Parsed reST document.
#[derive(Debug, Clone)]
pub struct Rst {
    /// Sections in order.
    pub sections: Vec<Section>,
    /// Directives in order.
    pub directives: Vec<Directive>,
}

fn under(c: u8) -> bool {
    matches!(
        c,
        b'=' | b'-' | b'~' | b'^' | b'"' | b'#' | b'*' | b'+' | b'`' | b':' | b'.'
    )
}

/// Parse a document; a non-blank line followed by a same-length underline of
/// one repeated punctuation char forms a section.
pub fn parse(data: &[u8]) -> Option<Rst> {
    let text = std::str::from_utf8(data).ok()?;
    let lines: Vec<&str> = text.lines().collect();
    let mut sections = Vec::new();
    let mut directives = Vec::new();
    let mut chars: Vec<u8> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim_end();
        if t.is_empty() {
            continue;
        }
        if let Some(rest) = t.strip_prefix(".. ") {
            if let Some(dpos) = rest.find("::") {
                let name = rest[..dpos].trim().to_lowercase();
                let arg = rest[dpos + 2..].trim().to_string();
                if !name.is_empty() {
                    directives.push(Directive { name, arg });
                }
                continue;
            }
        }
        // underline check: next line all one `under` char, len >= title len
        if let Some(next) = lines.get(i + 1) {
            let u = next.trim_end();
            if !u.is_empty()
                && u.len() >= t.len()
                && u.bytes().all(|b| b == u.as_bytes()[0])
                && under(u.as_bytes()[0])
                && !t.bytes().all(|b| b == t.as_bytes()[0])
            {
                let ch = u.as_bytes()[0];
                let level = match chars.iter().position(|&c| c == ch) {
                    Some(p) => p,
                    None => {
                        chars.push(ch);
                        chars.len() - 1
                    }
                };
                sections.push(Section {
                    title: t.to_string(),
                    level,
                });
                continue;
            }
        }
    }
    Some(Rst {
        sections,
        directives,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let d = b"Doc\n===\n\n.. image:: a.png\n\nPart A\n------\n\nDeep\n^^^^\n";
        let r = parse(d).unwrap();
        assert_eq!(r.sections.len(), 3);
        assert_eq!(r.sections[2].level, 2);
        assert_eq!(r.directives[0].arg, "a.png");
    }

    #[test]
    fn no_title_underline_is_not_section() {
        let d = b"-----\ntext\n";
        let r = parse(d).unwrap();
        assert!(r.sections.is_empty());
    }
}
