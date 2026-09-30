//! Abaqus `.inp` input deck — the keyword-driven text model for
//! Abaqus/Standard & Explicit. Comments start `**`, keywords are
//! `*Keyword, param=value` records, and data lines carry the node /
//! element tables under `*NODE` / `*ELEMENT` sections.
//!
//! `parse` requires at least one `*KEYWORD` line and one `*NODE` or
//! `*ELEMENT` section, counts keywords, and reports the `*Heading`
//! line text when present.
//!
//! ```
//! let f = b"*Heading\n** comment\n*Node\n1, 0., 0.\n*Element\n1, 1\n*End\n";
//! let i = izanagi_kit::inp::parse(f).unwrap();
//! assert_eq!(i.keywords, 4);
//! assert!(i.has_node && i.has_element);
//! assert_eq!(i.heading.as_deref(), Some("*Heading"));
//! ```

/// Parsed Abaqus input-deck summary.
#[derive(Debug, Clone, PartialEq)]
pub struct Inp {
    /// `*KEYWORD` records seen.
    pub keywords: usize,
    /// `*Heading` first line text, when present.
    pub heading: Option<String>,
    /// `*NODE` section present.
    pub has_node: bool,
    /// `*ELEMENT` section present.
    pub has_element: bool,
    /// `*MATERIAL` section present.
    pub has_material: bool,
}

fn kw(line: &str) -> Option<&str> {
    let l = line.trim_start();
    (l.starts_with('*') && !l.starts_with("**")).then_some(l)
}

/// Parse an `.inp` deck; `None` without keywords + node/element.
pub fn parse(d: &[u8]) -> Option<Inp> {
    let s = std::str::from_utf8(d).ok()?;
    let mut keywords = 0usize;
    let mut heading = None;
    let mut has_node = false;
    let mut has_element = false;
    let mut has_material = false;
    for line in s.lines() {
        let Some(k) = kw(line) else { continue };
        keywords += 1;
        let up = k.to_uppercase();
        if up.starts_with("*HEADING") && heading.is_none() {
            heading = Some(k.to_string());
        }
        if up.starts_with("*NODE") {
            has_node = true;
        }
        if up.starts_with("*ELEMENT") {
            has_element = true;
        }
        if up.starts_with("*MATERIAL") {
            has_material = true;
        }
    }
    if keywords == 0 || !(has_node || has_element) {
        return None;
    }
    Some(Inp {
        keywords,
        heading,
        has_node,
        has_element,
        has_material,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let f = b"*Heading\n*Node\n1, 0., 0.\n*Element\n1, 1\n*Material\n";
        let i = parse(f).unwrap();
        assert_eq!(i.keywords, 4);
        assert!(i.has_material);
    }

    #[test]
    fn rejects() {
        assert!(parse(b"").is_none());
        assert!(parse(b"** only a comment\n").is_none());
        assert!(parse(b"*Heading\n*Material\n").is_none());
    }
}
